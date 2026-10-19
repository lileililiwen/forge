# Proposal: Publish the platform crates to crates.io

## Why

Roadmap decision D4 (2026-09-30, registered in the workspace decision
register draft) selects crates.io as the consumption model for these crates.
Today `publish = false` sits in `[workspace.package]`, consumers hand-write
sibling path dependencies (`trailCrew`'s `Cargo.toml:38` pattern), and the
consumer recipe from `rust-platform-consumer-baseline` documents exactly
that workaround. Name evidence gathered 2026-09-30 via the crates.io API:
all seven crate names (`platform-core`, `platform-contracts`,
`platform-identity`, `platform-audit`, `platform-testing`,
`platform-mailing`, `platform-ai`) currently return 404 — free — so
first-publish doubles as name reservation; none of the fuzzy search hits
(`mercury-platform-core`, `tless`, `stakpak`) occupy an exact name.

## What Changes

- Flip `publish = true` at workspace level with a per-crate `publish =
  false` override for `platform-arch-tests` (internal dev tooling only).
- Add required registry metadata: `repository`, `homepage`,
  `documentation`, `keywords`, `categories` in `[workspace.package]` /
  crate manifests.
- Adopt lockstep versioning: one workspace version, `vX.Y.Z` tags gate
  publication; a tag-driven CI job verifies, dry-runs, then publishes in
  topological order with registry-propagation retries.
- Document the pre-1.0 semver rule, yank policy, and the partial-failure
  procedure in `docs/publication.md`.

## Package Boundary and Split Assessment

**Single outcome:** `cargo add platform-mailing` works for any consumer,
replacing hand-written paths.

**Included:** publish flags, metadata, tag CI job, version/yank policy docs.

**Excluded (split signals):** every consumer's migration off path deps is
its own change (adoption also proves version pinning); docs.rs fine-tuning;
supply-chain attestation tooling (cargo-vet/attest) — a distinct trust
decision with its own lifecycle.

**Dependencies — one hard blocker, not hidden:** the repository has **no
git remote and no GitHub publication** (verified 2026-09-30:
`git remote -v` empty; no `repository` key anywhere). crates.io
publication makes sources permanently public; it requires an explicit owner
approval and the workspace-governance GitHub flow
(`scripts/github_metadata.py` + push) as a precondition. This package stays
blocked on that named decision, consistent with D4's public intent.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| this repository | `[workspace.package] publish = false`, `rust-platform-consumer-baseline` consumer recipe | workspace dep table already mirrors registry shape (`version = "…"` usable) | no metadata keys, no CI publish path | rust-platform-libs | **extend shared owner** |
| workspace-governance | `scripts/github_metadata.py` (check/apply/publish modes, preserves remotes) | authenticated GitHub publication + metadata verification | operates repo-level, not crate-level | workspace-governance | **adopt** as precondition tool |
| dotnet-platform-libs | packaging/`docs/packages.md` conventions | per-package docs index pattern | NuGet semantics differ | dotnet-platform-libs | **adapt pattern** (docs layout only) |
| trailCrew + future consumers | hand-written `path =` deps | proof of demand | migration is consumer-owned | each consumer | **keep local** (their changes) |

## BFS Impact Map

- **Capabilities:** new `rust-platform-publication`; README/ROADMAP status
  lines; `docs/publication.md` new.
- **Files:** root `Cargo.toml`, seven crate manifests,
  `.github/workflows/publish.yml` (new, tag-triggered), docs.
- **Unaffected:** every crate's public API, tests, arch rules, `ci.yml`.
- **Config/secrets:** `CARGO_REGISTRY_TOKEN` repository secret; owner
  creates a crate-scoped, time-limited token (not user-wide). Nothing else
  secret-adjacent changes.
- **Failure/boundary:** tag/version mismatch, dry-run failure, index
  propagation lag, partial publish (see design §6).
- **Compatibility/privacy:** publishing is irreversible; pre-1.0 yank
  discipline; `platform-testing` ships for consumers' dev-dependencies only
  — production-crate ban in `platform-arch-tests` unchanged.

## Capabilities

- **New:** `rust-platform-publication`

## Non-goals

- No renaming, no vendoring mirrors, no private registry, no cargo-vet,
  no consumer edits, no API changes, no docs.rs configuration beyond what
  crates.io does automatically.

## Evidence boundary

Dry-run packaging (CI + local) proves each publishable crate's archive is
complete and self-describing. The change is only DONE when the owner's
GitHub-publication approval lands and a real `v0.1.0` publish + external
`cargo add` succeeds — recorded in HANDOFF as release evidence. Name
availability was true as of 2026-09-30 and MUST be re-verified at
publication time (404 checks are cheap and decisive).
