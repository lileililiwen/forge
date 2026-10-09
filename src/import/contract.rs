//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

pub(super) const MAX_SCAN_BYTES: u64 = 128 * 1024;

pub(super) const MAX_SUBDIRS: usize = 32;

/// Versioned contract id for the `forge workspace sync` JSON envelope.
pub const WORKSPACE_SYNC_CONTRACT: &str = "forge-workspace-sync/0.1.0";

/// Synthetic journal owner for the counts-only `workspace.sync` summary row.
/// Registry-wide operations never belong to one project; the double-
/// underscore convention matches the existing `__api__` / `__component__`
/// synthetic owners.
pub(super) const WORKSPACE_SYNC_JOURNAL_PROJECT: &str = "__workspace__";
