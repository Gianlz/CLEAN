use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ItemIssue {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Clone, Debug, Default)]
pub struct CleanReceipt {
    pub requested_files: usize,
    pub deleted_files: usize,
    pub deleted_logical_bytes: u64,
    pub deleted_allocated_bytes: u64,
    pub skipped: Vec<ItemIssue>,
    pub failed: Vec<ItemIssue>,
    pub cancelled: bool,
    pub available_before: Option<u64>,
    pub available_after: Option<u64>,
    pub remaining_files: usize,
    pub remaining_bytes: u64,
}

impl CleanReceipt {
    pub fn available_delta(&self) -> Option<i128> {
        Some(i128::from(self.available_after?) - i128::from(self.available_before?))
    }
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else if value >= 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_bytes_readably() {
        assert_eq!(human_bytes(999), "999 B");
        assert_eq!(human_bytes(1_500), "1.50 KB");
        assert_eq!(human_bytes(15_000_000), "15.0 MB");
    }
}
