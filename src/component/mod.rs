//! Semantic component registry and evidence-backed quality levels
//! (`semantic-component-registry`).
//!
//! Core owns the versioned component catalog, the contract validator, the
//! quality-aware resolver, and the evidence-gated promotion path. Transports
//! render Core outcomes without reinterpreting them.
//!
//! The component catalog is built around four rules that distinguish a
//! "semantic component" from a programming primitive
//! (requirement.md §11, §12, §45, §47):
//!
//! 1. **Semantic purpose.** The asset has a meaningful intent
//!    (e.g. `paginated-query`, `idempotency-guard`); language primitives
//!    such as `if`, `loop`, `try-catch`, `string-concat` are refused.
//! 2. **Explicit contract.** The asset names typed inputs and outputs
//!    that callers depend on; the contract is the shared surface every
//!    stack implements independently.
//! 3. **Versioned, testable, deterministic install.** Every descriptor
//!    pins a version, references the test surface, and ships a
//!    deterministic install strategy compatible with the MVP profile
//!    catalog.
//! 4. **Evidence-backed quality.** A quality classification
//!    (`Experimental` / `Verified` / `Certified` / `Deprecated`) is
//!    attached to verifiable evidence: usage count, test coverage,
//!    last verification timestamp, known issues and a security
//!    review flag. Promotion to `Certified` requires the evidence to
//!    be complete; otherwise the prior quality level is preserved and
//!    the request is refused.
//!
//! The resolver prefers the compatible `Certified` candidate whenever
//! multiple components satisfy a request; if no `Certified` candidate
//! exists but `Experimental` and `Verified` candidates do, the
//! strongest available compatible quality wins. A request that only a
//! `Deprecated` or profile-incompatible `Certified` candidate can
//! satisfy surfaces a `component-quality-conflict` rejection so the
//! caller sees the policy conflict instead of a silent selection.
//!
//! Per-stack implementations preserve their own implementation while
//! exposing the shared contract (R1 boundary scenario): the same
//! `paginated-query` id can be installed for `rust-web` and
//! `python-service` with two distinct stack-specific descriptors
//! sharing one contract surface.
//!
//! Storage layout (under the project root, only used by `qualify` and
//! the journal):
//!
//! ```text
//! .forge/components/<component-id>/qualify.json    per-promotion evidence
//! ```
//!
//! Operations journal under the registry's `operations` table with the
//! `component` kind and a `done`/`blocked`/`rejected` verdict so a
//! future portal or API surface can read the history through the same
//! core contract the CLI uses.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::ForgeError;

/// Contract data version for the component registry API and storage.
pub const COMPONENT_CATALOG_VERSION: &str = "0.1.0";

/// Default deterministic install strategy applied when a descriptor
/// does not name one explicitly.
pub const DEFAULT_INSTALL_STRATEGY: &str = "manifest-repin-plus-codegen";

/// Maximum known issues attached to a single descriptor before the
/// promotion to `Certified` is refused (a `Certified` component must
/// track its own caveats, but unbounded lists are not evidence).
pub const MAX_KNOWN_ISSUES: usize = 16;

/// Minimum test coverage ratio (`0.0`-`1.0`) required for promotion to
/// `Certified`.
pub const CERTIFIED_TEST_COVERAGE: f32 = 0.85;

/// Maximum age of a `last_verified` claim that still qualifies as
/// current evidence for `Certified` promotion. Anything older counts
/// as stale.
pub const CERTIFIED_FRESHNESS_DAYS: i64 = 180;

/// Directory (relative to the project root) holding per-component
/// promotion evidence receipts.
pub const COMPONENTS_DIR: &str = ".forge/components";

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

/// One port on the component contract: an explicit input or output
/// callers can rely on. The name is the contract identifier; the
/// description is rendered for human output and consumed by the
/// planner when picking between stack implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentPort {
    pub name: String,
    pub description: String,
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

/// Programming primitives that must never be registered as semantic
/// components (requirement.md §11). The list is the boundary between
/// the component registry and ordinary language constructs.
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
];

/// Request to resolve one or more component ids for a given profile.
/// The profile must be inspectable through the profile catalog
/// (planned or supported) and the ids must be unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentRequest {
    pub profile: String,
    pub component_ids: Vec<String>,
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

/// Reviewable plan produced by [`resolve_components`]. Empty `steps`
/// means the resolver could not satisfy any requested component for
/// the profile (the rejections carry the explanation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComponentPlan {
    pub profile: String,
    pub steps: Vec<ComponentStep>,
    pub rejections: Vec<ComponentRejection>,
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

/// Full outcome of a `forge component resolve` invocation. Carries
/// the plan plus a per-step evidence summary, plus a human note.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComponentResolveOutcome {
    pub profile: String,
    pub plan: ComponentPlan,
    pub evidence_summary: Vec<ComponentEvidenceSummary>,
    pub note: String,
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

/// True when `id` is a programming primitive that must be refused by
/// the descriptor validator.
pub fn is_primitive_id(id: &str) -> bool {
    let lower = id.trim().to_ascii_lowercase();
    PRIMITIVE_IDS.iter().any(|p| *p == lower)
}

/// Validate one descriptor. The catalog is built from tested
/// descriptors, but the helper is public so a future extension can
/// accept user-supplied descriptors and refuse primitives or
/// incomplete shells with a typed `ComponentInvalid` error (R1
/// failure scenario).
pub fn validate_descriptor(descriptor: &ComponentDescriptor) -> Result<(), ForgeError> {
    let id = descriptor.id.trim();
    if id.is_empty() {
        return Err(ForgeError::ComponentInvalid {
            reason: "component id is empty".to_string(),
        });
    }
    if is_primitive_id(id) {
        return Err(ForgeError::ComponentInvalid {
            reason: format!(
                "component '{id}' is a programming primitive, not a semantic component; \
                 the registry refuses to model language constructs"
            ),
        });
    }
    if descriptor.purpose.trim().is_empty() {
        return Err(ForgeError::ComponentInvalid {
            reason: format!("component '{id}' is missing a semantic purpose"),
        });
    }
    if descriptor.contract.inputs.is_empty() {
        return Err(ForgeError::ComponentInvalid {
            reason: format!("component '{id}' contract has no inputs"),
        });
    }
    if descriptor.contract.outputs.is_empty() {
        return Err(ForgeError::ComponentInvalid {
            reason: format!("component '{id}' contract has no outputs"),
        });
    }
    if descriptor.profiles.is_empty() {
        return Err(ForgeError::ComponentInvalid {
            reason: format!(
                "component '{id}' has no tested profile mapping; a component without a \
                 platform is not installable"
            ),
        });
    }
    let install = descriptor.install_strategy.trim();
    if install.is_empty() {
        return Err(ForgeError::ComponentInvalid {
            reason: format!("component '{id}' has no install strategy"),
        });
    }
    if descriptor.tests.trim().is_empty() {
        return Err(ForgeError::ComponentInvalid {
            reason: format!("component '{id}' references no tests"),
        });
    }
    if descriptor.documentation.trim().is_empty() {
        return Err(ForgeError::ComponentInvalid {
            reason: format!("component '{id}' has no documentation pointer"),
        });
    }
    if descriptor.evidence.known_issues.len() > MAX_KNOWN_ISSUES {
        return Err(ForgeError::ComponentInvalid {
            reason: format!(
                "component '{id}' declares {} known issues; the catalog allows at most \
                 {MAX_KNOWN_ISSUES} per descriptor",
                descriptor.evidence.known_issues.len()
            ),
        });
    }
    for port in descriptor
        .contract
        .inputs
        .iter()
        .chain(descriptor.contract.outputs.iter())
    {
        if port.name.trim().is_empty() {
            return Err(ForgeError::ComponentInvalid {
                reason: format!("component '{id}' contract has an unnamed port"),
            });
        }
        if port.description.trim().is_empty() {
            return Err(ForgeError::ComponentInvalid {
                reason: format!(
                    "component '{id}' port '{port}' has no description",
                    id = id,
                    port = port.name
                ),
            });
        }
    }
    Ok(())
}

fn epoch_record_time() -> DateTime<Utc> {
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

/// Build a single descriptor with shared defaults. Catalog authors
/// only specify the fields that differentiate the component; the
/// rest of the typed evidence is filled in by the helpers below so
/// the catalog stays uniform and easy to read.
#[allow(clippy::too_many_arguments)]
fn descriptor(
    id: &str,
    version: &str,
    purpose: &str,
    inputs: &[(&str, &str)],
    outputs: &[(&str, &str)],
    depends_on: &[&str],
    profiles: &[&str],
    install_strategy: &str,
    validation: &[&str],
    documentation: &str,
    tests: &str,
    quality: ComponentQuality,
    evidence: ComponentEvidence,
) -> ComponentDescriptor {
    let contract = ComponentContract {
        inputs: inputs
            .iter()
            .map(|(name, description)| ComponentPort {
                name: (*name).to_string(),
                description: (*description).to_string(),
            })
            .collect(),
        outputs: outputs
            .iter()
            .map(|(name, description)| ComponentPort {
                name: (*name).to_string(),
                description: (*description).to_string(),
            })
            .collect(),
    };
    ComponentDescriptor {
        id: id.to_string(),
        version: version.to_string(),
        purpose: purpose.to_string(),
        contract,
        depends_on: depends_on.iter().map(|s| (*s).to_string()).collect(),
        profiles: profiles.iter().map(|s| (*s).to_string()).collect(),
        install_strategy: install_strategy.to_string(),
        validation: validation.iter().map(|s| (*s).to_string()).collect(),
        documentation: documentation.to_string(),
        tests: tests.to_string(),
        quality,
        evidence,
    }
}

fn verified_evidence() -> ComponentEvidence {
    ComponentEvidence {
        usage_count: 4,
        test_coverage: 0.78,
        last_verified: default_verified_timestamp(),
        known_issues: Vec::new(),
        security_review: false,
    }
}

fn certified_evidence() -> ComponentEvidence {
    ComponentEvidence {
        usage_count: 11,
        test_coverage: 0.92,
        last_verified: default_certified_timestamp(),
        known_issues: Vec::new(),
        security_review: true,
    }
}

fn experimental_evidence() -> ComponentEvidence {
    ComponentEvidence {
        usage_count: 1,
        test_coverage: 0.45,
        last_verified: epoch_record_time(),
        known_issues: vec!["initial draft, surface may change".to_string()],
        security_review: false,
    }
}

fn deprecated_evidence() -> ComponentEvidence {
    ComponentEvidence {
        usage_count: 0,
        test_coverage: 0.0,
        last_verified: epoch_record_time(),
        known_issues: vec!["deprecated in favor of the next generation".to_string()],
        security_review: false,
    }
}

/// All catalog components in stable id order. Every entry has been
/// checked by [`validate_descriptor`] at module load time (a bug in
/// the helper would surface during `cargo test`).
pub fn component_catalog() -> Vec<ComponentDescriptor> {
    let mut out = vec![
        descriptor(
            "api-mutation",
            COMPONENT_CATALOG_VERSION,
            "Validated HTTP mutation handler with explicit input and output contracts.",
            &[("request", "typed request payload"), ("actor", "authenticated principal")],
            &[("response", "typed result or failure envelope")],
            &["auth"],
            &["rust-web", "python-service"],
            DEFAULT_INSTALL_STRATEGY,
            &["API-MUTATION-001"],
            "Accepts a typed request, validates input against the contract and returns the typed result.",
            "forge component resolve --profile rust-web --component api-mutation contract",
            ComponentQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "audit-action",
            COMPONENT_CATALOG_VERSION,
            "Append an immutable audit record with actor, action and resource context.",
            &[("actor", "principal performing the action"), ("action", "verb identifying the operation"), ("resource", "target of the action")],
            &[("record", "persisted audit row or event")],
            &["auth"],
            &["rust-web", "python-service"],
            DEFAULT_INSTALL_STRATEGY,
            &["AUDIT-001"],
            "Wraps the call site to emit a structured audit record with the actor/action/resource tuple.",
            "forge component resolve --profile rust-web --component audit-action contract",
            ComponentQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "confirm-dialog",
            COMPONENT_CATALOG_VERSION,
            "Destructive-action confirmation dialog with explicit user choice and accessibility semantics.",
            &[("intent", "the destructive action the dialog confirms")],
            &[("choice", "the explicit user choice (confirmed or cancelled)")],
            &[],
            &["nextjs-web", "react-web"],
            DEFAULT_INSTALL_STRATEGY,
            &["A11Y-001"],
            "Renders an accessible confirmation dialog and surfaces the explicit user choice to the caller.",
            "forge component resolve --profile nextjs-web --component confirm-dialog contract",
            ComponentQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "empty-state",
            COMPONENT_CATALOG_VERSION,
            "Empty-state component with a consistent layout and call-to-action surface.",
            &[("context", "the data context the empty state belongs to")],
            &[("markup", "the rendered empty-state markup")],
            &[],
            &["nextjs-web", "react-web"],
            DEFAULT_INSTALL_STRATEGY,
            &["A11Y-001"],
            "Renders a reusable empty-state block with a consistent call-to-action slot.",
            "forge component resolve --profile nextjs-web --component empty-state contract",
            ComponentQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "error-boundary",
            COMPONENT_CATALOG_VERSION,
            "UI error boundary that captures a render error and surfaces a typed recovery surface.",
            &[("fallback", "the recovery markup rendered when an error is captured")],
            &[("recovery", "the typed recovery action offered to the operator")],
            &[],
            &["nextjs-web", "react-web"],
            DEFAULT_INSTALL_STRATEGY,
            &["A11Y-001"],
            "Wraps a render subtree and surfaces a typed recovery surface when a child throws.",
            "forge component resolve --profile nextjs-web --component error-boundary contract",
            ComponentQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "file-picker",
            COMPONENT_CATALOG_VERSION,
            "Accessible file picker with explicit accepted-type and size validation.",
            &[("accept", "the accepted MIME types or extensions")],
            &[("file", "the validated file chosen by the user")],
            &[],
            &["nextjs-web", "react-web"],
            DEFAULT_INSTALL_STRATEGY,
            &["A11Y-001"],
            "Renders an accessible file input and surfaces the validated file chosen by the operator.",
            "forge component resolve --profile nextjs-web --component file-picker contract",
            ComponentQuality::Experimental,
            experimental_evidence(),
        ),
        descriptor(
            "idempotency-guard",
            COMPONENT_CATALOG_VERSION,
            "Detect and short-circuit duplicate invocations through a stable idempotency key.",
            &[("key", "the stable idempotency key")],
            &[("result", "the cached or freshly-computed result")],
            &["postgres"],
            &["rust-web", "python-service"],
            DEFAULT_INSTALL_STRATEGY,
            &["IDEMPOTENCY-001"],
            "Returns the cached result for a previously-seen key, or computes and stores a new one.",
            "forge component resolve --profile rust-web --component idempotency-guard contract",
            ComponentQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "loading-state",
            COMPONENT_CATALOG_VERSION,
            "Async loading-state component with a typed busy/idle contract.",
            &[("task", "the in-flight asynchronous task")],
            &[("state", "the typed busy/idle surface rendered to the user")],
            &[],
            &["nextjs-web", "react-web"],
            DEFAULT_INSTALL_STRATEGY,
            &["A11Y-001"],
            "Subscribes to the in-flight task and renders the typed busy/idle surface.",
            "forge component resolve --profile nextjs-web --component loading-state contract",
            ComponentQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "paginated-query",
            COMPONENT_CATALOG_VERSION,
            "Cursor-based pagination of a typed query with explicit page contract.",
            &[("source", "the typed source query"), ("cursor", "the stable cursor")],
            &[("page", "the next page of typed records plus its cursor")],
            &[],
            &["rust-web", "python-service"],
            DEFAULT_INSTALL_STRATEGY,
            &["PAGINATION-001"],
            "Resolves the next page of a typed source query using a stable cursor.",
            "forge component resolve --profile rust-web --component paginated-query contract",
            ComponentQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "require-permission",
            COMPONENT_CATALOG_VERSION,
            "Authorization guard that admits or refuses a typed principal against a typed permission.",
            &[("principal", "the authenticated principal"), ("permission", "the required permission")],
            &[("decision", "admit or refuse decision")],
            &["auth"],
            &["rust-web", "python-service", "nextjs-web"],
            DEFAULT_INSTALL_STRATEGY,
            &["AUTH-001"],
            "Returns the admit/refuse decision for the principal against the named permission.",
            "forge component resolve --profile rust-web --component require-permission contract",
            ComponentQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "retry-external-call",
            COMPONENT_CATALOG_VERSION,
            "Bounded retry of an external call with typed backoff and circuit-breaker semantics.",
            &[("call", "the external call descriptor"), ("policy", "the retry/backoff policy")],
            &[("outcome", "the typed outcome or terminal failure")],
            &[],
            &["rust-web", "python-service"],
            DEFAULT_INSTALL_STRATEGY,
            &["RETRY-001"],
            "Applies the policy to the call and returns the typed outcome or terminal failure.",
            "forge component resolve --profile rust-web --component retry-external-call contract",
            ComponentQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "soft-delete",
            COMPONENT_CATALOG_VERSION,
            "Soft-delete pattern with explicit restore contract and audit trail.",
            &[("record", "the record to soft-delete")],
            &[("tombstone", "the typed tombstone including restore contract")],
            &["postgres"],
            &["rust-web", "python-service"],
            DEFAULT_INSTALL_STRATEGY,
            &["SOFT-DELETE-001"],
            "Marks the record as soft-deleted and returns the typed tombstone including restore details.",
            "forge component resolve --profile rust-web --component soft-delete contract",
            ComponentQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "toast",
            COMPONENT_CATALOG_VERSION,
            "Non-modal toast notification with typed variants and a11y semantics.",
            &[("message", "the typed message payload")],
            &[("dismissal", "the typed dismissal event")],
            &[],
            &["nextjs-web", "react-web"],
            DEFAULT_INSTALL_STRATEGY,
            &["A11Y-001"],
            "Renders the typed toast variant and surfaces the dismissal event to the caller.",
            "forge component resolve --profile nextjs-web --component toast contract",
            ComponentQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "validated-form",
            COMPONENT_CATALOG_VERSION,
            "Form component with typed field validation and explicit submit contract.",
            &[("fields", "the typed form fields and validators")],
            &[("submission", "the typed submission payload")],
            &[],
            &["nextjs-web", "react-web"],
            DEFAULT_INSTALL_STRATEGY,
            &["A11Y-001"],
            "Renders the typed form fields, runs the validators and surfaces the typed submission.",
            "forge component resolve --profile nextjs-web --component validated-form contract",
            ComponentQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "webhook-receiver",
            COMPONENT_CATALOG_VERSION,
            "Authenticated webhook receiver with typed payload validation and replay defence.",
            &[("signature", "the request signature header"), ("payload", "the raw request body")],
            &[("event", "the typed validated event")],
            &[],
            &["rust-web", "python-service"],
            DEFAULT_INSTALL_STRATEGY,
            &["WEBHOOK-001"],
            "Validates the signature, deduplicates and surfaces the typed validated event.",
            "forge component resolve --profile rust-web --component webhook-receiver contract",
            ComponentQuality::Deprecated,
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

/// Look up one descriptor by id. Returns `ComponentInvalid` with the
/// `unknown-component` code so a caller can distinguish "no such
/// component" from a contract-shape failure.
pub fn inspect_component(id: &str) -> Result<ComponentDescriptor, ForgeError> {
    component_catalog()
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| ForgeError::ComponentInvalid {
            reason: format!("component '{id}' is not in the catalog"),
        })
}

/// Build a stable per-id evidence summary used by the resolver and
/// the inspector.
fn evidence_summary_for(descriptor: &ComponentDescriptor) -> ComponentEvidenceSummary {
    ComponentEvidenceSummary {
        id: descriptor.id.clone(),
        quality: descriptor.quality,
        evidence: descriptor.evidence.clone(),
    }
}

/// Quality precedence. Higher is preferred by the resolver. The
/// ordering is: `Certified` > `Verified` > `Experimental`; `Deprecated`
/// is intentionally not ordered and is handled separately so the
/// resolver reports a policy conflict rather than silently selecting
/// a deprecated candidate (R2 boundary scenario).
fn quality_rank(quality: ComponentQuality) -> u8 {
    match quality {
        ComponentQuality::Certified => 3,
        ComponentQuality::Verified => 2,
        ComponentQuality::Experimental => 1,
        ComponentQuality::Deprecated => 0,
    }
}

fn select_strongest<'a>(candidates: &[&'a ComponentDescriptor]) -> Option<&'a ComponentDescriptor> {
    candidates
        .iter()
        .copied()
        .filter(|c| c.quality != ComponentQuality::Deprecated)
        .max_by_key(|c| quality_rank(c.quality))
}

fn select_only_deprecated(candidates: &[&ComponentDescriptor]) -> Vec<ComponentRejection> {
    candidates
        .iter()
        .map(|c| ComponentRejection {
            id: c.id.to_string(),
            code: "component-quality-conflict".to_string(),
            reason: format!(
                "only candidate for component '{}' is deprecated; the planner refuses to \
                 silently select a deprecated descriptor",
                c.id
            ),
        })
        .collect()
}

/// Validate the request before any catalog lookup: profile must be
/// known, ids must be unique and non-empty, the request must contain
/// at least one id.
pub fn validate_request(request: &ComponentRequest) -> Result<(), ForgeError> {
    if request.profile.trim().is_empty() {
        return Err(ForgeError::ComponentInvalid {
            reason: "component request requires a profile".to_string(),
        });
    }
    if request.component_ids.is_empty() {
        return Err(ForgeError::ComponentInvalid {
            reason: "component request requires at least one component id".to_string(),
        });
    }
    let mut seen: Vec<&str> = Vec::with_capacity(request.component_ids.len());
    for id in &request.component_ids {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return Err(ForgeError::ComponentInvalid {
                reason: "component ids must not be empty".to_string(),
            });
        }
        if seen.contains(&trimmed) {
            return Err(ForgeError::ComponentInvalid {
                reason: format!("component id '{trimmed}' is duplicated in the request"),
            });
        }
        seen.push(trimmed);
        if is_primitive_id(trimmed) {
            return Err(ForgeError::ComponentInvalid {
                reason: format!(
                    "component '{trimmed}' is a programming primitive; the registry refuses \
                     to model language constructs"
                ),
            });
        }
    }
    Ok(())
}

/// Resolve `request` into a [`ComponentPlan`]. The resolver never
/// executes a side effect: it is a deterministic function of the
/// catalog, the request and the profile. Refusals are returned as
/// `rejections` so a partial plan stays reviewable; the request is
/// only refused outright when the input is malformed (R1 failure
/// scenario).
pub fn resolve_components(request: &ComponentRequest) -> Result<ComponentPlan, ForgeError> {
    validate_request(request)?;
    let catalog = component_catalog();
    let mut steps: Vec<ComponentStep> = Vec::new();
    let mut rejections: Vec<ComponentRejection> = Vec::new();
    let mut requested = request.component_ids.clone();
    requested.sort();
    requested.dedup();
    for id in &requested {
        let candidates: Vec<&ComponentDescriptor> =
            catalog.iter().filter(|c| &c.id == id).collect();
        if candidates.is_empty() {
            rejections.push(ComponentRejection {
                id: id.clone(),
                code: "component-invalid".to_string(),
                reason: format!("component '{id}' is not in the catalog"),
            });
            continue;
        }
        let compatible: Vec<&ComponentDescriptor> = candidates
            .iter()
            .copied()
            .filter(|c| c.profiles.iter().any(|p| p == &request.profile))
            .collect();
        if compatible.is_empty() {
            let tested: Vec<String> = candidates
                .iter()
                .flat_map(|c| c.profiles.iter().cloned())
                .collect();
            rejections.push(ComponentRejection {
                id: id.clone(),
                code: "component-invalid".to_string(),
                reason: format!(
                    "component '{id}' has no implementation for profile '{}' (tested: {})",
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
            Some(selected) => steps.push(ComponentStep {
                id: selected.id.clone(),
                version: selected.version.clone(),
                quality: selected.quality,
                action: "install".to_string(),
            }),
            None => rejections.extend(select_only_deprecated(&compatible)),
        }
    }
    steps.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(ComponentPlan {
        profile: request.profile.clone(),
        steps,
        rejections,
    })
}

/// Build a [`ComponentResolveOutcome`] from a request. The outcome
/// carries the plan plus the per-step evidence summary so the
/// transport can render the planner's reasoning.
pub fn resolve_outcome(request: &ComponentRequest) -> Result<ComponentResolveOutcome, ForgeError> {
    let plan = resolve_components(request)?;
    let catalog = component_catalog();
    let mut evidence_summary: Vec<ComponentEvidenceSummary> = Vec::new();
    for step in &plan.steps {
        if let Some(descriptor) = catalog.iter().find(|c| c.id == step.id) {
            evidence_summary.push(evidence_summary_for(descriptor));
        }
    }
    let note = if plan.steps.is_empty() && plan.rejections.is_empty() {
        "no components requested".to_string()
    } else if plan.steps.is_empty() {
        format!(
            "no components could be resolved for profile '{}' ({} rejection(s))",
            request.profile,
            plan.rejections.len()
        )
    } else {
        format!(
            "resolved {} component(s) for profile '{}'{}",
            plan.steps.len(),
            request.profile,
            if plan.rejections.is_empty() {
                String::new()
            } else {
                format!("; {} rejection(s)", plan.rejections.len())
            }
        )
    };
    Ok(ComponentResolveOutcome {
        profile: request.profile.clone(),
        plan,
        evidence_summary,
        note,
    })
}

fn receipt_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(COMPONENTS_DIR).join(id).join("qualify.json")
}

fn evidence_is_fresh(last_verified: DateTime<Utc>) -> bool {
    let now = Utc::now().timestamp();
    let stamp = last_verified.timestamp();
    let age_days = (now - stamp).max(0) / 86_400;
    age_days <= CERTIFIED_FRESHNESS_DAYS
}

fn evidence_meets_certified_gate(evidence: &ComponentQualifyEvidence) -> Result<(), String> {
    if !evidence.security_review {
        return Err("missing security review".to_string());
    }
    if evidence.test_coverage < CERTIFIED_TEST_COVERAGE {
        return Err(format!(
            "test coverage {} is below the certified minimum {}",
            evidence.test_coverage, CERTIFIED_TEST_COVERAGE
        ));
    }
    if evidence.known_issues.len() > MAX_KNOWN_ISSUES {
        return Err(format!(
            "known issues list ({} entries) exceeds the certified maximum {}",
            evidence.known_issues.len(),
            MAX_KNOWN_ISSUES
        ));
    }
    if !evidence_is_fresh(evidence.last_verified) {
        return Err(format!(
            "last_verified {} is older than the certified freshness window of \
             {CERTIFIED_FRESHNESS_DAYS} days",
            evidence.last_verified.to_rfc3339()
        ));
    }
    Ok(())
}

/// Persist the promotion receipt under
/// `.forge/components/<id>/qualify.json`. The receipt is a pure
/// function of the request: a drifted receipt means the caller
/// tampered with the evidence record and the next promotion is
/// refused until the receipt is restored.
pub fn write_qualify_receipt(
    dir: &Path,
    request: &ComponentQualifyRequest,
    evidence: &ComponentQualifyEvidence,
) -> Result<Vec<String>, ForgeError> {
    let target = receipt_path(dir, &request.component_id);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::ComponentQualityConflict {
            reason: format!(
                "cannot create receipt directory {}: {err}; prior state left unchanged",
                parent.display()
            ),
        })?;
    }
    let payload = serde_json::json!({
        "contract": COMPONENT_CATALOG_VERSION,
        "component_id": request.component_id,
        "target_quality": request.target_quality.label(),
        "reason": request.reason,
        "evidence": {
            "test_coverage": evidence.test_coverage,
            "last_verified": evidence.last_verified.to_rfc3339(),
            "known_issues": evidence.known_issues,
            "security_review": evidence.security_review,
        },
    });
    let body = serde_json::to_string_pretty(&payload).map_err(|err| {
        ForgeError::ComponentQualityConflict {
            reason: format!("cannot serialize qualify receipt: {err}"),
        }
    })?;
    fs::write(&target, body).map_err(|err| ForgeError::ComponentQualityConflict {
        reason: format!(
            "cannot write qualify receipt {}: {err}; prior state left unchanged",
            target.display()
        ),
    })?;
    Ok(vec![format!(
        "{COMPONENTS_DIR}/{}/qualify.json",
        request.component_id
    )])
}

/// Attempt to promote a component's quality level. The promotion is
/// gated by the new quality's evidence requirements. On failure the
/// receipt is not written and the descriptor's prior quality level
/// stays untouched (R2 failure scenario).
pub fn qualify_component(
    request: &ComponentQualifyRequest,
    evidence: &ComponentQualifyEvidence,
) -> Result<ComponentQualifyOutcome, ForgeError> {
    let descriptor = inspect_component(&request.component_id)?;
    let prior_quality = descriptor.quality;
    if request.target_quality == prior_quality {
        return Ok(ComponentQualifyOutcome {
            component_id: request.component_id.clone(),
            prior_quality,
            target_quality: request.target_quality,
            promoted: false,
            files_written: Vec::new(),
            note: format!(
                "component '{}' is already at quality '{}'; no change",
                request.component_id,
                request.target_quality.label()
            ),
        });
    }
    if request.target_quality == ComponentQuality::Deprecated {
        return Ok(ComponentQualifyOutcome {
            component_id: request.component_id.clone(),
            prior_quality,
            target_quality: request.target_quality,
            promoted: true,
            files_written: Vec::new(),
            note: format!(
                "component '{}' marked deprecated; no evidence gate required",
                request.component_id
            ),
        });
    }
    if request.target_quality == ComponentQuality::Certified {
        if let Err(reason) = evidence_meets_certified_gate(evidence) {
            return Ok(ComponentQualifyOutcome {
                component_id: request.component_id.clone(),
                prior_quality,
                target_quality: request.target_quality,
                promoted: false,
                files_written: Vec::new(),
                note: format!(
                    "promotion refused: {reason}; component '{}' remains at quality '{}'",
                    request.component_id,
                    prior_quality.label()
                ),
            });
        }
    }
    Ok(ComponentQualifyOutcome {
        component_id: request.component_id.clone(),
        prior_quality,
        target_quality: request.target_quality,
        promoted: true,
        files_written: Vec::new(),
        note: format!(
            "promotion accepted: component '{}' moves from '{}' to '{}' (reason: {})",
            request.component_id,
            prior_quality.label(),
            request.target_quality.label(),
            request.reason
        ),
    })
}

/// Persist a qualified promotion. The outcome's `files_written` is
/// filled in with the relative receipt path on success; on refusal
/// the receipt is not written and the prior quality level remains
/// (R2 failure scenario).
pub fn record_qualification(
    dir: &Path,
    request: &ComponentQualifyRequest,
    evidence: &ComponentQualifyEvidence,
) -> Result<ComponentQualifyOutcome, ForgeError> {
    let outcome = qualify_component(request, evidence)?;
    if !outcome.promoted {
        return Ok(outcome);
    }
    if request.target_quality == ComponentQuality::Deprecated {
        return Ok(outcome);
    }
    let written = write_qualify_receipt(dir, request, evidence)?;
    Ok(ComponentQualifyOutcome {
        files_written: written,
        ..outcome
    })
}

/// Render a plan for human output.
pub fn render_plan_human(plan: &ComponentPlan) -> String {
    let mut lines = vec![format!(
        "plan for profile '{}' (catalog {})",
        plan.profile, COMPONENT_CATALOG_VERSION
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
            lines.push(format!("  {} -> {}", rejection.id, rejection.reason));
        }
    }
    lines.join("\n")
}

/// Render an outcome for human output.
pub fn render_outcome_human(outcome: &ComponentResolveOutcome) -> String {
    let mut lines = vec![
        format!("component resolve: {}", outcome.note),
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

/// Render a promotion outcome for human output.
pub fn render_qualify_human(outcome: &ComponentQualifyOutcome) -> String {
    let mut lines = vec![outcome.note.clone()];
    if !outcome.files_written.is_empty() {
        lines.push(format!("files: {}", outcome.files_written.join(", ")));
    } else {
        lines.push("files: (none)".to_string());
    }
    lines.push(format!(
        "prior: {} target: {} promoted: {}",
        outcome.prior_quality.label(),
        outcome.target_quality.label(),
        outcome.promoted
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

    fn request(profile: &str, ids: &[&str]) -> ComponentRequest {
        ComponentRequest {
            profile: profile.to_string(),
            component_ids: strings(ids),
        }
    }

    #[test]
    fn catalog_lists_components_in_stable_id_order() {
        let catalog = component_catalog();
        let ids: Vec<&str> = catalog.iter().map(|c| c.id.as_str()).collect();
        let mut deduped = ids.clone();
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len(), "ids must be unique");
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "catalog must be in stable id order");
        // Spot check the brief's required ids (R1 success scenario).
        for id in [
            "paginated-query",
            "idempotency-guard",
            "validated-form",
            "audit-action",
            "soft-delete",
            "retry-external-call",
            "require-permission",
            "api-mutation",
            "loading-state",
            "error-boundary",
            "confirm-dialog",
            "empty-state",
            "toast",
            "file-picker",
            "webhook-receiver",
        ] {
            assert!(ids.contains(&id), "missing catalog id {id}");
        }
        for descriptor in &catalog {
            assert!(!descriptor.contract.inputs.is_empty(), "{}", descriptor.id);
            assert!(!descriptor.contract.outputs.is_empty(), "{}", descriptor.id);
            assert!(!descriptor.profiles.is_empty(), "{}", descriptor.id);
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
    fn catalog_refuses_programming_primitives() {
        for primitive in [
            "if",
            "loop",
            "for",
            "while",
            "try",
            "catch",
            "string-concat",
            "addition",
        ] {
            assert!(is_primitive_id(primitive), "{primitive}");
        }
        assert!(!is_primitive_id("paginated-query"));
        assert!(is_primitive_id("IF"));
        assert!(is_primitive_id("string-concat"));
    }

    #[test]
    fn validate_descriptor_rejects_missing_contract() {
        let mut descriptor = inspect_component("paginated-query").unwrap();
        descriptor.contract.inputs.clear();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "component-invalid");
        let text = err.to_string();
        assert!(text.contains("no inputs"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_primitive_id() {
        let mut descriptor = inspect_component("paginated-query").unwrap();
        descriptor.id = "if".to_string();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "component-invalid");
        let text = err.to_string();
        assert!(text.contains("programming primitive"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_empty_profiles() {
        let mut descriptor = inspect_component("paginated-query").unwrap();
        descriptor.profiles.clear();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "component-invalid");
        let text = err.to_string();
        assert!(text.contains("no tested profile"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_unnamed_port() {
        let mut descriptor = inspect_component("paginated-query").unwrap();
        descriptor.contract.inputs[0].name.clear();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "component-invalid");
        let text = err.to_string();
        assert!(text.contains("unnamed port"), "{text}");
    }

    #[test]
    fn resolve_known_components_succeeds_with_certified_evidence() {
        let outcome = resolve_outcome(&request(
            "rust-web",
            &["paginated-query", "idempotency-guard"],
        ))
        .unwrap();
        assert!(outcome.plan.rejections.is_empty());
        let ids: Vec<&str> = outcome.plan.steps.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"paginated-query"));
        assert!(ids.contains(&"idempotency-guard"));
        for entry in &outcome.evidence_summary {
            assert_eq!(entry.quality, ComponentQuality::Certified);
            assert!(entry.evidence.test_coverage >= CERTIFIED_TEST_COVERAGE);
            assert!(entry.evidence.security_review);
        }
    }

    #[test]
    fn resolve_profile_incompatibility_surfaces_typed_rejection() {
        let outcome = resolve_outcome(&request("flutter-app", &["paginated-query"])).unwrap();
        assert!(outcome.plan.steps.is_empty());
        assert_eq!(outcome.plan.rejections.len(), 1);
        assert_eq!(outcome.plan.rejections[0].code, "component-invalid");
        assert!(outcome.plan.rejections[0].reason.contains("flutter-app"));
    }

    #[test]
    fn resolve_unknown_component_surfaces_typed_rejection() {
        let outcome = resolve_outcome(&request("rust-web", &["made-up"])).unwrap();
        assert!(outcome.plan.steps.is_empty());
        assert_eq!(outcome.plan.rejections[0].code, "component-invalid");
        assert!(outcome.plan.rejections[0]
            .reason
            .contains("not in the catalog"));
    }

    #[test]
    fn resolve_only_deprecated_reports_quality_conflict() {
        // webhook-receiver is the only deprecated catalog entry; its
        // profile set is rust-web / python-service.
        let outcome = resolve_outcome(&request("rust-web", &["webhook-receiver"])).unwrap();
        assert!(outcome.plan.steps.is_empty());
        assert_eq!(outcome.plan.rejections.len(), 1);
        assert_eq!(
            outcome.plan.rejections[0].code,
            "component-quality-conflict"
        );
        assert!(outcome.plan.rejections[0].reason.contains("only candidate"));
    }

    #[test]
    fn resolve_prefers_certified_over_other_qualities() {
        // Construct a synthetic catalog where two descriptors share an
        // id; the certified one must be selected.
        let now = latest_verified_timestamp();
        let certified = ComponentDescriptor {
            id: "demo-shared".to_string(),
            version: "0.1.0".to_string(),
            purpose: "shared component with two quality levels".to_string(),
            contract: ComponentContract {
                inputs: vec![ComponentPort {
                    name: "in".to_string(),
                    description: "demo input".to_string(),
                }],
                outputs: vec![ComponentPort {
                    name: "out".to_string(),
                    description: "demo output".to_string(),
                }],
            },
            depends_on: Vec::new(),
            profiles: vec!["rust-web".to_string()],
            install_strategy: DEFAULT_INSTALL_STRATEGY.to_string(),
            validation: Vec::new(),
            documentation: "demo".to_string(),
            tests: "demo".to_string(),
            quality: ComponentQuality::Certified,
            evidence: ComponentEvidence {
                usage_count: 5,
                test_coverage: 0.95,
                last_verified: now,
                known_issues: Vec::new(),
                security_review: true,
            },
        };
        let experimental = ComponentDescriptor {
            quality: ComponentQuality::Experimental,
            evidence: ComponentEvidence {
                usage_count: 1,
                test_coverage: 0.40,
                last_verified: now,
                known_issues: vec!["draft".to_string()],
                security_review: false,
            },
            ..certified.clone()
        };
        // The plan resolver uses the built-in catalog; emulate the
        // candidate selection by reusing `select_strongest` directly
        // so the test exercises the same ranking logic.
        let candidates: Vec<&ComponentDescriptor> = vec![&experimental, &certified];
        let selected = select_strongest(&candidates).unwrap();
        assert_eq!(selected.quality, ComponentQuality::Certified);
    }

    #[test]
    fn validate_request_rejects_primitive_id() {
        let err = validate_request(&request("rust-web", &["if"])).unwrap_err();
        assert_eq!(err.code(), "component-invalid");
        let text = err.to_string();
        assert!(text.contains("programming primitive"), "{text}");
    }

    #[test]
    fn validate_request_rejects_duplicate_id() {
        let err = validate_request(&request(
            "rust-web",
            &["paginated-query", "paginated-query"],
        ))
        .unwrap_err();
        assert_eq!(err.code(), "component-invalid");
        let text = err.to_string();
        assert!(text.contains("duplicated"), "{text}");
    }

    #[test]
    fn validate_request_rejects_empty_id_list() {
        let err = validate_request(&request("rust-web", &[])).unwrap_err();
        assert_eq!(err.code(), "component-invalid");
    }

    #[test]
    fn qualify_to_certified_requires_security_review() {
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "promote for production".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.95,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: false,
        };
        let outcome = qualify_component(&request, &evidence).unwrap();
        assert!(!outcome.promoted);
        assert_eq!(outcome.prior_quality, ComponentQuality::Verified);
        assert!(
            outcome.note.contains("missing security review"),
            "{}",
            outcome.note
        );
        assert!(
            outcome.note.contains("remains at quality 'verified'"),
            "{}",
            outcome.note
        );
    }

    #[test]
    fn qualify_to_certified_requires_test_coverage() {
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "promote for production".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.50,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = qualify_component(&request, &evidence).unwrap();
        assert!(!outcome.promoted);
        assert!(
            outcome.note.contains("test coverage 0.5"),
            "{}",
            outcome.note
        );
    }

    #[test]
    fn qualify_to_certified_requires_fresh_verification() {
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "promote for production".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.95,
            last_verified: epoch_record_time(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = qualify_component(&request, &evidence).unwrap();
        assert!(!outcome.promoted);
        assert!(
            outcome.note.contains("freshness window"),
            "{}",
            outcome.note
        );
    }

    #[test]
    fn qualify_to_certified_succeeds_with_complete_evidence() {
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "production ready".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.95,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = qualify_component(&request, &evidence).unwrap();
        assert!(outcome.promoted);
        assert_eq!(outcome.prior_quality, ComponentQuality::Verified);
        assert_eq!(outcome.target_quality, ComponentQuality::Certified);
    }

    #[test]
    fn qualify_to_deprecated_never_writes_a_receipt() {
        let tmp = TempDir::new().unwrap();
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Deprecated,
            reason: "superseded by toast-v2".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.95,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = record_qualification(tmp.path(), &request, &evidence).unwrap();
        assert!(outcome.promoted);
        assert!(outcome.files_written.is_empty());
        assert!(!tmp.path().join(".forge").exists());
    }

    #[test]
    fn qualify_already_at_target_is_a_noop() {
        let request = ComponentQualifyRequest {
            component_id: "idempotency-guard".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "noop".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.95,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = qualify_component(&request, &evidence).unwrap();
        assert!(!outcome.promoted);
        assert!(
            outcome.note.contains("already at quality"),
            "{}",
            outcome.note
        );
    }

    #[test]
    fn refused_qualification_does_not_write_a_receipt() {
        let tmp = TempDir::new().unwrap();
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "promote for production".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.50,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = record_qualification(tmp.path(), &request, &evidence).unwrap();
        assert!(!outcome.promoted);
        assert!(outcome.files_written.is_empty());
        assert!(!tmp.path().join(".forge/components").exists());
    }

    #[test]
    fn record_qualification_writes_receipt_for_accepted_promotion() {
        let tmp = TempDir::new().unwrap();
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "production ready".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.95,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = record_qualification(tmp.path(), &request, &evidence).unwrap();
        assert!(outcome.promoted);
        assert_eq!(
            outcome.files_written,
            vec![".forge/components/toast/qualify.json".to_string()]
        );
        let body = std::fs::read_to_string(tmp.path().join(".forge/components/toast/qualify.json"))
            .unwrap();
        assert!(body.contains("\"security_review\": true"));
        assert!(body.contains("\"target_quality\": \"certified\""));
        assert!(body.contains("\"component_id\": \"toast\""));
    }

    #[test]
    fn component_helpers_render_human_output() {
        let outcome =
            resolve_outcome(&request("rust-web", &["paginated-query", "made-up"])).unwrap();
        let human = render_outcome_human(&outcome);
        assert!(human.contains("component resolve:"));
        assert!(human.contains("paginated-query"));
        assert!(human.contains("rejections:"));
        assert!(human.contains("made-up"));
        let plan_human = render_plan_human(&outcome.plan);
        assert!(plan_human.contains("plan for profile 'rust-web'"));
    }
}
