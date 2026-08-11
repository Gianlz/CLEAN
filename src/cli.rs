use std::io;
use std::sync::atomic::AtomicBool;

use crate::report::human_bytes;
use crate::rules::{current_home, default_roots};
use crate::scanner::{ScanConfig, scan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandMode {
    Tui,
    Doctor,
    DryRun { include_recent: bool },
    Help,
    Version,
}

pub fn parse_args<I>(args: I) -> Result<CommandMode, String>
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();
    match args.as_slice() {
        [] => Ok(CommandMode::Tui),
        [arg] if arg == "doctor" => Ok(CommandMode::Doctor),
        [arg] if arg == "scan" || arg == "--dry-run" => Ok(CommandMode::DryRun {
            include_recent: false,
        }),
        [first, second]
            if first == "scan" && (second == "--dry-run" || second == "--include-recent") =>
        {
            Ok(CommandMode::DryRun {
                include_recent: second == "--include-recent",
            })
        }
        [first, second, third]
            if first == "scan"
                && ((second == "--dry-run" && third == "--include-recent")
                    || (third == "--dry-run" && second == "--include-recent")) =>
        {
            Ok(CommandMode::DryRun {
                include_recent: true,
            })
        }
        [arg] if arg == "--help" || arg == "-h" || arg == "help" => Ok(CommandMode::Help),
        [arg] if arg == "--version" || arg == "-V" => Ok(CommandMode::Version),
        _ => Err(format!("unrecognized arguments: {}", args.join(" "))),
    }
}

pub fn print_help() {
    println!(
        "clean — safe macOS cache cleanup\n\n\
         USAGE\n  clean\n  clean doctor\n  clean scan --dry-run [--include-recent]\n\n\
         SAFETY\n  Interactive cleanup is preview-first and requires a typed phrase.\n  No administrator access, recursive directory deletion, or arbitrary paths."
    );
}

pub fn run_doctor() -> Result<(), Box<dyn std::error::Error>> {
    let home = current_home()?;
    println!("CLEAN safety doctor");
    println!("  account home  {}", home.display());
    println!("  privileges    current user only");
    println!("  network       unused");
    println!("  delete mode   manifest files only; never recursive directories");
    for root in default_roots()? {
        let status = match std::fs::read_dir(&root.path) {
            Ok(_) => "readable",
            Err(error) if error.kind() == io::ErrorKind::NotFound => "not present",
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => "permission denied",
            Err(_) => "unavailable",
        };
        println!("  {:<14} {} ({status})", root.label, root.path.display());
    }
    println!("\nDoctor is read-only. No cache data was modified.");
    Ok(())
}

pub fn run_dry_scan(include_recent: bool) -> Result<(), Box<dyn std::error::Error>> {
    let roots = default_roots()?;
    let config = ScanConfig::new(roots, include_recent);
    let cancel = AtomicBool::new(false);
    let report = scan(&config, &cancel, |_| {});
    println!("CLEAN dry run");
    println!(
        "  age policy    {}",
        if include_recent {
            "all ages"
        } else {
            "older than seven days"
        }
    );
    println!("  cache groups  {}", report.groups.len());
    println!("  files         {}", report.total_files());
    println!(
        "  reclaimable   {} estimated allocated",
        human_bytes(report.allocated_bytes())
    );
    println!("  issues        {}", report.issues.len());
    println!("\nLargest groups");
    let mut groups: Vec<_> = report.groups.iter().collect();
    groups.sort_by_key(|group| std::cmp::Reverse(group.allocated_bytes));
    for group in groups.into_iter().take(12) {
        println!(
            "  {:>10}  {:>7} files  {} / {}",
            human_bytes(group.allocated_bytes),
            group.files.len(),
            group.root_label,
            group.name
        );
    }
    println!("\nDry run complete. No cache data was modified.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_known_non_destructive_commands() {
        assert_eq!(parse_args(Vec::<String>::new()).unwrap(), CommandMode::Tui);
        assert_eq!(
            parse_args(vec!["doctor".into()]).unwrap(),
            CommandMode::Doctor
        );
        assert_eq!(
            parse_args(vec!["scan".into(), "--include-recent".into()]).unwrap(),
            CommandMode::DryRun {
                include_recent: true
            }
        );
        assert!(parse_args(vec!["delete".into(), "/".into()]).is_err());
    }
}
