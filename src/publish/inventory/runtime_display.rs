//! # `RuntimeClass` - Trait Implementations
//!
//! This module contains trait implementations for `RuntimeClass`.
//!
//! ## Implemented Traits
//!
//! - `Display`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::fmt;

use super::model::RuntimeClass;

impl fmt::Display for RuntimeClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
