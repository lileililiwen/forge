//! # `RemediationClass` - Trait Implementations
//!
//! This module contains trait implementations for `RemediationClass`.
//!
//! ## Implemented Traits
//!
//! - `Display`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::RemediationClass;

impl std::fmt::Display for RemediationClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}
