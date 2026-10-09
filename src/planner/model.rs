//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::{Deserialize, Serialize};

/// One pinned step in an [`AssemblyPlan`]. The `kind` and `target`
/// drive the executor; `version` is the catalog-pinned value the
/// executor will request; `evidence` records the per-step catalog
/// entry the resolver used so the operator can audit the choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanStep {
    pub kind: PlanStepKind,
    pub target: String,
    pub version: String,
    pub action: String,
    pub evidence: String,
}
/// One named constraint attached to an intent. Constraints are
/// declarative metadata the planner either recognises (and threads
/// into the resolver) or refuses as unknown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentConstraint {
    pub key: String,
    pub value: String,
}
/// Structured intent submitted to the planner. The model (or a
/// hand-written fixture) produces this; the validator binds the
/// intent to a profile version and produces a [`ValidatedIntent`]
/// before any side effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    pub action: IntentAction,
    pub profile: String,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    #[serde(default)]
    pub forbidden_capabilities: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<IntentConstraint>,
}
/// Bounded set of action verbs the planner accepts. The validator
/// refuses any action outside this set so a model cannot smuggle
/// shell or arbitrary Core calls into the pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IntentAction {
    /// Create a new project from the named profile with the required
    /// capabilities; the registry records the project and the plan
    /// installs the deterministic assets in dependency order.
    CreateProject,
    /// Extend an existing project with additional features,
    /// components and UI patterns. The plan never touches the
    /// project's existing ownership receipts.
    ExtendProject,
}
impl IntentAction {
    pub fn label(&self) -> &'static str {
        match self {
            IntentAction::CreateProject => "create_project",
            IntentAction::ExtendProject => "extend_project",
        }
    }
}
/// Result of [`validate_intent`]: the original intent, the pinned
/// profile descriptor, and a normalized form the resolver can
/// consume without re-running the validator. The `backend_boundary`
/// hint is set when a client profile is paired with a server-side
/// capability; the planner surfaces the recommended backend so the
/// caller can decide whether to add a backend profile or remove the
/// capability (R1 boundary).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedIntent {
    pub intent: Intent,
    pub profile_id: String,
    pub profile_version: String,
    pub adapter: String,
    pub backend_boundary: Option<String>,
    pub note: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PlanStepKind {
    /// Run the project's doctor check.
    Doctor,
    /// Run the profile's native test command.
    Test,
    /// Run the configured DriftWatch adapter.
    QualityPolicy,
    /// Install a feature (delegates to `feature::resolve_plan`).
    InstallFeature,
    /// Install a component (delegates to `component::resolve_outcome`).
    InstallComponent,
    /// Install a UI pattern (delegates to `ui_pattern::resolve_outcome`).
    InstallUiPattern,
}
impl PlanStepKind {
    pub fn label(&self) -> &'static str {
        match self {
            PlanStepKind::Doctor => "doctor",
            PlanStepKind::Test => "test",
            PlanStepKind::QualityPolicy => "quality_policy",
            PlanStepKind::InstallFeature => "install_feature",
            PlanStepKind::InstallComponent => "install_component",
            PlanStepKind::InstallUiPattern => "install_ui_pattern",
        }
    }
}
/// Outcome of `forge intent validate`. Either a
/// [`ValidatedIntent`] (with the normalized form) or a typed
/// `IntentInvalid` / `IntentAmbiguous` rejection so the caller can
/// surface the boundary conflict without silently continuing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentValidationOutcome {
    pub validated: Option<ValidatedIntent>,
    pub note: String,
}
/// Reviewable deterministic plan produced by
/// [`resolve_plan`]. Steps are pinned to the validated
/// profile version, the catalog hashes captured at resolve time, and
/// the working-tree revision the resolver saw. The
/// `intent_hash` and `catalog_hash` are the values `apply_plan`
/// re-checks to detect a stale plan (R2 failure scenario). The
/// captured `intent` lets the executor recompute the intent hash
/// without re-deriving the intent from the plan's steps (the
/// assembly plan is the post-resolve artifact and may include
/// doctor/test/quality-policy steps that the original intent did
/// not name).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssemblyPlan {
    pub plan_id: String,
    pub profile: String,
    pub profile_version: String,
    pub intent: Intent,
    pub intent_hash: String,
    pub catalog_hash: String,
    pub source_revision: Option<String>,
    pub steps: Vec<PlanStep>,
    pub unresolved: Vec<UnresolvedWork>,
    pub note: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedStep {
    pub kind: PlanStepKind,
    pub target: String,
    pub status: String,
    pub note: String,
}
/// Outcome of `forge intent resolve`. The plan is the artifact the
/// operator reviews; the receipt path is the on-disk file `apply`
/// will read to re-validate before execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentResolveOutcome {
    pub plan: AssemblyPlan,
    pub receipt_path: String,
    pub note: String,
}
/// Outcome of `forge intent apply`. The `applied_steps` list is the
/// order in which the executor reached the underlying Core contracts
/// (already-delivered steps are reported as `skipped` so retries do
/// not redo recorded work); `stale` is true when the re-validation
/// gate refused the plan before any step ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentApplyOutcome {
    pub plan_id: String,
    pub applied_steps: Vec<AppliedStep>,
    pub stale: bool,
    pub note: String,
    pub files_written: Vec<String>,
}
/// A gap the deterministic resolver cannot fill: the requirement
/// needs glue/business code or a semantic spec. The executor skips
/// unresolved entries so a missing deterministic part is observable
/// rather than masked by an AI substitution (R2 boundary).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnresolvedWork {
    pub requirement: String,
    pub reason: String,
    pub hint: String,
}
