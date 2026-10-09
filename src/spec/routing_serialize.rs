//! # `RoutingStatus` - Trait Implementations
//!
//! This module contains trait implementations for `RoutingStatus`.
//!
//! ## Implemented Traits
//!
//! - `Serialize`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::Serialize;

use super::model::RoutingStatus;

impl Serialize for RoutingStatus {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.label())
    }
}
