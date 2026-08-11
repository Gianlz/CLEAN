use std::fs;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, SystemTime};

use clean_tui::rules::roots_for_home;
use clean_tui::scanner::{ScanConfig, scan};
use tempfile::TempDir;

fn fixture() -> (TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(home.join("Library/Caches/com.example.app")).unwrap();
    fs::create_dir_all(home.join(".cache/tool")).unwrap();
    (temp, home)
}

#[test]
fn dry_run_is_read_only_and_applies_the_age_window() {
    let (_temp, home) = fixture();
    let old = home.join("Library/Caches/com.example.app/old.bin");
    fs::write(&old, b"precious fixture bytes").unwrap();
    let roots = roots_for_home(&home).unwrap();

    let recent = scan(
        &ScanConfig::new(roots.clone(), false),
        &AtomicBool::new(false),
        |_| {},
    );
    assert_eq!(recent.total_files(), 0);

    let mut config = ScanConfig::new(roots, false);
    config.now = SystemTime::now() + Duration::from_secs(8 * 24 * 60 * 60);
    let old_enough = scan(&config, &AtomicBool::new(false), |_| {});
    assert_eq!(old_enough.total_files(), 1);
    assert_eq!(fs::read(&old).unwrap(), b"precious fixture bytes");
}

#[test]
fn include_recent_is_an_explicit_override() {
    let (_temp, home) = fixture();
    let recent = home.join(".cache/tool/fresh.bin");
    fs::write(&recent, b"fresh").unwrap();
    let report = scan(
        &ScanConfig::new(roots_for_home(&home).unwrap(), true),
        &AtomicBool::new(false),
        |_| {},
    );
    assert_eq!(report.total_files(), 1);
    assert!(recent.exists());
}

#[test]
fn symlink_escape_and_hard_links_never_become_candidates() {
    use std::os::unix::fs::symlink;

    let (temp, home) = fixture();
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let sentinel = outside.join("sentinel.txt");
    fs::write(&sentinel, b"do not touch").unwrap();
    symlink(&outside, home.join("Library/Caches/com.example.app/escape")).unwrap();

    let original = home.join(".cache/tool/hard-a");
    let second = home.join(".cache/tool/hard-b");
    fs::write(&original, b"shared").unwrap();
    fs::hard_link(&original, &second).unwrap();

    let mut config = ScanConfig::new(roots_for_home(&home).unwrap(), false);
    config.now = SystemTime::now() + Duration::from_secs(8 * 24 * 60 * 60);
    let report = scan(&config, &AtomicBool::new(false), |_| {});
    assert_eq!(report.total_files(), 0);
    assert_eq!(report.stats.skipped_symlinks, 1);
    assert_eq!(report.stats.skipped_hardlinks, 2);
    assert_eq!(fs::read(&sentinel).unwrap(), b"do not touch");
}

#[test]
fn a_pre_cancelled_scan_does_no_work() {
    let (_temp, home) = fixture();
    fs::write(home.join(".cache/tool/file"), b"data").unwrap();
    let cancel = AtomicBool::new(true);
    let report = scan(
        &ScanConfig::new(roots_for_home(&home).unwrap(), true),
        &cancel,
        |_| {},
    );
    assert!(report.cancelled);
    assert_eq!(report.total_files(), 0);
}
