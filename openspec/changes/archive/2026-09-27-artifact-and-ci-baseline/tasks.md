# Tasks: Give Forge the artifact and CI baseline its own gate runtime already has

## 1. BFS — Baseline and impact coverage

- [x] Record the exact current state before changing anything: `Cargo.toml`
  contents (no `[workspace]`, no `rust-version`, no package metadata,
  `[dev-dependencies]` restating three `[dependencies]` entries), the absence of
  `LICENSE` and `CHANGELOG.md`, `.github/workflows/ci.yml` as a single job, and
  the step-by-step list of checks `scripts/release-check.sh` performs today.
- [x] Establish the MSRV floor from the real dependency graph rather than
  copying the sibling's number: inspect the floors of `clap 4`, `rusqlite 0.32`,
  `thiserror 2`, `serde`, `chrono`, `sha2`, `rand`, `tempfile`, `serde_yaml`, and
  record which one binds. Confirm the chosen floor builds
  `cargo check --workspace --all-targets`.
- [x] Inventory which Forge surfaces have no CI execution today (`forge gate`,
  `gate status`, `doctor`, `check`, `fleet`, `governance`, `provider matrix`,
  `mcp serve`, `api serve`, `portal dashboard|view`) and which are covered only
  by tests.
- [x] Reconcile the runner toolchains against `docs/release-readiness.md`:
  `dotnet 8.0.x` in CI versus the `dotnet 10.0.400` qualification, Node 24, and
  the per-profile native commands, and decide per profile whether CI qualifies
  the row or the docs stop implying it does.
- [x] Confirm the three existing long-running native tests and how they are
  currently excluded or included, so the job split does not silently drop the
  only real native-build evidence.
- [x] Write the job-graph table into design.md and have a reviewer confirm that
  every job's name matches the commands it runs, before any YAML is written.

## 2. DFS — Requirement-by-requirement implementation

- [x] Add the `LICENSE` file matching `license = "MIT"`
  (`Cargo.toml:6`), and `CHANGELOG.md` with an initial honest entry so
  `src/release/mod.rs:80 DEFAULT_CHANGELOG` resolves against a real path.
- [x] Add `[workspace]` so the recorded
  `.project.json:verification.command` (`cargo test --workspace`) describes the
  actual package graph; keep `[lib]`/`[[bin]]` names and the
  `./target/debug/forge` output path unchanged.
- [x] Add `[workspace.dependencies]`, inherit the floors in `[dependencies]`,
  delete the duplicated `[dev-dependencies]` entries, and keep only genuinely
  dev-only requirements explicit.
- [x] Declare `rust-version` and record the derivation, the binding dependency,
  the previous floor (none) and the raise procedure in a new
  `docs/adr/0002-msrv-and-toolchain-floor.md`; update ADR 0001's recorded
  commands and README quickstart to point at the same entry points.
- [x] Add full `[package]` metadata (`readme`, `repository`, `homepage`,
  `keywords`, `categories`) and a contract test asserting `forge --version`,
  the Cargo version and the newest `CHANGELOG.md` entry agree.
- [x] Implement `scripts/package.sh` (release build, target triple discovered
  from `rustc -vV`, archive containing binary + `LICENSE` + `README.md` +
  `CHANGELOG.md` with relative paths only), `scripts/checksum.sh`, and
  `scripts/install.sh` that refuses a missing archive, an unknown prefix or a
  digest mismatch without ever fetching.
- [x] Implement `scripts/smoke.sh` against the packaged binary: `--version`,
  `--help`, `forge list`, `forge doctor .`, `forge readiness check` on a scratch
  project, and `forge check .` document parseability.
- [x] Implement `scripts/bump.sh` (version + changelog + tag suggestion, no push,
  no remote mutation) and document that tag creation remains an operator action.
- [x] Add `deny.toml` covering advisories, a licence allow-list, bans and
  duplicates, with the `rusqlite` bundled-SQLite case recorded as an explicit
  allow-list entry with its reason; wire `cargo deny check` and `cargo audit`.
- [x] Rewrite `.github/workflows/ci.yml` into the agreed job graph
  (`fmt`, `clippy`, `test`, `native-scaffold`, `msrv`, `deny`, `governance`,
  `readiness`, `surfaces`, `contract-parity`, `gate`, `artifact`), each job with
  `permissions: contents: read`, an explicit timeout, and a cargo cache.
- [x] Implement the `gate` job for real: install the declared runtime, initialize
  its store inside the runner workspace, run `forge gate .` then
  `forge gate status .`, and mirror the runtime verdict without adjusting
  `.ai-gate/gate.yaml` or the rule pack to obtain green.
- [x] Implement the `surfaces` job so the read-only planes CI never invoked gain
  execution, and the `contract-parity` job cloning `platform-contracts` at the
  revision pinned in `contracts/manifest.json`.
- [x] Extend `scripts/release-check.sh` with the deny step and an optional
  parity step under the existing "missing tool blocks, never skips" rule, so the
  local script and the CI job set remain one contract.
- [x] Remove the misleading "gate" wording from any step label that does not
  invoke the gate runtime, in CI and in the docs.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-run the full existing suite unchanged and prove byte-identity of the
  documents the surfaces emit (`forge gate status --format json`, `forge check`,
  `forge fleet list`, `forge provider matrix`) before and after, so a CI
  reshuffle cannot smuggle a behaviour change.
- [x] Verify the packaged binary behaves identically to `./target/debug/forge`
  on the contract suites, so installation is not a second code path.
- [x] Confirm every added job fails loudly rather than skipping when its tool is
  absent, and that no job is added for a check whose result would be
  unverified-by-default.
- [x] Re-check that no workflow pushes, tags, publishes or deploys, and that
  `auto-tag.yml` is deliberately absent with the reason recorded.
- [x] Update `README.md` (replace "There is no Forge installation packaging yet",
  document install and digest verification), `docs/release-readiness.md`
  (qualified toolchains and the honest matrix rows), `docs/provider-evidence.md`
  (rows that gain a real CI environment and rows that stay `not-run`), and
  `docs/architecture.md` if the verification boundary description changes.
- [x] Reconcile `HANDOFF.md` evidence wording for future cycles: the CI job set
  is now the second verification layer, never the first check
  (`.ai-rules/completion.md`).

## 4. Verification

- [x] `shellcheck` every added or modified shell script; `cargo fmt --all --
  --check`; `cargo build`; `cargo clippy --workspace --all-targets --all-features
  -- -D warnings`; the full `cargo test --workspace --all-targets` plus the
  separately recorded native-toolchain test with its actual duration.
- [x] `cargo deny check` and `cargo audit` green, or each denial explained in
  `deny.toml` with a reason.
- [x] Local end-to-end: `scripts/package.sh` → `scripts/checksum.sh --verify` →
  `scripts/install.sh --prefix <scratch>` → `scripts/smoke.sh`; record the
  verbatim output as evidence.
- [x] `forge gate .` in an environment where the runtime is installed and the
  store initialized (the CI job satisfies this); record the true verdict. For
  this checkout the sibling-owned prerequisite (`driftwatch init`) remains open,
  so no local gate pass is claimed and the row stays
  `gate-runtime-unavailable`.
- [x] `node scripts/check-openspec-change-names.mjs`; `openspec validate --all
  --strict --no-interactive`; `git diff --check` and review of newly added
  files; archive without `--skip-specs` once every scoped scenario has evidence.
- [ ] Record in `HANDOFF.md` the CI job graph, the MSRV floor with its binding
  dependency, the packaging commands, the digests produced, and the explicitly
  deferred publication/tag decisions.
