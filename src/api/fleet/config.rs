//! API fleet: config.

use super::contract::{
    FLEET_MAX_AGE_ENV, PUBLISH_HISTORY_DEFAULT_LIMIT, PUBLISH_HISTORY_ENV,
    PUBLISH_HISTORY_LIMIT_ENV, PUBLISH_HISTORY_MAX_LIMIT, SELF_DEFAULT_ID, SELF_ID_ENV,
};
use crate::core::validate_project_id;
use crate::fleet;

/// Resolve the configured fleet freshness window, falling back to the shared
/// fleet default when the environment value is absent or out of bounds.
pub fn configured_max_age() -> i64 {
    let raw = std::env::var(FLEET_MAX_AGE_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(value) = raw {
        if let Ok(parsed) = value.parse::<i64>() {
            if fleet::validate_max_age(parsed).is_ok() {
                return parsed;
            }
        }
    }
    fleet::DEFAULT_MAX_AGE_SECONDS
}

/// The stable Forge-self identity: an explicit valid override, else the
/// package name. Never derived from a filesystem path.
pub fn self_identity() -> String {
    std::env::var(SELF_ID_ENV)
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty() && validate_project_id(value).is_ok())
        .unwrap_or_else(|| SELF_DEFAULT_ID.to_string())
}

/// Resolve the bounded publish-history limit: a positive machine value
/// clamped to the hard ceiling, else the default.
pub fn configured_publish_limit() -> usize {
    std::env::var(PUBLISH_HISTORY_LIMIT_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .map(|value| value.min(PUBLISH_HISTORY_MAX_LIMIT))
        .unwrap_or(PUBLISH_HISTORY_DEFAULT_LIMIT)
}

/// True when the operator has explicitly disabled the publish-history
/// projection. The projection is on by default because it reads the local,
/// Forge-owned registry; only the exact opt-out tokens turn it off.
pub(crate) fn publish_history_disabled() -> bool {
    match std::env::var(PUBLISH_HISTORY_ENV) {
        Ok(value) => {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off"
            )
        }
        Err(_) => false,
    }
}
