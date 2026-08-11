use std::fmt;
use std::io;
use std::path::{Component, Path, PathBuf};

use uzers::os::unix::UserExt;
use uzers::{get_current_uid, get_user_by_uid};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CacheKind {
    MacOs,
    DotCache,
}

impl CacheKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::MacOs => "macOS app caches",
            Self::DotCache => "Developer caches",
        }
    }

    fn suffix(self) -> &'static Path {
        match self {
            Self::MacOs => Path::new("Library/Caches"),
            Self::DotCache => Path::new(".cache"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RootSpec {
    pub kind: CacheKind,
    pub label: String,
    pub home: PathBuf,
    pub path: PathBuf,
}

#[derive(Debug)]
pub enum RuleError {
    MissingUser,
    ElevatedPrivileges,
    InvalidHome(PathBuf),
    InvalidRoot(PathBuf),
    Io(io::Error),
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingUser => write!(f, "could not resolve the current system account"),
            Self::ElevatedPrivileges => write!(f, "refusing to run cache cleanup as root"),
            Self::InvalidHome(path) => {
                write!(f, "refusing unsafe home directory: {}", path.display())
            }
            Self::InvalidRoot(path) => write!(f, "refusing unsafe cache root: {}", path.display()),
            Self::Io(error) => write!(f, "filesystem error: {error}"),
        }
    }
}

impl std::error::Error for RuleError {}

impl From<io::Error> for RuleError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn current_home() -> Result<PathBuf, RuleError> {
    let uid = get_current_uid();
    if uid == 0 {
        return Err(RuleError::ElevatedPrivileges);
    }
    let user = get_user_by_uid(uid).ok_or(RuleError::MissingUser)?;
    validate_home(user.home_dir())
}

pub fn default_roots() -> Result<Vec<RootSpec>, RuleError> {
    roots_for_home(&current_home()?)
}

pub fn roots_for_home(home: &Path) -> Result<Vec<RootSpec>, RuleError> {
    let home = validate_home(home)?;
    [CacheKind::MacOs, CacheKind::DotCache]
        .into_iter()
        .map(|kind| RootSpec::new(&home, kind))
        .collect()
}

pub fn validate_home(home: &Path) -> Result<PathBuf, RuleError> {
    if !home.is_absolute()
        || home == Path::new("/")
        || home
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(RuleError::InvalidHome(home.to_path_buf()));
    }

    if home.exists() {
        let canonical = home.canonicalize()?;
        if canonical == Path::new("/") {
            return Err(RuleError::InvalidHome(canonical));
        }
        Ok(canonical)
    } else {
        Ok(home.to_path_buf())
    }
}

impl RootSpec {
    pub fn new(home: &Path, kind: CacheKind) -> Result<Self, RuleError> {
        let home = validate_home(home)?;
        let expected = home.join(kind.suffix());
        validate_exact_root(&home, &expected, kind)?;

        Ok(Self {
            kind,
            label: kind.label().to_string(),
            home,
            path: expected,
        })
    }
}

fn validate_exact_root(home: &Path, root: &Path, kind: CacheKind) -> Result<(), RuleError> {
    let expected = home.join(kind.suffix());
    if root != expected
        || root == home
        || !root.starts_with(home)
        || root
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(RuleError::InvalidRoot(root.to_path_buf()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_broad_home_paths() {
        assert!(validate_home(Path::new("/")).is_err());
        assert!(validate_home(Path::new("relative/home")).is_err());
        assert!(validate_home(Path::new("/tmp/../etc")).is_err());
    }

    #[test]
    fn creates_only_exact_allowlisted_roots() {
        let roots = roots_for_home(Path::new("/Users/tester")).unwrap();
        assert_eq!(roots[0].path, Path::new("/Users/tester/Library/Caches"));
        assert_eq!(roots[1].path, Path::new("/Users/tester/.cache"));
    }
}
