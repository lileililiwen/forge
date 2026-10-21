//! The shared native-command runner.
//!
//! One runner serves every Core operation that runs a native command:
//! the generator's post-generation verification, the readiness
//! fixtures, the build step and the `forge test` operation. The runner
//! owns the working directory, the environment layering, the
//! redaction and the bounded capture, so every native command is
//! observed the same way. A command runs either as an argument array
//! or through the shell; the shell form exists for a profile's
//! declared test command, which may rely on shell semantics such as
//! pipes, redirections and `&&`.
//!
//! Every native command runs under [`NATIVE_COMMAND_TIMEOUT`]. When the
//! bound elapses the command's whole process group is killed and
//! reaped, so a command that spawned descendants leaves nothing behind
//! and a hung command fails instead of hanging the caller.

pub mod native_tests;

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::process::{kill_process_tree, spawn_in_own_group};

/// The maximum number of characters kept from each captured stream.
pub const MAX_CAPTURE_CHARS: usize = 8000;

/// The maximum number of characters kept in one evidence line.
pub const MAX_EVIDENCE_CHARS: usize = 800;

/// The wall-clock bound for one native command.
///
/// Thirty minutes: the house's value for a long external operation
/// (`PUBLISH_DEPLOY_TIMEOUT` and `PUBLISH_PROVIDER_TIMEOUT` are both
/// 1800 seconds). The bound is a deadline on the command, not on
/// Forge's patience with it, and it is not a manifest field, so a
/// project cannot raise it.
pub const NATIVE_COMMAND_TIMEOUT: Duration = Duration::from_secs(1800);

/// The grace period for the capture streams after the command's
/// process group was killed. The bound is on the command, not on the
/// capture: this window only lets a drained pipe close, and it is
/// what a descendant that outlives the command is given before the
/// group takes it.
const CAPTURE_GRACE: Duration = Duration::from_secs(2);

/// The result of one native command.
///
/// The evidence of the run is the redacted, bounded stdout and stderr
/// the outcome carries; [`NativeOutcome::evidence`] renders them as the
/// report's evidence lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOutcome {
    /// The command's exit code, when the process reported one.
    pub exit_code: Option<i32>,
    /// True when the command exited with status zero.
    pub success: bool,
    /// The captured stdout, redacted and bounded.
    pub stdout: String,
    /// The captured stderr, redacted and bounded.
    pub stderr: String,
    /// Why the command did not start, when it did not start.
    pub start_error: Option<String>,
    /// True when the command was killed because the wall-clock bound
    /// elapsed. A timed-out command is not a command that exited
    /// non-zero, so the two are reported differently.
    pub timed_out: bool,
}

impl NativeOutcome {
    /// The failing tail of the captured output: stderr when it carries
    /// content, stdout otherwise.
    pub fn tail(&self, limit: usize) -> String {
        let detail = if self.stderr.trim().is_empty() {
            &self.stdout
        } else {
            &self.stderr
        };
        truncate_tail(detail, limit)
    }

    /// The evidence lines of the run: stdout and stderr, each bounded.
    pub fn evidence(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.stdout.trim().is_empty() {
            lines.push(format!(
                "stdout: {}",
                truncate_head(&self.stdout, MAX_EVIDENCE_CHARS)
            ));
        }
        if !self.stderr.trim().is_empty() {
            lines.push(format!(
                "stderr: {}",
                truncate_head(&self.stderr, MAX_EVIDENCE_CHARS)
            ));
        }
        lines
    }
}

/// Split a command string into a program and its arguments.
///
/// The command is split on whitespace, so it carries no shell syntax. An
/// empty command is refused with a reason; each caller wraps that reason in
/// its own error variant, so the shared runner names no caller.
pub fn split_command(command: &str) -> Result<(String, Vec<String>), String> {
    let mut parts: Vec<String> = command.split_whitespace().map(str::to_string).collect();
    if parts.is_empty() {
        return Err("empty native command; nothing was verified".to_string());
    }
    let program = parts.remove(0);
    Ok((program, parts))
}

/// Run a native command as an argument array in `dir`.
///
/// `env` is layered on top of the inherited environment: every entry
/// replaces the inherited value of that variable. A command that cannot
/// start is reported through [`NativeOutcome::start_error`] rather than
/// as an error, so each caller renders the failure in its own error
/// variant. The `Result` is reserved for a future refusal; the runner
/// itself refuses nothing.
pub fn run_native(
    dir: &Path,
    program: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<NativeOutcome, ForgeError> {
    run_native_with_timeout(dir, program, args, env, NATIVE_COMMAND_TIMEOUT)
}

/// Run a native command as an argument array under an explicit bound.
///
/// Every product path binds [`NATIVE_COMMAND_TIMEOUT`]. This entry
/// point takes the bound as a parameter so a test can exercise the
/// bound without waiting the full window; a product path never passes
/// another bound.
pub fn run_native_with_timeout(
    dir: &Path,
    program: &str,
    args: &[String],
    env: &[(String, String)],
    timeout: Duration,
) -> Result<NativeOutcome, ForgeError> {
    let mut command = Command::new(program);
    command.args(args).current_dir(dir);
    apply_env(&mut command, env);
    run_bounded(&mut command, timeout)
}

/// Run a native command through `sh -c` in `dir`.
///
/// The command is honoured as written: a profile's declared test
/// command may rely on shell syntax — pipes, redirections, `&&` — and
/// the shell form is the only one that carries it. Everything else —
/// the working directory, the environment layering, the redaction, the
/// bounded capture and the wall-clock bound — is the runner's.
pub fn run_native_shell(
    dir: &Path,
    command: &str,
    env: &[(String, String)],
) -> Result<NativeOutcome, ForgeError> {
    run_native_shell_with_timeout(dir, command, env, NATIVE_COMMAND_TIMEOUT)
}

/// Run a native command through `sh -c` under an explicit bound.
///
/// Every product path binds [`NATIVE_COMMAND_TIMEOUT`]. This entry
/// point takes the bound as a parameter so a test can exercise the
/// bound without waiting the full window; a product path never passes
/// another bound.
pub fn run_native_shell_with_timeout(
    dir: &Path,
    command: &str,
    env: &[(String, String)],
    timeout: Duration,
) -> Result<NativeOutcome, ForgeError> {
    let mut shell = Command::new("sh");
    shell.arg("-c").arg(command).current_dir(dir);
    apply_env(&mut shell, env);
    run_bounded(&mut shell, timeout)
}

/// Layer `env` on top of the inherited environment.
fn apply_env(command: &mut Command, env: &[(String, String)]) {
    for (key, value) in env {
        command.env(key, value);
    }
}

/// Run `command` under `timeout` and capture its outcome.
///
/// The command is spawned as the leader of its own process group, so
/// the deadline can signal the whole tree the runner created. A
/// command that overruns the bound is killed, reaped and reported as
/// a timeout: no exit code, no success. A command that cannot start
/// is reported through [`NativeOutcome::start_error`].
///
/// The capture is bounded too: a descendant that outlives the command
/// keeps the capture pipes open, so the captures are given a grace
/// period and the group is killed when they do not finish. A runner
/// that waits for a descendant is a runner that hangs.
fn run_bounded(command: &mut Command, timeout: Duration) -> Result<NativeOutcome, ForgeError> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match spawn_in_own_group(command) {
        Ok(child) => child,
        Err(err) => return Ok(start_error_outcome(&err.to_string())),
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let stderr_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let stdout_reader = spawn_capture(stdout, Arc::clone(&stdout_buf), done_tx.clone());
    let stderr_reader = spawn_capture(stderr, Arc::clone(&stderr_buf), done_tx);
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    // The whole group goes first: the direct child is
                    // often a shell whose real work runs as a
                    // grandchild, and killing only the direct child
                    // would leave that descendant running.
                    kill_process_tree(&mut child);
                    finish_capture(&done_rx, stdout_reader, stderr_reader);
                    return Ok(timeout_outcome(&stdout_buf, &stderr_buf));
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(err) => {
                kill_process_tree(&mut child);
                finish_capture(&done_rx, stdout_reader, stderr_reader);
                return Ok(start_error_outcome(&format!(
                    "cannot wait on the command: {err}"
                )));
            }
        }
    };
    if !wait_for_capture(&done_rx, CAPTURE_GRACE) {
        // The command is gone, but something it started still holds
        // the capture pipes open. The group goes with it: the host is
        // left with no process the runner created.
        kill_process_tree(&mut child);
        let _ = wait_for_capture(&done_rx, CAPTURE_GRACE);
    }
    join_capture(stdout_reader, stderr_reader);
    Ok(NativeOutcome {
        exit_code: status.code(),
        success: status.success(),
        stdout: capture(&stdout_buf),
        stderr: capture(&stderr_buf),
        start_error: None,
        timed_out: false,
    })
}

/// Read one captured stream to end and report the capture's end.
fn spawn_capture<R: Read + Send + 'static>(
    stream: Option<R>,
    buf: Arc<Mutex<Vec<u8>>>,
    done: mpsc::Sender<()>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        if let Some(mut stream) = stream {
            let mut local = Vec::new();
            let _ = stream.read_to_end(&mut local);
            if let Ok(mut guard) = buf.lock() {
                *guard = local;
            }
        }
        let _ = done.send(());
    })
}

/// Wait for both captures to end, bounded by `grace`. True when both
/// finished.
fn wait_for_capture(done: &mpsc::Receiver<()>, grace: Duration) -> bool {
    let deadline = Instant::now() + grace;
    let mut finished = 0;
    while finished < 2 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        match done.recv_timeout(remaining) {
            Ok(()) => finished += 1,
            Err(_) => return false,
        }
    }
    true
}

/// Join the capture threads after the group was killed. A thread that
/// did not finish is detached rather than waited for: the runner
/// never blocks on a descendant that outlived the command.
fn join_capture(stdout: thread::JoinHandle<()>, stderr: thread::JoinHandle<()>) {
    if stdout.is_finished() {
        let _ = stdout.join();
    }
    if stderr.is_finished() {
        let _ = stderr.join();
    }
}

/// Wait for the captures to end after the group was killed.
fn finish_capture(
    done: &mpsc::Receiver<()>,
    stdout: thread::JoinHandle<()>,
    stderr: thread::JoinHandle<()>,
) {
    let _ = wait_for_capture(done, CAPTURE_GRACE);
    join_capture(stdout, stderr);
}

/// The outcome of a command that could not run at all.
fn start_error_outcome(reason: &str) -> NativeOutcome {
    NativeOutcome {
        exit_code: None,
        success: false,
        stdout: String::new(),
        stderr: String::new(),
        start_error: Some(reason.to_string()),
        timed_out: false,
    }
}

/// The outcome of a command that overran the bound: no exit code, no
/// success, and the output it managed to produce.
fn timeout_outcome(
    stdout_buf: &Arc<Mutex<Vec<u8>>>,
    stderr_buf: &Arc<Mutex<Vec<u8>>>,
) -> NativeOutcome {
    NativeOutcome {
        exit_code: None,
        success: false,
        stdout: capture(stdout_buf),
        stderr: capture(stderr_buf),
        start_error: None,
        timed_out: true,
    }
}

/// The redacted, bounded text of one captured stream.
fn capture(buf: &Arc<Mutex<Vec<u8>>>) -> String {
    let bytes = buf.lock().map(|guard| guard.clone()).unwrap_or_default();
    bound(&redact_credentials(&String::from_utf8_lossy(&bytes)))
}

/// Bound a captured stream to the last [`MAX_CAPTURE_CHARS`] characters.
///
/// The tail is kept because a failure is reported at the end of the
/// output: a bound never changes the last characters a reader sees.
fn bound(text: &str) -> String {
    let count = text.chars().count();
    if count <= MAX_CAPTURE_CHARS {
        return text.to_string();
    }
    text.chars().skip(count - MAX_CAPTURE_CHARS).collect()
}

/// The last `limit` characters of `text`, with the surrounding
/// whitespace removed.
fn truncate_tail(text: &str, limit: usize) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed
        .chars()
        .rev()
        .take(limit)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

/// The first `limit` characters of `text`, marked when the text was
/// longer.
fn truncate_head(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let head: String = text.chars().take(limit).collect();
    format!("{head}…")
}
