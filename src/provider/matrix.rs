//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use chrono::Utc;

use super::contract::{
    DEFAULT_PROBE_BINARIES, LIVE_ENV, PROBE_BIN_ENV, PROVIDER_CONTRACT_VERSION, PROVIDER_IDS,
};
use super::model::{ProviderDescriptor, ProviderMatrix, ProviderRow, ProviderStatus};
use super::probes::probe_binary_reachability;

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

pub(super) fn display_for(provider: &str) -> &'static str {
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

pub(super) fn default_binary_for(provider: &str) -> Option<String> {
    // One shared probe order for every DriftWatch runtime surface: the
    // pipe-joined rendering stays byte-identical to the previous table
    // literals (`driftwatchdog|driftwatch`), while the order itself has
    // exactly one definition in `crate::policy`.
    match provider {
        "driftwatch-policy" | "gate-runtime" => {
            Some(crate::policy::DRIFTWATCH_BINARY_CANDIDATES.join("|"))
        }
        _ => DEFAULT_PROBE_BINARIES
            .iter()
            .find(|(id, _)| *id == provider)
            .map(|(_, bin)| bin.to_string()),
    }
}

pub(super) fn binary_env_for(provider: &str) -> Option<&'static str> {
    PROBE_BIN_ENV
        .iter()
        .find(|(id, _)| *id == provider)
        .map(|(_, env)| *env)
}

pub(super) fn not_run_row(provider: &str, live_requested: bool) -> ProviderRow {
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
        default_binary: default_binary_for(&id),
        secret_rule: "secrets reach Forge only through the runner environment; manifests and the repository never carry provider secrets; evidence is redacted through policy::redact_credentials".to_string(),
        teardown_rule: "probes run in disposable temp dirs and teardown removes what the probe created; targeted live runs never write outside the named project's state layout".to_string(),
    })
}
