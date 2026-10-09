//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::time::Duration;

/// Versioned contract for the provider evidence surface.
pub const PROVIDER_CONTRACT_VERSION: &str = "0.1.0";

/// Synthetic project id for project-agnostic evidence rows. Keeps the
/// `operations` table project-agnostic without inventing a user-visible
/// project, matching the `__planner__` / `__component__` pattern.
pub const PROVIDER_SYNTHETIC_PROJECT: &str = "__provider__";

/// Opt-in flag: live sandbox binaries are attempted only when
/// `FORGE_PROVIDER_LIVE=1`. Anything else reports `not-run` so an
/// unconfigured sandbox is never misread as a result.
pub const LIVE_ENV: &str = "FORGE_PROVIDER_LIVE";

/// Bounded wait per probe so an unresponsive binary cannot hang the
/// registry. Shorter than the per-adapter defaults because a probe is a
/// reachability + envelope-shape check, not a full provider run.
pub const EVIDENCE_TIMEOUT: Duration = Duration::from_secs(10);

/// Bounded receipt excerpt per probe so a chatty binary cannot flood the
/// journal. The full output stays with the provider; Forge keeps proof.
pub const MAX_RECEIPT_CHARS: usize = 2000;

/// Evidence provider ids in stable catalog order.
pub const PROVIDER_IDS: &[&str] = &[
    "driftwatch-policy",
    "gate-runtime",
    "oidc-identity",
    "analytics",
    "deploy",
    "release",
];

/// Default binaries probed for `live` runs, mirroring each adapter's own
/// default. Every default is overridable per run so tests substitute
/// fixture scripts without touching the source tree.
///
/// The DriftWatch probe order is NOT listed here: `driftwatch-policy`
/// and `gate-runtime` share [`crate::policy::DRIFTWATCH_BINARY_CANDIDATES`]
/// as the one ordered definition (see [`default_binary_for`]), so the
/// gate, policy and provider surfaces can never disagree on candidate
/// order. The gate's *declared* runtime names stay a strict subset
/// (`SUPPORTED_GATE_RUNTIMES` in `src/gate/mod.rs`): declaration
/// vocabulary and probe order are different roles.
pub const DEFAULT_PROBE_BINARIES: &[(&str, &str)] = &[
    ("analytics", "forge-analytics-adapter"),
    ("deploy", "forge-deployer"),
    ("release", "forge-package-publisher"),
];

/// Per-run environment override for the probed binary, mirroring each
/// adapter's `FORGE_*_BIN` pattern.
pub const PROBE_BIN_ENV: &[(&str, &str)] = &[
    ("driftwatch-policy", "FORGE_DRIFTWATCH_BIN"),
    ("gate-runtime", "FORGE_GATE_BIN"),
    ("analytics", "FORGE_ANALYTICS_BIN"),
    ("deploy", "FORGE_DEPLOYER_BIN"),
    ("release", "FORGE_PACKAGE_BIN"),
];
