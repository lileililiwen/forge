//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Reviewable plan produced by [`resolve_patterns`]. Empty
/// `steps` means the resolver could not satisfy any requested
/// pattern for the profile (the rejections carry the
/// explanation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UiPatternPlan {
    pub profile: String,
    pub steps: Vec<UiPatternStep>,
    pub rejections: Vec<UiPatternRejection>,
}
/// Versioned descriptor for one catalog pattern. A descriptor
/// that lacks states, typography, spacing, responsive,
/// accessibility, interaction, a version, an install strategy, a
/// tested platform mapping, tests or a documentation pointer is
/// rejected by [`validate_descriptor`] so meaningless primitives
/// or copied markup fragments never reach the catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiPatternDescriptor {
    pub id: String,
    pub version: String,
    /// Human-readable semantic intent (must be one of
    /// [`UI_PATTERN_INTENTS`]).
    pub intent: String,
    /// Short documentation pointer used by the inspector.
    pub documentation: String,
    /// State contract. The states named in
    /// [`UI_PATTERN_REQUIRED_STATES`] must all be present.
    pub states: Vec<UiPatternState>,
    pub typography: UiPatternTypography,
    pub spacing: UiPatternSpacing,
    pub responsive: UiPatternResponsive,
    pub accessibility: UiPatternAccessibility,
    pub interaction: UiPatternInteraction,
    /// Linked components the pattern depends on (e.g. a `form`
    /// pattern may depend on the `validated-form` component).
    /// A pattern that depends on a deprecated component is
    /// reported as a `ui-pattern-deprecated-dependency` so the
    /// planner does not silently install a stale surface.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Linked features the pattern depends on (e.g. `auth`).
    #[serde(default)]
    pub feature_deps: Vec<String>,
    /// Platform-specific adapters. A pattern with no adapter is
    /// refused.
    pub adapters: Vec<UiPatternAdapter>,
    /// Deterministic install strategy; required and non-empty.
    pub install_strategy: String,
    /// Test surface the descriptor references (catalog-wide,
    /// not per-adapter).
    pub tests: String,
    /// Current quality level.
    pub quality: UiPatternQuality,
    /// Evidence attached to the current quality level.
    pub evidence: UiPatternEvidence,
}
/// Responsive contract. Every pattern names at least one
/// breakpoint and a layout rule so the pattern does not collapse
/// to a single fixed width.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPatternResponsive {
    pub breakpoints: Vec<String>,
    pub layout: String,
}
/// Spacing contract. Every pattern names a token and a scale so
/// the layout follows the project design conventions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPatternSpacing {
    pub token: String,
    pub scale: String,
}
/// Request to resolve one or more pattern ids for a given
/// profile. The profile must be inspectable through the profile
/// catalog (planned or supported) and the ids must be unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPatternRequest {
    pub profile: String,
    pub pattern_ids: Vec<String>,
}
/// Request to install a pattern for a given profile. The
/// caller names the pattern id, the profile and an explicit
/// reason; the receipt is written under
/// `.forge/ui-patterns/<id>/install.json` and the source
/// artifact is written at the adapter's `artifact_path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPatternInstallRequest {
    pub pattern_id: String,
    pub profile: String,
    pub reason: String,
}
/// Evidence attached to a pattern. Promotion to `Certified`
/// requires the evidence to be complete and fresh: a security
/// review, a recent `last_verified` timestamp, test coverage at
/// or above [`UI_PATTERN_CERTIFIED_TEST_COVERAGE`] and at most
/// [`MAX_KNOWN_ISSUES`] known issues. Any missing or stale piece
/// is a refusal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiPatternEvidence {
    /// Number of registered projects that report the pattern as
    /// installed.
    pub usage_count: u32,
    /// Ratio (`0.0`-`1.0`) of the pattern surface covered by the
    /// referenced tests.
    pub test_coverage: f32,
    /// Timestamp of the last independent verification.
    pub last_verified: DateTime<Utc>,
    /// Public list of known issues; bounded by
    /// [`MAX_KNOWN_ISSUES`].
    pub known_issues: Vec<String>,
    /// Whether a security review has been recorded.
    pub security_review: bool,
}
/// One ordered step in a [`UiPatternPlan`]. Deterministic, exact
/// version, includes the quality level that was selected so the
/// caller can show the planner's evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UiPatternStep {
    pub id: String,
    pub version: String,
    pub quality: UiPatternQuality,
    pub action: String,
}
/// Evidence summary attached to a resolved step. The planner
/// surfaces the underlying evidence so the operator can audit
/// why a candidate was preferred.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UiPatternEvidenceSummary {
    pub id: String,
    pub quality: UiPatternQuality,
    pub evidence: UiPatternEvidence,
}
/// Quality classification for one pattern. A stricter level
/// requires stricter evidence; `Certified` is the planner-preferred
/// quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum UiPatternQuality {
    /// Initial implementation; not preferred by the planner. No
    /// security review required.
    Experimental,
    /// Independently exercised on at least one supported platform;
    /// tests are wired and the surface is reviewed.
    Verified,
    /// Production-grade: full evidence, security review, and fresh
    /// verification. The planner prefers compatible `Certified`
    /// patterns when multiple candidates satisfy a request.
    Certified,
    /// Marked for retirement; the planner must surface a policy
    /// conflict rather than silently selecting a deprecated
    /// candidate.
    Deprecated,
}
impl UiPatternQuality {
    pub fn label(&self) -> &'static str {
        match self {
            UiPatternQuality::Experimental => "experimental",
            UiPatternQuality::Verified => "verified",
            UiPatternQuality::Certified => "certified",
            UiPatternQuality::Deprecated => "deprecated",
        }
    }
}
/// Typography contract. Every pattern names a family, a scale and
/// a line-height so two patterns that share a typography token
/// stay visually consistent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPatternTypography {
    pub family: String,
    pub scale: String,
    pub line_height: String,
    pub weight: String,
}
/// Interaction contract. The pattern names the explicit user
/// choices and the typed outcomes so a copied markup fragment
/// without interaction is refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPatternInteraction {
    pub choices: Vec<String>,
    pub outcomes: Vec<String>,
}
/// Accessibility contract. Every pattern names a keyboard model,
/// a focus model, an aria model and a contrast guarantee so a
/// pattern without an accessibility contract is refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPatternAccessibility {
    pub keyboard: String,
    pub focus: String,
    pub aria: String,
    pub contrast: String,
}
/// One explicit state the pattern surfaces to the user. A
/// pattern must declare the canonical states
/// ([`UI_PATTERN_REQUIRED_STATES`]) so a copied markup fragment
/// that omits, e.g., the error state, is refused at validation
/// time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPatternState {
    pub name: String,
    pub description: String,
}
/// A single refusal in a plan. A successful resolution can
/// still carry rejections for ids that were requested but could
/// not be satisfied: a programming primitive, an unknown id, an
/// incompatible platform, a quality conflict, or a dependency on
/// a deprecated component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UiPatternRejection {
    pub id: String,
    pub code: String,
    pub reason: String,
}
/// Outcome of a `forge ui-pattern install` invocation. On
/// success `installed == true` and the relative artifact and
/// receipt paths are reported. On failure the prior state is
/// preserved and `note` names the missing precondition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UiPatternInstallOutcome {
    pub pattern_id: String,
    pub profile: String,
    pub installed: bool,
    pub files_written: Vec<String>,
    pub note: String,
}
/// Full outcome of a `forge ui-pattern resolve` invocation.
/// Carries the plan plus a per-step evidence summary, plus a
/// human note.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UiPatternResolveOutcome {
    pub profile: String,
    pub plan: UiPatternPlan,
    pub evidence_summary: Vec<UiPatternEvidenceSummary>,
    pub note: String,
}
/// One platform-specific adapter for a pattern. A pattern without
/// a tested adapter is refused (R1 failure scenario: a pattern
/// with no platform is not installable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPatternAdapter {
    /// Profile id (e.g. `react-web`, `nextjs-web`, `flutter-app`).
    pub profile: String,
    /// Short description of the platform-specific surface
    /// (e.g. `react-jsx` for `react-web`).
    pub surface: String,
    /// Path of the source artifact the installer writes (relative
    /// to the project root).
    pub artifact_path: String,
    /// Test surface the pattern references.
    pub tests: String,
    /// Content the installer writes for the source artifact. The
    /// content must be ordinary source: a copy-pasted HTML fragment
    /// or a screenshot base64 blob is refused by
    /// [`is_ordinary_source`].
    pub artifact_source: String,
}
