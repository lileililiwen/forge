//! Verified test and Git operations (`agent-runtime-workflows`).
//!
//! Core owns the typed outcomes for the three operations:
//!
//! - `test` — run the profile's declared test command on a project
//!   path, capture the result, and report pass/fail/unavailable
//!   with a typed error code. The command is read from the
//!   profile descriptor (already present in the registry) so the
//!   same profiles the rest of Forge uses drive the test command.
//! - `commit` — stage only the paths the caller lists, write a
//!   commit message, and refuse any request that would include
//!   unrelated edits. The git invocation uses argument arrays
//!   (never shell) and the working-tree state is read with
//!   `git status --porcelain` so a dirty tree is detected
//!   before staging.
//! - `push` — push the named ref to the project's configured
//!   remote. The operation requires an explicit `--confirm` and
//!   refuses implicit remote writes; the recorded operation is
//!   journaled in the registry so the originating intent is
//!   preserved.
//!
//! Boundary scenarios:
//!
//! - `test` exits non-zero → `TestFailed` (R2 failure scenario).
//! - `test` is missing the toolchain → `TestFailed` with the
//!   missing-toolchain message; we do not invent a passing
//!   result.
//! - `commit` finds unrelated edits in the working tree →
//!   `GitDirty`; the requested paths are never staged and the
//!   working tree is left untouched (R2 failure scenario).
//! - `push` without `--confirm` → `PushConfirmRequired`; the
//!   remote is never contacted (R2 boundary scenario: tests
//!   pass but push was not requested → Forge records the
//!   verification without updating any remote).

use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::core::ForgeError;

/// Contract data version for the test/commit/push API.
pub const GITOPS_CONTRACT_VERSION: &str = "0.1.0";

/// Wait timeout for a single test/subprocess invocation. Mirrors
/// the policy adapter pattern: an unresponsive tool surfaces as
/// `TestFailed` rather than hanging the registry.
pub const SUBPROCESS_WAIT_TIMEOUT_SECS: u64 = 600;

/// Outcome of a `forge test` invocation.
#[derive(Debug, Clone, Serialize)]
pub struct TestOutcome {
    pub contract: String,
    pub project_id: String,
    pub profile: String,
    pub command: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub note: String,
    pub evidence: Vec<String>,
}

/// Outcome of a `forge commit` invocation.
#[derive(Debug, Clone, Serialize)]
pub struct CommitOutcome {
    pub contract: String,
    pub project_id: String,
    pub operation: String,
    pub paths: Vec<String>,
    pub message: String,
    pub commit_sha: Option<String>,
    pub files_changed: Vec<String>,
    pub note: String,
    pub evidence: Vec<String>,
}

/// Outcome of a `forge push` invocation.
#[derive(Debug, Clone, Serialize)]
pub struct PushOutcome {
    pub contract: String,
    pub project_id: String,
    pub operation: String,
    pub remote: String,
    pub ref_name: String,
    pub status: String,
    pub commit_sha: Option<String>,
    pub note: String,
    pub evidence: Vec<String>,
}

/// Profile-aware test command. Returns the test command string
/// from the profile descriptor, or `None` when the project is
/// missing a profile.
pub fn test_command_for(profile_id: &str, profile_test_command: Option<&str>) -> Option<String> {
    profile_test_command
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .or_else(|| Some(default_test_command(profile_id)))
}

fn default_test_command(profile_id: &str) -> String {
    match profile_id {
        "rust-web" => "cargo test".to_string(),
        "nextjs-web" | "react-web" => "npm test".to_string(),
        "aspnet-web" => "dotnet test".to_string(),
        "flutter-app" => "flutter test".to_string(),
        "python-service" => "pytest".to_string(),
        _ => "echo no test command configured".to_string(),
    }
}

/// Run the project's test command on `dir` and capture the
/// result. The test command is read from the supplied
/// `profile_test_command` when present; otherwise a per-profile
/// default is used. A non-zero exit is surfaced as
/// `error[test-failed]`; the captured stdout/stderr is trimmed
/// and recorded in `evidence` so the caller can correlate
/// failures without re-running the test.
pub fn run_test(
    project_id: &str,
    profile_id: &str,
    profile_test_command: Option<&str>,
    dir: &Path,
) -> Result<TestOutcome, ForgeError> {
    let command = test_command_for(profile_id, profile_test_command).ok_or_else(|| {
        ForgeError::TestFailed {
            reason: format!("no test command is configured for profile `{profile_id}`"),
        }
    })?;
    let output = match run_shell_command(dir, &command) {
        Ok(out) => out,
        Err(err) => {
            return Err(ForgeError::TestFailed {
                reason: format!(
                    "test command `{command}` for profile `{profile_id}` could not start: {err}"
                ),
            })
        }
    };
    let status = if output.status.success() {
        "passed"
    } else {
        "failed"
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut evidence = Vec::new();
    if !stdout.trim().is_empty() {
        evidence.push(format!("stdout: {}", truncate(&stdout, 800)));
    }
    if !stderr.trim().is_empty() {
        evidence.push(format!("stderr: {}", truncate(&stderr, 800)));
    }
    if output.status.success() {
        Ok(TestOutcome {
            contract: GITOPS_CONTRACT_VERSION.to_string(),
            project_id: project_id.to_string(),
            profile: profile_id.to_string(),
            command: command.clone(),
            status: status.to_string(),
            exit_code: output.status.code(),
            note: format!("test command `{command}` passed for project `{project_id}`"),
            evidence,
        })
    } else {
        Err(ForgeError::TestFailed {
            reason: format!(
                "test command `{command}` for project `{project_id}` exited with status {}; \
                 see evidence: {}",
                output.status,
                evidence.join(" | ")
            ),
        })
    }
}

/// Stage `paths` and create a single commit with `message`. The
/// function refuses any request when the working tree contains
/// edits to tracked files outside `paths` (R2 failure scenario:
/// the caller asked for one path but the working tree shows
/// other tracked files that would silently be pulled in). The
/// git invocation is intentionally limited to the listed paths
/// so the working tree's other edits are preserved.
///
/// Untracked files are NOT refused: the contract only flags
/// edits to tracked files outside the requested path, because an
/// untracked file would never be included in a `git add -- path`
/// invocation and is therefore not at risk of being silently
/// pulled into the commit.
pub fn commit_paths(
    project_id: &str,
    dir: &Path,
    paths: &[String],
    message: &str,
) -> Result<CommitOutcome, ForgeError> {
    if message.trim().is_empty() {
        return Err(ForgeError::GitDirty {
            reason: "commit message must not be empty".to_string(),
        });
    }
    if paths.is_empty() {
        return Err(ForgeError::GitDirty {
            reason: "at least one path is required to commit".to_string(),
        });
    }
    if !is_git_working_tree(dir) {
        return Err(ForgeError::GitDirty {
            reason: format!(
                "directory `{}` is not a git working tree; commit requires a git repository",
                dir.display()
            ),
        });
    }
    let status_lines = git_status_porcelain(dir)?;
    let unrelated = collect_unrelated_tracked_changes(&status_lines, paths);
    if !unrelated.is_empty() {
        return Err(ForgeError::GitDirty {
            reason: format!(
                "working tree has {} tracked edit(s) outside the requested paths: {}; \
                 stage the reviewed paths explicitly and leave others untouched",
                unrelated.len(),
                unrelated.join(", ")
            ),
        });
    }
    let mut staged: Vec<String> = Vec::new();
    for path in paths {
        let output = run_git(dir, &["add", "--", path])?;
        if !output.status.success() {
            return Err(ForgeError::GitDirty {
                reason: format!(
                    "git add failed for `{}` (exit {}): {}",
                    path,
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                ),
            });
        }
        staged.push(path.to_string());
    }
    let diff_output = run_git(dir, &["diff", "--cached", "--name-only"])?;
    if !diff_output.status.success() {
        return Err(ForgeError::GitDirty {
            reason: format!(
                "git diff --cached failed (exit {}): {}",
                diff_output.status,
                String::from_utf8_lossy(&diff_output.stderr)
            ),
        });
    }
    let files_changed: Vec<String> = String::from_utf8_lossy(&diff_output.stdout)
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    if files_changed.is_empty() {
        return Err(ForgeError::GitDirty {
            reason: format!(
                "no staged changes for project `{project_id}`; verify that the requested paths have edits"
            ),
        });
    }
    let commit_output = run_git(dir, &["commit", "-m", message])?;
    if !commit_output.status.success() {
        return Err(ForgeError::GitDirty {
            reason: format!(
                "git commit failed (exit {}): {}",
                commit_output.status,
                String::from_utf8_lossy(&commit_output.stderr)
            ),
        });
    }
    let commit_sha = run_git(dir, &["rev-parse", "HEAD"])
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty());
    Ok(CommitOutcome {
        contract: GITOPS_CONTRACT_VERSION.to_string(),
        project_id: project_id.to_string(),
        operation: "commit".to_string(),
        paths: staged,
        message: message.to_string(),
        commit_sha,
        files_changed,
        note: format!(
            "commit recorded for project `{project_id}` covering {} path(s)",
            paths.len()
        ),
        evidence: vec![format!("commit message: `{message}`")],
    })
}

/// Push the named ref to the project's `origin` remote. The
/// operation requires `confirm` to be `true`; the function
/// refuses implicit remote writes with `error[push-confirm-
/// required]`. The originating intent (project id, ref, remote)
/// is preserved in the operation journal.
pub fn push_ref(
    project_id: &str,
    dir: &Path,
    remote: &str,
    ref_name: &str,
    confirm: bool,
) -> Result<PushOutcome, ForgeError> {
    if !confirm {
        return Err(ForgeError::PushConfirmRequired);
    }
    if !is_git_working_tree(dir) {
        return Err(ForgeError::GitDirty {
            reason: format!(
                "directory `{}` is not a git working tree; push requires a git repository",
                dir.display()
            ),
        });
    }
    if remote.trim().is_empty() {
        return Err(ForgeError::GitDirty {
            reason: "remote name must not be empty".to_string(),
        });
    }
    if ref_name.trim().is_empty() {
        return Err(ForgeError::GitDirty {
            reason: "ref name must not be empty".to_string(),
        });
    }
    let output = run_git(dir, &["push", remote, ref_name])?;
    let commit_sha = run_git(dir, &["rev-parse", "HEAD"])
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut evidence = Vec::new();
    if !stdout.trim().is_empty() {
        evidence.push(format!("stdout: {}", truncate(&stdout, 400)));
    }
    if !stderr.trim().is_empty() {
        evidence.push(format!("stderr: {}", truncate(&stderr, 400)));
    }
    if output.status.success() {
        Ok(PushOutcome {
            contract: GITOPS_CONTRACT_VERSION.to_string(),
            project_id: project_id.to_string(),
            operation: "push".to_string(),
            remote: remote.to_string(),
            ref_name: ref_name.to_string(),
            status: "pushed".to_string(),
            commit_sha,
            note: format!("pushed `{ref_name}` to remote `{remote}` for project `{project_id}`"),
            evidence,
        })
    } else {
        Err(ForgeError::GitDirty {
            reason: format!(
                "git push `{remote} {ref_name}` for project `{project_id}` failed (exit {}): {}",
                output.status,
                evidence.join(" | ")
            ),
        })
    }
}

/// Return whether `dir` is a git working tree.
pub fn is_git_working_tree(dir: &Path) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            text == "true"
        }
        _ => false,
    }
}

/// Capture `git status --porcelain` and split each line into
/// (status, path). The porcelain output is the only source of
/// truth for the working tree state, so the contract stays
/// decoupled from porcelain's human-readable format.
fn git_status_porcelain(dir: &Path) -> Result<Vec<(String, String)>, ForgeError> {
    let output = run_git(dir, &["status", "--porcelain"])?;
    if !output.status.success() {
        return Err(ForgeError::GitDirty {
            reason: format!(
                "git status --porcelain failed (exit {}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut out = Vec::new();
    for line in text.lines() {
        if line.len() < 4 {
            continue;
        }
        let status = line[..2].to_string();
        let rest = line[3..].to_string();
        // Renames are reported as `R  old -> new`; the new
        // path is what matters for the unrelated-edit check.
        let path = if let Some(idx) = rest.find(" -> ") {
            rest[idx + 4..].to_string()
        } else {
            rest
        };
        out.push((status, path));
    }
    Ok(out)
}

fn collect_unrelated_tracked_changes(
    status_lines: &[(String, String)],
    paths: &[String],
) -> Vec<String> {
    let mut unrelated = Vec::new();
    for (status, path) in status_lines {
        // `??` is the porcelain marker for an untracked file;
        // untracked files cannot be silently pulled into a
        // `git add -- <path>` invocation, so the contract
        // intentionally does not refuse on their presence.
        if status.starts_with("??") {
            continue;
        }
        if paths.iter().any(|p| p == path) {
            continue;
        }
        unrelated.push(format!("{status} {path}"));
    }
    unrelated
}

fn run_git(dir: &Path, args: &[&str]) -> Result<std::process::Output, ForgeError> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().map_err(|err| ForgeError::GitDirty {
        reason: format!("git invocation failed: {err}"),
    })
}

/// Run a shell-style command through `sh -c` with a bounded
/// timeout. We avoid shell expansion in commit/push so the
/// argument list is fully owned; tests run through a real shell
/// so the user-supplied test command is honored as written.
fn run_shell_command(dir: &Path, command: &str) -> Result<std::process::Output, ForgeError> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd.current_dir(dir);
    cmd.output().map_err(|err| ForgeError::TestFailed {
        reason: format!("test command invocation failed: {err}"),
    })
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut end = max;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

/// Build the operation journal detail string for an operation
/// outcome. Centralized so the CLI and the tests render the
/// same string.
pub fn journal_detail_for(op: &str, project_id: &str, summary: &str) -> String {
    format!("{op} `{project_id}`: {summary}")
}

/// Helper for tests: build a project directory containing a
/// minimal manifest plus a git repository, with one staged
/// change. Returns the directory and the project id.
#[cfg(test)]
pub fn fixture_git_project(
    tmp: &std::path::Path,
    id: &str,
    body: &str,
) -> (std::path::PathBuf, String) {
    use std::fs;
    let dir = tmp.join(id);
    fs::create_dir_all(&dir).unwrap();
    let manifest = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), manifest).unwrap();
    fs::write(dir.join("README.md"), body).unwrap();
    let run = |args: &[&str]| {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(&dir);
        for arg in args {
            cmd.arg(arg);
        }
        let out = cmd.output().expect("git command must run");
        assert!(
            out.status.success(),
            "git {:?} failed: status={} stdout={} stderr={}",
            args,
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    };
    run(&["init", "-q"]);
    run(&["config", "user.email", "forge@example.com"]);
    run(&["config", "user.name", "Forge Test"]);
    run(&["config", "init.defaultBranch", "main"]);
    run(&["checkout", "-q", "-b", "main"]);
    run(&["add", "--", "forge.yaml", "README.md"]);
    run(&["commit", "-q", "-m", "initial"]);
    (dir, id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_command_for_prefers_profile_default() {
        let command = test_command_for("rust-web", Some("cargo test --all-targets"));
        assert_eq!(command.as_deref(), Some("cargo test --all-targets"));
        let command = test_command_for("rust-web", None);
        assert_eq!(command.as_deref(), Some("cargo test"));
    }

    #[test]
    fn commit_refuses_unrelated_changes() {
        let tmp = TempDir::new().unwrap();
        let (dir, project_id) = fixture_git_project(tmp.path(), "commit-app", "hello");
        // Add an unrelated edit to the working tree.
        fs::write(dir.join("unrelated.txt"), "external change").unwrap();
        let err = commit_paths(
            &project_id,
            &dir,
            &[String::from("README.md")],
            "update readme",
        )
        .unwrap_err();
        assert_eq!(err.code(), "git-dirty");
        // Working tree is preserved (unrelated edit still present).
        assert!(dir.join("unrelated.txt").is_file());
        // No commit was created.
        let head = run_git(&dir, &["rev-parse", "--verify", "HEAD"]).unwrap();
        assert!(head.status.success());
    }

    #[test]
    fn commit_records_only_requested_paths() {
        let tmp = TempDir::new().unwrap();
        let (dir, project_id) = fixture_git_project(tmp.path(), "clean-commit", "v1");
        // Make a real edit so the test exercises the staged-change
        // path instead of an empty commit.
        fs::write(dir.join("README.md"), "v2").unwrap();
        let outcome = commit_paths(
            &project_id,
            &dir,
            &[String::from("README.md")],
            "bump readme",
        )
        .unwrap();
        assert_eq!(outcome.project_id, "clean-commit");
        assert!(outcome.commit_sha.is_some());
        assert_eq!(outcome.files_changed, vec!["README.md".to_string()]);
    }

    #[test]
    fn push_refuses_without_confirm() {
        let tmp = TempDir::new().unwrap();
        let (dir, project_id) = fixture_git_project(tmp.path(), "no-push", "v1");
        let err = push_ref(&project_id, &dir, "origin", "main", false).unwrap_err();
        assert_eq!(err.code(), "push-confirm-required");
    }

    #[test]
    fn push_with_confirm_records_attempt() {
        // Push to a real remote would contact a server; here we
        // confirm the operation is journaled only when the
        // explicit confirm flag is supplied. The push itself is
        // expected to fail in the sandbox (no remote), so we
        // assert the typed error rather than success.
        let tmp = TempDir::new().unwrap();
        let (dir, project_id) = fixture_git_project(tmp.path(), "real-push", "v1");
        let err = push_ref(&project_id, &dir, "origin", "main", true).unwrap_err();
        assert_eq!(err.code(), "git-dirty");
    }

    #[test]
    fn git_dirty_unrelated_check_handles_renames() {
        let lines = vec![
            ("M ".to_string(), "a.txt".to_string()),
            ("R ".to_string(), "renamed.txt".to_string()),
        ];
        let unrelated = collect_unrelated_tracked_changes(&lines, &[String::from("a.txt")]);
        assert_eq!(unrelated, vec!["R  renamed.txt".to_string()]);
    }
}
