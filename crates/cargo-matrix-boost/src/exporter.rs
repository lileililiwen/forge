use crate::model::MatrixReport;
use std::fs;
use std::path::Path;

pub fn export_report(report: &MatrixReport, output_dir: &Path) -> anyhow::Result<()> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    // 1. Export JSON database
    let json_path = output_dir.join("report.json");
    let json_data = serde_json::to_string_pretty(report)?;
    fs::write(&json_path, json_data)?;

    // 2. Export HTML dashboard
    let html_path = output_dir.join("report.html");
    let html_content = generate_html_report(report)?;
    fs::write(&html_path, html_content)?;

    println!("📊 Exported JSON report to: {}", json_path.display());
    println!("🌐 Exported HTML dashboard to: {}", html_path.display());

    Ok(())
}

fn generate_html_report(report: &MatrixReport) -> anyhow::Result<String> {
    let serialized_report = serde_json::to_string(report)?;

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Cargo Matrix Boost Report</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Fira+Code:wght@400;500&family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
    <style>
        :root {{
            --bg-primary: #0f172a;
            --bg-secondary: #1e293b;
            --card-bg: #1e293b;
            --border-color: #334155;
            --text-primary: #f8fafc;
            --text-secondary: #94a3b8;
            --success: #10b981;
            --success-bg: rgba(16, 185, 129, 0.15);
            --danger: #ef4444;
            --danger-bg: rgba(239, 68, 68, 0.15);
            --running: #06b6d4;
            --running-bg: rgba(6, 182, 212, 0.15);
            --pending: #64748b;
            --pending-bg: rgba(100, 116, 139, 0.15);
            --accent: #6366f1;
            --accent-bg: rgba(99, 102, 241, 0.1);
            --console-bg: #090d16;
        }}

        .light-theme {{
            --bg-primary: #f8fafc;
            --bg-secondary: #f1f5f9;
            --card-bg: #ffffff;
            --border-color: #e2e8f0;
            --text-primary: #0f172a;
            --text-secondary: #475569;
            --success: #10b981;
            --success-bg: rgba(16, 185, 129, 0.1);
            --danger: #ef4444;
            --danger-bg: rgba(239, 68, 68, 0.1);
            --running: #06b6d4;
            --running-bg: rgba(6, 182, 212, 0.1);
            --pending: #94a3b8;
            --pending-bg: rgba(148, 163, 184, 0.1);
            --accent: #4f46e5;
            --accent-bg: rgba(79, 70, 229, 0.1);
            --console-bg: #0f172a;
        }}

        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }}

        body {{
            font-family: 'Inter', sans-serif;
            background-color: var(--bg-primary);
            color: var(--text-primary);
            transition: background-color 0.3s, color 0.3s;
            line-height: 1.5;
            padding: 2rem;
        }}

        .container {{
            max-width: 1400px;
            margin: 0 auto;
        }}

        header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 2rem;
            border-bottom: 1px solid var(--border-color);
            padding-bottom: 1.5rem;
        }}

        .logo-section {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}

        .logo-icon {{
            font-size: 2rem;
        }}

        .logo-title {{
            font-size: 1.5rem;
            font-weight: 700;
            letter-spacing: -0.025em;
        }}

        .logo-title span {{
            color: var(--accent);
        }}

        .meta-info {{
            font-size: 0.875rem;
            color: var(--text-secondary);
            margin-top: 0.25rem;
        }}

        .controls {{
            display: flex;
            gap: 1rem;
            align-items: center;
        }}

        button {{
            background-color: var(--card-bg);
            border: 1px solid var(--border-color);
            color: var(--text-primary);
            padding: 0.5rem 1rem;
            border-radius: 0.375rem;
            cursor: pointer;
            font-weight: 500;
            display: flex;
            align-items: center;
            gap: 0.5rem;
            transition: all 0.2s;
        }}

        button:hover {{
            background-color: var(--border-color);
        }}

        .dashboard-grid {{
            display: grid;
            grid-template-columns: 1fr 1fr 1fr 1fr;
            gap: 1.5rem;
            margin-bottom: 2.5rem;
        }}

        .stat-card {{
            background-color: var(--card-bg);
            border: 1px solid var(--border-color);
            border-radius: 0.5rem;
            padding: 1.5rem;
            display: flex;
            flex-direction: column;
            gap: 0.5rem;
            box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1);
        }}

        .stat-title {{
            font-size: 0.875rem;
            font-weight: 600;
            color: var(--text-secondary);
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}

        .stat-value {{
            font-size: 2rem;
            font-weight: 700;
        }}

        .stat-meta {{
            font-size: 0.875rem;
            color: var(--text-secondary);
        }}

        .main-content {{
            display: grid;
            grid-template-columns: 3fr 2fr;
            gap: 2rem;
        }}

        .matrix-section {{
            background-color: var(--card-bg);
            border: 1px solid var(--border-color);
            border-radius: 0.75rem;
            padding: 1.5rem;
            box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
        }}

        .section-header {{
            font-size: 1.25rem;
            font-weight: 600;
            margin-bottom: 1.5rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}

        .matrix-table {{
            width: 100%;
            border-collapse: collapse;
            margin-top: 1rem;
        }}

        .matrix-table th, .matrix-table td {{
            padding: 0.75rem 1rem;
            border: 1px solid var(--border-color);
            text-align: left;
        }}

        .matrix-table th {{
            background-color: var(--bg-secondary);
            font-weight: 600;
            font-size: 0.875rem;
            color: var(--text-secondary);
        }}

        .matrix-cell {{
            cursor: pointer;
            transition: background-color 0.2s;
        }}

        .matrix-cell:hover {{
            opacity: 0.85;
        }}

        .status-badge {{
            display: inline-flex;
            align-items: center;
            justify-content: center;
            padding: 0.25rem 0.75rem;
            border-radius: 9999px;
            font-size: 0.75rem;
            font-weight: 600;
            text-transform: uppercase;
        }}

        .status-success {{
            background-color: var(--success-bg);
            color: var(--success);
        }}

        .status-failed {{
            background-color: var(--danger-bg);
            color: var(--danger);
        }}

        .status-running {{
            background-color: var(--running-bg);
            color: var(--running);
        }}

        .status-pending {{
            background-color: var(--pending-bg);
            color: var(--pending);
        }}

        .detail-section {{
            display: flex;
            flex-direction: column;
            gap: 1.5rem;
        }}

        .detail-card {{
            background-color: var(--card-bg);
            border: 1px solid var(--border-color);
            border-radius: 0.75rem;
            padding: 1.5rem;
            box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
        }}

        .detail-row {{
            display: flex;
            justify-content: space-between;
            padding: 0.5rem 0;
            border-bottom: 1px solid var(--border-color);
            font-size: 0.95rem;
        }}

        .detail-row:last-child {{
            border-bottom: none;
        }}

        .detail-label {{
            color: var(--text-secondary);
            font-weight: 500;
        }}

        .detail-value {{
            font-weight: 600;
        }}

        .console-log {{
            background-color: var(--console-bg);
            color: #e2e8f0;
            font-family: 'Fira Code', monospace;
            font-size: 0.875rem;
            padding: 1.25rem;
            border-radius: 0.5rem;
            white-space: pre-wrap;
            max-height: 450px;
            overflow-y: auto;
            border: 1px solid var(--border-color);
        }}

        .console-error {{
            color: #ef4444;
            font-weight: 600;
            margin-bottom: 1rem;
            border-bottom: 1px dashed rgba(239, 68, 68, 0.3);
            padding-bottom: 0.5rem;
        }}

        @media (max-width: 1024px) {{
            .dashboard-grid {{
                grid-template-columns: 1fr 1fr;
            }}
            .main-content {{
                grid-template-columns: 1fr;
            }}
        }}

        @media (max-width: 640px) {{
            .dashboard-grid {{
                grid-template-columns: 1fr;
            }}
            body {{
                padding: 1rem;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div class="logo-section">
                <span class="logo-icon">🛡️</span>
                <div>
                    <h1 class="logo-title">Cargo Matrix <span>Boost</span></h1>
                    <p class="meta-info">Crate: <strong id="crate-title">mock-crate</strong> | Generated: <span id="report-date">...</span></p>
                </div>
            </div>
            <div class="controls">
                <button id="theme-toggle">🌓 Theme Toggle</button>
            </div>
        </header>

        <section class="dashboard-grid">
            <div class="stat-card">
                <span class="stat-title">Success Rate</span>
                <span class="stat-value" id="success-rate">0%</span>
                <span class="stat-meta" id="success-meta">0 / 0 succeeded</span>
            </div>
            <div class="stat-card">
                <span class="stat-title">Failed Jobs</span>
                <span class="stat-value" style="color: var(--danger);" id="failed-count">0</span>
                <span class="stat-meta">Requires resolution</span>
            </div>
            <div class="stat-card">
                <span class="stat-title">Total Duration</span>
                <span class="stat-value" id="total-duration">0.00s</span>
                <span class="stat-meta">Simulated parallel time</span>
            </div>
            <div class="stat-card">
                <span class="stat-title">Total Configurations</span>
                <span class="stat-value" id="total-configs">0</span>
                <span class="stat-meta">Toolchains × Features</span>
            </div>
        </section>

        <main class="main-content">
            <section class="matrix-section">
                <div class="section-header">
                    <span>Matrix Grid View</span>
                </div>
                <table class="matrix-table">
                    <thead>
                        <tr id="table-headers">
                            <th>Features Matrix</th>
                        </tr>
                    </thead>
                    <tbody id="table-body">
                        <!-- Matrix rows rendered by JavaScript -->
                    </tbody>
                </table>
            </section>

            <section class="detail-section">
                <div class="detail-card">
                    <h2 class="section-header">Selected Configuration Detail</h2>
                    <div id="job-details">
                        <p style="color: var(--text-secondary);">Click any cell in the matrix grid to inspect its logs.</p>
                    </div>
                </div>

                <div class="detail-card" style="flex-grow: 1;">
                    <h2 class="section-header">Diagnostics Console Log</h2>
                    <div class="console-log" id="console-logs">Console output...</div>
                </div>
            </section>
        </main>
    </div>

    <script>
        const report = {serialized_report};

        document.getElementById('crate-title').innerText = report.project_name;
        document.getElementById('report-date').innerText = report.timestamp;
        document.getElementById('total-configs').innerText = report.total_jobs;
        document.getElementById('failed-count').innerText = report.failed_jobs;
        document.getElementById('total-duration').innerText = (report.elapsed_ms / 1000).toFixed(2) + 's';

        const successRate = report.total_jobs > 0 ? (report.successful_jobs / report.total_jobs * 100).toFixed(1) : '0.0';
        document.getElementById('success-rate').innerText = successRate + '%';
        document.getElementById('success-meta').innerText = `${{report.successful_jobs}} / ${{report.total_jobs}} succeeded`;

        // Theme toggler
        const themeBtn = document.getElementById('theme-toggle');
        themeBtn.addEventListener('click', () => {{
            document.body.classList.toggle('light-theme');
        }});

        // Generate matrix grid
        const toolchains = [...new Set(report.jobs.map(j => j.toolchain || 'default'))];
        const featureKeys = [...new Set(report.jobs.map(j => {{
            if (j.features.length === 0) {{
                return j.no_default_features ? 'none' : 'default';
            }}
            let p = j.no_default_features ? '' : 'default,';
            return p + j.features.join(',');
        }}))];

        // Draw headers
        const headerRow = document.getElementById('table-headers');
        toolchains.forEach(tc => {{
            const th = document.createElement('th');
            th.innerText = tc;
            headerRow.appendChild(th);
        }});

        // Draw rows
        const tableBody = document.getElementById('table-body');
        featureKeys.forEach(feats => {{
            const tr = document.createElement('tr');
            
            const featureTd = document.createElement('td');
            featureTd.innerHTML = `<code>${{feats}}</code>`;
            tr.appendChild(featureTd);

            toolchains.forEach(tc => {{
                // Find matching job
                const job = report.jobs.find(j => {{
                    const jobTc = j.toolchain || 'default';
                    let jobFeats = '';
                    if (j.features.length === 0) {{
                        jobFeats = j.no_default_features ? 'none' : 'default';
                    }} else {{
                        let p = j.no_default_features ? '' : 'default,';
                        jobFeats = p + j.features.join(',');
                    }}
                    return jobTc === tc && jobFeats === feats;
                }});

                const cell = document.createElement('td');
                if (job) {{
                    cell.className = 'matrix-cell';
                    let statusClass = 'status-pending';
                    if (job.status === 'Success') statusClass = 'status-success';
                    if (job.status === 'Failed') statusClass = 'status-failed';
                    if (job.status === 'Running') statusClass = 'status-running';

                    cell.innerHTML = `<span class="status-badge ${{statusClass}}">${{job.status}}</span>`;
                    cell.addEventListener('click', () => selectJob(job));
                }} else {{
                    cell.innerText = '-';
                }}
                tr.appendChild(cell);
            }});

            tableBody.appendChild(tr);
        }});

        // Job details selection
        function selectJob(job) {{
            const detailsDiv = document.getElementById('job-details');
            const logsDiv = document.getElementById('console-logs');

            const tcStr = job.toolchain || 'default';
            const featsStr = job.features.length === 0 ? (job.no_default_features ? 'none' : 'default') : (job.no_default_features ? '' : 'default,') + job.features.join(',');
            const duration = (job.duration_ms / 1000).toFixed(2) + 's';

            let statusClass = 'status-pending';
            if (job.status === 'Success') statusClass = 'status-success';
            if (job.status === 'Failed') statusClass = 'status-failed';
            if (job.status === 'Running') statusClass = 'status-running';

            detailsDiv.innerHTML = `
                <div class="detail-row">
                    <span class="detail-label">ID:</span>
                    <span class="detail-value">#${{job.id}}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Toolchain:</span>
                    <span class="detail-value">${{tcStr}}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Features:</span>
                    <span class="detail-value"><code>${{featsStr}}</code></span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Status:</span>
                    <span class="detail-value"><span class="status-badge ${{statusClass}}">${{job.status}}</span></span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Duration:</span>
                    <span class="detail-value">${{duration}}</span>
                </div>
            `;

            let logs = '';
            if (job.error_message) {{
                logs += `<div class="console-error">${{job.error_message}}</div>`;
            }}
            logs += escapeHtml(job.log || 'No logs captured.');
            logsDiv.innerHTML = logs;
        }}

        function escapeHtml(text) {{
            return text
                .replace(/&/g, "&amp;")
                .replace(/</g, "&lt;")
                .replace(/>/g, "&gt;")
                .replace(/"/g, "&quot;")
                .replace(/'/g, "&#039;");
        }}

        // Automatically select the first failed job, or the first job
        if (report.jobs.length > 0) {{
            const firstFailed = report.jobs.find(j => j.status === 'Failed');
            selectJob(firstFailed || report.jobs[0]);
        }}
    </script>
</body>
</html>
"#,
        serialized_report = serialized_report
    );

    Ok(html)
}
