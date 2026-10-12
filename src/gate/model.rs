//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use super::contract::{DEFAULT_GATE_TIMEOUT, GATE_BIN_ENV};
use super::rehearsal::GateRehearsal;

/// One normalized check row from the sibling's `results[]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateCheck {
    pub id: String,
    pub state: GateCheckState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredRuntime {
    /// No `.project.json`, no key, or an empty value.
    Undeclared,
    /// A declaration exists but the document could not be read as JSON.
    Unreadable(String),
    /// A non-empty declaration string.
    Declared(String),
}
/// Aggregate verdict mapped from the sibling's gate status document.
/// `failed` and `unknown` keep a non-blocking failure and an
/// unclassifiable document honestly separated from `blocked`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateAggregate {
    Passed,
    Blocked,
    Failed,
    Unknown,
}
impl GateAggregate {
    pub fn label(self) -> &'static str {
        match self {
            GateAggregate::Passed => "passed",
            GateAggregate::Blocked => "blocked",
            GateAggregate::Failed => "failed",
            GateAggregate::Unknown => "unknown",
        }
    }
}
/// Resolved gate runtime plus the human-readable attempt list that made
/// the resolution (or its failure) auditable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRuntime {
    pub binary: PathBuf,
    pub name: String,
    pub attempts: Vec<String>,
}
/// The sibling's gate document has no `contract` field; it is
/// discriminated by shape (`blocked` + `results`), the same rule the
/// policy plane applies.
pub(super) struct MappedDocument {
    pub(super) status: String,
    pub(super) blocked: bool,
    pub(super) checks: Vec<GateCheck>,
    pub(super) manifest_digest: Option<String>,
    pub(super) rule_pack_version: Option<String>,
    pub(super) note: Option<String>,
}
/// How the persisted evidence compares against the current revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateFreshness {
    /// Evidence exists and its bound revision is the current HEAD.
    Fresh,
    /// Evidence exists but the revision moved (or the record was never
    /// bound / the current revision is unreadable): stale evidence can
    /// never satisfy a verification claim.
    Stale,
    /// No persisted evidence exists.
    Absent,
}
impl GateFreshness {
    pub fn label(self) -> &'static str {
        match self {
            GateFreshness::Fresh => "fresh",
            GateFreshness::Stale => "stale",
            GateFreshness::Absent => "absent",
        }
    }
}
/// Bounded invocation configuration, resolved from the environment and
/// CLI input by the caller.
#[derive(Debug, Clone)]
pub struct GateConfig {
    /// `FORGE_GATE_BIN` (or an explicit test path). Runs exactly as
    /// named with no fall-through, like every other adapter override.
    pub binary: Option<OsString>,
    pub timeout: Duration,
}
impl GateConfig {
    /// Resolve the override from the environment. Whitespace-only values
    /// are ignored so `FORGE_GATE_BIN=""` keeps declared/fallback
    /// resolution (same rule as the policy plane's override).
    pub fn from_env() -> Self {
        let binary =
            std::env::var_os(GATE_BIN_ENV).filter(|v| !v.to_string_lossy().trim().is_empty());
        GateConfig {
            binary,
            timeout: DEFAULT_GATE_TIMEOUT,
        }
    }
}
/// Outcome of one bounded gate invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    /// A parseable status document mapped into evidence. Only a real run
    /// produces this: a rehearsal document is recognised by its own shape
    /// and routed to [`GateOutcome::Rehearsal`] instead, so no aggregate
    /// or check state can ever be derived from a plan that ran nothing.
    Evidence(GateEvidence),
    /// A `--dry-run` whose runtime answered with the sibling's
    /// machine-readable plan rehearsal document, recognised by shape.
    /// Nothing was executed, persisted or journaled, and the rehearsal
    /// carries no aggregate, no check state and no revision binding, so
    /// it can never be read as a run outcome.
    Rehearsal {
        runtime: String,
        runtime_version: Option<String>,
        rehearsal: GateRehearsal,
    },
    /// A `--dry-run` whose answer Forge did not recognise as a rehearsal
    /// document — the older sibling's human plan text, an unrecognised
    /// contract family, a renamed field, or any non-JSON output. It is a
    /// genuine side-effect-free preview, not evidence; the plan lines are
    /// already redacted and bounded.
    PlanPreview {
        runtime: String,
        runtime_version: Option<String>,
        plan: Vec<String>,
    },
    /// The runtime could not be resolved, could not be spawned, timed
    /// out, or produced no parseable document. Prior evidence stays
    /// byte-identical.
    Unavailable { reason: String },
}
/// The versioned, revision-bound evidence record persisted per project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateEvidence {
    pub contract: String,
    pub project_id: String,
    pub runtime: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_version: Option<String>,
    /// Git HEAD captured at invocation. Evidence without a binding can
    /// never be `fresh` — an unbound revision is never proven current.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_pack_version: Option<String>,
    pub aggregate: GateAggregate,
    #[serde(default)]
    pub checks: Vec<GateCheck>,
    pub observed_at: String,
    /// Rehearsal marker. A persisted record is always the product of a
    /// real run (`dry_run: false`); a `--dry-run` is reported in the
    /// rehearsal or plan-preview shape and never persisted — this field
    /// is the last line of defence if one ever reaches this type.
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
impl GateEvidence {
    /// Short form of the bound revision for display and journal lines.
    pub fn short_revision(&self) -> Option<String> {
        self.revision
            .as_ref()
            .map(|sha| sha.chars().take(12).collect())
    }
}
/// One per-check row. Unknown or unexpected state strings become
/// `unresolved` — never `pass`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateCheckState {
    Pass,
    Fail,
    Skip,
    NotApplicable,
    Unresolved,
}
impl GateCheckState {
    pub fn label(self) -> &'static str {
        match self {
            GateCheckState::Pass => "pass",
            GateCheckState::Fail => "fail",
            GateCheckState::Skip => "skip",
            GateCheckState::NotApplicable => "not_applicable",
            GateCheckState::Unresolved => "unresolved",
        }
    }
    pub(super) fn from_raw(raw: &str) -> Self {
        match raw {
            "PASS" => GateCheckState::Pass,
            "FAIL" => GateCheckState::Fail,
            "SKIP" => GateCheckState::Skip,
            "NOT_APPLICABLE" => GateCheckState::NotApplicable,
            _ => GateCheckState::Unresolved,
        }
    }
}
