//! # `InventoryClassification` - Trait Implementations
//!
//! This module contains trait implementations for `InventoryClassification`.
//!
//! ## Implemented Traits
//!
//! - `Display`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::fmt;

use super::model::InventoryClassification;

impl fmt::Display for InventoryClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
