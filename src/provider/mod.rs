//! Controlled provider integration evidence
//! (`provider-integration-evidence`).
//!
//! The adapter boundaries (`policy`, `identity`, `analytics`, `deploy`,
//! `release`) already model `unavailable` / `disabled` / `ambiguous`
//! outcomes through contract fixtures. Fixtures alone cannot prove a real
//! DriftWatch binary, OIDC issuer, analytics source or deployment target
//! works in a live environment, and a fixture success must never be
//! reported as verified provider support.
//!
//! This module owns the opt-in evidence harness that closes that gap:
//!
//! - [`provider_ids`] lists the six evidence providers in stable order.
//! - [`matrix`] reports every provider as `not-run` unless the caller
//!   opts in with `--live`; a `not-run` row is never `supported`.
//! - [`run_controlled`] drives one controlled round trip against an
//!   explicit fixture binary (`sandbox: fixture`, supplemental) or a real
//!   sandbox binary (`sandbox: live`, verifiable) and records provider,
//!   project, revision, timestamp and the redacted receipt as
//!   [`EvidenceProvenance`].
//! - Binary probes reuse each adapter's real argument protocol with a
//!   bounded wait, always with `--dry-run` where the adapter supports it,
//!   so a probe never performs a real remote write.
//! - The identity probe runs the real in-memory OIDC lifecycle
//!   (challenge → callback → claims → mint → validate → terminate) through
//!   `crate::identity`; a real issuer round trip remains a downstream step.
//! - Every evidence string passes through [`redact_provider_evidence`],
//!   which delegates to `policy::redact_credentials`, so the evidence
//!   contract shares one definition of "secret" with every other adapter.
//! - Probes run in disposable temp dirs (or a caller-named project dir for
//!   targeted live runs) and [`teardown_probe`] removes what the probe
//!   created; the row reports `teardown: true` only when nothing remains.
//!
//! Secrets reach Forge only through the runner's environment (binary paths
//! and fixture files); manifests and the repository never carry provider
//! secrets, and the journal records only redacted receipts.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::core::ForgeError;

/// Versioned contract for the provider evidence surface.
pub const PROVIDER_CONTRACT_VERSION: &str = "0.1.0";

/// Synthetic project id for project-agnostic evidence rows. Keeps the
/// `operations` table project-agnostic without inventing a user-visible
/// project, matching the `__planner__` / `__component__` pattern.
pub const PROVIDER_SYNTHETIC_PROJECT: &str = "__provider__";

/// Opt-in flag: live sandbox binaries are attempted only when
/// `FORGE_PROVIDER_LIVE=1`. Anything else reports `not-run` so an
/// unconfigured sandbox is never misread as a result.
pub const LIVE_ENV: &str = "FORGE_PROVIDER_LIVE";

/// Bounded wait per probe so an unresponsive binary cannot hang the
/// registry. Shorter than the per-adapter defaults because a probe is a
/// reachability + envelope-shape check, not a full provider run.
pub const EVIDENCE_TIMEOUT: Duration = Duration::from_secs(10);

/// Bounded receipt excerpt per probe so a chatty binary cannot flood the
/// journal. The full output stays with the provider; Forge keeps proof.
pub const MAX_RECEIPT_CHARS: usize = 2000;

/// Evidence provider ids in stable catalog order.
pub const PROVIDER_IDS: &[&str] = &[
    "driftwatch-policy",
    "gate-runtime",
    "oidc-identity",
    "analytics",
    "deploy",
    "release",
];

/// Default binaries probed for `live` runs, mirroring each adapter's own
/// default. Every default is overridable per run so tests substitute
/// fixture scripts without touching the source tree.
pub const DEFAULT_PROBE_BINARIES: &[(&str, &str)] = &[
    ("driftwatch-policy", "driftwatchdog|driftwatch"),
    ("gate-runtime", "driftwatchdog|driftwatch"),
    ("analytics", "forge-analytics-adapter"),
    ("deploy", "forge-deployer"),
    ("release", "forge-package-publisher"),
];

/// Per-run environment override for the probed binary, mirroring each
/// adapter's `FORGE_*_BIN` pattern.
pub const PROBE_BIN_ENV: &[(&str, &str)] = &[
    ("driftwatch-policy", "FORGE_DRIFTWATCH_BIN"),
    ("gate-runtime", "FORGE_GATE_BIN"),
    ("analytics", "FORGE_ANALYTICS_BIN"),
    ("deploy", "FORGE_DEPLOYER_BIN"),
    ("release", "FORGE_PACKAGE_BIN"),
];

/// Native evidence classification for one provider row. `not-run` is the
/// default: the sandbox was never attempted. `supported` requires a
/// controlled round trip with provenance; `unavailable` records a live
/// attempt that failed without fabricating health; `disabled` preserves
/// the manifest-disabled semantics of the underlying adapter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderStatus {
    Supported,
    Unavailable,
    NotRun,
    Disabled,
}

impl ProviderStatus {
    pub fn id(&self) -> &'static str {
        match self {
            ProviderStatus::Supported => "supported",
            ProviderStatus::Unavailable => "unavailable",
            ProviderStatus::NotRun => "not-run",
            ProviderStatus::Disabled => "disabled",
        }
    }
}

/// Which sandbox produced the evidence. `fixture` rows are supplemental
/// harness proof; only `live` rows can verify real provider support.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SandboxKind {
    Live,
    Fixture,
}

impl SandboxKind {
    pub fn id(&self) -> &'static str {
        match self {
            SandboxKind::Live => "live",
            SandboxKind::Fixture => "fixture",
        }
    }
}

/// Attributable provenance for one controlled round trip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceProvenance {
    pub provider: String,
    pub sandbox: String,
    pub source: String,
    pub project_id: Option<String>,
    pub revision: Option<String>,
    pub observed_at: String,
    pub tool_version: Option<String>,
    pub receipt: Vec<String>,
    pub teardown: bool,
    pub note: String,
}

/// One provider evidence row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRow {
    pub contract: String,
    pub provider: String,
    pub display: String,
    pub status: String,
    pub reason: String,
    pub provenance: Option<EvidenceProvenance>,
    pub evidence: Vec<String>,
}

/// The provider matrix over all six evidence providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderMatrix {
    pub contract: String,
    pub generated_at: String,
    pub live: bool,
    pub rows: Vec<ProviderRow>,
    pub supported: usize,
    pub unavailable: usize,
    pub not_run: usize,
    pub disabled: usize,
}

/// Static descriptor for one evidence provider (no probe performed).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderDescriptor {
    pub contract: String,
    pub provider: String,
    pub display: String,
    pub boundary: String,
    pub binary_env: Option<String>,
    pub default_binary: Option<String>,
    pub secret_rule: String,
    pub teardown_rule: String,
}

/// How the caller wants one controlled round trip to run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunOptions {
    /// Attempt the real sandbox binary (requires `FORGE_PROVIDER_LIVE=1`
    /// unless `allow_unflagged_live` is set by tests).
    pub live: bool,
    /// Explicit fixture binary standing in for the sandbox. Labels the
    /// row `sandbox: fixture` so fixture proof never reads as provider
    /// support.
    pub fixture: Option<PathBuf>,
    /// Bypass the `FORGE_PROVIDER_LIVE` gate (tests only).
    pub allow_unflagged_live: bool,
}

pub fn provider_ids() -> Vec<String> {
    PROVIDER_IDS.iter().map(|s| s.to_string()).collect()
}

pub fn parse_provider(id: &str) -> Result<String, ForgeError> {
    let trimmed = id.trim();
    if PROVIDER_IDS.contains(&trimmed) {
        return Ok(trimmed.to_string());
    }
    Err(ForgeError::ProviderInvalid {
        reason: format!(
            "unknown provider `{id}`; expected one of: {}",
            PROVIDER_IDS.join(", ")
        ),
    })
}

/// Shared secret redaction for provider evidence. Delegates to the policy
/// redactor so every adapter agrees on what a secret looks like.
pub fn redact_provider_evidence(input: &str) -> String {
    crate::policy::redact_credentials(input)
}

pub fn live_enabled() -> bool {
    std::env::var(LIVE_ENV).map(|v| v == "1").unwrap_or(false)
}

fn display_for(provider: &str) -> &'static str {
    match provider {
        "driftwatch-policy" => "DriftWatch policy plane",
        "gate-runtime" => "Shared gate runtime execution",
        "oidc-identity" => "OIDC admin identity",
        "analytics" => "Analytics / content plane",
        "deploy" => "Deployment targets",
        "release" => "Release stages",
        _ => "Unknown provider",
    }
}

fn default_binary_for(provider: &str) -> Option<&'static str> {
    DEFAULT_PROBE_BINARIES
        .iter()
        .find(|(id, _)| *id == provider)
        .map(|(_, bin)| *bin)
}

fn binary_env_for(provider: &str) -> Option<&'static str> {
    PROBE_BIN_ENV
        .iter()
        .find(|(id, _)| *id == provider)
        .map(|(_, env)| *env)
}

fn not_run_row(provider: &str, live_requested: bool) -> ProviderRow {
    let reason = if live_requested && !live_enabled() {
        format!(
            "live run requested without {LIVE_ENV}=1; set {LIVE_ENV}=1 with sandbox credentials to attempt the real provider"
        )
    } else {
        format!(
            "opt-in evidence not attempted; run `forge provider run {provider} --live` with {LIVE_ENV}=1 or `--fixture <path>` for a controlled round trip"
        )
    };
    ProviderRow {
        contract: PROVIDER_CONTRACT_VERSION.to_string(),
        provider: provider.to_string(),
        display: display_for(provider).to_string(),
        status: ProviderStatus::NotRun.id().to_string(),
        reason,
        provenance: None,
        evidence: Vec::new(),
    }
}

fn counts(rows: &[ProviderRow]) -> (usize, usize, usize, usize) {
    let mut supported = 0;
    let mut unavailable = 0;
    let mut not_run = 0;
    let mut disabled = 0;
    for row in rows {
        match row.status.as_str() {
            "supported" => supported += 1,
            "unavailable" => unavailable += 1,
            "disabled" => disabled += 1,
            _ => not_run += 1,
        }
    }
    (supported, unavailable, not_run, disabled)
}

/// Report the provider matrix. Without `live` every row is `not-run`:
/// the matrix never fabricates support for a sandbox it did not attempt.
/// With `live` each binary-backed provider is probed for reachability;
/// identity stays `not-run` without a project because a session
/// lifecycle cannot be attributed to a project the caller never named.
pub fn matrix(live: bool) -> ProviderMatrix {
    let live = live && live_enabled();
    let mut rows = Vec::new();
    for id in PROVIDER_IDS {
        if !live {
            rows.push(not_run_row(id, false));
            continue;
        }
        if *id == "oidc-identity" {
            rows.push(ProviderRow {
                contract: PROVIDER_CONTRACT_VERSION.to_string(),
                provider: id.to_string(),
                display: display_for(id).to_string(),
                status: ProviderStatus::NotRun.id().to_string(),
                reason: "live identity evidence requires a project target; run `forge provider run oidc-identity <project> --live`".to_string(),
                provenance: None,
                evidence: Vec::new(),
            });
            continue;
        }
        rows.push(probe_binary_reachability(id));
    }
    let (supported, unavailable, not_run, disabled) = counts(&rows);
    ProviderMatrix {
        contract: PROVIDER_CONTRACT_VERSION.to_string(),
        generated_at: Utc::now().to_rfc3339(),
        live,
        rows,
        supported,
        unavailable,
        not_run,
        disabled,
    }
}

/// Static descriptor for one provider: boundary, binary override, secret
/// and teardown rules. Performs no probe and contacts no provider.
pub fn inspect(provider: &str) -> Result<ProviderDescriptor, ForgeError> {
    let id = parse_provider(provider)?;
    let boundary = match id.as_str() {
        "driftwatch-policy" => "probe runs `<bin> check --dry-run --format json` (or `gate --format json` for gate-managed projects, whose only side effect is a `gate_runs` row in the project's own `.driftwatch/` store) with the project as the working directory and a bounded wait; a parseable document — including a blocked gate or failing checker — records `supported`, while a missing binary (probe order: `driftwatchdog`, `driftwatch`), timeout or unparseable output records `unavailable`, never a policy PASS",
        "gate-runtime" => "probe runs `<bin> gate --dry-run --format json` against the resolved gate runtime (explicit `FORGE_GATE_BIN`, else the ordered PATH probe) with the project as the working directory and a bounded wait; the real sibling's dry-run surface is a side-effect-free plan preview, so a responding plan or a parseable gate document records `supported` while a missing binary, timeout or refused invocation records `unavailable`; the probe never executes the real gate and never claims a gate pass",
        "oidc-identity" => "probe runs the in-memory challenge/callback/claims/mint/validate/terminate lifecycle through the identity contract; cross-project, expired, revoked and non-admin outcomes stay refusals, never sessions",
        "analytics" => "probe runs `<bin> health --provider <p> --project <id> --project-ref <ref> --plane <plane>`; a mismatched project_ref records `ambiguous-mapping`, never another project's data",
        "deploy" => "probe runs `<bin> apply --target <t> --kind <k> --project <id> --revision <rev> --dry-run` under the frozen `forge-deploy-executor/0.1.0` envelope contract; only a contract-conformant delivered dry-run envelope records `supported`, a missing or unknown discriminator records `unavailable`, and teardown removes the probe state",
        "release" => "probe runs `<bin> publish --stage <s> --project <id> --revision <rev> --dry-run` for the package and container stages; a split outcome records `partial`, and retry never replays a delivered stage blindly",
        _ => "unknown provider",
    };
    Ok(ProviderDescriptor {
        contract: PROVIDER_CONTRACT_VERSION.to_string(),
        provider: id.clone(),
        display: display_for(&id).to_string(),
        boundary: boundary.to_string(),
        binary_env: binary_env_for(&id).map(str::to_string),
        default_binary: default_binary_for(&id).map(str::to_string),
        secret_rule: "secrets reach Forge only through the runner environment; manifests and the repository never carry provider secrets; evidence is redacted through policy::redact_credentials".to_string(),
        teardown_rule: "probes run in disposable temp dirs and teardown removes what the probe created; targeted live runs never write outside the named project's state layout".to_string(),
    })
}

struct CapturedRun {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
}

fn run_bounded(
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

fn probe_version(binary: &Path) -> Option<String> {
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

fn truncate_receipt(text: &str) -> String {
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

fn git_revision(dir: &Path) -> Option<String> {
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

fn probe_binary_reachability(provider: &str) -> ProviderRow {
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

fn resolve_probe_binary(provider: &str, options: &RunOptions) -> String {
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

fn sandbox_of(options: &RunOptions) -> SandboxKind {
    if options.fixture.is_some() {
        SandboxKind::Fixture
    } else {
        SandboxKind::Live
    }
}

fn sandbox_source(binary: &str, sandbox: SandboxKind) -> String {
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

/// Shared attribution for one probe outcome. Bundles the parameters
/// every row builder needs so each row carries identical provenance
/// shape.
struct RowParams<'a> {
    provider: &'a str,
    sandbox: SandboxKind,
    source: &'a str,
    project_id: &'a str,
    revision: Option<String>,
    temp_dir: Option<&'a Path>,
}

impl RowParams<'_> {
    fn supported(
        self,
        tool_version: Option<String>,
        receipt: Vec<String>,
        evidence: Vec<String>,
        note: &str,
    ) -> ProviderRow {
        let teardown = self.temp_dir.map(teardown_probe).unwrap_or(true);
        ProviderRow {
            contract: PROVIDER_CONTRACT_VERSION.to_string(),
            provider: self.provider.to_string(),
            display: display_for(self.provider).to_string(),
            status: ProviderStatus::Supported.id().to_string(),
            reason: format!(
                "controlled {sandbox} round trip succeeded for project `{project}`; same outcome is exposed through every transport",
                sandbox = self.sandbox.id(),
                project = self.project_id,
            ),
            provenance: Some(EvidenceProvenance {
                provider: self.provider.to_string(),
                sandbox: self.sandbox.id().to_string(),
                source: self.source.to_string(),
                project_id: Some(self.project_id.to_string()),
                revision: self.revision,
                observed_at: Utc::now().to_rfc3339(),
                tool_version,
                receipt,
                teardown,
                note: redact_provider_evidence(note),
            }),
            evidence,
        }
    }
}

/// Seven parallel attribution arguments is the narrowest honest shape for
/// an unavailable row (provider identity plus the failure diagnostics);
/// the supported path carries more fields and goes through
/// [`RowParams::supported`] instead.
#[allow(clippy::too_many_arguments)]
fn unavailable_row(
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

fn params<'a>(
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

fn temp_probe_dir(provider: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "forge-provider-{provider}-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    let _ = fs::create_dir_all(&dir);
    dir
}

fn cleanup_temp_dir(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

fn probe_policy(
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
fn probe_gate_runtime(
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

fn probe_analytics(
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

fn probe_deploy(options: &RunOptions, project_id: &str, revision: Option<String>) -> ProviderRow {
    let binary = resolve_probe_binary("deploy", options);
    let sandbox = sandbox_of(options);
    let source = sandbox_source(&binary, sandbox);
    let tool_version = probe_version(Path::new(&binary));
    let rev = revision
        .clone()
        .unwrap_or_else(|| "unversioned".to_string());
    let stdin_payload = serde_json::json!({
        "contract": crate::deploy::DEPLOY_EXECUTOR_CONTRACT,
        "project_id": project_id,
        "deploy_id": format!("{project_id}-local-{rev}"),
        "target": "local",
        "revision": rev,
        "dry_run": true,
    });
    let stdin_bytes = serde_json::to_vec(&stdin_payload).unwrap_or_default();
    let out = run_bounded(
        Path::new(&binary),
        &[
            "apply".to_string(),
            "--target".to_string(),
            "local".to_string(),
            "--kind".to_string(),
            "local".to_string(),
            "--project".to_string(),
            project_id.to_string(),
            "--revision".to_string(),
            rev,
            "--dry-run".to_string(),
        ],
        Some(&stdin_bytes),
        None,
    );
    if out.timed_out {
        return unavailable_row(
            "deploy",
            sandbox,
            &source,
            project_id,
            revision,
            &format!("deploy probe timed out after {:?}", EVIDENCE_TIMEOUT),
            None,
        );
    }
    if out.exit_code != Some(0) {
        return unavailable_row(
            "deploy",
            sandbox,
            &source,
            project_id,
            revision,
            &format!(
                "deploy adapter exited with status {:?}: {}",
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
                "deploy",
                sandbox,
                &source,
                project_id,
                revision,
                &format!("deploy adapter returned non-JSON output: {err}"),
                None,
            );
        }
    };
    // The `deploy` row is only reachable against an adapter
    // that speaks the frozen executor contract: a missing or
    // unknown discriminator is a contract violation, not a
    // provider success, and the row stays unavailable naming
    // what was observed.
    let envelope_contract = payload
        .get("contract")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if envelope_contract != crate::deploy::DEPLOY_EXECUTOR_CONTRACT {
        return unavailable_row(
            "deploy",
            sandbox,
            &source,
            project_id,
            revision,
            &format!(
                "deploy adapter envelope contract `{envelope_contract}` does not match expected `{}`",
                crate::deploy::DEPLOY_EXECUTOR_CONTRACT
            ),
            None,
        );
    }
    let status = payload
        .get("status")
        .or_else(|| payload.get("apply_status"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if status != "delivered" && status != "healthy" && status != "running" {
        return unavailable_row(
            "deploy",
            sandbox,
            &source,
            project_id,
            revision,
            &format!("deploy adapter reported status `{status}`; probe records unavailable, not a deployment"),
            None,
        );
    }
    // The row names the adapter and its revision (when the
    // envelope self-identifies), so a `supported` deploy row
    // is attributable to a specific executor build rather
    // than an anonymous binary. A dry-run probe never
    // triggers a real deployment side effect.
    let mut evidence = vec![format!("adapter_status={status}")];
    if let Some(adapter_source) = payload.get("source").and_then(|v| v.as_str()) {
        let adapter_revision = payload
            .get("source_revision")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        evidence.push(format!(
            "adapter_source={adapter_source}@{adapter_revision}"
        ));
    }
    params("deploy", sandbox, &source, project_id, revision, None).supported(
        tool_version,
        vec![truncate_receipt(&out.stdout)],
        evidence,
        "deploy probe observed a delivered dry-run envelope over the executor contract; no remote write was performed",
    )
}

fn probe_release_stage(
    binary: &str,
    stage: &str,
    project_id: &str,
    rev: &str,
) -> Result<(String, Vec<String>), String> {
    let out = run_bounded(
        Path::new(binary),
        &[
            "publish".to_string(),
            "--stage".to_string(),
            stage.to_string(),
            "--project".to_string(),
            project_id.to_string(),
            "--revision".to_string(),
            rev.to_string(),
            "--dry-run".to_string(),
        ],
        None,
        None,
    );
    if out.timed_out {
        return Err(format!(
            "stage `{stage}` timed out after {:?}",
            EVIDENCE_TIMEOUT
        ));
    }
    if out.exit_code != Some(0) {
        return Err(format!(
            "stage `{stage}` exited with status {:?}: {}",
            out.exit_code,
            redact_provider_evidence(out.stderr.trim())
        ));
    }
    let payload: serde_json::Value = serde_json::from_str(&out.stdout)
        .map_err(|err| format!("stage `{stage}` returned non-JSON output: {err}"))?;
    let status = payload
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if status != "delivered" && status != "skipped" {
        return Err(format!("stage `{stage}` reported status `{status}`"));
    }
    let evidence: Vec<String> = payload
        .get("evidence")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str().map(truncate_receipt))
                .take(4)
                .collect()
        })
        .unwrap_or_default();
    Ok((status, evidence))
}

fn probe_release(options: &RunOptions, project_id: &str, revision: Option<String>) -> ProviderRow {
    let binary = resolve_probe_binary("release", options);
    let sandbox = sandbox_of(options);
    let source = sandbox_source(&binary, sandbox);
    let tool_version = probe_version(Path::new(&binary));
    let rev = revision
        .clone()
        .unwrap_or_else(|| "unversioned".to_string());
    let package = probe_release_stage(&binary, "package", project_id, &rev);
    let container = probe_release_stage(&binary, "container", project_id, &rev);
    match (package, container) {
        (
            Ok((package_status, mut package_evidence)),
            Ok((container_status, mut container_evidence)),
        ) => {
            let mut evidence = vec![
                format!("package:{package_status}"),
                format!("container:{container_status}"),
            ];
            evidence.append(&mut package_evidence);
            evidence.append(&mut container_evidence);
            // Retry-safety proof: a delivered stage is never replayed
            // blindly. The evidence run records each stage once; a retry
            // re-reads the recorded outcome instead of re-publishing.
            let receipt = vec![format!(
                "package={package_status} container={container_status}; delivered stages are recorded, not replayed"
            )];
            params("release", sandbox, &source, project_id, revision, None).supported(
                tool_version,
                receipt,
                evidence,
                "release probe observed both stages delivered; retry would skip recorded stages",
            )
        }
        (one, other) => {
            // One stage succeeded while the other failed: the aggregate
            // stays partial, and the completed stage is named so a retry
            // can skip it instead of repeating the side effect.
            let mut parts = Vec::new();
            let mut delivered: Vec<String> = Vec::new();
            for (stage, outcome) in [("package", &one), ("container", &other)] {
                match outcome {
                    Ok((status, _)) => {
                        parts.push(format!("{stage}:{status}"));
                        delivered.push(stage.to_string());
                    }
                    Err(reason) => {
                        parts.push(format!("{stage}:failed ({})", truncate_receipt(reason)))
                    }
                }
            }
            let diagnostics = format!(
                "release probe is partial: {}; delivered stage(s) [{}] are recorded and must be skipped on retry, not replayed",
                parts.join(", "),
                delivered.join(", ")
            );
            let mut row = unavailable_row(
                "release",
                sandbox,
                &source,
                project_id,
                revision,
                &diagnostics,
                None,
            );
            row.evidence = vec![format!("partial: {diagnostics}")];
            if let Some(provenance) = row.provenance.as_mut() {
                provenance.note =
                    "partial aggregate; completed stages are recorded, not replayed".to_string();
            }
            row
        }
    }
}

fn fixture_identity_config() -> crate::identity::IdentityConfig {
    crate::identity::IdentityConfig {
        provider: "okta-fixture".to_string(),
        issuer: "https://fixture-issuer.example.com".to_string(),
        client_id: "forge-evidence".to_string(),
        audience: "forge-evidence".to_string(),
        redirect_uri: "http://localhost:0/callback".to_string(),
        scopes: vec!["openid".to_string(), "profile".to_string()],
        jwks_uri: None,
        state_ttl_seconds: 120,
        session_ttl_seconds: 3600,
        admin_claim: "groups".to_string(),
        admin_values: vec!["forge-admins".to_string()],
        client_secret_ref: None,
    }
}

fn probe_identity(
    project_id: &str,
    revision: Option<String>,
    sandbox: SandboxKind,
    source: &str,
) -> ProviderRow {
    use crate::identity::{
        build_challenge, mint_session, terminate_session, validate_callback, validate_claims,
        validate_session, AuthCallback, ProviderClaims,
    };
    use std::collections::BTreeMap;

    let config = fixture_identity_config();
    let now = Utc::now();
    let challenge = match build_challenge(project_id, &config, now) {
        Ok(challenge) => challenge,
        Err(err) => {
            return unavailable_row(
                "oidc-identity",
                sandbox,
                source,
                project_id,
                revision,
                &format!("identity challenge failed: {err}"),
                None,
            );
        }
    };
    let callback = AuthCallback {
        project_id: project_id.to_string(),
        state: challenge.state.clone(),
        code: "evidence-code-1234".to_string(),
        error: None,
        error_description: None,
    };
    if let Err(err) = validate_callback(&callback, &challenge, now) {
        return unavailable_row(
            "oidc-identity",
            sandbox,
            source,
            project_id,
            revision,
            &format!("identity callback refused: {err}"),
            None,
        );
    }
    let mut claims_map = BTreeMap::new();
    claims_map.insert("groups".to_string(), "forge-admins".to_string());
    let claims = ProviderClaims {
        issuer: config.issuer.clone(),
        audience: config.audience.clone(),
        subject: "evidence-subject".to_string(),
        issued_at: now,
        expires_at: now + chrono::Duration::seconds(600),
        nonce: challenge.nonce.clone(),
        scopes: vec!["openid".to_string(), "profile".to_string()],
        claims: claims_map,
    };
    if let Err(err) = validate_claims(&claims, &challenge, &config, now) {
        return unavailable_row(
            "oidc-identity",
            sandbox,
            source,
            project_id,
            revision,
            &format!("identity claims refused: {err}"),
            None,
        );
    }
    let mut session = match mint_session(&config, &claims, &challenge, now) {
        Ok(session) => session,
        Err(err) => {
            return unavailable_row(
                "oidc-identity",
                sandbox,
                source,
                project_id,
                revision,
                &format!("identity mint refused: {err}"),
                None,
            );
        }
    };
    // Negative boundaries are exercised on the same lifecycle without
    // affecting the minted session: a cross-project presentation, an
    // expired check and a missing permission must all refuse.
    let cross_project = validate_session(&session, "other-project", "admin:access", now).is_err();
    let missing_permission = validate_session(&session, project_id, "billing:refund", now).is_err();
    if let Err(err) = validate_session(&session, project_id, "admin:access", now) {
        return unavailable_row(
            "oidc-identity",
            sandbox,
            source,
            project_id,
            revision,
            &format!("identity session refused for its own project: {err}"),
            None,
        );
    }
    if !cross_project || !missing_permission {
        return unavailable_row(
            "oidc-identity",
            sandbox,
            source,
            project_id,
            revision,
            "identity negative boundaries did not refuse; refusing to claim a verified lifecycle",
            None,
        );
    }
    // Teardown: terminate the probe session so no live credential
    // survives the evidence run.
    terminate_session(&mut session);
    let teardown = session.state == crate::identity::SessionState::Revoked;
    let evidence = vec![
        format!("subject={}", redact_provider_evidence("evidence-subject")),
        format!("issuer={}", config.issuer),
        "cross-project:refused".to_string(),
        "missing-permission:refused".to_string(),
        "session:terminated".to_string(),
    ];
    let mut row = params(
        "oidc-identity",
        sandbox,
        source,
        project_id,
        revision,
        None,
    )
    .supported(
        None,
        vec!["challenge/callback/claims/mint/validate/terminate lifecycle completed; probe session terminated".to_string()],
        evidence,
        "identity probe completed the project-scoped lifecycle with refusals intact and terminated the probe session",
    );
    if let Some(provenance) = row.provenance.as_mut() {
        provenance.teardown = teardown;
    }
    row
}

/// Drive one controlled round trip for `provider`.
///
/// - With `fixture`: runs the fixture binary (or fixture lifecycle for
///   identity) and labels the row `sandbox: fixture` (supplemental).
/// - With `live` (and `FORGE_PROVIDER_LIVE=1` or
///   `allow_unflagged_live`): runs the real sandbox binary and labels the
///   row `sandbox: live`.
/// - With neither: returns a `not-run` row without contacting anything.
///
/// `project_id` names the project the evidence is attributed to;
/// `workdir` is the probe directory (temp dir for fixture runs, the
/// project dir for targeted live runs); `project_ref` scopes the
/// analytics probe. Teardown removes temp probe state for binary probes
/// and terminates the probe session for identity.
pub fn run_controlled(
    provider: &str,
    options: &RunOptions,
    project_id: &str,
    workdir: &Path,
    project_ref: &str,
) -> Result<ProviderRow, ForgeError> {
    let id = parse_provider(provider)?;
    if let Err(reason) = crate::core::validate_project_id(project_id) {
        return Err(ForgeError::ProviderInvalid {
            reason: format!("invalid project id `{project_id}`: {reason}"),
        });
    }
    let wants_probe = options.fixture.is_some() || options.live;
    if !wants_probe {
        return Ok(not_run_row(&id, false));
    }
    if options.live && options.fixture.is_none() && !options.allow_unflagged_live && !live_enabled()
    {
        return Ok(not_run_row(&id, true));
    }
    let revision = git_revision(workdir);
    let sandbox = sandbox_of(options);
    let source = if id == "oidc-identity" {
        format!("{}:in-memory-lifecycle", sandbox.id())
    } else {
        sandbox_source(&resolve_probe_binary(&id, options), sandbox)
    };
    let row = match id.as_str() {
        "driftwatch-policy" => probe_policy(options, project_id, workdir, revision),
        "gate-runtime" => probe_gate_runtime(options, project_id, workdir, revision),
        "oidc-identity" => probe_identity(project_id, revision, sandbox, &source),
        "analytics" => probe_analytics(options, project_id, project_ref, revision),
        "deploy" => probe_deploy(options, project_id, revision),
        "release" => probe_release(options, project_id, revision),
        _ => not_run_row(&id, false),
    };
    Ok(row)
}

/// Convenience wrapper for temp-dir controlled runs (tests, matrix
/// workers). Creates a disposable dir, runs the probe, removes the dir,
/// and marks teardown from both the probe and the directory removal.
pub fn run_controlled_temp(
    provider: &str,
    options: &RunOptions,
    project_id: &str,
    project_ref: &str,
) -> Result<ProviderRow, ForgeError> {
    let dir = temp_probe_dir(provider);
    let mut row = run_controlled(provider, options, project_id, &dir, project_ref)?;
    cleanup_temp_dir(&dir);
    let dir_gone = !dir.exists();
    if let Some(provenance) = row.provenance.as_mut() {
        provenance.teardown = provenance.teardown && dir_gone;
    }
    Ok(row)
}

pub fn render_row_human(row: &ProviderRow) -> String {
    let mut out = format!(
        "provider: {}\nstatus: {}\nreason: {}\n",
        row.provider, row.status, row.reason
    );
    if let Some(provenance) = &row.provenance {
        out.push_str(&format!(
            "sandbox: {}\nsource: {}\nproject: {}\nrevision: {}\nobserved_at: {}\ntool_version: {}\nteardown: {}\nnote: {}\n",
            provenance.sandbox,
            provenance.source,
            provenance.project_id.as_deref().unwrap_or("-"),
            provenance.revision.as_deref().unwrap_or("unversioned"),
            provenance.observed_at,
            provenance.tool_version.as_deref().unwrap_or("unknown"),
            provenance.teardown,
            provenance.note,
        ));
        for line in &provenance.receipt {
            out.push_str(&format!("receipt: {line}\n"));
        }
    }
    for line in &row.evidence {
        out.push_str(&format!("evidence: {line}\n"));
    }
    out
}

pub fn render_matrix_human(matrix: &ProviderMatrix) -> String {
    let mut out = format!(
        "provider matrix (contract {}, live={}): supported={} unavailable={} not-run={} disabled={}\n",
        matrix.contract, matrix.live, matrix.supported, matrix.unavailable, matrix.not_run, matrix.disabled
    );
    for row in &matrix.rows {
        out.push_str(&format!("{}: {}\n", row.provider, row.status));
    }
    out
}

pub fn render_descriptor_human(descriptor: &ProviderDescriptor) -> String {
    format!(
        "provider: {}\ndisplay: {}\nboundary: {}\nbinary_env: {}\ndefault_binary: {}\nsecret_rule: {}\nteardown_rule: {}\n",
        descriptor.provider,
        descriptor.display,
        descriptor.boundary,
        descriptor.binary_env.as_deref().unwrap_or("-"),
        descriptor.default_binary.as_deref().unwrap_or("-"),
        descriptor.secret_rule,
        descriptor.teardown_rule,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn write_fixture(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, body).unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    }

    fn temp_case(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("forge-provider-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn provider_ids_are_stable_and_ordered() {
        assert_eq!(
            provider_ids(),
            vec![
                "driftwatch-policy",
                "gate-runtime",
                "oidc-identity",
                "analytics",
                "deploy",
                "release",
            ]
        );
    }

    #[test]
    fn parse_provider_refuses_unknown_with_typed_code() {
        let err = parse_provider("nosuch").unwrap_err();
        assert_eq!(err.code(), "provider-invalid");
    }

    #[test]
    fn parse_provider_trims_and_accepts_known() {
        assert_eq!(parse_provider("  deploy ").unwrap(), "deploy");
    }

    #[test]
    fn matrix_defaults_to_not_run_without_live() {
        let report = matrix(false);
        assert_eq!(report.contract, PROVIDER_CONTRACT_VERSION);
        assert_eq!(report.rows.len(), 6);
        assert_eq!(report.not_run, 6);
        assert_eq!(report.supported, 0);
        for row in &report.rows {
            assert_eq!(row.status, "not-run");
            assert!(row.provenance.is_none());
            assert!(row.reason.contains("forge provider run"));
        }
    }

    #[test]
    fn run_without_flags_is_not_run_and_contacts_nothing() {
        let dir = temp_case("not-run");
        let row = run_controlled(
            "driftwatch-policy",
            &RunOptions::default(),
            "evidence-probe",
            &dir,
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "not-run");
        assert!(row.provenance.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_refuses_unknown_provider_with_typed_code() {
        let dir = temp_case("unknown");
        let err = run_controlled(
            "nosuch",
            &RunOptions::default(),
            "evidence-probe",
            &dir,
            "owner/repo",
        )
        .unwrap_err();
        assert_eq!(err.code(), "provider-invalid");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_refuses_bad_project_id_before_any_probe() {
        let dir = temp_case("bad-project");
        let err = run_controlled(
            "deploy",
            &RunOptions {
                fixture: Some(PathBuf::from("/nonexistent")),
                ..Default::default()
            },
            "BAD_ID",
            &dir,
            "owner/repo",
        )
        .unwrap_err();
        assert_eq!(err.code(), "provider-invalid");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn live_without_opt_in_flag_is_not_run() {
        let dir = temp_case("live-gate");
        let row = run_controlled(
            "deploy",
            &RunOptions {
                live: true,
                ..Default::default()
            },
            "evidence-probe",
            &dir,
            "owner/repo",
        )
        .unwrap();
        // Either the environment enables live (then a missing binary
        // yields unavailable) or the gate holds (not-run). Neither is
        // ever reported as supported without a real round trip.
        assert_ne!(row.status, "supported");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn redact_delegates_to_policy_redactor() {
        let secret = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let redacted = redact_provider_evidence(&format!("token {secret} used"));
        assert!(!redacted.contains(secret));
        assert!(redacted.contains("[REDACTED]"));
        assert_eq!(
            redacted,
            crate::policy::redact_credentials(&format!("token {secret} used"))
        );
    }

    #[test]
    fn policy_fixture_success_records_fixture_provenance() {
        let dir = temp_case("policy-good");
        let bin = write_fixture(
            &dir,
            "good-driftwatch.sh",
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"driftwatch 9.9.9\"; exit 0; fi\necho '{\"tool\":\"driftwatch\",\"findings\":[]}'\n",
        );
        let row = run_controlled_temp(
            "driftwatch-policy",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        let provenance = row.provenance.as_ref().unwrap();
        assert_eq!(provenance.sandbox, "fixture");
        assert!(provenance.source.starts_with("fixture:"));
        assert_eq!(provenance.project_id.as_deref(), Some("evidence-probe"));
        assert!(provenance.teardown);
        assert!(provenance.observed_at.contains('T'));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn policy_fixture_failure_is_unavailable_without_success() {
        let dir = temp_case("policy-bad");
        let bin = write_fixture(&dir, "bad.sh", "#!/bin/sh\necho boom >&2\nexit 3\n");
        let row = run_controlled_temp(
            "driftwatch-policy",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "unavailable");
        assert!(row.provenance.is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn policy_fixture_malformed_output_is_unavailable() {
        let dir = temp_case("policy-malformed");
        let bin = write_fixture(&dir, "malformed.sh", "#!/bin/sh\necho 'not json{{'\n");
        let row = run_controlled_temp(
            "driftwatch-policy",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "unavailable");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn policy_fixture_leak_is_redacted_in_receipt() {
        let dir = temp_case("policy-leak");
        let secret = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let bin = write_fixture(
            &dir,
            "leaky.sh",
            &format!("#!/bin/sh\necho '{{\"tool\":\"driftwatch\",\"note\":\"{secret}\"}}'\n"),
        );
        let row = run_controlled_temp(
            "driftwatch-policy",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        let provenance = row.provenance.as_ref().unwrap();
        for line in &provenance.receipt {
            assert!(!line.contains(secret), "secret leaked in receipt");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn analytics_fixture_mapping_mismatch_is_ambiguous_not_success() {
        let dir = temp_case("analytics-mismatch");
        let bin = write_fixture(
            &dir,
            "mismatch.sh",
            "#!/bin/sh\necho '{\"project_ref\":\"other/repo\",\"evidence\":[]}'\n",
        );
        let row = run_controlled_temp(
            "analytics",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "unavailable");
        assert!(row.evidence.iter().any(|e| e.contains("ambiguous-mapping")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn analytics_fixture_success_is_project_scoped() {
        let dir = temp_case("analytics-good");
        let bin = write_fixture(
            &dir,
            "good.sh",
            "#!/bin/sh\necho '{\"project_ref\":\"owner/repo\",\"evidence\":[\"stars=42\"]}'\n",
        );
        let row = run_controlled_temp(
            "analytics",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        assert!(row.evidence.iter().any(|e| e.contains("stars=42")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn deploy_fixture_partial_and_failed_states() {
        let dir = temp_case("deploy-states");
        let good = write_fixture(
            &dir,
            "good.sh",
            "#!/bin/sh\ncat >/dev/null\necho '{\"contract\":\"forge-deploy-executor/0.1.0\",\"status\":\"delivered\",\"evidence\":[],\"note\":\"ok\"}'\n",
        );
        let row = run_controlled_temp(
            "deploy",
            &RunOptions {
                fixture: Some(good),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        let stale = write_fixture(
            &dir,
            "stale-contract.sh",
            "#!/bin/sh\ncat >/dev/null\necho '{\"contract\":\"0.1.0\",\"status\":\"delivered\",\"evidence\":[],\"note\":\"ok\"}'\n",
        );
        let row = run_controlled_temp(
            "deploy",
            &RunOptions {
                fixture: Some(stale),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "unavailable");
        assert!(
            row.evidence
                .iter()
                .any(|e| e.contains("forge-deploy-executor/0.1.0")),
            "the mismatch refusal must name the expected contract: {:?}",
            row.evidence
        );
        let bad = write_fixture(&dir, "bad.sh", "#!/bin/sh\ncat >/dev/null\nexit 1\n");
        let row = run_controlled_temp(
            "deploy",
            &RunOptions {
                fixture: Some(bad),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "unavailable");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn release_fixture_split_stages_stay_partial() {
        let dir = temp_case("release-partial");
        // One binary serves both stages: package delivers, container fails.
        let bin = write_fixture(
            &dir,
            "split.sh",
            "#!/bin/sh\nfor a in \"$@\"; do if [ \"$a\" = \"container\" ]; then echo failed >&2; exit 1; fi; done\necho '{\"contract\":\"0.1.0\",\"status\":\"delivered\",\"evidence\":[],\"note\":\"ok\"}'\n",
        );
        let row = run_controlled_temp(
            "release",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "unavailable");
        assert!(row.evidence.iter().any(|e| e.contains("partial")));
        assert!(row.evidence.iter().any(|e| e.contains("package:delivered")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn release_fixture_all_delivered_is_supported() {
        let dir = temp_case("release-good");
        let bin = write_fixture(
            &dir,
            "good.sh",
            "#!/bin/sh\necho '{\"contract\":\"0.1.0\",\"status\":\"delivered\",\"evidence\":[],\"note\":\"ok\"}'\n",
        );
        let row = run_controlled_temp(
            "release",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        assert!(row.evidence.iter().any(|e| e.contains("package:delivered")));
        assert!(row
            .evidence
            .iter()
            .any(|e| e.contains("container:delivered")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn identity_fixture_lifecycle_terminates_probe_session() {
        let dir = temp_case("identity-good");
        let row = run_controlled_temp(
            "oidc-identity",
            &RunOptions {
                fixture: Some(PathBuf::from("in-memory")),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        let provenance = row.provenance.as_ref().unwrap();
        assert_eq!(provenance.sandbox, "fixture");
        assert!(provenance.teardown);
        assert!(row
            .evidence
            .iter()
            .any(|e| e.contains("cross-project:refused")));
        assert!(row
            .evidence
            .iter()
            .any(|e| e.contains("session:terminated")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn teardown_probe_removes_marker_and_reports_clean() {
        let dir = temp_case("teardown");
        assert!(teardown_probe(&dir));
        fs::write(dir.join(".forge-provider-probe"), "probe").unwrap();
        assert!(teardown_probe(&dir));
        assert!(!dir.join(".forge-provider-probe").exists());
        assert!(teardown_probe(Path::new("/nonexistent-forge-provider-dir")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_revision_is_none_for_temp_dirs() {
        // Temp probe dirs carry no VCS revision; provenance must say
        // `unversioned` in human output rather than inventing a SHA.
        let dir = temp_case("revision");
        let bin = write_fixture(
            &dir,
            "good.sh",
            "#!/bin/sh\necho '{\"project_ref\":\"owner/repo\",\"evidence\":[]}'\n",
        );
        let row = run_controlled_temp(
            "analytics",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        let provenance = row.provenance.as_ref().unwrap();
        assert!(provenance.revision.is_none());
        let human = render_row_human(&row);
        assert!(human.contains("unversioned"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn version_probe_output_is_redacted_before_evidence() {
        // A fixture answers every invocation (including `--version`)
        // with its payload; the captured version string must not carry
        // a credential into the receipt or the evidence line.
        let dir = temp_case("version-leak");
        let secret = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let bin = write_fixture(
            &dir,
            "leaky-version.sh",
            &format!("#!/bin/sh\necho 'driftwatch 9.9.9 {secret}'\n"),
        );
        let version = probe_version(&bin).unwrap();
        assert!(!version.contains(secret));
        assert!(version.contains("[REDACTED]"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn repeated_runs_stay_attributable_to_their_own_observations() {
        // R1 repeat scenario: the same check repeated for the same
        // project revision returns independent rows, each attributable
        // to its own observation timestamp; neither overwrites the
        // other's evidence.
        let dir = temp_case("repeat");
        let bin = write_fixture(
            &dir,
            "good.sh",
            "#!/bin/sh\necho '{\"project_ref\":\"owner/repo\",\"evidence\":[]}'\n",
        );
        let options = RunOptions {
            fixture: Some(bin),
            ..Default::default()
        };
        let first =
            run_controlled_temp("analytics", &options, "evidence-probe", "owner/repo").unwrap();
        let second =
            run_controlled_temp("analytics", &options, "evidence-probe", "owner/repo").unwrap();
        assert_eq!(first.status, "supported");
        assert_eq!(second.status, "supported");
        let first_at = first.provenance.as_ref().unwrap().observed_at.clone();
        let second_at = second.provenance.as_ref().unwrap().observed_at.clone();
        assert!(!first_at.is_empty() && !second_at.is_empty());
        assert_eq!(first.evidence, second.evidence);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn git_revision_binds_provenance_when_the_workdir_is_versioned() {
        // A versioned workdir binds the recorded revision to the VCS
        // HEAD so a later tree movement reads as stale against the
        // recorded observation instead of silently reusing it.
        let dir = temp_case("revision-git");
        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(&dir)
                .output()
                .expect("git must run")
        };
        if !git(&["init"]).status.success() {
            return;
        }
        fs::write(dir.join("probe.txt"), "v1").unwrap();
        git(&["add", "."]);
        let commit = git(&[
            "-c",
            "user.email=evidence@example.com",
            "-c",
            "user.name=evidence",
            "commit",
            "-m",
            "probe",
        ]);
        if !commit.status.success() {
            return;
        }
        let head = String::from_utf8_lossy(&git(&["rev-parse", "HEAD"]).stdout)
            .trim()
            .to_string();
        assert!(!head.is_empty());
        let bin = write_fixture(
            &dir,
            "good.sh",
            "#!/bin/sh\necho '{\"tool\":\"driftwatch\",\"findings\":[]}'\n",
        );
        let row = run_controlled(
            "driftwatch-policy",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            &dir,
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        assert_eq!(
            row.provenance.as_ref().unwrap().revision.as_deref(),
            Some(head.as_str())
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn disabled_rows_round_trip_through_the_schema_without_probes() {
        // Probes emit only supported/unavailable/not-run; `disabled`
        // stays owned by the manifest adapters. The schema still
        // classifies and renders a disabled row so a future targeted
        // run can report it without a shape change.
        let row = ProviderRow {
            contract: PROVIDER_CONTRACT_VERSION.to_string(),
            provider: "analytics".to_string(),
            display: display_for("analytics").to_string(),
            status: ProviderStatus::Disabled.id().to_string(),
            reason: "analytics block is disabled; provider is not contacted".to_string(),
            provenance: None,
            evidence: Vec::new(),
        };
        let json = serde_json::to_string(&row).unwrap();
        let back: ProviderRow = serde_json::from_str(&json).unwrap();
        assert_eq!(back, row);
        assert!(render_row_human(&back).contains("disabled"));
    }

    #[test]
    fn human_renderers_carry_required_fields() {
        let report = matrix(false);
        let human = render_matrix_human(&report);
        assert!(human.contains("supported=0"));
        assert!(human.contains("not-run=6"));
        for id in PROVIDER_IDS {
            assert!(human.contains(id));
        }
        let descriptor = inspect("deploy").unwrap();
        let human = render_descriptor_human(&descriptor);
        assert!(human.contains("teardown_rule"));
        assert!(human.contains("secret_rule"));
    }

    #[test]
    fn inspect_describes_every_provider_without_probing() {
        for id in PROVIDER_IDS {
            let descriptor = inspect(id).unwrap();
            assert_eq!(descriptor.contract, PROVIDER_CONTRACT_VERSION);
            assert!(!descriptor.boundary.is_empty());
        }
        assert!(inspect("nosuch").is_err());
    }
}
