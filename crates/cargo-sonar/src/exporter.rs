use crate::model::SonarReport;
use std::fs;
use std::path::Path;

pub fn export_report(report: &SonarReport, output_dir: &Path) -> Result<(), String> {
    // Ensure output directory exists
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // 1. Export JSON Report
    let json_path = output_dir.join("report.json");
    let json_content = serde_json::to_string_pretty(report)
        .map_err(|e| format!("Failed to serialize report to JSON: {}", e))?;
    fs::write(&json_path, json_content)
        .map_err(|e| format!("Failed to write JSON report: {}", e))?;
    println!("💾 Saved JSON report to {}", json_path.display());

    // 2. Export HTML Report
    let html_path = output_dir.join("report.html");
    let report_json_str = serde_json::to_string(report)
        .map_err(|e| format!("Failed to serialize report for HTML: {}", e))?;

    let html_content = HTML_TEMPLATE
        .replace("__PROJECT_NAME__", &report.summary.project_name)
        .replace("__REPORT_JSON__", &report_json_str);

    fs::write(&html_path, html_content)
        .map_err(|e| format!("Failed to write HTML report: {}", e))?;
    println!("📊 Saved premium HTML report to {}", html_path.display());

    Ok(())
}

const HTML_TEMPLATE: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Sonar Report - __PROJECT_NAME__</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700;800&family=JetBrains+Mono:wght@400;500;700&display=swap" rel="stylesheet">
    <style>
        :root {
            /* Light Theme Palette */
            --bg: #f8fafc;
            --panel: #ffffff;
            --text: #0f172a;
            --text-secondary: #64748b;
            --border: #e2e8f0;
            --primary: #4f46e5;
            --primary-hover: #4338ca;
            --primary-light: #e0e7ff;
            --success: #10b981;
            --success-light: #d1fae5;
            --warning: #f59e0b;
            --warning-light: #fef3c7;
            --danger: #ef4444;
            --danger-light: #fee2e2;
            --info: #06b6d4;
            --shadow: 0 4px 6px -1px rgb(0 0 0 / 0.05), 0 2px 4px -2px rgb(0 0 0 / 0.05);
            --shadow-lg: 0 10px 15px -3px rgb(0 0 0 / 0.05), 0 4px 6px -4px rgb(0 0 0 / 0.05);
            --radius: 12px;
            --transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
        }

        [data-theme="dark"] {
            /* Dark Theme Palette */
            --bg: #0b0f19;
            --panel: #151c2c;
            --text: #f8fafc;
            --text-secondary: #94a3b8;
            --border: #222d44;
            --primary: #6366f1;
            --primary-hover: #4f46e5;
            --primary-light: rgba(99, 102, 241, 0.15);
            --success: #10b981;
            --success-light: rgba(16, 185, 129, 0.15);
            --warning: #f59e0b;
            --warning-light: rgba(245, 158, 11, 0.15);
            --danger: #f87171;
            --danger-light: rgba(248, 113, 113, 0.15);
            --info: #22d3ee;
            --shadow: 0 4px 6px -1px rgb(0 0 0 / 0.25), 0 2px 4px -2px rgb(0 0 0 / 0.20);
            --shadow-lg: 0 10px 15px -3px rgb(0 0 0 / 0.3), 0 4px 6px -4px rgb(0 0 0 / 0.2);
        }

        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }

        body {
            font-family: 'Outfit', sans-serif;
            background-color: var(--bg);
            color: var(--text);
            min-height: 100vh;
            display: flex;
            transition: var(--transition);
        }

        /* Sidebar Navigation */
        aside {
            width: 280px;
            background-color: var(--panel);
            border-right: 1px solid var(--border);
            padding: 2rem 1.5rem;
            display: flex;
            flex-direction: column;
            gap: 2rem;
            position: fixed;
            height: 100vh;
            left: 0;
            top: 0;
            z-index: 10;
        }

        .logo-area {
            display: flex;
            align-items: center;
            gap: 0.75rem;
            font-size: 1.3rem;
            font-weight: 800;
            color: var(--text);
        }

        .logo-area svg {
            width: 32px;
            height: 32px;
            fill: var(--primary);
        }

        nav {
            display: flex;
            flex-direction: column;
            gap: 0.5rem;
            flex: 1;
        }

        .nav-btn {
            display: flex;
            align-items: center;
            gap: 0.75rem;
            padding: 0.8rem 1rem;
            border: none;
            background: transparent;
            color: var(--text-secondary);
            font-family: inherit;
            font-size: 0.95rem;
            font-weight: 500;
            border-radius: var(--radius);
            cursor: pointer;
            text-align: left;
            transition: var(--transition);
        }

        .nav-btn:hover {
            color: var(--text);
            background-color: var(--bg);
        }

        .nav-btn.active {
            color: var(--primary);
            background-color: var(--primary-light);
            font-weight: 600;
        }

        .nav-btn svg {
            width: 20px;
            height: 20px;
        }

        .theme-toggle-area {
            border-top: 1px solid var(--border);
            padding-top: 1.5rem;
        }

        .theme-toggle-btn {
            width: 100%;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 0.5rem;
            padding: 0.8rem;
            border: 1px solid var(--border);
            background-color: var(--panel);
            color: var(--text);
            border-radius: var(--radius);
            cursor: pointer;
            font-family: inherit;
            font-weight: 600;
            transition: var(--transition);
        }

        .theme-toggle-btn:hover {
            background-color: var(--bg);
            border-color: var(--text-secondary);
        }

        /* Main Content Panel */
        main {
            margin-left: 280px;
            flex: 1;
            padding: 2.5rem;
            max-width: 1400px;
        }

        header {
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
            margin-bottom: 2.5rem;
        }

        .title-group h1 {
            font-size: 2.2rem;
            font-weight: 800;
            background: linear-gradient(135deg, var(--primary), #a855f7);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            margin-bottom: 0.25rem;
        }

        .title-group p {
            color: var(--text-secondary);
            font-size: 0.95rem;
        }

        .timestamp {
            font-size: 0.85rem;
            color: var(--text-secondary);
            background-color: var(--panel);
            padding: 0.5rem 1rem;
            border-radius: 20px;
            border: 1px solid var(--border);
            box-shadow: var(--shadow);
        }

        /* Tabs Panels */
        .tab-panel {
            display: none;
            animation: fadeIn 0.3s ease;
        }

        .tab-panel.active {
            display: block;
        }

        @keyframes fadeIn {
            from { opacity: 0; transform: translateY(8px); }
            to { opacity: 1; transform: translateY(0); }
        }

        /* Dashboard Overview */
        .summary-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
            gap: 1.5rem;
            margin-bottom: 2rem;
        }

        .summary-card {
            background-color: var(--panel);
            border: 1px solid var(--border);
            border-radius: var(--radius);
            padding: 1.5rem;
            box-shadow: var(--shadow);
            display: flex;
            flex-direction: column;
            gap: 0.5rem;
        }

        .summary-card h3 {
            font-size: 0.9rem;
            color: var(--text-secondary);
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }

        .summary-card .value {
            font-size: 2rem;
            font-weight: 800;
            color: var(--text);
        }

        .summary-card.accent-green { border-left: 4px solid var(--success); }
        .summary-card.accent-yellow { border-left: 4px solid var(--warning); }
        .summary-card.accent-red { border-left: 4px solid var(--danger); }
        .summary-card.accent-blue { border-left: 4px solid var(--info); }

        /* Dashboard Core View */
        .dashboard-content {
            display: grid;
            grid-template-columns: 1fr 2fr;
            gap: 2rem;
        }

        .score-circle-panel {
            background-color: var(--panel);
            border: 1px solid var(--border);
            border-radius: var(--radius);
            padding: 2.5rem;
            box-shadow: var(--shadow-lg);
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            text-align: center;
            gap: 1.5rem;
        }

        .gauge-container {
            position: relative;
            width: 200px;
            height: 200px;
        }

        .gauge-svg {
            transform: rotate(-90deg);
            width: 100%;
            height: 100%;
        }

        .gauge-bg {
            fill: none;
            stroke: var(--border);
            stroke-width: 12px;
        }

        .gauge-bar {
            fill: none;
            stroke: url(#scoreGradient);
            stroke-dasharray: 565.48; /* 2 * pi * 90 */
            stroke-dashoffset: 565.48;
            stroke-width: 12px;
            stroke-linecap: round;
            transition: stroke-dashoffset 1s ease-out;
        }

        .gauge-content {
            position: absolute;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            display: flex;
            flex-direction: column;
            align-items: center;
        }

        .gauge-grade {
            font-size: 3.5rem;
            font-weight: 900;
            color: var(--text);
            line-height: 1;
        }

        .gauge-score {
            font-size: 1rem;
            color: var(--text-secondary);
            font-weight: 600;
            margin-top: 0.25rem;
        }

        .grade-description {
            font-size: 1.1rem;
            font-weight: 600;
            color: var(--text);
        }

        .metric-bars-panel {
            background-color: var(--panel);
            border: 1px solid var(--border);
            border-radius: var(--radius);
            padding: 2rem;
            box-shadow: var(--shadow);
            display: flex;
            flex-direction: column;
            gap: 1.5rem;
        }

        .metric-bar-group {
            display: flex;
            flex-direction: column;
            gap: 0.5rem;
        }

        .metric-bar-header {
            display: flex;
            justify-content: space-between;
            font-weight: 600;
            font-size: 0.95rem;
        }

        .progress-track {
            height: 10px;
            background-color: var(--bg);
            border-radius: 5px;
            overflow: hidden;
            width: 100%;
        }

        .progress-fill {
            height: 100%;
            border-radius: 5px;
            background-color: var(--primary);
            width: 0%;
            transition: width 0.8s ease-out;
        }

        /* Filter Controls */
        .filter-row {
            display: flex;
            gap: 1rem;
            margin-bottom: 1.5rem;
            flex-wrap: wrap;
            background-color: var(--panel);
            padding: 1rem;
            border-radius: var(--radius);
            border: 1px solid var(--border);
            box-shadow: var(--shadow);
            align-items: center;
        }

        .search-input {
            flex: 1;
            min-width: 250px;
            padding: 0.6rem 1rem;
            border-radius: 8px;
            border: 1px solid var(--border);
            background-color: var(--bg);
            color: var(--text);
            font-family: inherit;
            font-size: 0.9rem;
            outline: none;
            transition: var(--transition);
        }

        .search-input:focus {
            border-color: var(--primary);
            box-shadow: 0 0 0 2px var(--primary-light);
        }

        .select-filter {
            padding: 0.6rem 1rem;
            border-radius: 8px;
            border: 1px solid var(--border);
            background-color: var(--bg);
            color: var(--text);
            font-family: inherit;
            font-size: 0.9rem;
            outline: none;
            cursor: pointer;
            min-width: 160px;
        }

        /* Lists & Tables */
        .issues-list {
            display: flex;
            flex-direction: column;
            gap: 1rem;
        }

        .issue-card {
            background-color: var(--panel);
            border: 1px solid var(--border);
            border-radius: var(--radius);
            box-shadow: var(--shadow);
            overflow: hidden;
            transition: var(--transition);
        }

        .issue-header {
            padding: 1rem 1.5rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
            cursor: pointer;
            user-select: none;
        }

        .issue-info {
            display: flex;
            align-items: center;
            gap: 1rem;
        }

        .badge {
            font-size: 0.75rem;
            font-weight: 700;
            padding: 0.25rem 0.6rem;
            border-radius: 20px;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }

        .badge-error { background-color: var(--danger-light); color: var(--danger); }
        .badge-warning { background-color: var(--warning-light); color: var(--warning); }
        .badge-info { background-color: var(--primary-light); color: var(--primary); }
        .badge-note { background-color: var(--success-light); color: var(--success); }

        .issue-title {
            font-weight: 600;
            font-size: 0.95rem;
        }

        .issue-location {
            font-size: 0.85rem;
            color: var(--text-secondary);
            font-family: 'JetBrains Mono', monospace;
        }

        .issue-meta {
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.8rem;
            color: var(--text-secondary);
            background-color: var(--bg);
            padding: 0.25rem 0.5rem;
            border-radius: 4px;
            border: 1px solid var(--border);
        }

        .issue-body {
            display: none;
            padding: 1.5rem;
            border-top: 1px solid var(--border);
            background-color: var(--bg);
        }

        .issue-card.expanded .issue-body {
            display: block;
        }

        .code-snippet {
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.85rem;
            padding: 1rem;
            background-color: var(--panel);
            border-radius: 6px;
            border: 1px solid var(--border);
            overflow-x: auto;
            color: var(--text);
            margin-top: 0.5rem;
        }

        /* Coverage & Unsafe Tables */
        .table-panel {
            background-color: var(--panel);
            border: 1px solid var(--border);
            border-radius: var(--radius);
            box-shadow: var(--shadow);
            overflow: hidden;
        }

        table {
            width: 100%;
            border-collapse: collapse;
            text-align: left;
        }

        th, td {
            padding: 1rem 1.5rem;
            border-bottom: 1px solid var(--border);
        }

        th {
            background-color: var(--bg);
            font-weight: 600;
            color: var(--text-secondary);
            font-size: 0.85rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }

        tr:last-child td {
            border-bottom: none;
        }

        .file-path {
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.9rem;
            font-weight: 500;
        }

        .coverage-bar-cell {
            display: flex;
            align-items: center;
            gap: 1rem;
            width: 300px;
        }

        .coverage-percent {
            font-weight: 600;
            font-size: 0.9rem;
            width: 50px;
            text-align: right;
        }

        /* Responsive */
        @media (max-width: 1024px) {
            body {
                flex-direction: column;
            }
            aside {
                width: 100%;
                height: auto;
                position: relative;
                padding: 1.5rem;
                border-right: none;
                border-bottom: 1px solid var(--border);
            }
            main {
                margin-left: 0;
                padding: 1.5rem;
            }
            .dashboard-content {
                grid-template-columns: 1fr;
            }
        }
    </style>
</head>
<body>
    <aside>
        <div class="logo-area">
            <svg viewBox="0 0 24 24">
                <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z"/>
            </svg>
            <span>Cargo Sonar</span>
        </div>
        <nav>
            <button class="nav-btn active" onclick="switchTab('dashboard')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v4a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v4a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v4a2 2 0 01-2 2H6a2 2 0 01-2-2v-4zM14 16a2 2 0 012-2h2a2 2 0 012 2v4a2 2 0 01-2 2h-2a2 2 0 01-2-2v-4z"/></svg>
                Dashboard
            </button>
            <button class="nav-btn" onclick="switchTab('clippy')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/></svg>
                Clippy Lints
            </button>
            <button class="nav-btn" onclick="switchTab('security')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"/></svg>
                Security & Audits
            </button>
            <button class="nav-btn" onclick="switchTab('coverage')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"/></svg>
                Code Coverage
            </button>
            <button class="nav-btn" onclick="switchTab('unsafe')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/></svg>
                Unsafe Analysis
            </button>
        </nav>
        <div class="theme-toggle-area">
            <button class="theme-toggle-btn" onclick="toggleTheme()">
                <span>🌓</span> Toggle Theme
            </button>
        </div>
    </aside>

    <main>
        <header>
            <div class="title-group">
                <h1 id="headerProjectName">__PROJECT_NAME__</h1>
                <p>Code Quality and Technical Debt Analysis Dashboard</p>
            </div>
            <div class="timestamp" id="headerTimestamp">Date Generated</div>
        </header>

        <!-- DASHBOARD PANEL -->
        <section id="dashboardPanel" class="tab-panel active">
            <div class="summary-grid">
                <div class="summary-card accent-blue">
                    <h3>Total Scanned Files</h3>
                    <div class="value" id="cardFiles">0</div>
                </div>
                <div class="summary-card accent-yellow">
                    <h3>Clippy Issues</h3>
                    <div class="value" id="cardLints">0</div>
                </div>
                <div class="summary-card accent-red">
                    <h3>Security Issues</h3>
                    <div class="value" id="cardSecurity">0</div>
                </div>
                <div class="summary-card accent-green">
                    <h3>Test Coverage</h3>
                    <div class="value" id="cardCoverage">0%</div>
                </div>
            </div>

            <div class="dashboard-content">
                <div class="score-circle-panel">
                    <h3>Overall Quality Grade</h3>
                    <div class="gauge-container">
                        <svg class="gauge-svg" viewBox="0 0 200 200">
                            <defs>
                                <linearGradient id="scoreGradient" x1="0%" y1="0%" x2="100%" y2="100%">
                                    <stop offset="0%" stop-color="#4f46e5" />
                                    <stop offset="100%" stop-color="#10b981" />
                                </linearGradient>
                            </defs>
                            <circle class="gauge-bg" cx="100" cy="100" r="90"></circle>
                            <circle class="gauge-bar" id="scoreGaugeBar" cx="100" cy="100" r="90"></circle>
                        </svg>
                        <div class="gauge-content">
                            <span class="gauge-grade" id="badgeGrade">A</span>
                            <span class="gauge-score" id="badgeScore">0.0</span>
                        </div>
                    </div>
                    <div class="grade-description" id="gradeDescription">Codebase health is in excellent standing</div>
                </div>

                <div class="metric-bars-panel">
                    <h3>Quality Dimensions</h3>
                    <div class="metric-bar-group">
                        <div class="metric-bar-header">
                            <span>Lints & Code Style</span>
                            <span id="labelClippyMetric">0%</span>
                        </div>
                        <div class="progress-track">
                            <div class="progress-fill" id="barClippyMetric"></div>
                        </div>
                    </div>
                    <div class="metric-bar-group">
                        <div class="metric-bar-header">
                            <span>Security & Dependencies</span>
                            <span id="labelSecurityMetric">0%</span>
                        </div>
                        <div class="progress-track">
                            <div class="progress-fill" id="barSecurityMetric"></div>
                        </div>
                    </div>
                    <div class="metric-bar-group">
                        <div class="metric-bar-header">
                            <span>Test Coverage</span>
                            <span id="labelCoverageMetric">0%</span>
                        </div>
                        <div class="progress-track">
                            <div class="progress-fill" id="barCoverageMetric"></div>
                        </div>
                    </div>
                    <div class="metric-bar-group">
                        <div class="metric-bar-header">
                            <span>Memory & Code Safety (Safe blocks)</span>
                            <span id="labelUnsafeMetric">0%</span>
                        </div>
                        <div class="progress-track">
                            <div class="progress-fill" id="barUnsafeMetric"></div>
                        </div>
                    </div>
                </div>
            </div>
        </section>

        <!-- CLIPPY PANEL -->
        <section id="clippyPanel" class="tab-panel">
            <div class="filter-row">
                <input type="text" class="search-input" id="searchClippy" placeholder="Search by lint rule or message..." oninput="filterClippy()">
                <select class="select-filter" id="filterClippyLevel" onchange="filterClippy()">
                    <option value="all">All Levels</option>
                    <option value="error">Error</option>
                    <option value="warning">Warning</option>
                    <option value="help">Help</option>
                    <option value="note">Note</option>
                </select>
            </div>
            <div class="issues-list" id="clippyList">
                <!-- Populated by JavaScript -->
            </div>
        </section>

        <!-- SECURITY PANEL -->
        <section id="securityPanel" class="tab-panel">
            <div class="filter-row">
                <input type="text" class="search-input" id="searchSecurity" placeholder="Search packages, vulnerabilities, licenses..." oninput="filterSecurity()">
                <select class="select-filter" id="filterSecurityCat" onchange="filterSecurity()">
                    <option value="all">All Categories</option>
                    <option value="vulnerability">Vulnerability</option>
                    <option value="license">License Violations</option>
                    <option value="banned">Banned Crates</option>
                </select>
            </div>
            <div class="issues-list" id="securityList">
                <!-- Populated by JavaScript -->
            </div>
        </section>

        <!-- COVERAGE PANEL -->
        <section id="coveragePanel" class="tab-panel">
            <div class="filter-row">
                <input type="text" class="search-input" id="searchCoverage" placeholder="Search file paths..." oninput="filterCoverage()">
            </div>
            <div class="table-panel">
                <table>
                    <thead>
                        <tr>
                            <th>File</th>
                            <th>Covered Lines</th>
                            <th>Total Lines</th>
                            <th>Coverage Percentage</th>
                        </tr>
                    </thead>
                    <tbody id="coverageTableBody">
                        <!-- Populated by JavaScript -->
                    </tbody>
                </table>
            </div>
        </section>

        <!-- UNSAFE PANEL -->
        <section id="unsafePanel" class="tab-panel">
            <div class="table-panel">
                <table>
                    <thead>
                        <tr>
                            <th>File</th>
                            <th>Unsafe Functions</th>
                            <th>Unsafe Impls</th>
                            <th>Unsafe Blocks</th>
                        </tr>
                    </thead>
                    <tbody id="unsafeTableBody">
                        <!-- Populated by JavaScript -->
                    </tbody>
                </table>
            </div>
        </section>
    </main>

    <script>
        const report = __REPORT_JSON__;

        // Initialize Page Content
        document.addEventListener('DOMContentLoaded', () => {
            // Apply theme on load
            const savedTheme = localStorage.getItem('theme') || 'light';
            document.documentElement.setAttribute('data-theme', savedTheme);

            loadDashboardSummary();
            renderClippyIssues();
            renderSecurityIssues();
            renderCoverageTable();
            renderUnsafeTable();
        });

        // Toggle Dark/Light Mode
        function toggleTheme() {
            const currentTheme = document.documentElement.getAttribute('data-theme');
            const newTheme = currentTheme === 'dark' ? 'light' : 'dark';
            document.documentElement.setAttribute('data-theme', newTheme);
            localStorage.setItem('theme', newTheme);
        }

        // Tab Navigation switching
        function switchTab(tabId) {
            document.querySelectorAll('.nav-btn').forEach(btn => btn.classList.remove('active'));
            document.querySelectorAll('.tab-panel').forEach(panel => panel.classList.remove('active'));

            event.currentTarget.classList.add('active');
            document.getElementById(`${tabId}Panel`).classList.add('active');
        }

        // Render Dashboard Overview
        function loadDashboardSummary() {
            const sum = report.summary;
            document.getElementById('headerTimestamp').innerText = new Date(sum.timestamp).toLocaleString();
            document.getElementById('cardFiles').innerText = sum.total_files;
            document.getElementById('cardLints').innerText = sum.total_clippy_errors + sum.total_clippy_warnings;
            document.getElementById('cardSecurity').innerText = sum.total_security_vulnerabilities + sum.total_license_issues;
            document.getElementById('cardCoverage').innerText = `${sum.overall_coverage.toFixed(1)}%`;

            // Grade details
            document.getElementById('badgeGrade').innerText = sum.health_grade;
            document.getElementById('badgeScore').innerText = sum.overall_score.toFixed(1);

            let desc = "Codebase health is in excellent standing";
            if (sum.health_grade === 'B') desc = "Codebase is healthy, with minor issues to solve";
            if (sum.health_grade === 'C') desc = "Moderate technical debt detected; consider refactoring";
            if (sum.health_grade === 'D') desc = "Significant quality issues, warnings require immediate review";
            if (sum.health_grade === 'F') desc = "Critical warning state. Immediate audit required!";
            document.getElementById('gradeDescription').innerText = desc;

            // Set Radial Gauge
            const radius = 90;
            const circumference = 2 * Math.PI * radius;
            const strokeOffset = circumference - (sum.overall_score / 100) * circumference;
            document.getElementById('scoreGaugeBar').style.strokeDashoffset = strokeOffset;

            // Set Dimension Bars
            // Calculate mock dimensional percentages for illustration based on grade
            const clippyErrWarn = sum.total_clippy_errors * 5.0 + sum.total_clippy_warnings * 1.5;
            const clippyPct = Math.max(0, 100 - clippyErrWarn);
            
            const secIssues = sum.total_security_vulnerabilities * 30.0 + sum.total_license_issues * 15.0;
            const secPct = Math.max(0, 100 - secIssues);
            
            const covPct = sum.overall_coverage;
            const unsafePct = Math.max(0, 100 - (sum.total_unsafe_blocks * 2.0));

            setMetric('Clippy', clippyPct);
            setMetric('Security', secPct);
            setMetric('Coverage', covPct);
            setMetric('Unsafe', unsafePct);
        }

        function setMetric(name, percentage) {
            document.getElementById(`label${name}Metric`).innerText = `${percentage.toFixed(0)}%`;
            document.getElementById(`bar${name}Metric`).style.width = `${percentage}%`;
        }

        // Render Clippy list
        function renderClippyIssues() {
            const container = document.getElementById('clippyList');
            container.innerHTML = '';

            if (report.clippy_issues.length === 0) {
                container.innerHTML = '<div style="padding: 2rem; text-align: center; color: var(--text-secondary);">🎉 No Clippy lints found! Great job.</div>';
                return;
            }

            report.clippy_issues.forEach((issue, idx) => {
                const badgeClass = `badge badge-${getBadgeType(issue.level)}`;
                const card = document.createElement('div');
                card.className = 'issue-card';
                card.id = `clippy-${idx}`;
                card.innerHTML = `
                    <div class="issue-header" onclick="toggleExpand('clippy-${idx}')">
                        <div class="issue-info">
                            <span class="${badgeClass}">${getLintLevelStr(issue.level)}</span>
                            <span class="issue-title">${issue.message}</span>
                            <span class="issue-location">${issue.location.file}:${issue.location.line}:${issue.location.column}</span>
                        </div>
                        <span class="issue-meta">${issue.lint_name}</span>
                    </div>
                    <div class="issue-body">
                        <strong>Source File Location:</strong> ${issue.location.file} (Line ${issue.location.line})
                        ${issue.location.snippet ? `
                        <div class="code-snippet">
                            <code>${escapeHtml(issue.location.snippet)}</code>
                        </div>` : ''}
                    </div>
                `;
                container.appendChild(card);
            });
        }

        function toggleExpand(id) {
            const element = document.getElementById(id);
            element.classList.toggle('expanded');
        }

        function getBadgeType(level) {
            if (level === 'Error' || level.Error !== undefined) return 'error';
            if (level === 'Warning' || level.Warning !== undefined) return 'warning';
            if (level === 'Help' || level.Help !== undefined) return 'info';
            return 'note';
        }

        function getLintLevelStr(level) {
            if (typeof level === 'string') return level;
            return Object.keys(level)[0];
        }

        function escapeHtml(text) {
            return text
                .replace(/&/g, "&amp;")
                .replace(/</g, "&lt;")
                .replace(/>/g, "&gt;")
                .replace(/"/g, "&quot;")
                .replace(/'/g, "&#039;");
        }

        // Render Security Panel
        function renderSecurityIssues() {
            const container = document.getElementById('securityList');
            container.innerHTML = '';

            if (report.security_issues.length === 0) {
                container.innerHTML = '<div style="padding: 2rem; text-align: center; color: var(--text-secondary);">🛡️ Dependency scan clean! Zero vulnerabilities found.</div>';
                return;
            }

            report.security_issues.forEach((issue, idx) => {
                const badge = issue.category === 'Vulnerability' || issue.category.Vulnerability !== undefined ? 'badge-error' : 'badge-warning';
                let catStr = typeof issue.category === 'string' ? issue.category : Object.keys(issue.category)[0];
                if (catStr === 'LicenseViolation') catStr = 'License';
                
                const card = document.createElement('div');
                card.className = 'issue-card';
                card.id = `security-${idx}`;
                card.innerHTML = `
                    <div class="issue-header" onclick="toggleExpand('security-${idx}')">
                        <div class="issue-info">
                            <span class="badge ${badge}">${catStr}</span>
                            <span class="issue-title" style="font-weight: 700;">${issue.package_name} v${issue.package_version}</span>
                            <span class="issue-location" style="font-family: inherit;">Severity: ${issue.severity}</span>
                        </div>
                    </div>
                    <div class="issue-body">
                        <p style="margin-bottom: 0.75rem;">${issue.message}</p>
                        ${issue.advisory_url ? `<a href="${issue.advisory_url}" target="_blank" style="color: var(--primary); font-weight: 600; text-decoration: none;">🔗 View Advisory Details</a>` : ''}
                    </div>
                `;
                container.appendChild(card);
            });
        }

        // Render Test Coverage Table
        function renderCoverageTable() {
            const tbody = document.getElementById('coverageTableBody');
            tbody.innerHTML = '';

            if (report.coverage.length === 0) {
                tbody.innerHTML = '<tr><td colspan="4" style="text-align: center; color: var(--text-secondary);">No test coverage data found. Run with cargo-tarpaulin.</td></tr>';
                return;
            }

            report.coverage.forEach(cov => {
                const tr = document.createElement('tr');
                tr.innerHTML = `
                    <td class="file-path">${cov.file}</td>
                    <td>${cov.covered_lines}</td>
                    <td>${cov.total_lines}</td>
                    <td>
                        <div class="coverage-bar-cell">
                            <div class="progress-track" style="width: 150px; height: 8px;">
                                <div class="progress-fill" style="width: ${cov.percentage}%; background-color: ${getCoverageColor(cov.percentage)}"></div>
                            </div>
                            <span class="coverage-percent" style="color: ${getCoverageColor(cov.percentage)}">${cov.percentage.toFixed(1)}%</span>
                        </div>
                    </td>
                `;
                tbody.appendChild(tr);
            });
        }

        function getCoverageColor(pct) {
            if (pct >= 80) return 'var(--success)';
            if (pct >= 60) return 'var(--warning)';
            return 'var(--danger)';
        }

        // Render Unsafe code safety metrics
        function renderUnsafeTable() {
            const tbody = document.getElementById('unsafeTableBody');
            tbody.innerHTML = '';

            if (report.unsafe_metrics.length === 0) {
                tbody.innerHTML = '<tr><td colspan="4" style="text-align: center; color: var(--text-secondary);">No unsafe code metrics scanned.</td></tr>';
                return;
            }

            report.unsafe_metrics.forEach(u => {
                const tr = document.createElement('tr');
                tr.innerHTML = `
                    <td class="file-path">${u.file}</td>
                    <td>${u.unsafe_functions}</td>
                    <td>${u.unsafe_impls}</td>
                    <td style="font-weight: 700; color: ${u.unsafe_blocks > 0 ? 'var(--danger)' : 'inherit'}">${u.unsafe_blocks}</td>
                `;
                tbody.appendChild(tr);
            });
        }

        // Filters implementations
        function filterClippy() {
            const search = document.getElementById('searchClippy').value.toLowerCase();
            const level = document.getElementById('filterClippyLevel').value;

            document.querySelectorAll('#clippyList .issue-card').forEach((card, idx) => {
                const issue = report.clippy_issues[idx];
                const text = (issue.message + ' ' + issue.lint_name + ' ' + issue.location.file).toLowerCase();
                const levelStr = getLintLevelStr(issue.level).toLowerCase();

                const matchesSearch = text.includes(search);
                const matchesLevel = level === 'all' || levelStr === level;

                if (matchesSearch && matchesLevel) {
                    card.style.display = 'block';
                } else {
                    card.style.display = 'none';
                }
            });
        }

        function filterSecurity() {
            const search = document.getElementById('searchSecurity').value.toLowerCase();
            const category = document.getElementById('filterSecurityCat').value;

            document.querySelectorAll('#securityList .issue-card').forEach((card, idx) => {
                const issue = report.security_issues[idx];
                const text = (issue.package_name + ' ' + issue.message + ' ' + issue.severity).toLowerCase();
                let catStr = typeof issue.category === 'string' ? issue.category : Object.keys(issue.category)[0];
                catStr = catStr.toLowerCase();

                const matchesSearch = text.includes(search);
                
                let matchesCat = category === 'all';
                if (!matchesCat) {
                    if (category === 'vulnerability' && catStr === 'vulnerability') matchesCat = true;
                    if (category === 'license' && (catStr === 'licenseviolation' || catStr === 'license')) matchesCat = true;
                    if (category === 'banned' && catStr === 'bannedcrate') matchesCat = true;
                }

                if (matchesSearch && matchesCat) {
                    card.style.display = 'block';
                } else {
                    card.style.display = 'none';
                }
            });
        }

        function filterCoverage() {
            const search = document.getElementById('searchCoverage').value.toLowerCase();
            const rows = document.querySelectorAll('#coverageTableBody tr');

            rows.forEach((row, idx) => {
                const file = report.coverage[idx].file.toLowerCase();
                if (file.includes(search)) {
                    row.style.display = '';
                } else {
                    row.style.display = 'none';
                }
            });
        }
    </script>
</body>
</html>
"##;
