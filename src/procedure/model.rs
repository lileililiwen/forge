//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use serde::{Deserialize, Serialize};

/// One ordered step in a [`ProcedureSpec`]. The `op` is the
/// stable Core operation the step invokes; `args` are the
/// literal arguments (no shell interpolation, no bypass
/// markers); `description` is the audit trail so a model or
/// an operator can read the SOP without re-deriving the
/// rationale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcedureStep {
    /// 1-based ordinal inside the procedure, for human and
    /// JSON output. The catalog assigns the value; the
    /// validator refuses steps with a missing or zero
    /// ordinal.
    pub ordinal: u32,
    /// Stable Core operation token.
    pub op: CoreOperation,
    /// Literal arguments the step forwards to the Core
    /// operation. Empty list when the operation takes no
    /// arguments. The validator scans every argument for a
    /// bypass marker (R2 failure scenario).
    #[serde(default)]
    pub args: Vec<String>,
    /// Human-readable description of what the step does and
    /// why the procedure runs it.
    pub description: String,
}
/// Stable, machine-readable operation tokens a procedure
/// step may reference. Each variant maps to a Core contract
/// that exists in the current build; an unknown operation is
/// refused by [`validate_procedure`] with the typed
/// `procedure-unsupported-operation` code (R1 failure
/// scenario).
///
/// The set is intentionally a closed enumeration: planned
/// operations (`push`, `planner.dispatch`, `agent.pause`,
/// `agent.takeover`) are absent so a procedure that
/// references an unavailable or unstable operation is
/// refused by validation rather than silently scheduled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CoreOperation {
    /// `forge profile inspect` (read-only, lists the
    /// descriptor for a given profile id).
    ProfileInspect,
    /// `forge profile resolve` (read-only, returns the
    /// resolved profile descriptor for a capability set).
    ProfileResolve,
    /// `forge profile preflight` (read-only, checks the
    /// required toolchain is on PATH).
    ProfilePreflight,
    /// `forge feature resolve` (read-only, deterministic
    /// install plan).
    FeatureResolve,
    /// `forge feature add` (mutating, registry journal).
    FeatureAdd,
    /// `forge feature remove` (mutating, registry journal).
    FeatureRemove,
    /// `forge feature upgrade` (mutating, registry journal).
    FeatureUpgrade,
    /// `forge component resolve` (read-only, quality-aware
    /// selection).
    ComponentResolve,
    /// `forge ui-pattern resolve` (read-only, certified-first
    /// selection).
    UiPatternResolve,
    /// `forge ui-pattern install` (mutating, writes ordinary
    /// source plus the install receipt).
    UiPatternInstall,
    /// `forge intent validate` (read-only, returns a
    /// `ValidatedIntent`).
    IntentValidate,
    /// `forge intent resolve` (mutating, persists a plan
    /// receipt).
    IntentResolve,
    /// `forge intent apply` (mutating, requires `--confirm`).
    IntentApply,
    /// `forge doctor` (read-only, returns the finding
    /// inventory).
    DoctorRun,
    /// `forge test` (mutating, runs the profile's native
    /// test command).
    TestRun,
    /// `forge commit` (mutating, requires explicit paths).
    Commit,
    /// `forge doctor --policy` via the configured
    /// DriftWatch adapter.
    PolicyRun,
    /// `forge spec generate` (mutating, writes a bounded
    /// proposal under `.forge/specs/`).
    SpecGenerate,
    /// `forge spec apply` (mutating, applies a routing
    /// decision).
    SpecApply,
    /// `forge agent start` (mutating, opens a session).
    AgentStart,
    /// `forge upgrade` (mutating, applies an upgrade plan).
    UpgradeApply,
    /// `forge upgrade --all` (mutating, fleet).
    UpgradeFleet,
    /// `forge import` (mutating, requires `--accept`).
    ImportRun,
    /// `forge deploy plan` (read-only, captures target +
    /// revision).
    DeployPlan,
    /// `forge deploy apply` (mutating, requires
    /// `--confirm`).
    DeployApply,
    /// `forge deploy observe` (read-only, re-runs the
    /// health check).
    DeployObserve,
    /// `forge release prepare` (read-only, captures checks
    /// and changelog).
    ReleasePrepare,
    /// `forge release apply` (mutating, requires
    /// `--confirm`).
    ReleaseApply,
    /// `forge docs translate` (mutating, runs the
    /// configured translation adapter).
    DocsTranslate,
    /// `forge mirror` (mutating, requires `--confirm`).
    MirrorApply,
    /// The synthetic final step that every procedure
    /// schedules to surface unresolved findings; the step
    /// never carries an action, only the description of the
    /// gaps that must be reported (R2 boundary scenario).
    ReportFindings,
}
impl CoreOperation {
    /// Stable, snake_case token used in JSON and on the CLI.
    pub fn label(&self) -> &'static str {
        match self {
            CoreOperation::ProfileInspect => "profile.inspect",
            CoreOperation::ProfileResolve => "profile.resolve",
            CoreOperation::ProfilePreflight => "profile.preflight",
            CoreOperation::FeatureResolve => "feature.resolve",
            CoreOperation::FeatureAdd => "feature.add",
            CoreOperation::FeatureRemove => "feature.remove",
            CoreOperation::FeatureUpgrade => "feature.upgrade",
            CoreOperation::ComponentResolve => "component.resolve",
            CoreOperation::UiPatternResolve => "ui_pattern.resolve",
            CoreOperation::UiPatternInstall => "ui_pattern.install",
            CoreOperation::IntentValidate => "intent.validate",
            CoreOperation::IntentResolve => "intent.resolve",
            CoreOperation::IntentApply => "intent.apply",
            CoreOperation::DoctorRun => "doctor.run",
            CoreOperation::TestRun => "test.run",
            CoreOperation::Commit => "commit",
            CoreOperation::PolicyRun => "policy.run",
            CoreOperation::SpecGenerate => "spec.generate",
            CoreOperation::SpecApply => "spec.apply",
            CoreOperation::AgentStart => "agent.start",
            CoreOperation::UpgradeApply => "upgrade.apply",
            CoreOperation::UpgradeFleet => "upgrade.fleet",
            CoreOperation::ImportRun => "import.run",
            CoreOperation::DeployPlan => "deploy.plan",
            CoreOperation::DeployApply => "deploy.apply",
            CoreOperation::DeployObserve => "deploy.observe",
            CoreOperation::ReleasePrepare => "release.prepare",
            CoreOperation::ReleaseApply => "release.apply",
            CoreOperation::DocsTranslate => "docs.translate",
            CoreOperation::MirrorApply => "mirror.apply",
            CoreOperation::ReportFindings => "report_findings",
        }
    }
    /// Parse the snake_case label. Returns `None` for any
    /// token outside the closed enumeration so the validator
    /// can refuse it with a typed `procedure-unsupported-operation`
    /// rejection (R1 failure scenario).
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "profile.inspect" => Some(CoreOperation::ProfileInspect),
            "profile.resolve" => Some(CoreOperation::ProfileResolve),
            "profile.preflight" => Some(CoreOperation::ProfilePreflight),
            "feature.resolve" => Some(CoreOperation::FeatureResolve),
            "feature.add" => Some(CoreOperation::FeatureAdd),
            "feature.remove" => Some(CoreOperation::FeatureRemove),
            "feature.upgrade" => Some(CoreOperation::FeatureUpgrade),
            "component.resolve" => Some(CoreOperation::ComponentResolve),
            "ui_pattern.resolve" => Some(CoreOperation::UiPatternResolve),
            "ui_pattern.install" => Some(CoreOperation::UiPatternInstall),
            "intent.validate" => Some(CoreOperation::IntentValidate),
            "intent.resolve" => Some(CoreOperation::IntentResolve),
            "intent.apply" => Some(CoreOperation::IntentApply),
            "doctor.run" => Some(CoreOperation::DoctorRun),
            "test.run" => Some(CoreOperation::TestRun),
            "commit" => Some(CoreOperation::Commit),
            "policy.run" => Some(CoreOperation::PolicyRun),
            "spec.generate" => Some(CoreOperation::SpecGenerate),
            "spec.apply" => Some(CoreOperation::SpecApply),
            "agent.start" => Some(CoreOperation::AgentStart),
            "upgrade.apply" => Some(CoreOperation::UpgradeApply),
            "upgrade.fleet" => Some(CoreOperation::UpgradeFleet),
            "import.run" => Some(CoreOperation::ImportRun),
            "deploy.plan" => Some(CoreOperation::DeployPlan),
            "deploy.apply" => Some(CoreOperation::DeployApply),
            "deploy.observe" => Some(CoreOperation::DeployObserve),
            "release.prepare" => Some(CoreOperation::ReleasePrepare),
            "release.apply" => Some(CoreOperation::ReleaseApply),
            "docs.translate" => Some(CoreOperation::DocsTranslate),
            "mirror.apply" => Some(CoreOperation::MirrorApply),
            "report_findings" => Some(CoreOperation::ReportFindings),
            _ => None,
        }
    }
    /// True when this operation is the synthetic final
    /// `report_findings` step. A procedure that ends with
    /// any other step is refused so the workflow always
    /// surfaces gaps rather than asserting completion (R2
    /// boundary scenario).
    pub fn is_report(&self) -> bool {
        matches!(self, CoreOperation::ReportFindings)
    }
}
/// One immutable, versioned procedure spec. The catalog
/// ships eight named specs in v0.1.0; new procedures land in
/// later cycles by adding an entry to the catalog and bumping
/// [`PROCEDURE_CONTRACT_VERSION`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcedureSpec {
    /// Stable kebab-case id, unique in the catalog.
    pub id: String,
    /// Contract data version the spec was authored against.
    pub version: String,
    /// Human-readable title.
    pub title: String,
    /// One-paragraph description of the SOP.
    pub description: String,
    /// Declarative prerequisites the operator must satisfy
    /// before the procedure runs. The validator does not
    /// enforce them; the list is a reviewer-facing
    /// checklist.
    #[serde(default)]
    pub prerequisites: Vec<String>,
    /// Ordered list of steps. The list must end with a
    /// single [`CoreOperation::ReportFindings`] step so the
    /// workflow always surfaces gaps (R2 boundary).
    pub steps: Vec<ProcedureStep>,
    /// Human-readable verification criteria the operator
    /// can audit after the workflow ends.
    pub verification: String,
}
/// Lightweight entry returned by [`procedure_catalog`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcedureListEntry {
    pub id: String,
    pub version: String,
    pub title: String,
    pub description: String,
}

/// Outcome of a [`validate_procedure`] call. `Ok(_)` returns
/// the canonical spec; `Err(_)` is the typed Core rejection
/// (`procedure-invalid` / `procedure-unsupported-operation` /
/// `procedure-bypass-refused`) the CLI/MCP transport
/// renders.
pub type ProcedureValidationOutcome = Result<ProcedureSpec, ForgeError>;
