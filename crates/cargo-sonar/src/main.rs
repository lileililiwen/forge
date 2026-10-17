mod model;
mod analyzer;
mod exporter;
mod demo;

use clap::Parser;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    #[command(name = "sonar-boost")]
    SonarBoost(SonarArgs),
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "🛡️ Zero-config code quality & technical debt dashboard for Rust")]
struct SonarArgs {
    /// Run in simulation mode with a mock project dataset to test the visual dashboard
    #[arg(long)]
    demo: bool,

    /// Do not automatically open the generated HTML report in the browser
    #[arg(long)]
    no_open: bool,

    /// Custom target directory for writing JSON and HTML reports
    #[arg(short, long, default_value = "target/sonar")]
    output_dir: String,

    /// Directory of the Rust project to scan (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    project_dir: String,
}

#[tokio::main]
async fn main() {
    // Cargo subcommands pass the subcommand name ("sonar") as the first argument,
    // so we parse it via a top-level enum wrapper `CargoCli`
    let args = match CargoCli::try_parse() {
        Ok(CargoCli::SonarBoost(args)) => args,
        _ => {
            // Handle running the binary directly as `cargo-sonar-boost` instead of `cargo sonar-boost`
            match SonarArgs::try_parse() {
                Ok(args) => args,
                Err(e) => {
                    e.exit();
                }
            }
        }
    };

    let start_time = Instant::now();
    let output_path = PathBuf::from(&args.output_dir);

    if args.demo {
        println!("🚀 Running cargo-sonar in Simulation Mode...");
        let report = demo::generate_demo_report();
        if let Err(e) = exporter::export_report(&report, &output_path) {
            eprintln!("❌ Failed to export demo report: {}", e);
            std::process::exit(1);
        }
        println!("✨ Simulation complete in {:.2?}!", start_time.elapsed());
        
        if !args.no_open {
            open_report_in_browser(&output_path);
        }
        return;
    }

    let project_path = PathBuf::from(&args.project_dir);
    let cargo_toml = project_path.join("Cargo.toml");

    if !cargo_toml.exists() {
        eprintln!("❌ Error: No Cargo.toml found in {}.", project_path.display());
        eprintln!("Please run cargo-sonar in the root of a Rust project or use --project-dir.");
        std::process::exit(1);
    }

    let project_name = get_project_name(&project_path);
    println!("🛡️ Starting Sonar check for project: {}...", project_name);

    // Run all analysis modules concurrently
    let clippy_fut = analyzer::run_clippy(&project_path);
    let deny_fut = analyzer::run_deny(&project_path);
    let geiger_fut = analyzer::run_geiger(&project_path);
    let tarpaulin_fut = analyzer::run_tarpaulin(&project_path);

    let (clippy_res, deny_res, geiger_res, tarpaulin_res) = tokio::join!(
        clippy_fut,
        deny_fut,
        geiger_fut,
        tarpaulin_fut
    );

    let clippy_issues = clippy_res.unwrap_or_else(|e| {
        eprintln!("⚠️ Clippy failed: {}", e);
        Vec::new()
    });

    let security_issues = deny_res.unwrap_or_else(|e| {
        eprintln!("⚠️ Dependency check failed: {}", e);
        Vec::new()
    });

    let unsafe_metrics = geiger_res.unwrap_or_else(|e| {
        eprintln!("⚠️ Unsafe code scan failed: {}", e);
        Vec::new()
    });

    let coverage_metrics = tarpaulin_res.unwrap_or_else(|e| {
        eprintln!("⚠️ Coverage scan failed: {}", e);
        Vec::new()
    });

    let total_files = count_rs_files(&project_path);

    // Calculate final grade metrics and score
    let report = model::SonarReport::calculate_grade(
        project_name,
        &clippy_issues,
        &security_issues,
        &coverage_metrics,
        &unsafe_metrics,
        total_files,
    );

    // Export report
    if let Err(e) = exporter::export_report(&report, &output_path) {
        eprintln!("❌ Failed to export reports: {}", e);
        std::process::exit(1);
    }

    println!("🎉 Sonar check completed successfully in {:.2?}!", start_time.elapsed());
    println!("📊 Overall Grade: {} (Score: {:.1}/100.0)", report.summary.health_grade, report.summary.overall_score);

    if !args.no_open {
        open_report_in_browser(&output_path);
    }
}

/// Helper to parse the package name from Cargo.toml
fn get_project_name(path: &Path) -> String {
    let cargo_toml_path = path.join("Cargo.toml");
    if cargo_toml_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_toml_path) {
            let mut in_package = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed == "[package]" {
                    in_package = true;
                } else if trimmed.starts_with('[') {
                    in_package = false;
                }
                if in_package && trimmed.starts_with("name") {
                    if let Some(eq_idx) = trimmed.find('=') {
                        let name_val = trimmed[eq_idx + 1..].trim();
                        return name_val.trim_matches('"').trim_matches('\'').to_string();
                    }
                }
            }
        }
    }
    "Rust Project".to_string()
}

/// Helper to count the total number of .rs files in the workspace (excluding build dirs)
fn count_rs_files(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name != "target" && name != ".git" && name != ".github" {
                    count += count_rs_files(&path);
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                count += 1;
            }
        }
    }
    count
}

/// Open generated report in browser
fn open_report_in_browser(output_path: &Path) {
    let report_html = output_path.join("report.html");
    if report_html.exists() {
        println!("🌐 Opening dashboard in browser...");
        if let Err(e) = webbrowser::open(report_html.to_str().unwrap_or("")) {
            eprintln!("⚠️ Failed to launch browser: {}", e);
            println!("Please view the report manually at: file://{}", report_html.canonicalize().unwrap_or(report_html).display());
        }
    }
}
