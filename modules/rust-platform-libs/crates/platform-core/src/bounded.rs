//! Length-bounded string.
//!
//! [`BoundedString<N>`] enforces an upper bound of `N` bytes (or
//! characters) at construction time so callers cannot accidentally
//! forward an unbounded user input into a contract field that the
//! language-neutral schema declares as `maxLength: 256`.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Error returned when a value would exceed the bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BoundedStringError {
    /// The value's character count exceeds the bound.
    #[error("string length {actual} exceeds the maximum {max}")]
    TooLong {
        /// The configured maximum.
        max: usize,
        /// The actual character count.
        actual: usize,
    },
    /// The value is empty and the bound forbids empty strings.
    #[error("string is empty but must contain at least one character")]
    Empty,
}

/// String type whose character count is bounded by `MAX` at construction.
///
/// `MAX` is the inclusive upper bound. The type stores `String` and
/// does not perform runtime checks after construction; callers that
/// mutate the value (which is impossible without `unsafe` or interior
/// mutability) would defeat the bound. The bound is checked exactly
/// once, in [`BoundedString::try_new`].
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BoundedString<const MAX: usize> {
    inner: String,
}

impl<const MAX: usize> BoundedString<MAX> {
    /// Construct a bounded string, rejecting values whose character
    /// count exceeds `MAX`.
    pub fn try_new(value: impl Into<String>) -> Result<Self, BoundedStringError> {
        let inner = value.into();
        let actual = inner.chars().count();
        if actual > MAX {
            return Err(BoundedStringError::TooLong { max: MAX, actual });
        }
        Ok(Self { inner })
    }

    /// Construct a non-empty bounded string.
    pub fn try_new_non_empty(value: impl Into<String>) -> Result<Self, BoundedStringError> {
        let s = Self::try_new(value)?;
        if s.inner.is_empty() {
            return Err(BoundedStringError::Empty);
        }
        Ok(s)
    }

    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.inner
    }

    /// Consume and return the inner `String`.
    #[must_use]
    pub fn into_string(self) -> String {
        self.inner
    }

    /// Configured maximum.
    #[must_use]
    pub const fn max() -> usize {
        MAX
    }
}

impl<const MAX: usize> fmt::Debug for BoundedString<MAX> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Debug shows the inner value but never the bound; this is
        // safe because the inner is already bounded and is not a
        // secret by construction.
        f.debug_tuple("BoundedString").field(&self.inner).finish()
    }
}

impl<const MAX: usize> fmt::Display for BoundedString<MAX> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.inner)
    }
}

impl<const MAX: usize> AsRef<str> for BoundedString<MAX> {
    fn as_ref(&self) -> &str {
        &self.inner
    }
}

impl<const MAX: usize> Serialize for BoundedString<MAX> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(serializer)
    }
}

impl<'de, const MAX: usize> Deserialize<'de> for BoundedString<MAX> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_value_within_bound() {
        let s: BoundedString<8> = BoundedString::try_new("hello").unwrap();
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn accepts_value_exactly_at_bound() {
        let s: BoundedString<5> = BoundedString::try_new("abcde").unwrap();
        assert_eq!(s.as_str(), "abcde");
    }

    #[test]
    fn rejects_value_one_over_bound() {
        let err = BoundedString::<5>::try_new("abcdef").unwrap_err();
        assert_eq!(err, BoundedStringError::TooLong { max: 5, actual: 6 });
    }

    #[test]
    fn rejects_empty_when_non_empty_required() {
        let err = BoundedString::<5>::try_new_non_empty("").unwrap_err();
        assert_eq!(err, BoundedStringError::Empty);
    }

    #[test]
    fn counts_characters_not_bytes() {
        // "héllo" is 5 chars but 6 bytes in UTF-8; the bound is on chars.
        let s: BoundedString<5> = BoundedString::try_new("héllo").unwrap();
        assert_eq!(s.as_str(), "héllo");
    }

    #[test]
    fn deserialize_rejects_overlong_value() {
        let err: Result<BoundedString<3>, _> = serde_json::from_str("\"abcd\"");
        assert!(err.is_err());
    }
}
