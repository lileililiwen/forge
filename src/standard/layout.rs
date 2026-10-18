//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Repository-local snapshot directory. Most owned files live under it. A pack
/// may also own a small number of repository-relative paths outside it (the
/// shared scripts the workspace Gate binds by name), so ownership is carried by
/// the receipt's per-file digests rather than by this prefix: `check`, `diff`
/// and `upgrade` read exactly the receipt's paths, an existing owned path with
/// different bytes is a conflict, and an unrelated file is never touched.
pub const STANDARD_DIR: &str = ".standard";

/// Profile declaration rendered into the snapshot.
pub const PROFILE_PATH: &str = ".standard/profile.yaml";

/// Ownership receipt recording the pack identity and per-file digests.
pub const RECEIPT_PATH: &str = ".standard/receipt.json";

/// Verification entry point the project (and its CI) runs locally.
pub const VERIFY_PATH: &str = ".standard/scripts/verify.sh";

/// Checksummed Forge release installer carried by current baseline packs.
pub const INSTALL_FORGE_PATH: &str = ".standard/scripts/install-forge.sh";

/// CI workflow definition carried by the snapshot.
pub const CI_PATH: &str = ".standard/ci/verify.yml";

/// GitHub Actions discovers workflow files only under this repository path.
pub const GITHUB_CI_PATH: &str = ".github/workflows/forge-gate.yml";

/// Pre-commit framework configuration rendered at the repository root.
pub const PRE_COMMIT_PATH: &str = ".pre-commit-config.yaml";

/// Quality configuration carried by the snapshot.
pub const QUALITY_PATH: &str = ".standard/quality/quality.yaml";

/// Compose family selection carried by the snapshot.
pub const COMPOSE_PATH: &str = ".standard/compose/docker-compose.yaml";

/// Snapshot document schema version.
pub const STANDARD_SCHEMA: u8 = 1;

/// Pack family mapped to every supported server profile.
pub const BASELINE_PACK: &str = "baseline-service";

/// Gate-only baseline for the current client profiles.
pub const CLIENT_PACK: &str = "baseline-client";

/// Default snapshot for compatible new projects. Bump only when a new
/// supported, verified pack version becomes the generation default.
pub const DEFAULT_BASELINE_PACK_SPEC: &str = "baseline-service@1.6.0";

/// Default snapshot for client profiles with verified Gate-only templates.
pub const DEFAULT_CLIENT_PACK_SPEC: &str = "baseline-client@1.3.0";

/// Service profiles supported by `baseline-service`.
pub const BASELINE_PROFILES: &[&str] = &["aspnet-web", "nextjs-web", "python-service", "rust-web"];

/// Client profiles receive the Gate-only pack until their native CI baseline
/// has its own verified language verifier.
pub const CLIENT_PROFILES: &[&str] = &["flutter-app", "react-web"];

/// Deterministic timestamp used by the pure renderer (`render_files`), so
/// byte-equality tests are not defeated by the wall clock. Real generation
/// substitutes the actual emission time.
pub const DETERMINISTIC_TIMESTAMP: &str = "1970-01-01T00:00:00+00:00";

pub(super) const CI_TEMPLATE: &str = "# Forge standard snapshot CI ({pack}@{version}).\nname: verify\non:\n  push:\n  pull_request:\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - name: standard verify\n        run: sh .standard/scripts/verify.sh\n";

/// Forge-owned service CI template for the Gate-enabled baseline version.
pub(super) const GATE_CI_TEMPLATE: &str =
    include_str!("../../standards/baseline-service/1.3.0/ci/verify.yml");

/// Release-backed service CI asset for the current baseline pack.
pub(super) const RELEASE_CI_TEMPLATE: &str =
    include_str!("../../standards/baseline-service/1.4.0/ci/verify.yml");

/// Release-backed client CI asset for the current baseline pack.
pub(super) const RELEASE_CLIENT_CI_TEMPLATE: &str =
    include_str!("../../standards/baseline-client/1.1.0/ci/verify.yml");

/// Pinned Forge release installer carried by both current baseline packs.
pub(super) const RELEASE_INSTALLER_TEMPLATE: &str =
    include_str!("../../standards/baseline-service/1.4.0/install-forge.sh");

/// Client pack installer bytes, kept under the client's immutable pack tree.
pub(super) const RELEASE_CLIENT_INSTALLER_TEMPLATE: &str =
    include_str!("../../standards/baseline-client/1.1.0/install-forge.sh");

pub(super) const HOSTED_CI_SERVICE_WORKFLOW: &str =
    include_str!("../../standards/baseline-service/1.5.0/ci/verify.yml");
pub(super) const HOSTED_CI_CLIENT_WORKFLOW: &str =
    include_str!("../../standards/baseline-client/1.2.0/ci/verify.yml");
pub(super) const HOSTED_CI_SERVICE_INSTALLER: &str =
    include_str!("../../standards/baseline-service/1.5.0/install-forge.sh");
pub(super) const HOSTED_CI_CLIENT_INSTALLER: &str =
    include_str!("../../standards/baseline-client/1.2.0/install-forge.sh");
pub(super) const HOSTED_CI_UPLOADER: &str =
    include_str!("../../standards/baseline-service/1.5.0/upload-hosted-ci-evidence.py");

/// Forge-owned pre-commit entry point for the Gate-enabled baseline version.
pub(super) const PRE_COMMIT_TEMPLATE: &str =
    include_str!("../../standards/baseline-service/1.3.0/.pre-commit-config.yaml");

/// Gate-only CI asset for the client profile pack.
pub(super) const CLIENT_CI_TEMPLATE: &str =
    include_str!("../../standards/baseline-client/1.0.0/ci/verify.yml");

/// Pre-commit hook asset for the client profile pack.
pub(super) const CLIENT_PRE_COMMIT_TEMPLATE: &str =
    include_str!("../../standards/baseline-client/1.0.0/.pre-commit-config.yaml");

pub(super) const QUALITY_TEMPLATE: &str =
    "schema: 1\npack: {pack}\nversion: {version}\nchecks:\n  - verify\n";

pub(super) const COMPOSE_TEMPLATE: &str = "# Forge standard snapshot Compose selection ({pack}@{version}).\nservices:\n  {id}:\n    build:\n      context: ../..\n      dockerfile: Dockerfile\n";

/// Repository-relative path of the shared CI rehearsal script. It must live at
/// the repository root, not under `.standard/`: the script derives its working
/// directory from its own location, so any other placement would run it against
/// the wrong tree (measured: installed under `.standard/scripts/` it reports
/// "No recognized project type here" and exits 0 — a silent false pass).
pub const VERIFY_LOCAL_PATH: &str = "verify-local.sh";

/// Snapshot verification entry point: a Forge-owned wrapper that runs the
/// repository-root rehearsal script from the project root, so
/// `.standard/ci/verify.yml` and a human both invoke the same checks.
pub(super) const VERIFY_WRAPPER_TEMPLATE: &str = "#!/usr/bin/env sh\n# Forge standard snapshot {pack}@{version}: run the project's shared rehearsal script.\nset -eu\nROOT=\"$(cd \"$(dirname \"$0\")/../..\" && pwd)\"\ncd \"$ROOT\"\nif [ ! -f ./verify-local.sh ]; then\n  echo \"verify-local.sh is missing from the project root; the standard snapshot's rehearsal script was removed\" >&2\n  exit 1\nfi\nexec sh ./verify-local.sh \"$@\"\n";

/// Repository-relative path of the shared OpenSpec change-name checker. This
/// exact path is what the workspace Gate generator binds
/// (`workspace-governance/scripts/bind_gate_commands.py`), so the pack must own
/// it there and not under `.standard/`.
pub const OPENSPEC_NAME_CHECK_PATH: &str = "scripts/check-openspec-change-names.mjs";

/// Repository-relative path of the shared accessibility concern shim, bound by
/// the same generator for the `accessibility` concern.
pub const ACCESSIBILITY_CHECK_PATH: &str = "scripts/check-accessibility.py";

/// Repository-relative path of the shared privacy concern shim.
pub const PRIVACY_CHECK_PATH: &str = "scripts/check-privacy.py";

/// Pinned content of the shared CI rehearsal script, owned at the project root.
/// Authored by `workspace-governance` and bundled here so a snapshot renders
/// with no sibling checkout present. Provenance and digests:
/// `standards/baseline-service/1.2.0/PROVENANCE.md`.
pub(super) const SHARED_VERIFY_SCRIPT: &str =
    include_str!("../../standards/baseline-service/1.2.0/verify-local.sh");

/// Pinned content of the shared OpenSpec change-name checker.
pub(super) const SHARED_OPENSPEC_NAME_CHECK: &str =
    include_str!("../../standards/baseline-service/1.2.0/scripts/check-openspec-change-names.mjs");

/// Pinned content of the shared accessibility concern shim.
pub(super) const SHARED_ACCESSIBILITY_CHECK: &str =
    include_str!("../../standards/baseline-service/1.2.0/scripts/check-accessibility.py");

/// Pinned content of the shared privacy concern shim.
pub(super) const SHARED_PRIVACY_CHECK: &str =
    include_str!("../../standards/baseline-service/1.2.0/scripts/check-privacy.py");
