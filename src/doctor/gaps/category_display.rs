//! # `GapCategory` - Trait Implementations
//!
//! This module contains trait implementations for `GapCategory`.
//!
//! ## Implemented Traits
//!
//! - `Display`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::GapCategory;

impl std::fmt::Display for GapCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}
