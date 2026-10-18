# Design: portable Forge binary distribution

## 1. Implementation boundary

The implementation stays in `/home/paul/code/forge` on `main`.

- `.github/workflows/ci.yml` owns CI gating and the tagged GitHub Release
  publication job.
- `scripts/package.sh`, `scripts/checksum.sh`, `scripts/install.sh` and
  `scripts/smoke.sh` remain the single archive/checksum/local-install
  implementation. The release job consumes the already verified `artifact`
  job output.
- `scripts/validate-release-tag.sh` is a small POSIX shell guard that compares
  `GITHUB_REF_NAME` to `v` plus the package version read from `Cargo.toml`.
- `scripts/checksum.sh` continues to define sidecar content as
  `<sha256><two spaces><basename>\n`.
- `scripts/contract-parity.sh` remains a deterministic local comparison.
  Only `.github/workflows/ci.yml` supplies its upstream source, using the
  immutable revision stored in `contracts/manifest.json`.
- `standards/baseline-service/1.4.0/` and
  `standards/baseline-client/1.1.0/` add immutable verifier/CI assets. The CI
  template source is the `.standard/ci/verify.yml` file; the active workflow
  `.github/workflows/forge-gate.yml` receives the same bytes. Each descriptor
  is registered in `src/standard/snapshot.rs::all_packs`; defaults are set in
  `src/standard/layout.rs`.
- `tests/standard_contract.rs`, `src/standard/snapshot_tests.rs`,
  `tests/standard_cross_surface.rs`, and a focused CI release contract test
  prove rendering, default selection, old-pack compatibility, publication
  gates and checksum behavior.
- `docs/release-readiness.md`, `README.md`, and pack provenance document
  supported target, tag/release process, first-release prerequisite and
  consumer install behavior. `ROADMAP.md` corrects the stale previous row and
  records package state.

No source module, public Rust API, persistent schema, generated application
runtime or sibling repository is changed.

## 2. Language, runtime and toolchain

- Forge implementation: Rust 2021, minimum Rust 1.88, current pinned
  `Cargo.lock`; no dependency additions.
- Release runner: GitHub-hosted `ubuntu-latest`, whose host target must be
  exactly `x86_64-unknown-linux-gnu` for this package. `scripts/package.sh`
  uses the host target from `rustc -vV`; CI verifies the filename and target.
- Scripts and generated installer: POSIX `sh`, `curl`, GNU `sha256sum`, GNU
  `tar`, `mktemp`, `awk`; these are available on the declared Ubuntu runner.
- CI orchestration: GitHub Actions YAML. Release publication uses the
  runner-provided `gh` command and `GH_TOKEN` only in a job with
  `permissions.contents: write`.
- Local checks include focused `cargo test`, full
  `RUSTC_WRAPPER= CARGO_INCREMENTAL=0 timeout 600 scripts/check-tests.sh`,
  workspace build, strict OpenSpec validation, `forge gate --dry-run` and the
  bounded full `forge gate` required by `AGENTS.md`.

## 3. Ownership and shared code

Forge owns its executable archive, checksum format, generated release
installer, release workflow and standard pack bytes. The `platform-contracts`
repository remains the owner of schema bytes; the parity CI job checks out the
exact revision named by Forge's local `contracts/manifest.json` and performs
the existing read-only comparison. The source checkout is ephemeral under
`$RUNNER_TEMP`, never a runtime dependency or submodule.

Workspace Governance remains the source of shared verifier guidance; Forge's
already bundled and pinned copies are unchanged. The .NET and Rust library
repositories retain their own package release boundaries. No shared release
library extraction is justified: the artifact, tag, platform and digest
contracts here are Forge-specific and have no current second consumer.

## 4. User experience and interface

There is no new Forge web route or local wizard. The human release surface is
the Git tag and the named jobs/steps in GitHub Actions; the consumer surface is
the generated project's existing workflow run.

1. Maintainer ensures the changelog and `Cargo.toml` version agree (existing
   artifact contract), creates and pushes `v<version>`.
2. The normal CI job set runs. The release job is skipped on branch and pull
   request refs; on a tag it waits for every required job, including
   `contract-parity` and `artifact`.
3. The artifact job's archive and sidecar are downloaded; the release job
   checks the tag, asset names and checksums, then calls
   `gh release create --verify-tag` without `--clobber`. An existing release
   fails visibly and is not altered.
4. A generated project's `Install pinned Forge release` step downloads only
   the pinned version's two assets, verifies a one-line checksum with the
   exact archive basename, validates the archive contains the expected
   `forge-<version>-<target>/forge` regular file, and puts the executable in
   `$RUNNER_TEMP/forge/bin` via `GITHUB_PATH`.
5. Existing language verification and `forge gate .` then run unchanged.

Missing tag, failed CI job, no release yet, 404, timeout, bad sidecar, digest
mismatch, tar failure, unexpected member, non-executable binary or Gate
failure produces a failed Actions step with the named cause. The installer
does not execute downloaded content before verification. The job permissions
stay read-only for consumers and are limited to `contents: write` for the
single publication job. GitHub-hosted Actions supplies keyboard-accessible
job navigation and plain-text logs; stable names are used for those actors.
Responsive layout and assistive-technology UI work in Forge itself are not
applicable because no Forge UI is changed.

## 5. Behavioral model

### Version and release identity

| Input | Validation | Output/side effect |
|---|---|---|
| Push tag `v<semver>` | Tag exactly equals `v` plus the version in the checked-out `Cargo.toml`; package/checksum/smoke step passes; all CI jobs succeed | Existing archive and `.sha256` sidecar are uploaded to one GitHub Release at that tag |
| Branch or pull request | Not a version-tag push | Normal CI runs; release job is skipped and no `contents: write` job runs |
| Mismatched or malformed tag | Exact string comparison fails | Release job fails before release creation |
| Existing GitHub Release | `gh release create --verify-tag` refuses existing release; no `--clobber` or asset-delete command | No asset is overwritten; publish step fails for operator review |
| Template default | Service profiles select `baseline-service@1.4.0`; client profiles select `baseline-client@1.1.0` | New generation writes the matching owned snapshot and receipt |
| Existing snapshot | Receipt still names its original pack version | No implicit update; `standard upgrade` plans the new version and preserves conflict checks |

The release job downloads the workflow artifact by the unique CI artifact name
and uses the exact `forge-<version>-x86_64-unknown-linux-gnu.tar.gz` and
`.sha256` pair. The publisher re-runs checksum verification before invoking
`gh`. The release tag is already present because the event is a tag push;
`--verify-tag` prevents accidental creation against a missing tag.

### Generated installer

The installer uses compile-time standard asset bytes with pinned values:

- repository: `lileililiwen/forge`;
- release tag: `v0.1.0` (the current Cargo version at authoring time);
- target: `x86_64-unknown-linux-gnu`;
- archive basename: `forge-0.1.0-x86_64-unknown-linux-gnu.tar.gz`.

When Forge's Cargo version changes, the next default standard pack must be
versioned with the new binary pin. The installer never uses `latest`, checks
the checksum sidecar names exactly the expected basename, computes SHA-256
locally and compares before extracting, and requires exactly one expected
binary file. It writes only below `$RUNNER_TEMP/forge` and appends the binary
directory to the Actions-provided `GITHUB_PATH` file after installation
succeeds. A missing environment variable or executable fails before Gate.

## 6. Contract and compatibility

- Release tag: `v` + exact Cargo SemVer (currently `v0.1.0`).
- Release assets: `forge-<version>-x86_64-unknown-linux-gnu.tar.gz` and the
  same filename plus `.sha256`.
- Checksum format: exactly one sidecar line with lowercase SHA-256, two spaces,
  and the archive basename. No user-controlled filename is passed to `tar`.
- Archive format remains the existing one-root-directory gzip tarball with
  `forge`, `LICENSE`, `README.md` and `CHANGELOG.md`; package test checks
  layout and rejects absolute paths.
- GitHub consumer URL:
  `https://github.com/lileililiwen/forge/releases/download/v0.1.0/<asset>`.
  Consumer workflows have `contents: read` and do not need an authenticated
  release read.
- CI release job uses the workflow run artifact, exact tag validation and
  `gh release create --verify-tag`; it does not use `--clobber`, delete, retag,
  mutate the repository or publish from pull-request contexts.
- No schema, binary protocol, manifest, database or API contract changes.
- Pack asset digests and receipts are computed over each version's exact
  template bytes. Older pack descriptors and historical generated receipts
  remain valid. Current defaults move only after the new versions pass their
  render and upgrade tests.

## 7. Failure, retry and recovery

| Failure | Behavior |
|---|---|
| Required CI job fails, including pinned-source contract parity | Release job does not start; no Release API write |
| Tag differs from Cargo version or is not a valid `v<semver>` | Tag validation fails before publication |
| Workflow artifact, archive or sidecar is missing | Publication fails with expected name; no fallback to a different build |
| Archive checksum is wrong | Existing verifier refuses before installation/publication |
| GitHub release already exists | `gh release create` fails; no replacement, deletion or retry that changes assets |
| Consumer download unavailable, timed out or returns 404 | Installer exits nonzero; does not invoke Forge Gate |
| Checksum sidecar is malformed, names another file or digest differs | Installer exits nonzero before extraction or writing to the destination |
| Archive is malformed or omits the expected binary | Installer exits nonzero; does not add a path to `GITHUB_PATH` |
| Downloaded Forge Gate fails | Job preserves the existing nonzero Gate verdict |
| No `v0.1.0` release exists yet | Newly generated pinned workflow fails visibly on download; operator must publish that exact first release before relying on generated CI |

The package does not claim atomic rollback for a remote GitHub Release. The
publish command runs once after all local verification jobs, and duplicate
release identity refuses replacement. Corrected bits require a new Cargo
version and tag.

## 8. Verification oracle

- Unit/contract tests prove `v<version>` matching and mismatch refusal.
- Workflow contract tests assert each CI job is a `needs` dependency of the
  release job, release is tag-only, publication permissions are job-scoped,
  contract parity checks out the mirror-pinned source, and no clobber/delete
  operation exists.
- Package test asserts the produced host archive is named for the exact
  expected target, has only the expected top-level directory/files, verifies
  its sidecar and smoke-tests installed `forge --version`, `--help` and Gate
  dry-run.
- Offline generated-installer test supplies release assets through a fake
  `curl`; it proves success installs the expected binary and checksum/name
  corruption, missing assets, malformed tar and missing Actions environment
  fail before Gate/path exposure.
- Standard-pack tests prove new versions are selectable, compatible with all
  six profiles, defaulted for new projects, rendered deterministically and
  upgraded without changing old pack bytes or bypassing ownership conflicts.
- Run targeted standard and release contracts, full serial workspace suite,
  workspace build, changed-Rust `rustfmt --check`, OpenSpec name preflight,
  strict validation, `git diff --check`, `forge gate --dry-run`, and full
  `forge gate --timeout-secs 600` before archive.
- Remote release publication is not run in local verification. Its oracle is
  the tagged GitHub Actions workflow and resulting release asset listing; this
  change does not push a tag or publish a release.

## 9. Decision ledger

- **Resolved:** the only target is Ubuntu x86_64 GNU because both current
  baseline workflows use `ubuntu-latest`; no cross-target artifacts are
  advertised. Add targets only with their own build, install, checksum and
  smoke oracle.
- **Resolved:** publish from Forge's existing CI workflow after all jobs pass,
  rather than add a second workflow with an independent gate or duplicate the
  test matrix.
- **Resolved:** publication is triggered by an operator-pushed matching
  version tag. CI does not create, push, retag or delete tags/releases.
- **Resolved:** use the existing package and checksum format; do not introduce
  a new Rust dependency, installer service, release schema or adapter.
- **Resolved:** the contract parity job checks out the immutable commit already
  recorded by `contracts/manifest.json`; no moving `main` reference or sibling
  checkout assumption remains in hosted CI.
- **Known rollout prerequisite:** repository currently has no published Forge
  release; actual generation consumers become usable after the operator
  publishes `v0.1.0`. The code and tests can be complete before that explicit
  external operation.
- **Deferred:** additional target triples, signatures/Sigstore attestations,
  Windows installer, package-manager distributions, local macOS installation,
  automated version/tag creation and Forge self-hosted release orchestration.
