# Design: The contract parity gate compares, or it is not a pass

## 1. Implementation boundary

Repository `/home/paul/code/forge`, POSIX `sh` script plus mirrored data. No
Rust change. `AGENTS.md` mandates no shell dialect and the file is already
`#!/bin/sh` with `set -eu`; that is preserved. `python3` is already used by
this script and by `scripts/release-check.sh` (CI's `surfaces` job pipes
`forge check` through it), so it is an accepted dependency; the comparison
itself is plain `sh` so that the pass/fail verdict never depends on a
programmer-language runtime.

Changed files:

- `scripts/contract-parity.sh` — rewritten to compare.
- `scripts/release-check.sh` — comment and docstring corrected; the invocation
  and its blocking behaviour are unchanged.
- `contracts/registry.json`, `contracts/schemas/public-portfolio-manifest.schema.json`
  — mirror re-synced from the source.
- `contracts/manifest.json` — the two digests above updated, one entry added.

`platform-contracts/` is read-only. No file in it is edited.

## 2. What the script compares

Two independent anchors, because each covers a different mistake, and the two
sides contribute a different declaration.

**What each side declares.** The **mirror** declares what it *retains*: every
file under `contracts/` except `contracts/manifest.json` (Forge's own record)
and `contracts/vocabulary/**` (derived, or owned by `workspace-governance`). The
**source's own `manifest.json`** declares what it *publishes*: one entry per
family, each carrying a `schema` path and a `schema_digest`, plus a
`registry_revision`.

**Anchor A — byte comparison of everything the mirror retains.** Each retained
file is mapped to its source counterpart and both sides are hashed. The map is
`contracts/registry.json` ↔ `<src>/schemas/registry.json` (the rename
`scripts/sync-contracts.mjs` performs) and `contracts/<rel>` ↔ `<src>/<rel>` for
everything else. A retained file with no source counterpart fails as
`parity-undeclared-vendor`, which catches an upstream deletion as well as a
vendored invention.

**Anchor B — the contract's own record.** For every family the source's record
declares, the mirror's digest is compared against that record's `schema_digest`,
and a family the record publishes but the mirror lacks fails as
`parity-missing-mirror`. This is the requirement that the digest authority be
the contract's own record: a mirror that agrees with Forge's manifest but
disagrees with the source's record fails here.

The anchors are complementary and neither is derived from the other. The
source's `manifest.json` does not digest `registry.schema.json` or
`envelope.schema.json`, so Anchor A is the only coverage those have;
conversely Anchor B is bound to a record produced by the contract owner rather
than to a byte count that happened to match.

**Why the published set comes from the record, not from the source's directory
listing.** The source checkout is a working tree, not a release. At the time of
writing it carries an uncommitted `schemas/consumer-record.schema.json` and other
in-flight work. Deriving the required set from "every file under
`<src>/schemas`" would demand that Forge vendor a file its owner has not
published and may never publish, and would make Forge's release gate hostage to
someone's uncommitted scratch state. Deriving it from the record keeps the
requirement set equal to what is published, which is the question parity is
actually asking. The gate therefore reports this tree as needing **10** family
digests, and ignores `consumer-record.schema.json`; when that file is published
and the record declares a family for it, the mirror will be required to carry
it.

## 3. Failure policy

`set -eu`, one non-zero exit per class, no pass line on any failure. The `OK`
line is emitted at exactly one place, after the comparison count is known to be
greater than zero and after every failure list is empty.

| class | condition | exit |
| --- | --- | --- |
| `parity-source-unresolvable` | no `PLATFORM_CONTRACTS_DIR` and no sibling checkout | 1 |
| `parity-source-record-invalid` | source `manifest.json` missing, unparseable, or carrying no families | 1 |
| `parity-self-reference` | resolved source resolves to `contracts/` itself | 1 |
| `parity-mismatch` | retained file digest ≠ source digest | 1 |
| `parity-undeclared-vendor` | retained file with no counterpart in the source | 1 |
| `parity-missing-mirror` | family the record publishes with no retained schema | 1 |
| `parity-family-digest-mismatch` | mirror digest ≠ the record's `schema_digest` | 1 |
| `parity-nothing-compared` | comparison count is zero | 1 |

Mismatches are accumulated and all reported before exiting, so one run names
every drifted file rather than only the first. Exit is 1 for every class: these
are check verdicts, and no caller distinguishes severity.

## 4. Inputs that may not weaken the comparison

`PLATFORM_CONTRACTS_DIR` locates the source and does nothing else. The
resolution order is unchanged from the current script: the variable when it
names a directory, otherwise the `../platform-contracts` sibling. A value that
is not a directory falls through to the sibling exactly as today.

The properties that make the check non-weakenable:

- **No mode flags.** There is no `--skip`, `--ignore`, `--report-only`,
  `--advisory` or `--no-strict`. A comparison cannot be turned off.
- **No counted-as-passed outcomes.** A retained file the source no longer
  publishes is a failure, not an exclusion. A family the record publishes and
  the mirror lacks does not reduce the comparison count; it fails.
- **The count cannot be gamed downward.** The count only gates the `OK` line; it
  never gates a failure. Driving it to zero produces
  `parity-nothing-compared` and a non-zero exit.
- **Self-reference is refused.** A `PLATFORM_CONTRACTS_DIR` pointing at
  `contracts/` resolves the source to the mirror, every digest matches itself,
  and the script exits non-zero naming the reason. Without this guard, the one
  input that could make the check pass trivially would be the environment
  variable naming the very directory under test.
- **`contracts/manifest.json` is excluded from the parity decision**, so
  regenerating Forge's manifest to match a stale mirror cannot influence the
  verdict.

## 5. Digest authority — why not `contracts/manifest.json`

`verify_manifest_digests()` (`src/contract/mod.rs:349`) re-hashes `contracts/**`
against `contracts/manifest.json`. Both sides are Forge's own, and
`vendored_revision()` (`src/vocabulary.rs:291`) reads its `revision` from the
same self-referential pair. That mechanism is correct for what the canonical
spec asks of it — offline tamper detection over a committed copy — and it is
kept unchanged. It is simply not a parity oracle: a copy that drifted a year ago
still matches its own manifest perfectly.

So the two records coexist with different jobs. `contracts/manifest.json` answers
"were these exact bytes tampered with since they were committed?" and is checked
by the Rust test suite with no sibling checkout present.
`platform-contracts/manifest.json` answers "are these bytes still what the
contract owner published?" and is checked only by the parity script, which
requires the resolved source. Neither substitutes for the other, and the parity
verdict never reads Forge's.

## 6. Decision ledger

**Resolved.** `contracts/` stays as a **verified mirror** of the resolved
`platform-contracts` source, byte-exact, with a re-sync that makes the gate pass
on a truthful tree. The parity digest authority is the source's own
`manifest.json`. A missing source is a hard failure. Zero comparisons is a
failure. No input weakens the comparison, and the one that could — a source
resolving to the mirror — is explicitly refused. `release-check.sh` and CI both
invoke the script and a mismatch blocks.

**Open, left to the owner.** Whether Forge should adopt
`platform.public-portfolio-manifest` as a *supported* family. The mirror now
carries its schema, but `supported_families()` and `family_schema_path()` in
`src/contract/mod.rs` are unchanged. Adopting it is a product decision about
what Forge emits, and it is deliberately not taken here.

### Decision 1 — vendored copy: **keep as a verified mirror**

*Options considered.* (A) delete `contracts/` and resolve the source at
runtime; (B) keep it as a verified byte-exact mirror.

*Why (A) lost.* `contracts/` is not a convenience cache; it is the runtime
substrate of the archived, canonical `platform-contract-consumption` capability.
`family_schema_path()` resolves a family's schema,
`secret_field_substrings()` reads the vendored vocabulary, `forge contract emit`
and `forge contract validate` load the bytes, and `verify_manifest_digests()`
hashes them. The canonical spec states Forge "SHALL verify those digests
offline without reaching a sibling checkout or the network", and the
`Additive-only compatibility boundary` requirement forbids introducing a new
network or sibling dependency. Deleting the tree converts an offline capability
into a checkout-dependent one, inverts that requirement, and touches four Rust
call sites — a different change with a different owner. The producer side
separately requires the documented contract not to present an undeclared copy as
a supported adoption path, but it permits a *declared* one: declaring and
verifying the mirror is the compliant shape.

*What was kept from the rejected option:* the enforcement discipline itself. A
mirror with no comparison behind it is the thing being fixed; the mirror is not
the defect.

*Cost accepted:* the repository still commits upstream bytes, and a real
mismatch now blocks instead of passing silently. That is the intended cost.

### Decision 2 — `registry.json` `current_version`: **the source's values, verbatim**

Because the mirror is byte-exact, `contracts/registry.json` becomes the source's
`schemas/registry.json`. Concretely: the nine families Forge already mirrored
keep `current_version` `0.1.0`, and the mirror **gains**
`platform.public-portfolio-manifest` at `1.0.0` with `supported_major_versions`
`[1]`. No existing family's version is rewritten and none is pinned separately by
Forge.

*Why this is safe:* no Rust code reads `current_version`. The only reader in the
repository was this script's own reporting loop, which is being replaced.
`supported_families()` and `family_schema_path()` are hand-maintained
`match`/`vec!` tables in `src/contract/mod.rs` and are untouched, so the new
family's presence in the mirror does **not** make `forge contract list` advertise
it, and `forge contract emit public-portfolio-manifest` still refuses. That is
the intended separation: the mirror tracks the source, while what Forge supports
stays a deliberate product decision. Were the tables derived from the registry
instead, the mirror update would have silently widened Forge's public surface —
a compatibility change that belongs in its own package.

*Note on `registry_revision`.* The script reports the source's
`registry_revision`. It is **reported, not enforced**: upstream currently sets
`registry_revision` to the sha256 of its own `schemas/registry.json`, and that
was verified, but the relationship is an upstream convention rather than part of
the published contract, so this check does not make its own mirror fail if the
convention changes. The registry's *bytes* are compared regardless, under Anchor
A, so a registry that drifted is still caught. Forge does not pin the revision
in `contracts/manifest.json`; the `revision` field there records the source
commit the mirror was synced from and is left truthful by the re-sync.

### Decision 3 — re-sync method: **targeted copy, not a full `sync-contracts.mjs` run**

`scripts/sync-contracts.mjs` is the sanctioned sync tool and both sibling
checkouts are present, but running it would also restamp `synced_at`/`revision`
and — because `collect()` records every non-`manifest.json` file it finds —
would newly record `contracts/vocabulary/README.md`, which today's
`contracts/manifest.json` does not list. That is a pre-existing manifest gap
unrelated to parity; folding it in here would widen this change and alter the
archived capability's manifest for a reason that has nothing to do with contract
parity. The re-sync is therefore exactly the three drifted files
(`registry.json`, `schemas/registry.schema.json`, and the newly published
`schemas/public-portfolio-manifest.schema.json`) plus the three
`contracts/manifest.json` digest updates and one new entry, leaving the gap
visible for its own change.

## 7. Verification oracle

1. **Corrupted mirror** — append a byte to a vendored schema, run the script,
   assert non-zero and assert the string `OK` is absent from stdout *and* stderr.
2. **Correct mirror** — after the re-sync, assert exit 0 with a comparison count
   greater than zero.
3. **No source** — run with `PLATFORM_CONTRACTS_DIR` unset and the sibling
   temporarily unreachable, assert non-zero and no pass line. The sibling is
   exercised by running from a directory that has no `../platform-contracts`
   rather than by moving a checkout.
4. **Self-reference** — `PLATFORM_CONTRACTS_DIR` pointed at `contracts/`, assert
   non-zero.
5. **Regression** — `cargo test --workspace --all-targets`,
   `openspec validate --all --strict --no-interactive`, `git diff --check`, and
   a `git diff` review of `.github/workflows/ci.yml` and
   `scripts/release-check.sh` proving no existing job, check or divergence was
   removed or weakened.

## 8. What this change does not verify

Stated so the gate is not read as more than it is. The script compares **vendored
schema and registry bytes against the resolved source tree**. It does not:

- execute the source's fixture matrix (`platform-contracts/tools/parity.py` owns
  that);
- validate any document against a schema;
- check that the source tree is itself correct, only that the mirror matches it;
- fetch upstream by pinned SHA, so it does not detect a source checkout that has
  itself moved past the pinned revision;
- require a file the source's record does not publish, so an uncommitted file in
  a source working tree is deliberately not demanded of the mirror (see §2);
- enforce that the source's `registry_revision` equals any particular digest —
  it is reported, not asserted;
- prove Forge's *supported* family set matches the registry
  (`supported_families()` is hand-maintained and unchanged);
- cover `contracts/vocabulary/**` or `contracts/manifest.json`.
