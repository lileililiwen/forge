//! Correlation identifier.
//!
//! Wraps a UUIDv4 in a typed wrapper so callers cannot accidentally
//! pass a [`CorrelationId`] where a user ID, tenant ID, or session ID
//! is expected.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// A correlation identifier. Generated as a UUIDv4.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CorrelationId(Uuid);

impl CorrelationId {
    /// Generate a fresh random correlation id.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse a correlation id from its canonical hyphenated form.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `value` is not a valid UUID.
    pub fn parse(value: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(value)?))
    }

    /// Borrow the underlying UUID.
    #[must_use]
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CorrelationId").field(&self.0).finish()
    }
}

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.hyphenated())
    }
}

impl Serialize for CorrelationId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.hyphenated().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CorrelationId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Uuid::parse_str(&s)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_generates_unique_ids() {
        let a = CorrelationId::new();
        let b = CorrelationId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn parse_roundtrip() {
        let id = CorrelationId::new();
        let s = id.to_string();
        let back = CorrelationId::parse(&s).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(CorrelationId::parse("not-a-uuid").is_err());
    }

    #[test]
    fn serialize_roundtrip() {
        let id = CorrelationId::new();
        let json = serde_json::to_string(&id).unwrap();
        let back: CorrelationId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
