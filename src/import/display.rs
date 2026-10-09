//! # `FieldStatus` - Trait Implementations
//!
//! This module contains trait implementations for `FieldStatus`.
//!
//! ## Implemented Traits
//!
//! - `Display`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::FieldStatus;

impl std::fmt::Display for FieldStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldStatus::Detected => write!(f, "detected"),
            FieldStatus::Missing => write!(f, "missing"),
            FieldStatus::Unknown => write!(f, "unknown"),
        }
    }
}
