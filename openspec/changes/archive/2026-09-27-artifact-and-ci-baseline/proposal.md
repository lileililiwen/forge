# Proposal: Give Forge the artifact and CI baseline its own gate runtime already has

## Why

Forge's CLI is a real binary that 1,180 tests exercise, but nothing about it is
shippable or verifiable in CI beyond one job:

- `README.md:39` states it plainly: "There is no Forge installation packaging
  yet." There is no `Dockerfile`, no `Makefile`/`justfile`, no `package.json`, no
  `.deb`/`.rpm`, no install script, no release artifact and no published digest.
- `Cargo.toml:6` declares `license = "MIT"` while **no `LICENSE` file exists** in
  the repository — the metadata asserts something the tree does not contain.
- There is no `CHANGELOG.md`, yet `src/release/mod.rs:80` sets
  `DEFAULT_CHANGELOG = "CHANGELOG.md"` as the changelog every *governed* project
  is expected to carry. Forge does not meet the requirement it enforces on
  others.
- `Cargo.toml` has no `[workspace]` section, so the repository's own
  `.project.json` declaration —
  `"verification": { "command": "cargo test --workspace" }` — names a command
  against a structure that does not exist. Workspace Governance's
  `VERIFICATION_COMMAND` check exists precisely to catch a declaration that does
  not describe a real command.
- `Cargo.toml:3` pins no `rust-version`; the toolchain floor lives only as prose
  ("Rust stable, validated 1.98.1") in `docs/adr/0001-foundation-toolchain.md:22`
  and `docs/release-readiness.md:31`. Nothing enforces it.
- `[dev-dependencies]` (`Cargo.toml:28-31`) re-declares `tempfile`, `serde_json`
  and `rusqlite` with their own floors, duplicating `[dependencies]` instead of
  inheriting it, and there is no `[workspace.dependencies]`.
- There is no `deny.toml`, no `cargo-audit`, no `cargo-deny` and no coverage
  configuration. `.gitignore` mentions `/coverage/` and `lcov.info`
  (`.gitignore:16,39`) for tooling the repository does not run.

`.github/workflows/ci.yml` is 25 lines with a **single job** (`readiness`) that
runs `scripts/release-check.sh`, which executes
`cargo fmt --check`, `cargo build`, `cargo test`,
`cargo clippy --all-targets -- -D warnings`, the two Node governance scripts,
`openspec validate --all --strict --no-interactive`,
`forge readiness matrix` and `forge readiness check`
(`scripts/release-check.sh:53-83`). That is a genuine quality gate, but it is
also the whole surface: **nothing in CI invokes `forge gate`, `forge doctor`,
`forge check`, `forge fleet`, the governance provider, the MCP transport, the API
server or the portal**, and nothing installs the declared gate runtime. The step
is labelled "…native matrix, gate" (`ci.yml:20`) while no Gate Runtime runs — a
label that claims more than the workflow does. CI also provisions
`dotnet-version: "8.0.x"` (`ci.yml:19`) while `docs/release-readiness.md:31`
qualified the `aspnet-web` row on `dotnet 10.0.400`, so the runner proves a
different configuration from the one the evidence records.

The comparison is not abstract. `driftwatchdog` — the runtime Forge drives, and
the sibling whose gate semantics this repository deliberately does not re-decide
— carries `rust-version = "1.74"` with the floor's rationale in a comment,
`deny.toml`, `LICENSE`, `CHANGELOG.md`, full `[package]` metadata,
`scripts/{install,package,checksum,smoke,bump}.sh` plus a `scripts/lib/`, an
`npm/` package, and three workflows (`ci.yml`, `release.yml`, `auto-tag.yml`)
with a job graph that includes an MSRV job and a cargo-deny job.

Requirement.md §34 and §37 describe a CLI that operators install, and the AGENTS
invariant "record actual build/test commands when application tooling is
introduced" obliges this repository to record and enforce them. Forge is
currently consumed by checking out source and running `cargo build`, which
caps every maturity claim a sibling can make about it.

## What Changes

- Make the declared verification command true: add a `[workspace]` section to
  `Cargo.toml` so `cargo test --workspace` (as already recorded in
  `.project.json`) describes the real package graph, and keep
  `scripts/release-check.sh` as the full local gate whose name CI and docs then
  reference consistently.
- Add the missing repository assets: a `LICENSE` file matching the declared MIT
  licence, a `CHANGELOG.md` that `src/release`'s own default path can find,
  `[package]` metadata (`repository`, `homepage`, `readme`, `keywords`,
  `categories`), and a `rust-version` floor derived from the actual dependency
  requirement rather than asserted.
- Add dependency policy: `[workspace.dependencies]` with inherited floors,
  de-duplicated dev-dependencies, a `deny.toml` covering advisories, licence
  allow-list and bans (with the bundled-SQLite exception recorded explicitly),
  and `cargo audit` + `cargo deny check` as enforced steps.
- Add packaging and installation: `scripts/package.sh` producing a release
  build plus a per-target archive with a sha256 digest file,
  `scripts/install.sh` installing a chosen prefix from that archive,
  `scripts/checksum.sh`, `scripts/smoke.sh` running the built binary's
  `--version`, `forge --help`, `forge list`, `forge doctor` and
  `forge readiness check` against a scratch project, and `scripts/bump.sh`
  keeping `Cargo.toml` version and `CHANGELOG.md` in step with a `v<semver>` tag.
- Restructure CI into the job graph the current single job implies but does not
  provide: `fmt`, `clippy`, `test`, `msrv`, `deny`, `coverage`, `governance`
  (Node checks plus strict OpenSpec validation), `readiness matrix` +
  `readiness check`, `contract-parity` (clone `platform-contracts` at the pinned
  revision and run `scripts/contract-parity.sh`), `gate` (install the declared
  runtime, initialize its store in the runner, run `forge gate .` and mirror its
  real verdict), and `artifact` (build and upload the packaged archive plus
  digests). Every job declares explicit `permissions: contents: read`.
- Correct the runner/toolchain mismatch: the profile rows CI selects and the
  toolchain versions it installs become the rows and versions the readiness
  evidence records, or the divergence is stated in the same file that records
  the qualification.
- Fix the step labels so a workflow never names a check it did not run, and give
  `.project.json` `verification.command` and `docs/release-readiness.md` the
  same authoritative entry point.
- **No publication.** Packaging produces artifacts and digests; pushing them to
  crates.io, npm, a container registry or a Jenkins job is explicitly out of
  scope and stays recorded as `not-run`.

## BFS Impact Map

- **Capabilities:** new `artifact-and-ci-baseline`; consumes
  `profile-and-release-readiness` (native matrix rows and the qualified-toolchain
  claim), `gate-runtime-evidence` (CI becomes the environment where a real gate
  run can honestly execute), `provider-integration-evidence` (CI rows for
  provider probes stay opt-in and honestly `not-run` without credentials),
  `platform-contract-consumption` (the parity job this package wires),
  `core-manifest-registry` and the foundation ADR (toolchain floor and the
  recorded build/test commands).
- **Users and flows:** an operator installs a versioned binary with a published
  digest instead of cloning and building; a contributor's pull request gets a
  named failing job instead of one 60-minute job that stops at the first block;
  Workspace Governance can read a truthful `verification.command` for this
  project.
- **Contracts/data/persistence:** no runtime contract changes. `Cargo.toml`
  gains metadata and a workspace section; `.project.json` keeps
  `schema_version: 1` with a corrected command value; archives and digests are
  CI artifacts, never registry state.
- **Integrations/configuration:** CI installs the declared gate runtime and the
  sibling contract source as *job inputs*, so integration surfaces gain a real
  execution environment without making either a runtime dependency of the
  binary; the packaging path is stdlib shell only and calls no external service.
- **Callers:** CI, `scripts/release-check.sh` (which gains the deny and parity
  steps so local and CI stay one contract), `docs/` evidence files, README
  quickstart (install instead of "no packaging yet"), and the foundation ADR's
  recorded commands.
- **Failure/boundary behavior:** a missing `driftwatch`/`driftwatchdog` in the
  gate job fails that job naming the resolution attempts — it does not convert
  to a skipped pass; an uninitialized or corrupt runtime store is
  `gate-runtime-unavailable`, recorded as such; a denied advisory or an
  unallow-listed licence fails `deny`; a toolchain below the declared floor
  fails `msrv`; a digest mismatch between an artifact and its checksum file
  fails `artifact`; `coverage` reports measured coverage and fails only on a
  declared regression floor, never on an invented target.
- **Tests:** shell-level tests for the packaging scripts against a scratch
  prefix (install, smoke, checksum verification, refusal when the archive is
  absent), and contract tests asserting `forge --version` equals the
  `Cargo.toml` version and the changelog's newest entry, plus CI YAML structural
  checks (every job has explicit permissions and a timeout; no job label names a
  command the job does not run).
- **Dependencies:** the package metadata, licence file and changelog are
  prerequisites for anything that later publishes; `contract-parity` depends on
  `platform-contract-consumption` shipping the vendored set and script.
- **Compatibility/security/privacy:** additive to the CLI's behaviour — no
  command, flag, exit code or document changes; digests and artifacts contain no
  host paths; secrets are never embedded in artifacts or CI logs, and captured
  CI evidence continues through `policy::redact_credentials`; `auto-tag` or
  equivalent automation is **not** introduced, because AGENTS.md forbids implicit
  publish, push or deploy.

## Capabilities

- `artifact-and-ci-baseline`: Forge publishes an installable, versioned,
  checksummed artifact built by CI whose jobs actually execute Forge's declared
  verification surfaces — including the shared gate runtime — and whose recorded
  commands, licences, metadata and toolchain floor are consistent with what the
  repository enforces on the projects it governs.

## Non-goals

- No publication to crates.io, npm, a container registry or any package host, and
  no tag automation. Artifacts and digests are produced and verified; promotion
  is an operator decision (requirement.md §44, AGENTS.md "do not force higher
  maturity, publish, push or deploy implicitly").
- No CI/CD product feature inside Forge. This is this repository's own
  verification, not a workflow engine for governed projects — `forge gate`,
  `forge release` and Driftwatchdog already own those semantics (requirement.md
  §24, §29, §32).
- No container image, cross-distribution packaging (`.deb`/`.rpm`), code signing,
  notarization or SBOM generation in this package. Each is a separate decision
  with its own evidence requirement.
- No claim of a gate pass for this repository from local evidence; the gate job's
  verdict is whatever the real runtime returns in CI.
- No new maturity-level assertion. Maturity stays evidence-derived and optional
  (requirement.md §25); this package supplies evidence, it does not promote Forge.
- No change to `capabilities` / `quality` / `release_evidence` declaration blocks
  or the portfolio vocabulary (`governance-vocabulary-consumption`).

Source: requirement.md §34, §37, §39, §44, §45.
