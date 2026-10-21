//! The one native command runner contract (`one-native-command-runner`).
//!
//! The shared runner owns the native command: the argument array or
//! the shell, the working directory, the environment layering, the
//! redaction, the bounded capture and the wall-clock bound. This
//! contract pins the behaviour the capability promises:
//!
//! - A native command runs under one runner, so a test command and a
//!   build command are observed the same way.
//! - The runner is bounded: a command that overruns the bound is
//!   killed with its whole process group and reported as a timeout,
//!   never as a non-zero exit.
//! - The kill takes the tree: a command that spawned a descendant
//!   leaves no descendant behind.
//! - The `forge test` operation runs through the shared runner, keeps
//!   its shell semantics and reports redacted output.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use forge::core::ForgeError;
use forge::gitops::{run_test, run_test_with_timeout};
use forge::native::{run_native_shell, run_native_shell_with_timeout};

/// A short bound, so a test does not wait the product's thirty
/// minutes. The product path binds
/// `forge::native::NATIVE_COMMAND_TIMEOUT`; only the test passes its
/// own bound.
const SHORT_BOUND: Duration = Duration::from_millis(300);

/// The reason a `test-failed` refusal carries.
fn test_failed_reason(err: &ForgeError) -> &str {
    match err {
        ForgeError::TestFailed { reason } => reason.as_str(),
        other => panic!("expected a test failure, got: {other}"),
    }
}

/// Whether `pid` is gone. A killed descendant can linger as a zombie
/// until the host's init reaps it; a zombie is gone for the purpose of
/// this assertion.
fn process_is_gone(pid: i32) -> bool {
    let output = Command::new("ps")
        .args(["-o", "state=", "-p", &pid.to_string()])
        .output();
    match output {
        Ok(out) => {
            let state = String::from_utf8_lossy(&out.stdout).trim().to_string();
            state.is_empty() || state.starts_with('Z')
        }
        Err(_) => true,
    }
}

#[test]
fn the_runner_honours_shell_syntax() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let outcome = run_native_shell(tmp.path(), "echo one && echo two", &[]).expect("run");
    assert!(outcome.success, "outcome: {outcome:?}");
    assert!(outcome.stdout.contains("one"), "stdout: {}", outcome.stdout);
    assert!(outcome.stdout.contains("two"), "stdout: {}", outcome.stdout);
    assert!(!outcome.timed_out, "outcome: {outcome:?}");
}

#[test]
fn the_runner_redacts_a_credential_looking_string() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let outcome = run_native_shell(tmp.path(), "echo AKIAIOSFODNN7EXAMPLE", &[]).expect("run");
    let evidence = outcome.evidence().join(" | ");
    assert!(
        !evidence.contains("AKIAIOSFODNN7EXAMPLE"),
        "evidence: {evidence}"
    );
    assert!(evidence.contains("[REDACTED]"), "evidence: {evidence}");
}

#[test]
fn a_command_that_overruns_the_bound_is_reported_as_a_timeout() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let outcome =
        run_native_shell_with_timeout(tmp.path(), "sleep 30", &[], SHORT_BOUND).expect("run");
    assert!(outcome.timed_out, "outcome: {outcome:?}");
    assert!(!outcome.success, "outcome: {outcome:?}");
    assert_eq!(outcome.exit_code, None, "outcome: {outcome:?}");
    assert!(outcome.start_error.is_none(), "outcome: {outcome:?}");
}

#[test]
fn a_command_that_overruns_the_bound_leaves_no_descendant_behind() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let marker = tmp.path().join("descendant-ran");
    let pid_file = tmp.path().join("descendant.pid");
    let command = format!(
        "(sleep 1; touch '{}') & echo $! > '{}'; wait",
        marker.display(),
        pid_file.display()
    );
    let outcome =
        run_native_shell_with_timeout(tmp.path(), &command, &[], SHORT_BOUND).expect("run");
    assert!(outcome.timed_out, "outcome: {outcome:?}");
    let pid = fs::read_to_string(&pid_file)
        .expect("the command recorded its descendant")
        .trim()
        .to_string();
    assert!(!pid.is_empty(), "the command recorded no descendant pid");
    // The descendant would have written the marker a second later. The
    // group kill took it with the command, so the marker is never
    // written.
    thread::sleep(Duration::from_millis(1500));
    assert!(
        !marker.exists(),
        "the descendant survived the group kill: {}",
        marker.display()
    );
    let pid: i32 = pid.parse().expect("pid");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline && !process_is_gone(pid) {
        thread::sleep(Duration::from_millis(50));
    }
    assert!(
        process_is_gone(pid),
        "the descendant {pid} is still running"
    );
}

#[test]
fn a_command_that_leaves_a_descendant_holding_the_capture_does_not_hang() {
    // The command itself is done, but the descendant it left behind
    // still holds the capture pipes open. The runner takes the
    // descendant with the group instead of waiting for it: a runner
    // that waits for a descendant is a runner that hangs.
    let tmp = tempfile::tempdir().expect("temp dir");
    let command = "sleep 30 & echo done";
    let start = std::time::Instant::now();
    let outcome = run_native_shell_with_timeout(tmp.path(), command, &[], Duration::from_secs(60))
        .expect("run");
    let elapsed = start.elapsed();
    assert!(outcome.success, "outcome: {outcome:?}");
    assert!(!outcome.timed_out, "outcome: {outcome:?}");
    assert!(
        elapsed < Duration::from_secs(25),
        "the runner waited for the descendant: elapsed = {elapsed:?}"
    );
}

#[test]
fn the_test_operation_reports_an_ordinary_failure() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let err = run_test("failing-app", "rust-web", Some("exit 3"), tmp.path())
        .expect_err("the test command failed");
    let reason = test_failed_reason(&err);
    assert!(reason.contains("exit 3"), "reason: {reason}");
    assert!(reason.contains("failing-app"), "reason: {reason}");
    assert!(reason.contains("status 3"), "reason: {reason}");
    assert!(!reason.contains("timeout"), "reason: {reason}");
}

#[test]
fn the_test_operation_keeps_its_shell_semantics() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let outcome = run_test(
        "shell-app",
        "rust-web",
        Some("echo one && echo two"),
        tmp.path(),
    )
    .expect("the test command passed");
    assert_eq!(outcome.status, "passed");
    assert_eq!(outcome.exit_code, Some(0));
    let evidence = outcome.evidence.join(" | ");
    assert!(evidence.contains("one"), "evidence: {evidence}");
    assert!(evidence.contains("two"), "evidence: {evidence}");
}

#[test]
fn the_test_operation_redacts_credential_looking_output() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let outcome = run_test(
        "secret-app",
        "rust-web",
        Some("echo AKIAIOSFODNN7EXAMPLE"),
        tmp.path(),
    )
    .expect("the test command passed");
    let evidence = outcome.evidence.join(" | ");
    assert!(
        !evidence.contains("AKIAIOSFODNN7EXAMPLE"),
        "evidence: {evidence}"
    );
    assert!(evidence.contains("[REDACTED]"), "evidence: {evidence}");
}

#[test]
fn the_test_operation_reports_a_command_that_overruns_the_bound_as_a_timeout() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let err = run_test_with_timeout(
        "hanging-app",
        "rust-web",
        Some("sleep 30"),
        tmp.path(),
        SHORT_BOUND,
    )
    .expect_err("the test command overran the bound");
    let reason = test_failed_reason(&err);
    assert!(reason.contains("sleep 30"), "reason: {reason}");
    assert!(reason.contains("rust-web"), "reason: {reason}");
    assert!(reason.contains("300ms"), "reason: {reason}");
    assert!(reason.contains("process group"), "reason: {reason}");
    assert!(!reason.contains("exited with"), "reason: {reason}");
}

#[test]
fn the_test_operation_reports_a_missing_program_as_an_ordinary_failure() {
    // The shell form always starts: a program the shell cannot find
    // is an ordinary failure with the shell's status, not a start
    // error and not a timeout.
    let tmp = tempfile::tempdir().expect("temp dir");
    let err = run_test(
        "missing-toolchain-app",
        "rust-web",
        Some("forge-missing-toolchain"),
        tmp.path(),
    )
    .expect_err("the test command failed");
    let reason = test_failed_reason(&err);
    assert!(reason.contains("status 127"), "reason: {reason}");
    assert!(!reason.contains("could not start"), "reason: {reason}");
    assert!(!reason.contains("timeout"), "reason: {reason}");
}

#[test]
fn the_test_operation_reports_a_command_that_cannot_start() {
    // A command that cannot start at all is refused as a start
    // failure, naming the command and the profile. The working
    // directory does not exist, so the spawn itself fails.
    let tmp = tempfile::tempdir().expect("temp dir");
    let missing = tmp.path().join("no-such-directory");
    let err = run_test("unstartable-app", "rust-web", Some("echo hi"), &missing)
        .expect_err("the test command could not start");
    let reason = test_failed_reason(&err);
    assert!(reason.contains("could not start"), "reason: {reason}");
    assert!(reason.contains("rust-web"), "reason: {reason}");
    assert!(!reason.contains("timeout"), "reason: {reason}");
}

#[test]
fn the_test_operation_runs_through_the_shared_runner_from_the_cli() {
    // The CLI resolves the profile's declared test command and hands
    // it to the same operation, so the refusal the CLI renders is the
    // shared runner's refusal.
    let tmp = tempfile::tempdir().expect("temp dir");
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).expect("project dir");
    fs::write(
        project.join("forge.yaml"),
        "schema: 1\nproject:\n  id: cli-app\n  name: Cli\n  profile: rust-web\n  \
         maturity: L1\nruntime:\n  language: rust\n",
    )
    .expect("manifest");
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("--registry")
        .arg(&db)
        .arg("test")
        .arg(&project)
        .output()
        .expect("run forge");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert_eq!(output.status.code(), Some(1), "stderr={stderr}");
    assert!(stderr.contains("error[test-failed]"), "stderr={stderr}");
    assert!(stderr.contains("cargo test"), "stderr={stderr}");
    assert!(!stderr.contains("timeout"), "stderr={stderr}");
}

/// The runner's own bound is the product's bound: thirty minutes, and
/// not a number a project can raise.
#[test]
fn the_runner_bound_is_the_house_value() {
    assert_eq!(
        forge::native::NATIVE_COMMAND_TIMEOUT,
        Duration::from_secs(1800)
    );
}

/// The runner keeps the working directory it was handed.
#[test]
fn the_runner_uses_the_callers_working_directory() {
    let tmp = tempfile::tempdir().expect("temp dir");
    fs::write(tmp.path().join("marker.txt"), "here").expect("marker");
    let outcome = run_native_shell(tmp.path(), "ls marker.txt", &[]).expect("run");
    assert!(outcome.success, "outcome: {outcome:?}");
    assert!(
        outcome.stdout.contains("marker.txt"),
        "stdout: {}",
        outcome.stdout
    );
}

/// A path outside the project is not what the runner reports.
#[test]
fn the_runner_reads_the_command_from_the_working_directory() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let script = tmp.path().join("where.sh");
    fs::write(&script, "#!/bin/sh\npwd\n").expect("script");
    let outcome = run_native_shell(tmp.path(), "sh ./where.sh", &[]).expect("run");
    assert!(outcome.success, "outcome: {outcome:?}");
    let reported = Path::new(outcome.stdout.trim());
    assert!(
        reported.exists(),
        "the command did not run in the caller's directory: {}",
        outcome.stdout
    );
}
