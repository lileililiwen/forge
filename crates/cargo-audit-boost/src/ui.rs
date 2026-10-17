use crate::model::{BuildState, BuildStatus};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};

pub fn get_filtered_crates(
    state: &BuildState,
    search_query: &str,
    show_cached: bool,
    show_only_bottlenecks: bool,
) -> Vec<String> {
    let bottlenecks = state.get_slowest_crates(3);
    let bottleneck_names: std::collections::HashSet<String> = bottlenecks.iter().map(|(n, _)| n.clone()).collect();
    
    state.crate_order
        .iter()
        .filter(|name| {
            if let Some(c) = state.crates.get(*name) {
                if !show_cached && c.cached {
                    return false;
                }
                if show_only_bottlenecks && !bottleneck_names.contains(&c.name) {
                    return false;
                }
                if !search_query.is_empty() && !c.name.to_lowercase().contains(&search_query.to_lowercase()) {
                    return false;
                }
                true
            } else {
                false
            }
        })
        .cloned()
        .collect()
}

fn strip_ansi_codes(input: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\x1b' {
            in_escape = true;
            i += 1;
            continue;
        }
        if in_escape {
            if chars[i].is_ascii_alphabetic() {
                in_escape = false;
            }
            i += 1;
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

pub fn draw_ui(
    f: &mut Frame,
    state: &BuildState,
    selected_index: usize,
    scroll_offset: &mut usize,
    search_query: &str,
    search_mode: bool,
    show_cached: bool,
    show_only_bottlenecks: bool,
    diagnostics_scroll: usize,
) {
    let now = Instant::now();
    let total_duration = if let Some(_success) = state.success {
        state
            .crates
            .values()
            .filter_map(|c| c.end_time)
            .max()
            .unwrap_or(now)
            .duration_since(state.start_time)
    } else {
        now.duration_since(state.start_time)
    };

    // Split screen: Header (4 lines), Main (remaining), Footer (1 line)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(f.size());

    // 1. Header Widget
    let total_crates = state.crates.len();
    let completed_crates = state
        .crates
        .values()
        .filter(|c| c.status == BuildStatus::Completed)
        .count();
    let compiling_crates = state
        .crates
        .values()
        .filter(|c| c.status == BuildStatus::Compiling)
        .count();
    let failed_crates = state
        .crates
        .values()
        .filter(|c| c.status == BuildStatus::Failed)
        .count();

    let status_str = match state.success {
        None => "RUNNING".to_string(),
        Some(true) => "SUCCESS".to_string(),
        Some(false) => "FAILED".to_string(),
    };
    let status_style = match state.success {
        None => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        Some(true) => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        Some(false) => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    };

    let bottlenecks = state.get_slowest_crates(3);
    let mut line2_spans = vec![Span::raw("Top Bottlenecks: ")];
    if bottlenecks.is_empty() {
        line2_spans.push(Span::styled("None", Style::default().fg(Color::DarkGray)));
    } else {
        for (i, (name, dur)) in bottlenecks.iter().enumerate() {
            if i > 0 {
                line2_spans.push(Span::raw(", "));
            }
            line2_spans.push(Span::styled(
                format!("{} ({:.1}s)", name, dur.as_secs_f64()),
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            ));
        }
    }

    let header_text = vec![
        Line::from(vec![
            Span::raw("Status: "),
            Span::styled(status_str, status_style),
            Span::raw(" | Progress: "),
            Span::styled(
                format!("{}/{}", completed_crates, total_crates),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | Compiling: "),
            Span::styled(
                format!("{}", compiling_crates),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(" | Failed: "),
            Span::styled(
                format!("{}", failed_crates),
                Style::default().fg(Color::Red),
            ),
            Span::raw(" | Max Concurrency: "),
            Span::styled(
                format!("{}", state.max_concurrency),
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | Elapsed: "),
            Span::styled(
                format!("{:.1}s", total_duration.as_secs_f64()),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(line2_spans),
    ];

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).title(" Cargo Build Scan - Dashboard "));
    f.render_widget(header, chunks[0]);

    // Split Main area: Gantt (70%) and Details (30%) if screen width >= 90
    let main_layout = if f.size().width >= 90 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70),
                Constraint::Percentage(30),
            ])
            .split(chunks[1])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(chunks[1])
    };

    let gantt_chunk = main_layout[0];
    let main_height = gantt_chunk.height.saturating_sub(2) as usize; // subtract borders
    let main_width = gantt_chunk.width.saturating_sub(2) as usize; // subtract borders

    let filtered_crates = get_filtered_crates(state, search_query, show_cached, show_only_bottlenecks);
    let total_filtered = filtered_crates.len();

    // Adjust scroll offset
    if total_filtered > 0 {
        let sel = selected_index.min(total_filtered - 1);
        if sel < *scroll_offset {
            *scroll_offset = sel;
        } else if sel >= *scroll_offset + main_height {
            *scroll_offset = sel - main_height + 1;
        }
    } else {
        *scroll_offset = 0;
    }

    if total_filtered == 0 {
        let msg = if total_crates == 0 {
            "Waiting for compilation to start..."
        } else {
            "No crates match the active filters."
        };
        let empty_msg = Paragraph::new(msg)
            .block(Block::default().borders(Borders::ALL).title(" Compilation Timeline "))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(empty_msg, gantt_chunk);
    } else {
        let name_col_width: usize = 30;
        let dur_col_width: usize = 8;
        let separator_width: usize = 3; // " | "
        let gantt_width = if main_width > name_col_width + dur_col_width + separator_width {
            main_width - name_col_width - dur_col_width - separator_width
        } else {
            10
        };

        let mut lines = Vec::new();
        let start_idx = *scroll_offset;
        let end_idx = (start_idx + main_height).min(total_filtered);

        for idx in start_idx..end_idx {
            let is_selected = idx == selected_index;
            let name = &filtered_crates[idx];
            if let Some(crate_info) = state.crates.get(name) {
                let is_bottleneck = bottlenecks.iter().any(|(b_name, _)| b_name == name);

                // Prefix selected row
                let prefix = if is_selected { "▶ " } else { "  " };
                let full_name = format!("{} v{}", crate_info.name, crate_info.version);
                let adjusted_name_width = name_col_width.saturating_sub(2);
                let display_name_base = if full_name.len() > adjusted_name_width {
                    format!("{}...", &full_name[..adjusted_name_width.saturating_sub(3)])
                } else {
                    format!("{:<width$}", full_name, width = adjusted_name_width)
                };
                let display_name = format!("{}{}", prefix, display_name_base);

                let name_span = if is_selected {
                    Span::styled(display_name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                } else {
                    match crate_info.status {
                        BuildStatus::Pending => Span::styled(display_name, Style::default().fg(Color::DarkGray)),
                        BuildStatus::Compiling => {
                            let color = if is_bottleneck { Color::Magenta } else { Color::Yellow };
                            Span::styled(display_name, Style::default().fg(color).add_modifier(Modifier::BOLD))
                        }
                        BuildStatus::Completed => {
                            if crate_info.cached {
                                Span::styled(display_name, Style::default().fg(Color::DarkGray))
                            } else {
                                let color = if is_bottleneck { Color::Magenta } else { Color::Green };
                                Span::styled(display_name, Style::default().fg(color))
                            }
                        }
                        BuildStatus::Failed => {
                            Span::styled(display_name, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                        }
                    }
                };

                // Duration formatting
                let dur_span = if crate_info.cached {
                    Span::styled(
                        format!("{:>width$}", "cached", width = dur_col_width),
                        Style::default().fg(Color::DarkGray),
                    )
                } else {
                    let dur = crate_info.duration(now);
                    let dur_str = format!("{:.1}s", dur.as_secs_f64());
                    Span::styled(
                        format!("{:>width$}", dur_str, width = dur_col_width),
                        Style::default().fg(Color::Gray),
                    )
                };

                // Gantt Timeline logic
                let separator = Span::raw(" | ");
                let crate_start_offset = crate_info
                    .start_time
                    .map_or(total_duration, |t| t.duration_since(state.start_time));
                let crate_end_offset = crate_info
                    .end_time
                    .map_or(total_duration, |t| t.duration_since(state.start_time));

                let total_secs = total_duration.as_secs_f64().max(0.001);
                let start_ratio = (crate_start_offset.as_secs_f64() / total_secs).clamp(0.0, 1.0);
                let end_ratio = (crate_end_offset.as_secs_f64() / total_secs).clamp(0.0, 1.0);

                let start_cell = (start_ratio * gantt_width as f64).round() as usize;
                let end_cell = (end_ratio * gantt_width as f64).round() as usize;

                let start_cell = start_cell.min(gantt_width);
                let end_cell = end_cell.clamp(start_cell, gantt_width);

                let mut bar_chars = String::new();
                for _ in 0..start_cell {
                    bar_chars.push(' ');
                }

                let fill_char = match crate_info.status {
                    BuildStatus::Pending => ' ',
                    BuildStatus::Compiling => '▒',
                    BuildStatus::Completed => {
                        if crate_info.cached {
                            ' '
                        } else {
                            '█'
                        }
                    }
                    BuildStatus::Failed => '█',
                };

                let fill_color = match crate_info.status {
                    BuildStatus::Pending => Color::DarkGray,
                    BuildStatus::Compiling => {
                        if is_bottleneck {
                            Color::Magenta
                        } else {
                            Color::Yellow
                        }
                    }
                    BuildStatus::Completed => {
                        if crate_info.cached {
                            Color::DarkGray
                        } else if is_bottleneck {
                            Color::Magenta
                        } else {
                            Color::Green
                        }
                    }
                    BuildStatus::Failed => Color::Red,
                };

                for _ in start_cell..end_cell {
                    bar_chars.push(fill_char);
                }
                for _ in end_cell..gantt_width {
                    bar_chars.push(' ');
                }

                let bar_span = Span::styled(bar_chars, Style::default().fg(fill_color));
                lines.push(Line::from(vec![name_span, dur_span, separator, bar_span]));
            }
        }

        let title_suffix = if !search_query.is_empty() {
            format!(" (Filtered {}/{}) ", total_filtered, state.crate_order.len())
        } else {
            String::new()
        };

        let main_panel = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(format!(" Compilation Timeline (Gantt Chart){} ", title_suffix)));
        f.render_widget(main_panel, gantt_chunk);
    }

    // 2. Sidebar Details Panel
    if main_layout.len() > 1 {
        let right_chunk = main_layout[1];
        if total_filtered > 0 {
            let sel_idx = selected_index.min(total_filtered - 1);
            let name = &filtered_crates[sel_idx];
            if let Some(crate_info) = state.crates.get(name) {
                let is_bottleneck = bottlenecks.iter().any(|(b_name, _)| b_name == name);
                let dur = crate_info.duration(now);
                let dur_str = format!("{:.2}s", dur.as_secs_f64());
                let crate_start_offset = crate_info
                    .start_time
                    .map_or(Duration::from_secs(0), |t| t.duration_since(state.start_time));

                let mut details_lines = vec![
                    Line::from(vec![
                        Span::styled("Name: ", Style::default().fg(Color::Gray)),
                        Span::styled(&crate_info.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::styled("Version: ", Style::default().fg(Color::Gray)),
                        Span::styled(&crate_info.version, Style::default().fg(Color::White)),
                    ]),
                    Line::from(vec![
                        Span::styled("Status: ", Style::default().fg(Color::Gray)),
                        match crate_info.status {
                            BuildStatus::Pending => Span::styled("Pending", Style::default().fg(Color::DarkGray)),
                            BuildStatus::Compiling => Span::styled("Compiling", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                            BuildStatus::Completed => {
                                if crate_info.cached {
                                    Span::styled("Cached", Style::default().fg(Color::DarkGray))
                                } else {
                                    Span::styled("Completed", Style::default().fg(Color::Green))
                                }
                            }
                            BuildStatus::Failed => Span::styled("Failed", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        },
                    ]),
                    Line::from(vec![
                        Span::styled("Duration: ", Style::default().fg(Color::Gray)),
                        Span::styled(dur_str, Style::default().fg(Color::White)),
                    ]),
                    Line::from(vec![
                        Span::styled("Start Offset: ", Style::default().fg(Color::Gray)),
                        Span::styled(format!("{:.2}s", crate_start_offset.as_secs_f64()), Style::default().fg(Color::White)),
                    ]),
                    Line::from(vec![
                        Span::styled("Bottleneck: ", Style::default().fg(Color::Gray)),
                        if is_bottleneck {
                            Span::styled("YES (Top 3)", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
                        } else {
                            Span::styled("NO", Style::default().fg(Color::DarkGray))
                        },
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("--- Diagnostics / Logs ---", Style::default().fg(Color::Gray))),
                ];

                if crate_info.diagnostics.is_empty() {
                    details_lines.push(Line::from(Span::styled("No warnings or errors.", Style::default().fg(Color::DarkGray))));
                } else {
                    let mut diag_lines = Vec::new();
                    for d in &crate_info.diagnostics {
                        for line in d.lines() {
                            let clean = strip_ansi_codes(line);
                            diag_lines.push(clean);
                        }
                    }

                    let sidebar_height = right_chunk.height.saturating_sub(2) as usize;
                    let metadata_lines_count = 9;
                    let visible_diag_lines = if sidebar_height > metadata_lines_count {
                        sidebar_height - metadata_lines_count
                    } else {
                        1
                    };

                    let max_scroll = diag_lines.len().saturating_sub(visible_diag_lines);
                    let scroll = diagnostics_scroll.min(max_scroll);

                    for i in scroll..(scroll + visible_diag_lines).min(diag_lines.len()) {
                        let line_text = &diag_lines[i];
                        let max_line_len = right_chunk.width.saturating_sub(2) as usize;
                        let display_line = if line_text.len() > max_line_len {
                            format!("{}...", &line_text[..max_line_len.saturating_sub(3)])
                        } else {
                            line_text.to_string()
                        };

                        let line_style = if display_line.starts_with("error:") || display_line.contains("error[") {
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                        } else if display_line.starts_with("warning:") || display_line.contains("warning[") {
                            Style::default().fg(Color::Yellow)
                        } else if display_line.trim().starts_with("-->") {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default().fg(Color::Gray)
                        };
                        details_lines.push(Line::from(Span::styled(display_line, line_style)));
                    }

                    if diag_lines.len() > visible_diag_lines {
                        details_lines.push(Line::from(Span::styled(
                            format!("(PgUp/PgDn scroll: {}/{})", scroll + 1, max_scroll + 1),
                            Style::default().fg(Color::Yellow),
                        )));
                    }
                }

                let sidebar_panel = Paragraph::new(details_lines)
                    .block(Block::default().borders(Borders::ALL).title(" Crate Details "));
                f.render_widget(sidebar_panel, right_chunk);
            }
        } else {
            let empty_panel = Paragraph::new("No crates to display.")
                .block(Block::default().borders(Borders::ALL).title(" Crate Details "));
            f.render_widget(empty_panel, right_chunk);
        }
    }

    // 3. Footer Widget
    let mut footer_spans = vec![
        Span::styled(" Q ", Style::default().bg(Color::White).fg(Color::Black)),
        Span::raw(" Quit | "),
        Span::styled(" Up/Down ", Style::default().bg(Color::White).fg(Color::Black)),
        Span::raw(" Select | "),
        Span::styled(" / or S ", Style::default().bg(Color::White).fg(Color::Black)),
        Span::raw(" Search | "),
        Span::styled(" C ", Style::default().bg(Color::White).fg(Color::Black)),
        Span::raw(format!(" Cached ({}) | ", if show_cached { "ON" } else { "OFF" })),
        Span::styled(" B ", Style::default().bg(Color::White).fg(Color::Black)),
        Span::raw(format!(" Bottlenecks ({})", if show_only_bottlenecks { "ON" } else { "OFF" })),
    ];

    if search_mode {
        footer_spans = vec![
            Span::styled(" ESC ", Style::default().bg(Color::White).fg(Color::Black)),
            Span::raw(" Exit Search | "),
            Span::styled(" SEARCH: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(search_query),
            Span::styled("█", Style::default().fg(Color::Green)),
        ];
    } else if !search_query.is_empty() {
        footer_spans.push(Span::raw(" | "));
        footer_spans.push(Span::styled(" Filter: ", Style::default().fg(Color::Green)));
        footer_spans.push(Span::raw(search_query));
    }

    let footer = Paragraph::new(Line::from(footer_spans));
    f.render_widget(footer, chunks[2]);
}
