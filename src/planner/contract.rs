//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Contract data version for the planner API and persisted plan receipts.
pub const PLANNER_CONTRACT_VERSION: &str = "0.1.0";

/// Maximum number of capabilities (required + forbidden + unresolved)
/// accepted in a single intent; larger requests are refused to keep
/// the validated intent bounded.
pub const MAX_CAPABILITIES_PER_INTENT: usize = 32;

/// Maximum number of unresolved custom-work entries the planner
/// reports; a request whose every requirement is satisfiable emits an
/// empty list, and a request with too many gaps is refused so the
/// plan is never silently truncated.
pub const MAX_UNRESOLVED_PER_PLAN: usize = 32;

/// Directory (relative to the project root) holding persisted plan
/// receipts and the normalized intent that produced them. The receipt
/// is the only persisted state the planner owns; the validator and
/// resolver are otherwise pure.
pub const PLANS_DIR: &str = ".forge/planner";

/// Default backend profile hint surfaced when a client-only profile
/// is paired with a server-side capability.
pub(super) const DEFAULT_BACKEND_HINTS: &[&str] = &["rust-web", "python-service"];

/// Server-side capabilities a client-only profile cannot serve
/// (mirror of `profile::SERVER_SIDE_CAPABILITIES`).
pub(super) const SERVER_SIDE_CAPABILITIES: &[&str] = &[
    "postgres",
    "redis",
    "background-jobs",
    "storage",
    "email",
    "audit",
    "rate-limit",
];

/// Client-only profile ids (mirror of `profile`'s boundary rule).
pub(super) const CLIENT_ONLY_PROFILES: &[&str] = &["flutter-app", "react-web"];

/// Constraints the validator recognises. Unknown keys are refused so
/// a model cannot smuggle a flag the executor would silently honour.
pub(super) const KNOWN_CONSTRAINT_KEYS: &[&str] = &["public", "deploy"];
