//! # `GovernanceProviderConfig` - Trait Implementations
//!
//! This module contains trait implementations for `GovernanceProviderConfig`.
//!
//! ## Implemented Traits
//!
//! - `Default`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::contract::{DEFAULT_TIMEOUT_MS, LOCAL_PROVIDER_ID};
use super::engine::default_protocol_version;
use super::model::GovernanceProviderConfig;

impl Default for GovernanceProviderConfig {
    fn default() -> Self {
        Self {
            provider: LOCAL_PROVIDER_ID.to_string(),
            adapter: None,
            workspace_root: None,
            enabled: true,
            protocol_version: default_protocol_version(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }
}
