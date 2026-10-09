//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::contract::{EVIDENCE_TIMEOUT, MAX_RECEIPT_CHARS, PROVIDER_CONTRACT_VERSION};
use super::matrix::{binary_env_for, default_binary_for, display_for, redact_provider_evidence};
use super::model::{
    CapturedRun, EvidenceProvenance, ProviderRow, ProviderStatus, RowParams, RunOptions,
    SandboxKind,
};

pub(super) fn run_bounded(
    binary: &Path,
    args: &[String],
    stdin_bytes: Option<&[u8]>,
    cwd: Option<&Path>,
) -> CapturedRun {
    let mut cmd = Command::new(binary);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    cmd.stdin(if stdin_bytes.is_some() {
        std::process::Stdio::piped()
    } else {
        std::process::Stdio::null()
    });
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            return CapturedRun {
                exit_code: None,
                stdout: String::new(),
                stderr: format!("spawn failed: {err}"),
                timed_out: false,
            };
        }
    };
    if let Some(bytes) = stdin_bytes {
        use std::io::Write;
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(bytes);
        }
        drop(child.stdin.take());
    }
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                use std::io::Read;
                let mut stdout = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_end(&mut stdout);
                }
                let mut stderr = Vec::new();
                if let Some(mut err) = child.stderr.take() {
                    let _ = err.read_to_end(&mut stderr);
                }
                return CapturedRun {
                    exit_code: status.code(),
                    stdout: String::from_utf8_lossy(&stdout).to_string(),
                    stderr: String::from_utf8_lossy(&stderr).to_string(),
                    timed_out: false,
                };
            }
            Ok(None) => {
                if start.elapsed() > EVIDENCE_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return CapturedRun {
                        exit_code: None,
                        stdout: String::new(),
                        stderr: format!("timeout after {:?}", EVIDENCE_TIMEOUT),
                        timed_out: true,
                    };
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(err) => {
                return CapturedRun {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: format!("wait failed: {err}"),
                    timed_out: false,
                };
            }
        }
    }
}

pub(super) fn probe_version(binary: &Path) -> Option<String> {
    let out = run_bounded(binary, &["--version".to_string()], None, None);
    if out.timed_out || out.exit_code != Some(0) {
        return None;
    }
    let line = out.stdout.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        return None;
    }
    // A fixture (or a compromised binary) answers every invocation with
    // its payload, so the version probe output is untrusted evidence:
    // redact it like every other captured string before it reaches a
    // receipt, a row or the journal.
    let redacted = redact_provider_evidence(line);
    let short: String = redacted.chars().take(120).collect();
    if short.trim().is_empty() {
        None
    } else {
        Some(short)
    }
}

pub(super) fn truncate_receipt(text: &str) -> String {
    let redacted = redact_provider_evidence(text);
    if redacted.len() <= MAX_RECEIPT_CHARS {
        return redacted;
    }
    let mut end = MAX_RECEIPT_CHARS;
    while !redacted.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…[truncated]", &redacted[..end])
}

pub(super) fn git_revision(dir: &Path) -> Option<String> {
    let out = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(dir)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
}

pub(super) fn probe_binary_reachability(provider: &str) -> ProviderRow {
    let binary = resolve_probe_binary(provider, &RunOptions::default());
    let source = binary.clone();
    let path = Path::new(&binary);
    let reason = if !path.exists()
        && Command::new(&binary)
            .arg("--version")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_err()
    {
        format!("live probe binary `{binary}` is not on PATH; provider is unavailable, not healthy")
    } else {
        format!("live probe for `{provider}` requires a project target; run `forge provider run {provider} <project> --live` for an attributable round trip")
    };
    ProviderRow {
        contract: PROVIDER_CONTRACT_VERSION.to_string(),
        provider: provider.to_string(),
        display: display_for(provider).to_string(),
        status: if reason.contains("requires a project target") {
            ProviderStatus::NotRun.id().to_string()
        } else {
            ProviderStatus::Unavailable.id().to_string()
        },
        reason: redact_provider_evidence(&reason),
        provenance: Some(EvidenceProvenance {
            provider: provider.to_string(),
            sandbox: SandboxKind::Live.id().to_string(),
            source,
            project_id: None,
            revision: None,
            observed_at: Utc::now().to_rfc3339(),
            tool_version: None,
            receipt: Vec::new(),
            teardown: true,
            note: "reachability only; no provider round trip was attempted".to_string(),
        }),
        evidence: Vec::new(),
    }
}

pub(super) fn resolve_probe_binary(provider: &str, options: &RunOptions) -> String {
    if let Some(fixture) = &options.fixture {
        return fixture.display().to_string();
    }
    if let Some(env) = binary_env_for(provider) {
        if let Ok(value) = std::env::var(env) {
            if !value.trim().is_empty() {
                return value;
            }
        }
    }
    match default_binary_for(provider) {
        // Providers whose real binaries ship under several names (the
        // sibling is `driftwatchdog` on cargo hosts, `driftwatch` on npm
        // hosts) list them `|`-separated in resolution order: probe the
        // PATH in order and run the first hit, so a host with only one
        // name installed still resolves.
        Some(default) if default.contains('|') => {
            let candidates: Vec<&str> = default.split('|').collect();
            let path_env = std::env::var_os("PATH").unwrap_or_default();
            crate::policy::first_binary_on_path(&path_env, &candidates)
                .map(|found| found.display().to_string())
                .unwrap_or_else(|| default.to_string())
        }
        Some(default) => default.to_string(),
        None => "forge-provider-probe".to_string(),
    }
}

pub(super) fn sandbox_of(options: &RunOptions) -> SandboxKind {
    if options.fixture.is_some() {
        SandboxKind::Fixture
    } else {
        SandboxKind::Live
    }
}

pub(super) fn sandbox_source(binary: &str, sandbox: SandboxKind) -> String {
    format!("{}:{binary}", sandbox.id())
}

/// Remove what a probe created inside `dir` and report whether the
/// directory is clean. Targeted live runs pass the project's state
/// layout; temp probes pass their temp dir.
pub fn teardown_probe(dir: &Path) -> bool {
    if !dir.is_dir() {
        return true;
    }
    let marker = dir.join(".forge-provider-probe");
    if marker.exists() {
        let _ = fs::remove_file(&marker);
    }
    !marker.exists()
}

/// Seven parallel attribution arguments is the narrowest honest shape for
/// an unavailable row (provider identity plus the failure diagnostics);
/// the supported path carries more fields and goes through
/// [`RowParams::supported`] instead.
#[allow(clippy::too_many_arguments)]
pub(super) fn unavailable_row(
    provider: &str,
    sandbox: SandboxKind,
    source: &str,
    project_id: &str,
    revision: Option<String>,
    diagnostics: &str,
    temp_dir: Option<&Path>,
) -> ProviderRow {
    let teardown = temp_dir.map(teardown_probe).unwrap_or(true);
    ProviderRow {
        contract: PROVIDER_CONTRACT_VERSION.to_string(),
        provider: provider.to_string(),
        display: display_for(provider).to_string(),
        status: ProviderStatus::Unavailable.id().to_string(),
        reason: format!(
            "controlled {sandbox} round trip did not produce a healthy result; no success state is recorded",
            sandbox = sandbox.id()
        ),
        provenance: Some(EvidenceProvenance {
            provider: provider.to_string(),
            sandbox: sandbox.id().to_string(),
            source: source.to_string(),
            project_id: Some(project_id.to_string()),
            revision,
            observed_at: Utc::now().to_rfc3339(),
            tool_version: None,
            receipt: vec![truncate_receipt(diagnostics)],
            teardown,
            note: "unavailable is reported, never a healthy result".to_string(),
        }),
        evidence: vec![truncate_receipt(diagnostics)],
    }
}

pub(super) fn params<'a>(
    provider: &'a str,
    sandbox: SandboxKind,
    source: &'a str,
    project_id: &'a str,
    revision: Option<String>,
    temp_dir: Option<&'a Path>,
) -> RowParams<'a> {
    RowParams {
        provider,
        sandbox,
        source,
        project_id,
        revision,
        temp_dir,
    }
}

pub(super) fn temp_probe_dir(provider: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "forge-provider-{provider}-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    let _ = fs::create_dir_all(&dir);
    dir
}

pub(super) fn cleanup_temp_dir(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

pub(super) fn probe_policy(
    options: &RunOptions,
    project_id: &str,
    workdir: &Path,
    revision: Option<String>,
) -> ProviderRow {
    let sandbox = sandbox_of(options);
    let source_name = policy_probe_target(options)
        .map(|found| {
            if options.fixture.is_some() {
                found.display().to_string()
            } else {
                found
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("driftwatch")
                    .to_string()
            }
        })
        .unwrap_or_else(|| crate::policy::DRIFTWATCH_BINARY_CANDIDATES[0].to_string());
    let source = sandbox_source(&source_name, sandbox);
    let tool_version = policy_probe_target(options).and_then(|found| probe_version(&found));
    let Some(target) = policy_probe_target(options) else {
        return unavailable_row(
            "driftwatch-policy",
            sandbox,
            &source,
            project_id,
            revision,
            &format!(
                "no driftwatch binary found: tried '{}' (argument-array probe, bounded wait); \
                 nothing was contacted",
                crate::policy::DRIFTWATCH_BINARY_CANDIDATES.join("', '")
            ),
            Some(workdir),
        );
    };
    let surface = crate::policy::policy_surface_args(workdir)
        .iter()
        .map(|arg| arg.to_string())
        .collect::<Vec<String>>();
    let out = run_bounded(&target, &surface, None, Some(workdir));
    if out.timed_out {
        return unavailable_row(
            "driftwatch-policy",
            sandbox,
            &source,
            project_id,
            revision,
            &format!("driftwatch probe timed out after {:?}", EVIDENCE_TIMEOUT),
            Some(workdir),
        );
    }
    // A parseable document is a successful round trip whatever the exit
    // code: a blocked gate or a failing checker is evidence, not adapter
    // failure. Only unparseable/empty stdout degrades to unavailable.
    let value: serde_json::Value = match serde_json::from_str::<serde_json::Value>(&out.stdout) {
        Ok(value) if value.is_object() => value,
        Ok(_) => {
            return unavailable_row(
                "driftwatch-policy",
                sandbox,
                &source,
                project_id,
                revision,
                "driftwatch output is valid JSON but not a policy document object",
                Some(workdir),
            );
        }
        Err(err) => {
            return unavailable_row(
                "driftwatch-policy",
                sandbox,
                &source,
                project_id,
                revision,
                &format!(
                    "driftwatch output is not parseable JSON (exit {:?}): {err}: {}",
                    out.exit_code,
                    out.stderr.trim()
                ),
                Some(workdir),
            );
        }
    };
    let receipt = vec![truncate_receipt(&out.stdout)];
    let mut evidence = vec![format!(
        "tool_version={}",
        tool_version.as_deref().unwrap_or("unknown")
    )];
    // Shape attribution so a human reading the row can tell which
    // surface answered without re-running anything.
    if let Some(contract) = value.get("contract").and_then(|c| c.as_str()) {
        evidence.push(format!("contract={contract}"));
    }
    if let Some(rows) = value.get("checkers").and_then(|c| c.as_array()) {
        evidence.push(format!("checkers={}", rows.len()));
    }
    if let Some(findings) = value.get("findings").and_then(|f| f.as_array()) {
        evidence.push(format!("findings={}", findings.len()));
    }
    if let Some(blocked) = value.get("blocked").and_then(|b| b.as_bool()) {
        evidence.push(format!("gate_blocked={blocked}"));
    }
    if out.exit_code != Some(0) {
        evidence.push(format!(
            "exit={:?}: parseable document; findings keep their own severities",
            out.exit_code
        ));
    }
    params(
        "driftwatch-policy",
        sandbox,
        &source,
        project_id,
        revision,
        Some(workdir),
    )
    .supported(
        tool_version,
        receipt,
        evidence,
        "policy probe observed a well-formed document; findings keep their own severities",
    )
}

/// Drive the `gate-runtime` probe: resolve the runtime the same ordered
/// way the gate plane does (fixture, then `FORGE_GATE_BIN`, then the
/// PATH probe of the sibling binary names) and exercise ONLY the
/// side-effect-free dry-run plan surface. A gate pass is never claimed
/// here; only that the runtime's gate surface responds.
pub(super) fn probe_gate_runtime(
    options: &RunOptions,
    project_id: &str,
    workdir: &Path,
    revision: Option<String>,
) -> ProviderRow {
    let sandbox = sandbox_of(options);
    let target = gate_runtime_probe_target(options);
    let source_name = target
        .as_ref()
        .map(|found| {
            if options.fixture.is_some() {
                found.display().to_string()
            } else {
                found
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("driftwatch")
                    .to_string()
            }
        })
        .unwrap_or_else(|| crate::policy::DRIFTWATCH_BINARY_CANDIDATES[0].to_string());
    let source = sandbox_source(&source_name, sandbox);
    let Some(target) = target else {
        return unavailable_row(
            "gate-runtime",
            sandbox,
            &source,
            project_id,
            revision,
            &format!(
                "no gate runtime resolvable: tried `{}` then PATH candidates '{}' (the gate \
                 plane's own ordered resolution); nothing was contacted",
                crate::gate::GATE_BIN_ENV,
                crate::policy::DRIFTWATCH_BINARY_CANDIDATES.join("', '")
            ),
            Some(workdir),
        );
    };
    let tool_version = probe_version(&target);
    let out = run_bounded(
        &target,
        &[
            "gate".to_string(),
            "--dry-run".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ],
        None,
        Some(workdir),
    );
    if out.timed_out {
        return unavailable_row(
            "gate-runtime",
            sandbox,
            &source,
            project_id,
            revision,
            &format!("gate dry-run probe timed out after {:?}", EVIDENCE_TIMEOUT),
            Some(workdir),
        );
    }
    // Forward-compat first: should a runtime version answer the dry-run
    // with a parseable status document, its aggregate stands as
    // attributed evidence (and the probe still persists nothing).
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&out.stdout) {
        if value.is_object() && value.get("blocked").is_some() && value.get("results").is_some() {
            let aggregate = match (
                value.get("status").and_then(|s| s.as_str()),
                value.get("blocked").and_then(|b| b.as_bool()),
            ) {
                (Some("PASS"), Some(false)) => "passed",
                (_, Some(true)) => "blocked",
                (Some("FAIL"), Some(false)) => "failed",
                _ => "unknown",
            };
            let mut evidence = vec![
                format!(
                    "tool_version={}",
                    tool_version.as_deref().unwrap_or("unknown")
                ),
                format!("surface=gate-status aggregate={aggregate}"),
            ];
            if out.exit_code != Some(0) {
                evidence.push(format!(
                    "exit={:?}: parseable document; the document's own aggregate stands",
                    out.exit_code
                ));
            }
            return params(
                "gate-runtime",
                sandbox,
                &source,
                project_id,
                revision,
                Some(workdir),
            )
            .supported(
                tool_version,
                vec![truncate_receipt(&out.stdout)],
                evidence,
                "the gate runtime answered with a parseable status document; the probe claims nothing beyond the document",
            );
        }
    }
    // The real sibling surface (verified at 25811ed): a clean-exit human
    // plan preview, or the honest "nothing to gate" answer for a project
    // without a gate manifest. Both prove the gate surface exists without
    // executing or persisting anything; neither is a gate pass.
    if out.exit_code == Some(0) {
        let first_line = out
            .stdout
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
            .to_string();
        let (label, note) = if first_line.contains("nothing to gate") {
            (
                "plan=none (target is not gate-managed)",
                "the gate surface responded; this target carries no gate manifest, so no plan resolved and no gate pass is claimed",
            )
        } else {
            (
                "plan-preview observed (nothing was executed)",
                "the gate runtime answered the side-effect-free dry-run plan surface; no gate pass is claimed",
            )
        };
        let evidence = vec![
            format!(
                "tool_version={}",
                tool_version.as_deref().unwrap_or("unknown")
            ),
            format!("surface=gate-dry-run {label}"),
        ];
        return params(
            "gate-runtime",
            sandbox,
            &source,
            project_id,
            revision,
            Some(workdir),
        )
        .supported(
            tool_version,
            vec![truncate_receipt(&out.stdout)],
            evidence,
            note,
        );
    }
    unavailable_row(
        "gate-runtime",
        sandbox,
        &source,
        project_id,
        revision,
        &format!(
            "gate dry-run probe failed (exit {:?}): {}",
            out.exit_code,
            out.stderr.trim()
        ),
        Some(workdir),
    )
}

/// Ordered gate-runtime resolution for the probe: fixture script, then
/// `FORGE_GATE_BIN`, then the PATH probe of the real sibling names —
/// mirroring the gate plane's own fallback without executing anything.
fn gate_runtime_probe_target(options: &RunOptions) -> Option<std::path::PathBuf> {
    if let Some(fixture) = &options.fixture {
        return Some(fixture.clone());
    }
    if let Ok(value) = std::env::var(crate::gate::GATE_BIN_ENV) {
        if !value.trim().is_empty() {
            return Some(std::path::PathBuf::from(value));
        }
    }
    let path_env = std::env::var_os("PATH").unwrap_or_default();
    crate::policy::first_binary_on_path(&path_env, crate::policy::DRIFTWATCH_BINARY_CANDIDATES)
}

/// Where the driftwatch-policy probe sends its invocation: the fixture
/// script, then `FORGE_DRIFTWATCH_BIN`, then the ordered PATH probe of
/// the real sibling binary names. `None` means nothing is installed and
/// the probe must contact nothing.
fn policy_probe_target(options: &RunOptions) -> Option<std::path::PathBuf> {
    if let Some(fixture) = &options.fixture {
        return Some(fixture.clone());
    }
    if let Ok(value) = std::env::var("FORGE_DRIFTWATCH_BIN") {
        if !value.trim().is_empty() {
            return Some(std::path::PathBuf::from(value));
        }
    }
    let path_env = std::env::var_os("PATH").unwrap_or_default();
    crate::policy::first_binary_on_path(&path_env, crate::policy::DRIFTWATCH_BINARY_CANDIDATES)
}

pub(super) fn probe_analytics(
    options: &RunOptions,
    project_id: &str,
    project_ref: &str,
    revision: Option<String>,
) -> ProviderRow {
    let binary = resolve_probe_binary("analytics", options);
    let sandbox = sandbox_of(options);
    let source = sandbox_source(&binary, sandbox);
    let tool_version = probe_version(Path::new(&binary));
    let out = run_bounded(
        Path::new(&binary),
        &[
            "health".to_string(),
            "--provider".to_string(),
            "unified-content".to_string(),
            "--project".to_string(),
            project_id.to_string(),
            "--project-ref".to_string(),
            project_ref.to_string(),
            "--plane".to_string(),
            "content".to_string(),
        ],
        None,
        None,
    );
    if out.timed_out {
        return unavailable_row(
            "analytics",
            sandbox,
            &source,
            project_id,
            revision,
            &format!("analytics probe timed out after {:?}", EVIDENCE_TIMEOUT),
            None,
        );
    }
    if out.exit_code != Some(0) {
        return unavailable_row(
            "analytics",
            sandbox,
            &source,
            project_id,
            revision,
            &format!(
                "analytics adapter exited with status {:?}: {}",
                out.exit_code,
                out.stderr.trim()
            ),
            None,
        );
    }
    let payload: serde_json::Value = match serde_json::from_str(&out.stdout) {
        Ok(payload) => payload,
        Err(err) => {
            return unavailable_row(
                "analytics",
                sandbox,
                &source,
                project_id,
                revision,
                &format!("analytics adapter returned non-JSON output: {err}"),
                None,
            );
        }
    };
    let reported_ref = payload
        .get("project_ref")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if reported_ref != project_ref {
        let diagnostics = format!(
            "adapter reported project_ref=`{reported_ref}`; expected `{project_ref}`; refusing to attach another project's data"
        );
        let row = unavailable_row(
            "analytics",
            sandbox,
            &source,
            project_id,
            revision,
            &diagnostics,
            None,
        );
        // A mapping refusal is structurally distinct from a transport
        // failure: surface it as `ambiguous-mapping` in the evidence so
        // callers can distinguish "wrong project" from "provider down".
        let mut row = row;
        row.evidence = vec![format!("ambiguous-mapping: {diagnostics}")];
        if let Some(provenance) = row.provenance.as_mut() {
            provenance.note = "ambiguous-mapping is refused, not a transport failure".to_string();
        }
        return row;
    }
    let evidence_items: Vec<String> = payload
        .get("evidence")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str().map(truncate_receipt))
                .take(8)
                .collect()
        })
        .unwrap_or_default();
    params("analytics", sandbox, &source, project_id, revision, None).supported(
        tool_version,
        vec![truncate_receipt(&out.stdout)],
        evidence_items,
        "analytics probe observed a project-scoped health payload",
    )
}
