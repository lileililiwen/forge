use anyhow::Result;
use clap::Parser;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

mod demo;
mod exporter;
mod model;
mod parser;
mod ui;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run a simulated build profiling demo
    #[arg(long)]
    demo: bool,

    /// Arguments to pass to cargo (e.g. "build", "check", "--release")
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    cargo_args: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Default to "check" if no arguments are provided
    let cargo_args = if args.cargo_args.is_empty() {
        vec!["check".to_string()]
    } else {
        args.cargo_args
    };

    // Setup terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Shared state
    let state = Arc::new(Mutex::new(model::BuildState::new()));

    // Spawn Cargo subprocess in background
    let state_clone = Arc::clone(&state);
    let is_demo = args.demo;
    let cargo_handle = tokio::spawn(async move {
        if is_demo {
            demo::run_demo_simulation(state_clone).await
        } else {
            // Run cargo in the current directory
            let dir = ".";
            parser::run_cargo(dir, &cargo_args, state_clone).await
        }
    });

    let mut scroll_offset = 0;
    let mut selected_index = 0;
    let mut search_query = String::new();
    let mut search_mode = false;
    let mut show_cached = true;
    let mut show_only_bottlenecks = false;
    let mut diagnostics_scroll = 0;

    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(50);
    let mut should_quit = false;

    while !should_quit {
        // Clamp selected_index
        {
            let state_guard = state.lock().await;
            let filtered_len = ui::get_filtered_crates(&state_guard, &search_query, show_cached, show_only_bottlenecks).len();
            if filtered_len > 0 {
                selected_index = selected_index.min(filtered_len - 1);
            } else {
                selected_index = 0;
            }
        }

        // Draw TUI frame
        {
            let state_guard = state.lock().await;
            terminal.draw(|f| {
                ui::draw_ui(
                    f,
                    &state_guard,
                    selected_index,
                    &mut scroll_offset,
                    &search_query,
                    search_mode,
                    show_cached,
                    show_only_bottlenecks,
                    diagnostics_scroll,
                );
            })?;
        }

        // Adjust poll timeout to match our tick rate
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                // Respond to key presses (Press or Repeat)
                if key.kind == crossterm::event::KeyEventKind::Press || key.kind == crossterm::event::KeyEventKind::Repeat {
                    if search_mode {
                        match key.code {
                            crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Enter => {
                                search_mode = false;
                            }
                            crossterm::event::KeyCode::Backspace => {
                                search_query.pop();
                                selected_index = 0;
                                diagnostics_scroll = 0;
                            }
                            crossterm::event::KeyCode::Char(c) => {
                                search_query.push(c);
                                selected_index = 0;
                                diagnostics_scroll = 0;
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Char('Q') | crossterm::event::KeyCode::Esc => {
                                should_quit = true;
                            }
                            crossterm::event::KeyCode::Up => {
                                selected_index = selected_index.saturating_sub(1);
                                diagnostics_scroll = 0;
                            }
                            crossterm::event::KeyCode::Down => {
                                selected_index = selected_index.saturating_add(1);
                                diagnostics_scroll = 0;
                            }
                            crossterm::event::KeyCode::Char('/') | crossterm::event::KeyCode::Char('s') | crossterm::event::KeyCode::Char('S') => {
                                search_mode = true;
                            }
                            crossterm::event::KeyCode::Char('c') | crossterm::event::KeyCode::Char('C') => {
                                show_cached = !show_cached;
                                selected_index = 0;
                                diagnostics_scroll = 0;
                            }
                            crossterm::event::KeyCode::Char('b') | crossterm::event::KeyCode::Char('B') => {
                                show_only_bottlenecks = !show_only_bottlenecks;
                                selected_index = 0;
                                diagnostics_scroll = 0;
                            }
                            crossterm::event::KeyCode::PageUp => {
                                diagnostics_scroll = diagnostics_scroll.saturating_sub(1);
                            }
                            crossterm::event::KeyCode::PageDown => {
                                diagnostics_scroll = diagnostics_scroll.saturating_add(1);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        // Auto-quit if cargo is finished and we are not in a TTY (e.g. CI/CD or script runner)
        use std::io::IsTerminal;
        if cargo_handle.is_finished() && io::stdout().is_terminal() == false {
            should_quit = true;
        }
    }

    // Clean up terminal and restore system defaults
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Print final result to stdout
    let final_state = state.lock().await;
    let build_duration = Instant::now().duration_since(final_state.start_time);
    
    println!("\n=== Cargo Build Scan Summary ===");
    println!("Total Elapsed Time: {:.2}s", build_duration.as_secs_f32());
    println!("Total Crates Audited: {}", final_state.crates.len());
    
    if let Some(success) = final_state.success {
        if success {
            println!("Compilation Status: \x1b[32mSuccess\x1b[0m");
        } else {
            println!("Compilation Status: \x1b[31mFailed\x1b[0m");
        }
    } else {
        println!("Compilation Status: \x1b[33mInterrupted\x1b[0m");
    }

    // Catch any runner errors
    match cargo_handle.await {
        Ok(Err(e)) => println!("Cargo runner error: {:?}", e),
        Err(e) => println!("Task join error: {:?}", e),
        _ => {}
    }

    // Export build scan HTML and JSON reports
    if let Err(e) = exporter::export_build_scan(&final_state) {
        println!("Warning: Failed to export build scan: {:?}", e);
    }

    Ok(())
}
