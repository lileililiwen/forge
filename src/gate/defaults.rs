//! # `GateConfig` - Trait Implementations
//!
//! This module contains trait implementations for `GateConfig`.
//!
//! ## Implemented Traits
//!
//! - `Default`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::GateConfig;

impl Default for GateConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
