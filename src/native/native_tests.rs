//! The shared native-command runner.

#[cfg(test)]
pub(super) mod tests {
    use crate::native::{run_native, split_command};
    use std::fs;
    use tempfile::TempDir;

    /// Write a shell script into `dir` and return the command that runs
    /// it. The script is read by `sh` instead of being executed
    /// directly, so the test never execs a file it just wrote.
    fn script(dir: &std::path::Path, name: &str, body: &str) -> String {
        fs::write(dir.join(name), body).expect("write script");
        format!("sh ./{name}")
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
    }

    #[test]
    fn the_runner_reports_a_failing_exit_code() {
        let tmp = TempDir::new().expect("temp dir");
        let command = script(tmp.path(), "fail.sh", "#!/bin/sh\necho boom >&2\nexit 3\n");
        let (program, args) = split_command(&command).expect("split");
        let outcome = run_native(tmp.path(), &program, &args, &[]).expect("run");
        assert!(!outcome.success);
        assert_eq!(outcome.exit_code, Some(3));
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
}
