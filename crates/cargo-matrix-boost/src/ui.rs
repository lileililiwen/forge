use crate::model::{JobStatus, MatrixJob};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct UiContext<'a> {
    pub project_name: &'a str,
    pub jobs: &'a [MatrixJob],
    pub table_state: &'a mut TableState,
    pub log_scroll: u16,
    pub total_jobs: usize,
    pub finished_count: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub running_count: usize,
    pub is_done: bool,
}

pub fn run_tui(
    project_name: String,
    jobs: Arc<Mutex<Vec<MatrixJob>>>,
    runner_finished: Arc<Mutex<bool>>,
) -> io::Result<()> {
    // Setup terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut table_state = TableState::default();
    table_state.select(Some(0));

    let mut log_scroll: u16 = 0;
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(50);
    let mut should_quit = false;

    while !should_quit {
        let (current_jobs, total_jobs, finished_count, success_count, failed_count, running_count) = {
            let guard = jobs.lock().unwrap();
            let total = guard.len();
            let success = guard
                .iter()
                .filter(|j| j.status == JobStatus::Success)
                .count();
            let failed = guard
                .iter()
                .filter(|j| j.status == JobStatus::Failed)
                .count();
            let running = guard
                .iter()
                .filter(|j| j.status == JobStatus::Running)
                .count();
            (
                guard.clone(),
                total,
                success + failed,
                success,
                failed,
                running,
            )
        };

        // Clamp table selection
        if total_jobs > 0 {
            let sel = table_state.selected().unwrap_or(0).min(total_jobs - 1);
            table_state.select(Some(sel));
        }

        let is_done = *runner_finished.lock().unwrap();

        use std::io::IsTerminal;
        if is_done && !std::io::stdout().is_terminal() {
            should_quit = true;
        }

        let mut ctx = UiContext {
            project_name: &project_name,
            jobs: &current_jobs,
            table_state: &mut table_state,
            log_scroll,
            total_jobs,
            finished_count,
            success_count,
            failed_count,
            running_count,
            is_done,
        };

        terminal.draw(|f| {
            draw_ui(f, &mut ctx);
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key)
                    if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat =>
                {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                            should_quit = true;
                        }
                        KeyCode::Up => {
                            let i = table_state.selected().unwrap_or(0);
                            if i > 0 {
                                table_state.select(Some(i - 1));
                                log_scroll = 0; // Reset scroll on change
                            }
                        }
                        KeyCode::Down => {
                            let i = table_state.selected().unwrap_or(0);
                            if total_jobs > 0 && i < total_jobs - 1 {
                                table_state.select(Some(i + 1));
                                log_scroll = 0; // Reset scroll on change
                            }
                        }
                        KeyCode::PageUp => {
                            log_scroll = log_scroll.saturating_sub(5);
                        }
                        KeyCode::PageDown => {
                            log_scroll = log_scroll.saturating_add(5);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    // Restore terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn draw_ui(f: &mut Frame, ctx: &mut UiContext) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Header Summary
            Constraint::Min(8),     // Matrix Jobs Table
            Constraint::Length(12), // Selected Job Logs
            Constraint::Length(1),  // Key Info Footer
        ])
        .split(f.area());

    // 1. Header Summary Panel
    let progress_ratio = if ctx.total_jobs > 0 {
        ctx.finished_count as f32 / ctx.total_jobs as f32
    } else {
        0.0
    };
    let progress_percentage = (progress_ratio * 100.0) as usize;
    let progress_bar_width = (chunks[0].width as usize).saturating_sub(25).max(10);
    let filled_width = (progress_bar_width as f32 * progress_ratio) as usize;
    let progress_bar: String = format!(
        "[{}{}] {}%",
        "█".repeat(filled_width),
        "░".repeat(progress_bar_width.saturating_sub(filled_width)),
        progress_percentage
    );

    let status_str = if ctx.is_done {
        Span::styled(
            "FINISHED",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    } else if ctx.running_count > 0 {
        Span::styled(
            "RUNNING",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("PENDING", Style::default().fg(Color::DarkGray))
    };

    let summary_lines = vec![
        Line::from(vec![
            Span::styled("📦 Crate: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                ctx.project_name,
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::White),
            ),
            Span::styled("  |  Status: ", Style::default().fg(Color::DarkGray)),
            status_str,
        ]),
        Line::from(vec![
            Span::styled("📊 Jobs: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", ctx.total_jobs),
                Style::default().fg(Color::White),
            ),
            Span::styled(" Total   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", ctx.success_count),
                Style::default().fg(Color::Green),
            ),
            Span::styled(" Success   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", ctx.failed_count),
                Style::default().fg(Color::Red),
            ),
            Span::styled(" Failed   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", ctx.running_count),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" Running", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("⏳ Progress: ", Style::default().fg(Color::DarkGray)),
            Span::styled(progress_bar, Style::default().fg(Color::Magenta)),
        ]),
    ];

    let header_block = Block::default()
        .title(" Cargo Matrix Check Boost 🛡️ ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    f.render_widget(Paragraph::new(summary_lines).block(header_block), chunks[0]);

    // 2. Matrix Jobs Table
    let selected_index = ctx.table_state.selected().unwrap_or(0);
    let selected_job = ctx.jobs.get(selected_index);

    let header_cells = ["ID", "Toolchain", "Features", "Status", "Duration"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(header_cells).style(Style::default().bg(Color::Rgb(30, 30, 30)));

    let rows: Vec<Row> = ctx
        .jobs
        .iter()
        .map(|job| {
            let id_str = format!("{}", job.id);
            let tc_str = job.toolchain.as_deref().unwrap_or("default");
            let feats_str = job.display_features();

            let status_cell = match job.status {
                JobStatus::Pending => {
                    Cell::from("Pending").style(Style::default().fg(Color::DarkGray))
                }
                JobStatus::Running => Cell::from("Running").style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                JobStatus::Success => {
                    Cell::from("Success").style(Style::default().fg(Color::Green))
                }
                JobStatus::Failed => Cell::from("Failed")
                    .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            };

            let duration_str = if job.duration_ms == 0 {
                "-".to_string()
            } else {
                format!("{:.2}s", job.duration_ms as f64 / 1000.0)
            };

            Row::new(vec![
                Cell::from(id_str),
                Cell::from(tc_str.to_string()),
                Cell::from(feats_str),
                status_cell,
                Cell::from(duration_str),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Length(15),
            Constraint::Min(30),
            Constraint::Length(12),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(" Matrix Jobs ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(50, 50, 50))
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, chunks[1], ctx.table_state);

    // 3. Log Viewport Panel
    let log_content = if let Some(job) = selected_job {
        if job.status == JobStatus::Pending {
            "Job is waiting to be processed...".to_string()
        } else if job.log.is_empty() {
            "Executing command...".to_string()
        } else {
            job.log.clone()
        }
    } else {
        "No job selected.".to_string()
    };

    let log_block = Block::default()
        .title(" Selected Job Output logs ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let log_lines: Vec<Line> = log_content
        .lines()
        .map(|line| {
            if line.contains("error[E") || line.contains("error:") {
                Line::from(Span::styled(line, Style::default().fg(Color::Red)))
            } else if line.contains("warning:") {
                Line::from(Span::styled(line, Style::default().fg(Color::Yellow)))
            } else if line.contains("Compiling") || line.contains("Finished") {
                Line::from(Span::styled(line, Style::default().fg(Color::Cyan)))
            } else if line.contains("test result: ok")
                || line.contains("test tests::") && line.contains("... ok")
            {
                Line::from(Span::styled(line, Style::default().fg(Color::Green)))
            } else {
                Line::from(Span::styled(line, Style::default().fg(Color::Gray)))
            }
        })
        .collect();

    let logs_para = Paragraph::new(log_lines)
        .block(log_block)
        .wrap(Wrap { trim: false })
        .scroll((ctx.log_scroll, 0));

    f.render_widget(logs_para, chunks[2]);

    // 4. Instructions Footer
    let footer_text = if ctx.is_done {
        "q: Quit | Esc: Quit | Up/Down: Navigate | PageUp/PageDown: Scroll Logs | Report generated! Check target/matrix/"
    } else {
        "q: Quit | Esc: Quit | Up/Down: Navigate | PageUp/PageDown: Scroll Logs | Running matrix..."
    };
    let footer =
        Paragraph::new(footer_text).style(Style::default().fg(Color::Black).bg(Color::DarkGray));
    f.render_widget(footer, chunks[3]);
}
