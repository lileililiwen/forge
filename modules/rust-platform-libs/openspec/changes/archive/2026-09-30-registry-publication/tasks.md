# Tasks: crates.io publication

## 0. Caveats and confirmations

- [x] **Release execution is owner-gated.** No tag push, no real
  publication, until the approval marker + `CARGO_REGISTRY_TOKEN` exist
  (design P6). Tasks 3.x stay unchecked with that reason recorded.
- [x] **Do not edit consumers.** Migration to registry deps is each
  consumer's own change.
- [x] **`platform-arch-tests` stays unpublishable.**
- [x] **Re-verify name availability at publish time** (2026-09-30 evidence
  says all seven names free).
- [x] **`--allow-dirty` banned**; `--no-verify` banned.

## 1. BFS — Baseline and impact coverage

- [x] Read `[workspace.package]`, every crate manifest, the DAG used by
  `platform-arch-tests`, and existing `ci.yml` trigger patterns.
  - `[workspace.package]` has `version = "0.1.0"`, `license = "Apache-2.0"`,
    `rust-version = "1.75"`, `edition = "2021"`, `publish = false`; no
    `repository`, `homepage`, `documentation`, `keywords`, `categories`.
  - Eight crate manifests all use `publish.workspace = true` (inherits the
    workspace's `publish = false` today), all `version.workspace = true`,
    all `license.workspace = true`, all `rust-version.workspace = true`,
    all `edition.workspace = true`. Inter-crate deps are `path =` only
    (no `version`); the new `cargo publish --dry-run` will require the
    `version = "0.1.0"` form so `cargo publish` can rewrite the path
    dep to a registry requirement.
  - DAG (read from arch-tests `SATELLITE_ALLOWLIST` and the manifest
    table; foundation rule + satellite allowlist unchanged):
    1. `platform-core` (no platform deps)
    2. `platform-contracts` (→ `platform-core`)
    3. `platform-identity` (→ `platform-core`, `platform-contracts`)
    4. `platform-audit` (→ `platform-core`, `platform-contracts`, `platform-identity`)
    5. `platform-testing` (→ `platform-core`, `platform-contracts`, `platform-identity`, `platform-audit`)
    6. `platform-mailing` (→ `platform-core`; satellite; allows `lettre`, `tokio`)
    7. `platform-ai` (→ `platform-core`; satellite; allows `reqwest`, `tokio`, `futures`)
    8. `platform-arch-tests` (no platform deps; `publish = false` override)
    `tests/consumer-fixture` is a workspace member but a local consumer
    of path deps; not packaged, not published.
  - `ci.yml` triggers on `push: [main, master]` and
    `pull_request: [main, master]`; the new `publish.yml` triggers on
    `push: tags: ['v*']` only (design §4 "Publish trigger: tag push
    `v*` only; branch pushes never publish"). The two files do not
    overlap.
- [x] Map PUB-001..004 scenarios to the workflow matrix and dry-run checks
  in design §7; confirm proposal/design/spec agree.
  - PUB-001 → `cargo publish --dry-run` per crate + `cargo package --list`
    on each.
  - PUB-002 → `publish.yml` tag trigger, marker/secret/tag-version
    prechecks, topological order list, 404 pre-flight, publish, propagation
    poll, skip-existing.
  - PUB-003 → `docs/publication.md` content + README publication line +
    ROADMAP entry.
  - PUB-004 → `publish.yml` precheck (no marker → fail with precondition
    message, zero registry calls).
- [x] Record the precondition state: `git remote -v` is empty; no
  `repository` key anywhere; the workspace-governance GitHub flow
  (`scripts/github_metadata.py` + push) is the next action for the
  owner. The change ships the pipeline so that, after the owner
  approval, only the two preconditions block the first real `v0.1.0`
  release: the GitHub-publication precondition (which will set the
  `repository` URL) and the `PUBLICATION-APPROVED` marker file (which
  the owner adds when they explicitly approve).

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 PUB-001: workspace metadata keys (`repository` left as a
  documented placeholder until the URL exists — recorded in
  `docs/publication.md` §"Pre-publish checklist" and in HANDOFF as a
  tracked precondition, with the workspace-governance
  `github_metadata.py` flow as the replacement path), per-crate
  `keywords`/`categories`/`exclude`; flip `publish = true`; explicit
  `publish = false` for `platform-arch-tests`.
- [x] 2.2 PUB-001: inter-crate deps to `{ path = …, version = … }` form per
  the DAG; `cargo publish --dry-run` all seven green; `cargo package --list`
  excludes openspec/.github; `platform-arch-tests` keeps `publish = false`
  so a dry-run there is refused by Cargo before any archive is built.
- [x] 2.3 PUB-002/004: `.github/workflows/publish.yml` — tag trigger;
  marker + secret + tag-vs-workspace version prechecks; topological order
  list (`.publish-order`) with a unit test in `platform-arch-tests`
  comparing it to the manifest DAG; per-crate 404 pre-flight, publish,
  propagation poll, skip-existing idempotence; failure table from
  design §6 reflected in step names.
- [x] 2.4 PUB-003: `docs/publication.md` (lockstep rule, pre-1.0 policy,
  yank procedure, token scope/rotation, changelog section); README
  publication line; ROADMAP entry marked.

## 3. BFS — Cross-surface regression and completeness

- [x] `ci.yml` unaffected and green; `cargo test --workspace` green with
  the version-inheriting dep table; arch-tests still pass (dev-only ban
  unchanged).
- [x] Workflow negative matrix runs locally (bash dry-run of the
  precheck logic with the marker absent and with a tag/version
  mismatch) — each fails with the named reason, zero publishes.
- [x] No placeholder remains except the two tracked external
  preconditions (GitHub URL value, approval marker), both named in
  HANDOFF.

## 4. Verification

- [x] `cargo fmt --all -- --check`; `cargo test --workspace`;
  `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `node scripts/check-openspec-change-names.mjs`;
  `openspec validate --changes --strict --no-interactive`
- [x] State recorded in `HANDOFF.md`: implementation of the pipeline is
  DONE; the release itself is BLOCKED pending owner approval with the
  exact commands (`git remote add`, `github_metadata.py … --publish`,
  tag `v0.1.0`). Two-commit cadence; do not push.
