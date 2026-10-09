//! # `SpecId` - Trait Implementations
//!
//! This module contains trait implementations for `SpecId`.
//!
//! ## Implemented Traits
//!
//! - `Serialize`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::Serialize;

use super::model::SpecId;

impl Serialize for SpecId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("SpecId", 3)?;
        s.serialize_field("project_id", &self.project_id)?;
        s.serialize_field("hash", &self.hash)?;
        s.serialize_field("dir_name", &self.dir_name())?;
        s.end()
    }
}
