//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use crate::core::ForgeError;
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    use tempfile::TempDir;

    use crate::governance::contract::{
        LOCAL_PROVIDER_ID, MAX_GIT_REVISION_BYTES, WORKSPACE_GOVERNANCE_ADAPTER_RELPATH,
        WORKSPACE_GOVERNANCE_PROVIDER_ID, WORKSPACE_ROOT_ENV,
    };
    use crate::governance::engine::{
        git_revision, parse_status, resolve_known_adapter, run_bounded, write_adapter_request,
    };
    use crate::governance::model::{GovernanceStatus, ProviderStatus};

    /// End-of-file is not an exit event, and this is the **deterministic** guard
    /// for that. The child closes both pipes itself and then lives on for a
    /// while, so end-of-file provably precedes the exit instead of racing it —
    /// which is why a realistic `printf` child is not enough here: closing its
    /// own pipes and exiting leaves a window a few microseconds wide, and a
    /// waiter that only woke on end-of-file passed the `printf` guard while
    /// still failing real adapters 2–4 times per run.
    ///
    /// A waiter that blocks out its budget once there is nothing left to wake it
    /// reports a timeout for a child that answered in 200 ms.
    #[cfg(unix)]
    #[test]
    fn a_child_that_closed_its_pipes_and_keeps_running_is_not_reported_as_a_timeout() {
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "exec 1>&- 2>&-; sleep 0.2; exit 0"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();

        let started = Instant::now();
        let run = run_bounded(
            &mut child,
            started + Duration::from_secs(5),
            MAX_GIT_REVISION_BYTES,
            MAX_GIT_REVISION_BYTES,
        )
        .unwrap();
        let elapsed = started.elapsed();

        assert!(
            run.status.success(),
            "a child that closed its pipes and then exited 0 must not be reported as killed"
        );
        assert!(!run.timed_out, "200 ms of work is not a 5 s timeout");
        assert!(elapsed < Duration::from_secs(2), "elapsed {elapsed:?}");
    }

    /// A revision lookup that never answers gives up inside its bound instead
    /// of hanging the check forever.
    ///
    /// Before the fix the lookup was `Command::output()`, which blocks until the
    /// child exits with no timeout at all — so this command never returns and
    /// the caller waits indefinitely. The bound here is the same one
    /// `git_revision` receives: the selected provider's existing `timeout_ms`.
    #[cfg(unix)]
    #[test]
    fn a_revision_lookup_that_never_answers_gives_up_within_its_bound() {
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "sleep 30"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();

        let started = Instant::now();
        let run =
            run_bounded(&mut child, started + Duration::from_millis(200), 4096, 4096).unwrap();
        let elapsed = started.elapsed();

        assert!(
            run.timed_out,
            "a 30 s child must not report success: {run:?}"
        );
        // Generous next to the 200 ms bound so a loaded machine cannot turn a
        // passing run into a failure, while still being far below the 30 s the
        // child would otherwise take. Pre-fix this assertion is never reached:
        // the call does not return.
        assert!(
            elapsed < Duration::from_secs(10),
            "the bound did not take effect: {elapsed:?}"
        );
    }

    /// The positive control for the guard above, and the guard for the subtler
    /// half of the same defect: a pipe reaching end-of-file is **not** the child
    /// being reaped. `/bin/sh -c 'printf …'` forks on this host, so both pipes
    /// are done microseconds before the shell itself exits. A waiter that only
    /// woke on end-of-file and then blocked for the rest of its budget would
    /// report a timeout for an adapter that had already answered — measured,
    /// that is exactly what happened.
    #[cfg(unix)]
    #[test]
    fn a_bounded_revision_lookup_returns_the_object_name_it_printed() {
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "printf 'a1b2c3d4\\n'"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();

        let started = Instant::now();
        let run = run_bounded(
            &mut child,
            started + Duration::from_secs(5),
            MAX_GIT_REVISION_BYTES,
            MAX_GIT_REVISION_BYTES,
        )
        .unwrap();
        let elapsed = started.elapsed();

        assert!(!run.timed_out, "a child that answered cannot time out");
        assert!(run.status.success());
        assert!(!run.truncated);
        assert_eq!(
            String::from_utf8_lossy(&run.stdout.bytes).trim(),
            "a1b2c3d4"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "answering took {elapsed:?} of a 5 s budget: the waiter slept past the exit"
        );
    }

    /// Bounding the lookup must not turn it into one that always reports no
    /// revision: a real repository's head is still recorded.
    #[test]
    fn git_revision_still_reports_the_repository_head() {
        let repo = TempDir::new().unwrap();
        let head = git_revision(repo.path(), 10_000);
        // A fresh temp directory is not a repository, so the honest assertion
        // here is the `None` mapping; the recorded head is exercised through
        // `evaluate_project` in `tests/governance_contract.rs`. What this pins
        // is that the bounded call runs and returns rather than panicking or
        // blocking.
        assert_eq!(head, None);

        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        if !root.join(".git").exists() {
            return;
        }
        let revision = git_revision(root, 10_000).expect("this repository's head");
        assert_eq!(revision.len(), 40, "{revision}");
        assert!(
            revision.chars().all(|c| c.is_ascii_hexdigit()),
            "{revision}"
        );
    }

    #[test]
    fn unknown_status_is_rejected() {
        assert_eq!(parse_status("healthy"), None);
    }

    #[test]
    fn normalized_failure_status_is_not_healthy() {
        assert!(!GovernanceStatus::from(ProviderStatus::Unavailable).is_healthy());
    }

    /// An adapter that answers without reading its request is a legitimate
    /// provider, so Forge losing the write race against it must not become an
    /// `unavailable` observation.
    ///
    /// Deterministic by construction: the child is reaped *before* the write,
    /// so the read end of the pipe is provably closed. An end-to-end test
    /// cannot establish that ordering, because `run_adapter` writes
    /// immediately after spawning and never waits first.
    #[cfg(unix)]
    #[test]
    fn a_request_write_to_an_adapter_that_already_exited_is_not_a_failure() {
        let mut child = Command::new("/bin/sh")
            .args(["-c", "exec 0<&-; exit 0"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let status = child.wait().unwrap();
        assert!(status.success());

        // Before the fix this was `Err(cannot write adapter request: Broken
        // pipe (os error 32))`, which `run_external_provider` reports as an
        // `Unavailable` observation while discarding the adapter's answer.
        write_adapter_request(&mut stdin, br#"{"action":"check"}"#).unwrap();
    }

    /// A write failure that is not the adapter closing its input is still a
    /// real inability to talk to the adapter, and keeps its typed refusal.
    #[test]
    fn a_request_write_failure_that_is_not_a_broken_pipe_still_refuses() {
        struct Failing;
        impl Write for Failing {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let refusal = write_adapter_request(&mut Failing, b"{}").unwrap_err();
        assert!(
            matches!(refusal, ForgeError::GovernanceUnavailable { .. }),
            "{refusal:?}"
        );
        assert!(refusal.to_string().contains("cannot write adapter request"));
    }

    /// Build `<root>/workspace-governance/scripts/forge_governance_adapter.py`
    /// with the given executable state.
    fn staged_candidate(root: &Path, executable: bool) -> PathBuf {
        let candidate = root.join(WORKSPACE_GOVERNANCE_ADAPTER_RELPATH);
        fs::create_dir_all(candidate.parent().unwrap()).unwrap();
        fs::write(&candidate, "#!/bin/sh\nexit 0\n").unwrap();
        set_mode(&candidate, if executable { 0o755 } else { 0o644 });
        candidate
    }

    #[cfg(unix)]
    fn set_mode(path: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    #[cfg(not(unix))]
    fn set_mode(_path: &Path, _mode: u32) {}

    fn refusal(provider: &str, root: Option<&Path>, env: Option<&str>) -> String {
        match resolve_known_adapter(provider, root, env) {
            Some(Err(ForgeError::GovernanceInvalid { reason })) => reason,
            other => panic!("expected a governance-invalid refusal, got {other:?}"),
        }
    }

    #[test]
    fn providers_without_a_preset_are_left_to_the_caller() {
        assert!(resolve_known_adapter(LOCAL_PROVIDER_ID, None, None).is_none());
        assert!(resolve_known_adapter("external", Some(Path::new("/root")), None).is_none());
    }

    #[test]
    fn missing_root_refuses_naming_both_explicit_inputs() {
        let reason = refusal(WORKSPACE_GOVERNANCE_PROVIDER_ID, None, None);
        assert!(reason.contains("--workspace-root"), "{reason}");
        assert!(reason.contains(WORKSPACE_ROOT_ENV), "{reason}");
    }

    #[test]
    fn blank_env_root_is_treated_as_absent() {
        let reason = refusal(WORKSPACE_GOVERNANCE_PROVIDER_ID, None, Some("   "));
        assert!(reason.contains("--workspace-root"), "{reason}");
    }

    #[test]
    fn missing_candidate_refuses_naming_exact_path() {
        let workspace = TempDir::new().unwrap();
        let reason = refusal(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            Some(workspace.path()),
            None,
        );
        assert!(
            reason.contains(
                &workspace
                    .path()
                    .join(WORKSPACE_GOVERNANCE_ADAPTER_RELPATH)
                    .display()
                    .to_string()
            ),
            "{reason}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_executable_candidate_refuses_naming_exact_path() {
        let workspace = TempDir::new().unwrap();
        let candidate = staged_candidate(workspace.path(), false);
        let reason = refusal(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            Some(workspace.path()),
            None,
        );
        assert!(reason.contains("not an executable file"), "{reason}");
        assert!(
            reason.contains(&candidate.display().to_string()),
            "{reason}"
        );
    }

    #[test]
    fn directory_candidate_refuses_as_not_regular() {
        let workspace = TempDir::new().unwrap();
        fs::create_dir_all(workspace.path().join(WORKSPACE_GOVERNANCE_ADAPTER_RELPATH)).unwrap();
        let reason = refusal(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            Some(workspace.path()),
            None,
        );
        assert!(reason.contains("not a regular file"), "{reason}");
    }

    #[cfg(unix)]
    #[test]
    fn executable_candidate_resolves_to_a_canonical_absolute_path() {
        let workspace = TempDir::new().unwrap();
        let candidate = staged_candidate(workspace.path(), true);
        let resolved = resolve_known_adapter(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            Some(workspace.path()),
            None,
        )
        .unwrap()
        .unwrap();
        assert!(resolved.is_absolute());
        assert_eq!(resolved, candidate.canonicalize().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn explicit_root_wins_over_env_and_never_falls_back() {
        let chosen = TempDir::new().unwrap();
        let other = TempDir::new().unwrap();
        staged_candidate(chosen.path(), true);
        // The env root has no candidate: the flag still resolves.
        let resolved = resolve_known_adapter(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            Some(chosen.path()),
            Some(other.path().to_str().unwrap()),
        )
        .unwrap()
        .unwrap();
        assert!(resolved.starts_with(chosen.path().canonicalize().unwrap()));
        // The flag root without a candidate refuses even when the env root
        // would hold one — the argument is authoritative, never a merge.
        let reason = refusal(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            Some(other.path()),
            Some(chosen.path().to_str().unwrap()),
        );
        assert!(reason.contains("not an existing regular file"), "{reason}");
    }

    #[cfg(unix)]
    #[test]
    fn env_root_resolves_the_packaged_candidate() {
        let workspace = TempDir::new().unwrap();
        staged_candidate(workspace.path(), true);
        let resolved = resolve_known_adapter(
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            None,
            Some(workspace.path().to_str().unwrap()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            resolved,
            workspace
                .path()
                .join(WORKSPACE_GOVERNANCE_ADAPTER_RELPATH)
                .canonicalize()
                .unwrap()
        );
    }
}
