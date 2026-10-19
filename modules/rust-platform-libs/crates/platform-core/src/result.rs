//! Common result alias.

use crate::error::SafeError;

/// Result alias used across the platform crates.
pub type PlatformResult<T> = Result<T, SafeError>;
