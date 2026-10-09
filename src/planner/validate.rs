//! Planner intent validation.

use crate::component::component_catalog;
use crate::core::ForgeError;
use crate::feature::feature_catalog;
use crate::profile::inspect_profile;
use crate::ui_pattern::ui_pattern_catalog;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashSet};

use super::contract::{
    CLIENT_ONLY_PROFILES, DEFAULT_BACKEND_HINTS, KNOWN_CONSTRAINT_KEYS,
    MAX_CAPABILITIES_PER_INTENT, SERVER_SIDE_CAPABILITIES,
};
use super::model::{Intent, ValidatedIntent};

fn short_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Bounded set of capabilities the planner accepts. A model that
/// references a capability outside the union of the profile
/// capabilities, the component catalog, the UI pattern catalog and
/// the feature catalog is refused before any planning.
fn known_capability_set() -> BTreeSet<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    let profile = inspect_profile("aspnet-web").ok();
    if let Some(p) = profile {
        for cap in &p.capabilities {
            set.insert(cap.clone());
        }
    }
    for feature in feature_catalog() {
        set.insert(feature.id.clone());
    }
    for component in component_catalog() {
        set.insert(component.id.clone());
    }
    for pattern in ui_pattern_catalog() {
        set.insert(pattern.id.clone());
    }
    set
}

/// True when the profile id is the client-only half of the catalog
/// and cannot itself provide the server-side capability.
pub(super) fn profile_is_client_only(profile: &str) -> bool {
    CLIENT_ONLY_PROFILES.contains(&profile)
}

/// True when the capability needs a server-side backend.
fn capability_is_server_side(capability: &str) -> bool {
    SERVER_SIDE_CAPABILITIES.contains(&capability)
}

/// Stable content hash over the validated intent. Two requests with
/// the same action, profile, capabilities and constraints produce
/// the same hash, so a repeated `resolve` against the same intent
/// returns the existing receipt (boundary scenario) rather than
/// emitting a different plan id.
pub fn intent_hash(intent: &Intent) -> String {
    let mut hasher = Sha256::new();
    hasher.update(intent.action.label().as_bytes());
    hasher.update(b"|");
    hasher.update(intent.profile.as_bytes());
    hasher.update(b"|");
    for cap in &intent.required_capabilities {
        hasher.update(cap.as_bytes());
        hasher.update(b",");
    }
    hasher.update(b"|");
    for cap in &intent.forbidden_capabilities {
        hasher.update(cap.as_bytes());
        hasher.update(b",");
    }
    hasher.update(b"|");
    let mut constraints = intent.constraints.clone();
    constraints.sort_by(|a, b| a.key.cmp(&b.key).then(a.value.cmp(&b.value)));
    for c in &constraints {
        hasher.update(c.key.as_bytes());
        hasher.update(b"=");
        hasher.update(c.value.as_bytes());
        hasher.update(b",");
    }
    let bytes = hasher.finalize();
    short_hex(&bytes[..16])
}

/// Content hash over the versioned catalog surface the resolver
/// relies on. A change to any catalog version, evidence or quality
/// level produces a new hash, so the apply path can detect that the
/// captured plan is no longer in sync with the catalog.
pub fn catalog_hash() -> String {
    let mut hasher = Sha256::new();
    for feature in feature_catalog() {
        hasher.update(feature.id.as_bytes());
        hasher.update(b"@");
        hasher.update(feature.version.as_bytes());
        hasher.update(b",");
    }
    for component in component_catalog() {
        hasher.update(component.id.as_bytes());
        hasher.update(b"@");
        hasher.update(component.version.as_bytes());
        hasher.update(b",");
    }
    for pattern in ui_pattern_catalog() {
        hasher.update(pattern.id.as_bytes());
        hasher.update(b"@");
        hasher.update(pattern.version.as_bytes());
        hasher.update(b",");
    }
    let bytes = hasher.finalize();
    short_hex(&bytes[..16])
}

/// Validate a structured intent before any side effect. The
/// returned [`ValidatedIntent`] is the only form the resolver
/// accepts; raw `Intent` values reach no further planner surface.
pub fn validate_intent(intent: &Intent) -> Result<ValidatedIntent, ForgeError> {
    let profile_id = intent.profile.trim();
    if profile_id.is_empty() {
        return Err(ForgeError::IntentInvalid {
            reason: "intent profile is empty".to_string(),
        });
    }

    let mut required: Vec<String> = intent
        .required_capabilities
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let mut forbidden: Vec<String> = intent
        .forbidden_capabilities
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    required.sort();
    required.dedup();
    forbidden.sort();
    forbidden.dedup();

    if required.len() + forbidden.len() > MAX_CAPABILITIES_PER_INTENT {
        return Err(ForgeError::IntentInvalid {
            reason: format!(
                "intent carries {} capabilities (required {} + forbidden {}); the planner \
                 accepts at most {MAX_CAPABILITIES_PER_INTENT} per request",
                required.len() + forbidden.len(),
                required.len(),
                forbidden.len()
            ),
        });
    }

    let intersection: Vec<&String> = required.iter().filter(|c| forbidden.contains(c)).collect();
    if !intersection.is_empty() {
        let names: Vec<String> = intersection.iter().map(|s| (*s).clone()).collect();
        return Err(ForgeError::IntentInvalid {
            reason: format!(
                "intent declares the same capability as both required and forbidden: {}; \
                 the planner refuses an unresolvable capability graph",
                names.join(",")
            ),
        });
    }

    let profile = inspect_profile(profile_id).map_err(|_| ForgeError::IntentInvalid {
        reason: format!(
            "intent profile '{profile_id}' is not in the supported catalog; the planner \
             refuses unknown profiles before any planning"
        ),
    })?;
    if profile.support_status != crate::profile::ProfileSupportStatus::Supported {
        return Err(ForgeError::IntentInvalid {
            reason: format!(
                "intent profile '{profile_id}' is reserved on the catalog as a planned \
                 candidate; promote it to a versioned supported descriptor before the \
                 planner can plan against it"
            ),
        });
    }

    let profile_caps: HashSet<String> = profile.capabilities.iter().cloned().collect();
    for cap in &required {
        if !profile_caps.contains(cap) {
            if profile_is_client_only(profile_id) && capability_is_server_side(cap) {
                let backend_hint = if profile_id == "flutter-app" {
                    "flutter-app + rust-web or python-service backend"
                } else {
                    "react-web + rust-web or python-service backend"
                };
                return Err(ForgeError::IntentInvalid {
                    reason: format!(
                        "intent profile '{profile_id}' is a client-only stack and cannot \
                         serve the server-side capability '{cap}'; recommended boundary: \
                         {backend_hint}"
                    ),
                });
            }
            return Err(ForgeError::IntentInvalid {
                reason: format!(
                    "intent profile '{profile_id}' (adapter '{}', version '{}') does not \
                     support required capability '{cap}'",
                    profile.adapter, profile.version
                ),
            });
        }
    }

    for constraint in &intent.constraints {
        if !KNOWN_CONSTRAINT_KEYS.contains(&constraint.key.as_str()) {
            return Err(ForgeError::IntentInvalid {
                reason: format!(
                    "intent constraint '{}' is not recognised by the planner; accepted \
                     keys are {}",
                    constraint.key,
                    KNOWN_CONSTRAINT_KEYS.join(",")
                ),
            });
        }
    }

    let known = known_capability_set();
    for cap in required.iter().chain(forbidden.iter()) {
        if !known.contains(cap) {
            return Err(ForgeError::IntentInvalid {
                reason: format!(
                    "intent capability '{cap}' is unknown to the planner catalog; the \
                     planner refuses a capability that exists in no supported profile, \
                     feature, component or UI pattern descriptor"
                ),
            });
        }
    }

    let backend_boundary = if profile_is_client_only(profile_id) {
        DEFAULT_BACKEND_HINTS
            .iter()
            .find(|hint| {
                inspect_profile(hint)
                    .map(|p| p.support_status == crate::profile::ProfileSupportStatus::Supported)
                    .unwrap_or(false)
            })
            .map(|s| s.to_string())
    } else {
        None
    };

    let note = format!(
        "validated intent: action={} profile={}@{} required=[{}] forbidden=[{}] \
         backend_boundary={}",
        intent.action.label(),
        profile.id,
        profile.version,
        required.join(","),
        forbidden.join(","),
        backend_boundary.as_deref().unwrap_or("none")
    );

    Ok(ValidatedIntent {
        intent: Intent {
            action: intent.action,
            profile: profile.id.clone(),
            required_capabilities: required,
            forbidden_capabilities: forbidden,
            constraints: intent.constraints.clone(),
        },
        profile_id: profile.id,
        profile_version: profile.version,
        adapter: profile.adapter,
        backend_boundary,
        note,
    })
}
