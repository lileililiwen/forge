//! Digest-pinned access to the vendored `kits/` tree.
//!
//! `kits/` is deliberately the same shape as `contracts/` in
//! `platform-contract-consumption`: a checked-in directory beside the
//! crate, a `manifest.json` recording the source revision and the sha256 of
//! every vendored file, and an offline verification pass. Nothing is fetched
//! and no sibling checkout is read at render time — the bytes Forge stages
//! are the bytes it verified.
//!
//! The vendored assets are the **generated** token pair, not the DTCG source:
//! the DTCG file stays upstream-owned, and the project compiles against the
//! generated pair. A tampered byte fails generation with `kit-digest-mismatch`
//! rather than producing a project that silently differs from its descriptor.

pub mod diff;
pub mod model;
pub mod paths;
pub mod receipt;
pub mod source;
pub mod upgrade;
pub mod verification;

// Re-export all types
pub use diff::*;
pub use model::*;
pub use paths::*;
pub use receipt::*;
pub use source::*;
pub use upgrade::*;
pub use verification::*;
