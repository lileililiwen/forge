mod demo;
mod exporter;
mod model;
mod resolver;
mod runner;
mod ui;

use crate::model::{JobStatus, MatrixReport};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    #[command(name = "matrix-boost")]
    MatrixBoost(MatrixArgs),
}

#[derive(Parser, Debug, Clone)]
#[command(
    author,
    version,
    about = "🚀 Multi-toolchain and feature matrix compiler & validator for Cargo"
)]
pub struct MatrixArgs {
    /// Run in simulation mode with a mock project dataset to test the TUI & visual dashboard
    #[arg(long)]
    pub demo: bool,

    /// Do not automatically open the generated HTML report in the browser
    #[arg(long)]
    pub no_open: bool,

    /// Custom target directory for writing JSON and HTML reports
    #[arg(short, long, default_value = "target/matrix")]
    pub output_dir: String,

    /// Directory of the Rust project to scan (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    pub project_dir: String,

    /// Comma-separated list of toolchains to check (e.g. stable,nightly). Defaults to current default toolchain.
    #[arg(short, long)]
    pub toolchains: Option<String>,

    /// Comma-separated list of features to test. If omitted, automatically resolves all features.
    #[arg(short, long)]
    pub features: Option<String>,

    /// Comma-separated list of features to exclude from testing matrix.
    #[arg(long)]
    pub exclude_features: Option<String>,

    /// Maximum number of feature combinations to generate (default: 32)
    #[arg(long, default_value = "32")]
    pub max_combinations: usize,

    /// Run tests (`test`) or only verify compilation (`check`)
    #[arg(short, long, default_value = "check")]
    pub command: String,

    /// Do not automatically run matrix builds with default features
    #[arg(long)]
    pub no_default: bool,

    /// Max parallel jobs concurrency limit (defaults to number of CPU cores)
    #[arg(long)]
    pub concurrency: Option<usize>,
}

#[tokio::main]
async fn main() {
    let args = match CargoCli::try_parse() {
        Ok(CargoCli::MatrixBoost(args)) => args,
        _ => match MatrixArgs::try_parse() {
            Ok(args) => args,
            Err(e) => {
                e.exit();
            }
        },
    };

    let start_time = Instant::now();
    let output_path = PathBuf::from(&args.output_dir);

    let jobs = Arc::new(Mutex::new(Vec::new()));
    let runner_finished = Arc::new(Mutex::new(false));

    let project_name = if args.demo {
        println!("🚀 Running cargo-matrix-boost in Simulation Mode...");

        let demo_report = demo::generate_demo_report();
        let mut initial_jobs = demo_report.jobs.clone();

        // Reset all job statuses for simulation progress visualization in TUI
        for j in &mut initial_jobs {
            j.status = JobStatus::Pending;
            j.log.clear();
            j.duration_ms = 0;
            j.error_message = None;
        }
        *jobs.lock().unwrap() = initial_jobs;

        let jobs_clone = Arc::clone(&jobs);
        let runner_finished_clone = Arc::clone(&runner_finished);
        let final_jobs = demo_report.jobs.clone();

        // Spawn mock simulator in the background
        tokio::spawn(async move {
            let concurrency = 3;
            let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
            let mut handles = Vec::new();

            for final_job in final_jobs {
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let jobs_inner = Arc::clone(&jobs_clone);
                let final_job = final_job.clone();

                let handle = tokio::spawn(async move {
                    // Transition to running
                    {
                        let mut guard = jobs_inner.lock().unwrap();
                        if let Some(j) = guard.iter_mut().find(|j| j.id == final_job.id) {
                            j.status = JobStatus::Running;
                        }
                    }

                    // Simulated build delay
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        400 + (final_job.id as u64 * 200) % 700,
                    ))
                    .await;

                    // Transition to completed status
                    {
                        let mut guard = jobs_inner.lock().unwrap();
                        if let Some(j) = guard.iter_mut().find(|j| j.id == final_job.id) {
                            j.status = final_job.status.clone();
                            j.duration_ms = final_job.duration_ms;
                            j.log = final_job.log.clone();
                            j.error_message = final_job.error_message.clone();
                        }
                    }
                    drop(permit);
                });
                handles.push(handle);
            }

            for h in handles {
                let _ = h.await;
            }
            *runner_finished_clone.lock().unwrap() = true;
        });

        "mock-crate".to_string()
    } else {
        let project_path = PathBuf::from(&args.project_dir);
        let cargo_toml = project_path.join("Cargo.toml");

        if !cargo_toml.exists() {
            eprintln!(
                "❌ Error: No Cargo.toml found in {}.",
                project_path.display()
            );
            eprintln!(
                "Please run cargo-matrix-boost in the root of a Rust project or use --project-dir."
            );
            std::process::exit(1);
        }

        // Parse toolchains
        let toolchain_list: Vec<String> = match &args.toolchains {
            Some(tc_str) => tc_str.split(',').map(|s| s.trim().to_string()).collect(),
            None => Vec::new(),
        };

        // Parse features
        let user_features: Option<Vec<String>> = args
            .features
            .as_ref()
            .map(|f_str| f_str.split(',').map(|s| s.trim().to_string()).collect());

        // Parse exclude features
        let exclude_features: Vec<String> = match &args.exclude_features {
            Some(ex_str) => ex_str.split(',').map(|s| s.trim().to_string()).collect(),
            None => Vec::new(),
        };

        // Resolve matrix configuration
        let include_default = !args.no_default;
        let (p_name, resolved_jobs) = match resolver::resolve_matrix(
            &project_path,
            &toolchain_list,
            user_features,
            &exclude_features,
            include_default,
            args.max_combinations,
        ) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("❌ Error: Failed to resolve feature matrix: {}", e);
                std::process::exit(1);
            }
        };

        let num_jobs = resolved_jobs.len();
        println!(
            "🛡️ Starting Matrix check for: {} ({} configs)",
            p_name, num_jobs
        );
        *jobs.lock().unwrap() = resolved_jobs;

        let jobs_clone = Arc::clone(&jobs);
        let runner_finished_clone = Arc::clone(&runner_finished);
        let project_dir_arc = Arc::new(project_path);
        let cargo_cmd = args.command.clone();

        let concurrency = args.concurrency.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        });

        // Spawn runner task
        tokio::spawn(async move {
            runner::run_matrix(project_dir_arc, jobs_clone, concurrency, cargo_cmd).await;
            *runner_finished_clone.lock().unwrap() = true;
        });

        p_name
    };

    // Run Terminal UI loop
    if let Err(e) = ui::run_tui(
        project_name.clone(),
        Arc::clone(&jobs),
        Arc::clone(&runner_finished),
    ) {
        eprintln!("⚠️ TUI Error: {}", e);
    }

    // Process results and export report
    let elapsed_ms = start_time.elapsed().as_millis() as u64;
    let final_jobs = jobs.lock().unwrap().clone();
    let total_jobs = final_jobs.len();
    let successful_jobs = final_jobs
        .iter()
        .filter(|j| j.status == JobStatus::Success)
        .count();
    let failed_jobs = final_jobs
        .iter()
        .filter(|j| j.status == JobStatus::Failed)
        .count();

    let report = MatrixReport {
        project_name,
        timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        elapsed_ms,
        total_jobs,
        successful_jobs,
        failed_jobs,
        jobs: final_jobs,
    };

    if let Err(e) = exporter::export_report(&report, &output_path) {
        eprintln!("❌ Failed to export reports: {}", e);
        std::process::exit(1);
    }

    println!("\n✨ Matrix validation run complete!");
    println!("📊 Succeeded: {}/{} configs", successful_jobs, total_jobs);
    if failed_jobs > 0 {
        println!("❌ Failed: {} configs", failed_jobs);
    }

    if !args.no_open {
        open_report_in_browser(&output_path);
    }
}

fn open_report_in_browser(output_path: &Path) {
    let report_html = output_path.join("report.html");
    if report_html.exists() {
        println!("🌐 Opening report dashboard in browser...");
        if let Err(e) = webbrowser::open(report_html.to_str().unwrap_or("")) {
            eprintln!("⚠️ Failed to launch browser: {}", e);
            println!(
                "Please view the report manually at: file://{}",
                report_html.canonicalize().unwrap_or(report_html).display()
            );
        }
    }
}
