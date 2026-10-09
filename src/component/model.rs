//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Reviewable plan produced by [`resolve_components`]. Empty `steps`
/// means the resolver could not satisfy any requested component for
/// the profile (the rejections carry the explanation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComponentPlan {
    pub profile: String,
    pub steps: Vec<ComponentStep>,
    pub rejections: Vec<ComponentRejection>,
}
/// Evidence attached to a descriptor. Promotion to `Certified`
/// requires the evidence to be complete and fresh: a security
/// review, a recent `last_verified` timestamp, test coverage at or
/// above [`CERTIFIED_TEST_COVERAGE`] and at most
/// [`MAX_KNOWN_ISSUES`] known issues. Any missing or stale piece is
/// a refusal (R2 failure scenario).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentEvidence {
    /// Number of registered projects that report the component as
    /// installed.
    pub usage_count: u32,
    /// Ratio (`0.0`-`1.0`) of the component surface covered by the
    /// referenced tests.
    pub test_coverage: f32,
    /// Timestamp of the last independent verification.
    pub last_verified: DateTime<Utc>,
    /// Public list of known issues; a `Certified` component must
    /// own its caveats, but the list is bounded.
    pub known_issues: Vec<String>,
    /// Whether a security review has been recorded.
    pub security_review: bool,
}
/// One port on the component contract: an explicit input or output
/// callers can rely on. The name is the contract identifier; the
/// description is rendered for human output and consumed by the
/// planner when picking between stack implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentPort {
    pub name: String,
    pub description: String,
}
/// Request to promote a component's quality level. The caller names
/// the new quality and supplies a reason; the receipt is written
/// under `.forge/components/<id>/qualify.json`. The promotion only
/// succeeds when the new quality's evidence gate is met and the
/// referenced evidence is fresh (R2 failure scenario: a missing
/// security review or stale verification refuses the promotion and
/// the prior quality level remains).
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentQualifyRequest {
    pub component_id: String,
    pub target_quality: ComponentQuality,
    pub reason: String,
}
/// Evidence required by the promotion gate for `Certified`. The
/// `last_verified` and `test_coverage` fields override the catalog
/// defaults; `security_review` and `known_issues` re-confirm the
/// descriptor's evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentQualifyEvidence {
    pub test_coverage: f32,
    pub last_verified: DateTime<Utc>,
    pub known_issues: Vec<String>,
    pub security_review: bool,
}
/// Outcome of a `forge component qualify` invocation. On success
/// `promoted == true` and `prior_quality` is the quality level the
/// descriptor carried before the promotion; on failure the prior
/// quality level is preserved and `note` names the missing or stale
/// evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComponentQualifyOutcome {
    pub component_id: String,
    pub prior_quality: ComponentQuality,
    pub target_quality: ComponentQuality,
    pub promoted: bool,
    pub files_written: Vec<String>,
    pub note: String,
}
/// Evidence summary attached to a resolved step. The planner
/// surfaces the underlying evidence so the operator can audit why a
/// candidate was preferred (R2 success scenario).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComponentEvidenceSummary {
    pub id: String,
    pub quality: ComponentQuality,
    pub evidence: ComponentEvidence,
}
/// One ordered step in a [`ComponentPlan`]. Deterministic, exact
/// version, includes the quality level that was selected so the
/// caller can show the planner's evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComponentStep {
    pub id: String,
    pub version: String,
    pub quality: ComponentQuality,
    pub action: String,
}
/// Request to resolve one or more component ids for a given profile.
/// The profile must be inspectable through the profile catalog
/// (planned or supported) and the ids must be unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentRequest {
    pub profile: String,
    pub component_ids: Vec<String>,
}
/// A single refusal in a plan. A successful resolution can still
/// carry rejections for ids that were requested but could not be
/// satisfied: a programming primitive, an unknown id, an incompatible
/// profile, or a quality conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComponentRejection {
    pub id: String,
    pub code: String,
    pub reason: String,
}
/// Full outcome of a `forge component resolve` invocation. Carries
/// the plan plus a per-step evidence summary, plus a human note.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComponentResolveOutcome {
    pub profile: String,
    pub plan: ComponentPlan,
    pub evidence_summary: Vec<ComponentEvidenceSummary>,
    pub note: String,
}
/// Quality classification for one descriptor. A stricter level
/// requires stricter evidence; `Certified` is the planner-preferred
/// quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ComponentQuality {
    /// Initial implementation; not preferred by the planner. No
    /// security review required.
    Experimental,
    /// Independently exercised on at least one supported profile;
    /// tests are wired and the surface is reviewed.
    Verified,
    /// Production-grade: full evidence, security review, and fresh
    /// verification. The planner prefers compatible `Certified`
    /// components when multiple candidates satisfy a request.
    Certified,
    /// Marked for retirement; the planner must surface a policy
    /// conflict rather than silently selecting a deprecated
    /// candidate.
    Deprecated,
}
impl ComponentQuality {
    pub fn label(&self) -> &'static str {
        match self {
            ComponentQuality::Experimental => "experimental",
            ComponentQuality::Verified => "verified",
            ComponentQuality::Certified => "certified",
            ComponentQuality::Deprecated => "deprecated",
        }
    }
}
/// Shared semantic contract: the typed inputs and outputs every
/// stack-specific implementation of the same component must expose.
/// Two descriptors that share a contract but differ in stack remain
/// distinct catalog entries (R1 boundary scenario).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentContract {
    pub inputs: Vec<ComponentPort>,
    pub outputs: Vec<ComponentPort>,
}
/// Versioned descriptor for one catalog component. A descriptor that
/// lacks inputs, outputs, a version, an install strategy, tests or a
/// tested profile mapping is rejected by [`validate_descriptor`] so
/// meaningless primitives or empty shells never reach the catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDescriptor {
    pub id: String,
    pub version: String,
    /// Human-readable semantic purpose (rendered in the inspector and
    /// in the resolver's evidence summary).
    pub purpose: String,
    pub contract: ComponentContract,
    /// Linked features the component depends on (e.g.
    /// `idempotency-guard` may depend on `postgres`).
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Profiles that ship a tested implementation of the contract.
    /// A descriptor without a tested profile mapping is refused
    /// (R1 failure scenario: a component with no platform is not
    /// installable).
    pub profiles: Vec<String>,
    /// Deterministic install strategy; required and non-empty.
    pub install_strategy: String,
    /// Policy validators the install reports; optional.
    #[serde(default)]
    pub validation: Vec<String>,
    /// Short documentation pointer used by the inspector.
    pub documentation: String,
    /// Test surface the descriptor references.
    pub tests: String,
    /// Current quality level.
    pub quality: ComponentQuality,
    /// Evidence attached to the current quality level.
    pub evidence: ComponentEvidence,
}
