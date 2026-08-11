use std::ffi::CString;
use std::fs::Metadata;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::scanner::CandidateFile;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileIdentity {
    pub device: u64,
    pub inode: u64,
    pub links: u64,
    pub length: u64,
    pub allocated: u64,
    pub modified_ns: u128,
}

impl FileIdentity {
    pub fn from_metadata(metadata: &Metadata) -> io::Result<Self> {
        let modified_ns = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            links: metadata.nlink(),
            length: metadata.len(),
            allocated: metadata.blocks().saturating_mul(512),
            modified_ns,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevalidationFailure {
    OutsideRoot,
    RootChanged,
    Metadata(String),
    NotRegularFile,
    HardLinked,
    DifferentDevice,
    IdentityChanged,
    InvalidComponent,
    SecureOpen(String),
}

impl std::fmt::Display for RevalidationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutsideRoot => write!(f, "path escaped its approved cache root"),
            Self::RootChanged => write!(f, "cache root changed after scanning"),
            Self::Metadata(error) => write!(f, "metadata unavailable: {error}"),
            Self::NotRegularFile => write!(f, "entry is no longer a regular file"),
            Self::HardLinked => write!(f, "entry has multiple hard links"),
            Self::DifferentDevice => write!(f, "entry crossed a filesystem boundary"),
            Self::IdentityChanged => write!(f, "entry changed after scanning"),
            Self::InvalidComponent => write!(f, "entry contains an unsafe path component"),
            Self::SecureOpen(error) => write!(f, "secure path traversal failed: {error}"),
        }
    }
}

pub fn validate_candidate(candidate: &CandidateFile) -> Result<(), RevalidationFailure> {
    if candidate.path == candidate.root || !candidate.path.starts_with(&candidate.root) {
        return Err(RevalidationFailure::OutsideRoot);
    }

    let root_meta = std::fs::symlink_metadata(&candidate.root)
        .map_err(|error| RevalidationFailure::Metadata(error.to_string()))?;
    if !root_meta.file_type().is_dir() || root_meta.dev() != candidate.identity.device {
        return Err(RevalidationFailure::RootChanged);
    }

    let relative = candidate
        .path
        .strip_prefix(&candidate.root)
        .map_err(|_| RevalidationFailure::OutsideRoot)?;
    let mut ancestor = candidate.root.clone();
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        if components.peek().is_none() {
            break;
        }
        let std::path::Component::Normal(component) = component else {
            return Err(RevalidationFailure::InvalidComponent);
        };
        ancestor.push(component);
        let metadata = std::fs::symlink_metadata(&ancestor)
            .map_err(|error| RevalidationFailure::Metadata(error.to_string()))?;
        if !metadata.file_type().is_dir() || metadata.dev() != candidate.identity.device {
            return Err(RevalidationFailure::RootChanged);
        }
    }

    let metadata = std::fs::symlink_metadata(&candidate.path)
        .map_err(|error| RevalidationFailure::Metadata(error.to_string()))?;
    if !metadata.file_type().is_file() {
        return Err(RevalidationFailure::NotRegularFile);
    }
    let identity = FileIdentity::from_metadata(&metadata)
        .map_err(|error| RevalidationFailure::Metadata(error.to_string()))?;
    if identity.links != 1 {
        return Err(RevalidationFailure::HardLinked);
    }
    if identity.device != candidate.identity.device {
        return Err(RevalidationFailure::DifferentDevice);
    }
    if identity != candidate.identity {
        return Err(RevalidationFailure::IdentityChanged);
    }
    Ok(())
}

/// Removes a manifest entry relative to directory descriptors opened with
/// `O_NOFOLLOW`. This prevents a replaced ancestor symlink from redirecting
/// the unlink operation outside the directory tree that was scanned.
#[cfg(target_os = "macos")]
pub fn secure_remove_candidate(candidate: &CandidateFile) -> Result<(), RevalidationFailure> {
    validate_candidate(candidate)?;
    let relative = candidate
        .path
        .strip_prefix(&candidate.root)
        .map_err(|_| RevalidationFailure::OutsideRoot)?;
    let components = relative
        .components()
        .map(|component| match component {
            std::path::Component::Normal(value) => {
                CString::new(value.as_bytes()).map_err(|_| RevalidationFailure::InvalidComponent)
            }
            _ => Err(RevalidationFailure::InvalidComponent),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (file_name, parent_components) = components
        .split_last()
        .ok_or(RevalidationFailure::InvalidComponent)?;
    let root = CString::new(candidate.root.as_os_str().as_bytes())
        .map_err(|_| RevalidationFailure::InvalidComponent)?;

    let root_fd = unsafe {
        // SAFETY: `root` is a valid NUL-terminated path and the return value is
        // checked before ownership is created.
        libc::open(
            root.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if root_fd < 0 {
        return Err(RevalidationFailure::SecureOpen(
            io::Error::last_os_error().to_string(),
        ));
    }
    let mut directory = unsafe {
        // SAFETY: `root_fd` is non-negative and uniquely owned here.
        OwnedFd::from_raw_fd(root_fd)
    };
    for component in parent_components {
        let next_fd = unsafe {
            // SAFETY: both the directory descriptor and component C string are
            // valid for the duration of this call.
            libc::openat(
                directory.as_raw_fd(),
                component.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if next_fd < 0 {
            return Err(RevalidationFailure::SecureOpen(
                io::Error::last_os_error().to_string(),
            ));
        }
        directory = unsafe {
            // SAFETY: `next_fd` is non-negative and uniquely owned here.
            OwnedFd::from_raw_fd(next_fd)
        };
    }

    let mut stat: libc::stat = unsafe {
        // SAFETY: an all-zero `stat` is a valid output buffer for `fstatat`.
        std::mem::zeroed()
    };
    let result = unsafe {
        // SAFETY: the descriptor, file name, and output pointer are valid.
        libc::fstatat(
            directory.as_raw_fd(),
            file_name.as_ptr(),
            &mut stat,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return Err(RevalidationFailure::SecureOpen(
            io::Error::last_os_error().to_string(),
        ));
    }
    if stat.st_mode & libc::S_IFMT != libc::S_IFREG {
        return Err(RevalidationFailure::NotRegularFile);
    }
    let modified_ns = u128::try_from(stat.st_mtime)
        .unwrap_or_default()
        .saturating_mul(1_000_000_000)
        .saturating_add(u128::try_from(stat.st_mtime_nsec).unwrap_or_default());
    let identity = FileIdentity {
        device: u64::try_from(stat.st_dev).unwrap_or_default(),
        inode: stat.st_ino,
        links: u64::from(stat.st_nlink),
        length: u64::try_from(stat.st_size).unwrap_or_default(),
        allocated: u64::try_from(stat.st_blocks)
            .unwrap_or_default()
            .saturating_mul(512),
        modified_ns,
    };
    if identity != candidate.identity {
        return Err(RevalidationFailure::IdentityChanged);
    }

    let result = unsafe {
        // SAFETY: unlink is anchored to the validated parent descriptor and
        // receives only the final file-name component.
        libc::unlinkat(directory.as_raw_fd(), file_name.as_ptr(), 0)
    };
    if result != 0 {
        return Err(RevalidationFailure::SecureOpen(
            io::Error::last_os_error().to_string(),
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn secure_remove_candidate(_candidate: &CandidateFile) -> Result<(), RevalidationFailure> {
    Err(RevalidationFailure::SecureOpen(
        "cleanup is supported only on macOS".to_string(),
    ))
}

pub fn path_for_display(path: &Path, home: &Path) -> PathBuf {
    path.strip_prefix(home)
        .map(|suffix| Path::new("~").join(suffix))
        .unwrap_or_else(|_| path.to_path_buf())
}

pub fn age_is_eligible(
    modified: SystemTime,
    now: SystemTime,
    minimum_age: std::time::Duration,
) -> bool {
    now.duration_since(modified)
        .map(|age| age >= minimum_age)
        .unwrap_or(false)
}
