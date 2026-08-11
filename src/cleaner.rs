use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::report::{CleanReceipt, ItemIssue};
use crate::safety::secure_remove_candidate;
use crate::scanner::ScanReport;

#[derive(Clone, Debug, Default)]
pub struct CleanProgress {
    pub completed: usize,
    pub total: usize,
    pub deleted: usize,
    pub deleted_bytes: u64,
    pub current_group: String,
}

pub fn confirmation_phrase(file_count: usize) -> String {
    format!("DELETE {file_count} FILES")
}

pub fn clean_selected<F>(
    report: &ScanReport,
    selected: &BTreeSet<usize>,
    cancel: &AtomicBool,
    volume_path: &Path,
    mut on_progress: F,
) -> CleanReceipt
where
    F: FnMut(CleanProgress),
{
    let requested_files = report
        .groups
        .iter()
        .filter(|group| selected.contains(&group.id))
        .map(|group| group.files.len())
        .sum();
    let mut receipt = CleanReceipt {
        requested_files,
        available_before: available_space(volume_path),
        ..CleanReceipt::default()
    };
    let mut completed = 0;

    for group in report
        .groups
        .iter()
        .filter(|group| selected.contains(&group.id))
    {
        for candidate in &group.files {
            if cancel.load(Ordering::SeqCst) {
                receipt.cancelled = true;
                receipt.available_after = available_space(volume_path);
                return receipt;
            }
            match secure_remove_candidate(candidate) {
                Ok(()) => {
                    receipt.deleted_files += 1;
                    receipt.deleted_logical_bytes = receipt
                        .deleted_logical_bytes
                        .saturating_add(candidate.logical_bytes());
                    receipt.deleted_allocated_bytes = receipt
                        .deleted_allocated_bytes
                        .saturating_add(candidate.allocated_bytes());
                }
                Err(reason) => receipt.skipped.push(ItemIssue {
                    path: candidate.path.clone(),
                    reason: reason.to_string(),
                }),
            }
            completed += 1;
            on_progress(CleanProgress {
                completed,
                total: requested_files,
                deleted: receipt.deleted_files,
                deleted_bytes: receipt.deleted_allocated_bytes,
                current_group: group.name.clone(),
            });
        }
    }
    receipt.available_after = available_space(volume_path);
    receipt
}

pub fn available_space(path: &Path) -> Option<u64> {
    let output = Command::new("/bin/df")
        .args(["-Pk"])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let line = stdout.lines().last()?;
    let fields: Vec<&str> = line.split_whitespace().collect();
    let available_kib: u64 = fields.get(fields.len().checked_sub(3)?)?.parse().ok()?;
    available_kib.checked_mul(1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_is_explicit() {
        assert_eq!(confirmation_phrase(42), "DELETE 42 FILES");
    }
}
