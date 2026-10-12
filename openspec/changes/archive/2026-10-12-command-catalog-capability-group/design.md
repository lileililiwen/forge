# Design: represent the `forge cap` facade in the command catalog

## 1. The defect, precisely

`src/cli/cap.rs` defines the facade and `CapCommands` (`src/cli/commands.rs:731-770`)
declares four subcommands. `src/main.rs:477-482` registers them under `Commands::Cap`.
The catalog (`src/api/command_catalog/`) contains no `cap` row, so
`src/cli/inventory.rs::command_catalog_tests::catalog_has_exactly_one_row_per_clap_path`
walks the live Clap tree, finds five paths with no catalog row, and fails:

```
missing=["cap", "cap.add", "cap.inspect", "cap.list", "cap.run"] extra=[]
```

`extra=[]` is the load-bearing half of that message: the catalog has nothing
the CLI does not have. This is pure under-coverage. The invariant is
"exactly one row per Clap path" and it stays exactly as written.

## 2. Shape: which existing builder pair

`group(...)` + `cli_only(...)`, the same pair the `agent` group uses
(`rows_platform.rs:366-463`). Rationale:

- `group(parent, name, summary, category, scope)` exists precisely for a
  top-level Clap subcommand that only organizes leaves. It hardcodes
  `Risk::Read`, `REASON_GROUP` and an empty capability list — the correct
  disposition for a group row, and the disposition every one of the ~30
  existing group rows already carries.
- `cli_only(parent, name, summary, category, scope, risk, reason, capabilities)`
  covers the four leaves: a per-row `risk`, a family `reason`, and the
  capability tokens.

Rejected alternatives:

- `leaf(...)` — sets `availability = NotYetWeb` and the reason
  `REASON_NOT_YET_WEB`, whose text claims "a web workflow for this command is
  planned in the staged web packages (workbench, portfolio controls, delivery
  controls) but not implemented yet". No package plans a `cap` web surface.
  That would be a false claim about intent, so `leaf` is not used.
- `web_at(...)` / `web(...)` / `web_exec(...)` — all three require a route
  that resolves into `IMPLEMENTED_WEB_ROUTES`. None exists (§3).
- `provider_required(...)` — would claim `cap list`/`cap.inspect` need a live
  external provider. They never contact one: `plugins::list` is a pure fold
  over the parsed provider config plus descriptors (`src/plugins/mod.rs:244-258`).
- A new builder method or enum variant — the existing six methods express
  every disposition here. Adding one would widen the vocabulary the frontend
  and `problems()` both reason over for no new state.

## 3. `route: None` — verified, not assumed

`route` is not a judgement call; it is a lookup. Four independent checks:

1. **No capability-group route exists.** `rg 'ROUTE_ADMIN_[A-Z_]*CAP'` over
   `src/` returns nothing. `src/api/admin/` has `creation.rs`,
   `shipping_reads.rs`, `assurance.rs`, `history.rs`, `agent_identity.rs` —
   no capability-group handler.
2. **Nothing in `routes.rs` names one.** `IMPLEMENTED_WEB_ROUTES` is the
   closed list a `web` row may point at; it contains no `cap` entry.
3. **No API or frontend consumer of the facade exists.**
   `rg 'cap_group_commands|CAP_CONTRACT'` matches only `src/cli/cap.rs`. The
   facade is reachable from the CLI and nowhere else.
4. **The nearest-looking neighbour is a different command.** `plugins list`
   *is* web (`GET /v1/admin/projects/{id}/plugins`,
   `WEB_ROUTE_ADMIN_SHIPPING_PLUGINS`). `cap list` is not that route: it
   composes `plugins::CAPABILITIES` × live `Ready` plugin records ×
   `cap_group_commands` (`src/cli/cap.rs:52-95`). Pointing a `cap` row at the
   plugins route would tell the operator the browser renders a capability
   group when it renders a plugin list.

Therefore `route: None` and `availability: cli_only` on all five rows, which
also satisfies `problems()`' "a non-`web` row must not name a route" rule.

## 4. The five rows

Placed at the end of `platform_rows()`, between the `agent` rows and the
terminal `help` row: `cap` is the last top-level subcommand in the Clap tree,
and `help` already closes the family.

| id | summary (from `CapCommands`) | category | scope | risk | availability | route | reason | capabilities |
|---|---|---|---|---|---|---|---|---|
| `cap` | Group flat commands by capability (gate, agent, contract, delivery); old flat commands stay working aliases and this facade only lists, inspects and previews the same surface. | `reference` | `forge` | `read` | `cli_only` | `None` | `REASON_GROUP` | `[]` |
| `cap.list` | List every capability group with live Ready plugin counts. | `reference` | `project` | `read` | `cli_only` | `None` | `REASON_LOCAL_FS` | `["local_filesystem"]` |
| `cap.inspect` | Inspect one capability group: Ready plugins plus mapped flat commands. | `reference` | `project` | `read` | `cli_only` | `None` | `REASON_LOCAL_FS` | `["local_filesystem"]` |
| `cap.add` | Print the descriptor snippet for adding a plugin to a group; writes nothing. | `reference` | `forge` | `read` | `cli_only` | `None` | `REASON_CAP_PREVIEW` | `[]` |
| `cap.run` | Print the underlying flat `forge …` command for a group action; never executes it. | `reference` | `forge` | `read` | `cli_only` | `None` | `REASON_CAP_PREVIEW` | `[]` |

The `cap.list` / `cap.inspect` / `cap.add` / `cap.run` summaries begin with the
authoritative `CapCommands` doc comment verbatim; the `cap.add` and `cap.run`
summaries keep the facade's own printed guarantees ("writes nothing" /
"never executes it") because those guarantees are what makes their risk and
availability honest.

## 5. Per-row judgement

### 5.1 `risk`

| row | observed behaviour | risk | why |
|---|---|---|---|
| `cap.list` | `live_records` → `plugins::resolve_config_path` + `providers::load_config` + `plugins::load_descriptors` + `plugins::list`, then counts `Ready && enabled` records per capability | `read` | Reads and parses two config files and folds them; writes nothing, contacts nothing. |
| `cap.inspect` | same load, then filters the records for one capability | `read` | Same shape, smaller result. |
| `cap.add` | lowercases and validates `capability` against `plugins::CAPABILITIES`, then `format!`s a YAML snippet from `id` and `cap` | `read` | Pure string construction. The command's own output ends "`cap add` writes nothing". `local_write` would assert a mutation that cannot occur. |
| `cap.run` | lowercases and validates `capability` against `cap_group_commands`, then `format!`s `forge {primary} {action}` | `read` | Pure string construction. The printed hint ends "run the flat command directly — `cap run` never executes it". `local_write` would assert a mutation; `remote_write` would assert a remote effect. Neither happens. |

The task framing "cap add and cap run are the actionable ones" is honoured as
*category/intent*, not as *risk*: they are the two verbs an operator reaches
for when they want to **do** something with a capability group, and that is
why they get the print-specific reason (§5.4) rather than the read reason. The
risk vocabulary describes what the command does to state, and neither verb
touches state.

### 5.2 `availability`

All five are `cli_only`. `cli_only` means "this command's work is not exposed
by an implemented typed JSON route, here is why, here is the next step", which
is exactly true (§3). The alternatives are worse:

- `web` — forbidden; no route exists, and `problems()` would reject it twice
  (unimplemented route / no route).
- `not_yet_web` — its reason text claims a planned staged web workflow. There
  is none. Using it would convert a fixed defect into a tracked lie.
- `disabled` — claims the command cannot run. All four run today.
- `provider_required` / `project_capability_required` — both claim a
  prerequisite the command does not have. `cap run` needs no plugin at all;
  `cap add` needs no provider.

### 5.3 `scope`

| row | scope | why |
|---|---|---|
| `cap.list`, `cap.inspect` | `project` | Both take `--project` (default `.`) and read `<project>/.forge/providers.yaml`. The value is one project's configured plugins. |
| `cap.add`, `cap.run` | `forge` | Neither takes a path, neither reads a project. `cap add` consults `plugins::CAPABILITIES` and `cap run` consults `cap_group_commands`, both Forge-owned constant tables compiled into the binary. `Scope::Forge` is already used for exactly this class of row (`identity status`, `identity generate-password`, `help`, `portal dashboard`). |

Mixed scope inside one group is established precedent: the `identity` group
carries `Forge` rows (`setup`, `change-password`, `generate-password`,
`status`) and `Project` rows (`validate-config`, `session-list`, …) side by
side. Forcing all four leaves to `project` would claim a project dependency
the two print rows do not have.

### 5.4 `reason`

- `cap.list`, `cap.inspect` → existing `REASON_LOCAL_FS`: *"This command reads
  or writes the local project tree directly; a browser cannot safely touch the
  filesystem and no typed JSON route for it exists yet. Next step: run it in a
  terminal inside the project."* Verbatim accurate: they read the project tree
  (and honour `FORGE_PUBLISH_PROVIDER_CONFIG` exactly as `forge plugins list`
  does), and no typed route exists for the composition.
- `cap.add`, `cap.run` → new `REASON_CAP_PREVIEW`: *"This command only prints
  the flat `forge …` command, or the descriptor snippet, that the capability
  group maps to; it never executes it and never writes the manifest. There is
  nothing for a browser route to perform. Next step: run it in a terminal,
  then run the flat command it names."*

`REASON_LOCAL_FS` was rejected for the two print rows because its text claims
a local-tree read or write that neither performs. `REASON_GROUP` was rejected
because its text addresses the group row's disposition ("command groups only
organize their subcommands") and would be attached to a leaf.

One precise family reason follows the existing precedent exactly:
`identity generate-password` prints OS entropy and takes `REASON_LOCAL_SECRET`
rather than borrowing a neighbouring family reason; `REASON_AGENT` and
`REASON_LEGACY_HTML` do the same for their families.

### 5.5 `category`

All five rows are `reference`. `cap` is by construction a discovery facade: the
module doc says it is a "thin grouping over the existing flat commands" that
"only lists, inspects, and previews the same surface", and `main.rs`'s own
command doc says the same. `reference` is where the catalog already keeps the
CLI's own context surface (`help`), so a browser user looking for "how do I do
X" finds `cap` next to `help` instead of misfiled under quality or delivery.

Rejected: `registry` — `cap list`/`cap.inspect` read the project's
`providers.yaml`, not the project registry, and `cap add`/`cap run` read
neither; labelling the group `registry` would file a command surface as
project bookkeeping. `quality` — only two of the seven capability groups
(gate, quality) are quality-shaped; the facade spans creation, delivery and
release too. Mixed per-row categories were rejected because `cap` is one
facade and splitting it across four categories would hide the facade in the
frontend's category filter.

### 5.6 `capabilities`

- `cap.list`, `cap.inspect` → `["local_filesystem"]`, the token already used by
  the local-tree read rows (`identity status`, `identity session-validate`,
  `identity validate-config`, `studio spec`). They read `.forge/providers.yaml`
  and stat adapter commands. They do **not** need `external_provider`:
  `plugins::list` (`src/plugins/mod.rs:244-258`) is a pure fold over the
  parsed config and descriptors, spawning nothing and probing nothing.
- `cap.add`, `cap.run` → `[]`, the same empty list `group()` assigns every
  group row and that `identity generate-password`, `provider matrix` and
  `provider inspect` carry. No filesystem, registry or provider is touched, so
  no token is honest; inventing one (a `capability_group` or `cap_preview`
  token) would claim a capability no provider declares and that no provider
  filter would ever match.

## 6. Integrity invariants, row by row

`problems()` (`src/api/command_catalog/catalog.rs:19-106`) and the two parity
tests. "✓" means the row satisfies the rule by construction, not by luck.

| invariant | `cap` | `cap.list` | `cap.inspect` | `cap.add` | `cap.run` |
|---|---|---|---|---|---|
| unique id (first `seen.insert` wins) | ✓ new | ✓ new | ✓ new | ✓ new | ✓ new |
| `parent_id` names a row that exists (`cap` is pushed first) | ✓ `None`, id has no dot | ✓ | ✓ | ✓ | ✓ |
| dotted id ⇒ `parent_id`; undotted id ⇒ no parent | ✓ | ✓ | ✓ | ✓ | ✓ |
| non-empty `summary` | ✓ | ✓ | ✓ | ✓ | ✓ |
| non-`web` ⇒ non-empty plain-language reason | ✓ `REASON_GROUP` | ✓ `REASON_LOCAL_FS` | ✓ `REASON_LOCAL_FS` | ✓ `REASON_CAP_PREVIEW` | ✓ `REASON_CAP_PREVIEW` |
| non-`web` ⇒ `route.is_none()` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `availability` ∈ the closed vocabulary (`cli_only`) | ✓ | ✓ | ✓ | ✓ | ✓ |
| `risk` ∈ {read, local_write, remote_write, session_admin} | ✓ `read` | ✓ `read` | ✓ `read` | ✓ `read` | ✓ `read` |
| `scope` ∈ {workspace, project, forge, profile, provider} | ✓ `forge` | ✓ `project` | ✓ `project` | ✓ `forge` | ✓ `forge` |
| `category` ∈ `ALL_CATEGORIES` | ✓ `reference` | ✓ | ✓ | ✓ | ✓ |
| `cli_invocation` = `forge ` + id with dots→spaces | ✓ `forge cap` | ✓ `forge cap list` | ✓ `forge cap inspect` | ✓ `forge cap add` | ✓ `forge cap run` |
| every category still used at least once | ✓ `reference` was already used by `help`; count only rises | | | | |

Test-side consequences, all additive:

- `catalog_has_exactly_one_row_per_clap_path` (`src/cli/inventory.rs`):
  `missing` and `extra` both become empty.
- `catalog_integrity_is_clean` (`src/cli/inventory.rs`): `problems()` stays
  empty; it never sees a new problem because no rule is relaxed.
- `catalog_has_no_integrity_problems` (`catalog.rs`): unchanged and green.
- `catalog_covers_every_clap_path` (`catalog.rs`): the hard-coded total moves
  **234 → 239**. This pin is the count of *all* rows and is the only
  assertion that must change; it is not the parity assertion.
- `ids_are_unique_and_parented_rows_follow_dot_paths`: every leaf's id starts
  with `cap.` ✓.
- `web_rows_only_point_at_implemented_routes`: untouched. No added row is a
  `web` row, so the pinned `(id, route)` vector is byte-identical.
- The execution-block pin: untouched. `cli_only` rows carry no `execution`
  block (`push` sets `execution: None`).

## 7. Row-count pins outside the catalog

Three files assert the catalog's total size. The invariant each one encodes is
"a *web conversion* must not add rows"; this change deliberately adds five
non-`web` rows, so each pin moves to the new true total and keeps guarding what
it was written to guard:

| file | assertion | change |
|---|---|---|
| `src/api/command_catalog/catalog.rs:205` | `rows().len() == 234` (+ delta comment) | 239 |
| `tests/web_release_deploy_history_contract.rs:536` | `rows.len() == 234` ("catalog conversion must not add rows") | 239 |
| `tests/web_agent_identity_readonly_contract.rs:517` | `rows.len() == 234` ("catalog conversion keeps row count") | 239 |

`tests/web_command_reference_browser_contract.rs:264` mentions "the 234-row
render" only inside an assertion *message*; it is prose, not a pin, and is
left alone rather than churning a file this change does not otherwise need.

`tests/forge_web_command_catalog_contract.rs` needs no edit at all: its
`TOP_LEVEL_COMMANDS` allowlist is a hand-written subset that never listed
`cap`, and its `web_ids` allowlist is unaffected because no `cap` row is
`web`. Its `catalog_covers_the_real_help_output_of_the_binary` case — which
runs the real binary's `forge --help` and requires every listed name to be
covered — was a pre-existing failure on exactly this gap and goes green from
the catalog fix alone.

## 8. Scope boundaries honoured

- `src/cli/inventory.rs` — untouched. The parity assertion is the contract.
- `src/cli/cap.rs`, `src/cli/commands.rs` — untouched. No subcommand added,
  removed or renamed; `CAP_CONTRACT` unchanged.
- `.ai-gate/gate.yaml`, `src/gate/**` — untouched. The per-check timeout fix
  (`ccb3b0b`) is not disturbed.
- `builder.rs`, `model.rs` — untouched. No new method, no new enum variant.
- No new dependency. No reformat of unrelated files.
- `problems()`, `web_rows_only_point_at_implemented_routes`'s web vector and
  the parity assertion are all unmodified.

## 9. Verification

- `cargo test --lib command_catalog` — count 239, integrity empty, web
  allowlist unchanged, parent/prefix rule green.
- `cargo test --lib inventory` — `catalog_has_exactly_one_row_per_clap_path`
  and `catalog_integrity_is_clean` both pass.
- `cargo build`, `cargo fmt --check` (touched files), `cargo clippy` (default
  targets), `node scripts/check-openspec-change-names.mjs`,
  `node scripts/check-spec-governance.mjs`,
  `openspec validate --all --strict --no-interactive`, `git diff --check`.
- `git diff -- src/cli/cap.rs src/cli/commands.rs .ai-gate/gate.yaml` empty.