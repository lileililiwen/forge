# Tasks: represent the `forge cap` facade in the command catalog

## Phase 1 — BFS baseline

- [x] 1.1 `node scripts/check-openspec-change-names.mjs` PASS before
      selection; `openspec list` reports no active change.
- [x] 1.2 Read `AGENTS.md`, `.ai-rules/workflow.md`,
      `.ai-rules/completion.md` and the canonical spec
      `forge-web-command-catalog`.
- [x] 1.3 Reproduce the defect from the assertion itself: the Clap tree has
      `cap`, `cap.add`, `cap.inspect`, `cap.list`, `cap.run`; the catalog has
      `missing=[…] extra=[]` — pure under-coverage, no over-coverage.
- [x] 1.4 Read the facade's actual behaviour end to end:
      `src/cli/cap.rs` (all four arms, `cap_group_commands`, `CAP_CONTRACT`)
      and `CapCommands` in `src/cli/commands.rs:731-770`.
- [x] 1.5 Read the catalog contract: `catalog.rs` (`rows`, `problems`,
      `envelope`), `builder.rs` (all six row helpers), `model.rs` (the
      `Availability`/`Risk`/`Scope`/`Category` vocabularies) and `routes.rs`
      (`IMPLEMENTED_WEB_ROUTES` + every `REASON_*`).
- [x] 1.6 Establish the builder shape from the nearest precedent: the `agent`
      group at `rows_platform.rs:366-463` (`group` + `cli_only`).
- [x] 1.7 Verify the `route: None` judgement instead of assuming it: no
      `ROUTE_ADMIN_*CAP*` constant anywhere in `src/`, no capability-group
      handler in `src/api/admin/`, no `cap` entry in `routes.rs`, and
      `rg 'cap_group_commands|CAP_CONTRACT'` matching only `src/cli/cap.rs`.
      Record the false lead: `plugins list` **is** web
      (`GET /v1/admin/projects/{id}/plugins`) but is a different command —
      `cap list` is a composition over it, not that route.
- [x] 1.8 Confirm `plugins::list` (`src/plugins/mod.rs:244-258`) is a pure
      fold — no spawn, no probe — so no row needs `external_provider`.
- [x] 1.9 Map every assertion that counts catalog rows and decide each:
      `catalog.rs:205` (234), `tests/web_release_deploy_history_contract.rs:536`
      (234), `tests/web_agent_identity_readonly_contract.rs:517` (234),
      `tests/web_command_reference_browser_contract.rs:264` (prose in a
      message, not a pin). Record that
      `tests/forge_web_command_catalog_contract.rs` needs no edit: its
      `TOP_LEVEL_COMMANDS` subset never listed `cap` and no `cap` row is `web`.
- [x] 1.10 Record the scope boundaries in design §8: `src/cli/inventory.rs`,
      `src/cli/cap.rs`, `src/cli/commands.rs`, `.ai-gate/gate.yaml` and
      `src/gate/**` stay untouched; no dependency is added.
- [x] 1.11 Set `current_spec: command-catalog-capability-group` in HANDOFF.

## Phase 2 — DFS requirement implementation

- [x] 2.1 `src/api/command_catalog/routes.rs`: add `REASON_CAP_PREVIEW`
      beside the other family reasons, naming the print-only behaviour and
      the next step, and stating that neither verb executes nor writes.
- [x] 2.2 `src/api/command_catalog/rows_platform.rs`: add the `cap` group row
      with `group(None, "cap", …, Reference, Forge)` at the end of
      `platform_rows()`, after the `agent` rows and before `help`.
- [x] 2.3 Add `cap.list` and `cap.inspect` with `cli_only(… Reference,
      Project, Read, REASON_LOCAL_FS, caps_local)` — the existing
      `local_filesystem` token, since both read `<project>/.forge/providers.yaml`
      and stat adapter commands.
- [x] 2.4 Add `cap.add` and `cap.run` with `cli_only(… Reference, Forge,
      Read, REASON_CAP_PREVIEW, none)` — `read` risk, empty capability list,
      because each only formats a string from its own arguments.
- [x] 2.5 `src/api/command_catalog/catalog.rs`: move the
      `catalog_covers_every_clap_path` total 234 → 239 and extend its delta
      comment to name the `cap` paths. Leave the web vector, the execution
      pin and every assertion in that file untouched.
- [x] 2.6 Update the two integration-test row-count pins to 239
      (`tests/web_release_deploy_history_contract.rs`,
      `tests/web_agent_identity_readonly_contract.rs`), keeping each one's
      "conversion adds no rows" intent intact.
- [x] 2.7 No change to `builder.rs`, `model.rs`, `src/cli/inventory.rs`,
      `src/cli/cap.rs`, `src/cli/commands.rs`, `src/api/` routes or
      handlers, the frontend, or `.ai-gate/gate.yaml`.

## Phase 3 — BFS regression/completeness

- [x] 3.1 `cargo test --lib command_catalog`: count pin 239, `problems()`
      empty, `web_rows_only_point_at_implemented_routes` vector unchanged,
      parent/prefix rule green, execution-block pins green.
- [x] 3.2 `cargo test --lib inventory`:
      `catalog_has_exactly_one_row_per_clap_path` and
      `catalog_integrity_is_clean` both pass — the parity assertion is
      unmodified.
- [x] 3.3 Run the three row-count assertions this change moved, so the new
      total is evidenced rather than assumed.
- [x] 3.4 `git diff -- src/cli/cap.rs src/cli/commands.rs .ai-gate/gate.yaml`
      is empty: the CLI surface and the Gate configuration are untouched.
- [x] 3.5 `git diff --check` clean, including newly added files; the
      pre-existing rustfmt drift in
      `tests/web_assurance_browser_contract.rs` is untouched and unstaged.
- [x] 3.6 Review the parity contract from the other side: no row was removed
      or renamed, so every previously pinned id, summary, category, scope,
      risk, availability, route, reason, capability token and execution block
      is byte-identical.

## Phase 4 — Verification

- [x] 4.1 `cargo build`.
- [x] 4.2 `cargo fmt --check` clean for every touched file.
- [x] 4.3 `cargo clippy` (default targets) — no new finding.
- [x] 4.4 `cargo test --lib command_catalog`; `cargo test --lib inventory`.
- [x] 4.5 `node scripts/check-openspec-change-names.mjs` PASS.
- [x] 4.6 `node scripts/check-spec-governance.mjs` PASS.
- [x] 4.7 `openspec validate --all --strict --no-interactive` PASS.
- [x] 4.8 `git diff --check` clean; staged diff reviewed path by path.
- [x] 4.9 Archive `command-catalog-capability-group` **without**
      `--skip-specs`; commit 1 = implementation + specs + archive, explicit
      paths only; commit 2 = HANDOFF evidence and pointer removal. No push.
- [x] 4.10 The heavy Gate runtime (`forge gate --dry-run` + bounded
      `forge gate`) is the orchestrator's step for this cycle; the HANDOFF
      evidence table records that delegation explicitly rather than claiming
      a verdict.