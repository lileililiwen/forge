# Design: Forge artifact and CI baseline

## Ownership and boundaries

| Concern | Owner | This package's role |
| --- | --- | --- |
| Gate plan resolution, blocking policy, run history, exit semantics | Driftwatchdog (`quality-policy-integration`, `gate-runtime-evidence`) | CI supplies the environment where the declared runtime can actually execute; Forge still never re-decides a verdict |
| Release stages for *governed projects* (`commit`, `tag`, `push`, `mirror`, `package`, `container`, `docs`, `notes`) | Forge's own `src/release` plane | Out of scope. Forge's repository release uses git tags plus CI artifacts; whether Forge should dogfood `forge release` on itself is recorded as an open decision, not silently assumed |
| Portfolio adoption, `verification.command` validity, capability vocabulary | Workspace Governance | This package only makes the declared command truthful; declaration vocabulary is `governance-vocabulary-consumption` |
| Contract schemas and fixture matrix | `platform-contracts` | CI clones it at the revision pinned in `contracts/manifest.json` and runs the parity script this package does not own |
| Artifact promotion to any host | the operator | Explicitly not automated here |

The rule for every label and doc string added by this package: **a workflow may
name only the checks it runs.** The current `ci.yml:20` label says "native
matrix, gate" while `scripts/release-check.sh` contains no gate invocation; that
kind of label is the failure mode this package is fixing, so it cannot be
carried forward into the new job graph.

## MSRV and toolchain floor

`rust-version` must be derived, not asserted. Procedure recorded in the new ADR
0002:

1. Take the highest floor among direct dependencies (`clap 4` is the binding one
   at 1.74; `rusqlite 0.32` and `thiserror 2` follow).
2. Declare that value in `Cargo.toml` and add an `msrv` CI job that runs
   `cargo check --workspace --all-targets` on a toolchain exactly at the floor,
   mirroring the sibling's approach (`driftwatchdog` pins `1.74` with the
   rationale in a comment).
3. Raise the floor only together with a dependency-floor bump and a green
   `msrv` job. The ADR records the previous floor so a regression is visible.
4. No `rust-toolchain.toml` pin. Forcing a toolchain on contributors would
   silently change the local build the docs describe; the floor plus the CI job
   is the enforceable form. This matches the sibling, which also carries no
   `rust-toolchain` file.

The runner/image choice and the readiness-qualified versions must agree, or the
divergence is written in the same paragraph that records the qualification:
today CI installs `dotnet 8.0.x` while `docs/release-readiness.md:31` qualified
`aspnet-web` on `dotnet 10.0.400`. Either the job installs the qualified version
or the docs stop implying CI covered it.

## Package metadata and single version source

```toml
[package]
name = "forge"
version = "0.1.0"            # the one version source
edition = "2021"
rust-version = "1.74"         # derived, see ADR 0002
license = "MIT"
readme = "README.md"
repository = "https://…/forge"
homepage = "https://…/forge"
keywords = ["control-plane", "registry", "scaffold", "governance", "cli"]
categories = ["development-tools", "command-line-utilities"]

[workspace]                    # makes `cargo test --workspace` true
[workspace.dependencies]        # shared floors, inherited by [dependencies]
```

- `forge --version` already resolves through clap from `CARGO_PKG_VERSION`
  (`src/main.rs:137-141`), and `src/registry/mod.rs:23` uses
  `env!("CARGO_PKG_VERSION")`. Those stay; the new invariant is that
  `CHANGELOG.md`'s newest entry, the Cargo version and the tag
  `v<semver>` agree, enforced by a contract test rather than by discipline.
- The `license = "MIT"` claim becomes true by adding the matching `LICENSE` file.
  No relicensing decision is made in this package.
- `[dev-dependencies]` entries that merely restate `[dependencies]`
  (`tempfile`, `serde_json`, `rusqlite` at `Cargo.toml:28-31`) are removed in
  favour of inheritance; anything genuinely dev-only (schema validation from
  `platform-contract-consumption`, coverage tooling) stays explicit.

## Artifacts, digests and installation

```text
scripts/package.sh    # cargo build --release → dist/forge-<version>-<target>.tar.gz + .sha256
scripts/install.sh    # installs one archive to a prefix; refuses a checksum mismatch
scripts/checksum.sh   # regenerates/verifies the digest file
scripts/smoke.sh      # built binary: --version, --help, list, doctor, readiness check
scripts/bump.sh       # version + CHANGELOG entry + tag suggestion (no push)
```

- Target triple is discovered from `rustc -vV`, never hardcoded, so a
  contributor on another host packages what their toolchain actually produced.
- Archive contents: the binary, `LICENSE`, `README.md`, `CHANGELOG.md`. Paths are
  relative inside the archive; no absolute host path enters it, consistent with
  the redaction and host-path-scrubbing rules the runtime surfaces already
  follow.
- `install.sh` writes only under the given prefix and refuses a prefix that does
  not exist, a missing archive, or a digest mismatch. It never fetches anything.
- `dist/` is gitignored output, not committed state.

## CI job graph

| Job | Runs | Fails the build when |
| --- | --- | --- |
| `fmt` | `cargo fmt --all -- --check` | formatting drifts |
| `clippy` | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | any warning |
| `test` | `cargo test --workspace --all-targets`, with the long native-toolchain scaffold test in its own job so a 500–900s build never masks a fast-suite failure | any test fails |
| `native-scaffold` | the `rust_scaffold_builds_and_tests_with_native_toolchain` test plus `…_without_forge` | a generated project needs Forge to build |
| `msrv` | `cargo check --workspace --all-targets` at the declared floor | the floor is not honoured |
| `deny` | `cargo deny check`, `cargo audit` | an advisory, an unallow-listed licence, a duplicate or a banned crate |
| `governance` | the two Node scripts + `openspec validate --all --strict --no-interactive` + `git diff --check` | spec governance or whitespace drifts |
| `readiness` | `forge readiness matrix` then `forge readiness check` over the profiles the runner can actually qualify | a selected profile row fails or is unverified |
| `surfaces` | `forge doctor .`, `forge check .`, `forge fleet list`, `forge provider matrix` — the read-only planes CI currently never runs | a surface regresses or a document stops parsing |
| `contract-parity` | clone `platform-contracts` at `contracts/manifest.json`'s revision, run `scripts/contract-parity.sh` | the vendored set drifts from upstream |
| `gate` | install the declared runtime, initialize its store in the runner workspace, run `forge gate .` and then `forge gate status .` | the real gate blocks — mirroring the sibling's own exit semantics |
| `artifact` | `scripts/package.sh`, `scripts/checksum.sh --verify`, `scripts/smoke.sh`, upload archive + digest | a package cannot be built, verified or smoked |

Rules that keep the graph honest:

- Every job sets `permissions: contents: read` and an explicit
  `timeout-minutes`.
- A job that needs an external tool either installs it or the job does not
  exist. There is no step that skips because a binary is absent — absence must
  surface as a named failure, matching the runtime behaviour of `forge gate`
  and the provider matrix.
- The `gate` job's verdict is recorded as whatever the runtime returned,
  including `blocked`. This package does not tune `.ai-gate/gate.yaml` or the
  runtime's rule pack to obtain a green job; a block is information about the
  repository, and the recovery path is named in the same run.
- `coverage` is deferred to the measured-baseline decision (open decision
  4): no coverage job ships in this package. When one is added it
  publishes measured numbers as a job summary with no asserted threshold
  until the baseline justifies one, because a fabricated floor is worse
  than no floor.
- No workflow pushes, tags, releases or deploys. `auto-tag.yml` exists in the
  sibling; copying it here would violate the explicit AGENTS invariant against
  implicit publish/push/deploy, so it is deliberately absent and recorded as a
  deferred decision.

## `scripts/release-check.sh` stays the local contract

Local verification must remain reproducible without GitHub. The script gains the
deny step and an optional parity step guarded by the same
"missing tool blocks, never skips" rule already documented in its header
(`scripts/release-check.sh:17-19`), and CI's job set is documented as the same
checks split across runners. Where CI qualifies a subset of profiles, the script
keeps its `--gate-profile` mechanism and the divergence is stated in both files.

## Failure boundaries

| Condition | Behaviour |
| --- | --- |
| Gate runtime not installed in CI | `gate` job fails naming the resolution attempts; no skip-to-green |
| Runtime store uninitialized or corrupt | `gate-runtime-unavailable` recorded as the verdict, with the runtime's own hint; not converted into a pass |
| Gate returns `blocked` or `REVIEW_REQUIRED` | job fails or warns per the sibling's blocking semantics; evidence is captured, not laundered |
| Advisory in the dependency graph | `deny` fails with the crate and advisory id; suppression requires an explicit `deny.toml` exceptions entry with a reason |
| Bundled SQLite licence (`rusqlite` feature `bundled`) | allow-listed explicitly in `deny.toml` with the reason recorded, not covered by a blanket wildcard |
| Package digest mismatch | `artifact` job and `install.sh` both refuse before any install |
| Version/changelog/tag disagreement | contract test fails, naming the three values |
| Coverage toolchain unavailable | the coverage job is removed rather than added as a silent skip |

## Compatibility and migration

- No CLI behaviour, flag, exit code or emitted document changes. Existing
  contract and cross-surface suites keep passing byte-identically.
- `.project.json` keeps `schema_version: 1`; only `verification.command` value
  changes, and it changes to a command that exists.
- Adding `[workspace]` changes no target layout: the package remains both the
  `[lib]` and the `[[bin]]`, and `./target/debug/forge` keeps its path — which
  several existing tests and docs reference.
- CI's single job disappears; every check it performed still runs, in a named
  job, so a green-to-red comparison stays interpretable.

## Open decisions (recorded, not silently fixed)

1. **Publishing.** Where artifacts eventually go (crates.io, an internal host,
   or nothing) is a separate decision requiring its own change and its own
   evidence; today the answer is deliberately "nowhere".
2. **Self-dogfooding `forge release`.** Forge could govern its own repository
   through its release plane. That is attractive but it would make a tool
   validate itself through a path the tool owns, so it needs a review decision
   and independent CI evidence before it is attempted.
3. **Tag automation.** `auto-tag.yml` is withheld on purpose (AGENTS invariant).
   If wanted later, it must arrive with an explicit operator authorization
   rather than as a copied file.
4. **Coverage floor.** No threshold is asserted in this package. Choosing one
   requires a measured baseline, which the first coverage run supplies.
5. **Windows/macOS runners.** Only Linux is proposed for now; adding a matrix
   without ever having produced or smoked an artifact for those hosts would be an
   unverified claim, so it follows real evidence instead of preceding it.

## Verification

- `sh scripts/release-check.sh --gate-profile rust-web --gate-profile nextjs-web
  --gate-profile aspnet-web` must stay the reproducible local path, and the new
  steps must fail loudly when `cargo-deny` is absent.
- Package and install are verified against a scratch prefix in CI and locally;
  `scripts/smoke.sh` output is recorded verbatim in the change's evidence.
- The `gate` job's first real verdict is recorded honestly whatever it is. No
  gate pass is claimed for this repository from local evidence, because this
  checkout has no `.driftwatch` store — the sibling-owned next action from the
  previous cycle stands until an operator initializes it.
- Every new script is shell-checked; `ruff`/`py_compile` stay as the adapter lint
  precedent already recorded.
