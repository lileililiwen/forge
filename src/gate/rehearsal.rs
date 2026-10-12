//! The gate runtime's machine-readable **plan rehearsal** document.
//!
//! `driftwatch gate --dry-run` resolves the plan and stops. Since the
//! sibling's `gate-dry-run-json-plan` change (`02a13d4`, `447d1b0`) it can
//! also answer `--format json`, and that answer is a **third document
//! kind**, not a variant of the status document: composed from the
//! resolved plan and the manifest alone, with no clock, no child process,
//! no database and no file. It carries two invariants this module exists to
//! preserve:
//!
//! 1. **No verdict.** There is no `status`, `result`, `passed`, `blocked`
//!    or `outcome` key anywhere in it — not at the root, not nested. Its
//!    `blocking_prediction.would_block` is `true` or `null`, **never**
//!    `false`: `false` would be a claim that nothing ran and passed.
//! 2. **Side-effect free.** Nothing was executed, nothing was persisted,
//!    no `gate_runs` row exists.
//!
//! Forge therefore parses it into its own typed structure with **no
//! aggregate, no check state and no evidence record anywhere in the
//! type**: a rehearsal has nowhere to put an outcome, so it cannot become
//! one by accident. `GateOutcome::Rehearsal` carries it and nothing else.
//!
//! ## Detection is by shape, never by version
//!
//! The sibling's `--version` string did not change when the document
//! shape changed: the stale Oct 5 copy at `~/.cargo/bin/driftwatchdog`
//! (human text) and the current build (this document) both report
//! `driftwatch 0.1.0`. See `design.md` §3 for the full rule; the rule is
//! three independent positive self-declarations plus a complete typed
//! parse plus the producer's own side-effect invariants. Anything that
//! fails is *not a rehearsal Forge understands*, and degrades to the
//! bounded plan-line preview — which is never a verdict either.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::contract::MAX_PLAN_LINES;
use super::engine::capture_note;

#[cfg(test)]
#[path = "rehearsal_tests.rs"]
mod tests;

/// Contract **family** of the rehearsal document. The patch version is
/// deliberately not compared: an additive sibling release still parses,
/// and an unrecognised family is refused.
pub const REHEARSAL_CONTRACT_FAMILY: &str = "driftwatch-gate-plan/";

/// The only `document_kind` a rehearsal carries. Present so a consumer
/// that has learned to recognise *one* rehearsal document does not have
/// to infer it from the absence of fields.
pub const REHEARSAL_DOCUMENT_KIND: &str = "gate-plan-rehearsal";

/// Keys that state the **outcome of execution** and must therefore never
/// appear in anything Forge renders from a rehearsal, at any depth.
///
/// This mirrors the sibling's own closed deny-set rather than importing
/// it: the two repositories cannot share code, and a consumer that forgot
/// the rule would be the failure this exists to prevent. `would_block` is
/// intentionally absent — it is a prediction, and the producer emits it
/// as `true` or `null`, never `false`.
pub const REHEARSAL_VERDICT_DENY_KEYS: &[&str] = &[
    "status",
    "result",
    "results",
    "passed",
    "blocked",
    "outcome",
    "aggregate",
    "exit_code",
    "gate_status",
    "ok",
    "success",
    "failures",
    "pending_reviews",
];

/// Every key in `value`, at every depth, that appears in
/// [`REHEARSAL_VERDICT_DENY_KEYS`]. Empty means the payload cannot be read
/// as a verdict.
pub fn verdict_like_keys(value: &serde_json::Value) -> Vec<String> {
    let mut found = Vec::new();
    collect_verdict_like_keys(value, String::new(), &mut found);
    found
}

fn collect_verdict_like_keys(value: &serde_json::Value, path: String, found: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                if REHEARSAL_VERDICT_DENY_KEYS.contains(&key.as_str()) {
                    found.push(child_path.clone());
                }
                collect_verdict_like_keys(child, child_path, found);
            }
        }
        serde_json::Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_verdict_like_keys(child, format!("{path}[{index}]"), found);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Wire shapes
// ---------------------------------------------------------------------------

/// The rehearsal document exactly as the sibling emits it.
///
/// Unknown fields are ignored on purpose (additive sibling releases parse),
/// while every field Forge renders is required to have the expected JSON
/// type: a partial parse is refused as a whole so Forge never renders a
/// rehearsal it only half understands.
#[derive(Debug, Clone, Deserialize)]
struct WireRehearsal {
    contract: String,
    document_kind: String,
    dry_run: bool,
    executed: bool,
    persisted: bool,
    #[serde(default)]
    execution_mode: String,
    #[serde(default)]
    plan_version: u64,
    #[serde(default)]
    profile: String,
    #[serde(default)]
    rule_pack_version: String,
    #[serde(default)]
    manifest_digest: String,
    #[serde(default)]
    default_timeout_ms: u64,
    #[serde(default)]
    blocking_policy: WirePolicy,
    #[serde(default)]
    source_size_policy: WireSourceSize,
    planned_checks: Vec<WirePlannedCheck>,
    #[serde(default)]
    unbound_required_checks: Vec<String>,
    #[serde(default)]
    exemption_budget: WireExemptionBudget,
    #[serde(default)]
    not_scheduled: Vec<String>,
    blocking_prediction: WirePrediction,
    #[serde(default)]
    notice: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct WirePolicy {
    #[serde(default)]
    review_required_blocks: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct WireSourceSize {
    #[serde(default)]
    max_lines: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct WirePlannedCheck {
    id: String,
    required: bool,
    #[serde(default)]
    exempt: bool,
    #[serde(default)]
    selected_by: String,
    #[serde(default)]
    command_source: String,
    #[serde(default)]
    timeout_ms: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct WireExemptionBudget {
    #[serde(default)]
    cap: u64,
    #[serde(default)]
    declared: u64,
    #[serde(default)]
    headroom: u64,
    #[serde(default)]
    required_checks: u64,
    #[serde(default)]
    verdict: String,
}

#[derive(Debug, Clone, Deserialize)]
struct WirePrediction {
    kind: String,
    #[serde(default)]
    derived_from: String,
    /// `true`, `null` or absent — never `false`; see [`parse_rehearsal`].
    #[serde(default)]
    would_block: Option<bool>,
    #[serde(default)]
    sufficient_but_not_necessary: bool,
    #[serde(default)]
    reasons: Vec<WireReason>,
    #[serde(default)]
    note: String,
}

#[derive(Debug, Clone, Deserialize)]
struct WireReason {
    #[serde(default)]
    rule: String,
    #[serde(default)]
    checks: Vec<String>,
}

// ---------------------------------------------------------------------------
// Display shapes
// ---------------------------------------------------------------------------

/// One planned check as the rehearsal describes it: what would run, where
/// its command would come from, and its effective deadline. There is no
/// status field because nothing ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RehearsalPlannedCheck {
    pub id: String,
    pub required: bool,
    pub exempt: bool,
    /// The manifest layer that scheduled the check.
    pub selected_by: String,
    /// Where the command would come from (`manifest`, `in-process`,
    /// `none`). An unbound word is rendered verbatim, never interpreted.
    pub command_source: String,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RehearsalBlockingReason {
    pub rule: String,
    pub checks: Vec<String>,
}

/// A plan-level **prediction**, never an observed verdict.
///
/// `would_block` is `Some(true)` ("the plan alone forces a block") or
/// `None` ("the bound commands decide"). It can never be `Some(false)`:
/// a rehearsal that claimed that would be refused as unrecognised, so a
/// consumer can never read this document as a passing rehearsal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RehearsalBlockingPrediction {
    pub kind: String,
    pub derived_from: String,
    pub would_block: Option<bool>,
    pub sufficient_but_not_necessary: bool,
    pub reasons: Vec<RehearsalBlockingReason>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RehearsalExemptionBudget {
    pub cap: u64,
    pub declared: u64,
    pub headroom: u64,
    pub required_checks: u64,
    /// Budget accounting (`within`, `exhausted`, `over-cap`,
    /// `hard-fail`) — never an execution outcome.
    pub verdict: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RehearsalBlockingPolicy {
    pub review_required_blocks: Option<bool>,
}

/// The parsed, redacted and character-bounded rehearsal.
///
/// The absence of an aggregate, a check state and a revision binding in
/// this type is the enforcement of "a rehearsal is never evidence": there
/// is no field to write one into, so no later call site can invent one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateRehearsal {
    pub contract: String,
    pub document_kind: String,
    pub execution_mode: String,
    pub plan_version: u64,
    pub profile: String,
    pub rule_pack_version: String,
    pub manifest_digest: String,
    pub default_timeout_ms: u64,
    pub source_size_max_lines: Option<u64>,
    pub blocking_policy: RehearsalBlockingPolicy,
    pub planned_checks: Vec<RehearsalPlannedCheck>,
    pub unbound_required_checks: Vec<String>,
    pub exemption_budget: RehearsalExemptionBudget,
    pub not_scheduled: Vec<String>,
    pub blocking_prediction: RehearsalBlockingPrediction,
    pub notice: String,
}

impl GateRehearsal {
    /// The plan-forced-block statement in the wording a reader must not
    /// be able to misread: `true` names the block, `None` names the
    /// unknown, and neither ever names a pass.
    pub fn would_block_label(&self) -> &'static str {
        if self.blocking_prediction.would_block == Some(true) {
            "block"
        } else {
            "unknown"
        }
    }

    pub fn required_check_count(&self) -> usize {
        self.planned_checks.iter().filter(|c| c.required).count()
    }
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Parse `value` as a rehearsal document Forge understands.
///
/// `None` means "not a rehearsal I can read", never "not a rehearsal":
/// the caller degrades to the bounded plan-line preview, which is also
/// never a verdict. See `design.md` §3b for the rule as a table.
pub fn parse_rehearsal(value: &serde_json::Value, dir: &Path) -> Option<GateRehearsal> {
    if !declares_rehearsal(value) {
        return None;
    }
    let wire: WireRehearsal = serde_json::from_value(value.clone()).ok()?;
    // The producer's own side-effect markers, re-checked on the parsed
    // document rather than trusted from the shape test: a document that
    // claims `dry_run: true` while reporting `executed: true` is a
    // contradiction, and Forge refuses to interpret it at all.
    if wire.executed || wire.persisted {
        return None;
    }
    if wire.blocking_prediction.would_block == Some(false) {
        return None;
    }
    let clean = |text: &str| capture_note(text, dir);
    Some(GateRehearsal {
        contract: clean(&wire.contract),
        document_kind: clean(&wire.document_kind),
        execution_mode: clean(&wire.execution_mode),
        plan_version: wire.plan_version,
        profile: clean(&wire.profile),
        rule_pack_version: clean(&wire.rule_pack_version),
        manifest_digest: clean(&wire.manifest_digest),
        default_timeout_ms: wire.default_timeout_ms,
        source_size_max_lines: wire.source_size_policy.max_lines,
        blocking_policy: RehearsalBlockingPolicy {
            review_required_blocks: wire.blocking_policy.review_required_blocks,
        },
        planned_checks: wire
            .planned_checks
            .iter()
            .map(|check| RehearsalPlannedCheck {
                id: clean(&check.id),
                required: check.required,
                exempt: check.exempt,
                selected_by: clean(&check.selected_by),
                command_source: clean(&check.command_source),
                timeout_ms: check.timeout_ms,
            })
            .collect(),
        unbound_required_checks: wire
            .unbound_required_checks
            .iter()
            .map(|id| clean(id))
            .collect(),
        exemption_budget: RehearsalExemptionBudget {
            cap: wire.exemption_budget.cap,
            declared: wire.exemption_budget.declared,
            headroom: wire.exemption_budget.headroom,
            required_checks: wire.exemption_budget.required_checks,
            verdict: clean(&wire.exemption_budget.verdict),
        },
        not_scheduled: wire.not_scheduled.iter().map(|id| clean(id)).collect(),
        blocking_prediction: RehearsalBlockingPrediction {
            kind: clean(&wire.blocking_prediction.kind),
            derived_from: clean(&wire.blocking_prediction.derived_from),
            would_block: wire.blocking_prediction.would_block,
            sufficient_but_not_necessary: wire.blocking_prediction.sufficient_but_not_necessary,
            reasons: wire
                .blocking_prediction
                .reasons
                .iter()
                .map(|reason| RehearsalBlockingReason {
                    rule: clean(&reason.rule),
                    checks: reason.checks.iter().map(|id| clean(id)).collect(),
                })
                .collect(),
            note: clean(&wire.blocking_prediction.note),
        },
        notice: clean(&wire.notice),
    })
}

/// The three independent self-declarations, and nothing else.
///
/// Never key off the presence of a plausible field alone: that would
/// match any unrelated JSON document. These three keys exist only in a
/// rehearsal, and the sibling's status document carries none of them
/// (it has no `contract` field at all).
fn declares_rehearsal(value: &serde_json::Value) -> bool {
    value
        .get("contract")
        .and_then(|c| c.as_str())
        .is_some_and(|contract| contract.starts_with(REHEARSAL_CONTRACT_FAMILY))
        && value
            .get("document_kind")
            .and_then(|k| k.as_str())
            .is_some_and(|kind| kind == REHEARSAL_DOCUMENT_KIND)
        && value.get("dry_run").and_then(|d| d.as_bool()) == Some(true)
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// One rendered rehearsal line, used by both the human surface and the
/// JSON envelope's `plan` list so the two never disagree.
pub fn rehearsal_plan_lines(rehearsal: &GateRehearsal) -> Vec<String> {
    let mut lines = vec![
        format!(
            "rehearsal: {} ({}) | execution mode: {} | nothing was executed, persisted or journaled",
            rehearsal.contract, rehearsal.document_kind, rehearsal.execution_mode
        ),
        format!(
            "plan: version {} | profile {} | rule pack {} | manifest {}",
            rehearsal.plan_version,
            word_or_unknown(&rehearsal.profile),
            word_or_unknown(&rehearsal.rule_pack_version),
            word_or_unknown(&rehearsal.manifest_digest)
        ),
        format!(
            "blocking policy: review_required_blocks={} | default timeout: {}ms | source size max_lines: {}",
            rehearsal
                .blocking_policy
                .review_required_blocks
                .map(|blocks| blocks.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            rehearsal.default_timeout_ms,
            rehearsal
                .source_size_max_lines
                .map(|max| max.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ),
        format!("checks ({} planned):", rehearsal.planned_checks.len()),
    ];
    for check in rehearsal.planned_checks.iter().take(MAX_PLAN_LINES) {
        let mut row = format!(
            "  - {} [{}] command={} selected-by={} timeout={}ms",
            check.id,
            if check.required {
                "required"
            } else {
                "optional"
            },
            word_or_unknown(&check.command_source),
            word_or_unknown(&check.selected_by),
            check.timeout_ms
        );
        if check.exempt {
            row.push_str(" (exempt)");
        }
        lines.push(row);
    }
    if rehearsal.planned_checks.len() > MAX_PLAN_LINES {
        lines.push(format!(
            "  … {} more planned check(s) not shown",
            rehearsal.planned_checks.len() - MAX_PLAN_LINES
        ));
    }
    lines.push(format!(
        "exemption budget: cap={} declared={} headroom={} required={} verdict={}",
        rehearsal.exemption_budget.cap,
        rehearsal.exemption_budget.declared,
        rehearsal.exemption_budget.headroom,
        rehearsal.exemption_budget.required_checks,
        word_or_unknown(&rehearsal.exemption_budget.verdict)
    ));
    lines.push(format!(
        "unbound required checks: {}",
        if rehearsal.unbound_required_checks.is_empty() {
            "none".to_string()
        } else {
            rehearsal.unbound_required_checks.join(", ")
        }
    ));
    if !rehearsal.not_scheduled.is_empty() {
        lines.push(format!(
            "not scheduled: {}",
            rehearsal.not_scheduled.join(", ")
        ));
    }
    let prediction = &rehearsal.blocking_prediction;
    lines.push(format!(
        "blocking prediction: {} — would_block={} ({}){}",
        prediction.kind,
        rehearsal.would_block_label(),
        if prediction.would_block == Some(true) {
            "the plan alone forces a block"
        } else {
            "the bound commands decide; unknown is never a pass"
        },
        if prediction.sufficient_but_not_necessary {
            ", a plan-level prediction from the resolved plan alone, never an observed outcome"
        } else {
            ""
        }
    ));
    for reason in &prediction.reasons {
        lines.push(format!(
            "  prediction reason: rule={} checks={}",
            word_or_unknown(&reason.rule),
            if reason.checks.is_empty() {
                "none".to_string()
            } else {
                reason.checks.join(", ")
            }
        ));
    }
    if !prediction.note.is_empty() {
        lines.push(format!("prediction note: {}", prediction.note));
    }
    if !rehearsal.notice.is_empty() {
        lines.push(format!("notice: {}", rehearsal.notice));
    }
    lines
}

/// Human rendering of a rehearsal: the resolved plan, never an outcome.
pub fn render_rehearsal_human(rehearsal: &GateRehearsal) -> String {
    rehearsal_plan_lines(rehearsal).join("\n")
}

/// The `forge gate --dry-run` JSON envelope for a rehearsal.
///
/// `persisted` and `journaled` are `false` by construction and there is no
/// aggregate key at all, so nothing downstream can read this payload as a
/// run. `plan` is kept alongside `rehearsal` so a consumer that only ever
/// read the plan-line preview keeps finding a plan.
pub fn rehearsal_envelope(
    project_id: &str,
    runtime: &str,
    runtime_version: Option<&str>,
    rehearsal: &GateRehearsal,
) -> serde_json::Value {
    serde_json::json!({
        "contract": super::contract::GATE_CONTRACT_VERSION,
        "dry_run": true,
        "gate": {
            "project": project_id,
            "runtime": runtime,
            "runtime_version": runtime_version,
            "persisted": false,
            "journaled": false,
            "plan": rehearsal_plan_lines(rehearsal),
            "rehearsal": rehearsal,
        },
    })
}

fn word_or_unknown(text: &str) -> &str {
    if text.trim().is_empty() {
        "unknown"
    } else {
        text
    }
}
