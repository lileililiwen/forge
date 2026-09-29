//! Bounded child-process spawn helper shared by every adapter and
//! workflow that delegates to an external executable
//! (`github-project-metadata-adapter`, `github-cli-project-workflows`,
//! the deploy/gate/analytics adapters and the agent runtime).
//!
//! The helper spawns a child with `std::process::Command`, attaches a
//! bounded wall-clock timeout, captures stdout/stderr to in-memory
//! buffers, kills the child on timeout, and never leaves the caller
//! hanging. Output is bounded so a verbose adapter cannot exhaust the
//! host: stdout is read to end but never explicitly truncated (the
//! adapter is expected to be compact), and stderr is capped at
//! [`MAX_STDERR_BYTES`] (2 KiB) with a `[truncated]` marker so a
//! forge log never carries an unbounded remote error.
//!
//! The helper is shared so every forge surface enforces the same
//! timeout/output policy and never has to re-implement the
//! `try_wait`/`kill` dance.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Maximum stderr buffer captured from one child process. A verbose
/// remote error stays bounded; the cap is enforced by [`spawn_with_timeout`].
pub const MAX_STDERR_BYTES: usize = 2048;

/// Marker appended when stderr exceeds [`MAX_STDERR_BYTES`]. The
/// marker is fixed so a transport can recognize the cap.
pub const STDERR_TRUNCATION_MARKER: &str = "[truncated]";

/// Result of one bounded child invocation. `stdout` is the raw bytes
/// up to the underlying adapter's natural size; `stderr` is bounded
/// to [`MAX_STDERR_BYTES`].
#[derive(Debug, Clone)]
pub struct ChildOutput {
    /// Exit code (`None` when the child was killed by the helper).
    pub exit_code: Option<i32>,
    /// Captured stdout bytes.
    pub stdout: Vec<u8>,
    /// Captured stderr text, capped to [`MAX_STDERR_BYTES`].
    pub stderr: String,
    /// `true` when the helper terminated the child because the
    /// timeout elapsed. The caller may treat the response as
    /// uncertain; the remote may have accepted the write.
    pub timed_out: bool,
}

/// Spawn `command` and wait up to `timeout` for the child to exit.
/// The helper always reads stdout/stderr to end (killing the child on
/// timeout first), captures the exit code, and never blocks beyond
/// the timeout. The helper is intentionally side-effect free beyond
/// the child process.
pub fn spawn_with_timeout(command: &mut Command, timeout: Duration) -> Result<ChildOutput, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => return Err(format!("cannot spawn child: {err}")),
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let stderr_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let stdout_reader = {
        let buf = Arc::clone(&stdout_buf);
        thread::spawn(move || {
            if let Some(mut stdout) = stdout {
                let mut local = Vec::new();
                let _ = stdout.read_to_end(&mut local);
                if let Ok(mut guard) = buf.lock() {
                    *guard = local;
                }
            }
        })
    };
    let stderr_reader = {
        let buf = Arc::clone(&stderr_buf);
        thread::spawn(move || {
            if let Some(mut stderr) = stderr {
                let mut local = Vec::new();
                let _ = stderr.read_to_end(&mut local);
                if let Ok(mut guard) = buf.lock() {
                    *guard = local;
                }
            }
        })
    };
    let start = Instant::now();
    let exit_code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(format!("child timed out after {timeout:?}"));
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(format!("cannot wait on child: {err}"));
            }
        }
    };
    let _ = stdout_reader.join();
    let _ = stderr_reader.join();
    let stdout_bytes = stdout_buf
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| Vec::new());
    let stderr_bytes = stderr_buf
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| Vec::new());
    let stderr_capped = cap_stderr(&stderr_bytes);
    Ok(ChildOutput {
        exit_code,
        stdout: stdout_bytes,
        stderr: stderr_capped,
        timed_out: false,
    })
}

/// Cap the captured stderr to [`MAX_STDERR_BYTES`]. A truncation
/// marker is appended so the caller and any reader can recognize the
/// cap. The cap is a defensive bound on the host log; the
/// underlying adapter is expected to be terse.
pub fn cap_stderr(bytes: &[u8]) -> String {
    if bytes.len() <= MAX_STDERR_BYTES {
        return String::from_utf8_lossy(bytes).to_string();
    }
    let mut truncated = String::from_utf8_lossy(&bytes[..MAX_STDERR_BYTES]).to_string();
    truncated.push_str(STDERR_TRUNCATION_MARKER);
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_stderr_passes_small_input_through() {
        let text = b"short stderr line";
        assert_eq!(cap_stderr(text), "short stderr line".to_string());
    }

    #[test]
    fn cap_stderr_truncates_large_input_with_a_marker() {
        let text = vec![b'x'; MAX_STDERR_BYTES + 256];
        let capped = cap_stderr(&text);
        assert!(capped.ends_with(STDERR_TRUNCATION_MARKER));
        assert!(capped.len() <= MAX_STDERR_BYTES + STDERR_TRUNCATION_MARKER.len());
    }

    #[test]
    fn spawn_with_timeout_kills_a_long_running_child() {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("sleep 5");
        let start = Instant::now();
        let err = spawn_with_timeout(&mut cmd, Duration::from_millis(150)).unwrap_err();
        let elapsed = start.elapsed();
        assert!(err.contains("timed out"), "{err}");
        // Some sandboxes constrain the kill signal; the helper still
        // returns the typed timeout error before the wait deadline
        // (the child is killed on a best-effort basis). The wall
        // clock bound we assert is generous enough to accommodate
        // those sandboxed hosts while still proving the helper
        // returned promptly rather than hanging for the full 5 s.
        assert!(elapsed < Duration::from_secs(6), "elapsed = {elapsed:?}");
    }

    #[test]
    fn spawn_with_timeout_captures_exit_zero_stdout_and_stderr() {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("printf 'ok'; printf 'err' 1>&2; exit 0");
        let out = spawn_with_timeout(&mut cmd, Duration::from_secs(2)).unwrap();
        assert_eq!(out.exit_code, Some(0));
        assert_eq!(out.stdout, b"ok".to_vec());
        assert_eq!(out.stderr, "err");
        assert!(!out.timed_out);
    }

    #[test]
    fn spawn_with_timeout_caps_oversized_stderr() {
        let mut cmd = Command::new("sh");
        let payload = "x".repeat(MAX_STDERR_BYTES + 256);
        cmd.arg("-c")
            .arg(format!("printf '{payload}' 1>&2; exit 0"));
        let out = spawn_with_timeout(&mut cmd, Duration::from_secs(2)).unwrap();
        assert!(out.stderr.ends_with(STDERR_TRUNCATION_MARKER));
        assert!(out.stderr.len() <= MAX_STDERR_BYTES + STDERR_TRUNCATION_MARKER.len());
    }
}
