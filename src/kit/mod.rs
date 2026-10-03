//! Shared-layer kit references: the compiled-in descriptors `forge new`
//! pre-wires into a scaffold (`scaffold-prewires-shared-layer`).
//!
//! A kit is a **versioned, offline descriptor** declared on
//! [`crate::profile::ProfileDescriptor`], exactly like a profile, a feature or
//! a standard pack. Nothing is fetched, resolved or discovered at render time:
//! the rendered manifest and the ownership receipt are a pure function of the
//! compiled-in descriptor plus the digest-pinned `kits/` tree.
//!
//! Four submodules carry the discipline:
//!
//! - [`registry`] — the compiled-in descriptors, the measured consumer
//!   evidence and the confirmed/provisional classification.
//! - [`floor`] — the declared minimum-consumption floor, the refusal, the
//!   recorded exception and the declared zero.
//! - [`feed`] — the repo-relative named-feed renderer, the feed-value
//!   validator, the reproducible pack command and the committed-feed drift
//!   check.
//! - [`assets`] — digest-pinned access to the vendored `kits/` tree.
//!
//! The product boundary holds: a kit is a manifest entry plus committed feed
//! bytes, never a Forge runtime dependency. A generated project restores and
//! builds through its own toolchain, with Forge absent, at any path, with no
//! sibling library and no environment variable.

pub mod assets;
pub mod feed;
pub mod floor;
pub mod registry;

pub use assets::{
    declared_kit_version, diff_kit_snapshot, kits_dir, manifest_digest, read_verified_feed_assets,
    receipt_text, render_receipt, upgrade_kit_snapshot, verify_committed_feed, verify_kit_digests,
    AssetState, FeedVerificationEntry, FeedVerificationReport, KitAssetReport, KitDiffChange,
    KitDiffEntry, KitDiffReport, KitReceipt, KitReceiptFile, KitUnavailable, KitUpgradeReport,
    PLATFORM_RECEIPT_PATH, PLATFORM_TOKENS_DIR,
};
pub use feed::{
    pack_platform_feed, refuse_source_reference, render_feed_config, validate_feed_value,
    FeedRejection, PackReport, RenderedFeed,
};
pub use floor::{
    check_floor, confirmed_count, FloorDecision, FloorException, KitExceptionRequired,
    DEFAULT_MINIMUM_PACKAGES,
};
pub use registry::{
    all_kits, classify_package, consumers_of, declared_zero, inspect_kit, kit_for_profile,
    FeedPackage, FeedPackageRole, KitDescriptor, KitPackage, KitReference, PackageClass,
    CONSUMER_BAR, KITS_FEED_DIR, NONE_KIT_ID, PLATFORM_FEED_PATH, PLATFORM_PACKAGE_EVIDENCE,
    PLATFORM_PACKAGE_VERSION, PLATFORM_REQUIRES_INFRA, PLATFORM_RESTORE_CLOSURE, PLATFORM_TFM,
};

/// Version of the kit descriptor contract itself. Bumped when the shape of
/// [`KitReference`] or [`KitDescriptor`] changes incompatibly, independently of
/// any package version a kit pins.
pub const KIT_CONTRACT_VERSION: &str = "0.1.0";
