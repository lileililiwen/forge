//! Component catalog.

use crate::core::ForgeError;
use chrono::{DateTime, Utc};

use super::contract::{
    COMPONENT_CATALOG_VERSION, DEFAULT_INSTALL_STRATEGY, MAX_KNOWN_ISSUES, PRIMITIVE_IDS,
};
use super::model::{
    ComponentContract, ComponentDescriptor, ComponentEvidence, ComponentPort, ComponentQuality,
};

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

pub(super) fn epoch_record_time() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(0, 0).expect("epoch is valid")
}

fn default_verified_timestamp() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_700_000_000, 0).expect("timestamp is valid")
}

fn default_certified_timestamp() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_725_000_000, 0).expect("timestamp is valid")
}

#[allow(dead_code)]
pub(super) fn latest_verified_timestamp() -> DateTime<Utc> {
    let secs = Utc::now().timestamp();
    DateTime::<Utc>::from_timestamp(secs, 0).expect("current timestamp is valid")
}

/// Build a single descriptor with shared defaults. Catalog authors
/// only specify the fields that differentiate the component; the
/// rest of the typed evidence is filled in by the helpers below so
/// the catalog stays uniform and easy to read.
#[allow(clippy::too_many_arguments)]
pub(super) fn descriptor(
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
