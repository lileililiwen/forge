//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Contract data version for the component registry API and storage.
pub const COMPONENT_CATALOG_VERSION: &str = "0.1.0";

/// Default deterministic install strategy applied when a descriptor
/// does not name one explicitly.
pub const DEFAULT_INSTALL_STRATEGY: &str = "manifest-repin-plus-codegen";

/// Maximum known issues attached to a single descriptor before the
/// promotion to `Certified` is refused (a `Certified` component must
/// track its own caveats, but unbounded lists are not evidence).
pub const MAX_KNOWN_ISSUES: usize = 16;

/// Minimum test coverage ratio (`0.0`-`1.0`) required for promotion to
/// `Certified`.
pub const CERTIFIED_TEST_COVERAGE: f32 = 0.85;

/// Maximum age of a `last_verified` claim that still qualifies as
/// current evidence for `Certified` promotion. Anything older counts
/// as stale.
pub const CERTIFIED_FRESHNESS_DAYS: i64 = 180;

/// Directory (relative to the project root) holding per-component
/// promotion evidence receipts.
pub const COMPONENTS_DIR: &str = ".forge/components";

/// Programming primitives that must never be registered as semantic
/// components (requirement.md §11). The list is the boundary between
/// the component registry and ordinary language constructs.
pub(super) const PRIMITIVE_IDS: &[&str] = &[
    "if",
    "else",
    "for",
    "while",
    "loop",
    "try",
    "catch",
    "throw",
    "string-concat",
    "string-concatenation",
    "addition",
    "subtraction",
    "multiplication",
    "division",
    "comparison",
    "assignment",
    "increment",
    "decrement",
];
