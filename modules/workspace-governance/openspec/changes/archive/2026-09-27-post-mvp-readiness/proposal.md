# Proposal: Post-MVP readiness — README usage, terminal capture, vocabulary

## Why

`workspace-governance` is the workspace's shared governance owner: it owns the
project registry, governance profiles, bootstrap templates, evidence vocabulary,
and deployment declarations, and the read-only Forge governance adapter. Two
changes shipped in sequence (`registry-and-declaration-integrity`,
`shared-rules-single-source`), the workspace audit is at **0 errors / 18
warnings**, `python3 -m unittest discover -s tests` reports **122 tests, 0
failures**, and six canonical specs validate under `--strict` (`HANDOFF.md`).

The README is 182 lines and accurately states ownership, the mandatory command,
onboarding, promotion, and Jenkins handoff. It is a **shared/infrastructure**
project, not a product, so readiness means a new maintainer or consumer can
*use* the governance surface, not that a UI exists. Three documentation gaps
remain:

1. **No consolidated CLI usage.** The commands are spread across the
   "Mandatory command", "Onboard a project", "Deployment promotion", and
   "Jenkins" sections, so there is no single copy-paste reference for the
   common tasks (self-check, audit, auto-gate, quality check, maturity, drift,
   init, promote, manifest, Forge adapter).
2. **No terminal evidence.** The repository is CLI-only, so screenshots are
   **not applicable**, but there is no checked-in capture of a real
   `workspace_check.py --self-check` or `--audit --json` run to show what a
   healthy and an unhealthy audit actually look like.
3. **Registry/evidence vocabulary is scattered.** "Declaration", `evidence_status`
   (`declared`/`configured`/`verified`/`unverified`), `release_evidence`,
   `deployable`, the promotion ledger, and the finding codes are explained across
   `README.md`, `docs/capabilities.md`, `docs/product-code-quality.md`, and the
   canonical specs, but nowhere is the distinction — **declaration is intent,
   evidence is verification, `release_evidence` backs a deployment, `deployable`
   requires both** — stated in one place. This is exactly the confusion packages
   1 and 2 removed from the tooling; the docs still carry it.

Readiness here means a consumer can copy a working command, see a real audit
capture, and look up the vocabulary without reading the scripts.

## What Changes

- Add a consolidated **Usage** section to `README.md` with copy-paste examples
  for each command (`workspace_check.py --self-check` / `--audit` /
  `--project`, `auto_gate.py`, `product_code_quality.py`, `maturity_audit.py`,
  `rules_drift.py`, `init_project.py`, `promote_project.py`,
  `jenkins_manifest.py`, and the Forge adapter).
- Add a checked-in **terminal capture** under `docs/assets/` showing a real
  `--self-check` and `--audit --json` run (healthy and one finding), redacted and
  reproducible offline, plus a short **"Screenshots are not applicable"** note
  (CLI-only).
- Add a **Vocabulary** section (or `docs/vocabulary.md`) that defines the
  registry/evidence terms and links `docs/capabilities.md`,
  `docs/product-code-quality.md`, and the canonical specs instead of restating
  policy.
- Keep the documentation-only boundary: no script, checker, schema, template, or
  canonical-spec behavior changes.

## Package Boundary and Split Assessment

This change is **one package** (`post-mvp-readiness`), not a split.

- **Single outcome:** a consumer can use the governance surface, see a real
  audit, and look up the vocabulary from the repository alone.
- **Included:** `README.md`, new assets under `docs/assets/`, an optional
  `docs/vocabulary.md`, and the change folder.
- **Excluded:** `scripts/**`, `tests/**`, `schemas/**`, `profiles/**`,
  `templates/**`, `projects.json`, `deployment/**`, the canonical specs, and any
  behavior change. A `LICENSE` file is explicitly out of scope for this
  docs package (see Non-goals).
- **Split signals considered:** the usage section, the capture, and the
  vocabulary share one owner (`README.md`/`docs`), one lifecycle (repository
  documentation), one contract (the docs link checks and strict OpenSpec
  validation), and one oracle (a consumer can run a command and resolve a term).
  Splitting them would leave the README partially documented. The scripts and
  checkers have their own oracle (`python3 -m unittest discover -s tests` +
  `workspace_check.py --self-check`) and are explicitly untouched.
- **Dependencies:** the shipped `registry-and-declaration-integrity` and
  `shared-rules-single-source` changes; the authored
  `portfolio-vocabulary-normalization` and `governance-front-door` packages
  (the vocabulary section links the vocabulary source; the front-door skill links
  this README).

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| this repository (shared owner) | `projects.json`, `scripts/*.py`, `schemas/*.json`, `profiles/*.json`, `docs/capabilities.md`, `openspec/specs/*` | the registry, checkers, evidence vocabulary, and canonical specs | usage is scattered; no terminal capture; vocabulary not defined in one place | **workspace-governance (shared owner)** | **extend shared owner** — this is the owner repository; the docs describe the vocabulary it owns |
| product repositories | `.project.json` (`kind`, `profile`, `evidence_status`, `release_evidence`) | declarations that consume the vocabulary | consume only; they must not re-declare terms | each product | **keep local** — consumers, documented, not edited |
| platform-contracts | `schemas/registry.json`, `secret_field_substrings` | language-neutral schema registry; a second encoding of the secret list | a consumer/sibling encoding; reconciliation is `contracts-consumer-parity`'s job | platform-contracts | **keep local** — not this change |
| driftwatchdog | gate runtime declared in each `.project.json` | shared quality/evidence gate runtime | the gate runtime is owned elsewhere; this repo declares and consumes it | driftwatchdog | **keep local** — no doc artifact to adopt |
| forge | `forge governance use workspace-governance --adapter …` | the control-plane consumer of the adapter | forge owns its own UI/export surface | forge | **keep local** — this repo only ships the adapter and documents its contract |
| dotnet-platform-libs / rust-platform-libs | platform templates | platform adoption packages | unrelated to the governance docs surface | each platform repo | **not applicable** |

**Shared-owner record:** `workspace-governance` IS the shared owner of the
registry/evidence vocabulary. The other repositories **keep local** and consume
it; none owns a second governance vocabulary. This change documents the
shared-owner surface and MUST NOT change the behavior it documents.

## BFS Impact Map

| Surface | Impact | Kind |
|---|---|---|
| `README.md` | Add Usage, terminal capture embed, screenshots-N/A note, and Vocabulary section; keep ownership/mandatory-command/promotion/Jenkins accurate | Documentation |
| `docs/assets/` (new) | Checked-in redacted terminal capture of `--self-check` and `--audit --json` | Documentation |
| `docs/vocabulary.md` (optional) | Definitions linking the canonical sources rather than restating policy | Documentation |
| `scripts/**`, `tests/**`, `schemas/**`, `projects.json`, `deployment/**` | Read-only; the Usage commands invoke them; no behavior change | Verification-only |
| `openspec/specs/**` | Untouched; the new `readiness` capability is added by this change only | None |
| Sibling repositories | None | None |

- **Failure and boundary:** a usage example that names a flag or output the
  scripts do not produce MUST be corrected to real behavior; the capture MUST be
  redacted of tokens, host usernames, absolute private paths, and secret-shaped
  values (the adapter already redacts secret-shaped substrings).
- **Compatibility, privacy, security:** the capture MUST NOT contain workspace
  roots, credentials, or secret values; no external image host is introduced.

## Capabilities

### New Capabilities

- `readiness`: post-MVP documentation readiness for the shared governance owner
  — consolidated CLI usage, a redacted terminal capture with an explicit
  screenshots-not-applicable justification, and a single vocabulary entry point
  that links the canonical sources.

### Modified Capabilities

- None. No registry, checker, schema, template, promotion, or spec behavior
  changes; this change updates documentation only.

## Non-goals

- No change to any script, checker, schema, profile, template, `projects.json`,
  promotion ledger, or canonical spec.
- No change to governance behavior, gates, or promotion semantics.
- No new governance authority and no push/deploy/promotion instructions.
- No GUI screenshots; the project is CLI-only and captures are terminal-only.
- No `LICENSE` file in this package; it is not in the stated readiness focus and
  adding one is a separate metadata decision, not part of a docs package.
- No edits to sibling repositories; they consume the vocabulary, and this change
  only documents it.
