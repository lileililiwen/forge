//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Contract data version for the UI pattern catalog API and storage.
pub const UI_PATTERN_CATALOG_VERSION: &str = "0.1.0";

/// Default deterministic install strategy applied when a pattern
/// does not name one explicitly. The installer writes a real
/// source artifact plus a separate metadata receipt; the source
/// is ordinary, editable, and continues to build through the
/// project native toolchain when Forge is removed.
pub const DEFAULT_INSTALL_STRATEGY: &str = "manifest-repin-plus-ordinary-source";

/// Maximum known issues attached to a single pattern before the
/// promotion to `Certified` is refused.
pub const MAX_KNOWN_ISSUES: usize = 16;

/// Minimum test coverage ratio required for promotion to
/// `Certified`.
pub const UI_PATTERN_CERTIFIED_TEST_COVERAGE: f32 = 0.85;

/// Maximum age of a `last_verified` claim that still qualifies as
/// current evidence for `Certified` promotion.
pub const UI_PATTERN_CERTIFIED_FRESHNESS_DAYS: i64 = 180;

/// Directory (relative to the project root) holding per-pattern
/// install receipts and source artifacts.
pub const UI_PATTERNS_DIR: &str = ".forge/ui-patterns";

/// Intent vocabulary for the semantic UI catalog. The list is
/// bounded on purpose: every pattern must name one of these
/// intents so a future planner can group patterns by purpose
/// without parsing free-form text. An unknown intent is refused
/// by [`validate_descriptor`].
pub const UI_PATTERN_INTENTS: &[&str] = &[
    "login",
    "register",
    "forgot-password",
    "dashboard",
    "crud-table",
    "filter-bar",
    "form",
    "settings",
    "profile",
    "billing",
    "empty-state",
    "success-page",
    "error-page",
    "modal",
    "confirm-dialog",
    "file-upload",
    "navigation",
];

/// State contract a pattern must surface. The brief (§13) calls
/// out loading, error, success, form, accessibility and
/// navigation; the contract here makes them first-class so a
/// copied markup fragment without these states is refused at
/// validation time.
pub const UI_PATTERN_REQUIRED_STATES: &[&str] = &[
    "loading",
    "error",
    "success",
    "form_validation",
    "empty",
    "keyboard_focus",
];

/// Programming primitives that must never be registered as
/// semantic UI patterns. The list mirrors the component
/// registry's primitive list (requirement.md §11) plus a
/// handful of generic template placeholders.
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
    "screenshot",
    "html-fragment",
    "copy-paste",
    "lorem-ipsum",
];
