use std::error::Error;
use std::io::{self, IsTerminal};
use std::time::Duration;

use clean_tui::app::App;
use clean_tui::cli::{CommandMode, parse_args, print_help, run_doctor, run_dry_scan};
use clean_tui::rules::default_roots;
use clean_tui::ui;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

fn main() {
    if let Err(error) = run() {
        eprintln!("clean: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mode = parse_args(std::env::args().skip(1))
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    match mode {
        CommandMode::Help => print_help(),
        CommandMode::Version => println!("clean {}", env!("CARGO_PKG_VERSION")),
        CommandMode::Doctor => run_doctor()?,
        CommandMode::DryRun { include_recent } => run_dry_scan(include_recent)?,
        CommandMode::Tui => {
            if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
                return Err(
                    "interactive mode needs a terminal; use `clean scan --dry-run` for text output"
                        .into(),
                );
            }
            run_tui(App::new(default_roots()?))?;
        }
    }
    Ok(())
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(io::stdout(), EnterAlternateScreen, Hide) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
}

fn install_panic_restorer() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        previous(info);
    }));
}

fn run_tui(mut app: App) -> io::Result<()> {
    install_panic_restorer();
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    while !app.should_quit {
        app.poll_worker();
        app.advance_tick();
        terminal.draw(|frame| ui::draw(frame, &app))?;
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            app.handle_key(key);
        }
    }
    Ok(())
}
