//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Contract data version for the procedure API and CLI. Bumped
/// when a new procedure is added or an existing step's
/// operation changes.
pub const PROCEDURE_CONTRACT_VERSION: &str = "0.1.0";

/// Maximum number of steps accepted in a single procedure
/// spec; a larger request is refused with `procedure-invalid`
/// so the procedure layer never silently truncates an SOP.
pub const MAX_PROCEDURE_STEPS: usize = 16;

/// Maximum number of arguments accepted in a single procedure
/// step; a larger request is refused with `procedure-invalid`
/// to keep the procedure layer bounded.
pub const MAX_STEP_ARGS: usize = 16;

/// Bypass markers that a step may not smuggle into a Core
/// call. Any step whose `args` carry one of these tokens is
/// refused by [`validate_procedure`] with the typed
/// `procedure-bypass-refused` code (R2 failure scenario).
/// Matched case-insensitively against the leading `--` flag
/// form (`--force`, `--Force`, `--FORCE`, `--skip-checks`,
/// etc.). Plain occurrences inside a value (e.g. a
/// description that mentions `--force`) are also rejected
/// because the procedure layer never accepts a step that
/// would tell Core to skip its own validation.
pub const BYPASS_MARKERS: &[&str] = &[
    "--force",
    "--skip-checks",
    "--no-validate",
    "--bypass",
    "--override",
    "--no-doctor",
    "--ignore-failures",
];

/// Stable id of the synthetic project used for procedure
/// operations journaled in the registry. Keeping the
/// operations table project-agnostic mirrors the
/// `__planner__` and `__ui_pattern__` patterns.
pub const PROCEDURE_SYNTHETIC_PROJECT: &str = "__procedure__";
