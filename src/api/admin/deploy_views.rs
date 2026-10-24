//! The path-free views the browser reads for a deployment.
//!
//! These helpers build the flat, path-free projections of a deployment plan
//! and of a deployment report. They are kept apart from the route handlers
//! so both stay inside the repository's source-size limit and so the
//! redaction discipline reads in one place: nothing that reaches the browser
//! may carry a filesystem path, a host, a user or the adapter binary.

use std::path::Path;

use serde_json::{json, Value};

use super::super::{ApiResponse, API_CONTRACT_VERSION};
use super::gateway::{scrub_json, scrub_text};

use crate::core::manifest::Manifest;
use crate::deploy::{engine, DeployConfig, DeployPlan, DeployReport, DeployRequest};

/// Run the read-only `prepare_deploy` for a resolved target and render its
/// path-free view. Errors are returned already scrubbed of the project
/// directory (plan/preview never reaches the adapter, so only that path can
/// appear).
pub(super) fn run_plan_view(
    project_dir: &Path,
    manifest: &Manifest,
    config: &DeployConfig,
    id: &str,
    target: &str,
) -> Result<Value, ApiResponse> {
    let deploy_request = DeployRequest {
        project_id: id.to_string(),
        target: target.to_string(),
        confirm: false,
        dry_run: true,
    };
    let plan = match engine::prepare_deploy(project_dir, manifest, config, &deploy_request) {
        Ok(plan) => plan,
        Err(err) => return Err(typed_deploy_error(project_dir, &err)),
    };
    let secrets = [project_dir.display().to_string()];
    Ok(scrub_json(plan_view(&plan), &secrets))
}

/// Flat, path-free plan view built from typed fields — never a serialized
/// `DeployPlan`. Host, user, target path and the adapter binary are omitted;
/// only the target name/kind, source revision, artifact hash/size, health kind,
/// readiness and the delivery rule's verdict reach the browser, so the generic
/// frontend renderer prints scalar lines with no `[object Object]` and no
/// filesystem location.
fn plan_view(plan: &DeployPlan) -> Value {
    let mut view = json!({
        "action": "deploy",
        "project_id": plan.project_id,
        "target": plan.target.name,
        "target_kind": plan.target.kind,
        "source_revision": plan.identity.source_revision,
        "ready": plan.ready,
        "ship_gate": plan.ship_gate.satisfied,
        "ship_gate_reason": plan.ship_gate.reason,
    });
    if let Some(artifact) = &plan.artifact {
        view["artifact_hash"] = json!(artifact.content_hash);
        view["artifact_bytes"] = json!(artifact.byte_size);
    }
    if let Some(health) = &plan.health {
        view["health_kind"] = json!(health.kind);
    }
    view
}

/// Flat, path-free apply report view built from typed fields — never a
/// serialized `DeployReport`. `state_path`, the target host/user/path and the
/// adapter binary are all omitted; stage/observation `note`s are the engine's
/// already-redacted text. The caller scrubs the whole value as a final guard.
pub(super) fn report_view(report: &DeployReport) -> Value {
    let stages: Vec<Value> = report
        .stages
        .iter()
        .map(|stage| {
            json!({
                "stage": stage.stage,
                "target": stage.target,
                "status": stage.status,
                "note": stage.note,
            })
        })
        .collect();
    let mut view = json!({
        "contract": report.contract,
        "project_id": report.project_id,
        "target": report.target.name,
        "target_kind": report.target.kind,
        "adapter": report.adapter,
        "deploy_id": report.identity.id,
        "source_revision": report.identity.source_revision,
        "dry_run": report.dry_run,
        "verdict": report.verdict.label(),
        "healthy": report.healthy,
        "note": report.note,
        "stages": stages,
    });
    if let Some(observation) = &report.observation {
        view["observation"] = json!({
            "status": observation.status,
            "detail": observation.detail,
        });
    }
    if let Some(artifact) = &report.artifact {
        view["artifact_hash"] = json!(artifact.content_hash);
        view["artifact_bytes"] = json!(artifact.byte_size);
    }
    view
}

/// Render a deploy-domain Core failure with the shared status mapping, scrubbing
/// the project directory (the only absolute path plan/preview can leak) from the
/// message. The typed code and status are preserved so a boundary scenario is
/// reported honestly, never as a fake success.
pub(super) fn typed_deploy_error(project_dir: &Path, err: &crate::core::ForgeError) -> ApiResponse {
    let secrets = [project_dir.display().to_string()];
    let status = super::super::err_status(err);
    ApiResponse::json(
        status,
        json!({
            "error": {
                "code": err.code(),
                "message": scrub_text(&err.to_string(), &secrets),
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy::{
        DeployIdentity, DeployStageOutcome, DeployTargetSpec, DeployVerdict, HealthObservation,
    };

    /// A report with the fields the view reads.
    fn report(verdict: DeployVerdict, healthy: bool) -> DeployReport {
        DeployReport {
            contract: "forge-deploy/0.1.0".to_string(),
            project_id: "p1".to_string(),
            identity: DeployIdentity {
                id: "p1-home-abc".to_string(),
                project_id: "p1".to_string(),
                target: "home".to_string(),
                source_revision: "cafe1234".to_string(),
            },
            target: DeployTargetSpec {
                name: "home".to_string(),
                kind: "local".to_string(),
                host: None,
                user: None,
                path: None,
                service: None,
                note: None,
            },
            adapter: "local".to_string(),
            artifact: None,
            health: None,
            stages: vec![DeployStageOutcome {
                stage: "apply".to_string(),
                target: "home".to_string(),
                status: "delivered".to_string(),
                note: "rehearsed".to_string(),
                evidence: Vec::new(),
                recovery: Vec::new(),
            }],
            observation: Some(HealthObservation {
                status: "running".to_string(),
                detail: "up".to_string(),
                observed_at: "2026-10-24T00:00:00Z".to_string(),
                evidence: Vec::new(),
            }),
            dry_run: matches!(verdict, DeployVerdict::Planned),
            state_path: "/tmp/p1/state.json".to_string(),
            note: "dry-run: 2 stage(s) rehearsed; nothing was deployed".to_string(),
            healthy,
            verdict,
        }
    }

    #[test]
    fn the_report_view_carries_the_verdict() {
        // The browser reads the same rule as the CLI: a rehearsal is a
        // plan, and the view says so next to its dry-run flag.
        let view = report_view(&report(DeployVerdict::Planned, false));
        assert_eq!(view["verdict"], "planned");
        assert_eq!(view["healthy"], false);
        assert_eq!(view["dry_run"], true);
    }

    #[test]
    fn the_report_view_carries_a_delivery() {
        let view = report_view(&report(DeployVerdict::Delivered, true));
        assert_eq!(view["verdict"], "delivered");
        assert_eq!(view["healthy"], true);
        assert_eq!(view["dry_run"], false);
    }
}
