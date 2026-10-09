//! # `GovernanceStatus` - Trait Implementations
//!
//! This module contains trait implementations for `GovernanceStatus`.
//!
//! ## Implemented Traits
//!
//! - `From`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::{GovernanceStatus, ProviderStatus};

impl From<ProviderStatus> for GovernanceStatus {
    fn from(status: ProviderStatus) -> Self {
        Self(status)
    }
}
