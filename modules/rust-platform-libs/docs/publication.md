# Publishing the platform crates to crates.io

This document defines the version, yank, and token policy for the
`platform-*` crates, plus the pre-publish checklist the owner follows
when releasing. The `registry-publication` OpenSpec change ships the
pipeline; this file is the policy that pipeline enforces and the
reference humans read when they cut a release.

## Lockstep versioning

The workspace ships one version across all publishable crates. The
version lives in `[workspace.package] version` in the root
`Cargo.toml`; every crate manifest inherits it through
`version.workspace = true`. The publish order
([`.publish-order`](../.publish-order)) is the single source of truth
for the topological sequence in which the crates are uploaded.

A version bump MUST land in one commit together with:

1. The `version` line in the root `Cargo.toml`.
2. The corresponding `rust-platform-libs-vX.Y.Z` tag (created at release time, not in
   the version-bump commit).
3. A new entry in [Changelog](#changelog) below.

Crates are not versioned independently. The contract families
(`identity-subject`, `permission-decision`, `tenant-context`,
`audit-event`, `readiness`, `gate-result`, `job-outcome`,
`release-evidence`, `capability`) are co-released because the
language-neutral schema and the Rust types evolve together; an
out-of-band version of any one crate would break the
`platform-contracts` parity walker.

## Pre-1.0 semver rule

The workspace is at `0.y.z`. Under cargo's pre-1.0 convention:

* A patch bump (`0.y.(z+1)`) is reserved for backward-compatible
  changes and bug fixes that do not alter any public API.
* A minor bump (`0.(y+1).0`) is the only way to ship a breaking
  change while the workspace is pre-1.0. The release MUST be
  accompanied by a `Breaking:` line in the changelog naming the
  affected crate(s) and the new API.
* A major bump (`(X+1).0.0`) is a future event when the foundation
  commits to API stability. Until then, the version stays below
  `1.0.0`.

`0.1.0` is the reserved first release; subsequent `0.1.x` patches
keep the API surface unchanged. Any intentional break is a
`0.2.0` (or later) and MUST carry the changelog notice.

## Yank procedure

crates.io versions are immutable: once a version is published, the
source it points at cannot be edited. A bad release is corrected
by yanking, not by re-publishing.

Yank rules:

* Yank is a manual `cargo yank` per crate, executed by the owner.
* Every yank MUST add a `Yanked:` line to the changelog below,
  naming the yanked version, the reason, and the replacement
  version (typically the next patch that fixes the defect).
* A yanked version stays on the registry (the `Cargo.toml` line
  is preserved) but new `cargo add` calls cannot pick it up by
  default; consumers who already depend on it keep building.
* A yanked version is NOT deleted from the registry; the
  registry does not support deletion. Do not promise deletion
  in any release note.

If a published version has a defect that consumers can mitigate
by upgrading, the patch is the right tool, not yank: bump the
patch version, fix the defect, and tell consumers to upgrade.
Yank is reserved for versions that are unsafe or misleading to
use even as a transitive dependency.

## Token handling

The `CARGO_REGISTRY_TOKEN` repository secret gates every release.
The token MUST be crate-scoped to `platform-*` (not user-wide),
time-limited, and rotated whenever a maintainer with the token
leaves the maintainer group. The token-creation recipe is:

1. Visit <https://crates.io/settings/tokens> and create a token
   with the scope `publish` for the `platform-*` crates.
2. Store the token in the repository's
   `CARGO_REGISTRY_TOKEN` secret (Settings → Secrets and
   variables → Actions → New repository secret).
3. Never paste the token into a commit, an issue, or a PR
   comment. If the token is exposed, revoke it immediately and
   create a replacement.

The publish workflow refuses to run without the secret. A leaked
secret is a release-stopping event: rotate before retrying.

## Pre-publish checklist

The first release remains BLOCKED until the module revision is available from
the published Forge repository, the owner adds the tracked
`modules/rust-platform-libs/PUBLICATION-APPROVED` marker, and the repository
secret `CARGO_REGISTRY_TOKEN` is configured. The release workflow checks the
marker and token before registry calls. The operator also supplies the exact
version and types `approve` for a manual dispatch; the workflow requires a
`rust-platform-libs-vX.Y.Z` tag for tag-triggered releases and matches its
version against `[workspace.package] version`.

Before the first release, verify that
`[workspace.package] repository = "https://github.com/lileililiwen/forge"`
and the module homepage point at the published Forge repository and that the
tagged revision contains `modules/rust-platform-libs/`. No GitHub remote,
approval marker, secret, tag or crate publication is created by local
verification.

When every precondition is met, the release steps are:

```sh
# One-time per release: bump the workspace version in Cargo.toml,
# commit, and tag.
$EDITOR Cargo.toml   # bump `version = "0.1.0"` to `0.1.1` (or `0.2.0` for a break)
git commit -am "Bump workspace version to 0.1.1"
git tag rust-platform-libs-v0.1.1
git push origin main rust-platform-libs-v0.1.1
```

The tag push triggers `.github/workflows/platform-rust-publish.yml`, which runs
the prechecks, the name pre-flight, the topological publish, and
the propagation polls. The release is complete when the workflow
ends green and every crate in `.publish-order` shows the new
version on <https://crates.io>.

## Changelog

This section records every published version, every yank, and
every breaking change. Entries are appended, never edited, so
the history is auditable.

| Version | Date | Notes |
|---|---|---|
| _pending_ | _pending_ | The first `v0.1.0` release is BLOCKED on repository availability, owner approval, and the registry token in the pre-publish checklist above. |
