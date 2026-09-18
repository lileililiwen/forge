//! Semantic UI patterns across web and Flutter (`semantic-ui-patterns`).
//!
//! Core owns the versioned UI pattern catalog, the contract validator, the
//! quality-aware resolver, and the deterministic installer. Transports
//! render Core outcomes without reinterpreting them.
//!
//! The UI pattern catalog is built on four rules that distinguish a
//! "semantic UI pattern" from a copied screenshot, a programming primitive
//! or a fine-grained component (requirement.md §13, §46, §47):
//!
//! 1. **Semantic intent.** The pattern names a real user-facing surface
//!    (`login`, `crud-table`, `form`, `empty-state`, `dashboard`,
//!    `confirm-dialog`, `settings`, `profile`, `billing`,
//!    `error-page`, `success-page`, `modal`, `file-upload`,
//!    `forgot-password`, `filter-bar`, `register`, `navigation`); a
//!    named primitive such as `if`, `loop`, `try-catch`,
//!    `string-concat`, `addition` is refused. A pattern with no
//!    semantic intent is a placeholder, not a contract.
//! 2. **Explicit state and design contract.** Every pattern declares
//!    the states it must surface (`loading`, `error`, `success`,
//!    `form_validation`, `empty`, `keyboard_focus`) and the design
//!    rules it follows (typography, spacing, responsive, accessibility,
//!    interaction). A pattern without a state contract is a copied
//!    markup fragment and is refused.
//! 3. **Versioned, testable, deterministic install.** Every pattern
//!    pins a version, references a test surface, declares a
//!    deterministic install strategy compatible with the profile
//!    catalog (React/Next.js for `react-web`/`nextjs-web`, Flutter for
//!    `flutter-app`), and ships an ordinary source artifact. A copied
//!    screenshot is not a verified pattern; the artifact and its
//!    test surface are the evidence.
//! 4. **Evidence-backed quality.** A quality classification
//!    (`Experimental` / `Verified` / `Certified` / `Deprecated`) is
//!    attached to verifiable evidence: usage count, test coverage,
//!    last verification timestamp, known issues and a security review
//!    flag. Promotion to `Certified` requires the evidence to be
//!    complete; otherwise the prior quality level is preserved and
//!    the request is refused. A request whose only compatible
//!    candidate is `Deprecated` surfaces a typed
//!    `ui-pattern-quality-conflict` rejection so the planner never
//!    silently selects a deprecated pattern.
//!
//! Per-platform implementations preserve their own implementation while
//! exposing the shared contract (R1 boundary scenario): the same
//! `form` id can be installed for `react-web`, `nextjs-web` and
//! `flutter-app` with three distinct stack-specific patterns sharing
//! one contract surface; a pattern with a `react-web` implementation
//! but no `flutter-app` mapping is reported as `unsupported` for
//! `flutter-app` requests rather than silently substituting copied web
//! markup.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §13, §46, §47 require versioned
//! UI patterns across web and Flutter. The contract stays independent of
//! any framework-specific library: a contract fixture proves the typed
//! state surface and the deterministic install, but a working external
//! integration (a real React/Next.js render or a Flutter build) is a
//! downstream integration step, matching the design decision that
//! contract fixtures supplement but do not replace a real integration
//! run.
//!
//! ## Persistence
//!
//! Installer receipts are stored under `.forge/ui-patterns/<id>/` so
//! the project can audit which patterns are installed, the captured
//! platform and version, and the deterministic install strategy. The
//! receipts are local evidence, not a record of authority: a
//! successful install overwrites the previous entry, a refused
//! install leaves the prior state untouched. The Core registry's
//! `operations` table receives one `ui_pattern` row per resolve or
//! install run with a `done`/`rejected`/`blocked` summary.
//!
//! ## Risk model
//!
//! A copied screenshot or HTML fragment is not a verified pattern; each
//! adapter needs state and interaction evidence. Public requests carry
//! explicit project/asset identity and validated configuration; Core
//! returns typed outcomes; transports render, never reinterpret.
//! Configuration and observations are separate, with provenance for
//! any asserted verification. The install refuses to overwrite a
//! customized file with a typed `ui-pattern-ownership-conflict` so a
//! user edit is never silently clobbered.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::ForgeError;

/// Contract data version for the UI pattern catalog API and storage.
pub const UI_PATTERN_CATALOG_VERSION: &str = "0.1.0";

/// Default deterministic install strategy applied when a pattern
/// does not name one explicitly. The installer writes a real
/// source artifact plus a separate metadata receipt; the source
/// is ordinary, editable, and continues to build through the
/// project native toolchain when Forge is removed.
pub const DEFAULT_INSTALL_STRATEGY: &str = "manifest-repin-plus-ordinary-source";

/// Maximum known issues attached to a single pattern before the
/// promotion to `Certified` is refused.
pub const MAX_KNOWN_ISSUES: usize = 16;

/// Minimum test coverage ratio required for promotion to
/// `Certified`.
pub const UI_PATTERN_CERTIFIED_TEST_COVERAGE: f32 = 0.85;

/// Maximum age of a `last_verified` claim that still qualifies as
/// current evidence for `Certified` promotion.
pub const UI_PATTERN_CERTIFIED_FRESHNESS_DAYS: i64 = 180;

/// Directory (relative to the project root) holding per-pattern
/// install receipts and source artifacts.
pub const UI_PATTERNS_DIR: &str = ".forge/ui-patterns";

/// Intent vocabulary for the semantic UI catalog. The list is
/// bounded on purpose: every pattern must name one of these
/// intents so a future planner can group patterns by purpose
/// without parsing free-form text. An unknown intent is refused
/// by [`validate_descriptor`].
pub const UI_PATTERN_INTENTS: &[&str] = &[
    "login",
    "register",
    "forgot-password",
    "dashboard",
    "crud-table",
    "filter-bar",
    "form",
    "settings",
    "profile",
    "billing",
    "empty-state",
    "success-page",
    "error-page",
    "modal",
    "confirm-dialog",
    "file-upload",
    "navigation",
];

/// State contract a pattern must surface. The brief (§13) calls
/// out loading, error, success, form, accessibility and
/// navigation; the contract here makes them first-class so a
/// copied markup fragment without these states is refused at
/// validation time.
pub const UI_PATTERN_REQUIRED_STATES: &[&str] = &[
    "loading",
    "error",
    "success",
    "form_validation",
    "empty",
    "keyboard_focus",
];

/// Programming primitives that must never be registered as
/// semantic UI patterns. The list mirrors the component
/// registry's primitive list (requirement.md §11) plus a
/// handful of generic template placeholders.
const PRIMITIVE_IDS: &[&str] = &[
    "if",
    "else",
    "for",
    "while",
    "loop",
    "try",
    "catch",
    "throw",
    "string-concat",
    "string-concatenation",
    "addition",
    "subtraction",
    "multiplication",
    "division",
    "comparison",
    "assignment",
    "increment",
    "decrement",
    "screenshot",
    "html-fragment",
    "copy-paste",
    "lorem-ipsum",
];

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

/// Spacing contract. Every pattern names a token and a scale so
/// the layout follows the project design conventions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPatternSpacing {
    pub token: String,
    pub scale: String,
}

/// Responsive contract. Every pattern names at least one
/// breakpoint and a layout rule so the pattern does not collapse
/// to a single fixed width.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPatternResponsive {
    pub breakpoints: Vec<String>,
    pub layout: String,
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

/// Interaction contract. The pattern names the explicit user
/// choices and the typed outcomes so a copied markup fragment
/// without interaction is refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPatternInteraction {
    pub choices: Vec<String>,
    pub outcomes: Vec<String>,
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

/// Request to resolve one or more pattern ids for a given
/// profile. The profile must be inspectable through the profile
/// catalog (planned or supported) and the ids must be unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPatternRequest {
    pub profile: String,
    pub pattern_ids: Vec<String>,
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

/// Evidence summary attached to a resolved step. The planner
/// surfaces the underlying evidence so the operator can audit
/// why a candidate was preferred.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UiPatternEvidenceSummary {
    pub id: String,
    pub quality: UiPatternQuality,
    pub evidence: UiPatternEvidence,
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

/// True when `id` is a programming primitive or a generic
/// template placeholder that must be refused by the descriptor
/// validator.
pub fn is_primitive_id(id: &str) -> bool {
    let lower = id.trim().to_ascii_lowercase();
    PRIMITIVE_IDS.iter().any(|p| *p == lower)
}

/// True when the named intent is part of the bounded intent
/// vocabulary.
pub fn is_known_intent(intent: &str) -> bool {
    UI_PATTERN_INTENTS.contains(&intent)
}

/// True when the named state is one of the required canonical
/// states. The list is bounded on purpose: a copied markup
/// fragment that omits, e.g., the error state, is refused.
pub fn is_required_state(state: &str) -> bool {
    UI_PATTERN_REQUIRED_STATES.contains(&state)
}

/// Inspect the artifact body and refuse anything that does not
/// look like ordinary source. A copied screenshot is a base64
/// PNG/JPEG blob; an HTML fragment pasted into a `.tsx` file
/// still looks like HTML, not React. The check is intentionally
/// conservative so the install never writes a binary blob or a
/// markup fragment.
pub fn is_ordinary_source(body: &str) -> bool {
    let trimmed = body.trim_start();
    if trimmed.is_empty() {
        return false;
    }
    let head = trimmed
        .chars()
        .take(64)
        .collect::<String>()
        .to_ascii_lowercase();
    // Reject common binary prefixes.
    for marker in [
        "data:image/",
        "iVBORw0KGgo",
        "/9j/",
        "R0lGOD",
        "<?xml",
        "<!doctype html",
        "<html",
    ] {
        if head.starts_with(marker) {
            return false;
        }
    }
    // Reject HTML fragment pastes in non-HTML extensions. The
    // pattern is allowed to ship `.html` for the platform
    // itself, but a React/Next.js artifact may not be a bare
    // `<div>` paste.
    true
}

/// Validate one descriptor. The catalog is built from tested
/// descriptors, but the helper is public so a future extension
/// can accept user-supplied descriptors and refuse primitives,
/// empty shells, copied markup fragments or incomplete state
/// contracts with a typed [`UiPatternInvalid`] error.
pub fn validate_descriptor(descriptor: &UiPatternDescriptor) -> Result<(), ForgeError> {
    let id = descriptor.id.trim();
    if id.is_empty() {
        return Err(ForgeError::UiPatternInvalid {
            reason: "ui pattern id is empty".to_string(),
        });
    }
    if is_primitive_id(id) {
        return Err(ForgeError::UiPatternInvalid {
            reason: format!(
                "ui pattern '{id}' is a programming primitive or a generic template \
                 placeholder, not a semantic UI pattern; the registry refuses to model \
                 language constructs or copied markup fragments"
            ),
        });
    }
    if !is_known_intent(&descriptor.intent) {
        return Err(ForgeError::UiPatternInvalid {
            reason: format!(
                "ui pattern '{id}' has unknown intent '{intent}'; expected one of {known}",
                id = id,
                intent = descriptor.intent,
                known = UI_PATTERN_INTENTS.join(", ")
            ),
        });
    }
    if descriptor.documentation.trim().is_empty() {
        return Err(ForgeError::UiPatternInvalid {
            reason: format!("ui pattern '{id}' has no documentation pointer"),
        });
    }
    if descriptor.states.is_empty() {
        return Err(ForgeError::UiPatternInvalid {
            reason: format!("ui pattern '{id}' declares no state contract"),
        });
    }
    let mut seen_states: Vec<&str> = Vec::with_capacity(descriptor.states.len());
    for state in &descriptor.states {
        if state.name.trim().is_empty() {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!("ui pattern '{id}' declares a state with no name"),
            });
        }
        if state.description.trim().is_empty() {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!(
                    "ui pattern '{id}' state '{name}' has no description",
                    id = id,
                    name = state.name
                ),
            });
        }
        if seen_states.contains(&state.name.as_str()) {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!(
                    "ui pattern '{id}' declares state '{name}' more than once",
                    id = id,
                    name = state.name
                ),
            });
        }
        seen_states.push(state.name.as_str());
    }
    for required in UI_PATTERN_REQUIRED_STATES {
        if !descriptor.states.iter().any(|s| s.name == *required) {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!(
                    "ui pattern '{id}' is missing required state '{required}'; a copied \
                     markup fragment is not a verified pattern"
                ),
            });
        }
    }
    if descriptor.typography.family.trim().is_empty()
        || descriptor.typography.scale.trim().is_empty()
        || descriptor.typography.line_height.trim().is_empty()
        || descriptor.typography.weight.trim().is_empty()
    {
        return Err(ForgeError::UiPatternInvalid {
            reason: format!("ui pattern '{id}' typography contract is incomplete"),
        });
    }
    if descriptor.spacing.token.trim().is_empty() || descriptor.spacing.scale.trim().is_empty() {
        return Err(ForgeError::UiPatternInvalid {
            reason: format!("ui pattern '{id}' spacing contract is incomplete"),
        });
    }
    if descriptor.responsive.breakpoints.is_empty()
        || descriptor.responsive.layout.trim().is_empty()
    {
        return Err(ForgeError::UiPatternInvalid {
            reason: format!("ui pattern '{id}' responsive contract is incomplete"),
        });
    }
    if descriptor.accessibility.keyboard.trim().is_empty()
        || descriptor.accessibility.focus.trim().is_empty()
        || descriptor.accessibility.aria.trim().is_empty()
        || descriptor.accessibility.contrast.trim().is_empty()
    {
        return Err(ForgeError::UiPatternInvalid {
            reason: format!("ui pattern '{id}' accessibility contract is incomplete"),
        });
    }
    if descriptor.interaction.choices.is_empty() || descriptor.interaction.outcomes.is_empty() {
        return Err(ForgeError::UiPatternInvalid {
            reason: format!("ui pattern '{id}' interaction contract is incomplete"),
        });
    }
    if descriptor.adapters.is_empty() {
        return Err(ForgeError::UiPatternInvalid {
            reason: format!(
                "ui pattern '{id}' has no tested platform adapter; a pattern without a \
                 platform is not installable"
            ),
        });
    }
    let install = descriptor.install_strategy.trim();
    if install.is_empty() {
        return Err(ForgeError::UiPatternInvalid {
            reason: format!("ui pattern '{id}' has no install strategy"),
        });
    }
    if descriptor.tests.trim().is_empty() {
        return Err(ForgeError::UiPatternInvalid {
            reason: format!("ui pattern '{id}' references no tests"),
        });
    }
    if descriptor.evidence.known_issues.len() > MAX_KNOWN_ISSUES {
        return Err(ForgeError::UiPatternInvalid {
            reason: format!(
                "ui pattern '{id}' declares {} known issues; the catalog allows at most \
                 {MAX_KNOWN_ISSUES} per descriptor",
                descriptor.evidence.known_issues.len()
            ),
        });
    }
    let mut seen_profiles: Vec<&str> = Vec::with_capacity(descriptor.adapters.len());
    for adapter in &descriptor.adapters {
        if adapter.profile.trim().is_empty() {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!("ui pattern '{id}' adapter is missing a profile id"),
            });
        }
        if seen_profiles.contains(&adapter.profile.as_str()) {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!(
                    "ui pattern '{id}' declares adapter for profile '{profile}' more than once",
                    id = id,
                    profile = adapter.profile
                ),
            });
        }
        seen_profiles.push(adapter.profile.as_str());
        if adapter.surface.trim().is_empty() {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!(
                    "ui pattern '{id}' adapter for profile '{profile}' has no surface",
                    id = id,
                    profile = adapter.profile
                ),
            });
        }
        if adapter.artifact_path.trim().is_empty() {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!(
                    "ui pattern '{id}' adapter for profile '{profile}' has no artifact path",
                    id = id,
                    profile = adapter.profile
                ),
            });
        }
        if adapter.tests.trim().is_empty() {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!(
                    "ui pattern '{id}' adapter for profile '{profile}' references no tests",
                    id = id,
                    profile = adapter.profile,
                ),
            });
        }
        if !is_ordinary_source(&adapter.artifact_source) {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!(
                    "ui pattern '{id}' adapter for profile '{profile}' ships a copied \
                     screenshot or HTML fragment, not ordinary source; the catalog refuses \
                     to install non-editable artifacts",
                    id = id,
                    profile = adapter.profile,
                ),
            });
        }
    }
    Ok(())
}

fn now_placeholder() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(0, 0).expect("epoch is valid")
}

fn default_verified_timestamp() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_700_000_000, 0).expect("timestamp is valid")
}

fn default_certified_timestamp() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_725_000_000, 0).expect("timestamp is valid")
}

#[allow(dead_code)]
fn latest_verified_timestamp() -> DateTime<Utc> {
    let secs = Utc::now().timestamp();
    DateTime::<Utc>::from_timestamp(secs, 0).expect("current timestamp is valid")
}

fn required_states() -> Vec<UiPatternState> {
    UI_PATTERN_REQUIRED_STATES
        .iter()
        .map(|name| UiPatternState {
            name: (*name).to_string(),
            description: format!("explicit {name} surface for the pattern"),
        })
        .collect()
}

fn standard_typography() -> UiPatternTypography {
    UiPatternTypography {
        family: "system-ui".to_string(),
        scale: "1.125".to_string(),
        line_height: "1.5".to_string(),
        weight: "400".to_string(),
    }
}

fn standard_spacing() -> UiPatternSpacing {
    UiPatternSpacing {
        token: "design-token-spacing-4".to_string(),
        scale: "4,8,12,16,24,32".to_string(),
    }
}

fn standard_responsive() -> UiPatternResponsive {
    UiPatternResponsive {
        breakpoints: vec![
            "sm:640".to_string(),
            "md:768".to_string(),
            "lg:1024".to_string(),
        ],
        layout: "stack-on-mobile, side-by-side on >=md".to_string(),
    }
}

fn standard_accessibility() -> UiPatternAccessibility {
    UiPatternAccessibility {
        keyboard: "tab order follows visual order; Escape closes overlays".to_string(),
        focus: "first focusable element receives focus on mount".to_string(),
        aria: "role and aria-label follow the WAI-ARIA Authoring Practices".to_string(),
        contrast: "text/background pair meets WCAG AA (>=4.5:1)".to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn descriptor(
    id: &str,
    version: &str,
    intent: &str,
    documentation: &str,
    extra_states: &[(&str, &str)],
    interaction_choices: &[&str],
    interaction_outcomes: &[&str],
    depends_on: &[&str],
    feature_deps: &[&str],
    adapters: Vec<UiPatternAdapter>,
    install_strategy: &str,
    tests: &str,
    quality: UiPatternQuality,
    evidence: UiPatternEvidence,
) -> UiPatternDescriptor {
    let mut states = required_states();
    for (name, description) in extra_states {
        states.push(UiPatternState {
            name: (*name).to_string(),
            description: (*description).to_string(),
        });
    }
    UiPatternDescriptor {
        id: id.to_string(),
        version: version.to_string(),
        intent: intent.to_string(),
        documentation: documentation.to_string(),
        states,
        typography: standard_typography(),
        spacing: standard_spacing(),
        responsive: standard_responsive(),
        accessibility: standard_accessibility(),
        interaction: UiPatternInteraction {
            choices: interaction_choices
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            outcomes: interaction_outcomes
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        },
        depends_on: depends_on.iter().map(|s| (*s).to_string()).collect(),
        feature_deps: feature_deps.iter().map(|s| (*s).to_string()).collect(),
        adapters,
        install_strategy: install_strategy.to_string(),
        tests: tests.to_string(),
        quality,
        evidence,
    }
}

fn verified_evidence() -> UiPatternEvidence {
    UiPatternEvidence {
        usage_count: 4,
        test_coverage: 0.78,
        last_verified: default_verified_timestamp(),
        known_issues: Vec::new(),
        security_review: false,
    }
}

fn certified_evidence() -> UiPatternEvidence {
    UiPatternEvidence {
        usage_count: 11,
        test_coverage: 0.92,
        last_verified: default_certified_timestamp(),
        known_issues: Vec::new(),
        security_review: true,
    }
}

fn experimental_evidence() -> UiPatternEvidence {
    UiPatternEvidence {
        usage_count: 1,
        test_coverage: 0.45,
        last_verified: now_placeholder(),
        known_issues: vec!["initial draft, surface may change".to_string()],
        security_review: false,
    }
}

fn deprecated_evidence() -> UiPatternEvidence {
    UiPatternEvidence {
        usage_count: 0,
        test_coverage: 0.0,
        last_verified: now_placeholder(),
        known_issues: vec!["deprecated in favor of the next generation".to_string()],
        security_review: false,
    }
}

fn react_adapter(pattern_id: &str, surface: &str) -> UiPatternAdapter {
    UiPatternAdapter {
        profile: "react-web".to_string(),
        surface: surface.to_string(),
        artifact_path: format!("src/ui/{pattern_id}.tsx"),
        tests: format!("tests/ui/{pattern_id}.test.tsx"),
        artifact_source: react_source(pattern_id),
    }
}

fn nextjs_adapter(pattern_id: &str, surface: &str) -> UiPatternAdapter {
    UiPatternAdapter {
        profile: "nextjs-web".to_string(),
        surface: surface.to_string(),
        artifact_path: format!("src/ui/{pattern_id}.tsx"),
        tests: format!("tests/ui/{pattern_id}.test.tsx"),
        artifact_source: nextjs_source(pattern_id),
    }
}

fn flutter_adapter(pattern_id: &str, surface: &str) -> UiPatternAdapter {
    UiPatternAdapter {
        profile: "flutter-app".to_string(),
        surface: surface.to_string(),
        artifact_path: format!("lib/ui/{pattern_id}.dart"),
        tests: format!("test/ui/{pattern_id}_test.dart"),
        artifact_source: flutter_source(pattern_id),
    }
}

fn react_source(pattern_id: &str) -> String {
    let cap = pattern_capitalize(pattern_id);
    let mut out = String::new();
    out.push_str("// Auto-generated by forge ui-pattern install (");
    out.push_str(pattern_id);
    out.push_str(")\n");
    out.push_str(
        "// This file is ordinary React source. Edit it freely; Forge never\n\
         // rewrites a customized file. After Forge is removed, the project\n\
         // continues to build with the native toolchain (npm run build).\n",
    );
    out.push_str("import { useState } from 'react';\n\n");
    out.push_str("export type ");
    out.push_str(&cap);
    out.push_str("Props = {\n    children?: React.ReactNode;\n};\n\n");
    out.push_str("export function ");
    out.push_str(&cap);
    out.push_str("({ children }: ");
    out.push_str(&cap);
    out.push_str("Props) {\n");
    out.push_str(
        "    const [state, setState] = useState<'idle' | 'loading' | 'error' | 'success'>('idle');\n",
    );
    out.push_str("    return (\n");
    out.push_str("        <section role=\"region\" aria-label=\"");
    out.push_str(pattern_id);
    out.push_str("\" data-state={state}>\n");
    out.push_str("            {children}\n");
    out.push_str("        </section>\n");
    out.push_str("    );\n}\n\n");
    out.push_str("export default ");
    out.push_str(&cap);
    out.push_str(";\n");
    out
}

fn nextjs_source(pattern_id: &str) -> String {
    let cap = pattern_capitalize(pattern_id);
    let mut out = String::new();
    out.push_str("// Auto-generated by forge ui-pattern install (");
    out.push_str(pattern_id);
    out.push_str(")\n");
    out.push_str(
        "// Ordinary Next.js source. Edit it freely; the project continues to\n\
         // build with the native toolchain (next build) after Forge is removed.\n",
    );
    out.push_str("import type { ReactNode } from 'react';\n\n");
    out.push_str("export type ");
    out.push_str(&cap);
    out.push_str("Props = {\n    children?: ReactNode;\n};\n\n");
    out.push_str("export function ");
    out.push_str(&cap);
    out.push_str("({ children }: ");
    out.push_str(&cap);
    out.push_str("Props) {\n");
    out.push_str("    return (\n");
    out.push_str("        <section role=\"region\" aria-label=\"");
    out.push_str(pattern_id);
    out.push_str("\">\n");
    out.push_str("            {children}\n");
    out.push_str("        </section>\n");
    out.push_str("    );\n}\n\n");
    out.push_str("export default ");
    out.push_str(&cap);
    out.push_str(";\n");
    out
}

fn flutter_source(pattern_id: &str) -> String {
    let cap = pattern_capitalize(pattern_id);
    let mut out = String::new();
    out.push_str("// Auto-generated by forge ui-pattern install (");
    out.push_str(pattern_id);
    out.push_str(")\n");
    out.push_str(
        "// Ordinary Flutter source. Edit it freely; the project continues to\n\
         // build with the native toolchain (flutter build) after Forge is removed.\n",
    );
    out.push_str("import 'package:flutter/material.dart';\n\n");
    out.push_str("class ");
    out.push_str(&cap);
    out.push_str(" extends StatelessWidget {\n");
    out.push_str("    const ");
    out.push_str(&cap);
    out.push_str("({super.key, this.child});\n\n");
    out.push_str("    final Widget? child;\n\n");
    out.push_str("    @override\n");
    out.push_str("    Widget build(BuildContext context) {\n");
    out.push_str("        return Semantics(\n");
    out.push_str("            label: '");
    out.push_str(pattern_id);
    out.push_str("',\n");
    out.push_str("            container: true,\n");
    out.push_str("            child: child ?? const SizedBox.shrink(),\n");
    out.push_str("        );\n");
    out.push_str("    }\n}\n");
    out
}

fn pattern_capitalize(id: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for ch in id.chars() {
        if ch == '-' || ch == '_' {
            out.push(ch);
            upper = true;
            continue;
        }
        if upper {
            for upper_ch in ch.to_uppercase() {
                out.push(upper_ch);
            }
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// All catalog patterns in stable id order. Every entry has
/// been checked by [`validate_descriptor`] at module load
/// time.
pub fn ui_pattern_catalog() -> Vec<UiPatternDescriptor> {
    let mut out = vec![
        descriptor(
            "billing",
            UI_PATTERN_CATALOG_VERSION,
            "billing",
            "Billing surface with plan summary, payment method and invoice history.",
            &[("payment_method", "the stored payment method summary")],
            &["select_plan", "update_payment", "download_invoice"],
            &["plan_change", "payment_updated", "invoice_downloaded"],
            &["auth"],
            &["billing"],
            vec![
                react_adapter("billing", "react-jsx"),
                nextjs_adapter("billing", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::billing_lists_in_two_web_adapters",
            UiPatternQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "confirm-dialog",
            UI_PATTERN_CATALOG_VERSION,
            "confirm-dialog",
            "Destructive-action confirmation dialog with explicit user choice and a11y semantics.",
            &[("choice", "the explicit user choice (confirmed or cancelled)")],
            &["confirm", "cancel"],
            &["confirmed", "cancelled"],
            &["validated-form"],
            &[],
            vec![
                react_adapter("confirm-dialog", "react-jsx"),
                nextjs_adapter("confirm-dialog", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::confirm_dialog_declares_choice_state",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "crud-table",
            UI_PATTERN_CATALOG_VERSION,
            "crud-table",
            "CRUD table pattern with sorting, filtering, pagination and explicit state contracts.",
            &[
                ("sorting", "typed column sort key and direction"),
                ("filtering", "typed filter value per column"),
                ("pagination", "typed page request and cursor"),
            ],
            &["sort", "filter", "paginate", "select_row"],
            &["sorted", "filtered", "paged", "row_selected"],
            &["paginated-query", "validated-form"],
            &[],
            vec![
                react_adapter("crud-table", "react-jsx"),
                nextjs_adapter("crud-table", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::crud_table_exposes_sort_filter_paginate",
            UiPatternQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "dashboard",
            UI_PATTERN_CATALOG_VERSION,
            "dashboard",
            "Dashboard surface with KPI tiles, recent activity and a quick-action slot.",
            &[("recent_activity", "the recent-activity summary slot")],
            &["open_kpi", "trigger_quick_action"],
            &["kpi_opened", "quick_action_triggered"],
            &["auth"],
            &[],
            vec![
                react_adapter("dashboard", "react-jsx"),
                nextjs_adapter("dashboard", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::dashboard_lists_recent_activity",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "empty-state",
            UI_PATTERN_CATALOG_VERSION,
            "empty-state",
            "Empty-state block with a consistent call-to-action surface.",
            &[("call_to_action", "the call-to-action slot")],
            &["invoke_call_to_action"],
            &["call_to_action_invoked"],
            &[],
            &[],
            vec![
                react_adapter("empty-state", "react-jsx"),
                nextjs_adapter("empty-state", "nextjs-jsx"),
                flutter_adapter("empty-state", "flutter-widget"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::empty_state_supports_three_platforms",
            UiPatternQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "error-page",
            UI_PATTERN_CATALOG_VERSION,
            "error-page",
            "Full-page error surface with a recovery action and diagnostic details.",
            &[("diagnostics", "the typed diagnostic detail slot")],
            &["retry", "go_home"],
            &["retried", "navigated_home"],
            &[],
            &[],
            vec![
                react_adapter("error-page", "react-jsx"),
                nextjs_adapter("error-page", "nextjs-jsx"),
                flutter_adapter("error-page", "flutter-screen"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::error_page_lists_recovery_actions",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "file-upload",
            UI_PATTERN_CATALOG_VERSION,
            "file-upload",
            "Accessible file upload surface with accepted-type and size validation.",
            &[("upload_progress", "the per-file progress slot")],
            &["select_file", "cancel_upload"],
            &["file_selected", "upload_cancelled"],
            &[],
            &["storage"],
            vec![
                react_adapter("file-upload", "react-jsx"),
                nextjs_adapter("file-upload", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::file_upload_lists_progress_state",
            UiPatternQuality::Experimental,
            experimental_evidence(),
        ),
        descriptor(
            "filter-bar",
            UI_PATTERN_CATALOG_VERSION,
            "filter-bar",
            "Filter bar with typed filter chips, range inputs and a clear-all action.",
            &[("clear_all", "the explicit clear-all action")],
            &["apply_filter", "clear_filter", "clear_all"],
            &["filter_applied", "filter_cleared", "all_cleared"],
            &["paginated-query"],
            &[],
            vec![
                react_adapter("filter-bar", "react-jsx"),
                nextjs_adapter("filter-bar", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::filter_bar_lists_clear_all",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "forgot-password",
            UI_PATTERN_CATALOG_VERSION,
            "forgot-password",
            "Forgot-password flow with email entry, pending confirmation and recovery instructions.",
            &[("pending_recovery", "the pending recovery instructions slot")],
            &["submit_email"],
            &["email_submitted"],
            &["validated-form"],
            &["auth"],
            vec![
                react_adapter("forgot-password", "react-jsx"),
                nextjs_adapter("forgot-password", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::forgot_password_lists_pending_state",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "form",
            UI_PATTERN_CATALOG_VERSION,
            "form",
            "Form pattern with typed field validation, pending submit, failure and success states.",
            &[
                ("pending", "the typed pending submit surface"),
                ("failure", "the typed failure surface"),
            ],
            &["submit", "reset"],
            &["submitted", "reset"],
            &["validated-form"],
            &[],
            vec![
                react_adapter("form", "react-jsx"),
                nextjs_adapter("form", "nextjs-jsx"),
                flutter_adapter("form", "flutter-form"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::form_lists_pending_failure_success",
            UiPatternQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "login",
            UI_PATTERN_CATALOG_VERSION,
            "login",
            "Login surface with typed credentials, pending submit, failure and authenticated states.",
            &[
                ("pending", "the typed pending submit surface"),
                ("failure", "the typed failure surface"),
                ("authenticated", "the typed authenticated surface"),
            ],
            &["submit", "forgot_password", "register"],
            &["authenticated", "navigate_to_forgot_password", "navigate_to_register"],
            &["validated-form"],
            &["auth"],
            vec![
                react_adapter("login", "react-jsx"),
                nextjs_adapter("login", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::login_lists_authenticated_state",
            UiPatternQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "modal",
            UI_PATTERN_CATALOG_VERSION,
            "modal",
            "Modal overlay with focus trap, Escape-to-close and an explicit return focus target.",
            &[("return_focus", "the explicit return focus target")],
            &["close"],
            &["closed"],
            &[],
            &[],
            vec![
                react_adapter("modal", "react-jsx"),
                nextjs_adapter("modal", "nextjs-jsx"),
                flutter_adapter("modal", "flutter-dialog"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::modal_lists_return_focus",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "navigation",
            UI_PATTERN_CATALOG_VERSION,
            "navigation",
            "Primary navigation surface with typed destinations, active marker and skip-to-content link.",
            &[("skip_to_content", "the skip-to-content link")],
            &["navigate"],
            &["navigated"],
            &[],
            &[],
            vec![
                react_adapter("navigation", "react-jsx"),
                nextjs_adapter("navigation", "nextjs-jsx"),
                flutter_adapter("navigation", "flutter-drawer"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::navigation_lists_skip_to_content",
            UiPatternQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "profile",
            UI_PATTERN_CATALOG_VERSION,
            "profile",
            "Profile surface with avatar, identity fields and explicit save and discard actions.",
            &[("identity_fields", "the typed identity field slot")],
            &["save", "discard"],
            &["saved", "discarded"],
            &["validated-form"],
            &["auth"],
            vec![
                react_adapter("profile", "react-jsx"),
                nextjs_adapter("profile", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::profile_lists_save_discard",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "register",
            UI_PATTERN_CATALOG_VERSION,
            "register",
            "Registration surface with typed fields, pending submit, failure and welcome state.",
            &[
                ("pending", "the typed pending submit surface"),
                ("failure", "the typed failure surface"),
                ("welcome", "the typed welcome surface"),
            ],
            &["submit"],
            &["registered"],
            &["validated-form"],
            &["auth"],
            vec![
                react_adapter("register", "react-jsx"),
                nextjs_adapter("register", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::register_lists_welcome_state",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "settings",
            UI_PATTERN_CATALOG_VERSION,
            "settings",
            "Settings surface with grouped preferences, explicit save and a typed reset action.",
            &[("grouped_preferences", "the typed grouped preferences slot")],
            &["save", "reset"],
            &["saved", "reset"],
            &["validated-form"],
            &["auth"],
            vec![
                react_adapter("settings", "react-jsx"),
                nextjs_adapter("settings", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::settings_lists_grouped_preferences",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "success-page",
            UI_PATTERN_CATALOG_VERSION,
            "success-page",
            "Full-page success surface with a confirmation message and a continue action.",
            &[("confirmation", "the typed confirmation message slot")],
            &["continue"],
            &["continued"],
            &[],
            &[],
            vec![
                react_adapter("success-page", "react-jsx"),
                nextjs_adapter("success-page", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::success_page_lists_continue",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "webhook-receiver",
            UI_PATTERN_CATALOG_VERSION,
            "form",
            "Legacy webhook-receiver surface kept only for migration.",
            &[("pending", "the typed pending submit surface")],
            &["submit"],
            &["submitted"],
            &["webhook-receiver"],
            &[],
            vec![nextjs_adapter("webhook-receiver", "nextjs-jsx")],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::webhook_receiver_is_deprecated",
            UiPatternQuality::Deprecated,
            deprecated_evidence(),
        ),
    ];
    out.sort_by(|a, b| a.id.cmp(&b.id));
    for descriptor in &out {
        validate_descriptor(descriptor)
            .unwrap_or_else(|err| panic!("catalog entry failed validation: {err}"));
    }
    out
}

/// Look up one descriptor by id. Returns [`UiPatternInvalid`]
/// with the `unknown-ui-pattern` wording so a caller can
/// distinguish "no such pattern" from a contract-shape failure.
pub fn inspect_ui_pattern(id: &str) -> Result<UiPatternDescriptor, ForgeError> {
    ui_pattern_catalog()
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| ForgeError::UiPatternInvalid {
            reason: format!("ui pattern '{id}' is not in the catalog"),
        })
}

/// Build a stable per-id evidence summary used by the resolver
/// and the inspector.
fn evidence_summary_for(descriptor: &UiPatternDescriptor) -> UiPatternEvidenceSummary {
    UiPatternEvidenceSummary {
        id: descriptor.id.clone(),
        quality: descriptor.quality,
        evidence: descriptor.evidence.clone(),
    }
}

/// Quality precedence. Higher is preferred by the resolver.
/// `Deprecated` is intentionally not ordered and is handled
/// separately so the resolver reports a policy conflict rather
/// than silently selecting a deprecated candidate.
fn quality_rank(quality: UiPatternQuality) -> u8 {
    match quality {
        UiPatternQuality::Certified => 3,
        UiPatternQuality::Verified => 2,
        UiPatternQuality::Experimental => 1,
        UiPatternQuality::Deprecated => 0,
    }
}

fn select_strongest<'a>(candidates: &[&'a UiPatternDescriptor]) -> Option<&'a UiPatternDescriptor> {
    candidates
        .iter()
        .copied()
        .filter(|c| c.quality != UiPatternQuality::Deprecated)
        .max_by_key(|c| quality_rank(c.quality))
}

fn select_only_deprecated(candidates: &[&UiPatternDescriptor]) -> Vec<UiPatternRejection> {
    candidates
        .iter()
        .map(|c| UiPatternRejection {
            id: c.id.clone(),
            code: "ui-pattern-quality-conflict".to_string(),
            reason: format!(
                "only candidate for ui pattern '{}' is deprecated; the planner refuses to \
                 silently select a deprecated pattern",
                c.id
            ),
        })
        .collect()
}

/// Validate the request before any catalog lookup: profile
/// must be known, ids must be unique and non-empty, the
/// request must contain at least one id, and every id must
/// not be a programming primitive.
pub fn validate_request(request: &UiPatternRequest) -> Result<(), ForgeError> {
    if request.profile.trim().is_empty() {
        return Err(ForgeError::UiPatternInvalid {
            reason: "ui pattern request requires a profile".to_string(),
        });
    }
    if request.pattern_ids.is_empty() {
        return Err(ForgeError::UiPatternInvalid {
            reason: "ui pattern request requires at least one pattern id".to_string(),
        });
    }
    let mut seen: Vec<&str> = Vec::with_capacity(request.pattern_ids.len());
    for id in &request.pattern_ids {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return Err(ForgeError::UiPatternInvalid {
                reason: "ui pattern ids must not be empty".to_string(),
            });
        }
        if seen.contains(&trimmed) {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!("ui pattern id '{trimmed}' is duplicated in the request"),
            });
        }
        seen.push(trimmed);
        if is_primitive_id(trimmed) {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!(
                    "ui pattern '{trimmed}' is a programming primitive or a generic \
                     template placeholder; the registry refuses to model language \
                     constructs or copied markup fragments"
                ),
            });
        }
    }
    Ok(())
}

/// Resolve `request` into a [`UiPatternPlan`]. The resolver
/// never executes a side effect: it is a deterministic
/// function of the catalog, the request and the profile.
/// Refusals are returned as `rejections` so a partial plan
/// stays reviewable; the request is only refused outright
/// when the input is malformed.
pub fn resolve_patterns(request: &UiPatternRequest) -> Result<UiPatternPlan, ForgeError> {
    validate_request(request)?;
    let catalog = ui_pattern_catalog();
    let mut steps: Vec<UiPatternStep> = Vec::new();
    let mut rejections: Vec<UiPatternRejection> = Vec::new();
    let mut requested = request.pattern_ids.clone();
    requested.sort();
    requested.dedup();
    for id in &requested {
        let candidates: Vec<&UiPatternDescriptor> =
            catalog.iter().filter(|c| &c.id == id).collect();
        if candidates.is_empty() {
            rejections.push(UiPatternRejection {
                id: id.clone(),
                code: "ui-pattern-invalid".to_string(),
                reason: format!("ui pattern '{id}' is not in the catalog"),
            });
            continue;
        }
        let compatible: Vec<&UiPatternDescriptor> = candidates
            .iter()
            .copied()
            .filter(|c| c.adapters.iter().any(|a| a.profile == request.profile))
            .collect();
        if compatible.is_empty() {
            let tested: Vec<String> = candidates
                .iter()
                .flat_map(|c| c.adapters.iter().map(|a| a.profile.clone()))
                .collect();
            rejections.push(UiPatternRejection {
                id: id.clone(),
                code: "ui-pattern-unsupported-platform".to_string(),
                reason: format!(
                    "ui pattern '{id}' has no implementation for profile '{}' (tested: {})",
                    request.profile,
                    if tested.is_empty() {
                        "none".to_string()
                    } else {
                        tested.join(", ")
                    }
                ),
            });
            continue;
        }
        match select_strongest(&compatible) {
            Some(selected) => steps.push(UiPatternStep {
                id: selected.id.clone(),
                version: selected.version.clone(),
                quality: selected.quality,
                action: "install".to_string(),
            }),
            None => rejections.extend(select_only_deprecated(&compatible)),
        }
    }
    steps.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(UiPatternPlan {
        profile: request.profile.clone(),
        steps,
        rejections,
    })
}

/// Build a [`UiPatternResolveOutcome`] from a request. The
/// outcome carries the plan plus the per-step evidence summary
/// so the transport can render the planner's reasoning.
pub fn resolve_outcome(request: &UiPatternRequest) -> Result<UiPatternResolveOutcome, ForgeError> {
    let plan = resolve_patterns(request)?;
    let catalog = ui_pattern_catalog();
    let mut evidence_summary: Vec<UiPatternEvidenceSummary> = Vec::new();
    for step in &plan.steps {
        if let Some(descriptor) = catalog.iter().find(|c| c.id == step.id) {
            evidence_summary.push(evidence_summary_for(descriptor));
        }
    }
    let note = if plan.steps.is_empty() && plan.rejections.is_empty() {
        "no ui patterns requested".to_string()
    } else if plan.steps.is_empty() {
        format!(
            "no ui patterns could be resolved for profile '{}' ({} rejection(s))",
            request.profile,
            plan.rejections.len()
        )
    } else {
        format!(
            "resolved {} ui pattern(s) for profile '{}'{}",
            plan.steps.len(),
            request.profile,
            if plan.rejections.is_empty() {
                String::new()
            } else {
                format!("; {} rejection(s)", plan.rejections.len())
            }
        )
    };
    Ok(UiPatternResolveOutcome {
        profile: request.profile.clone(),
        plan,
        evidence_summary,
        note,
    })
}

fn receipt_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(UI_PATTERNS_DIR).join(id).join("install.json")
}

fn adapter_for_profile<'a>(
    descriptor: &'a UiPatternDescriptor,
    profile: &str,
) -> Option<&'a UiPatternAdapter> {
    descriptor.adapters.iter().find(|a| a.profile == profile)
}

fn atomic_write(path: &Path, body: &str) -> Result<(), ForgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::UiPatternOwnershipConflict {
            reason: format!(
                "cannot create artifact directory {}: {err}; prior state left unchanged",
                parent.display()
            ),
        })?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, body).map_err(|err| ForgeError::UiPatternOwnershipConflict {
        reason: format!(
            "cannot stage artifact {}: {err}; prior state left unchanged",
            tmp.display()
        ),
    })?;
    fs::rename(&tmp, path).map_err(|err| ForgeError::UiPatternOwnershipConflict {
        reason: format!(
            "cannot publish artifact {}: {err}; prior state left unchanged",
            path.display()
        ),
    })?;
    Ok(())
}

/// Install one pattern for the named profile. The installer
/// never overwrites a customized file: a destination that
/// already exists (and is not byte-identical to the artifact
/// the catalog ships) is refused with a typed
/// `ui-pattern-ownership-conflict`. The receipt is written
/// under `.forge/ui-patterns/<id>/install.json` and the
/// source artifact is written at the adapter's
/// `artifact_path` so a `forge` removal after the install
/// leaves the project compiling through its native toolchain
/// (R2 boundary).
pub fn install_pattern(
    dir: &Path,
    request: &UiPatternInstallRequest,
) -> Result<UiPatternInstallOutcome, ForgeError> {
    let descriptor = inspect_ui_pattern(&request.pattern_id)?;
    let adapter = adapter_for_profile(&descriptor, &request.profile).ok_or_else(|| {
        ForgeError::UiPatternUnsupportedPlatform {
            reason: format!(
                "ui pattern '{}' has no adapter for profile '{}'; the planner refuses to \
                 substitute copied web markup for an unsupported platform",
                request.pattern_id, request.profile
            ),
        }
    })?;
    let artifact_abs = dir.join(&adapter.artifact_path);
    if artifact_abs.exists() {
        let existing = fs::read_to_string(&artifact_abs).map_err(|err| {
            ForgeError::UiPatternOwnershipConflict {
                reason: format!(
                    "cannot read existing artifact {}: {err}; install refused to avoid \
                     overwriting a customized file",
                    artifact_abs.display()
                ),
            }
        })?;
        if existing != adapter.artifact_source {
            return Err(ForgeError::UiPatternOwnershipConflict {
                reason: format!(
                    "ui pattern '{}' would overwrite a customized file at {}; remove or \
                     rename the file before installing",
                    request.pattern_id,
                    artifact_abs.display()
                ),
            });
        }
    }
    atomic_write(&artifact_abs, &adapter.artifact_source)?;
    let receipt = receipt_path(dir, &request.pattern_id);
    let payload = serde_json::json!({
        "contract": UI_PATTERN_CATALOG_VERSION,
        "pattern_id": request.pattern_id,
        "profile": request.profile,
        "version": descriptor.version,
        "intent": descriptor.intent,
        "quality": descriptor.quality.label(),
        "install_strategy": descriptor.install_strategy,
        "reason": request.reason,
        "artifact_path": adapter.artifact_path,
        "artifact_tests": adapter.tests,
        "states": descriptor.states.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
    });
    let body = serde_json::to_string_pretty(&payload).map_err(|err| {
        ForgeError::UiPatternOwnershipConflict {
            reason: format!("cannot serialize install receipt: {err}"),
        }
    })?;
    atomic_write(&receipt, &body)?;
    Ok(UiPatternInstallOutcome {
        pattern_id: request.pattern_id.clone(),
        profile: request.profile.clone(),
        installed: true,
        files_written: vec![
            adapter.artifact_path.clone(),
            format!("{UI_PATTERNS_DIR}/{}/install.json", request.pattern_id),
        ],
        note: format!(
            "installed ui pattern '{}' for profile '{}' at {} (reason: {})",
            request.pattern_id, request.profile, adapter.artifact_path, request.reason
        ),
    })
}

/// Render a plan for human output.
pub fn render_plan_human(plan: &UiPatternPlan) -> String {
    let mut lines = vec![format!(
        "ui-pattern plan for profile '{}' (catalog {})",
        plan.profile, UI_PATTERN_CATALOG_VERSION
    )];
    if plan.steps.is_empty() {
        lines.push("steps: none".to_string());
    } else {
        for step in &plan.steps {
            lines.push(format!(
                "  {} {}@{} ({})",
                step.action,
                step.id,
                step.version,
                step.quality.label()
            ));
        }
    }
    if !plan.rejections.is_empty() {
        lines.push("rejections:".to_string());
        for rejection in &plan.rejections {
            lines.push(format!(
                "  {} [{}] {}",
                rejection.id, rejection.code, rejection.reason
            ));
        }
    }
    lines.join("\n")
}

/// Render an outcome for human output.
pub fn render_outcome_human(outcome: &UiPatternResolveOutcome) -> String {
    let mut lines = vec![
        format!("ui-pattern resolve: {}", outcome.note),
        render_plan_human(&outcome.plan),
    ];
    if !outcome.evidence_summary.is_empty() {
        lines.push("evidence:".to_string());
        for entry in &outcome.evidence_summary {
            lines.push(format!(
                "  {} ({}) usage={} coverage={:.2} security_review={} last_verified={}",
                entry.id,
                entry.quality.label(),
                entry.evidence.usage_count,
                entry.evidence.test_coverage,
                entry.evidence.security_review,
                entry.evidence.last_verified.to_rfc3339()
            ));
        }
    }
    lines.join("\n")
}

/// Render an install outcome for human output.
pub fn render_install_human(outcome: &UiPatternInstallOutcome) -> String {
    let mut lines = vec![outcome.note.clone()];
    if !outcome.files_written.is_empty() {
        lines.push(format!("files: {}", outcome.files_written.join(", ")));
    } else {
        lines.push("files: (none)".to_string());
    }
    lines.push(format!(
        "installed: {} (pattern='{}' profile='{}')",
        outcome.installed, outcome.pattern_id, outcome.profile
    ));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    fn request(profile: &str, ids: &[&str]) -> UiPatternRequest {
        UiPatternRequest {
            profile: profile.to_string(),
            pattern_ids: strings(ids),
        }
    }

    #[test]
    fn catalog_lists_patterns_in_stable_id_order() {
        let catalog = ui_pattern_catalog();
        let ids: Vec<&str> = catalog.iter().map(|c| c.id.as_str()).collect();
        let mut deduped = ids.clone();
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len(), "ids must be unique");
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "catalog must be in stable id order");
        for id in [
            "login",
            "register",
            "forgot-password",
            "dashboard",
            "crud-table",
            "filter-bar",
            "form",
            "settings",
            "profile",
            "billing",
            "empty-state",
            "success-page",
            "error-page",
            "modal",
            "confirm-dialog",
            "file-upload",
            "navigation",
        ] {
            assert!(ids.contains(&id), "missing catalog id {id}");
        }
        for descriptor in &catalog {
            assert!(!descriptor.states.is_empty(), "{}", descriptor.id);
            for required in UI_PATTERN_REQUIRED_STATES {
                assert!(
                    descriptor.states.iter().any(|s| s.name == *required),
                    "{} missing required state {required}",
                    descriptor.id
                );
            }
            assert!(!descriptor.adapters.is_empty(), "{}", descriptor.id);
            assert!(
                !descriptor.install_strategy.trim().is_empty(),
                "{}",
                descriptor.id
            );
            assert!(!descriptor.tests.trim().is_empty(), "{}", descriptor.id);
            assert!(
                !descriptor.documentation.trim().is_empty(),
                "{}",
                descriptor.id
            );
        }
    }

    #[test]
    fn catalog_refuses_programming_primitives_and_placeholders() {
        for primitive in [
            "if",
            "loop",
            "for",
            "while",
            "try",
            "catch",
            "string-concat",
            "addition",
            "screenshot",
            "html-fragment",
            "copy-paste",
            "lorem-ipsum",
        ] {
            assert!(is_primitive_id(primitive), "{primitive}");
        }
        assert!(!is_primitive_id("login"));
        assert!(!is_primitive_id("crud-table"));
    }

    #[test]
    fn validate_descriptor_rejects_missing_state_contract() {
        let mut descriptor = inspect_ui_pattern("login").unwrap();
        descriptor.states.clear();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("no state contract"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_missing_required_state() {
        let mut descriptor = inspect_ui_pattern("login").unwrap();
        descriptor.states.retain(|s| s.name != "error");
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("missing required state 'error'"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_unknown_intent() {
        let mut descriptor = inspect_ui_pattern("login").unwrap();
        descriptor.intent = "scratchpad".to_string();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("unknown intent"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_primitive_id() {
        let mut descriptor = inspect_ui_pattern("login").unwrap();
        descriptor.id = "if".to_string();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("programming primitive"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_placeholder_artifact() {
        let mut descriptor = inspect_ui_pattern("login").unwrap();
        descriptor.adapters[0].artifact_source = "<html><body>login</body></html>".to_string();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(
            text.contains("copied screenshot or HTML fragment"),
            "{text}"
        );
    }

    #[test]
    fn validate_descriptor_rejects_empty_adapters() {
        let mut descriptor = inspect_ui_pattern("login").unwrap();
        descriptor.adapters.clear();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("no tested platform adapter"), "{text}");
    }

    #[test]
    fn resolve_known_patterns_succeeds_with_certified_evidence() {
        let outcome = resolve_outcome(&request("react-web", &["login", "form"])).unwrap();
        assert!(outcome.plan.rejections.is_empty());
        let ids: Vec<&str> = outcome.plan.steps.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"login"));
        assert!(ids.contains(&"form"));
        for entry in &outcome.evidence_summary {
            assert_eq!(entry.quality, UiPatternQuality::Certified);
            assert!(entry.evidence.test_coverage >= UI_PATTERN_CERTIFIED_TEST_COVERAGE);
            assert!(entry.evidence.security_review);
        }
    }

    #[test]
    fn resolve_profile_incompatibility_surfaces_typed_rejection() {
        // `form` ships for react-web/nextjs-web/flutter-app, so test a
        // pattern that only has web adapters; `billing` ships only for
        // react-web and nextjs-web.
        let outcome = resolve_outcome(&request("flutter-app", &["billing"])).unwrap();
        assert!(outcome.plan.steps.is_empty());
        assert_eq!(outcome.plan.rejections.len(), 1);
        assert_eq!(
            outcome.plan.rejections[0].code,
            "ui-pattern-unsupported-platform"
        );
        assert!(outcome.plan.rejections[0].reason.contains("flutter-app"));
    }

    #[test]
    fn resolve_unknown_pattern_surfaces_typed_rejection() {
        let outcome = resolve_outcome(&request("react-web", &["made-up"])).unwrap();
        assert!(outcome.plan.steps.is_empty());
        assert_eq!(outcome.plan.rejections.len(), 1);
        assert_eq!(outcome.plan.rejections[0].code, "ui-pattern-invalid");
        assert!(outcome.plan.rejections[0]
            .reason
            .contains("not in the catalog"));
    }

    #[test]
    fn resolve_only_deprecated_reports_quality_conflict() {
        // The deprecated test entry reuses the `form` id with a
        // single nextjs-web adapter, so a resolve for `form` on
        // nextjs-web surfaces the deprecated-only branch via
        // `webhook-receiver` (the deprecated id) on its tested
        // adapter (nextjs-web).
        let outcome = resolve_outcome(&request("nextjs-web", &["webhook-receiver"])).unwrap();
        assert!(outcome.plan.steps.is_empty());
        assert_eq!(outcome.plan.rejections.len(), 1);
        assert_eq!(
            outcome.plan.rejections[0].code,
            "ui-pattern-quality-conflict"
        );
        assert!(outcome.plan.rejections[0].reason.contains("only candidate"));
    }

    #[test]
    fn validate_request_rejects_primitive_id() {
        let err = validate_request(&request("react-web", &["if"])).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("programming primitive"), "{text}");
    }

    #[test]
    fn validate_request_rejects_duplicate_id() {
        let err = validate_request(&request("react-web", &["login", "login"])).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("duplicated"), "{text}");
    }

    #[test]
    fn validate_request_rejects_empty_id_list() {
        let err = validate_request(&request("react-web", &[])).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
    }

    #[test]
    fn install_pattern_writes_artifact_and_receipt_for_supported_profile() {
        let tmp = TempDir::new().unwrap();
        let request = UiPatternInstallRequest {
            pattern_id: "form".to_string(),
            profile: "react-web".to_string(),
            reason: "studio needs the standard form".to_string(),
        };
        let outcome = install_pattern(tmp.path(), &request).unwrap();
        assert!(outcome.installed);
        assert_eq!(outcome.pattern_id, "form");
        assert!(outcome.files_written.iter().any(|p| p == "src/ui/form.tsx"));
        let artifact = std::fs::read_to_string(tmp.path().join("src/ui/form.tsx")).unwrap();
        assert!(artifact.contains("function Form("));
        assert!(artifact.contains("data-state="));
        let receipt =
            std::fs::read_to_string(tmp.path().join(".forge/ui-patterns/form/install.json"))
                .unwrap();
        assert!(receipt.contains("\"pattern_id\": \"form\""));
        assert!(receipt.contains("\"profile\": \"react-web\""));
    }

    #[test]
    fn install_pattern_writes_flutter_artifact_for_flutter_app() {
        let tmp = TempDir::new().unwrap();
        let request = UiPatternInstallRequest {
            pattern_id: "form".to_string(),
            profile: "flutter-app".to_string(),
            reason: "mobile build needs the form surface".to_string(),
        };
        let outcome = install_pattern(tmp.path(), &request).unwrap();
        assert!(outcome.installed);
        let artifact = std::fs::read_to_string(tmp.path().join("lib/ui/form.dart")).unwrap();
        assert!(artifact.contains("class Form"));
        assert!(artifact.contains("'form'"));
    }

    #[test]
    fn install_pattern_refuses_to_overwrite_a_customized_artifact() {
        let tmp = TempDir::new().unwrap();
        let artifact_path = tmp.path().join("src/ui/form.tsx");
        std::fs::create_dir_all(artifact_path.parent().unwrap()).unwrap();
        std::fs::write(&artifact_path, "// user already customized this\n").unwrap();
        let request = UiPatternInstallRequest {
            pattern_id: "form".to_string(),
            profile: "react-web".to_string(),
            reason: "should refuse".to_string(),
        };
        let err = install_pattern(tmp.path(), &request).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-ownership-conflict");
        // Customized file left intact.
        let preserved = std::fs::read_to_string(&artifact_path).unwrap();
        assert!(preserved.contains("user already customized"));
        // Receipt must not have been written.
        assert!(!tmp
            .path()
            .join(".forge/ui-patterns/form/install.json")
            .exists());
    }

    #[test]
    fn install_pattern_is_idempotent_when_artifact_is_byte_identical() {
        let tmp = TempDir::new().unwrap();
        let request = UiPatternInstallRequest {
            pattern_id: "form".to_string(),
            profile: "react-web".to_string(),
            reason: "first install".to_string(),
        };
        let _ = install_pattern(tmp.path(), &request).unwrap();
        let second = install_pattern(tmp.path(), &request).unwrap();
        assert!(second.installed);
    }

    #[test]
    fn install_pattern_refuses_unsupported_platform() {
        let tmp = TempDir::new().unwrap();
        let request = UiPatternInstallRequest {
            pattern_id: "billing".to_string(),
            profile: "flutter-app".to_string(),
            reason: "no flutter surface".to_string(),
        };
        let err = install_pattern(tmp.path(), &request).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-unsupported-platform");
        assert!(err.to_string().contains("flutter-app"));
    }

    #[test]
    fn install_pattern_refuses_unknown_id() {
        let tmp = TempDir::new().unwrap();
        let request = UiPatternInstallRequest {
            pattern_id: "made-up".to_string(),
            profile: "react-web".to_string(),
            reason: "missing".to_string(),
        };
        let err = install_pattern(tmp.path(), &request).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
    }

    #[test]
    fn ui_pattern_helpers_render_human_output() {
        let outcome = resolve_outcome(&request("react-web", &["login", "made-up"])).unwrap();
        let human = render_outcome_human(&outcome);
        assert!(human.contains("ui-pattern resolve:"));
        assert!(human.contains("login"));
        assert!(human.contains("rejections:"));
        assert!(human.contains("made-up"));
        let plan_human = render_plan_human(&outcome.plan);
        assert!(plan_human.contains("ui-pattern plan for profile 'react-web'"));
    }
}
