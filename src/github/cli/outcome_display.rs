//! # `GhOutcome` - Trait Implementations
//!
//! This module contains trait implementations for `GhOutcome`.
//!
//! ## Implemented Traits
//!
//! - `Display`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::GhOutcome;

impl std::fmt::Display for GhOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}
