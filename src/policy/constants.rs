//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::time::Duration;

/// Contract data version for the DriftWatch adapter and the
/// `PolicyObservation` it produces. Independent of DriftWatch's own
/// tool version (reported per run inside [`PolicyReport::tool_version`]).
pub const POLICY_CONTRACT_VERSION: &str = "0.1.0";

/// Ordered binary candidates for the policy plane when no explicit
/// `FORGE_DRIFTWATCH_BIN` override is set: the cargo/installer name first,
/// then the npm launcher alias. First executable hit wins.
pub const DRIFTWATCH_BINARY_CANDIDATES: &[&str] = &["driftwatchdog", "driftwatch"];

/// Sibling machine-readable checker-report contract emitted by
/// `driftwatch check --format json` (driftwatchdog change
/// `checker-machine-output`). Same-major documents may add top-level
/// fields; consumers must ignore the unknown ones.
pub const CHECKER_REPORT_CONTRACT: &str = "driftwatch-checker/0.1.0";

/// Default per-run timeout. The adapter uses `Command::spawn` + bounded
/// `wait_timeout` so an unresponsive DriftWatch cannot hang the registry.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Categories that the brief §24 names as policy planes. Used as a
/// string on findings so a future taxonomy change does not require
/// schema regeneration.
pub const POLICY_CATEGORIES: &[&str] = &[
    "architecture",
    "security",
    "privacy",
    "dependency",
    "runtime",
    "spec",
    "documentation",
    "deployment",
    "accessibility",
];
