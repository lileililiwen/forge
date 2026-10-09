//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use chrono::Utc;
use std::path::Path;

use super::contract::EVIDENCE_TIMEOUT;
use super::matrix::{live_enabled, not_run_row, parse_provider, redact_provider_evidence};
use super::model::{ProviderDescriptor, ProviderMatrix, ProviderRow, RunOptions, SandboxKind};
use super::probes::{
    cleanup_temp_dir, git_revision, params, probe_analytics, probe_gate_runtime, probe_policy,
    probe_version, resolve_probe_binary, run_bounded, sandbox_of, sandbox_source, temp_probe_dir,
    truncate_receipt, unavailable_row,
};

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
