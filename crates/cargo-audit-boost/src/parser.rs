use crate::model::BuildState;
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

pub fn parse_package_id(package_id: &str) -> (String, String) {
    if let Some(pos) = package_id.rfind('#') {
        let after_hash = &package_id[pos + 1..];
        let before_hash = &package_id[..pos];
        if let Some(at_pos) = after_hash.find('@') {
            let name = &after_hash[..at_pos];
            let version = &after_hash[at_pos + 1..];
            (name.to_string(), version.to_string())
        } else {
            // Local path dependencies or workspaces:
            // "path+file:///path/to/crate#0.1.0" -> name is last segment of URL path
            let version = after_hash.to_string();
            let name = before_hash
                .split('/')
                .next_back()
                .unwrap_or(before_hash)
                .to_string();
            (name, version)
        }
    } else {
        (package_id.to_string(), "unknown".to_string())
    }
}

pub async fn run_cargo(
    dir: &str,
    cargo_args: &[String],
    state: Arc<Mutex<BuildState>>,
) -> Result<()> {
    // We want to force cargo to output JSON messages on stdout.
    // Also, we append `--message-format=json` to the user's arguments if not present.
    let mut cmd_args = cargo_args.to_vec();
    if !cmd_args
        .iter()
        .any(|arg| arg.starts_with("--message-format"))
    {
        cmd_args.push("--message-format=json".to_string());
    }

    let mut child = Command::new("cargo")
        .args(&cmd_args)
        .current_dir(dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn cargo process")?;

    let stdout = child.stdout.take().context("Failed to open stdout")?;
    let stderr = child.stderr.take().context("Failed to open stderr")?;

    let state_stdout = Arc::clone(&state);
    let stdout_handle = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                let mut state = state_stdout.lock().await;
                let now = Instant::now();
                if let Some(reason) = val.get("reason").and_then(|r| r.as_str()) {
                    match reason {
                        "compiler-artifact" => {
                            if let Some(pkg_id) = val.get("package_id").and_then(|p| p.as_str()) {
                                let (name, version) = parse_package_id(pkg_id);
                                let cached =
                                    val.get("fresh").and_then(|f| f.as_bool()).unwrap_or(false);
                                state.mark_completed(&name, &version, now, cached);
                            }
                        }
                        "compiler-message" => {
                            if let Some(pkg_id) = val.get("package_id").and_then(|p| p.as_str())
                                && let Some(msg) = val.get("message")
                            {
                                let (name, version) = parse_package_id(pkg_id);
                                let level = msg.get("level").and_then(|l| l.as_str()).unwrap_or("");
                                if level == "error" {
                                    state.mark_failed(&name, &version, now);
                                }

                                // Extract diagnostic message
                                let diagnostic = if let Some(rendered) =
                                    msg.get("rendered").and_then(|r| r.as_str())
                                {
                                    rendered.to_string()
                                } else if let Some(message_text) =
                                    msg.get("message").and_then(|m| m.as_str())
                                {
                                    message_text.to_string()
                                } else {
                                    String::new()
                                };

                                if !diagnostic.is_empty() {
                                    state.add_diagnostic(&name, &version, diagnostic);
                                }
                            }
                        }
                        "build-finished" => {
                            if let Some(success) = val.get("success").and_then(|s| s.as_bool()) {
                                state.success = Some(success);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    let state_stderr = Arc::clone(&state);
    let stderr_handle = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let trimmed = line.trim();
            if trimmed.starts_with("Compiling ") || trimmed.starts_with("Checking ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    let name = parts[1];
                    let mut version = parts[2];
                    if version.starts_with('v') {
                        version = &version[1..];
                    }
                    let mut state = state_stderr.lock().await;
                    state.mark_compiling(name, version, Instant::now());
                }
            }
        }
    });

    // Wait for the process to complete and the stream readers to finish
    let _status = child.wait().await?;
    let _ = tokio::join!(stdout_handle, stderr_handle);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BuildState;

    #[test]
    fn test_parse_package_id_registry() {
        let pkg_id = "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.228";
        let (name, version) = parse_package_id(pkg_id);
        assert_eq!(name, "serde");
        assert_eq!(version, "1.0.228");
    }

    #[test]
    fn test_parse_package_id_git() {
        let pkg_id = "git+https://github.com/tokio-rs/tokio?branch=master#tokio@1.35.0";
        let (name, version) = parse_package_id(pkg_id);
        assert_eq!(name, "tokio");
        assert_eq!(version, "1.35.0");
    }

    #[test]
    fn test_parse_package_id_path() {
        let pkg_id = "path+file:///home/paul/code/cargo-audit-boost#0.1.0";
        let (name, version) = parse_package_id(pkg_id);
        assert_eq!(name, "cargo-audit-boost");
        assert_eq!(version, "0.1.0");
    }

    #[test]
    fn test_parse_package_id_fallback() {
        let pkg_id = "some-weird-id";
        let (name, version) = parse_package_id(pkg_id);
        assert_eq!(name, "some-weird-id");
        assert_eq!(version, "unknown");
    }

    #[test]
    fn test_diagnostics_collection() {
        let mut state = BuildState::new();
        state.add_diagnostic("serde", "1.0.228", "warning: unused import".to_string());
        state.add_diagnostic(
            "serde",
            "1.0.228",
            "warning: field is never read".to_string(),
        );

        let info = state.crates.get("serde").unwrap();
        assert_eq!(info.diagnostics.len(), 2);
        assert_eq!(info.diagnostics[0], "warning: unused import");
        assert_eq!(info.diagnostics[1], "warning: field is never read");
    }
}
