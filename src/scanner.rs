use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use crate::report::ItemIssue;
use crate::rules::RootSpec;
use crate::safety::{FileIdentity, age_is_eligible};

pub const DEFAULT_MINIMUM_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Clone, Debug)]
pub struct ScanConfig {
    pub roots: Vec<RootSpec>,
    pub include_recent: bool,
    pub now: SystemTime,
}

impl ScanConfig {
    pub fn new(roots: Vec<RootSpec>, include_recent: bool) -> Self {
        Self {
            roots,
            include_recent,
            now: SystemTime::now(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CandidateFile {
    pub path: PathBuf,
    pub root: PathBuf,
    pub relative_path: PathBuf,
    pub identity: FileIdentity,
}

impl CandidateFile {
    pub fn logical_bytes(&self) -> u64 {
        self.identity.length
    }

    pub fn allocated_bytes(&self) -> u64 {
        self.identity.allocated
    }
}

#[derive(Clone, Debug)]
pub struct CandidateGroup {
    pub id: usize,
    pub root_label: String,
    pub name: String,
    pub root: PathBuf,
    pub files: Vec<CandidateFile>,
    pub logical_bytes: u64,
    pub allocated_bytes: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ScanStats {
    pub visited: usize,
    pub eligible: usize,
    pub logical_bytes: u64,
    pub allocated_bytes: u64,
    pub skipped_symlinks: usize,
    pub skipped_special: usize,
    pub skipped_hardlinks: usize,
    pub skipped_mounts: usize,
    pub skipped_recent: usize,
}

#[derive(Clone, Debug, Default)]
pub struct ScanReport {
    pub groups: Vec<CandidateGroup>,
    pub issues: Vec<ItemIssue>,
    pub stats: ScanStats,
    pub cancelled: bool,
}

impl ScanReport {
    pub fn total_files(&self) -> usize {
        self.groups.iter().map(|group| group.files.len()).sum()
    }

    pub fn logical_bytes(&self) -> u64 {
        self.groups.iter().map(|group| group.logical_bytes).sum()
    }

    pub fn allocated_bytes(&self) -> u64 {
        self.groups.iter().map(|group| group.allocated_bytes).sum()
    }

    pub fn selected_totals(&self, selected: &std::collections::BTreeSet<usize>) -> (usize, u64) {
        self.groups
            .iter()
            .filter(|group| selected.contains(&group.id))
            .fold((0, 0), |(files, bytes), group| {
                (files + group.files.len(), bytes + group.allocated_bytes)
            })
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScanProgress {
    pub visited: usize,
    pub eligible: usize,
    pub allocated_bytes: u64,
    pub current_group: String,
}

#[derive(Debug)]
struct GroupBuilder {
    root_label: String,
    name: String,
    root: PathBuf,
    files: Vec<CandidateFile>,
}

pub fn scan<F>(config: &ScanConfig, cancel: &AtomicBool, mut on_progress: F) -> ScanReport
where
    F: FnMut(ScanProgress),
{
    let mut report = ScanReport::default();
    let mut groups: BTreeMap<(PathBuf, String), GroupBuilder> = BTreeMap::new();

    for root in &config.roots {
        if cancel.load(Ordering::Relaxed) {
            report.cancelled = true;
            break;
        }
        let root_metadata = match fs::symlink_metadata(&root.path) {
            Ok(metadata) if metadata.file_type().is_dir() => metadata,
            Ok(_) => {
                report.issues.push(ItemIssue {
                    path: root.path.clone(),
                    reason: "allowlisted root is not a directory".to_string(),
                });
                continue;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => {
                report.issues.push(ItemIssue {
                    path: root.path.clone(),
                    reason: error.to_string(),
                });
                continue;
            }
        };
        if root_metadata.file_type().is_symlink() {
            report.stats.skipped_symlinks += 1;
            continue;
        }
        let root_device = root_metadata.dev();

        let entries = match fs::read_dir(&root.path) {
            Ok(entries) => entries,
            Err(error) => {
                report.issues.push(ItemIssue {
                    path: root.path.clone(),
                    reason: error.to_string(),
                });
                continue;
            }
        };

        for entry in entries {
            if cancel.load(Ordering::Relaxed) {
                report.cancelled = true;
                break;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    report.issues.push(ItemIssue {
                        path: root.path.clone(),
                        reason: error.to_string(),
                    });
                    continue;
                }
            };
            let group_name = entry.file_name().to_string_lossy().into_owned();
            let group_name = if group_name.is_empty() {
                "Loose files".to_string()
            } else {
                group_name
            };
            let key = (root.path.clone(), group_name.clone());
            groups.entry(key.clone()).or_insert_with(|| GroupBuilder {
                root_label: root.label.clone(),
                name: group_name.clone(),
                root: root.path.clone(),
                files: Vec::new(),
            });
            walk_entry(
                &entry.path(),
                &root.path,
                root_device,
                &group_name,
                config,
                cancel,
                groups.get_mut(&key).expect("inserted group"),
                &mut report,
                &mut on_progress,
            );
        }
    }

    report.groups = groups
        .into_values()
        .filter(|group| !group.files.is_empty())
        .enumerate()
        .map(|(id, group)| {
            let logical_bytes = group.files.iter().map(CandidateFile::logical_bytes).sum();
            let allocated_bytes = group.files.iter().map(CandidateFile::allocated_bytes).sum();
            CandidateGroup {
                id,
                root_label: group.root_label,
                name: group.name,
                root: group.root,
                files: group.files,
                logical_bytes,
                allocated_bytes,
            }
        })
        .collect();
    report
}

#[allow(clippy::too_many_arguments)]
fn walk_entry<F>(
    path: &Path,
    root: &Path,
    root_device: u64,
    group_name: &str,
    config: &ScanConfig,
    cancel: &AtomicBool,
    group: &mut GroupBuilder,
    report: &mut ScanReport,
    on_progress: &mut F,
) where
    F: FnMut(ScanProgress),
{
    if cancel.load(Ordering::Relaxed) {
        report.cancelled = true;
        return;
    }
    report.stats.visited += 1;

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            report.issues.push(ItemIssue {
                path: path.to_path_buf(),
                reason: error.to_string(),
            });
            return;
        }
    };
    let file_type = metadata.file_type();

    if file_type.is_symlink() {
        report.stats.skipped_symlinks += 1;
        return;
    }
    if metadata.dev() != root_device {
        report.stats.skipped_mounts += 1;
        return;
    }

    if file_type.is_dir() {
        match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries {
                    if cancel.load(Ordering::Relaxed) {
                        report.cancelled = true;
                        return;
                    }
                    match entry {
                        Ok(entry) => walk_entry(
                            &entry.path(),
                            root,
                            root_device,
                            group_name,
                            config,
                            cancel,
                            group,
                            report,
                            on_progress,
                        ),
                        Err(error) => report.issues.push(ItemIssue {
                            path: path.to_path_buf(),
                            reason: error.to_string(),
                        }),
                    }
                }
            }
            Err(error) => report.issues.push(ItemIssue {
                path: path.to_path_buf(),
                reason: error.to_string(),
            }),
        }
        return;
    }

    if !file_type.is_file() {
        report.stats.skipped_special += 1;
        return;
    }
    if metadata.nlink() != 1 {
        report.stats.skipped_hardlinks += 1;
        return;
    }

    let modified = match metadata.modified() {
        Ok(modified) => modified,
        Err(error) => {
            report.issues.push(ItemIssue {
                path: path.to_path_buf(),
                reason: error.to_string(),
            });
            return;
        }
    };
    if !config.include_recent && !age_is_eligible(modified, config.now, DEFAULT_MINIMUM_AGE) {
        report.stats.skipped_recent += 1;
        return;
    }

    let identity = match FileIdentity::from_metadata(&metadata) {
        Ok(identity) => identity,
        Err(error) => {
            report.issues.push(ItemIssue {
                path: path.to_path_buf(),
                reason: error.to_string(),
            });
            return;
        }
    };
    let relative_path = match path.strip_prefix(root) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => {
            report.issues.push(ItemIssue {
                path: path.to_path_buf(),
                reason: "path escaped its approved cache root".to_string(),
            });
            return;
        }
    };

    report.stats.eligible += 1;
    report.stats.logical_bytes = report.stats.logical_bytes.saturating_add(identity.length);
    report.stats.allocated_bytes = report
        .stats
        .allocated_bytes
        .saturating_add(identity.allocated);
    group.files.push(CandidateFile {
        path: path.to_path_buf(),
        root: root.to_path_buf(),
        relative_path,
        identity,
    });

    if report.stats.visited.is_multiple_of(64) || report.stats.eligible <= 8 {
        on_progress(ScanProgress {
            visited: report.stats.visited,
            eligible: report.stats.eligible,
            allocated_bytes: report.stats.allocated_bytes,
            current_group: group_name.to_string(),
        });
    }
}
