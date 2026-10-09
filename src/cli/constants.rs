//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Versioned contract for `forge plugins`. Additive: a plugin that does
/// not recognise a request kind answers `unsupported`, which the
/// registry reports as a capability gap rather than a failed run.
pub(super) const PLUGINS_CONTRACT: &str = "forge-plugins/0.1.0";

/// Publish every declared project in the resolved inventory.
///
/// Inventory resolution order:
/// 1. `--inventory <path>` — local JSON file or external adapter
///    executable (consumes the
///    `forge-project-inventory/0.1.0` contract).
/// 2. `--fleet-registry <path>` or `$FORGE_WORKSPACE_REGISTRY` —
///    legacy workspace-governance `projects.json` (compatibility
///    adapter that synthesizes an inventory snapshot).
///
/// Every declared entry is reported as `compose_ready`,
/// `compose_missing`, `invalid`, or `source_unavailable`. Only
/// `compose_ready` entries invoke a provider; the others are
/// surfaced to the operator instead of silently omitted.
/// Maximum `--jobs` value: 32 workers bound the target and the
/// controller without a thread-per-project explosion on large
/// rosters (`fleet-live-rollout`).
pub const FLEET_MAX_JOBS: usize = 32;

/// Default `--jobs` value: four concurrent per-project publishes.
pub const FLEET_DEFAULT_JOBS: usize = 4;
