use std::cmp::max;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Sparkline, Wrap,
};

use crate::app::{App, Screen};
use crate::report::human_bytes;
use crate::theme::{Theme, gradient};

const LOGO: [&str; 5] = [
    " ██████╗██╗     ███████╗ █████╗ ███╗   ██╗",
    "██╔════╝██║     ██╔════╝██╔══██╗████╗  ██║",
    "██║     ██║     █████╗  ███████║██╔██╗ ██║",
    "██║     ██║     ██╔══╝  ██╔══██║██║╚██╗██║",
    "╚██████╗███████╗███████╗██║  ██║██║ ╚████║",
];

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let theme = Theme::for_mode(app.no_color);
    frame.render_widget(Block::default().style(Style::default().bg(theme.bg)), area);

    if area.width < 70 || area.height < 20 {
        draw_too_small(frame, area, theme);
        return;
    }

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);
    draw_header(frame, vertical[0], app, theme);
    match app.screen {
        Screen::Scanning => draw_scanning(frame, vertical[1], app, theme),
        Screen::Results | Screen::Confirm => draw_results(frame, vertical[1], app, theme),
        Screen::Cleaning => draw_cleaning(frame, vertical[1], app, theme),
        Screen::Receipt => draw_receipt(frame, vertical[1], app, theme),
    }
    draw_footer(frame, vertical[2], app, theme);

    if app.screen == Screen::Confirm {
        draw_confirmation(frame, area, app, theme);
    }
    if app.show_help {
        draw_help(frame, area, theme);
    }
}

fn panel<'a>(title: impl Into<Line<'a>>, theme: Theme) -> Block<'a> {
    Block::default()
        .title(title)
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.muted))
        .style(Style::default().bg(theme.panel).fg(theme.text))
}

fn draw_header(frame: &mut Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(50), Constraint::Length(27)])
        .split(area);
    let logo = LOGO
        .iter()
        .enumerate()
        .map(|(index, row)| {
            Line::from(Span::styled(
                *row,
                Style::default()
                    .fg(gradient(app.tick, index, app.no_color))
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(logo), columns[0]);

    let pulse = if app.reduce_motion {
        "◆"
    } else {
        ["·", "✦", "◆", "✦"][(app.tick as usize / 2) % 4]
    };
    let age = if app.include_recent {
        "ALL AGES"
    } else {
        "7+ DAYS"
    };
    let badge = vec![
        Line::from(vec![
            Span::styled(format!(" {pulse} "), Style::default().fg(theme.lime)),
            Span::styled("SAFETY LOCKED", theme.title()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("scope  ", Style::default().fg(theme.muted)),
            Span::styled("USER CACHES", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("age    ", Style::default().fg(theme.muted)),
            Span::styled(age, Style::default().fg(theme.warning)),
        ]),
        Line::from(vec![
            Span::styled("mode   ", Style::default().fg(theme.muted)),
            Span::styled("PREVIEW FIRST", Style::default().fg(theme.cyan)),
        ]),
    ];
    frame.render_widget(Paragraph::new(badge).alignment(Alignment::Left), columns[1]);
}

fn draw_scanning(frame: &mut Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let inset = centered(area, 76, 16);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(5),
        ])
        .split(inset);
    let shimmer = ["◜", "◝", "◞", "◟"][(app.tick as usize / 2) % 4];
    let heading = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(format!("{shimmer} "), Style::default().fg(theme.violet)),
            Span::styled("READ-ONLY DEEP SCAN", theme.title()),
        ]),
        Line::from(Span::styled(
            "Mapping old cache files without changing a single byte",
            Style::default().fg(theme.muted),
        )),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(heading, rows[0]);

    let ratio = if app.reduce_motion {
        0.7
    } else {
        ((app.tick % 80) as f64 / 80.0).clamp(0.08, 1.0)
    };
    let gauge = Gauge::default()
        .block(panel(" SCAN BEAM ", theme))
        .gauge_style(Style::default().fg(theme.cyan).bg(theme.panel))
        .ratio(ratio)
        .label(format!(
            "{} visited  ·  {} eligible  ·  {}",
            app.scan_progress.visited,
            app.scan_progress.eligible,
            human_bytes(app.scan_progress.allocated_bytes)
        ));
    frame.render_widget(gauge, rows[1]);

    let wave = (0..64)
        .map(|index| {
            let x = (index as u64 + app.tick) % 20;
            if x < 10 { x + 2 } else { 22 - x }
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Sparkline::default()
            .block(panel(" LIVE CACHE SIGNAL ", theme))
            .data(&wave)
            .style(Style::default().fg(theme.violet)),
        rows[2],
    );
}

fn draw_results(frame: &mut Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let Some(report) = &app.report else {
        return;
    };
    if report.groups.is_empty() {
        let content = vec![
            Line::from(Span::styled("✦", Style::default().fg(theme.lime))),
            Line::from(Span::styled(
                "YOUR OLD CACHES ARE ALREADY LIGHT",
                theme.title(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Nothing eligible was found under the seven-day safety window.",
                Style::default().fg(theme.muted),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press i to include recent cache files · r to scan again",
                Style::default().fg(theme.warning),
            )),
        ];
        frame.render_widget(
            Paragraph::new(content)
                .block(panel(" CLEAN STATE ", theme))
                .alignment(Alignment::Center),
            centered(area, 78, 12),
        );
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(8)])
        .split(area);
    let (selected_files, selected_bytes) = app.selected_totals();
    let summary = Paragraph::new(Line::from(vec![
        Span::styled(
            "  FOUND  ",
            Style::default()
                .fg(theme.bg)
                .bg(theme.violet)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                " {} files · {}  ",
                report.total_files(),
                human_bytes(report.allocated_bytes())
            ),
            Style::default().fg(theme.text),
        ),
        Span::styled(
            "  SELECTED  ",
            Style::default()
                .fg(theme.bg)
                .bg(theme.cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {selected_files} files · {} ", human_bytes(selected_bytes)),
            Style::default().fg(if selected_files > 0 {
                theme.lime
            } else {
                theme.muted
            }),
        ),
        Span::styled(
            format!("  {} scan issues", report.issues.len()),
            Style::default().fg(if report.issues.is_empty() {
                theme.muted
            } else {
                theme.warning
            }),
        ),
    ]))
    .block(panel(" RECOVERY MAP ", theme))
    .alignment(Alignment::Center);
    frame.render_widget(summary, rows[0]);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(rows[1]);
    let items = report
        .groups
        .iter()
        .map(|group| {
            let mark = if app.selected.contains(&group.id) {
                "[◆]"
            } else {
                "[ ]"
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{mark} "),
                    Style::default().fg(if app.selected.contains(&group.id) {
                        theme.lime
                    } else {
                        theme.muted
                    }),
                ),
                Span::styled(
                    format!("{:<28}", truncate(&group.name, 28)),
                    Style::default().fg(theme.text),
                ),
                Span::styled(
                    format!(
                        " {:>9}  {:>6} files",
                        human_bytes(group.allocated_bytes),
                        group.files.len()
                    ),
                    Style::default().fg(theme.muted),
                ),
            ]))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(Some(app.cursor));
    let list = List::new(items)
        .block(panel(" CACHE GROUPS · SPACE TO SELECT ", theme))
        .highlight_style(theme.selected())
        .highlight_symbol("▸ ");
    frame.render_stateful_widget(list, columns[0], &mut state);

    let detail = if let Some(group) = app.current_group() {
        let share = if report.allocated_bytes() == 0 {
            0.0
        } else {
            group.allocated_bytes as f64 / report.allocated_bytes() as f64
        };
        let body = vec![
            Line::from(Span::styled(&group.name, theme.title())),
            Line::from(Span::styled(
                &group.root_label,
                Style::default().fg(theme.violet),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("estimated  ", Style::default().fg(theme.muted)),
                Span::styled(
                    human_bytes(group.allocated_bytes),
                    Style::default().fg(theme.lime),
                ),
            ]),
            Line::from(vec![
                Span::styled("logical    ", Style::default().fg(theme.muted)),
                Span::styled(
                    human_bytes(group.logical_bytes),
                    Style::default().fg(theme.text),
                ),
            ]),
            Line::from(vec![
                Span::styled("files      ", Style::default().fg(theme.muted)),
                Span::styled(
                    group.files.len().to_string(),
                    Style::default().fg(theme.text),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                format!("share of candidates  {:>5.1}%", share * 100.0),
                Style::default().fg(theme.muted),
            )),
            progress_bar(share, 24, theme),
            Line::from(""),
            Line::from(Span::styled(
                "Only unchanged regular files from the scan manifest can be removed.",
                Style::default().fg(theme.warning),
            )),
            Line::from(""),
            Line::from(Span::styled(
                if report.issues.is_empty() {
                    "No scan access issues"
                } else {
                    "Some paths were unreadable and are excluded"
                },
                Style::default().fg(if report.issues.is_empty() {
                    theme.lime
                } else {
                    theme.warning
                }),
            )),
        ];
        Paragraph::new(body)
            .block(panel(" INSPECTOR ", theme))
            .wrap(Wrap { trim: true })
    } else {
        Paragraph::new("No cache group selected").block(panel(" INSPECTOR ", theme))
    };
    frame.render_widget(detail, columns[1]);
}

fn draw_confirmation(frame: &mut Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let popup = centered(area, 70, 16);
    frame.render_widget(Clear, popup);
    let expected = app.expected_confirmation();
    let (files, bytes) = app.selected_totals();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(popup.inner(ratatui::layout::Margin {
            horizontal: 2,
            vertical: 1,
        }));
    frame.render_widget(panel(" IRREVERSIBLE GATE ", theme), popup);
    let warning = Paragraph::new(vec![
        Line::from(Span::styled(
            "PERMANENT DELETION",
            Style::default()
                .fg(theme.danger)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!(
            "{files} scanned files · {} estimated allocated",
            human_bytes(bytes)
        )),
        Line::from("No undo · no Trash · changed files will be skipped"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Type exactly  ", Style::default().fg(theme.muted)),
            Span::styled(
                &expected,
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(warning, rows[0]);
    let matches = app.confirmation_input == expected;
    frame.render_widget(
        Paragraph::new(format!(" {}", app.confirmation_input))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(if matches {
                        theme.lime
                    } else {
                        theme.danger
                    })),
            )
            .style(Style::default().fg(theme.text)),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new("Enter confirms · Esc returns with nothing deleted")
            .style(Style::default().fg(theme.muted))
            .alignment(Alignment::Center),
        rows[2],
    );
}

fn draw_cleaning(frame: &mut Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let box_area = centered(area, 78, 15);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Min(4),
        ])
        .split(box_area);
    let percent = if app.clean_progress.total == 0 {
        0.0
    } else {
        app.clean_progress.completed as f64 / app.clean_progress.total as f64
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("PRECISION CLEAN IN PROGRESS", theme.title())),
            Line::from(""),
            Line::from(Span::styled(
                "Every file is being revalidated immediately before removal",
                Style::default().fg(theme.muted),
            )),
        ])
        .alignment(Alignment::Center),
        rows[0],
    );
    frame.render_widget(
        Gauge::default()
            .block(panel(" VERIFIED MANIFEST ", theme))
            .ratio(percent.clamp(0.0, 1.0))
            .gauge_style(Style::default().fg(theme.lime).bg(theme.panel))
            .label(format!(
                "{:.0}% · {}",
                percent * 100.0,
                app.clean_progress.current_group
            )),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("completed  ", Style::default().fg(theme.muted)),
                Span::styled(
                    format!(
                        "{} / {}",
                        app.clean_progress.completed, app.clean_progress.total
                    ),
                    Style::default().fg(theme.text),
                ),
            ]),
            Line::from(vec![
                Span::styled("deleted    ", Style::default().fg(theme.muted)),
                Span::styled(
                    app.clean_progress.deleted.to_string(),
                    Style::default().fg(theme.lime),
                ),
            ]),
            Line::from(vec![
                Span::styled("reclaimed  ", Style::default().fg(theme.muted)),
                Span::styled(
                    human_bytes(app.clean_progress.deleted_bytes),
                    Style::default().fg(theme.lime),
                ),
            ]),
            Line::from(Span::styled(
                "Esc or Ctrl-C stops safely after the current file",
                Style::default().fg(theme.warning),
            )),
        ])
        .block(panel(" LIVE RECEIPT ", theme)),
        rows[2],
    );
}

fn draw_receipt(frame: &mut Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let Some(receipt) = &app.receipt else {
        return;
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(8)])
        .split(area);
    let headline = if receipt.cancelled {
        "CLEANUP STOPPED SAFELY"
    } else {
        "CLEANUP VERIFIED"
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                if receipt.cancelled { "◇" } else { "✦" },
                Style::default().fg(if receipt.cancelled {
                    theme.warning
                } else {
                    theme.lime
                }),
            )),
            Line::from(Span::styled(headline, theme.title())),
            Line::from(Span::styled(
                "A fresh scan completed after the operation",
                Style::default().fg(theme.muted),
            )),
        ])
        .alignment(Alignment::Center),
        rows[0],
    );
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(rows[1]);
    let delta = receipt
        .available_delta()
        .map(|value| {
            if value >= 0 {
                human_bytes(value as u64)
            } else {
                format!("-{}", human_bytes((-value) as u64))
            }
        })
        .unwrap_or_else(|| "unavailable".to_string());
    frame.render_widget(
        Paragraph::new(vec![
            metric_line(
                "files removed",
                receipt.deleted_files.to_string(),
                theme.lime,
                theme,
            ),
            metric_line(
                "estimated removed",
                human_bytes(receipt.deleted_allocated_bytes),
                theme.lime,
                theme,
            ),
            metric_line(
                "logical bytes",
                human_bytes(receipt.deleted_logical_bytes),
                theme.text,
                theme,
            ),
            metric_line("volume delta", delta, theme.cyan, theme),
            Line::from(""),
            metric_line(
                "skipped safely",
                receipt.skipped.len().to_string(),
                theme.warning,
                theme,
            ),
            metric_line(
                "failed",
                receipt.failed.len().to_string(),
                theme.danger,
                theme,
            ),
        ])
        .block(panel(" RECOVERED ", theme)),
        columns[0],
    );
    let mut verification = vec![
        metric_line(
            "requested",
            receipt.requested_files.to_string(),
            theme.text,
            theme,
        ),
        metric_line(
            "still eligible",
            receipt.remaining_files.to_string(),
            theme.text,
            theme,
        ),
        metric_line(
            "still reclaimable",
            human_bytes(receipt.remaining_bytes),
            theme.violet,
            theme,
        ),
        Line::from(""),
        Line::from(Span::styled(
            if receipt.failed.is_empty() && receipt.skipped.is_empty() {
                "Every requested manifest entry was handled."
            } else {
                "Some entries were preserved. Nothing outside the manifest was touched."
            },
            Style::default().fg(theme.muted),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter or r to scan again",
            Style::default().fg(theme.cyan),
        )),
    ];
    for issue in receipt.skipped.iter().chain(receipt.failed.iter()).take(2) {
        verification.push(Line::from(Span::styled(
            format!(
                "• {} · {}",
                truncate(&issue.path.to_string_lossy(), 22),
                truncate(&issue.reason, 28)
            ),
            Style::default().fg(theme.warning),
        )));
    }
    frame.render_widget(
        Paragraph::new(verification)
            .block(panel(" POST-CLEAN CHECK ", theme))
            .wrap(Wrap { trim: true }),
        columns[1],
    );
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let keys = match app.screen {
        Screen::Scanning => "q quit safely   ? help",
        Screen::Results => {
            "↑↓/jk move   Space select   a all   Enter continue   i age   r rescan   ? help   q quit"
        }
        Screen::Confirm => "type the phrase   Enter confirm   Esc cancel",
        Screen::Cleaning => "Esc/Ctrl-C stop safely   ? help",
        Screen::Receipt => "Enter/r rescan   ? help   q quit",
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(keys, Style::default().fg(theme.muted))),
            Line::from(Span::styled(&app.status, Style::default().fg(theme.cyan))),
        ])
        .alignment(Alignment::Center),
        area,
    );
}

fn draw_help(frame: &mut Frame<'_>, area: Rect, theme: Theme) {
    let popup = centered(area, 70, 20);
    frame.render_widget(Clear, popup);
    let help = vec![
        Line::from(Span::styled("KEYBOARD", theme.title())),
        Line::from(""),
        Line::from("↑ ↓ / j k     move through cache groups"),
        Line::from("Space         select or deselect a group"),
        Line::from("a             select all or clear all"),
        Line::from("Enter         continue or scan again"),
        Line::from("i             include or preserve recent files"),
        Line::from("r / s         run a new read-only scan"),
        Line::from("Esc           back, cancel, or stop safely"),
        Line::from("q             quit"),
        Line::from("Ctrl-C        restore terminal; stop between files"),
        Line::from(""),
        Line::from(Span::styled("SAFETY CONTRACT", theme.title())),
        Line::from("Only allowlisted user cache roots · no sudo · no symlink following"),
        Line::from("No recursive directory deletion · every file revalidated"),
        Line::from(""),
        Line::from(Span::styled(
            "Press ?, Esc, or q to close help",
            Style::default().fg(theme.warning),
        )),
    ];
    frame.render_widget(
        Paragraph::new(help)
            .block(panel(" CLEAN FIELD GUIDE ", theme))
            .alignment(Alignment::Center),
        popup,
    );
}

fn draw_too_small(frame: &mut Frame<'_>, area: Rect, theme: Theme) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("CLEAN", theme.title())),
            Line::from(""),
            Line::from("Terminal too small for safe review."),
            Line::from(format!(
                "Current {}×{} · minimum 70×20",
                area.width, area.height
            )),
            Line::from("Resize the terminal or press q to quit."),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true }),
        centered(area, area.width.saturating_sub(4), 7),
    );
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        value.to_string()
    } else {
        let keep = max(1, width.saturating_sub(1));
        format!("{}…", value.chars().take(keep).collect::<String>())
    }
}

fn progress_bar(ratio: f64, width: usize, theme: Theme) -> Line<'static> {
    let filled = (ratio.clamp(0.0, 1.0) * width as f64).round() as usize;
    Line::from(vec![
        Span::styled("━".repeat(filled), Style::default().fg(theme.violet)),
        Span::styled(
            "─".repeat(width.saturating_sub(filled)),
            Style::default().fg(theme.muted),
        ),
    ])
}

fn metric_line(
    label: &str,
    value: String,
    color: ratatui::style::Color,
    theme: Theme,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<20}"), Style::default().fg(theme.muted)),
        Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::roots_for_home;
    use crate::scanner::ScanReport;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn renders_empty_and_compact_states() {
        let roots = roots_for_home(std::path::Path::new("/Users/tester")).unwrap();
        let app = App::from_report(roots, ScanReport::default());
        for (width, height) in [(120, 36), (76, 22)] {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|frame| draw(frame, &app)).unwrap();
            let content = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(content.contains("CLEAN STATE"));
        }
    }

    #[test]
    fn renders_resize_guard() {
        let roots = roots_for_home(std::path::Path::new("/Users/tester")).unwrap();
        let app = App::from_report(roots, ScanReport::default());
        let backend = TestBackend::new(50, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let content = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(content.contains("Terminal too small"));
    }
}
