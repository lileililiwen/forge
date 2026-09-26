# Design: Portfolio contract consumption

## Ownership and boundaries

| Concern | Owner | Forge's role |
| --- | --- | --- |
| Schema shape of a shared document, its status enums, its compatibility rules and its secret-field name list | `platform-contracts` | Consumer: vendors the published set, validates against it, refuses what it cannot honour |
| Forge's own vocabulary (`forge-*` discriminators, `passed|blocked|failed|unknown`, readiness `passed|failed|unverified`, journal `state`) | Forge | Stays authoritative for Forge output; the contract form is an added projection, never a replacement |
| Gate plan resolution, blocking policy, `gate_runs` history, exit semantics | Driftwatchdog (`quality-policy-integration`, `gate-runtime-evidence`) | Unchanged. This change transports verdicts; it never re-decides them |
| Declaration vocabulary (`kind`, `profile`, capability names, placeholder markers) | Workspace Governance | Out of scope here; `governance-vocabulary-consumption` consumes it through the vendoring mechanism this change establishes |
| Rust contract types and validators | `rust-platform-libs` publishes a `platform-contracts` crate | **Not used as a dependency.** A path dependency on a sibling checkout would break Forge's "runs and tests with no sibling repository" invariant; only its *test conventions* are reused |

The one sentence that governs every decision below: a Forge surface may
**emit** a contract document, but it may never **decide** a contract status the
sibling runtime already decided.

## Vendoring mechanism

```text
contracts/
  manifest.json                       # pins the source revision and every digest
  registry.json                       # copied verbatim from schemas/registry.json
  schemas/envelope.schema.json        # plus the nine family schemas, verbatim
  vocabulary/secret-field-substrings.json
```

- `manifest.json` shape (Forge-owned, versioned):

```json
{
  "schema_version": 1,
  "source": "platform-contracts",
  "revision": "<git commit at sync time>",
  "synced_at": "<rfc3339>",
  "files": [{ "path": "schemas/gate-result.schema.json", "sha256": "<hex>" }]
}
```

- `scripts/sync-contracts.mjs` resolves the source through
  `PLATFORM_CONTRACTS_DIR`, then `../platform-contracts` — the same resolution
  order `rust-platform-libs` uses — and refuses to write anything unless every
  requested schema file exists and parses. It copies verbatim (no rewriting, no
  reordering, no comment stripping) so a diff of `contracts/` is a diff of the
  upstream, and it recomputes the manifest digests.
- `contracts/` is **build-time data, not runtime state**. The default Forge
  workflow needs none of it: no command reads `contracts/` unless the operator
  asks for a contract surface (`forge contract …`) or runs the parity test.
  Standalone behaviour is therefore byte-unchanged.
- Update policy: a sync is an explicit commit that changes `manifest.json`
  `revision` plus the affected schema bytes. A schema change that removes an
  enum value Forge maps *from*, or adds a required field Forge cannot produce,
  is a **major** event for Forge and must arrive with its own OpenSpec change,
  not inside a sync commit.
- Drift detection is a test, not a review habit: an offline test re-hashes
  every file listed in `manifest.json` and fails when bytes no longer match,
  naming the file. That closes "two consumers can disagree with the source
  silently" without any network or checkout.

## Contract inventory: enforcement, not consolidation

The 28 version constants stay where they are. What is new is a registry that
makes each one accountable.

```rust
pub struct ContractSpec {
    pub module: &'static str,        // "gate"
    pub constant: &'static str,      // "GATE_CONTRACT_VERSION"
    pub discriminator: &'static str, // "forge-gate" (namespaced) or ""
    pub version: &'static str,       // "0.1.0"
    pub platform_family: Option<&'static str>, // Some("platform.gate-result")
    pub doc: &'static str,           // "docs/…md" or "openspec/specs/…/spec.md"
}
pub const CONTRACTS: &[ContractSpec] = &[ /* one row per surface */ ];
```

Two tests make the table real:

1. **Completeness.** A test walks `src/**/*.rs` from `CARGO_MANIFEST_DIR`,
   finds every `pub const <NAME>… = "0.1.0"` and every
   `"<name>/0.1.0"` discriminator literal, and fails if any (module, constant)
   pair is absent from `CONTRACTS`. Adding a versioned surface without
   registering it becomes a build failure.
2. **Agreement.** For each row, the test asserts the table's `version` equals
   the live constant's value, so the inventory cannot rot into prose.

`forge contract list` renders this table; `forge contract inspect <family>`
renders one row plus the vendored schema's required fields, enum values and the
supported major versions. No second source of truth is introduced: the renderer
reads the table and the vendored files, nothing else.

## Projections: what `emit` produces

`forge contract emit <family> [TARGET]` reads the record Forge already persists
or computes, then prints an envelope on stdout. It writes no file, journals no
row and mutates no state — the same read-only discipline as `forge check` and
`forge fleet`.

| Family | Source record | Notes |
| --- | --- | --- |
| `platform.gate-result` | `.forge/gate/<project-id>/evidence.json` | `gate_id` = `<project-id>/<runtime>`, `pipeline` = runtime name, `checks[].check_id` from the sibling's own check ids |
| `platform.readiness` | `forge readiness check|matrix` result | `service` = profile id; one document per selected row |
| `platform.release-evidence` | `.forge/release/<project-id>/<release-id>/state.json` | `evidence_kind` from the stage vocabulary; artifacts/digests only when the stage really produced them |
| `platform.capability` | `.project.json` `capabilities` block when present | Emits nothing when the block is absent — absence is not a declaration. Field mapping is finalized by `governance-vocabulary-consumption` |
| `platform.audit-event` | registry `operations` rows (bounded `--limit`, default 64) | `category` `lifecycle` for mutating kinds, `configuration` for manifest-plane kinds; the actor is recorded as `service` type with the project id as subject, never an invented human identity |

Envelope invariants, enforced at construction rather than trusted:
`contract` matches the published pattern; `generated_at` is RFC3339 UTC;
`payload` validates against the vendored schema; no field name contains a
registered secret substring; no absolute host path survives
(`policy::redact_credentials` plus the existing `<project>` substitution and
char bounds).

## Status mapping: total functions that refuse

`platform.gate-result`:

| Forge `GateAggregate` | contract `result` |
| --- | --- |
| `Passed` | `passed` |
| `Blocked` | `failed` |
| `Failed` | `errored` |
| `Unknown` | `errored` |

`platform.readiness`:

| Forge `ReadinessStatus` | contract `status` |
| --- | --- |
| `Passed` | `ready` |
| `Failed` | `not_ready` |
| `Unverified` | **refuse** (`no evidence is not a readiness state`) |

`platform.audit-event.outcome`:

| journal `state` | contract `outcome` |
| --- | --- |
| `done` | `success` |
| `failed` | `failure` |
| `blocked`, `rejected` | `denied` |
| `pending`, `skipped`, `partial`, `unverified`, anything unknown | **refuse** the row, name it, and keep the rest of the batch |

Rules that make the map safe:

- Mapping is one direction only, Forge → contract. Nothing parses a
  `platform.*` document back into a Forge verdict in this change.
- An unmappable value returns `ContractUnmappable { family, value }`. It never
  defaults, because defaulting `unverified` to `not_ready` or `unknown` to
  `passed` would invert the honest-absence rule this repository already holds
  in `gate` (a contradictory `PASS` on a non-zero exit downgrades to `unknown`
  rather than fabricating a pass) and in doctor (`unverified` is never healthy).
- Where the contract has no slot for a real Forge fact (`partial`,
  `degraded` for a gate check, `skipped` for an unrun release stage), the row
  is refused and named rather than squeezed. The refusal text says which family
  lacks the value, which is the feedback channel `platform-contracts` needs to
  decide whether a minor or major change is warranted.
- Per-check states keep the sibling's meaning: `NOT_APPLICABLE` stays a
  non-passing row, `REVIEW_REQUIRED` never becomes `passed`.

## Secret vocabulary

Two lists exist and they answer different questions:

- `platform-contracts` `schemas/registry.json → secret_field_substrings` is a
  list of **field names** (`token`, `secret`, `apikey`, `bearer`, …) that
  validators reject in a document.
- `policy::redact_credentials` is a set of **value shapes** (AWS/GitHub/GitLab/
  Slack/JWT/private-key/`key=value`) that get replaced in captured text.

This change unions, it does not replace: `redact_credentials` additionally
treats any `key=<value>` whose *key* contains a consumed substring as a secret,
and the contract validator refuses any emitted document carrying a registered
field name. The authoritative list for **field names in shared documents** is
the registry; the authoritative list for **redaction of captured runtime
output** stays in Forge. Both statements land in the module docs so the next
reader does not have to re-derive them. `workspace-governance`'s placeholder
marker vocabulary is explicitly not in scope — `contracts-consumer-parity`
splits those lists on purpose.

## Failure boundaries

| Condition | Behaviour |
| --- | --- |
| `contracts/` missing or `manifest.json` unreadable | typed `contract-invalid`, names the exact path; no fallback to an embedded copy of the schemas |
| A vendored file whose bytes no longer match the manifest | test failure naming file + expected/actual digest; runtime surfaces keep working (they do not re-hash on every call) |
| Family not in the vendored registry | refused by name, listing the supported families |
| Major version unsupported | refused, naming supported majors (consumer rule from `docs/ownership.md`) |
| Unmappable status | refused as above; batch projections drop only the offending row and say so |
| Registered secret field name present in a payload | refused before validation completes; the value is never echoed |
| Document larger than the read bound | refused before parsing |
| Target has no source record (no gate evidence, no release state, no `capabilities`) | `unavailable`/emit-nothing, never a synthesized envelope |
| Parity walk cannot reach the source | the ignored test fails with the next action; default `cargo test` unaffected |

## Compatibility and migration

- Nothing to migrate: no persisted format, no schema column, no manifest key,
  no journal kind changes in this package.
- Every existing Forge document keeps its bytes. Verification includes a
  byte-identity check on `forge gate status . --format json`, `forge check .`,
  `forge fleet list`, `forge provider matrix` and a generated project tree,
  pinning digests the way `tests/workspace_metadata_contract.rs` already pins
  per-profile tree digests.
- `jsonschema` is added under `[dev-dependencies]` only, so the shipped binary
  and its runtime dependency graph do not change.
- Adding `src/contract` to `src/lib.rs` is the only module-list change; the
  modular-monolith rule (transports thin, Core owns rules) is preserved by
  keeping the CLI a renderer.

## Surfaces and deliberate non-surfaces

CLI only. The MCP mature registry gains no tool, `src/api` gains no route, and
`src/portal` gains no section — the same boundary `gate-runtime-evidence` and
`fleet-registry-observation` recorded, because a projection surface enters an
operation-bearing transport only through that transport's own classification
cycle. `forge contract emit` is a read, but it is also a *format authority*, and
putting a format authority behind MCP/API would invite consumers to depend on it
before the vocabulary has a real producer/consumer pair in the portfolio.

## Open decisions (recorded, not silently fixed)

1. **`platform.capability` field mapping waits on governance vocabulary.**
   `capability_id`, `service`, `version`, `declared_at`, `stability` are known
   required fields, but Forge's capability *names* belong to Workspace
   Governance. Emitting mapped names now would re-declare them locally, which is
   the exact anti-pattern this family of changes removes. Interim behaviour:
   emit nothing when `capabilities` is absent; when present, emit with
   `stability` `experimental` and the names quoted verbatim from the
   declaration, marked in the inventory as pending vocabulary consumption.
2. **Revision pinning is a commit, not a config knob.** Pinning through a
   runtime-configurable source URL would turn vendored data into a network
   dependency. If a configurable source is ever wanted, it belongs to the sync
   script only, never to the CLI.
3. **No inbound contract parsing in this package.** Accepting `platform.*`
   documents from other producers (for example driftwatchdog's future
   `gate-evidence-export`) is a separate capability with its own trust rules;
   `gate-evidence-export-consumption` owns it.
4. **`emit` prints, it does not write.** Persisting contract forms alongside
   `.forge/**` records would create a second copy of the same fact with its own
   staleness problem. If a downstream job needs a file, it redirects stdout.

## Verification

- Unit: envelope construction and pattern enforcement; each mapping table row
  plus a refusal case; inventory completeness and agreement tests; manifest
  digest test; secret-name refusal.
- Contract (`tests/platform_contract_contract.rs`): help surface, `list`,
  `inspect` for a known and an unknown family, `emit` per family against
  fixtures, `validate` accepting the vendored canonical fixtures and refusing
  `invalid.*` ones, refusal texts, exit codes, human/JSON parity.
- Cross-surface (`tests/platform_contract_cross_surface.rs`): byte-identity of
  the four existing surfaces listed above; a contract emit leaves the journal
  row count unchanged; MCP `tools/list`, API routes and portal sections gain
  nothing; a credential-shaped value in a source record does not escape into an
  envelope.
- Parity: `scripts/contract-parity.sh` walks the vendored family list against a
  pinned checkout and reports per-family counts; run locally once per sync and
  recorded verbatim in `docs/provider-evidence.md` (its CI job is
  `artifact-and-ci-baseline`'s).
- Live evidence expectation: `forge contract emit gate-result .` against a
  scratch project that really ran the installed sibling gate, and the resulting
  document validated through `forge contract validate -`. If the store is
  uninitialized the row stays honestly `unavailable`, exactly as `forge gate`
  already reports for this repository.
