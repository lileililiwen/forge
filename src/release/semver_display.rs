//! # `Semver` - Trait Implementations
//!
//! This module contains trait implementations for `Semver`.
//!
//! ## Implemented Traits
//!
//! - `Display`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::Semver;

impl std::fmt::Display for Semver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label())
    }
}
