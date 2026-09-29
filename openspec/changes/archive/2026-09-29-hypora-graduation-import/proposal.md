# Proposal: Import validated ideas from Hypora

## Why

Forge can adopt an **existing repository** (`forge import`) or generate
a new one from a pinned profile (`forge new`), but it has no way to
accept a *validated idea* — a brief, its requirements and the aggregate
evidence that the idea was tested — from the product that validated it.
Today that brief is re-typed by hand into a project, so the provenance
of the decision is lost and the evidence is invisible to Forge.

Hypora validates ideas and produces a privacy-bounded graduation
artifact. Forge should be able to read one operator-selected local
artifact and seed a new Forge project from it, **without** becoming a
Hypora runtime dependency, without holding a Hypora credential, and
without treating a validated idea as permission to scaffold, deploy,
or approve anything.

The change is grounded in a read of the current tree: `platform.idea-
graduation` is **not** vendored in this repository, `AppSpec` does not
exist, and `forge import` already establishes the "propose, then
accept" pattern this package reuses. The design states the pinned
contract family/major/revision and the local conformance fixtures; when
the producer publishes the schema, the vendoring package that pulls it
in updates one constant and one fixture.

## What Changes

1. A new `src/graduation/` module that owns the accepted
   `platform.idea-graduation` contract family, major and revision
   allowlist, the closed key set at every level of the artifact, the
   identity/raw-event/payment/credential deny lists, the field bounds,
   and a `GraduationRefusal` vocabulary.
2. One gate, `validate_graduation`, that every transport goes through:
   bounded read → document decode → closed-key/deny/bounds/value check →
   mapped `GraduationImport`. A refusal never echoes the offending
   value.
3. `forge graduation preview <ARTIFACT>`: a read-only projection of the
   mapped brief, the source provenance and the aggregate evidence,
   which writes nothing.
4. `forge graduation import <ARTIFACT> --path <DIR> --profile
   <PROFILE> [--id <ID>] [--actor <ACTOR>] [--confirm]`: without
   `--confirm` it is the same read-only preview with the resolved
   identity and location; with `--confirm` it writes a minimal
   `forge.yaml` and a `.forge/graduation/<id>/import.json` receipt, then
   registers the project, rolling the files back if registration fails.
5. An explicit persistence allowlist: the receipt carries the mapped
   brief, the source block, the evidence **count**, the import time and
   the actor — never the original artifact, never an evidence excerpt,
   never a field from outside the closed set.
6. Two additive, typed errors (`graduation-invalid`,
   `graduation-conflict`) and no other Core change.
7. Explicit non-goals: no live Hypora call, no credential exchange, no
   scaffold, no deploy, no publish, no gate approval, no new manifest
   field, no registry table or migration, no API route, no MCP tool.

## BFS Impact Map

| Surface | Impact |
|---|---|
| Module | New `src/graduation/{mod,validation,import}.rs`; `src/lib.rs` gains `pub mod graduation;` |
| Contract | Forge emits `forge-graduation-import/0.1.0`; accepts `platform.idea-graduation/0.1.0` (major `0`, revision allowlist) |
| Validation | Closed key sets at artifact/brief/metric/experiment/evidence levels; four deny lists; per-field bounds; credential/email/query-URL value refusal; `validated == true` required |
| CLI | New `forge graduation preview|import`; `import` requires `--path` and `--profile`; `--confirm` gates the write |
| Persistence | One `.forge/graduation/<id>/import.json` sidecar per imported project; one registry row via the existing `register` |
| Errors | Two additive variants: `GraduationInvalid` → `graduation-invalid`, `GraduationConflict` → `graduation-conflict`; unknown profile reuses `unknown-profile`, bad path reuses `path-unavailable` |
| Authorization | Local CLI only; no session, no route, no tool |
| Network / side effects | None: no HTTP client, no `gh`, no `git` invocation; only the destination directory and the registry are touched |
| Scaffold / deploy / gate | None: the chosen `--profile` is recorded in the manifest; applying it stays an explicit `forge new`/generation action |
| Unaffected | `forge import`, the manifest schema, the registry schema, the API route table, the portal, the MCP tool list, the share package, generation/publish |
| External dependency | The producer schema is not in-repo; the accepted revision constant plus the local fixtures are the oracle until the vendoring package lands |

## Capabilities

### New Capabilities

- `hypora-graduation-import`: a local, privacy-bounded, confirmed import
  of a `platform.idea-graduation` artifact into a new Forge project,
  with a mapped brief and an allowlisted provenance receipt.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `hypora-graduation-import` (this) | An operator can turn one validated graduation artifact into one Forge project without Hypora being a runtime dependency | Forge, Rust/SQLite | `forge-graduation-import/0.1.0` receipt; `platform.idea-graduation/0.1.0` input | profile registry, core manifest, registry | CLI + cross-surface contract suites |
| Hypora producer change | Hypora emits a privacy-bounded `platform.idea-graduation` artifact | Hypora (external) | Producer schema | Hypora's experiment store | Hypora's own tests |
| `platform-contracts` vendoring | The producer schema is digest-pinned under `contracts/` | platform-contracts (external) + a future Forge sync package | Vendored schema + digest manifest | the producer schema | The existing `platform-contract-consumption` digest tests |
| Scaffolding a chosen profile | A profile template is applied to the new project | Forge, existing generation | `forge new` / generation | this package's project | Existing generate contract tests |

End-to-end Hypora→Forge adoption waits for the producer change and the
vendoring package; this package's own contract is verifiable entirely
with local fixtures, which is why it can be implemented and verified
now.

## Non-goals

- No live Hypora API call, no polling, no shared session and no
  credential exchange.
- No scaffold, generate, deploy, publish, mirror or release side effect,
  and no quality-gate approval.
- No `gh` invocation and no Git remote mutation.
- No new `forge.yaml` field, no registry table, no registry migration.
- No API route, no MCP tool and no portal control.
- No invented `AppSpec` type: the mapped brief lives in the receipt, and
  the project manifest remains the identity source of truth.
- No inference of a stack, template or destination from the artifact.
- No retention of the original artifact bytes or of any evidence
  excerpt.
