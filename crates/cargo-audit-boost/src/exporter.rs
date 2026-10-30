use crate::model::{BuildState, BuildStatus};
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::time::Instant;

#[derive(Serialize)]
pub struct ExportCrateInfo {
    pub name: String,
    pub version: String,
    pub start_offset_secs: f64,
    pub duration_secs: f64,
    pub status: String,
    pub is_bottleneck: bool,
    pub cached: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Serialize)]
pub struct ExportReport {
    pub total_duration_secs: f64,
    pub total_crates: usize,
    pub max_concurrency: usize,
    pub success: bool,
    pub bottlenecks: Vec<(String, f64)>,
    pub crates: Vec<ExportCrateInfo>,
}

pub fn export_build_scan(state: &BuildState) -> Result<()> {
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

    let total_secs = total_duration.as_secs_f64().max(0.001);
    let raw_bottlenecks = state.get_slowest_crates(3);
    let bottleneck_names: Vec<String> = raw_bottlenecks.iter().map(|(n, _)| n.clone()).collect();

    let mut export_crates = Vec::new();
    for name in &state.crate_order {
        if let Some(c) = state.crates.get(name) {
            let start_offset = c
                .start_time
                .map_or(total_duration, |t| t.duration_since(state.start_time))
                .as_secs_f64();
            let duration = c.duration(now).as_secs_f64();
            let is_b = bottleneck_names.contains(&c.name);

            let status_str = match c.status {
                BuildStatus::Pending => "Pending",
                BuildStatus::Compiling => "Compiling",
                BuildStatus::Completed => "Completed",
                BuildStatus::Failed => "Failed",
            };

            export_crates.push(ExportCrateInfo {
                name: c.name.clone(),
                version: c.version.clone(),
                start_offset_secs: start_offset,
                duration_secs: duration,
                status: status_str.to_string(),
                is_bottleneck: is_b,
                cached: c.cached,
                diagnostics: c.diagnostics.clone(),
            });
        }
    }

    let bottlenecks = raw_bottlenecks
        .iter()
        .map(|(n, d)| (n.clone(), d.as_secs_f64()))
        .collect();

    let report = ExportReport {
        total_duration_secs: total_secs,
        total_crates: state.crates.len(),
        max_concurrency: state.max_concurrency,
        success: state.success.unwrap_or(false),
        bottlenecks,
        crates: export_crates,
    };

    // Ensure target/build-scan directory exists
    let scan_dir = Path::new("target").join("build-scan");
    fs::create_dir_all(&scan_dir).context("Failed to create target/build-scan directory")?;

    // Export JSON
    let json_content =
        serde_json::to_string_pretty(&report).context("Failed to serialize report to JSON")?;
    let json_path = scan_dir.join("report.json");
    fs::write(&json_path, json_content).context("Failed to write report.json")?;

    // Export HTML
    let html_content = generate_html(&report);
    let html_path = scan_dir.join("report.html");
    fs::write(&html_path, html_content).context("Failed to write report.html")?;

    println!("\n\x1b[32m✔\x1b[0m Build Scan Report exported successfully:");
    println!("  JSON: \x1b[36m{}\x1b[0m", json_path.to_string_lossy());
    println!("  HTML: \x1b[36m{}\x1b[0m", html_path.to_string_lossy());

    Ok(())
}

fn generate_html(report: &ExportReport) -> String {
    let status_class = if report.success { "success" } else { "failed" };
    let status_text = if report.success { "SUCCESS" } else { "FAILED" };

    // Calculate concurrency timeline sampled at 100 points
    let n_intervals = 100;
    let step_secs = report.total_duration_secs / n_intervals as f64;
    let mut concurrency_timeline = Vec::new();
    let mut max_observed_concurrency = 0;
    for i in 0..=n_intervals {
        let t = i as f64 * step_secs;
        let mut count = 0;
        for c in &report.crates {
            if (c.status == "Compiling" || c.status == "Completed" || c.status == "Failed")
                && !c.cached
                && t >= c.start_offset_secs
                && t <= (c.start_offset_secs + c.duration_secs)
            {
                count += 1;
            }
        }
        if count > max_observed_concurrency {
            max_observed_concurrency = count;
        }
        concurrency_timeline.push((t, count));
    }
    let max_concur_f = max_observed_concurrency.max(1) as f64;

    // Build SVG Concurrency Area Chart coordinates
    let svg_width = 800.0;
    let svg_height = 180.0;
    let margin_left = 45.0;
    let margin_right = 20.0;
    let margin_top = 15.0;
    let margin_bottom = 25.0;
    let chart_width = svg_width - margin_left - margin_right;
    let chart_height = svg_height - margin_top - margin_bottom;

    let mut area_points = format!("M {:.1} {:.1} ", margin_left, margin_top + chart_height);
    let mut line_points = String::new();

    for (i, &(_t, count)) in concurrency_timeline.iter().enumerate() {
        let x = margin_left + (i as f64 / n_intervals as f64) * chart_width;
        let y = margin_top + chart_height - (count as f64 / max_concur_f) * chart_height;
        if i == 0 {
            line_points.push_str(&format!("M {:.1} {:.1} ", x, y));
        } else {
            line_points.push_str(&format!("L {:.1} {:.1} ", x, y));
        }
        area_points.push_str(&format!("L {:.1} {:.1} ", x, y));
    }
    area_points.push_str(&format!(
        "L {:.1} {:.1} Z",
        margin_left + chart_width,
        margin_top + chart_height
    ));

    // SVG grid lines and Y axis labels
    let mut grid_html = String::new();
    let y_divisions = 3;
    for k in 0..=y_divisions {
        let pct = k as f64 / y_divisions as f64;
        let y_val = pct * max_observed_concurrency as f64;
        let y_pos = margin_top + chart_height - pct * chart_height;
        grid_html.push_str(&format!(
            "<line x1='{margin_left}' y1='{y_pos}' x2='{chart_end}' y2='{y_pos}' stroke='var(--border-color)' stroke-dasharray='4 4' stroke-width='0.5' />
             <text x='{lbl_x}' y='{lbl_y}' fill='var(--text-muted)' font-size='10' font-family='Inter, sans-serif' text-anchor='end'>{y_val_formatted}</text>",
            margin_left = margin_left,
            y_pos = y_pos,
            chart_end = margin_left + chart_width,
            lbl_x = margin_left - 8.0,
            lbl_y = y_pos + 3.0,
            y_val_formatted = y_val.round() as usize
        ));
    }

    // SVG grid lines and X axis labels (Time)
    let x_divisions = 5;
    for k in 0..=x_divisions {
        let pct = k as f64 / x_divisions as f64;
        let x_pos = margin_left + pct * chart_width;
        let t_val = pct * report.total_duration_secs;
        grid_html.push_str(&format!(
            "<line x1='{x_pos}' y1='{margin_top}' x2='{x_pos}' y2='{chart_end_y}' stroke='var(--border-color)' stroke-dasharray='4 4' stroke-width='0.5' />
             <text x='{x_pos}' y='{lbl_y}' fill='var(--text-muted)' font-size='10' font-family='Inter, sans-serif' text-anchor='middle'>{t_val_formatted:.1}s</text>",
            x_pos = x_pos,
            margin_top = margin_top,
            chart_end_y = margin_top + chart_height,
            lbl_y = margin_top + chart_height + 15.0,
            t_val_formatted = t_val
        ));
    }

    let svg_chart = format!(
        r#"<svg viewBox="0 0 800 180" class="concurrency-svg" style="width: 100%; height: auto; display: block;">
             <defs>
               <linearGradient id="areaGrad" x1="0" y1="0" x2="0" y2="1">
                 <stop offset="0%" stop-color="var(--color-magenta)" stop-opacity="0.3"/>
                 <stop offset="100%" stop-color="var(--color-magenta)" stop-opacity="0"/>
               </linearGradient>
             </defs>
             {grid}
             <path d="{area}" fill="url(#areaGrad)" />
             <path d="{line}" fill="none" stroke="var(--color-magenta)" stroke-width="2" />
           </svg>"#,
        grid = grid_html,
        area = area_points,
        line = line_points
    );

    // Render initial fallback row cards
    let mut rows_html = String::new();
    let total_secs = report.total_duration_secs.max(0.001);
    for c in &report.crates {
        let left_pct = (c.start_offset_secs / total_secs * 100.0).clamp(0.0, 100.0);
        let width_pct = (c.duration_secs / total_secs * 100.0).clamp(0.1, 100.0);

        let bar_class = if c.status == "Failed" {
            "bar failed"
        } else if c.cached {
            "bar cached"
        } else if c.is_bottleneck {
            "bar bottleneck"
        } else {
            "bar standard"
        };

        let row_class = if c.cached {
            "gantt-row cached"
        } else {
            "gantt-row"
        };

        let dur_text = if c.cached {
            "cached".to_string()
        } else {
            format!("{:.2}s", c.duration_secs)
        };

        let mut diagnostics_section = String::new();
        if !c.diagnostics.is_empty() {
            let mut diags_text = String::new();
            for d in &c.diagnostics {
                diags_text.push_str(&escape_html(&strip_ansi_codes(d)));
                diags_text.push('\n');
            }
            diagnostics_section = format!(
                r#"<div class="diagnostics-log">
                     <div class="log-title">Compiler Output</div>
                     <pre><code>{}</code></pre>
                   </div>"#,
                diags_text
            );
        }

        rows_html.push_str(&format!(
            "<div class='{row_class}' data-name='{name_lower}' onclick='toggleDiagnostics(this)'>
                <div class='gantt-row-main'>
                    <div class='crate-meta'>
                        <span class='crate-name'>{name}</span>
                        <span class='crate-version'>v{version}</span>
                        {warning_badge}
                    </div>
                    <div class='crate-duration'>{dur_text}</div>
                    <div class='gantt-timeline-cell'>
                        <div class='gantt-bar-container'>
                            <div class='{bar_class}' style='margin-left: {left_pct:.2}%; width: {width_pct:.2}%;' title='Crate: {name}, Duration: {dur_text}, Start Offset: {start_offset:.2}s'></div>
                        </div>
                    </div>
                </div>
                {diagnostics_section}
             </div>",
            row_class = row_class,
            name = c.name,
            version = c.version,
            name_lower = c.name.to_lowercase(),
            dur_text = dur_text,
            bar_class = bar_class,
            left_pct = left_pct,
            width_pct = width_pct,
            start_offset = c.start_offset_secs,
            warning_badge = if !c.diagnostics.is_empty() { "<span class='warning-indicator'>⚠️ log</span>" } else { "" },
            diagnostics_section = diagnostics_section
        ));
    }

    let mut bottlenecks_html = String::new();
    if report.bottlenecks.is_empty() {
        bottlenecks_html
            .push_str("<div class='no-bottlenecks'>No compilation metrics recorded.</div>");
    } else {
        for (name, dur) in &report.bottlenecks {
            bottlenecks_html.push_str(&format!(
                "<div class='bottleneck-card'>
                    <div class='bottleneck-name'>{}</div>
                    <div class='bottleneck-value'>{:.2}s</div>
                 </div>",
                name, dur
            ));
        }
    }

    let crates_json = serde_json::to_string(&report.crates).unwrap_or_else(|_| "[]".to_string());

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Cargo Build Scan - Report</title>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
    <style>
        :root {{
            --bg-color: #0f172a;
            --card-bg: #1e293b;
            --border-color: #334155;
            --text-main: #f8fafc;
            --text-muted: #94a3b8;
            --color-green: #10b981;
            --color-red: #ef4444;
            --color-magenta: #d946ef;
            --color-yellow: #f59e0b;
            --color-blue: #3b82f6;
            --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
            --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -2px rgba(0, 0, 0, 0.1);
        }}

        [data-theme="light"] {{
            --bg-color: #f8fafc;
            --card-bg: #ffffff;
            --border-color: #e2e8f0;
            --text-main: #0f172a;
            --text-muted: #64748b;
            --color-green: #10b981;
            --color-red: #ef4444;
            --color-magenta: #d946ef;
            --color-yellow: #d97706;
            --color-blue: #2563eb;
            --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.05), 0 2px 4px -2px rgba(0, 0, 0, 0.05);
        }}

        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }}

        body {{
            background-color: var(--bg-color);
            color: var(--text-main);
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            line-height: 1.5;
            padding: 2.5rem 1.5rem;
            transition: background-color 0.3s ease, color 0.3s ease;
        }}

        .container {{
            max-width: 1200px;
            margin: 0 auto;
        }}

        header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 1px solid var(--border-color);
            padding-bottom: 1.5rem;
            margin-bottom: 2rem;
        }}

        .header-title-area {{
            display: flex;
            align-items: center;
            gap: 1rem;
        }}

        h1 {{
            font-size: 1.8rem;
            font-weight: 800;
            letter-spacing: -0.025em;
            background: linear-gradient(to right, var(--text-main), var(--text-muted));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}

        .theme-toggle {{
            background: none;
            border: 1px solid var(--border-color);
            color: var(--text-main);
            cursor: pointer;
            padding: 0.5rem 1rem;
            border-radius: 8px;
            font-weight: 500;
            font-size: 0.875rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
            transition: all 0.2s ease;
        }}

        .theme-toggle:hover {{
            background-color: rgba(148, 163, 184, 0.1);
        }}

        .badge {{
            padding: 0.375rem 0.875rem;
            border-radius: 9999px;
            font-weight: 700;
            font-size: 0.75rem;
            letter-spacing: 0.05em;
            text-transform: uppercase;
        }}

        .badge.success {{
            background-color: rgba(16, 185, 129, 0.15);
            color: var(--color-green);
            border: 1px solid rgba(16, 185, 129, 0.3);
        }}

        .badge.failed {{
            background-color: rgba(239, 68, 68, 0.15);
            color: var(--color-red);
            border: 1px solid rgba(239, 68, 68, 0.3);
        }}

        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
            gap: 1.5rem;
            margin-bottom: 2.5rem;
        }}

        .stat-card {{
            background-color: var(--card-bg);
            border: 1px solid var(--border-color);
            border-radius: 16px;
            padding: 1.5rem;
            display: flex;
            flex-direction: column;
            box-shadow: var(--shadow-md);
            transition: transform 0.2s ease, border-color 0.2s ease;
        }}

        .stat-card:hover {{
            transform: translateY(-2px);
            border-color: var(--color-blue);
        }}

        .stat-label {{
            font-size: 0.75rem;
            color: var(--text-muted);
            margin-bottom: 0.5rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            font-weight: 600;
        }}

        .stat-value {{
            font-size: 2.25rem;
            font-weight: 800;
            letter-spacing: -0.02em;
        }}

        .section-title {{
            font-size: 1.25rem;
            font-weight: 700;
            margin-bottom: 1.25rem;
            color: var(--text-main);
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}

        .chart-panel {{
            background-color: var(--card-bg);
            border: 1px solid var(--border-color);
            border-radius: 16px;
            padding: 1.5rem;
            margin-bottom: 2.5rem;
            box-shadow: var(--shadow-md);
        }}

        .bottlenecks-container {{
            margin-bottom: 2.5rem;
        }}

        .bottlenecks-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 1rem;
        }}

        .bottleneck-card {{
            background-color: var(--card-bg);
            border: 1px solid var(--border-color);
            border-left: 4px solid var(--color-magenta);
            border-radius: 12px;
            padding: 1rem 1.25rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
            box-shadow: var(--shadow-sm);
        }}

        .bottleneck-name {{
            font-weight: 600;
            color: var(--text-main);
        }}

        .bottleneck-value {{
            font-weight: 700;
            color: var(--color-magenta);
        }}

        .no-bottlenecks {{
            color: var(--text-muted);
            font-style: italic;
        }}

        .controls-row {{
            display: flex;
            flex-wrap: wrap;
            gap: 1rem;
            margin-bottom: 1.5rem;
            align-items: center;
            justify-content: space-between;
        }}

        .search-box {{
            flex: 1;
            min-width: 280px;
            position: relative;
        }}

        .search-input {{
            width: 100%;
            padding: 0.625rem 1rem 0.625rem 2.5rem;
            border-radius: 10px;
            border: 1px solid var(--border-color);
            background-color: var(--card-bg);
            color: var(--text-main);
            font-family: inherit;
            font-size: 0.9rem;
            transition: all 0.2s ease;
        }}

        .search-input:focus {{
            outline: none;
            border-color: var(--color-blue);
            box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15);
        }}

        .search-icon {{
            position: absolute;
            left: 0.875rem;
            top: 50%;
            transform: translateY(-50%);
            color: var(--text-muted);
            pointer-events: none;
        }}

        .filter-tabs {{
            display: flex;
            background-color: var(--card-bg);
            border: 1px solid var(--border-color);
            border-radius: 10px;
            padding: 0.25rem;
            gap: 0.25rem;
        }}

        .tab-btn {{
            background: none;
            border: none;
            padding: 0.5rem 1rem;
            border-radius: 8px;
            color: var(--text-muted);
            font-weight: 500;
            font-size: 0.85rem;
            cursor: pointer;
            transition: all 0.15s ease;
        }}

        .tab-btn:hover {{
            color: var(--text-main);
        }}

        .tab-btn.active {{
            background-color: var(--color-blue);
            color: white;
        }}

        .sort-area {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
            font-size: 0.875rem;
            color: var(--text-muted);
        }}

        .sort-select {{
            padding: 0.5rem 1rem;
            border-radius: 8px;
            border: 1px solid var(--border-color);
            background-color: var(--card-bg);
            color: var(--text-main);
            font-family: inherit;
            font-size: 0.85rem;
            cursor: pointer;
            outline: none;
        }}

        .gantt-chart {{
            background-color: var(--card-bg);
            border: 1px solid var(--border-color);
            border-radius: 16px;
            padding: 1.5rem;
            box-shadow: var(--shadow-md);
        }}

        .gantt-header {{
            display: grid;
            grid-template-columns: 240px 90px 1fr;
            border-bottom: 1px solid var(--border-color);
            padding-bottom: 0.75rem;
            margin-bottom: 0.75rem;
            font-size: 0.75rem;
            font-weight: 700;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}

        .gantt-timeline-ticks {{
            display: flex;
            justify-content: space-between;
            position: relative;
            padding: 0 5px;
        }}

        .tick-label {{
            font-size: 0.7rem;
            color: var(--text-muted);
        }}

        .gantt-row {{
            border-bottom: 1px solid rgba(148, 163, 184, 0.1);
            transition: background-color 0.15s ease;
        }}

        .gantt-row-main {{
            display: grid;
            grid-template-columns: 240px 90px 1fr;
            align-items: center;
            padding: 0.625rem 0;
            cursor: pointer;
        }}

        .gantt-row:hover {{
            background-color: rgba(148, 163, 184, 0.04);
        }}

        .gantt-row.cached {{
            opacity: 0.55;
        }}

        .crate-meta {{
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            padding-right: 1rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}

        .crate-name {{
            font-weight: 600;
            color: var(--text-main);
        }}

        .crate-version {{
            font-size: 0.75rem;
            color: var(--text-muted);
        }}

        .warning-indicator {{
            background-color: rgba(245, 158, 11, 0.15);
            color: var(--color-yellow);
            border: 1px solid rgba(245, 158, 11, 0.3);
            border-radius: 4px;
            padding: 1px 5px;
            font-size: 0.65rem;
            font-weight: 600;
        }}

        .crate-duration {{
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.85rem;
            color: var(--text-muted);
        }}

        .gantt-timeline-cell {{
            width: 100%;
        }}

        .gantt-bar-container {{
            width: 100%;
            background-color: rgba(148, 163, 184, 0.08);
            height: 10px;
            border-radius: 9999px;
            overflow: hidden;
            display: flex;
            position: relative;
        }}

        .bar {{
            height: 100%;
            border-radius: 9999px;
            transition: all 0.3s ease;
        }}

        .bar.standard {{
            background-color: var(--color-green);
        }}

        .bar.bottleneck {{
            background-color: var(--color-magenta);
        }}

        .bar.failed {{
            background-color: var(--color-red);
        }}

        .bar.cached {{
            background-color: transparent;
            border: 1px dashed var(--border-color);
        }}

        .diagnostics-log {{
            display: none;
            padding: 0 1rem 1rem 1rem;
            margin-top: -0.25rem;
        }}

        .gantt-row.expanded .diagnostics-log {{
            display: block;
        }}

        .log-title {{
            font-size: 0.75rem;
            font-weight: 700;
            color: var(--text-muted);
            margin-bottom: 0.5rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}

        .diagnostics-log pre {{
            background-color: rgba(15, 23, 42, 0.6);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 1rem;
            overflow-x: auto;
            color: #e2e8f0;
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.85rem;
            max-height: 300px;
            overflow-y: auto;
        }}

        @media (max-width: 768px) {{
            .gantt-header, .gantt-row-main {{
                grid-template-columns: 160px 70px 1fr;
            }}
            body {{
                padding: 1.5rem 1rem;
            }}
            h1 {{
                font-size: 1.5rem;
            }}
            .stats-grid {{
                grid-template-columns: 1fr;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div class="header-title-area">
                <h1>Cargo Build Scan Report</h1>
                <span class="badge {status_class}">{status_text}</span>
            </div>
            <button class="theme-toggle" onclick="toggleTheme()" aria-label="Toggle Theme">
                <svg width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                    <circle cx="12" cy="12" r="5"></circle>
                    <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"></path>
                </svg>
                <span>Theme</span>
            </button>
        </header>

        <section class="stats-grid">
            <div class="stat-card">
                <span class="stat-label">Elapsed Time</span>
                <span class="stat-value">{total_duration:.2}s</span>
            </div>
            <div class="stat-card">
                <span class="stat-label">Crates Compiled</span>
                <span class="stat-value">{total_crates}</span>
            </div>
            <div class="stat-card">
                <span class="stat-label">Max Concurrency</span>
                <span class="stat-value">{max_concurrency}</span>
            </div>
        </section>

        <section class="chart-panel">
            <h2 class="section-title">Compilation Concurrency Timeline</h2>
            {svg_chart}
        </section>

        <section class="bottlenecks-container">
            <h2 class="section-title">Top Compiler Bottlenecks</h2>
            <div class="bottlenecks-grid">
                {bottlenecks}
            </div>
        </section>

        <section class="gantt-container">
            <div class="section-title">
                <span>Compilation Gantt Timeline</span>
            </div>

            <div class="controls-row">
                <div class="search-box">
                    <svg class="search-icon" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                        <circle cx="11" cy="11" r="8"></circle>
                        <path d="M21 21l-4.35-4.35"></path>
                    </svg>
                    <input type="text" id="searchInput" class="search-input" placeholder="Search crates by name..." oninput="onFilterChange()">
                </div>

                <div class="filter-tabs">
                    <button id="tab-all" class="tab-btn active" onclick="setFilter('all')">All</button>
                    <button id="tab-compiled" class="tab-btn" onclick="setFilter('compiled')">Compiled</button>
                    <button id="tab-cached" class="tab-btn" onclick="setFilter('cached')">Cached</button>
                    <button id="tab-failed" class="tab-btn" onclick="setFilter('failed')">Failed</button>
                    <button id="tab-bottleneck" class="tab-btn" onclick="setFilter('bottleneck')">Bottlenecks</button>
                </div>

                <div class="sort-area">
                    <span>Sort by:</span>
                    <select id="sortSelect" class="sort-select" onchange="onSortChange()">
                        <option value="start">Start Time</option>
                        <option value="duration">Duration (Slowest First)</option>
                        <option value="name">Crate Name</option>
                    </select>
                </div>
            </div>

            <div class="gantt-chart">
                <div class="gantt-header">
                    <div>Crate Name</div>
                    <div>Duration</div>
                    <div class="gantt-timeline-ticks">
                        <span class="tick-label">0%</span>
                        <span class="tick-label">25%</span>
                        <span class="tick-label">50%</span>
                        <span class="tick-label">75%</span>
                        <span class="tick-label">100%</span>
                    </div>
                </div>
                <div id="ganttRowsContainer">
                    {rows}
                </div>
            </div>
        </section>
    </div>

    <script>
        const cratesData = {crates_json};
        const totalDurationSecs = {total_duration_raw};
        let activeFilter = 'all';
        let currentSort = 'start';

        if (localStorage.getItem('theme') === 'light') {{
            document.documentElement.setAttribute('data-theme', 'light');
        }}

        function toggleTheme() {{
            const currentTheme = document.documentElement.getAttribute('data-theme');
            if (currentTheme === 'light') {{
                document.documentElement.removeAttribute('data-theme');
                localStorage.setItem('theme', 'dark');
            }} else {{
                document.documentElement.setAttribute('data-theme', 'light');
                localStorage.setItem('theme', 'light');
            }}
        }}

        function toggleDiagnostics(rowElement) {{
            rowElement.classList.toggle('expanded');
        }}

        function setFilter(filterName) {{
            activeFilter = filterName;
            document.querySelectorAll('.tab-btn').forEach(btn => btn.classList.remove('active'));
            document.getElementById('tab-' + filterName).classList.add('active');
            renderRows();
        }}

        function onFilterChange() {{
            renderRows();
        }}

        function onSortChange() {{
            currentSort = document.getElementById('sortSelect').value;
            renderRows();
        }}

        function escapeHTML(str) {{
            return str
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')
                .replace(/"/g, '&quot;')
                .replace(/'/g, '&#x27;');
        }}

        function renderRows() {{
            const query = document.getElementById('searchInput').value.toLowerCase();
            let filtered = [...cratesData];

            if (query) {{
                filtered = filtered.filter(c => c.name.toLowerCase().includes(query));
            }}

            if (activeFilter === 'compiled') {{
                filtered = filtered.filter(c => !c.cached);
            }} else if (activeFilter === 'cached') {{
                filtered = filtered.filter(c => c.cached);
            }} else if (activeFilter === 'failed') {{
                filtered = filtered.filter(c => c.status === 'Failed');
            }} else if (activeFilter === 'bottleneck') {{
                filtered = filtered.filter(c => c.is_bottleneck);
            }}

            if (currentSort === 'start') {{
                filtered.sort((a, b) => a.start_offset_secs - b.start_offset_secs);
            }} else if (currentSort === 'duration') {{
                filtered.sort((a, b) => b.duration_secs - a.duration_secs);
            }} else if (currentSort === 'name') {{
                filtered.sort((a, b) => a.name.localeCompare(b.name));
            }}

            const container = document.getElementById('ganttRowsContainer');
            if (filtered.length === 0) {{
                container.innerHTML = '<div style="text-align: center; padding: 2rem; color: var(--text-muted);">No crates match the active filters.</div>';
                return;
            }}

            let html = '';
            filtered.forEach(c => {{
                const leftPct = (c.start_offset_secs / totalDurationSecs * 100);
                const widthPct = Math.max(0.1, (c.duration_secs / totalDurationSecs * 100));

                let barClass = 'bar standard';
                if (c.status === 'Failed') {{
                    barClass = 'bar failed';
                }} else if (c.cached) {{
                    barClass = 'bar cached';
                }} else if (c.is_bottleneck) {{
                    barClass = 'bar bottleneck';
                }}

                let rowClass = c.cached ? 'gantt-row cached' : 'gantt-row';
                let durText = c.cached ? 'cached' : c.duration_secs.toFixed(2) + 's';

                let warningBadge = c.diagnostics && c.diagnostics.length > 0 ? '<span class="warning-indicator">⚠️ log</span>' : '';

                let diagnosticsSection = '';
                if (c.diagnostics && c.diagnostics.length > 0) {{
                    let logText = escapeHTML(c.diagnostics.join('\\n'));
                    diagnosticsSection = `
                        <div class="diagnostics-log">
                            <div class="log-title">Compiler Output</div>
                            <pre><code>${{logText}}</code></pre>
                        </div>`;
                }}

                html += `
                    <div class="${{rowClass}}" data-name="${{c.name.toLowerCase()}}" onclick="toggleDiagnostics(this)">
                        <div class="gantt-row-main">
                            <div class="crate-meta">
                                <span class="crate-name">${{c.name}}</span>
                                <span class="crate-version">v${{c.version}}</span>
                                ${{warningBadge}}
                            </div>
                            <div class="crate-duration">${{durText}}</div>
                            <div class="gantt-timeline-cell">
                                <div class="gantt-bar-container">
                                    <div class="${{barClass}}" style="margin-left: ${{leftPct.toFixed(2)}}%; width: ${{widthPct.toFixed(2)}}%;" title="Crate: ${{c.name}}, Duration: ${{durText}}, Start Offset: ${{c.start_offset_secs.toFixed(2)}}s"></div>
                                </div>
                            </div>
                        </div>
                        ${{diagnosticsSection}}
                    </div>`;
            }});

            container.innerHTML = html;
        }}
    </script>
</body>
</html>"#,
        status_class = status_class,
        status_text = status_text,
        total_duration = report.total_duration_secs,
        total_duration_raw = report.total_duration_secs,
        total_crates = report.total_crates,
        max_concurrency = report.max_concurrency,
        svg_chart = svg_chart,
        bottlenecks = bottlenecks_html,
        rows = rows_html,
        crates_json = crates_json
    )
}

fn escape_html(input: &str) -> String {
    let mut escaped = String::new();
    for c in input.chars() {
        match c {
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '&' => escaped.push_str("&amp;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(c),
        }
    }
    escaped
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
