# Release readiness

Contract `0.1.0` (`forge readiness matrix|artifact|check`). This checklist
is the human entry point; the machine gate is `scripts/release-check.sh`
(local) and `.github/workflows/ci.yml` (CI). Both run the same contract —
local as one script, CI split across named jobs (`fmt`, `clippy`, `test`,
`native-scaffold`, `msrv`, `deny`, `governance`, `readiness`, `surfaces`,
`gate`, `artifact`) — and both block (non-zero exit with the exact failed or
unavailable check named) instead of representing a gap as release-ready.
The `test` job and local release check verify the Forge-owned contract asset
manifest and Draft 2020-12 fixtures offline. No sibling source checkout is
required.

## Gate steps

1. `cargo fmt --check`, `cargo build`, `cargo test --workspace
   --all-targets`, `cargo clippy --workspace --all-targets --all-features
   -- -D warnings` (the `native-scaffold` evidence runs the three
   long-running native-toolchain tests in its own job so a slow build
   never masks a fast-suite failure).
2. `node scripts/check-contracts.mjs` and
   `python3 tests/contract_schema_tests.py` (Forge-owned asset digests,
   supported/unsupported families, and schema fixtures); install the pinned
   test validator with `python3 -m pip install -r requirements-dev.txt`.
3. `cargo deny check` and `cargo audit` (dependency/advisory policy,
   `deny.toml`); `openspec validate --all --strict --no-interactive`
   (required; a missing CLI blocks the script, it never skips validation).
4. `forge readiness matrix` over every supported profile: disposable
   fixture per profile, native build/test with Forge absent from `PATH`,
   toolchain version, the version the profile declares, source SHA-256 and
   timestamp captured per row, each row classified `passed` / `failed` /
   `unverified`.
5. `forge readiness check [--profile ...]` over the runner-qualified
   profiles plus the artifact smoke. Exit 0 with `gate ready=true` only
   when every selected row passes.

## Runner / profile contract

The matrix always reports every supported profile. The gate passes only
for profiles the runner qualifies:

| Runner | Qualified profiles | Basis |
| --- | --- | --- |
| CI (`ubuntu-latest` + Rust/Node/.NET toolchains) | `rust-web`, `nextjs-web`, `aspnet-web` | toolchains provisioned by the workflow (`dotnet 10.0.x`, matching this qualification; Node 24) |
| Local host (this cycle) | `rust-web`, `nextjs-web`, `react-web` and `aspnet-web` passed; `python-service` and `flutter-app` are `unverified` (see below) | `cargo` 1.99.1, `node` 24.18.0 / `npm` 11.16.0, `python3` 3.14.4, `dotnet` 10.0.400, `flutter` 3.47.1 |

A profile whose toolchain is absent is `unverified`, never passing. A
profile whose runtime version does not satisfy the version the profile
declares is `unverified` too, never passing: the toolchain is present, its
version is not the one the profile declares. A profile whose native command
fails is `failed` with the command named. Unselected profiles are out of the
gate but stay visible in the matrix.

## Artifact procedure

The release artifact is the platform-native Forge binary, `LICENSE`,
`README.md`, `CHANGELOG.md`, and the maintained `share/forge/contracts/` data
tree, packaged by `scripts/package.sh` into
`dist/forge-<version>-<target>.tar.gz` with a `.sha256` digest file
(`dist/` is gitignored output, never committed state). Verify and
install it locally with:

```sh
scripts/package.sh
scripts/checksum.sh --verify dist/forge-<version>-<target>.tar.gz
scripts/install.sh --archive dist/forge-<version>-<target>.tar.gz --prefix <dir>
scripts/smoke.sh --bin <dir>/bin/forge
```

`forge readiness artifact` still reports the platform-native binary
evidence (path, SHA-256, version smoke) for the locally built tree;
installation is not a second code path — the package smoke runs contract list
and inspect from a scratch working directory using installed assets. Only a
push of `v< Cargo.toml version >` triggers
publication. The release job waits for every CI verification job, rechecks the
workflow artifact checksum, and uses the built-in `gh` CLI to publish the exact
archive and sidecar as a GitHub Release. The job has `contents: write`; all
other jobs and generated consumer workflows keep `contents: read`. It refuses
an existing release and never creates or pushes the tag. The current asset is
`forge-0.1.0-x86_64-unknown-linux-gnu.tar.gz`; standard packs pin that exact
version and target. Publishing this initial tag/release remains an explicit
operator action.

## Known gaps (not release evidence)

Recorded by the matrix on 2026-10-22: every supported profile's declared
native build and test commands pass against the tree that profile
generates, so the full-matrix gate is available on a runner that supplies
the six toolchains. Runner prerequisite:

- A declared toolchain version is checked, not decoration. The declared
  version is compared with the runtime's own version, component-wise on the
  declared version's component count, and the runtime is the ecosystem's
  binary: the Node ecosystem's runtime is `node`, so `react-web` (toolchain
  `npm`) and `nextjs-web` (toolchain `node`) declare and check the same node
  version. `nextjs-web` and `react-web` declare the Node version the CI
  workflow provisions (`node-version: "24"`).
- On this host `node` is 24.18.0, so both Node rows satisfy the declared
  24 and pass. `python3` 3.14.4 does not satisfy `python-service`'s 3.12
  and `flutter` 3.47.1 does not satisfy `flutter-app`'s 3.22 — both are
  newer than the declaration — so those two rows are `unverified` and the
  full-matrix gate is not ready, while the qualified subset gate
  (`rust-web`, `nextjs-web`, `aspnet-web`) passes. The CI runner
  provisions Node 24, so its qualified gate passes too. A host whose
  `node` reports an older major than the declared 24 makes the Node rows
  `unverified` as well, and the qualified gate is blocked on
  `nextjs-web` until `node` reports 24.
- `flutter-app` (`flutter build bundle` / `flutter test`) requires the Flutter
  SDK; without it the row is `unverified`. The app store package
  (`flutter build appbundle`) remains a release-stage command requiring
  `android/` and the Android SDK.
- `python-service` (`python3 -m compileall -q app tests` /
  `python3 -m unittest discover -s tests -v`) needs only the `python3`
  toolchain; neither command needs a third-party runner.
- Planned specialist profiles (`rust-cli`, `aspnet-saas`, …) stay
  `unsupported-profile`: discoverable via `forge profile inspect`, never
  matrix rows and never gate members.

When a toolchain the host lacks is absent the row reports `unverified`
and the qualified subset gate remains the release-eligible verdict for
that runner; the full-matrix claim requires the prerequisite to be
present. The same holds for a runtime version the host does not satisfy:
the row reports `unverified` and names the profile, the declared version
and the observed version.
