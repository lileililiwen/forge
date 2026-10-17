use crate::model::{JobStatus, MatrixJob, MatrixReport};
use chrono::Local;

pub fn generate_demo_report() -> MatrixReport {
    let toolchains = vec![
        Some("stable".to_string()),
        Some("beta".to_string()),
        Some("nightly".to_string()),
        Some("1.75.0".to_string()),
    ];

    let feature_configs = vec![
        (false, vec![]), // default
        (true, vec![]),  // none
        (true, vec!["serde".to_string()]),
        (true, vec!["tokio".to_string()]),
        (true, vec!["serde".to_string(), "tokio".to_string()]),
        (true, vec!["postgres".to_string()]),
        (true, vec!["serde".to_string(), "postgres".to_string()]),
    ];

    let mut jobs = Vec::new();
    let mut id = 0;

    for tc in toolchains {
        for (no_default, feats) in &feature_configs {
            let mut job = MatrixJob::new(id, tc.clone(), feats.clone(), *no_default);

            // Randomly determine outcomes for the demo
            let tc_str = tc.as_deref().unwrap_or("default");

            // Simulate failures for specific configurations to demonstrate failure representation
            if tc_str == "nightly" && feats.contains(&"postgres".to_string()) {
                job.status = JobStatus::Failed;
                job.duration_ms = 450;
                job.error_message =
                    Some("error[E0432]: unresolved import `postgres::NoTls`".to_string());
                job.log = r#"   Compiling postgres v0.19.7
   Compiling mock-crate v0.1.0 (/home/paul/code/mock-crate)
error[E0432]: unresolved import `postgres::NoTls`
  --> src/db.rs:12:9
   |
12 |     use postgres::NoTls;
   |         ^^^^^^^^^^^^^^^ no `NoTls` in the root of `postgres`

For more information about this error, try `rustc --explain E0432`.
error: could not compile `mock-crate` (bin "mock-crate") due to 1 previous error
"#
                .to_string();
            } else if tc_str == "1.75.0" && feats.is_empty() && *no_default {
                job.status = JobStatus::Failed;
                job.duration_ms = 320;
                job.error_message = Some("error[E0277]: the trait bound `tokio::runtime::Runtime: std::marker::Send` is not satisfied".to_string());
                job.log = r#"   Compiling mock-crate v0.1.0 (/home/paul/code/mock-crate)
error[E0277]: the trait bound `tokio::runtime::Runtime: std::marker::Send` is not satisfied
  --> src/main.rs:45:5
   |
45 |     tokio::spawn(async move {
   |     ^^^^^^^^^^^^ spawn requires Sync + Send on default configuration without async feature
   |

error: could not compile `mock-crate` (bin "mock-crate") due to 1 previous error
"#
                .to_string();
            } else {
                job.status = JobStatus::Success;
                job.duration_ms = 120 + (id as u64 * 35) % 150;
                job.log = format!(
                    "   Compiling serde v1.0.204\n   Compiling tokio v1.38.0\n   Compiling mock-crate v0.1.0 (/home/paul/code/mock-crate)\n    Finished dev [unoptimized + debuginfo] target(s) in {:.2}s\n     Running unittests src/main.rs\n\ntest tests::test_basic ... ok\ntest tests::test_advanced ... ok\n\ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s",
                    job.duration_ms as f64 / 1000.0
                );
            }
            jobs.push(job);
            id += 1;
        }
    }

    let total_jobs = jobs.len();
    let successful_jobs = jobs
        .iter()
        .filter(|j| j.status == JobStatus::Success)
        .count();
    let failed_jobs = jobs
        .iter()
        .filter(|j| j.status == JobStatus::Failed)
        .count();
    let elapsed_ms = jobs.iter().map(|j| j.duration_ms).sum::<u64>() / 4; // Simulated parallel overlap

    MatrixReport {
        project_name: "mock-crate".to_string(),
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        elapsed_ms,
        total_jobs,
        successful_jobs,
        failed_jobs,
        jobs,
    }
}
