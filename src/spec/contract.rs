//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Contract data version for the spec/remediation API and storage.
pub const SPEC_CONTRACT_VERSION: &str = "0.1.0";

/// Directory (relative to the project root) holding generated specs.
pub const SPECS_DIR: &str = ".forge/specs";

/// Maximum finding identifiers included in a single spec; overlapping
/// finding sets must be split or routed through separate specs to
/// keep each proposal bounded.
pub const MAX_FINDINGS_PER_SPEC: usize = 32;

/// Stable short hash length used in spec identifiers.
pub(super) const SPEC_HASH_LEN: usize = 12;
