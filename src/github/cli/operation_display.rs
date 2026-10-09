//! # `GhOperation` - Trait Implementations
//!
//! This module contains trait implementations for `GhOperation`.
//!
//! ## Implemented Traits
//!
//! - `Display`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::GhOperation;

impl std::fmt::Display for GhOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}
