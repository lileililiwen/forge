//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Owned subtree in a generated project holding the vendored token source.
pub const PLATFORM_TOKENS_DIR: &str = ".platform/tokens";

/// Ownership receipt for the kit-owned subtree. A separate receipt rather
/// than a `.standard/` pack, because reusing that machinery requires a
/// standard-pack descriptor owned by the sibling library, which is outside
/// this repository's boundary. The shapes and the refusal vocabulary are
/// deliberately identical to the standard snapshot receipt.
pub const PLATFORM_RECEIPT_PATH: &str = ".platform/receipt.json";

/// Version of the kit receipt document.
pub const PLATFORM_RECEIPT_SCHEMA: u8 = 1;
