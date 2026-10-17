use crate::model::{ClippyIssue, CodeLocation, CoverageMetric, IssueLevel, SecurityIssue, SecurityIssueCategory, UnsafeMetric};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use serde_json::Value;

/// Checks if a cargo subcommand/binary is available by running it with --version
async fn check_tool_exists(tool: &str) -> bool {
    Command::new("cargo")
        .arg(tool)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Runs `cargo clippy --message-format=json` and parses its output
pub async fn run_clippy(project_path: &Path) -> Result<Vec<ClippyIssue>, String> {
    println!("🔍 Running Clippy checks...");
    
    let mut child = Command::new("cargo")
        .current_dir(project_path)
        .args(&["clippy", "--message-format=json", "--", "-D", "warnings"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null()) // Mute compilation stderr to keep output clean
        .spawn()
        .map_err(|e| format!("Failed to spawn cargo clippy: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to open clippy stdout")?;
    let mut reader = BufReader::new(stdout).lines();
    let mut issues = Vec::new();

    while let Ok(Some(line)) = reader.next_line().await {
        if let Ok(value) = serde_json::from_str::<Value>(&line) {
            if value.get("reason").and_then(|r| r.as_str()) == Some("compiler-message") {
                if let Some(msg) = value.get("message") {
                    let level_str = msg.get("level").and_then(|l| l.as_str()).unwrap_or("warning");
                    let level = match level_str {
                        "error" => IssueLevel::Error,
                        "warning" => IssueLevel::Warning,
                        "help" => IssueLevel::Help,
                        "note" => IssueLevel::Note,
                        _ => IssueLevel::Warning,
                    };

                    let message = msg.get("message").and_then(|m| m.as_str()).unwrap_or("").to_string();
                    let lint_name = msg.get("code")
                        .and_then(|c| c.get("code"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("compiler-error")
                        .to_string();

                    // Find primary span
                    if let Some(spans) = msg.get("spans").and_then(|s| s.as_array()) {
                        let primary_span = spans.iter().find(|s| s.get("is_primary").and_then(|ip| ip.as_bool()) == Some(true))
                            .or_else(|| spans.first());

                        if let Some(span) = primary_span {
                            let file = span.get("file_name").and_then(|f| f.as_str()).unwrap_or("").to_string();
                            
                            // Skip external crates files or macro expansions outside project
                            if file.starts_with('/') || file.contains(".cargo") {
                                continue;
                            }

                            let line_num = span.get("line_start").and_then(|l| l.as_u64()).unwrap_or(0) as usize;
                            let col_num = span.get("column_start").and_then(|c| c.as_u64()).unwrap_or(0) as usize;

                            let snippet = span.get("text")
                                .and_then(|t| t.as_array())
                                .and_then(|arr| arr.first())
                                .and_then(|first_line| first_line.get("text"))
                                .and_then(|txt| txt.as_str())
                                .map(|s| s.to_string());

                            issues.push(ClippyIssue {
                                level,
                                lint_name,
                                message,
                                location: CodeLocation {
                                    file,
                                    line: line_num,
                                    column: col_num,
                                    snippet,
                                },
                            });
                        }
                    }
                }
            }
        }
    }

    let _ = child.wait().await;
    println!("✅ Clippy completed. Found {} issue(s).", issues.len());
    Ok(issues)
}

/// Runs `cargo deny check --format json` if cargo-deny is installed
pub async fn run_deny(project_path: &Path) -> Result<Vec<SecurityIssue>, String> {
    if !check_tool_exists("deny").await {
        println!("⚠️ cargo-deny is not installed. Skipping dependency audit.");
        return Ok(Vec::new());
    }

    println!("🛡️ Auditing dependencies with cargo-deny...");
    let mut child = Command::new("cargo")
        .current_dir(project_path)
        .args(&["deny", "check", "--format", "json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn cargo deny: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to open deny stdout")?;
    let mut reader = BufReader::new(stdout).lines();
    let mut issues = Vec::new();

    while let Ok(Some(line)) = reader.next_line().await {
        if let Ok(value) = serde_json::from_str::<Value>(&line) {
            // Check diagnostic outputs from cargo-deny
            if value.get("type").and_then(|t| t.as_str()) == Some("diagnostic") {
                if let Some(diag) = value.get("diagnostic") {
                    let severity = diag.get("severity").and_then(|s| s.as_str()).unwrap_or("warning").to_string();
                    let message = diag.get("message").and_then(|m| m.as_str()).unwrap_or("").to_string();
                    let code = diag.get("code").and_then(|c| c.as_str()).unwrap_or("");

                    let category = if code.contains("license") || message.to_lowercase().contains("license") {
                        SecurityIssueCategory::LicenseViolation
                    } else if code.contains("advisory") || message.to_lowercase().contains("vulnerab") {
                        SecurityIssueCategory::Vulnerability
                    } else {
                        SecurityIssueCategory::BannedCrate
                    };

                    issues.push(SecurityIssue {
                        category,
                        package_name: "Workspace Dependency".to_string(), // cargo-deny diagnostic reports are aggregate
                        package_version: "Unknown".to_string(),
                        severity,
                        message,
                        advisory_url: None,
                    });
                }
            }
        }
    }

    let _ = child.wait().await;
    println!("✅ cargo-deny completed. Found {} issue(s).", issues.len());
    Ok(issues)
}

/// Runs `cargo geiger --output-format json` if cargo-geiger is installed
pub async fn run_geiger(project_path: &Path) -> Result<Vec<UnsafeMetric>, String> {
    if !check_tool_exists("geiger").await {
        println!("⚠️ cargo-geiger is not installed. Skipping safety checks.");
        // Return default: assume 0 unsafe code for files we find
        return Ok(Vec::new());
    }

    println!("☣️ Checking unsafe blocks using cargo-geiger...");
    let mut child = Command::new("cargo")
        .current_dir(project_path)
        .args(&["geiger", "--output-format", "json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn cargo geiger: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to open geiger stdout")?;
    let mut reader = BufReader::new(stdout).lines();
    let mut metrics = Vec::new();
    let mut json_data = String::new();

    while let Ok(Some(line)) = reader.next_line().await {
        json_data.push_str(&line);
    }

    let _ = child.wait().await;

    if let Ok(value) = serde_json::from_str::<Value>(&json_data) {
        if let Some(packages) = value.get("packages").and_then(|p| p.as_array()) {
            for pkg in packages {
                // If it is the current workspace/local project
                if let Some(unsafety) = pkg.get("unsafety") {
                    // Gather metric details if geiger successfully compiled
                    if let Some(files) = unsafety.get("files").and_then(|f| f.as_object()) {
                        for (file_path, file_data) in files {
                            if file_path.contains(".cargo") || file_path.contains("/registry") {
                                continue;
                            }
                            let counts = file_data.get("unsafe_counter");
                            let unsafe_functions = counts.and_then(|c| c.get("functions")).and_then(|f| f.as_u64()).unwrap_or(0) as usize;
                            let unsafe_impls = counts.and_then(|c| c.get("exprs")).and_then(|f| f.as_u64()).unwrap_or(0) as usize; // geiger count mapping
                            let unsafe_blocks = counts.and_then(|c| c.get("item_impls")).and_then(|f| f.as_u64()).unwrap_or(0) as usize;

                            metrics.push(UnsafeMetric {
                                file: file_path.clone(),
                                unsafe_functions,
                                unsafe_impls,
                                unsafe_blocks,
                            });
                        }
                    }
                }
            }
        }
    }

    println!("✅ cargo-geiger completed.");
    Ok(metrics)
}

/// Runs `cargo tarpaulin --out Json` if cargo-tarpaulin is installed
pub async fn run_tarpaulin(project_path: &Path) -> Result<Vec<CoverageMetric>, String> {
    if !check_tool_exists("tarpaulin").await {
        println!("⚠️ cargo-tarpaulin is not installed. Skipping test coverage analysis.");
        return Ok(Vec::new());
    }

    println!("🧪 Measuring test coverage with cargo-tarpaulin...");
    let status = Command::new("cargo")
        .current_dir(project_path)
        .args(&["tarpaulin", "--out", "Json", "--output-dir", "target/sonar"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(|e| format!("Failed to run tarpaulin: {}", e))?;

    let mut metrics = Vec::new();
    if status.success() {
        let report_path = project_path.join("target/sonar/tarpaulin-report.json");
        if report_path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&report_path).await {
                if let Ok(value) = serde_json::from_str::<Value>(&content) {
                    if let Some(files) = value.get("files").and_then(|f| f.as_array()) {
                        for file in files {
                            let name = file.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                            let covered = file.get("covered").and_then(|c| c.as_u64()).unwrap_or(0) as usize;
                            let total = file.get("total").and_then(|t| t.as_u64()).unwrap_or(0) as usize;
                            let percentage = if total > 0 {
                                (covered as f64 / total as f64) * 100.0
                            } else {
                                100.0
                            };

                            metrics.push(CoverageMetric {
                                file: name,
                                covered_lines: covered,
                                total_lines: total,
                                percentage,
                            });
                        }
                    }
                }
            }
            let _ = tokio::fs::remove_file(report_path).await; // Clean up report file
        }
    }

    println!("✅ cargo-tarpaulin completed. Scanned {} file(s).", metrics.len());
    Ok(metrics)
}
