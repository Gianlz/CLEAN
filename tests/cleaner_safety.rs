use std::collections::BTreeSet;
use std::fs;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, SystemTime};

use clean_tui::cleaner::clean_selected;
use clean_tui::rules::roots_for_home;
use clean_tui::scanner::{ScanConfig, scan};

fn scanned_fixture() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    clean_tui::scanner::ScanReport,
) {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let group = home.join("Library/Caches/com.example.app");
    fs::create_dir_all(&group).unwrap();
    fs::create_dir_all(home.join(".cache")).unwrap();
    fs::write(group.join("one.bin"), b"one").unwrap();
    fs::write(group.join("two.bin"), b"two two").unwrap();
    let mut config = ScanConfig::new(roots_for_home(&home).unwrap(), false);
    config.now = SystemTime::now() + Duration::from_secs(8 * 24 * 60 * 60);
    let report = scan(&config, &AtomicBool::new(false), |_| {});
    (temp, home, report)
}

#[test]
fn deletes_only_selected_manifest_files_and_leaves_directories() {
    let (_temp, home, report) = scanned_fixture();
    assert_eq!(report.total_files(), 2);
    let selected: BTreeSet<_> = report.groups.iter().map(|group| group.id).collect();
    let receipt = clean_selected(&report, &selected, &AtomicBool::new(false), &home, |_| {});
    assert_eq!(receipt.deleted_files, 2);
    assert!(receipt.skipped.is_empty());
    assert!(receipt.failed.is_empty());
    assert!(home.join("Library/Caches/com.example.app").is_dir());
    assert_eq!(
        fs::read_dir(home.join("Library/Caches/com.example.app"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn changed_file_is_preserved() {
    let (_temp, home, report) = scanned_fixture();
    let changed = &report.groups[0].files[0].path;
    fs::write(changed, b"changed after consent").unwrap();
    let selected: BTreeSet<_> = report.groups.iter().map(|group| group.id).collect();
    let receipt = clean_selected(&report, &selected, &AtomicBool::new(false), &home, |_| {});
    assert_eq!(receipt.deleted_files, 1);
    assert_eq!(receipt.skipped.len(), 1);
    assert_eq!(fs::read(changed).unwrap(), b"changed after consent");
}

#[test]
fn replaced_ancestor_symlink_cannot_redirect_deletion() {
    use std::os::unix::fs::symlink;

    let (temp, home, report) = scanned_fixture();
    let group = home.join("Library/Caches/com.example.app");
    let moved = home.join("Library/Caches/moved-app");
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let sentinel = outside.join("one.bin");
    fs::write(&sentinel, b"outside sentinel").unwrap();
    fs::rename(&group, &moved).unwrap();
    symlink(&outside, &group).unwrap();

    let selected: BTreeSet<_> = report.groups.iter().map(|group| group.id).collect();
    let receipt = clean_selected(&report, &selected, &AtomicBool::new(false), &home, |_| {});
    assert_eq!(receipt.deleted_files, 0);
    assert_eq!(receipt.skipped.len(), 2);
    assert_eq!(fs::read(&sentinel).unwrap(), b"outside sentinel");
    assert!(moved.join("one.bin").exists());
    assert!(moved.join("two.bin").exists());
}

#[test]
fn cancellation_before_cleanup_preserves_everything() {
    let (_temp, home, report) = scanned_fixture();
    let selected: BTreeSet<_> = report.groups.iter().map(|group| group.id).collect();
    let receipt = clean_selected(&report, &selected, &AtomicBool::new(true), &home, |_| {});
    assert!(receipt.cancelled);
    assert_eq!(receipt.deleted_files, 0);
    assert!(home.join("Library/Caches/com.example.app/one.bin").exists());
    assert!(home.join("Library/Caches/com.example.app/two.bin").exists());
}

#[test]
fn no_selection_is_a_noop() {
    let (_temp, home, report) = scanned_fixture();
    let receipt = clean_selected(
        &report,
        &BTreeSet::new(),
        &AtomicBool::new(false),
        &home,
        |_| {},
    );
    assert_eq!(receipt.requested_files, 0);
    assert_eq!(receipt.deleted_files, 0);
    assert!(home.join("Library/Caches/com.example.app/one.bin").exists());
}
