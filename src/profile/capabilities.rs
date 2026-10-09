//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Capabilities that require a server-side backend. A client-only profile
/// (today: `flutter-app`, `react-web`) rejects these at resolution time,
/// before any file change, and suggests a backend boundary instead.
pub(super) const SERVER_SIDE_CAPABILITIES: &[&str] =
    &["postgres", "redis", "background-jobs", "storage"];
