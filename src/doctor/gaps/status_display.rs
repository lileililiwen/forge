//! # `GapStatus` - Trait Implementations
//!
//! This module contains trait implementations for `GapStatus`.
//!
//! ## Implemented Traits
//!
//! - `Display`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::GapStatus;

impl std::fmt::Display for GapStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}
