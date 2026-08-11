use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::cleaner::{CleanProgress, clean_selected, confirmation_phrase};
use crate::report::CleanReceipt;
use crate::rules::RootSpec;
use crate::scanner::{ScanConfig, ScanProgress, ScanReport, scan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    Scanning,
    Results,
    Confirm,
    Cleaning,
    Receipt,
}

#[derive(Debug)]
enum WorkerEvent {
    ScanProgress(ScanProgress),
    ScanDone(ScanReport),
    CleanProgress(CleanProgress),
    CleanDone(CleanReceipt, ScanReport),
}

pub struct App {
    pub roots: Vec<RootSpec>,
    pub screen: Screen,
    pub report: Option<ScanReport>,
    pub receipt: Option<CleanReceipt>,
    pub selected: BTreeSet<usize>,
    pub cursor: usize,
    pub include_recent: bool,
    pub confirmation_input: String,
    pub scan_progress: ScanProgress,
    pub clean_progress: CleanProgress,
    pub tick: u64,
    pub show_help: bool,
    pub should_quit: bool,
    pub status: String,
    pub reduce_motion: bool,
    pub no_color: bool,
    worker_rx: Option<Receiver<WorkerEvent>>,
    cancel: Arc<AtomicBool>,
}

impl App {
    pub fn new(roots: Vec<RootSpec>) -> Self {
        let reduce_motion = std::env::var_os("CLEAN_REDUCE_MOTION").is_some();
        let no_color = std::env::var_os("NO_COLOR").is_some()
            || std::env::var("TERM").is_ok_and(|term| term == "dumb");
        let mut app = Self {
            roots,
            screen: Screen::Scanning,
            report: None,
            receipt: None,
            selected: BTreeSet::new(),
            cursor: 0,
            include_recent: false,
            confirmation_input: String::new(),
            scan_progress: ScanProgress::default(),
            clean_progress: CleanProgress::default(),
            tick: 0,
            show_help: false,
            should_quit: false,
            status: "Preparing a read-only scan…".to_string(),
            reduce_motion,
            no_color,
            worker_rx: None,
            cancel: Arc::new(AtomicBool::new(false)),
        };
        app.start_scan();
        app
    }

    pub fn from_report(roots: Vec<RootSpec>, report: ScanReport) -> Self {
        Self {
            roots,
            screen: Screen::Results,
            report: Some(report),
            receipt: None,
            selected: BTreeSet::new(),
            cursor: 0,
            include_recent: false,
            confirmation_input: String::new(),
            scan_progress: ScanProgress::default(),
            clean_progress: CleanProgress::default(),
            tick: 0,
            show_help: false,
            should_quit: false,
            status: "Scan complete".to_string(),
            reduce_motion: true,
            no_color: false,
            worker_rx: None,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start_scan(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.screen = Screen::Scanning;
        self.selected.clear();
        self.cursor = 0;
        self.scan_progress = ScanProgress::default();
        self.status = if self.include_recent {
            "Scanning all cache ages…".to_string()
        } else {
            "Scanning caches older than seven days…".to_string()
        };

        let config = ScanConfig::new(self.roots.clone(), self.include_recent);
        let cancel = Arc::clone(&self.cancel);
        let (tx, rx) = mpsc::channel();
        self.worker_rx = Some(rx);
        thread::spawn(move || {
            let progress_tx = tx.clone();
            let report = scan(&config, &cancel, |progress| {
                let _ = progress_tx.send(WorkerEvent::ScanProgress(progress));
            });
            let _ = tx.send(WorkerEvent::ScanDone(report));
        });
    }

    fn start_clean(&mut self) {
        let Some(report) = self.report.take() else {
            return;
        };
        if self.selected.is_empty() {
            return;
        }
        self.cancel = Arc::new(AtomicBool::new(false));
        self.screen = Screen::Cleaning;
        self.clean_progress = CleanProgress::default();
        self.status = "Removing only revalidated manifest files…".to_string();

        let selected = self.selected.clone();
        let cancel = Arc::clone(&self.cancel);
        let config = ScanConfig::new(self.roots.clone(), self.include_recent);
        let volume_path = self
            .roots
            .first()
            .map(|root| root.home.clone())
            .unwrap_or_else(|| std::path::PathBuf::from("/"));
        let (tx, rx) = mpsc::channel();
        self.worker_rx = Some(rx);
        thread::spawn(move || {
            let progress_tx = tx.clone();
            let mut receipt =
                clean_selected(&report, &selected, &cancel, &volume_path, |progress| {
                    let _ = progress_tx.send(WorkerEvent::CleanProgress(progress));
                });
            let rescan_cancel = AtomicBool::new(false);
            let rescan = scan(&config, &rescan_cancel, |_| {});
            receipt.remaining_files = rescan.total_files();
            receipt.remaining_bytes = rescan.allocated_bytes();
            let _ = tx.send(WorkerEvent::CleanDone(receipt, rescan));
        });
    }

    pub fn poll_worker(&mut self) {
        let mut events = Vec::new();
        if let Some(rx) = &self.worker_rx {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }
        for event in events {
            match event {
                WorkerEvent::ScanProgress(progress) => {
                    self.scan_progress = progress;
                }
                WorkerEvent::ScanDone(report) => {
                    self.status = if report.cancelled {
                        "Scan cancelled safely".to_string()
                    } else {
                        format!("Scan complete · {} cache groups", report.groups.len())
                    };
                    self.report = Some(report);
                    self.screen = Screen::Results;
                    self.worker_rx = None;
                }
                WorkerEvent::CleanProgress(progress) => {
                    self.clean_progress = progress;
                }
                WorkerEvent::CleanDone(receipt, report) => {
                    self.status = if receipt.cancelled {
                        "Cleanup stopped safely · receipt is partial".to_string()
                    } else {
                        "Cleanup verified by a fresh scan".to_string()
                    };
                    self.receipt = Some(receipt);
                    self.report = Some(report);
                    self.screen = Screen::Receipt;
                    self.worker_rx = None;
                    self.selected.clear();
                }
            }
        }
    }

    pub fn advance_tick(&mut self) {
        if !self.reduce_motion {
            self.tick = self.tick.wrapping_add(1);
        }
    }

    pub fn selected_totals(&self) -> (usize, u64) {
        self.report
            .as_ref()
            .map(|report| report.selected_totals(&self.selected))
            .unwrap_or_default()
    }

    pub fn expected_confirmation(&self) -> String {
        confirmation_phrase(self.selected_totals().0)
    }

    pub fn current_group(&self) -> Option<&crate::scanner::CandidateGroup> {
        self.report
            .as_ref()
            .and_then(|report| report.groups.get(self.cursor))
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if self.screen == Screen::Cleaning {
                self.cancel.store(true, Ordering::SeqCst);
                self.status = "Cancelling after the current file…".to_string();
            } else {
                self.cancel.store(true, Ordering::SeqCst);
                self.should_quit = true;
            }
            return;
        }

        if self.show_help {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
            ) {
                self.show_help = false;
            }
            return;
        }
        if key.code == KeyCode::Char('?') {
            self.show_help = true;
            return;
        }

        match self.screen {
            Screen::Scanning => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.cancel.store(true, Ordering::SeqCst);
                    self.should_quit = true;
                }
                _ => {}
            },
            Screen::Results => self.handle_results_key(key.code),
            Screen::Confirm => self.handle_confirm_key(key.code),
            Screen::Cleaning => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.cancel.store(true, Ordering::SeqCst);
                    self.status = "Cancelling after the current file…".to_string();
                }
                _ => {}
            },
            Screen::Receipt => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                KeyCode::Char('r') | KeyCode::Char('s') | KeyCode::Enter => self.start_scan(),
                _ => {}
            },
        }
    }

    fn handle_results_key(&mut self, key: KeyCode) {
        let group_count = self.report.as_ref().map_or(0, |report| report.groups.len());
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = self.cursor.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor + 1 < group_count {
                    self.cursor += 1;
                }
            }
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = group_count.saturating_sub(1),
            KeyCode::Char(' ') => {
                if let Some(group) = self.current_group() {
                    let id = group.id;
                    if !self.selected.remove(&id) {
                        self.selected.insert(id);
                    }
                }
            }
            KeyCode::Char('a') => {
                if self.selected.len() == group_count {
                    self.selected.clear();
                } else if let Some(report) = &self.report {
                    self.selected = report.groups.iter().map(|group| group.id).collect();
                }
            }
            KeyCode::Char('i') => {
                self.include_recent = !self.include_recent;
                self.start_scan();
            }
            KeyCode::Char('r') | KeyCode::Char('s') => self.start_scan(),
            KeyCode::Enter if !self.selected.is_empty() => {
                self.confirmation_input.clear();
                self.screen = Screen::Confirm;
            }
            _ => {}
        }
    }

    fn handle_confirm_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.confirmation_input.clear();
                self.screen = Screen::Results;
            }
            KeyCode::Backspace => {
                self.confirmation_input.pop();
            }
            KeyCode::Enter => {
                if self.confirmation_input == self.expected_confirmation() {
                    self.start_clean();
                } else {
                    self.status = "Confirmation did not match · nothing was deleted".to_string();
                }
            }
            KeyCode::Char(character)
                if !character.is_control() && self.confirmation_input.len() < 80 =>
            {
                self.confirmation_input.push(character);
            }
            _ => {}
        }
    }
}
