//! The target-addressed build operation.

#[cfg(test)]
pub(super) mod tests {
    use crate::build::run_build;
    use crate::core::ForgeError;
    use crate::target::{Architecture, OperatingSystem, TargetRequest};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    /// Write a shell script into `dir` and return the command that runs
    /// it. The script is read by `sh` instead of being executed
    /// directly, so the test never execs a file it just wrote.
    fn script(dir: &Path, name: &str, body: &str) -> String {
        fs::write(dir.join(name), body).expect("write script");
        format!("sh ./{name}")
    }

    #[test]
    fn a_host_build_reports_the_command_the_target_and_passed() {
        let tmp = TempDir::new().expect("temp dir");
        let command = script(tmp.path(), "build.sh", "#!/bin/sh\necho compiled\n");
        let outcome = run_build(
            "host-app",
            "rust-web",
            &command,
            tmp.path(),
            &TargetRequest::Host,
        )
        .expect("build");
        assert_eq!(outcome.contract, "forge-build/0.1.0");
        assert_eq!(outcome.project_id, "host-app");
        assert_eq!(outcome.profile, "rust-web");
        assert_eq!(outcome.command, command);
        assert_eq!(outcome.target, "host");
        assert_eq!(outcome.status, "passed");
        assert_eq!(outcome.exit_code, Some(0));
        assert_eq!(
            outcome.note,
            format!("build command `{command}` passed for project `host-app`")
        );
        assert!(
            outcome.evidence.join(" ").contains("compiled"),
            "evidence: {:?}",
            outcome.evidence
        );
    }

    #[test]
    fn a_target_build_binds_the_toolchain_variable_and_names_the_target() {
        let tmp = TempDir::new().expect("temp dir");
        let command = script(
            tmp.path(),
            "build.sh",
            "#!/bin/sh\necho \"target=$CARGO_BUILD_TARGET\"\n",
        );
        let request = TargetRequest::Platform {
            os: OperatingSystem::Linux,
            arch: Architecture::Aarch64,
        };
        let outcome =
            run_build("target-app", "rust-web", &command, tmp.path(), &request).expect("build");
        assert_eq!(outcome.target, "aarch64-unknown-linux-gnu");
        assert_eq!(outcome.status, "passed");
        assert_eq!(
            outcome.note,
            format!(
                "build command `{command}` passed for project `target-app` at target \
                 `aarch64-unknown-linux-gnu`"
            )
        );
        assert!(
            outcome
                .evidence
                .join(" ")
                .contains("target=aarch64-unknown-linux-gnu"),
            "evidence: {:?}",
            outcome.evidence
        );
    }

    #[test]
    fn a_runtime_identifier_build_binds_the_dotnet_variable() {
        let tmp = TempDir::new().expect("temp dir");
        let command = script(
            tmp.path(),
            "build.sh",
            "#!/bin/sh\necho \"rid=$DOTNET_RUNTIME_ID\"\n",
        );
        let request = TargetRequest::RuntimeIdentifier("linux-arm64".to_string());
        let outcome =
            run_build("dotnet-app", "aspnet-web", &command, tmp.path(), &request).expect("build");
        assert_eq!(outcome.target, "linux-arm64");
        assert!(
            outcome.evidence.join(" ").contains("rid=linux-arm64"),
            "evidence: {:?}",
            outcome.evidence
        );
    }

    #[test]
    fn a_failing_build_fails_with_its_evidence() {
        let tmp = TempDir::new().expect("temp dir");
        let command = script(
            tmp.path(),
            "build.sh",
            "#!/bin/sh\necho broken >&2\nexit 2\n",
        );
        let err = run_build(
            "broken-app",
            "rust-web",
            &command,
            tmp.path(),
            &TargetRequest::Host,
        )
        .expect_err("failing build");
        assert_eq!(err.code(), "build-failed");
        let reason = reason_of(&err);
        assert!(reason.contains("broken"), "reason: {reason}");
        assert!(reason.contains("status 2"), "reason: {reason}");
    }

    #[test]
    fn a_build_command_that_cannot_start_fails_with_the_reason() {
        let tmp = TempDir::new().expect("temp dir");
        let err = run_build(
            "missing-app",
            "rust-web",
            "./forge-missing-build-tool",
            tmp.path(),
            &TargetRequest::Host,
        )
        .expect_err("missing toolchain");
        assert_eq!(err.code(), "build-failed");
        let reason = reason_of(&err);
        assert!(reason.contains("could not start"), "reason: {reason}");
    }

    #[test]
    fn a_profile_without_a_build_command_is_refused_by_name() {
        let tmp = TempDir::new().expect("temp dir");
        let err = run_build(
            "empty-app",
            "rust-web",
            "   ",
            tmp.path(),
            &TargetRequest::Host,
        )
        .expect_err("empty build command");
        assert_eq!(err.code(), "build-invalid");
        let reason = reason_of(&err);
        assert!(reason.contains("rust-web"), "reason: {reason}");
    }

    #[test]
    fn a_toolchain_without_a_target_mechanism_is_refused() {
        let tmp = TempDir::new().expect("temp dir");
        let command = script(tmp.path(), "build.sh", "#!/bin/sh\necho compiled\n");
        let request = TargetRequest::Platform {
            os: OperatingSystem::Linux,
            arch: Architecture::Aarch64,
        };
        let err = run_build(
            "python-app",
            "python-service",
            &command,
            tmp.path(),
            &request,
        )
        .expect_err("no target mechanism");
        assert_eq!(err.code(), "build-invalid");
        let reason = reason_of(&err);
        assert!(reason.contains("python3"), "reason: {reason}");
        assert!(reason.contains("python-service"), "reason: {reason}");
    }

    #[test]
    fn the_evidence_is_redacted() {
        let tmp = TempDir::new().expect("temp dir");
        let command = script(
            tmp.path(),
            "build.sh",
            "#!/bin/sh\necho \"AKIAIOSFODNN7EXAMPLE\"\n",
        );
        let outcome = run_build(
            "secret-app",
            "rust-web",
            &command,
            tmp.path(),
            &TargetRequest::Host,
        )
        .expect("build");
        let evidence = outcome.evidence.join(" | ");
        assert!(
            !evidence.contains("AKIAIOSFODNN7EXAMPLE"),
            "evidence: {evidence}"
        );
        assert!(evidence.contains("[REDACTED]"), "evidence: {evidence}");
    }

    /// The reason of a typed error, without the variant's own prefix.
    fn reason_of(err: &ForgeError) -> String {
        match err {
            ForgeError::BuildInvalid { reason } | ForgeError::BuildFailed { reason } => {
                reason.clone()
            }
            other => panic!("expected a build error, got {other:?}"),
        }
    }
}
