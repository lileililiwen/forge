//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Repository-local snapshot directory. Forge owns exactly this subtree:
/// every owned file lives under it, so an upgrade can never touch unrelated
/// project files.
pub const STANDARD_DIR: &str = ".standard";

/// Profile declaration rendered into the snapshot.
pub const PROFILE_PATH: &str = ".standard/profile.yaml";

/// Ownership receipt recording the pack identity and per-file digests.
pub const RECEIPT_PATH: &str = ".standard/receipt.json";

/// Verification entry point the project (and its CI) runs locally.
pub const VERIFY_PATH: &str = ".standard/scripts/verify.sh";

/// CI workflow definition carried by the snapshot.
pub const CI_PATH: &str = ".standard/ci/verify.yml";

/// Quality configuration carried by the snapshot.
pub const QUALITY_PATH: &str = ".standard/quality/quality.yaml";

/// Compose family selection carried by the snapshot.
pub const COMPOSE_PATH: &str = ".standard/compose/docker-compose.yaml";

/// Snapshot document schema version.
pub const STANDARD_SCHEMA: u8 = 1;

/// Pack family mapped to every supported server profile.
pub const BASELINE_PACK: &str = "baseline-service";

/// Profiles the baseline pack supports. Client-only profiles
/// (`flutter-app`, `react-web`) map to no pack and render nothing.
pub const BASELINE_PROFILES: &[&str] = &["aspnet-web", "nextjs-web", "python-service", "rust-web"];

/// Deterministic timestamp used by the pure renderer (`render_files`), so
/// byte-equality tests are not defeated by the wall clock. Real generation
/// substitutes the actual emission time.
pub const DETERMINISTIC_TIMESTAMP: &str = "1970-01-01T00:00:00+00:00";

pub(super) const CI_TEMPLATE: &str = "# Forge standard snapshot CI ({pack}@{version}).\nname: verify\non:\n  push:\n  pull_request:\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - name: standard verify\n        run: sh .standard/scripts/verify.sh\n";

pub(super) const QUALITY_TEMPLATE: &str =
    "schema: 1\npack: {pack}\nversion: {version}\nchecks:\n  - verify\n";

pub(super) const COMPOSE_TEMPLATE: &str = "# Forge standard snapshot Compose selection ({pack}@{version}).\nservices:\n  {id}:\n    build:\n      context: ../..\n      dockerfile: Dockerfile\n";
