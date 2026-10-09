//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use chrono::{DateTime, Utc};

use super::constants::{
    MAX_KNOWN_ISSUES, PRIMITIVE_IDS, UI_PATTERN_INTENTS, UI_PATTERN_REQUIRED_STATES,
};
use super::model::{UiPatternDescriptor, UiPatternState};

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

pub(super) fn epoch_record_time() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(0, 0).expect("epoch is valid")
}

pub(super) fn default_verified_timestamp() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_700_000_000, 0).expect("timestamp is valid")
}

pub(super) fn default_certified_timestamp() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_725_000_000, 0).expect("timestamp is valid")
}

#[allow(dead_code)]
fn latest_verified_timestamp() -> DateTime<Utc> {
    let secs = Utc::now().timestamp();
    DateTime::<Utc>::from_timestamp(secs, 0).expect("current timestamp is valid")
}

pub(super) fn required_states() -> Vec<UiPatternState> {
    UI_PATTERN_REQUIRED_STATES
        .iter()
        .map(|name| UiPatternState {
            name: (*name).to_string(),
            description: format!("explicit {name} surface for the pattern"),
        })
        .collect()
}
