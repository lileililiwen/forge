# Design: crates.io publication pipeline and policy

## 1. Implementation boundary

- Repository: `rust-platform-libs` only.
- Changed files: root `Cargo.toml` (`[workspace.package]` publish/metadata
  keys), six crate manifests (metadata inheritance; `platform-arch-tests`
  keeps `publish = false` explicitly), new `.github/workflows/publish.yml`,
  new `docs/publication.md`, new `PUBLICATION-APPROVED` marker file (added
  only when the owner approves; absent ⇒ CI refuses), README/ROADMAP status
  lines, HANDOFF at completion.
- Must not change: any crate's public API, `ci.yml` semantics,
  `platform-arch-tests` rules, sibling repos.

## 2. Language and runtime

- Rust toolchain per `rust-toolchain.toml` pin; cargo ≥ 1.75 (workspace
  MSRV), Actions runner cargo for publishing.
- Commands: `cargo publish --dry-run -p <crate>` per crate in topological
  order; `cargo package --list` diff against expected file set; the real
  `cargo publish` executes only in the tag workflow with the secret.

## 3. Ownership and shared code

- This repo owns artifacts and the pipeline; consumers own adoption
  (`cargo add` migrations in their changes). Registry versions are the
  release boundary: a published version is immutable.
- Intra-workspace deps gain registry form: `platform-core = { path =
  "../platform-core", version = "0.1.0" }` — path keeps local development
  fast while `cargo publish` rewrites to version requirements. Applied to
  every inter-crate dep: contracts→core, identity→core, audit→core(+
  contracts?), testing→core/identity/audit, mailing→core, ai→core — exact
  DAG read from `platform-arch-tests` lockfile walk at implementation.
- `platform-arch-tests` and its dev-only deps never appear on the registry
  (workspace-internal tooling).

## 4. Behavioral model

| Element | Rule |
|---|---|
| Versioning | one workspace version in `[workspace.package]`; bump + `vX.Y.Z` tag + `docs/publication.md` changelog line land together in one commit |
| Pre-1.0 | `0.y.z`: minor bumps may break; patch must not; the policy file states it; `0.1.0` is the reserved first release |
| Publish trigger | tag push `v*` only; branch pushes never publish (no implicit action) |
| Order | topological: `platform-core` first, then the rest; the workflow computes order from an explicit list checked against the DAG test |
| Propagation | after each publish, poll `crates.io/api/v1/crates/<name>/<version>` up to 15 min; failure ⇒ job error naming the crate |
| Idempotence | pre-flight 404-check; if the version already exists (rerun after partial success), the crate is skipped, not re-published |
| Yank | manual `cargo yank` per policy doc; every yank recorded in `docs/publication.md` with reason and replacement version |

State transition: `publish = false (today) → metadata-ready (dry-run green)
→ approved (marker + secret) → released (tag v0.1.0 live)`. Each edge is
separately evidenced.

## 5. Contract and compatibility

- Crate names: the seven exact names verified free on 2026-09-30 (crates.io
  API 404); workflow re-checks immediately before each publish (names are
  first-come).
- Manifest metadata (workspace defaults): `repository = <GitHub URL —
  value arrives with the precondition>`, `homepage`, `documentation =
  docs.rs`, `readme`, `license = "Apache-2.0"` (already), per-crate
  `description` (present from consumer-baseline work), `keywords`,
  `categories`, `exclude = ["/.github", "/openspec", "/tests/**/fixtures"]`
  where archives must shrink.
- CI secret: `CARGO_REGISTRY_TOKEN` — crate-scope `platform-*`, no
  user-wide token; rotation documented.
- Backward compatibility: none required (first release); from `0.2.0` the
  pre-1.0 rule binds.

## 6. Failure and boundary policy

| Case | Behavior |
|---|---|
| Tag semver ≠ workspace version | job fails before any publish, names both values |
| Any dry-run/verify failure | job aborts; zero publishes |
| `PUBLICATION-APPROVED` absent or secret missing | job fails with the precondition message; never partial |
| Registry rejects (auth/permissions) | crate-level failure surfaces; preceding published crates stay (immutable) ⇒ procedure: fix, `yank`, bump patch |
| Propagation timeout | job error; rerun idempotence (skip-existing) makes retry safe |
| Dirty tree during package list check | `--allow-dirty` is banned; failure |

## 7. Verification oracle

- Local: `cargo publish --dry-run` for all seven (six publishable + confirm
  arch-tests refuses) green; `cargo package --list` asserted to exclude
  `openspec/` and `.github/`.
- Workflow test matrix: version-mismatch fixture fails; missing-marker
  fails; order list matches DAG (asserted by a unit test reading the same
  manifest table the job uses).
- Release evidence (out-of-band, owner-executed): after approval, tag
  `v0.1.0`; then from a clean project `cargo add platform-mailing` +
  `cargo build` resolves from the registry — recorded in HANDOFF with the
  crates.io URLs. Until then the change's tasks show the release step
  BLOCKED with the exact precondition — never checked off.
- `openspec validate --changes --strict --no-interactive` + existing CI
  unchanged and green.

## 8. Decision ledger

| # | Decision | Choice | Alternatives rejected |
|---|---|---|---|
| P1 | Names | bare `platform-*` (all free, re-checked at publish) | squat-risk prefixes (worse discoverability for no safety gain) |
| P2 | Versioning | workspace lockstep | per-crate semver (breaks the co-released contract family and the parity walkers) |
| P3 | Trigger | tag + approval marker + crate-scoped secret | branch push or manual dispatch (implicit publication violates repo invariants) |
| P4 | Internal crate exclusion | `platform-arch-tests` publish=false override | publishing dev-only tooling |
| P5 | Partial-failure procedure | yank + patch bump (immutable registry honored) | force-republish (impossible) or unpublish myth |
| P6 | Precondition | GitHub publication + owner approval tracked in workspace-governance flow BEFORE implementing §7 release | pretending sources stay private post-publish |
| — | Blockers | ONE: owner approval for public GitHub + crates.io publication (external, irreversible). Everything else in this package is implementable locally | — |

## Traceability

| Req | Outcome | Design | Boundary | Scenarios | Tasks | Oracle |
|---|---|---|---|---|---|---|
| PUB-001 publishable metadata completeness | §What #1–2 | §3 §5 | manifests | success/failure | 2.1–2.2 | §7 dry-run+list |
| PUB-002 tag-gated ordered publishing | §What #3 | §4 §5 §6 | publish.yml | success/failure/boundary | 2.3 | §7 matrix + idempotence |
| PUB-003 version/yank policy | §What #4 | §4 §5 | docs/publication.md | transition cases | 2.4 | §7 evidence rules |
| PUB-004 approval precondition | Blocker handling | §6 §8 | marker + secret | refusal scenario | 1.x, 2.3 | §7 missing-marker fails |
