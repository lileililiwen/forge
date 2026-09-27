# Release readiness

Contract `0.1.0` (`forge readiness matrix|artifact|check`). This checklist
is the human entry point; the machine gate is `scripts/release-check.sh`
(local) and `.github/workflows/ci.yml` (CI). Both run the same contract —
local as one script, CI split across named jobs (`fmt`, `clippy`, `test`,
`native-scaffold`, `msrv`, `deny`, `governance`, `readiness`, `surfaces`,
`contract-parity`, `gate`, `artifact`) — and both block (non-zero exit
with the exact failed or unavailable check named) instead of representing
a gap as release-ready. The one documented divergence: the parity walk
runs locally only when the `platform-contracts` source resolves
(`PLATFORM_CONTRACTS_DIR` or the `../platform-contracts` sibling
checkout); CI's `contract-parity` job blocks without it.

## Gate steps

1. `cargo fmt --check`, `cargo build`, `cargo test --workspace
   --all-targets`, `cargo clippy --workspace --all-targets --all-features
   -- -D warnings` (the `native-scaffold` evidence runs the three
   long-running native-toolchain tests in its own job so a slow build
   never masks a fast-suite failure).
2. `cargo deny check` and `cargo audit` (dependency/advisory policy,
   `deny.toml`); `openspec validate --all --strict --no-interactive`
   (required; a missing CLI blocks the script, it never skips validation).
3. `forge readiness matrix` over every supported profile: disposable
   fixture per profile, native build/test with Forge absent from `PATH`,
   toolchain version, source SHA-256 and timestamp captured per row, each
   row classified `passed` / `failed` / `unverified`.
4. `forge readiness check [--profile ...]` over the runner-qualified
   profiles plus the artifact smoke. Exit 0 with `gate ready=true` only
   when every selected row passes.

## Runner / profile contract

The matrix always reports every supported profile. The gate passes only
for profiles the runner qualifies:

| Runner | Qualified profiles | Basis |
| --- | --- | --- |
| CI (`ubuntu-latest` + Rust/Node/.NET toolchains) | `rust-web`, `nextjs-web`, `aspnet-web` | toolchains provisioned by the workflow (`dotnet 10.0.x`, matching this qualification; Node 24) |
| Local host (this cycle) | `rust-web`, `nextjs-web`, `aspnet-web` passed; see gaps below | `cargo` 1.98.1, `node` 24 / `npm` 11, `dotnet` 10.0.400 |

A profile whose toolchain is absent is `unverified`, never passing. A
profile whose native command fails is `failed` with the command named.
Unselected profiles are out of the gate but stay visible in the matrix.

## Artifact procedure

The release artifact is the platform-native Forge binary plus `LICENSE`,
`README.md` and `CHANGELOG.md`, packaged by `scripts/package.sh` into
`dist/forge-<version>-<target>.tar.gz` with a `.sha256` digest file
(`dist/` is gitignored output, never committed state). Verify and
install it with:

```sh
scripts/package.sh
scripts/checksum.sh --verify dist/forge-<version>-<target>.tar.gz
scripts/install.sh --archive dist/forge-<version>-<target>.tar.gz --prefix <dir>
scripts/smoke.sh --bin <dir>/bin/forge
```

`forge readiness artifact` still reports the platform-native binary
evidence (path, SHA-256, version smoke) for the locally built tree;
installation is not a second code path — the packaged binary runs the
same contract suites. Packaging formats beyond the tarball and any
publication destination stay bounded by the existing
release/distribution contracts; this package claims no remote
publication, and no workflow pushes, tags, releases or deploys.

## Known gaps (not release evidence)

Recorded by the matrix on 2026-09-26: every supported profile's declared
native build and test commands pass against the tree that profile
generates, so the full-matrix gate is available on a runner that supplies
the six toolchains. Runner prerequisite:

- `flutter-app` (`flutter analyze` / `flutter test`) requires the Flutter
  SDK; without it the row is `unverified`. The app bundle
  (`flutter build appbundle`) remains a release-stage command requiring
  `android/` and the Android SDK.
- `python-service` `python3 -m build` requires the `build` module;
  without it the row is `unverified` with the prerequisite named.
  `python3 -m unittest discover -s tests -v` needs no third-party runner.
- Planned specialist profiles (`rust-cli`, `aspnet-saas`, …) stay
  `unsupported-profile`: discoverable via `forge profile inspect`, never
  matrix rows and never gate members.

When a toolchain the host lacks is absent the row reports `unverified`
and the qualified subset gate remains the release-eligible verdict for
that runner; the full-matrix claim requires the prerequisite to be
present.
