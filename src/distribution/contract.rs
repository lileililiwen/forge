//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Contract data version for the distribution surface.
pub const DISTRIBUTION_CONTRACT_VERSION: &str = "0.1.0";

/// Distribution subdirectory inside the project. The state file
/// lives under `<project>/.forge/distribution/<project-id>/state.json`
/// so a single project root owns its evidence.
pub const DISTRIBUTION_DIR: &str = ".forge/distribution";

/// Git remote name used to publish the primary ref. The
/// registry already records this in `ProjectRecord.git_remote`,
/// but the value is also embedded in the manifest's
/// `distribution.primary` so the contract stays
/// provider-scoped without re-reading the live remote URL.
pub const PRIMARY_REMOTE: &str = "origin";
