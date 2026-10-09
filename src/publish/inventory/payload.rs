//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::{InventoryEntry, InventoryMalformedEntry};

/// Project payload returned by [`validated_document`]: the
/// declared provider id, RFC 3339 generation timestamp, and the
/// (valid projects, malformed entries) pair.
pub(super) type ValidatedPayload = (
    String,
    String,
    (Vec<InventoryEntry>, Vec<InventoryMalformedEntry>),
);
