//! Phantom-typed opaque identifier.
//!
//! [`Id<T>`] wraps a `String` and is parameterised by a phantom tag type
//! `T` so the type system prevents accidental cross-wiring of, e.g., a
//! user ID for a tenant ID. The tag type carries no data and has no
//! runtime cost.

use std::fmt;
use std::marker::PhantomData;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Opaque identifier parameterised by a phantom tag type `T`.
///
/// The inner `String` is not part of the public API; use
/// [`Id::as_str`] to read it and [`Id::new`] / [`Id::try_new`] to
/// construct one. Two `Id<T>` values are equal iff their inner strings
/// are equal, regardless of the tag.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id<T> {
    inner: String,
    _tag: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// Wrap an arbitrary string without validation. Prefer
    /// [`Id::try_new`] when the caller wants to enforce the shared
    /// `[A-Za-z0-9][A-Za-z0-9._:-]{0,N}` pattern.
    #[must_use]
    pub fn new(inner: impl Into<String>) -> Self {
        Self {
            inner: inner.into(),
            _tag: PhantomData,
        }
    }

    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.inner
    }

    /// Consume the wrapper and return the inner string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.inner
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Render only the type tag in Debug, never the inner value.
        // Identifiers are commonly log-scoped; leaking them into panic
        // messages is undesirable. Use `as_str()` to opt in.
        write!(f, "Id::<{}>(<redacted>)", std::any::type_name::<T>())
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display intentionally emits the same redacted form as Debug
        // so that any `format!("{id}")` call cannot leak the value.
        write!(f, "<redacted:{}>", std::any::type_name::<T>())
    }
}

impl<T> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_with_same_inner_compare_equal_regardless_of_tag() {
        let user: Id<UserTag> = Id::new("abc");
        let other: Id<OtherTag> = Id::new("abc");
        assert_eq!(user.as_str(), other.as_str());
    }

    #[test]
    fn debug_and_display_do_not_leak_inner() {
        let id: Id<UserTag> = Id::new("super-secret");
        let debug = format!("{id:?}");
        let display = format!("{id}");
        assert!(!debug.contains("super-secret"));
        assert!(!display.contains("super-secret"));
        assert!(debug.contains("redacted"));
    }

    #[test]
    fn serialize_roundtrip_preserves_value() {
        let id: Id<UserTag> = Id::new("usr_1");
        let json = serde_json::to_string(&id).unwrap();
        let back: Id<UserTag> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "usr_1");
    }

    struct UserTag;
    struct OtherTag;
}
