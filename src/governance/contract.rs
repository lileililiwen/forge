//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::time::Duration;

pub const GOVERNANCE_CONTRACT_VERSION: &str = "0.1.0";

pub const LOCAL_PROVIDER_ID: &str = "local";

/// Known-provider id whose adapter location is packaged as a preset: a
/// candidate below an explicitly supplied workspace root, never a guess.
pub const WORKSPACE_GOVERNANCE_PROVIDER_ID: &str = "workspace-governance";

/// Environment carrying the workspace root when `--workspace-root` is
/// absent. The root is configuration input only — Forge never searches
/// parent directories or the network for a provider.
pub const WORKSPACE_ROOT_ENV: &str = "FORGE_WORKSPACE_ROOT";

/// Packaged candidate relative to the workspace root. Live-verified
/// 2026-09-24: the real sibling layout nests the checkout inside the
/// portfolio it governs, so the packaged adapter lives at
/// `<workspace-root>/workspace-governance/scripts/forge_governance_adapter.py`
/// (the design's original `<root>/scripts/...` guess is disproved in
/// `tests/fixtures/governance-audit/NOTES.md`). The root itself is the
/// same value the adapter's own `WORKSPACE_ROOT` environment input takes.
pub(super) const WORKSPACE_GOVERNANCE_ADAPTER_RELPATH: &str =
    "workspace-governance/scripts/forge_governance_adapter.py";

pub(super) const CONFIG_RELATIVE_PATH: &str = ".forge/providers.yaml";

pub(super) const OBSERVATIONS_RELATIVE_PATH: &str = ".forge/governance/observations.json";

pub(super) const DEFAULT_TIMEOUT_MS: u64 = 10_000;

pub(super) const MAX_ADAPTER_OUTPUT_BYTES: usize = 256 * 1024;

pub(super) const MAX_EVIDENCE_CHARS: usize = 2_000;

/// Bound on the bytes read from the revision lookup. `git rev-parse HEAD`
/// answers with one object name — 40 hex characters, or 64 under SHA-256 — so
/// anything beyond this is not the answer, and an unbounded read from a child
/// process would be the very defect this lookup is being bounded to fix.
pub(super) const MAX_GIT_REVISION_BYTES: usize = 4 * 1024;

/// Appended to the adapter's stderr when a descendant kept a drained pipe open
/// past the deadline, so an incomplete read is visible instead of silent.
pub(super) const TRUNCATED_DRAIN_MARKER: &str = " [adapter output pipe still open at the deadline]";

/// Re-check interval for the one window where both pipes are at end-of-file but
/// the child has not been reaped yet. Documented at its only use in
/// `run_bounded`: end-of-file is not an exit event, so that window has no
/// thread left to wake the waiter.
pub(super) const EXIT_RECHECK: Duration = Duration::from_millis(1);
