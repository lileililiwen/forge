//! Read-only delivery status projection.
//!
//! Pure function over the existing `operations` table: walks the
//! per-verb journal rows, computes the current `DeliveryPhase`,
//! and assembles the evidence / recovery lists the CLI, API and UI
//! surfaces serialise.
//!
//! No filesystem side effects, no provider invocation, no journal
//! writes. The function is the single door every transport enters
//! when answering `forge delivery status <project>` or
//! `GET /v1/projects/{id}/delivery`.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::ForgeError;
use crate::registry::{OperationEntry, Registry};

use super::state::{scrub_detail, DeliveryPhase};
use super::DeliveryEnvironment;
use super::DELIVERY_CONTRACT_VERSION;

/// Maximum number of evidence / recovery strings rendered into the
/// projection. A verbose provider is bounded so the response stays
/// single-page.
pub const EVIDENCE_LIMIT: usize = 16;
pub const RECOVERY_LIMIT: usize = 8;

/// One bounded evidence or recovery line carried on the projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryEvidence {
    pub started_at: String,
    pub op_id: i64,
    pub source: String,
    pub text: String,
}

/// One delivery verb's most recent terminal row, plus the
/// additional per-verb evidence used by the next-confirmation
/// guards.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DeliveryVerbReport {
    pub op_id: Option<i64>,
    pub started_at: Option<String>,
    pub state: Option<String>,
    pub revision: Option<String>,
    pub build_status: Option<String>,
    pub run_status: Option<String>,
    pub container_identity: Option<String>,
    pub evidence: Vec<DeliveryEvidence>,
    pub recovery: Vec<DeliveryEvidence>,
    /// The op_id the next verb should pass back as `--confirm-operation-id`.
    pub confirm_operation_id: Option<i64>,
}

/// Hermora child operation summary. `site_id` / `environment_url`
/// are populated only when the adapter answered `connected`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DeliveryHermoraReport {
    pub op_id: Option<i64>,
    pub started_at: Option<String>,
    pub state: Option<String>,
    pub site_id: Option<String>,
    pub environment_url: Option<String>,
    pub reason: Option<String>,
}

/// The full delivery projection for one `(project, revision)` pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryReport {
    pub contract: String,
    pub project: String,
    pub phase: DeliveryPhase,
    pub environment: Option<DeliveryEnvironment>,
    pub revision: Option<String>,
    pub updated_at: String,
    pub preflight: DeliveryVerbReport,
    pub stage: DeliveryVerbReport,
    pub promote: DeliveryVerbReport,
    pub hermora: DeliveryHermoraReport,
}

/// Build the projection by reading the per-verb journal rows the
/// workflow has accumulated for this project. The result is a pure
/// snapshot: every render of the same registry at the same instant
/// produces the same bytes.
pub fn build_delivery_report(
    registry: &Registry,
    project_id: &str,
    now: DateTime<Utc>,
) -> Result<DeliveryReport, ForgeError> {
    let record = registry.inspect(project_id)?;
    let mut rows = registry
        .operations_for_project(project_id, 200)?
        .into_iter()
        .filter(|row| {
            matches!(
                row.kind.as_str(),
                "delivery.preflight" | "delivery.stage" | "delivery.promote" | "delivery.hermora"
            )
        })
        .collect::<Vec<_>>();

    // `operations_for_project` returns rows in `op_id DESC` order;
    // sort by `op_id ASC` so per-verb "most recent" picks up the
    // most recent terminal row last, and `find_latest` walks oldest
    // first. Deterministic.
    rows.sort_by_key(|row| row.op_id);

    let mut preflight = DeliveryVerbReport::default();
    let mut stage = DeliveryVerbReport::default();
    let mut promote = DeliveryVerbReport::default();
    let mut hermora = DeliveryHermoraReport::default();
    let mut last_updated: Option<String> = None;
    let mut last_revision: Option<String> = None;

    for row in &rows {
        let updated = row
            .finished_at
            .clone()
            .unwrap_or_else(|| row.started_at.clone());
        last_updated = Some(updated.clone());
        if let Some(rev) = row.revision.as_ref() {
            last_revision = Some(rev.clone());
        }
        let (evidence, recovery) = parse_evidence_and_recovery(row);
        match row.kind.as_str() {
            "delivery.preflight" => populate_verb(&mut preflight, row, evidence, recovery),
            "delivery.stage" => populate_verb(&mut stage, row, evidence, recovery),
            "delivery.promote" => populate_verb(&mut promote, row, evidence, recovery),
            "delivery.hermora" => populate_hermora(&mut hermora, row),
            _ => {}
        }
    }

    // Bounded evidence / recovery lists.
    bounded_truncate(&mut preflight.evidence, EVIDENCE_LIMIT);
    bounded_truncate(&mut preflight.recovery, RECOVERY_LIMIT);
    bounded_truncate(&mut stage.evidence, EVIDENCE_LIMIT);
    bounded_truncate(&mut stage.recovery, RECOVERY_LIMIT);
    bounded_truncate(&mut promote.evidence, EVIDENCE_LIMIT);
    bounded_truncate(&mut promote.recovery, RECOVERY_LIMIT);

    let phase = compute_phase(&preflight, &stage, &promote, &hermora);
    let environment = environment_for_phase(phase);

    Ok(DeliveryReport {
        contract: DELIVERY_CONTRACT_VERSION.to_string(),
        project: project_id.to_string(),
        phase,
        environment,
        // Prefer the journal row's recorded revision; fall back
        // to the project's registered `last_commit` so the
        // projection carries a revision even before the first
        // delivery verb runs.
        revision: last_revision.or(record.last_commit.clone()),
        updated_at: last_updated.unwrap_or_else(|| now.to_rfc3339()),
        preflight,
        stage,
        promote,
        hermora,
    })
}

fn populate_verb(
    report: &mut DeliveryVerbReport,
    row: &OperationEntry,
    evidence: Vec<String>,
    recovery: Vec<String>,
) {
    report.op_id = Some(row.op_id);
    report.started_at = Some(row.started_at.clone());
    report.state = Some(row.state.clone());
    if let Some(rev) = row.revision.as_ref() {
        report.revision = Some(rev.clone());
    }
    if let Some(build) = row.build_status.as_ref() {
        report.build_status = Some(build.clone());
    }
    if let Some(run) = row.run_status.as_ref() {
        report.run_status = Some(run.clone());
    }
    if let Some(container) = row.container_identity.as_ref() {
        report.container_identity = Some(container.clone());
    }
    report
        .evidence
        .extend(evidence.into_iter().map(|text| DeliveryEvidence {
            started_at: row.started_at.clone(),
            op_id: row.op_id,
            source: row.kind.clone(),
            text: scrub_detail(&text),
        }));
    report
        .recovery
        .extend(recovery.into_iter().map(|text| DeliveryEvidence {
            started_at: row.started_at.clone(),
            op_id: row.op_id,
            source: row.kind.clone(),
            text: scrub_detail(&text),
        }));
    report.confirm_operation_id = Some(row.op_id);
}

fn populate_hermora(report: &mut DeliveryHermoraReport, row: &OperationEntry) {
    report.op_id = Some(row.op_id);
    report.started_at = Some(row.started_at.clone());
    report.state = Some(row.state.clone());
    let detail = row.detail.clone().unwrap_or_default();
    let parsed: BTreeMap<String, String> = serde_json::from_str(&detail).unwrap_or_default();
    report.site_id = parsed.get("site_id").cloned();
    report.environment_url = parsed.get("environment_url").cloned();
    report.reason = parsed.get("reason").cloned();
}

fn parse_evidence_and_recovery(row: &OperationEntry) -> (Vec<String>, Vec<String>) {
    let detail = row.detail.clone().unwrap_or_default();
    let parsed: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(&detail).unwrap_or_default();
    let evidence = parsed
        .get("evidence")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let recovery = parsed
        .get("recovery")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    (evidence, recovery)
}

fn bounded_truncate<T>(items: &mut Vec<T>, max: usize) {
    if items.len() > max {
        items.truncate(max);
    }
}

/// Compute the closed `DeliveryPhase` from the per-verb terminal
/// rows. The order of evaluation is the order the spec documents:
///
///  1. If the latest terminal verb is a `promote` with `degraded`
///     health, the phase is `Degraded`.
///  2. If the latest terminal verb is a `promote` with healthy
///     build/run, the phase is `Healthy`.
///  3. If the latest terminal verb is a `stage` with failed health,
///     the phase is `StageFailed`.
///  4. If the latest terminal verb is a `stage` with healthy health,
///     the phase is `StageHealthy` and promote is the next verb.
///  5. If the latest terminal verb is a `preflight` with healthy
///     status, the phase is `Preflighted`.
///  6. Otherwise the phase is `Draft`.
fn compute_phase(
    preflight: &DeliveryVerbReport,
    stage: &DeliveryVerbReport,
    promote: &DeliveryVerbReport,
    hermora: &DeliveryHermoraReport,
) -> DeliveryPhase {
    if let Some(state) = hermora.state.as_deref() {
        if state == "done" && hermora.site_id.is_some() {
            return DeliveryPhase::HermoraConnected;
        }
        if state == "failed" {
            return DeliveryPhase::HermoraFailed;
        }
    }
    if promote.op_id.is_some() {
        let degraded = promote
            .build_status
            .as_deref()
            .map(|s| s != "succeeded" && s != "not_started" && s != "unknown")
            .unwrap_or(false)
            || promote
                .run_status
                .as_deref()
                .map(|s| s != "succeeded" && s != "not_started" && s != "unknown")
                .unwrap_or(false)
            || promote.state.as_deref() == Some("failed");
        if degraded {
            return DeliveryPhase::Degraded;
        }
        return DeliveryPhase::Healthy;
    }
    if stage.op_id.is_some() {
        let failed = stage.state.as_deref() == Some("failed")
            || stage.build_status.as_deref() == Some("failed")
            || stage.run_status.as_deref() == Some("failed");
        if failed {
            return DeliveryPhase::StageFailed;
        }
        return DeliveryPhase::AwaitingProductionApproval;
    }
    if preflight.op_id.is_some() {
        let failed = preflight.state.as_deref() == Some("failed");
        if failed {
            return DeliveryPhase::StageFailed;
        }
        return DeliveryPhase::AwaitingStageConfirmation;
    }
    DeliveryPhase::Draft
}

fn environment_for_phase(phase: DeliveryPhase) -> Option<DeliveryEnvironment> {
    match phase {
        DeliveryPhase::Staging
        | DeliveryPhase::StageHealthy
        | DeliveryPhase::StageFailed
        | DeliveryPhase::AwaitingStageConfirmation
        | DeliveryPhase::AwaitingProductionApproval => Some(DeliveryEnvironment::Stage),
        DeliveryPhase::Production
        | DeliveryPhase::Healthy
        | DeliveryPhase::Degraded
        | DeliveryPhase::HermoraPending
        | DeliveryPhase::HermoraConnected
        | DeliveryPhase::HermoraFailed => Some(DeliveryEnvironment::Production),
        DeliveryPhase::Draft | DeliveryPhase::Preflighted => None,
    }
}

/// Render the human table for one delivery report. Bounded so the
/// output stays single-page.
pub fn render_report_human(report: &DeliveryReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("contract: {}", report.contract));
    lines.push(format!("project: {}", report.project));
    lines.push(format!("phase: {}", report.phase));
    if let Some(env) = report.environment {
        lines.push(format!("environment: {env}"));
    }
    if let Some(rev) = report.revision.as_deref() {
        lines.push(format!("revision: {rev}"));
    }
    lines.push(format!("updated_at: {}", report.updated_at));
    lines.push(String::new());

    let verb_labels = [
        ("preflight", &report.preflight),
        ("stage", &report.stage),
        ("promote", &report.promote),
    ];
    for (label, verb) in verb_labels {
        if verb.op_id.is_none() {
            lines.push(format!("{label}: (not yet recorded)"));
            continue;
        }
        lines.push(format!(
            "{label}: op_id={} state={}",
            verb.op_id.unwrap_or_default(),
            verb.state.clone().unwrap_or_else(|| "—".to_string())
        ));
        if let Some(rev) = verb.revision.as_deref() {
            lines.push(format!("  revision: {rev}"));
        }
        if let Some(build) = verb.build_status.as_deref() {
            lines.push(format!("  build_status: {build}"));
        }
        if let Some(run) = verb.run_status.as_deref() {
            lines.push(format!("  run_status: {run}"));
        }
        if let Some(container) = verb.container_identity.as_deref() {
            lines.push(format!("  container_identity: {container}"));
        }
        if !verb.evidence.is_empty() {
            lines.push(format!("  evidence ({}):", verb.evidence.len()));
            for line in &verb.evidence {
                lines.push(format!("    - {line}", line = line.text));
            }
        }
        if !verb.recovery.is_empty() {
            lines.push(format!("  recovery ({}):", verb.recovery.len()));
            for line in &verb.recovery {
                lines.push(format!("    - {line}", line = line.text));
            }
        }
    }

    if report.hermora.op_id.is_none() {
        lines.push("hermora: (not configured)".to_string());
    } else {
        lines.push(format!(
            "hermora: op_id={} state={}",
            report.hermora.op_id.unwrap_or_default(),
            report
                .hermora
                .state
                .clone()
                .unwrap_or_else(|| "—".to_string())
        ));
        if let Some(site) = report.hermora.site_id.as_deref() {
            lines.push(format!("  site_id: {site}"));
        }
        if let Some(url) = report.hermora.environment_url.as_deref() {
            lines.push(format!("  environment_url: {url}"));
        }
        if let Some(reason) = report.hermora.reason.as_deref() {
            lines.push(format!("  reason: {reason}"));
        }
    }
    lines.join("\n")
}

/// Build a stable JSON value the CLI, API, and UI all serialise
/// from. Uses the same `DeliveryReport` struct so the JSON shape is
/// the same on every transport.
pub fn render_report_json(report: &DeliveryReport) -> serde_json::Value {
    serde_json::to_value(report).unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Registry;

    fn temp_registry() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("registry.db");
        (dir, path)
    }

    fn seed_project(path: &std::path::Path, id: &str) {
        let _registry = Registry::open(path).expect("open");
        let conn = rusqlite::Connection::open(path).expect("raw");
        conn.execute(
            "INSERT INTO projects
                (id, name, path, profile, maturity, schema_version, platform_version,
                 features, observed_at)
             VALUES (?1, ?1, ?2, 'rust-web', 'L2', 1, '0.1.0', '{}', ?3)",
            rusqlite::params![id, format!("/tmp/{id}"), "2026-09-29T00:00:00Z"],
        )
        .expect("seed");
    }

    #[test]
    fn empty_registry_yields_draft_phase() {
        let (_dir, path) = temp_registry();
        seed_project(&path, "alpha");
        let registry = Registry::open(&path).expect("open");
        let report = build_delivery_report(&registry, "alpha", Utc::now()).expect("report");
        assert_eq!(report.phase, DeliveryPhase::Draft);
        assert!(report.revision.is_none());
        assert_eq!(report.environment, None);
        assert_eq!(report.contract, DELIVERY_CONTRACT_VERSION);
    }

    #[test]
    fn unknown_project_yields_typed_error() {
        let (_dir, path) = temp_registry();
        let registry = Registry::open(&path).expect("open");
        let err = build_delivery_report(&registry, "missing", Utc::now()).unwrap_err();
        assert_eq!(err.code(), "unknown-project");
    }

    #[test]
    fn a_terminal_preflight_advances_to_awaiting_stage_confirmation() {
        let (_dir, path) = temp_registry();
        seed_project(&path, "alpha");
        let registry = Registry::open(&path).expect("open");
        registry
            .reserve_idempotent_operation(
                "delivery.preflight",
                "alpha",
                "delivery.preflight.alpha:0123456789ab",
                "abc",
            )
            .expect("reserve");
        registry
            .finalize_operation(
                1,
                "done",
                r#"{"evidence":["provider preflight: ok"],"recovery":[]}"#,
            )
            .expect("finalize");
        registry
            .update_operation_phase(
                1,
                Some("0123456789abcdef0123456789abcdef01234567"),
                None,
                None,
                None,
            )
            .ok();
        let report = build_delivery_report(&registry, "alpha", Utc::now()).expect("report");
        assert_eq!(report.phase, DeliveryPhase::AwaitingStageConfirmation);
        assert_eq!(report.preflight.op_id, Some(1));
        assert_eq!(report.preflight.state.as_deref(), Some("done"));
    }
}
