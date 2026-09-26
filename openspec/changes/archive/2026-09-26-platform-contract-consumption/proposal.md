# Proposal: Consume the portfolio contract source instead of re-declaring it

## Why

`platform-contracts` is the portfolio's contract source: JSON Schemas, a
machine-readable registry (`schemas/registry.json`), a canonical fixture matrix
(`fixtures/<family>/<category>.<case>.json` plus a per-family `manifest.json`)
and a validator. It defines the envelope every shared document must wear —
`{contract, generated_at, payload}` with `contract` matching
`^platform\.[a-z][a-z0-9]*(?:-[a-z0-9]+)*\/[0-9]+\.[0-9]+\.[0-9]+$` — and nine
families at `0.1.0`, including four whose producer or consumer is a Forge
surface today: `platform.gate-result` (owner runtime `driftwatchdog (gate
runner)`), `platform.readiness`, `platform.release-evidence`,
`platform.audit-event`, `platform.capability` and `platform.job-outcome`.

Forge consumes none of it. A content search of this repository for
`platform-contracts` and for any `platform.<family>` identifier returns **no
matches**, while Forge declares **28 separate `pub const … = "0.1.0"` version
constants across 27 modules** plus three independently spelled namespaced
discriminators (`forge-checker/0.1.0` in `src/checker/mod.rs:36`,
`forge-deploy-executor/0.1.0` in `src/deploy/mod.rs:96`, and the inbound
`driftwatch-checker/0.1.0` in `src/policy/mod.rs:42`). Each Forge surface also
invents its own status vocabulary that overlaps the contract's without being
it: the gate aggregate is `passed|blocked|failed|unknown`
(`src/gate/mod.rs:89-94`) against `platform.gate-result`'s
`passed|failed|errored|skipped`; readiness is `passed|failed|unverified`
(`src/readiness/mod.rs:44-48`) against `platform.readiness`'s
`ready|degraded|not_ready`; the operations journal's `state` column is free-form
`TEXT NOT NULL DEFAULT 'pending'` (`src/registry/mod.rs:51`, written from
`record_operation` at `src/registry/mod.rs:336`) whose per-plane callers use
`done|failed|blocked|skipped|partial|rejected|unverified` against
`platform.job-outcome`'s `succeeded|failed|partial|cancelled` and
`platform.audit-event`'s `success|failure|denied|unknown`. Two vocabularies for
one fact means a consumer has to guess, and today nothing checks the guess; the
untyped column is exactly the drift an inventory is meant to surface.

`platform-contracts` has an active change, `contracts-consumer-parity`, whose
stated problem is exactly this drift: "**a contract change can break a consumer
without any build noticing, and two consumers can disagree with the source
silently**", "**nothing clones this repository in consumer CI**", and the
secret-field vocabulary is "declared here (`schemas/registry.json:
secret_field_substrings`) and re-derived in `rust-platform-libs` and
`workspace-governance`". Forge is a third re-derivation:
`policy::redact_credentials` (`src/policy/mod.rs:932`) carries its own
credential-shaped word set and is already the shared redactor imported by
eleven other modules, so the duplicate lives at the busiest chokepoint in the
codebase.

`rust-platform-libs` shows the consumption path is real, not theoretical: it
publishes a `platform-contracts` crate and a cross-repository fixture-parity
test (`crates/platform-contracts/tests/fixture_parity.rs`) that resolves the
source through `PLATFORM_CONTRACTS_DIR` or a `../platform-contracts` sibling
checkout and **fails with an explicit next-action message** rather than
silently skipping. Forge must reach the same guarantee while keeping its own
harder invariant: Forge runs and tests with **no** sibling checkout, no network
and no account (`README.md` "Optional governance providers"; AGENTS.md
invariants).

## What Changes

- Add a **vendored contract set** under `contracts/`: the nine
  `*.schema.json` files, `registry.json`, `vocabulary/secret-field-substrings.json`
  and a `contracts/manifest.json` recording the source repository, the pinned
  source revision, and the sha256 of every vendored file. A
  `scripts/sync-contracts.mjs` regenerates the set and manifest from a sibling
  checkout; an offline test verifies the vendored bytes still hash to the
  pinned manifest, so drift is a test failure instead of a surprise.
- Add a **`src/contract` module** owning the envelope type, the family
  identifiers with their supported major versions, the Forge→contract status
  map, and a **contract inventory** (`CONTRACTS: &[ContractSpec]`) that
  enumerates every versioned surface Forge produces — one row per family with
  its discriminator, version, producing module and documentation path.
- Add the **CLI read surface** `forge contract list | inspect <family> |
  emit <family> [TARGET] | validate <file|->` (human and `--format json`).
  `emit` projects an existing Core record into a `platform.*` envelope; it is
  strictly read-only and journals nothing.
- **Dual-emit, never replace:** every existing Forge document keeps its current
  bytes, discriminator and exit codes. `platform.*` envelopes are additional
  projections of the same Core records. Consumer rules from
  `platform-contracts/docs/ownership.md` become typed refusals: unknown family
  or unsupported major → refused; payload failing the published schema →
  refused; an unmappable status string → refused rather than defaulted.
- **Single-source the secret vocabulary:** `policy::redact_credentials` unions
  the consumed `secret_field_substrings` into its existing patterns instead of
  maintaining an independent word list; no term is dropped and the eleven
  importing modules keep calling one function.
- Add `scripts/contract-parity.sh` as the runnable parity entry point against a
  pinned checkout (the CI job that invokes it belongs to
  `artifact-and-ci-baseline`; this change ships the script, the vendored set
  and the tests).
- Record in the module docs which vocabulary is authoritative for which
  surface, so `forge gate`, `forge readiness`, `forge check`, `forge fleet` and
  the operations journal each state whether Forge's own words or the
  contract's words are the reported fact.

## BFS Impact Map

- **Capabilities:** new `platform-contract-consumption`; touches
  `gate-runtime-evidence` (gate-result projection),
  `external-checker-emission` (severity vocabulary reconciliation, emission
  unchanged), `external-planes-analytics` and `profile-and-release-readiness`
  (readiness projection), `release-publishing` (release-evidence projection),
  `core-manifest-registry` (journal → audit-event projection),
  `quality-policy-integration` (shared redaction), and
  `governance-provider-contract` / `fleet-registry-observation` (input
  vocabulary stays as-is; only the inventory lists them).
- **Users and flows:** a portfolio consumer that already validates
  `platform.*` documents can accept Forge output without a Forge-specific
  parser; operators can print the contract form of a gate or readiness verdict
  and diff it against the sibling runtime's own export.
- **Contracts/data/persistence:** no persistence change. `contracts/` is
  vendored, versioned-by-manifest source, not runtime state; existing
  `.forge/**` records are byte-unchanged; the envelope is derived at read time.
- **Integrations/configuration:** `platform-contracts` becomes a declared
  upstream (build-time vendoring plus an optional parity checkout
  `PLATFORM_CONTRACTS_DIR`); no new runtime binary, no network call, no new
  `FORGE_*_BIN` probe. `FORGE_CONTRACTS_DIR` only affects the ignored parity
  test, never a default workflow.
- **Callers:** CLI first. **No** MCP tool, API route or portal section is added
  (the mature registry admits operation-bearing surfaces only through its own
  classification cycle, the precedent `gate` and `fleet` both followed).
- **Failure/boundary behavior:** absent or corrupt `contracts/manifest.json` →
  `forge contract *` refuses with a typed error naming the file; a family
  outside the vendored registry → refused by name; an unmappable or unknown
  status string → refused, never emitted as `passed`; a document carrying a
  registered secret field name → refused before validation completes;
  an unreachable source for the parity walk → the ignored test fails loudly
  when asked to run, while the default `cargo test` stays standalone-green.
- **Tests:** envelope shape and `contract` pattern; the status map as total
  functions with refusal cases; the inventory completeness test (every version
  constant is registered, no unregistered constant exists); offline vendored
  digest test; `emit` projections for the five families with byte-identity
  proofs that the underlying Forge records did not change; `validate`
  accept/refuse matrix driven by the vendored canonical fixtures; redaction
  union test proving old and newly consumed terms both redact; cross-surface
  test proving `forge gate status`, `forge check`, `forge fleet list` and
  `forge provider matrix` output is unchanged.
- **Dependencies:** none inside Forge beyond `src/core`; soft dependency for
  the CI wiring on `artifact-and-ci-baseline`; `governance-vocabulary-consumption`
  reuses this change's vendoring mechanism.
- **Compatibility/security/privacy:** additive only — no wire format, exit
  code, journal row or manifest key changes; the vendored set is static data
  covered by `git diff --check` review; `jsonschema` enters as a
  **dev-dependency only** so the shipped binary's dependency surface does not
  grow; emitted envelopes carry no host paths and pass the same
  `policy::redact_credentials` and char-bound rules every other surface uses.

## Capabilities

- `platform-contract-consumption`: Forge declares `platform-contracts` as an
  upstream contract source, vendors its schemas behind a digest-pinned manifest,
  registers every versioned surface it produces, and can project existing Core
  records into `platform.*` envelopes that pass the published schemas — while
  every existing Forge document stays byte-identical.

## Non-goals

- Replacing or re-versioning any existing `forge-*` discriminator, status
  vocabulary or output format.
- Collapsing the 28 version constants into one shared constant. They are
  genuinely independent wire contracts whose versions evolve separately; the
  deliverable is an enforced **inventory**, not a merge.
- Emitting `platform.identity-subject`, `platform.permission-decision` or
  `platform.tenant-context`. Forge has no tenant model and no portfolio
  identity issuer; `identity-admin` (C#) and `identity-rs` (Rust) own those
  families, and declaring them would assert a capability Forge does not have.
- Changing what a Gate, checker or provider verdict means. Driftwatchdog owns
  gate semantics (requirement.md §24, §32); this change only transports them.
- CI job restructure, packaging, publishing or installer work
  (`artifact-and-ci-baseline`).
- Governance `kind`/`profile`/marker vocabulary
  (`governance-vocabulary-consumption`).
- New MCP tools, API routes or portal sections.

Source: requirement.md §3.3, §12, §24, §29, §32, §37, §38, §45, §47.
