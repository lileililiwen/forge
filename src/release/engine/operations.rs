//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{
    ReleaseIdentity, ReleaseRequest, ReleaseState, ADAPTER_TIMEOUT, RELEASE_CONTRACT_VERSION,
};
use crate::core::ForgeError;
use crate::registry::Registry;
use std::fs;
use std::path::Path;
use std::process::Command;

use super::model::ReleaseListEntry;

pub(super) fn run_package_adapter(
    bin: &str,
    project_dir: &Path,
    package: &super::super::PackageSpec,
    identity: &ReleaseIdentity,
) -> Result<String, String> {
    let mut cmd = Command::new(bin);
    cmd.arg("--contract")
        .arg(RELEASE_CONTRACT_VERSION)
        .arg("--package-kind")
        .arg(&package.kind)
        .arg("--package-name")
        .arg(&package.name)
        .arg("--package-path")
        .arg(&package.path)
        .arg("--package-version")
        .arg(&package.version)
        .arg("--release-id")
        .arg(&identity.id)
        .arg("--source-revision")
        .arg(&identity.source_revision)
        .arg("--project-dir")
        .arg(project_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let output =
        run_with_timeout(cmd, ADAPTER_TIMEOUT, "package adapter").map_err(|err| err.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "package adapter `{bin}` exited {}: {} {}",
            output.status,
            stderr.trim(),
            stdout.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let receipt = stdout.lines().next().unwrap_or("").trim().to_string();
    if receipt.is_empty() {
        return Err(format!("package adapter `{bin}` returned an empty receipt"));
    }
    Ok(receipt)
}

pub(super) fn run_container_adapter(
    bin: &str,
    project_dir: &Path,
    container: &super::super::ContainerSpec,
    identity: &ReleaseIdentity,
) -> Result<String, String> {
    let mut cmd = Command::new(bin);
    cmd.arg("--contract")
        .arg(RELEASE_CONTRACT_VERSION)
        .arg("--container-name")
        .arg(&container.name)
        .arg("--dockerfile")
        .arg(&container.dockerfile)
        .arg("--container-tag")
        .arg(&container.tag)
        .arg("--release-id")
        .arg(&identity.id)
        .arg("--source-revision")
        .arg(&identity.source_revision)
        .arg("--project-dir")
        .arg(project_dir);
    if let Some(registry) = &container.registry {
        cmd.arg("--registry").arg(registry);
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let output = run_with_timeout(cmd, ADAPTER_TIMEOUT, "container adapter")
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "container adapter `{bin}` exited {}: {} {}",
            output.status,
            stderr.trim(),
            stdout.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let receipt = stdout.lines().next().unwrap_or("").trim().to_string();
    if receipt.is_empty() {
        return Err(format!(
            "container adapter `{bin}` returned an empty receipt"
        ));
    }
    Ok(receipt)
}

pub(super) fn run_notes_adapter(
    bin: &str,
    project_dir: &Path,
    notes: &super::super::NotesSpec,
    identity: &ReleaseIdentity,
    request: &ReleaseRequest,
) -> Result<String, String> {
    let mut cmd = Command::new(bin);
    cmd.arg("--contract")
        .arg(RELEASE_CONTRACT_VERSION)
        .arg("--template")
        .arg(&notes.template)
        .arg("--output")
        .arg(&notes.output)
        .arg("--release-id")
        .arg(&identity.id)
        .arg("--source-revision")
        .arg(&identity.source_revision)
        .arg("--project-dir")
        .arg(project_dir)
        .arg("--dry-run")
        .arg(if request.dry_run { "true" } else { "false" });
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let output =
        run_with_timeout(cmd, ADAPTER_TIMEOUT, "notes adapter").map_err(|err| err.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "notes adapter `{bin}` exited {}: {} {}",
            output.status,
            stderr.trim(),
            stdout.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let receipt = stdout.lines().next().unwrap_or("").trim().to_string();
    if receipt.is_empty() {
        return Err(format!("notes adapter `{bin}` returned an empty receipt"));
    }
    Ok(receipt)
}

pub(super) fn run_with_timeout(
    mut cmd: Command,
    timeout: std::time::Duration,
    label: &str,
) -> Result<std::process::Output, String> {
    let mut child = cmd
        .spawn()
        .map_err(|err| format!("{label} spawn failed: {err}"))?;
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return wait_with_output(child, status),
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{label} exceeded the {timeout:?} timeout"));
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{label} wait failed: {err}"));
            }
        }
    }
}

fn wait_with_output(
    mut child: std::process::Child,
    status: std::process::ExitStatus,
) -> Result<std::process::Output, String> {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_thread = stdout.map(|mut s| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = std::io::Read::read_to_end(&mut s, &mut buf);
            buf
        })
    });
    let stderr_thread = stderr.map(|mut s| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = std::io::Read::read_to_end(&mut s, &mut buf);
            buf
        })
    });
    let _ = child.wait();
    let stdout = stdout_thread
        .and_then(|t| t.join().ok())
        .unwrap_or_default();
    let stderr = stderr_thread
        .and_then(|t| t.join().ok())
        .unwrap_or_default();
    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

pub fn list_releases(
    project_dir: &Path,
    project_id: &str,
) -> Result<Vec<ReleaseListEntry>, ForgeError> {
    let root = project_dir
        .join(super::super::RELEASE_STATE_DIR)
        .join(project_id);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<ReleaseListEntry> = Vec::new();
    for dir in fs::read_dir(&root).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!("cannot read release directory {}: {err}", root.display()),
    })? {
        let entry = match dir {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path().join("state.json");
        let state = match super::super::load_release_state(&path) {
            Ok(state) => state,
            Err(_) => continue,
        };
        entries.push(ReleaseListEntry {
            project_id: state.identity.project_id.clone(),
            release_id: state.identity.id.clone(),
            version: state.identity.version.clone(),
            source_revision: state.identity.source_revision.clone(),
            stage_count: state.stage_outcomes.len(),
            last_run_at: state.last_run_at.clone(),
        });
    }
    entries.sort_by(|a, b| b.last_run_at.cmp(&a.last_run_at));
    Ok(entries)
}

/// Inspect one persisted release. Returns `Ok(None)` when the
/// release id is not found.
pub fn read_release(
    project_dir: &Path,
    project_id: &str,
    release_id: &str,
) -> Result<Option<ReleaseState>, ForgeError> {
    if release_id.trim().is_empty() {
        return Err(ForgeError::ReleaseInvalid {
            reason: "release id must not be empty".to_string(),
        });
    }
    let path = project_dir
        .join(super::super::RELEASE_STATE_DIR)
        .join(project_id)
        .join(release_id)
        .join("state.json");
    if !path.is_file() {
        return Ok(None);
    }
    let state = super::super::load_release_state(&path)?;
    Ok(Some(state))
}

/// Journal a release operation in the registry so the
/// originating intent is preserved. The CLI and the MCP
/// transport call this after every prepare/apply; the kind
/// is `release`.
pub fn record_release_operation(
    registry: &Registry,
    project_id: &str,
    state_label: &str,
    detail: &str,
) {
    let _ = registry.record_operation("release", project_id, state_label, detail);
}
