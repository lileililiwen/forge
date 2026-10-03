# Proposal: The contract parity gate compares, or it is not a pass

## Why

Forge ships a contract parity gate. It compares nothing.

`scripts/contract-parity.sh` is 30 lines. `grep -cE "sha256|cmp|diff"
scripts/contract-parity.sh` returns **0** — there is no digest, no `cmp`, no
`diff` in the file. It prints a vendored-version list read out of
`contracts/registry.json`, prints a fixture count looped over the *source's*
`fixtures/` directory, and then unconditionally prints:

    contract-parity: OK (next step: run ignored parity test with PLATFORM_CONTRACTS_DIR=$SRC)

The `OK` is not conditional on anything. The script cannot fail for a mismatch
because it never looks for one. Its only failure mode is a missing source
directory. A checker that reports OK without checking is worse than no checker,
because it retires the question while answering nothing.

Three compounding facts, each verified in this repository:

1. **`release-check.sh` softens it.** The parity step runs only when
   `PLATFORM_CONTRACTS_DIR` or `../platform-contracts` resolves; otherwise it
   prints a note and continues. The file's own docstring calls it "the contract
   parity walk", not a gate. `ci.yml`'s `contract-parity` job hard-fails only on
   **sibling absence** — the checkout not being present — and never on an actual
   mismatch, because no mismatch is reachable.
2. **Internal coherence is standing in for parity.**
   `verify_manifest_digests()` (`src/contract/mod.rs:349`) reads
   `contracts/manifest.json` and re-hashes `contracts/**` against it. Both sides
   are Forge's own. Upstream `platform-contracts` is never consulted. The test
   `manifest_digests_match` (`src/contract/mod.rs:773`) therefore asserts that
   Forge's copy agrees with Forge's own record of Forge's copy — true of a copy
   that is arbitrarily stale.
3. **The vendored copy is already measurably stale.** Comparing `contracts/`
   against `../platform-contracts/` file by file today: of 12 mirrored files, 10
   are byte-identical, `contracts/registry.json` **differs** from the source's
   `schemas/registry.json`, `contracts/schemas/registry.schema.json` **differs**
   from its source counterpart, and
   `contracts/schemas/public-portfolio-manifest.schema.json` is **absent from
   the mirror entirely** — upstream declares `platform.public-portfolio-manifest`
   at `current_version` `1.0.0` and the mirror does not contain it at all. This
   drift has been invisible precisely because nothing compared the two sides.

The upstream repository's own active change,
`platform-contracts/openspec/changes/contract-consumption-enforcement`, makes
the same ruling from the producer side: its consumption requirement states the
documented contract "MUST NOT present a partial or undeclared vendored copy as
a supported adoption path", and its consumer-drift requirement states a check
subject to contract consumption "MUST NOT be skipped when `platform-contracts`
is not checked out beside the repository". That change defers the Forge-side
remediation. This package is that remediation.

## What Changes

- **Make `scripts/contract-parity.sh` actually compare.** For every file the
  mirror owns, it SHALL compare the **actual bytes** of the vendored copy
  against the **actual bytes** of the resolved `platform-contracts` source, and
  it SHALL report a non-zero exit on any mismatch, any missing mirrored file,
  and any vendored file with no counterpart in the source.
- **Bind the digests to the contract's own record.** For every family the
  source's `manifest.json` declares, the script SHALL compare the mirror's
  digest against that manifest's `schema_digest`, and SHALL report the source's
  `registry_revision`. The digest authority for the parity comparison is the
  contract source's record — never `contracts/manifest.json`, which is Forge's
  own self-referential manifest.
- **Refuse to pass on an empty comparison.** The script SHALL count the
  comparisons it performed. Zero comparisons SHALL exit non-zero with an
  explicit statement that it verified nothing. The unconditional `OK` line is
  removed and is reachable only when every comparison passed and the count is
  greater than zero.
- **Keep a missing source a hard failure.** An unresolvable
  `platform-contracts` source remains exit non-zero. No note-and-continue path
  is added to the script.
- **Close the weakening inputs.** `PLATFORM_CONTRACTS_DIR` continues to *locate*
  the source and nothing else; there is no skip, no ignore, no advisory-only and
  no report-only mode, and the script SHALL refuse a resolved source that is the
  mirror itself, because comparing a directory to itself is the exact
  self-referential trap the change exists to close.
- **Re-sync the mirror so the gate passes on a truthful tree.** Bring
  `contracts/registry.json` and `contracts/schemas/registry.schema.json` to
  their source bytes, vendor the missing
  `public-portfolio-manifest.schema.json`, and update the corresponding
  `contracts/manifest.json` digests. Forge's own manifest stays as the offline
  internal-coherence record required by the archived
  `platform-contract-consumption` capability — the two records now coexist, and
  only the source's record decides parity.
- **Make the invocation blocking.** `release-check.sh` and the CI
  `contract-parity` job already invoke the script; the script's non-zero exit is
  what makes a mismatch block, and the surrounding comments are corrected so they
  no longer describe the step as an informational walk.

## BFS Impact Map

- **Requirements:** `platform-contract-consumption` is the owning capability and
  is delta'd with one `## MODIFIED Requirements` entry plus five
  `## ADDED Requirements`. No other canonical spec changes.
- **Contracts:** the mirror-to-source path map is `contracts/registry.json` ↔
  `<src>/schemas/registry.json` (a rename `scripts/sync-contracts.mjs` already
  performs) and `contracts/<rel>` ↔ `<src>/<rel>` for every other mirrored file.
  `contracts/manifest.json` and `contracts/vocabulary/**` are **excluded** from
  parity: the first is Forge's own record, the second is derived or owned by
  `workspace-governance`.
- **Callers:** `scripts/release-check.sh` (`step env PLATFORM_CONTRACTS_DIR=... sh
  scripts/contract-parity.sh`, under `set -eu`, so a non-zero exit already
  aborts) and `.github/workflows/ci.yml` job `contract-parity`.
- **Persistence:** none. No SQLite table, no migration, no new file outside
  `contracts/`.
- **Integrations:** the resolved `platform-contracts` checkout, read-only. No
  network access is added; the script reads the resolved source tree only.
- **Failures:** `parity-mismatch` (digest differs, both digests named),
  `parity-missing-mirror` (source file with no vendored counterpart),
  `parity-undeclared-vendor` (vendored file with no source counterpart),
  `parity-source-unresolvable` (no source), `parity-source-record-invalid`
  (source `manifest.json` missing or unparseable), `parity-self-reference`
  (source resolves to the mirror), `parity-nothing-compared` (zero comparisons).
  Each is a non-zero exit; none prints a pass line.
- **Tests:** the three acceptance cases (corrupted mirror, correct mirror, no
  source) plus `cargo test --workspace --all-targets`, `openspec validate --all
  --strict --no-interactive` and `git diff --check`.
- **Dependencies and concerns:** none added. `sh`, `sha256sum`/`shasum` and
  `python3` are already used by the repo's existing scripts.

## Capabilities

### Modified Capabilities

- `platform-contract-consumption`: "Digest-pinned vendored contract set" gains
  the external source comparison and the rule that the parity digest authority
  is the source's own record, not Forge's self-referential manifest.

### New Capabilities

None. The parity gate belongs to the capability that already owns contract
consumption; introducing a second capability for one script would split one
owner across two specs.

### Phase

Contract consumption remediation. Depends on `platform-contract-consumption`
(archived) and on the producer-side change active in `platform-contracts`. It
is a prerequisite for that change's consumer-drift requirement and contains no
part of the producer-side work.

### Source requirement sections

`requirement.md` §3.1 (deterministic first), §16 (Contracts) and §34 (CLI
Requirements), read with `AGENTS.md`'s "Verify all scoped requirements,
scenarios, callers and failure boundaries" and the workspace rule that a checker
reporting success without checking is worse than no checker.

## Non-goals

- **No deletion of `contracts/`.** The vendored tree is the runtime substrate of
  the archived `platform-contract-consumption` capability
  (`family_schema_path()`, `secret_field_substrings()`, `forge contract emit`,
  `forge contract validate` and `verify_manifest_digests()` all read it, and the
  canonical spec requires offline digest verification with no sibling checkout).
  Removing it is a different change; `design.md` §6 records the decision and the
  rejected alternative.
- **No new capability, no new CLI verb, no new transport.** The surface is one
  shell script, its two callers, and the mirrored bytes.
- **No auto-widening of Forge's supported family set.** `supported_families()`
  and `family_schema_path()` in `src/contract/mod.rs` stay as they are. The
  mirror tracks the source; what Forge *supports* remains a hand-maintained
  product decision and is not silently extended by a mirror update.
- **No fixture-matrix execution.** Running the source's own fixture matrix is
  `platform-contracts`' `tools/parity.py`, owned by that repository. This change
  compares bytes; it does not re-implement the producer's validator.
- **No network fetch, no pinned-SHA checkout.** The comparison runs against the
  resolved source tree exactly as today's resolution order provides it. Fetching
  upstream by pinned SHA is the producer-side change's subject.
- **No change to any existing CI job's set of checks, to `release-check.sh`'s
  documented local/CI divergence, or to any other gate.**
- **No archive and no commit.** The parent session commits.

## Open decisions — resolved

1. **Keep `forge/contracts/` as a verified mirror, or delete it?** **Taken: keep
   it as a verified mirror.** Deleting it would remove the runtime substrate of an
   archived canonical capability and force a network or sibling dependency into
   `forge contract emit`/`validate`, which the canonical spec forbids offline.
   `design.md` §6 records the options and the reasoning.
2. **What happens to `registry.json`'s `current_version` values?** **Taken: they
   become the source's values verbatim, because the mirror is byte-exact.** No
   existing family's version changes; the mirror *gains*
   `platform.public-portfolio-manifest` at `1.0.0`. No Rust code reads
   `current_version`, so this is a data-only change. `design.md` §7 records it.
3. **Which digest authority for parity?** **Taken: the source's own
   `manifest.json`** (`registry_revision`, `families[*].schema_digest`), plus a
   direct byte comparison for the files the source does not digest.
   `contracts/manifest.json` remains the offline internal record and is excluded
   from the parity decision. `design.md` §5 records it.
