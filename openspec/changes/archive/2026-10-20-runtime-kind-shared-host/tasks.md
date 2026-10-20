## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm the publish inventory's contract already carries `runtime: "web|worker|job|library"` and that the document's shape and contract version are fixed.
- [x] 1.2 Confirm `RuntimeClass` is a private type in the publish lane, that nothing outside the lane reads it, and that it owns its own `as_str`, `parse` and `Display`.
- [x] 1.3 Confirm the fleet's rule is "only `compose_ready` entries invoke the provider" and that it is applied in two places: the model's iterator and the CLI's eligibility filter.
- [x] 1.4 Confirm the canonical capability states the old rule, including "including non-web runtime classes", and that it needs a MODIFIED requirement.
- [x] 1.5 Confirm the repository's shared-host instance: `modules/dotnet-platform-libs`, its `role: platform` and `deployable: false` declaration, and that it is not edited by this change.
- [x] 1.6 Confirm the manifest's `runtime:` block, the registry's runtime string and the profile catalog stay untouched.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add `src/runtime/`: the module, `RuntimeKind` with the four kinds, and `HostModel` with `OwnProcess` and `SharedHost`.
- [x] 2.2 Add the kind table with a summary per row, and `RuntimeKind::host_model()` — the `library` row is the shared-host row.
- [x] 2.3 Add the shared parser: trimmed, case-insensitive, refused by name with the accepted values, returning the shared error type.
- [x] 2.4 Add the `Display` impl for the shared type and the runtime module's own test suite.
- [x] 2.5 Delete the publish lane's private `RuntimeClass` and point `InventoryEntry.runtime` and `InventoryFleetEntry.runtime` at the shared type.
- [x] 2.6 Delete `runtime_display.rs` with the type it implemented, and update the pipeline to use the shared parser and the shared refusal.
- [x] 2.7 Add the host model to the fleet report row, with the shared-host reason.
- [x] 2.8 Make the publishing rule require an own-process kind, in the model's iterator and in the CLI's eligibility filter.
- [x] 2.9 Update the CLI's counts and summary to report what was publishable, what was held back, and the rule that held it back.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Check that every kind reports its host model, and that only `library` is shared-host.
- [x] 3.2 Check that an unknown runtime value is refused with the accepted values, and that the refusal still names the entry.
- [x] 3.3 Check that the inventory document's contract string, field names and spellings are unchanged, and that the same document still parses.
- [x] 3.4 Check that a `library` entry is never published, whatever compose file it declares, and that its reason names the host that adopts it.
- [x] 3.5 Check that a `web`, `worker` or `job` entry is published exactly as before, including the non-web own-process kinds.
- [x] 3.6 Check that the public-routing refusal names the host model and that a `web` entry's public route is unchanged.
- [x] 3.7 Check that the manifest, the profile catalog, the plugin registry, the release lane, the deploy lane and every sibling module are unchanged.
- [x] 3.8 Check that the publish inventory module's documentation states the host model and the rule.
- [x] 3.9 Check that the observe lane (`forge fleet online`) reads the same rule as the publish lane, so a module the fleet never started is never probed.
- [x] 3.10 Check that the liveness report names each entry's host model, and that a shared-host entry is reported as `SKIPPED` with the shared-host reason.
- [x] 3.11 Check that a refused entry names no runtime kind and no host model, so the row carries only what is true.

## 4. Verification

- [x] 4.1 Run the repository test suite: `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1`.
- [x] 4.2 Run the repository Gate: the `forge gate --dry-run` rehearsal and the full `forge gate`.
- [x] 4.3 Run `openspec validate --all --strict --no-interactive` and `git diff --check`.
- [x] 4.4 Run a real `forge publish fleet` against a fixture inventory that declares an own-process entry and a shared-host entry, and show that the module is reported and not published.

## 5. Implementation — authorized surfaces (2026-10-20)

- [x] 5.1 `src/runtime/` (new): the module, the kind table, the host model, the shared parser and the test suite.
- [x] 5.2 `src/publish/inventory/model.rs`, `src/publish/inventory/pipeline.rs` and `src/publish/inventory/runtime_display.rs`.
- [x] 5.3 `src/cli/fleet.rs`: the eligibility filter, the counts and the summary.
- [x] 5.4 `src/publish/inventory/mod.rs`: the module documentation.
- [x] 5.5 The new and updated tests.
- [x] 5.6 Record the exact output of every verification command in the evidence table below.
- [x] 5.7 Archive without `--skip-specs` so the capability is promoted into `openspec/specs/runtime-kind-shared-host/spec.md` and the inventory fleet's requirements are replaced, commit the implementation with its specs, then update the handoff pointer.

## 6. Review fixes (main agent, after the subagent's implementation)

- [x] 6.1 The observe lane read the old rule: `forge fleet online` probed every `compose_ready` entry, so a compose-ready module the fleet never started would read `NO-ROUTE` or `NOT-DEPLOYED` — a false alarm. The rule now lives in one function, `forge::publish::inventory::is_publishable`, and both lanes call it (`src/publish/inventory/model.rs`, `src/fleet/online.rs`, `src/cli/inventory.rs`).
- [x] 6.2 The liveness report now carries the host model per entry, and a shared-host entry reads `SKIPPED` with the shared-host reason as detail (`src/fleet/online.rs`).
- [x] 6.3 `forge inventory show`'s human table listed `id / runtime / classification / subdomain` and omitted the host model. It now carries the host model, and the summary adds the publishable count (`src/cli/inventory.rs`).
- [x] 6.4 The `publish fleet` help text and the fleet constants still said "only `compose_ready` entries invoke a provider". They now state the rule: compose-ready **and** an own-process runtime kind (`src/cli/commands.rs`, `src/cli/constants.rs`).
- [x] 6.5 The pipeline filled a refused entry's row with a placeholder runtime kind. With a host model attached, that placeholder read as a lifecycle claim the entry never made. A refused row now names no runtime kind and no host model, and carries only the refusal (`src/publish/inventory/model.rs`, `src/publish/inventory/pipeline.rs`, `src/cli/fleet.rs`).
- [x] 6.6 The shared-host reason moved to the vocabulary owner, `crate::runtime::SHARED_HOST_REASON`; the publish inventory re-exports it so both lanes say the same thing (`src/runtime/kind.rs`, `src/publish/inventory/model.rs`).
- [x] 6.7 The canonical capability `fleet-liveness-status` now states the observe rule, with a scenario for the module that is never probed (`openspec/changes/runtime-kind-shared-host/specs/fleet-liveness-status/spec.md`).
- [x] 6.8 The `decoupled-remote-publish` capability stated the old roster filter ("filter to `compose_ready` entries only") and the old report key. Both are replaced (`openspec/changes/runtime-kind-shared-host/specs/decoupled-remote-publish/spec.md`).
- [x] 6.9 The Gate's `source-file-size` check refused `src/fleet/online.rs` at 1069 lines after the review fixes. The liveness unit suite moved to `src/fleet/online_tests.rs` (the repository's `*_tests.rs` pattern), leaving the surface at 694 lines.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Repository test suite | PASS | `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` → exit 0; **3990 passed, 0 failed, 1 ignored**. Re-run after the liveness suite was split out: exit 0, the same 3990 / 0 / 1. |
| Focused suites | PASS | `cargo test --test fleet_online_contract` → 16 passed, 0 failed (including `fleet_online_never_probes_a_shared_host_module`). `cargo test -p forge --lib fleet::online_tests` → 23 passed, 0 failed. The runtime module's suite, the inventory's suite and the fleet CLI's suite are part of the 3990. |
| Repository Gate | PASS | `forge gate --dry-run` resolved all 8 required checks with none unbound. The full `forge gate --timeout-secs 1800` run exited 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-20T18:11:57Z"` on revision `191e96d`, with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. The `source-file-size` check reported "evaluated 453 of 453 candidate file(s); maximum 1000 physical line(s)". The `security` check reported the pre-existing unmaintained-crate advisories for `paste` and `unsafe-libyaml` and still passed. |
| Strict OpenSpec validation | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 132 passed, 0 failed (132 items)`, exit 0 (`✓ change/runtime-kind-shared-host`). `openspec validate runtime-kind-shared-host --strict --no-interactive` → `Change 'runtime-kind-shared-host' is valid`. |
| OpenSpec change names | PASS | `node scripts/check-openspec-change-names.mjs` → `check-openspec-change-names: PASS`, exit 0. |
| Whitespace | PASS | `git diff --check` → exit 0, no output. |
| Formatting | PASS | `rustfmt --edition 2021 --check` on every file this change created or edited → clean (`src/runtime/{mod,kind,kind_tests}.rs`, `src/lib.rs`, `src/publish/inventory/{model,pipeline,mod}.rs`, `src/cli/{fleet,fleet_exec,commands,constants,inventory}.rs`, `src/fleet/{mod,online,online_tests}.rs`, `tests/{inventory,fleet_live_rollout,fleet_online}_contract.rs`). `cargo fmt --all` was never run. |
| Lint | PASS | `cargo clippy --workspace --all-targets --all-features` → no warning in any file this change touched; the reported warnings are pre-existing (`crates/cargo-audit-boost`, `crates/cargo-sonar`, `src/{hosted_ci/store,registry/model,deploy/model,contract/mod,cli/auth,auth/setup}.rs`, `tests/{origin_repository_retirement,deploy_ssh_adapter}_contract.rs`). |
| Real fleet | PASS | A fixture inventory at `/tmp/forge-change10/inventory.json` with three entries — an own-process `web` entry with a real Compose file, a compose-ready `library` entry, and an entry whose runtime is `svc`. `forge inventory show` reported `declared=3, compose_ready=2, … invalid=1, publishable=1` and the rows `web-app / web / own_process / compose_ready`, `platform-modules / library / shared_host / compose_ready`, `broken-svc / - / - / invalid` — the refused entry names no runtime kind and no host model. `forge publish fleet --inventory … --dry-run` published only `web-app` (4 stages planned) and reported `fleet publish: 1/1 publishable succeeded, 1 held back, 1 skipped (3 in inventory:/tmp/forge-change10/inventory.json); a shared-host module is never started by the fleet`. The JSON summary carried `publishable: 1`, `held_back: 1` with the held-back row naming `runtime: "library"`, `host_model: "shared_host"` and the shared-host reason, and `skipped: 1` with the refused row naming no runtime kind. |

## Discrepancies found while implementing

1. **The design's "nothing outside the publish lane can read it" is not true of the code.** `RuntimeClass` was `pub` and re-exported by `forge::publish::inventory`, and `src/cli/fleet_exec.rs` read `forge::publish::inventory::RuntimeClass::Web` in the legacy-registry adapter and in three scheduler test fixtures. The type was private to the *publish lane's own vocabulary* but publicly reachable. The design's intent — one owner for the vocabulary — is implemented; those readers now read `forge::runtime::RuntimeKind`.
2. **`src/lib.rs` module position.** The brief asked for `pub mod runtime;` "between `registry` and `release`". The list in `src/lib.rs` is alphabetical, and `runtime` sorts after `remediation`, so the module was added between `remediation` and `semantic`. The instruction's two halves conflict; the alphabetical convention was kept.
3. **The public-routing refusal has two shapes.** The design's sentence — "…runtime `library` is a shared-host module; only an own-process runtime may expose HTTP" — is true only for the shared-host kind. For an own-process non-web runtime (`worker`, `job`) the same sentence would be false, so the pipeline refuses with the parallel sentence "…runtime `worker` is an own-process runtime; only a web runtime may expose HTTP". Both name the host model; the behaviour (only `web` may declare `public_http`) is unchanged.
4. **The zero-publishable refusal changed wording.** It read "no compose_ready entries … check that every project declares a Compose file at a resolvable source path", which is false when the only entries are shared-host modules. It now reads "no publishable entries … an entry is publishable when it is compose-ready and its runtime kind is an own-process kind", plus "; a shared-host module is never started by the fleet" when an entry was held back.
5. **The fleet summary's JSON keys changed.** `fleet.compose_ready` is now `fleet.publishable`, and the summary gains `held_back` (a count) and `held_back_entries` (the rows the host model held back). The rows also carry `host_model`. The inventory *document* is unchanged: same contract string, same field names, same four spellings.
6. **The counts in the operator summary.** "held back" counts the entries the host model held back (compose-ready, shared-host) and "skipped" keeps its old meaning (not compose-ready), so the three counts still partition every declared entry and the message stays true when nothing is held back.
