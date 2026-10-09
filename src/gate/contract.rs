//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::time::Duration;

/// Versioned contract for the persisted [`GateEvidence`] record. The
/// sibling's own gate document carries no contract field and is
/// discriminated by shape, exactly as the policy plane does.
pub const GATE_CONTRACT_VERSION: &str = "0.1.0";

/// Environment override naming the gate binary explicitly. Like every
/// other adapter override it runs exactly the named binary with no
/// fall-through to PATH probing.
pub const GATE_BIN_ENV: &str = "FORGE_GATE_BIN";

/// Gate evidence lives at `<project>/.forge/gate/<project-id>/evidence.json`
/// — one latest record per project; run history belongs to the operations
/// table, matching how the release and deploy planes split state.
pub const GATE_EVIDENCE_DIR: &str = ".forge/gate";

pub const GATE_EVIDENCE_FILE: &str = "evidence.json";

/// Gates can be long: the default bounded wait is deliberately far larger
/// than the policy-plane default, and `--timeout-secs` may raise it to
/// this hard maximum. Values outside `MIN..=MAX` refuse before any spawn.
pub const DEFAULT_GATE_TIMEOUT: Duration = Duration::from_secs(600);

pub const MIN_GATE_TIMEOUT_SECS: u64 = 1;

pub const MAX_GATE_TIMEOUT_SECS: u64 = 86_400;

/// Bounded stdout capture (mirrors the governance adapter bound) so a
/// chatty runtime cannot flood the evidence file or the journal.
pub const MAX_GATE_OUTPUT_BYTES: usize = 256 * 1024;

/// Character bound on any single captured note, check annotation or plan
/// line (mirrors the supervised-agent evidence bound).
pub const MAX_NOTE_CHARS: usize = 300;

/// Bound on plan-preview lines kept from a `gate --dry-run` rehearsal.
pub const MAX_PLAN_LINES: usize = 40;

/// The only runtime names with a packaged resolution path. A declaration
/// naming anything else is refused rather than silently probed.
pub const SUPPORTED_GATE_RUNTIMES: &[&str] = &["driftwatchdog"];
