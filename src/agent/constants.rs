//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::time::Duration;

/// Contract data version for the agent session and transition API.
pub const AGENT_CONTRACT_VERSION: &str = "0.1.0";

/// Directory (relative to the project root) holding session records.
pub const AGENTS_DIR: &str = ".forge/agents";

/// Maximum recorded transitions per session. New transitions beyond
/// the cap refuse with a typed error so the session file never
/// grows unbounded.
pub const MAX_TRANSITIONS_PER_SESSION: usize = 256;

/// Wait timeout for adapter subprocess invocations. An adapter that
/// hangs past the timeout is recorded as a failed transition with
/// `unavailable` evidence; the session file is preserved.
pub const ADAPTER_WAIT_TIMEOUT: Duration = Duration::from_secs(15);

/// Environment variable naming the `ariadex` binary. Checked
/// before the PATH probe, matching the `FORGE_DEPLOYER_BIN` /
/// `FORGE_ANALYTICS_BIN` pattern the deploy and analytics planes
/// use.
pub const ARIADEX_BIN_ENV: &str = "FORGE_ARIADEX_BIN";

/// Default PATH name for the supervised session runtime.
pub const DEFAULT_ARIADEX_BIN: &str = "ariadex";

/// Environment variable naming the `sisyphusfy` iteration
/// supervisor binary.
pub const SISYPHUSFY_BIN_ENV: &str = "FORGE_SISYPHUSFY_BIN";

/// Default PATH name for the spec-execution supervisor.
pub const DEFAULT_SISYPHUSFY_BIN: &str = "sisyphusfy";

/// The only run-spec supervisor id accepted by
/// `forge agent run-spec --provider` and the MCP `run_agent`
/// `provider` argument on a `run_spec` transition.
pub const SISYPHUSFY_SUPERVISOR: &str = "sisyphusfy";

/// Bounded wait for a delegated `sisyphusfy run`. The supervisor
/// drives a whole iteration loop, so this is deliberately far
/// longer than a lifecycle probe; a supervisor that exceeds it is
/// recorded as `unverified`, never as `done`.
pub const RUN_SPEC_WAIT_TIMEOUT: Duration = Duration::from_secs(900);

/// Character bound for every single captured runtime-output line
/// promoted into session evidence (redaction applies first).
pub const MAX_EVIDENCE_CHARS: usize = 300;
