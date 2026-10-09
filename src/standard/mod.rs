//! Versioned standard-pack registry and standalone project snapshots
//! (`standard-pack-registry-and-snapshots`).
//!
//! A *standard pack* is a versioned descriptor that materializes a
//! repository-local convention set into a generated project: a profile
//! declaration, a verification entry point, CI and quality configuration, a
//! Compose selection, and an ownership receipt. Packs are compiled-in
//! descriptors (like profiles and features) so their identity, version,
//! compatibility and asset digest are stable and never fetched at runtime.
//!
//! Generation writes the snapshot under `.standard/` and records an
//! ownership receipt ([`RECEIPT_PATH`]) holding the pack identity, the
//! descriptor's asset digest, and one digest per owned file. The generated
//! project owns that snapshot: it builds, verifies and operates with its own
//! native toolchain and needs neither Forge nor a sibling checkout later.
//!
//! The lifecycle is `proposed -> supported -> deprecated`; only `supported`
//! packs with verified render fixtures are selectable for new generation.
//! Snapshot lifecycle is `absent -> rendered -> modified -> upgrade-planned ->
//! upgraded`; a user edit makes the bytes diverge from the receipt and turns
//! an upgrade into a reviewable conflict rather than a silent merge.
//!
//! Every mutation is explicit: `diff` is read-only and `upgrade` refuses
//! modified owned files (and unowned collisions) unless `--force` supplies a
//! reviewed resolution. Rendering and upgrades never contact the network and
//! never invent files.

pub mod layout;
pub mod model;
pub mod snapshot;
pub mod snapshot_tests;

// Re-export all types
pub use layout::*;
pub use model::*;
pub use snapshot::*;
