# Design: Source file size remediation

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge` crate
  (library + binary). No sibling touched.
- **Modules changed in this change's first slice:**
  - `tests/kit_contract.rs` (2376 lines) → `tests/kit_contract/`
    (directory with `main.rs` + eight focused submodules)
- **Modules reused unchanged:** every other `src/` and `tests/`
  file. The crate's `mod ...;` lines do not change; only the
  filesystem shape of one test target flips from a single
  `*.rs` file to a `*.rs/` directory.
- **Must NOT change:** any public symbol, journal column, route,
  CLI argument, catalog row, env var, `--version` output, or test
  name. The 60 `#[test]` functions in `tests/kit_contract.rs`
  (one of which is `#[ignore]`) move to submodules with their
  bodies verbatim.

## 2. Language and runtime

- Rust 2021, `rustc 1.87` floor. Build: `cargo build`. Test:
  `cargo test --test <name>` for each affected target plus the
  same full pre-archive suite the prior changes run.
- Linux; no platform-specific code added or moved.

## 3. Ownership and shared code

The one affected test target follows one pattern:

```
tests/<name>.rs        →  tests/<name>/
                          ├── main.rs        (helpers, `mod` decls, file doc)
                          ├── <area_1>.rs    (one group of `#[test]`)
                          ├── <area_2>.rs
                          └── ...
```

`main.rs` keeps the file-level doc comment, every shared `fn ...`
helper, and the `mod <area_n>;` declarations. Each submodule
contains the related `#[test]` functions plus the helper(s) used
only inside that submodule (e.g. `has_literal_hex_colour` for
`tokens_and_assets`, `tree_snapshot` for `tampering`).
Cargo discovers the test target the same way (`--test <name>`);
every test still runs identically. Total test count per target
is unchanged. The only mechanical change is the wrapping
`pub(crate) fn` visibility (a Rust integration test crate is its
own root, so `pub(super)` would refer above the crate root) and
the `mod <area>;` declarations in `main.rs`.

For `tests/kit_contract/`:

| Submodule | Concern | Tests |
|---|---|---|
| `main.rs` | file doc, shared helpers, mod declarations, `FeedRestore` RAII | — |
| `registration.rs` | kit registration, declared-zero, floor exception | 14 |
| `feed.rs` | feed pinning, source-mode refusal, CLI feed validation | 13 |
| `prewiring.rs` | pre-wired consumption for the .NET profile | 4 |
| `tokens_and_assets.rs` | vendored tokens, ownership receipts, deterministic rendering | 8 |
| `tampering.rs` | tampered-asset detection, source-mode reference detection, `forge kit pack` | 4 (`packing_leaves_the_sibling_checkout_byte_identical` is `#[ignore]`) |
| `classification.rs` | profile-by-profile classification, reason-string discipline | 4 |
| `upgrade.rs` | explicit confirmation, version pin reconciliation, registry observation | 8 |
| `manifest.rs` | generation, manifest parsing, Forge-absent operation | 5 |

## 4. User experience and interface

`UI/UX: N/A`. No CLI change, no API change, no frontend change,
no journal change, no env change. The only observable difference
is the file-vs-directory shape on disk, which the operator never
sees.

## 5. Behavioral model

None — this is a pure move. Each function body is copied
verbatim. The only mechanical change is the wrapping `pub mod`
or `mod` declaration in `mod.rs`, and the
`use super::helpers;` / `use super::*;` imports the submodules
need to reach shared helpers.

## 6. Contract and compatibility

- No new or modified public symbol. `cargo doc` output, the
  `forge --help` output, every `forge <verb> --help`, every
  `GET /v1/admin/...` envelope, every catalog row, every
  share/publication record, every registry/journal row, every
  CLI exit code — all byte-identical.
- No `cargo test` regression: every `cargo test --test <name>`
  target that was green before stays green, with the same
  pass count.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| A `mod.rs` re-exports a private helper incorrectly | `cargo test` fails compile; fixed before commit |
| A submodule imports a name the `mod.rs` did not export | `cargo test` fails compile; fixed before commit |
| The split changes the run order of `#[test]` functions | `cargo test` re-runs deterministically (parallel by default); no assertion relies on order |
| The split crosses the 1000-line cap again | The submodule is itself split, or a follow-on change picks it up; this change never claims a file under the cap if it isn't |

## 8. Verification oracle

- **The affected target:** `cargo test --test kit_contract` runs
  the same 60 tests as before, 59 green and 1 `#[ignore]`
  (the same `packing_leaves_the_sibling_checkout_byte_identical`
  the original had marked `#[ignore]`).
- **Catalog and gate stay green:** `cargo test --bin forge`,
  `cargo test --test forge_web_command_catalog_contract`,
  `cargo test --test forge_web_project_management_contract`,
  `cargo test --test forge_admin_api_contract`,
  `cargo test --test forge_web_fleet_contract`, and the
  workspace `source-file-size` count drops by exactly one file
  (the converted `tests/kit_contract/` directory's individual
  files all under 1000 lines).
- **Full pre-archive suite:** the same set the prior changes
  run (workspace `cargo test`, `cargo fmt --check`,
  `node scripts/check-openspec-change-names.mjs`,
  `openspec validate --all --strict --no-interactive`,
  `git diff --check`, `forge gate`).
- A task box is checked only with the command output for its
  assertion.

## 9. Follow-on roadmap (one file per future change)

The remaining oversized files, in priority order. Each one is a
single, well-bounded, future OpenSpec change. None of these is
part of this change; they are listed here so the next
implementer can pick one up without re-deciding the strategy.

| File | Lines | Natural split axis | Status |
|---|---|---|---|
| `tests/portal_ui_contract.rs` | 1,431 | Three submodules: list/detail/publish, sign-in/identity, accessible/responsive shell; main.rs holds shared helpers, constants, and assertion utilities | follow-on |
| `tests/portfolio_contract.rs` | 1,261 | CLI vs HTTP tests; helpers + helpers/seed into main.rs | follow-on |
| `tests/supervised_agent_contract.rs` | 1,201 | Per-provider scenario groups (legacy session, ariadex, sisyphusfy, native toolchain) | follow-on |
| `tests/gate_contract.rs` | 1,157 | Documented-help / passing / blocked / review-required / unknown-runtime scenarios | follow-on |
| `src/main.rs` | 14,619 | Per top-level subcommand family (`cli/import.rs`, `cli/workspace.rs`, `cli/identity.rs`, …) | follow-on |
| `src/api/mod.rs` | 5,410 | Per route family (`routes/admin.rs`, `routes/portal.rs`, `routes/delivery.rs`, …) | follow-on |
| `src/doctor/mod.rs` | 3,030 | Per concern (`gaps`, `maturity`, `output`, `report`) | follow-on |
| `src/api/command_catalog.rs` | 2,852 | Per row family + integrity/parity test module | follow-on |
| `src/api/admin.rs` | 2,589 | Per verb family (already split once by `forge-web-workspace-onboarding`) | follow-on |
| `src/provider/mod.rs` | 2,355 | Per provider concern (registry, selection, invocation, errors) | follow-on |
| `src/release/engine.rs` | 2,258 | Per stage | follow-on |
| `src/identity/mod.rs` | 2,207 | Per concern (global admin, project OIDC, session store, claims) | follow-on |
| `src/agent/mod.rs` | 2,204 | Per supervisor / primitive / contract | follow-on |
| `src/portal/mod.rs` | 2,191 | Per surface (HTML render, asset serve, route family) | follow-on |
| `src/mcp/mod.rs` | 2,157 | Per MCP method family + capability tables | follow-on |
| `src/ui_pattern/mod.rs` | 2,137 | Per pattern family | follow-on |
| `src/generate/mod.rs` | 1,998 | Per scaffold profile | follow-on |
| `src/policy/mod.rs` | 1,914 | Per rule family | follow-on |
| `src/publish/remote_compose.rs` | 1,913 | Per compose step | follow-on |
| `src/docs/mod.rs` | 1,911 | Per surface (render, translation, identity) | follow-on |
| `src/analytics/mod.rs` | 1,789 | Per projection family | follow-on |
| `src/procedure/mod.rs` | 1,718 | Per procedure | follow-on |
| `src/component/mod.rs` | 1,672 | Per component family | follow-on |
| `src/registry/portfolio.rs` | 1,669 | Per portfolio concern (tags, relations, reviews, goals, evidence) | follow-on |
| `src/gate/mod.rs` | 1,530 | Per gate concern (plan, document, run, evidence) | follow-on |
| `src/planner/mod.rs` | 1,527 | Per planner concern (intent, plan, apply) | follow-on |
| `src/distribution/mod.rs` | 1,517 | Per surface (channel, manifest, publish) | follow-on |
| `src/import/mod.rs` | 1,469 | Per import concern (adopt, inspect, derive, sync) | follow-on |
| `src/registry/mod.rs` | 1,443 | Per registry concern (projects, journal, operations, share audit) | follow-on |
| `src/governance.rs` | 1,436 | Per governance concern (rules, adapters, providers) | follow-on |
| `src/publish/providers.rs` | 1,393 | Per provider family | follow-on |
| `src/deploy/engine.rs` | 1,388 | Per stage | follow-on |
| `src/feature/mod.rs` | 1,365 | Per feature concern (add, remove, upgrade, manifest edit) | follow-on |
| `src/profile/mod.rs` | 1,363 | Per profile concern (registry, render, verify) | follow-on |
| `src/spec/mod.rs` | 1,339 | Per spec concern (generate, apply, validate) | follow-on |
| `src/api/ui/routes.rs` | 1,335 | Per route family | follow-on |
| `src/fleet/mod.rs` | 1,327 | Per fleet source (registry, inventory, workspace, publish) | follow-on |
| `src/upgrade/mod.rs` | 1,300 | Per upgrade stage | follow-on |
| `src/github/adapter.rs` | 1,291 | Per adapter operation | follow-on |
| `src/standard/mod.rs` | 1,237 | Per standard concern (pack, registry, snapshot) | follow-on |
| `src/publish/mod.rs` | 1,230 | Per publish concern (compose, queue, inventory, providers) | follow-on |
| `src/doctor/gaps.rs` | 1,200 | Per gap family | follow-on |
| `src/api/fleet.rs` | 1,180 | Per source adapter | follow-on |
| `src/semantic/review.rs` | 1,134 | Per review concern | follow-on |
| `src/kit/assets.rs` | 1,122 | Per asset concern | follow-on |
| `src/deploy/mod.rs` | 1,121 | Per deploy concern (engine, adapter, manifest) | follow-on |
| `src/release/mod.rs` | 1,112 | Per release concern (engine, tag, notes) | follow-on |
| `src/github/cli.rs` | 1,099 | Per `gh` subcommand | follow-on |
| `src/publish/inventory.rs` | 1,090 | Per inventory concern | follow-on |
| `src/portfolio/interest/activation.rs` | 1,054 | Per activation stage | follow-on |
| `src/registry/interest/mod.rs` | 1,047 | Per interest concern | follow-on |
| `src/api/ui/render.rs` | 1,043 | Per render concern (chrome, sign-in, project, dashboard) | follow-on |
| `src/portfolio/mod.rs` | 1,019 | Per portfolio concern (metadata, share, interest) | follow-on |

## 10. Decision ledger

- **Resolved:** convert test files to directories in this change's
  first slice. Test files have no `pub` boundary, so the move is
  a pure refactor and the `cargo test --test <name>` run is the
  per-file oracle.
- **Resolved:** leave the 40+ source files for follow-on
  single-file packages. Mixing them in this change would exceed
  the workflow's "one change → local verify → strict validate →
  archive" budget and obscure the per-file decision.
- **Resolved:** keep every function body verbatim. No body
  change; no "while I'm here" cleanup. The change's only
  responsibility is the file→directory move.
- **Resolved:** do not change the test count of any target. Every
  test runs once, in the same target, with the same name.
- **Blockers:** none.
