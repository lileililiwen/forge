//! The shared native-command runner.

#[cfg(test)]
pub(super) mod tests {
    use crate::native::{
        run_native, run_native_with_input, run_native_with_timeout, split_command,
    };
    use std::fs;
    use std::thread;
    use std::time::{Duration, Instant};
    use tempfile::TempDir;

    /// Write a shell script into `dir` and return the command that runs
    /// it. The script is read by `sh` instead of being executed
    /// directly, so the test never execs a file it just wrote.
    fn script(dir: &std::path::Path, name: &str, body: &str) -> String {
        fs::write(dir.join(name), body).expect("write script");
        format!("sh ./{name}")
    }

    /// Whether `pid` is gone. A killed descendant can linger as a
    /// zombie until the host's init reaps it; a zombie is gone for
    /// the purpose of this assertion.
    fn process_is_gone(pid: i32) -> bool {
        let output = std::process::Command::new("ps")
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
    fn a_command_is_split_into_a_program_and_its_arguments() {
        let (program, args) = split_command("cargo build --release").expect("split");
        assert_eq!(program, "cargo");
        assert_eq!(args, vec!["build".to_string(), "--release".to_string()]);
    }

    #[test]
    fn an_empty_command_is_refused() {
        let err = split_command("   ").expect_err("empty command");
        assert_eq!(err, "empty native command; nothing was verified");
    }

    #[test]
    fn the_runner_captures_stdout_and_the_exit_code() {
        let tmp = TempDir::new().expect("temp dir");
        let command = script(tmp.path(), "ok.sh", "#!/bin/sh\necho hello\n");
        let (program, args) = split_command(&command).expect("split");
        let outcome = run_native(tmp.path(), &program, &args, &[]).expect("run");
        assert!(outcome.success, "outcome: {outcome:?}");
        assert_eq!(outcome.exit_code, Some(0));
        assert!(
            outcome.stdout.contains("hello"),
            "stdout: {}",
            outcome.stdout
        );
        assert!(outcome.start_error.is_none());
        assert!(!outcome.timed_out);
    }

    #[test]
    fn the_runner_reports_a_failing_exit_code() {
        let tmp = TempDir::new().expect("temp dir");
        let command = script(tmp.path(), "fail.sh", "#!/bin/sh\necho boom >&2\nexit 3\n");
        let (program, args) = split_command(&command).expect("split");
        let outcome = run_native(tmp.path(), &program, &args, &[]).expect("run");
        assert!(!outcome.success);
        assert_eq!(outcome.exit_code, Some(3));
        assert!(!outcome.timed_out);
        assert!(
            outcome.tail(100).contains("boom"),
            "tail: {}",
            outcome.tail(100)
        );
    }

    #[test]
    fn a_command_that_cannot_start_is_reported_in_the_outcome() {
        let tmp = TempDir::new().expect("temp dir");
        let outcome = run_native(tmp.path(), "forge-missing-toolchain", &[], &[]).expect("run");
        assert!(!outcome.success);
        assert_eq!(outcome.exit_code, None);
        assert!(outcome.start_error.is_some(), "outcome: {outcome:?}");
        // A command that never started did not overrun the bound.
        assert!(!outcome.timed_out, "outcome: {outcome:?}");
    }

    #[test]
    fn the_environment_overrides_reach_the_command() {
        let tmp = TempDir::new().expect("temp dir");
        let command = script(
            tmp.path(),
            "env.sh",
            "#!/bin/sh\necho \"target=$CARGO_BUILD_TARGET\"\n",
        );
        let (program, args) = split_command(&command).expect("split");
        let env = vec![(
            "CARGO_BUILD_TARGET".to_string(),
            "aarch64-unknown-linux-gnu".to_string(),
        )];
        let outcome = run_native(tmp.path(), &program, &args, &env).expect("run");
        assert!(
            outcome.stdout.contains("target=aarch64-unknown-linux-gnu"),
            "outcome: {outcome:?}"
        );
    }

    #[test]
    fn the_captured_output_is_redacted_before_it_reaches_the_evidence() {
        let tmp = TempDir::new().expect("temp dir");
        let command = script(
            tmp.path(),
            "secret.sh",
            "#!/bin/sh\necho \"AKIAIOSFODNN7EXAMPLE\"\n",
        );
        let (program, args) = split_command(&command).expect("split");
        let outcome = run_native(tmp.path(), &program, &args, &[]).expect("run");
        let evidence = outcome.evidence().join(" | ");
        assert!(
            !evidence.contains("AKIAIOSFODNN7EXAMPLE"),
            "evidence: {evidence}"
        );
        assert!(evidence.contains("[REDACTED]"), "evidence: {evidence}");
    }

    /// A command that overruns the bound is killed and reported as a
    /// timeout: no exit code, no success, and a start error is not
    /// claimed.
    #[test]
    fn a_command_that_overruns_the_bound_is_killed_and_reported_as_a_timeout() {
        let tmp = TempDir::new().expect("temp dir");
        let command = script(tmp.path(), "hang.sh", "#!/bin/sh\nsleep 30\n");
        let (program, args) = split_command(&command).expect("split");
        let start = Instant::now();
        let outcome =
            run_native_with_timeout(tmp.path(), &program, &args, &[], Duration::from_millis(300))
                .expect("run");
        let elapsed = start.elapsed();
        assert!(outcome.timed_out, "outcome: {outcome:?}");
        assert!(!outcome.success, "outcome: {outcome:?}");
        assert_eq!(outcome.exit_code, None, "outcome: {outcome:?}");
        assert!(outcome.start_error.is_none(), "outcome: {outcome:?}");
        assert!(
            elapsed < Duration::from_secs(20),
            "the runner waited for the command: elapsed = {elapsed:?}"
        );
    }

    /// The kill takes the tree with it: a command that spawned a
    /// descendant leaves no descendant behind.
    #[test]
    fn a_command_that_overruns_the_bound_leaves_no_descendant_behind() {
        let tmp = TempDir::new().expect("temp dir");
        let marker = tmp.path().join("descendant-ran");
        let pid_file = tmp.path().join("descendant.pid");
        let body = format!(
            "#!/bin/sh\n(sleep 1; touch '{}') &\necho $! > '{}'\nwait\n",
            marker.display(),
            pid_file.display()
        );
        let command = script(tmp.path(), "descendant.sh", &body);
        let (program, args) = split_command(&command).expect("split");
        let outcome =
            run_native_with_timeout(tmp.path(), &program, &args, &[], Duration::from_millis(300))
                .expect("run");
        assert!(outcome.timed_out, "outcome: {outcome:?}");
        let pid = fs::read_to_string(&pid_file)
            .expect("the command recorded its descendant")
            .trim()
            .to_string();
        assert!(!pid.is_empty(), "the command recorded no descendant pid");
        // The descendant would have written the marker a second later.
        // The group kill took it with the command, so the marker is
        // never written.
        thread::sleep(Duration::from_millis(1500));
        assert!(
            !marker.exists(),
            "the descendant survived the group kill: {}",
            marker.display()
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && !process_is_gone(pid.parse().expect("pid")) {
            thread::sleep(Duration::from_millis(50));
        }
        assert!(
            process_is_gone(pid.parse().expect("pid")),
            "the descendant {pid} is still running"
        );
    }

    #[test]
    fn the_runner_delivers_one_bounded_document_on_stdin() {
        let dir = TempDir::new().expect("tempdir");
        let input = b"{\"contract\":\"example/0.1.0\"}";
        let outcome = run_native_with_input(
            dir.path(),
            "sh",
            &["-c".to_string(), "cat".to_string()],
            &[],
            input,
        )
        .expect("run");
        assert!(outcome.success, "{:?}", outcome.start_error);
        assert_eq!(outcome.stdout.as_bytes(), input);
    }

    #[test]
    fn a_command_that_ignores_stdin_still_reports_its_status() {
        let dir = TempDir::new().expect("tempdir");
        let outcome = run_native_with_input(
            dir.path(),
            "sh",
            &["-c".to_string(), "exit 0".to_string()],
            &[],
            b"unread",
        )
        .expect("run");
        assert!(outcome.success, "{:?}", outcome.start_error);
    }
}
