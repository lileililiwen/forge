use crate::model::{JobStatus, MatrixJob};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::process::Command;
use tokio::sync::Semaphore;

pub async fn run_matrix(
    project_dir: Arc<std::path::PathBuf>,
    jobs: Arc<Mutex<Vec<MatrixJob>>>,
    concurrency: usize,
    cargo_cmd: String, // "check" or "test"
) {
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let num_jobs = {
        let guard = jobs.lock().unwrap();
        guard.len()
    };

    let mut handles = Vec::new();

    for id in 0..num_jobs {
        let semaphore_clone = Arc::clone(&semaphore);
        let jobs_clone = Arc::clone(&jobs);
        let project_dir_clone = Arc::clone(&project_dir);
        let cargo_cmd_clone = cargo_cmd.clone();

        let handle = tokio::spawn(async move {
            // Wait for concurrency permit
            let _permit = semaphore_clone.acquire().await.unwrap();

            // Set job status to Running
            {
                let mut guard = jobs_clone.lock().unwrap();
                if let Some(job) = guard.iter_mut().find(|j| j.id == id) {
                    job.status = JobStatus::Running;
                }
            }

            let start_time = Instant::now();

            // Extract job configuration details
            let (toolchain, features, no_default) = {
                let guard = jobs_clone.lock().unwrap();
                let job = guard.iter().find(|j| j.id == id).unwrap();
                (
                    job.toolchain.clone(),
                    job.features.clone(),
                    job.no_default_features,
                )
            };

            // Build Command
            let mut cmd = if let Some(tc) = &toolchain {
                let mut c = Command::new("rustup");
                c.arg("run").arg(tc).arg("cargo").arg(&cargo_cmd_clone);
                c
            } else {
                let mut c = Command::new("cargo");
                c.arg(&cargo_cmd_clone);
                c
            };

            // Apply feature arguments
            if no_default {
                cmd.arg("--no-default-features");
            }
            if !features.is_empty() {
                cmd.arg("--features").arg(features.join(","));
            }

            // Always run check --all-targets to ensure test files compile
            if cargo_cmd_clone == "check" {
                cmd.arg("--all-targets");
            }

            cmd.current_dir(&*project_dir_clone);

            // Execute the process and capture combined output
            let output_res = cmd.output().await;
            let duration = start_time.elapsed().as_millis() as u64;

            let (status, log, error_msg) = match output_res {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let combined_log = format!("{}{}", stdout, stderr);

                    if output.status.success() {
                        (JobStatus::Success, combined_log, None)
                    } else {
                        // Try to find a concise error message in stderr
                        let err_msg = stderr
                            .lines()
                            .find(|l| l.contains("error[E") || l.contains("error:"))
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| {
                                "Compilation failed. Check full logs below.".to_string()
                            });
                        (JobStatus::Failed, combined_log, Some(err_msg))
                    }
                }
                Err(e) => {
                    let err_msg = format!("Failed to spawn cargo command: {}", e);
                    (JobStatus::Failed, err_msg.clone(), Some(err_msg))
                }
            };

            // Update job results
            {
                let mut guard = jobs_clone.lock().unwrap();
                if let Some(job) = guard.iter_mut().find(|j| j.id == id) {
                    job.status = status;
                    job.log = log;
                    job.duration_ms = duration;
                    job.error_message = error_msg;
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all spawned jobs to finish
    for handle in handles {
        let _ = handle.await;
    }
}
