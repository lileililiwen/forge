# Proposal: represent the `forge cap` facade in the command catalog

## Why

`src/cli/cap.rs` and `CapCommands` (`src/cli/commands.rs:731-770`) ship the
`forge cap` capability-grouping facade with four subcommands. The web command
catalog has **zero** rows for them, so the catalog and the real Clap tree have
drifted apart and the parity oracle fails:

```
test cli::inventory::command_catalog_tests::catalog_has_exactly_one_row_per_clap_path ... FAILED
catalog drifted from the Clap tree: missing=["cap", "cap.add", "cap.inspect", "cap.list", "cap.run"] extra=[]
```

That single failing assertion is the whole of Forge's Gate `tests` and
`declared-verification` failures — every other check passes. `forge gate`
exits 1 with `aggregate: blocked` on no other cause.

The failure is in the catalog, not in the CLI. The facade is correct and
complete; the catalog is behind. The parity contract is "exactly one row per
Clap path", so the fix is five rows, not a weakened assertion.

The surface is also invisible to the operator: `GET /v1/admin/commands` and
the command-reference browser page list every other command in the tree, so
`forge --help` advertises a group the portal cannot explain.

## What Changes

- **Five catalog rows** for `cap`, `cap.add`, `cap.inspect`, `cap.list` and
  `cap.run`, added to the platform row family
  (`src/api/command_catalog/rows_platform.rs`) immediately before the
  terminal `help` row, using the existing `group(...)` + `cli_only(...)`
  builder pair — the same shape as the `agent` group
  (`rows_platform.rs:366-463`). No new builder method, no new enum variant,
  no new dependency.
- **All five rows are `cli_only` with `route: None`.** Verified, not assumed:
  there is no capability-group admin route anywhere in `src/api/` (no
  `ROUTE_ADMIN_*CAP*` constant, nothing capability-group-shaped in
  `routes.rs`, no `cap` handler in `src/api/admin/`), and
  `IMPLEMENTED_WEB_ROUTES` therefore lists none. `plugins list` is web
  (`GET /v1/admin/projects/{id}/plugins`) but it is a *different command*;
  `cap list` composes `plugins::CAPABILITIES` with those records plus the
  capability→command mapping, and that composition has no route.
- **Two honest reasons, matching what each command actually does.**
  `cap list` and `cap.inspect` read `.forge/providers.yaml` from the project
  tree, so they carry the existing `REASON_LOCAL_FS`. `cap add` and `cap run`
  touch nothing — they format a string from their own arguments — so a
  filesystem reason would overstate them; they carry one new family reason,
  `REASON_CAP_PREVIEW`, following the `REASON_AGENT` /
  `REASON_LOCAL_SECRET` precedent of a precise reason for one command family.
- **Risk `read` on every row, including `cap add` and `cap run`.** The facade
  writes nothing (`cap add`'s own output says "writes nothing"; `cap run`'s
  own hint says it "never executes it"). Labelling either as `local_write`
  would claim a mutation that does not happen.
- **Capability tokens follow the surrounding rows.** `cap list` and
  `cap.inspect` carry the existing `local_filesystem` token (they read the
  project tree and stat adapter commands; nothing is probed or spawned).
  `cap add` and `cap run` carry the empty list, like
  `identity generate-password` and the `provider` read rows — they touch no
  provider, registry or filesystem, and inventing a token for that would
  claim a capability no provider declares.
- **Row-count pins updated** from 234 to 239 in
  `src/api/command_catalog/catalog.rs`, plus the two integration-test count
  pins that assert the catalog's total size. The invariant they encode is
  "a web conversion must not add rows"; this change deliberately adds five
  rows, so the pin moves to the new true total. The `web` row allowlist is
  untouched — no row added here is a `web` row.
- **Canonical spec update.** `forge-web-command-catalog` gains a
  "Capability grouping facade coverage" requirement with scenarios for
  parity, for the read-only pair, for the two print-only verbs, and for the
  CLI-only disposition.

## BFS Impact Map

- **Requirements:** `forge-web-command-catalog` — "Exhaustive CLI command
  coverage" already owns the invariant this defect violates (scenario "New
  CLI command added → the coverage check fails and identifies the missing
  command path"); it is **not** modified. One new requirement, "Capability
  grouping facade coverage", is added so the terminal record states the
  facade's truthful per-command disposition.
- **Concepts/vocabulary:** `CommandRow`, `Availability`, `Risk`, `Scope`,
  `Category`, `CatalogBuilder` and every existing reason constant are
  unchanged. New vocabulary: the `cap`, `cap.add`, `cap.inspect`, `cap.list`
  and `cap.run` row ids, and `REASON_CAP_PREVIEW`.
- **Modules:** `src/api/command_catalog/rows_platform.rs` (five rows),
  `src/api/command_catalog/routes.rs` (one reason constant),
  `src/api/command_catalog/catalog.rs` (row-count pin and its delta
  comment). `src/api/command_catalog/builder.rs` and `model.rs` are
  **untouched** — `group` and `cli_only` already express this shape.
- **Contracts/data:** `forge-command-catalog/0.1.0` is unchanged. Row ids
  are additive; nothing is removed or renamed, so no contract migration is
  implied (`routes.rs` CONTRACT_VERSION note). `GET /v1/admin/commands`
  gains five rows in the envelope; no field changes shape.
- **Callers:** `rows()` consumers are the admin catalog handler
  (`GET /v1/admin/commands`), the fleet/portal views that render the
  catalog, `frontend/app.js` command search and reference, and the test
  oracles. Every consumer renders rows generically, so five additive
  `cli_only` rows need no caller change.
- **Persistence:** none. No registry row, no journal row, no evidence file.
  The catalog is static metadata assembled once into a `OnceLock`.
- **Integrations:** none touched. `driftwatchdog`, OpenPanel, Hypora,
  Hermora, DriftWatch and the agent adapters are unaffected; no provider,
  adapter or subprocess is invoked by these rows.
- **Failures:** every existing refusal is preserved. `problems()` continues
  to reject a non-`web` row that names a route, an empty reason, an unknown
  risk/scope/category, a dangling parent or an inconsistent
  `cli_invocation`; the parity test continues to reject a missing or extra
  path. No typed error variant changes.
- **Tests:** `cargo test --lib command_catalog` (integrity, count, web
  allowlist, parent/prefix, execution pins) and `cargo test --lib inventory`
  (the two parity assertions: one-row-per-Clap-path and
  `catalog_integrity_is_clean`). The three integration count pins that read
  the catalog total are updated to 239.
- **Compatibility:** purely additive to the catalog body. No existing id,
  summary, category, scope, risk, availability, route, reason, capability
  token or execution block changes, and no `web` row is added, so the pinned
  web allowlist in `catalog.rs` and
  `tests/forge_web_command_catalog_contract.rs` stays byte-identical.
- **Concerns:** `quality.md` (each row's evidence: source-verified
  behaviour, not an inferred label); `security.md` (the two read rows read
  only the operator's own project tree, honouring `FORGE_PUBLISH_PROVIDER_CONFIG`
  exactly as `forge plugins list` does; the two print rows read nothing and
  never execute what they print).

## Capabilities

- **`forge-web-command-catalog`** (modified): the capability-grouping facade
  is represented once per Clap path with a truthful, per-command
  availability, risk, scope and category, restoring catalog/CLI parity.

## Non-goals

- **Changing the parity assertion.** `catalog_has_exactly_one_row_per_clap_path`
  stays exactly as written; the contract is one row per Clap path and the
  catalog was the side that was wrong.
- **Changing the CLI surface.** No `forge cap` subcommand is added, removed
  or renamed; `src/cli/cap.rs` and `CapCommands` are untouched. The facade
  is correct — the catalog is behind.
- **Adding a web surface for `cap`.** No admin route, no handler, no
  frontend card, no SPA view. All five rows stay `cli_only` with a
  plain-language reason and next step.
- **Re-labelling the facade's own behaviour.** `cap add` and `cap run` keep
  printing; this change does not make either execute or write, and does not
  describe either as if it did.
- **Touching `src/cli/inventory.rs`, `.ai-gate/gate.yaml` or `src/gate/`.**
  The per-check timeout fix (`ccb3b0b`) is landed and verified.
- **A second durable plan.** `proposal.md`, `design.md`, `tasks.md` and the
  canonical `specs/` delta are the only plan of record.
- **Phase:** closing a catalog-parity defect. No publish, no push, no
  release.