## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm `requirement.md` §30 names an SSH server as an initial target and says deployment is adapter-based.
- [x] 1.2 Confirm the `ssh` kind is known but not wired in the shared registry, and that the one refusal sentence names it in every entry point.
- [x] 1.3 Confirm the manifest's `host`, `user`, `path` and `service` fields are never validated, and that `host` and `user` are read by nothing.
- [x] 1.4 Confirm one binary serves every kind today, and that the reference Jenkins adapter refuses the kinds it does not know.
- [x] 1.5 Confirm the executor contract's argv, envelope schema and classification table need no change for a remote target.
- [x] 1.6 Confirm the publish lane's SSH facts — a staged copy, an atomic rename, the two remote binaries — and that the publish lane itself is out of scope.
- [x] 1.7 Confirm the shared contract suite and the deploy suite hard-code the supported-kind list, and that they must be updated by the new kind.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Give the registry entry the three new fields: the required target fields, the kind's adapter variable and the kind's adapter default.
- [x] 2.2 Move the `ssh` kind from known-but-unwired to wired, and give it the required fields `host` and `path` and the adapter variable `FORGE_DEPLOYER_SSH_BIN` with the default `forge-deployer-ssh`.
- [x] 2.3 Add the accessors the deploy surface reads: the required fields of a kind and the default adapter of a kind.
- [x] 2.4 Refuse a target that does not state the fields its kind requires, naming the kind and the fields.
- [x] 2.5 Give `DeployAdapterConfig` the kind-specific binaries, and read each kind's own variable in `from_env`.
- [x] 2.6 Resolve the adapter per kind in one order — the kind's own variable, the shared variable, the kind's default — and use the resolved binary for the spawn, the failure messages and the attribution.
- [x] 2.7 Write `adapters/ssh/forge-deployer-ssh`: the staged copy, the atomic rename, the host-side health check and the envelope.
- [x] 2.8 Write `adapters/ssh/ssh-adapter.md`: the executor mapping, the exit-code mapping, the health vocabulary, the configuration and what a real host still needs.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Check that an `ssh` target with a host and a path is served, and that the plan reports the target's host as the kind's requirement.
- [x] 3.2 Check that an `ssh` target without a host is refused with the shared sentence, and that the refusal happens before any adapter runs.
- [x] 3.3 Check that a legacy `deployment.type: ssh` target is refused for the same missing fields.
- [x] 3.4 Check that the adapter is resolved in the stated order for each rung, including the case where nothing is set.
- [x] 3.5 Check that the other kinds resolve exactly the binary they resolved before this change.
- [x] 3.6 Check that the SSH adapter copies to a staged name and renames it, and that the remote path never appears in the evidence.
- [x] 3.7 Check that the health vocabulary maps to `running`, `failed` and `unknown`, and that an unreachable host observes `unknown`.
- [x] 3.8 Check that a dry run contacts no host and copies nothing.
- [x] 3.9 Check that the executor contract's argv, envelope schema, classification table and discriminator are unchanged.
- [x] 3.10 Check that the manifest shape, the plugin registry, the publish lane, the release and delivery lanes and every sibling module are unchanged.
- [x] 3.11 Check that the deploy executor's documentation, the deploy module's own documentation and the manifest field documentation state the new facts.

## 4. Verification

- [x] 4.1 Run the repository test suite: `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1`.
- [x] 4.2 Run the repository Gate: the `forge gate --dry-run` rehearsal and the full `forge gate`.
- [x] 4.3 Run `openspec validate --all --strict --no-interactive` and `git diff --check`.
- [x] 4.4 Run a real deploy against an `ssh` target with a fake `ssh`/`scp` and show each outcome: served, refused for a missing field, and the unreachable host.

## 5. Implementation — authorized surfaces (2026-10-20)

- [x] 5.1 `src/extension/kinds.rs`: the registry's new fields, the `ssh` entry and the accessors.
- [x] 5.2 `src/deploy/model.rs`: the adapter resolution and the required-field refusal.
- [x] 5.3 `src/deploy/engine/stages.rs`: the resolved binary used for the spawn, the failures and the attribution.
- [x] 5.4 `adapters/ssh/forge-deployer-ssh` and `adapters/ssh/ssh-adapter.md`.
- [x] 5.5 `docs/adapter-contracts/deploy-executor.md`, `src/deploy/mod.rs` and `src/core/manifest.rs`.
- [x] 5.6 The new and updated tests.
- [x] 5.7 Record the exact output of every verification command in the evidence table below.
- [x] 5.8 Archive without `--skip-specs` so the capability is promoted into `openspec/specs/deploy-target-ssh/spec.md` and the deploy contract's requirement is replaced, commit the implementation with its specs, then update the handoff pointer. — Archived at `openspec/changes/archive/2026-10-20-deploy-target-ssh/` without `--skip-specs`; four requirements promoted into `openspec/specs/deploy-target-ssh/spec.md`, the "Explicit adapter deployment" requirement replaced in `openspec/specs/adapter-deployment/spec.md`, and both `TBD` or stale Purposes corrected before the commit. The implementation and its specs are one commit; the handoff pointer update is the second.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Repository test suite | PASS | `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` → exit 0; **3971 passed, 0 failed, 1 ignored**. The count is one higher than the worker's run because the main agent added `the_adapter_refuses_a_kind_it_does_not_serve`. |
| Focused suite: the shared contract | PASS | `cargo test -p forge --lib extension::` → **12 passed, 0 failed** (1338 filtered out). |
| Focused suite: the deploy lane | PASS | `cargo test -p forge --lib deploy::` → **49 passed, 0 failed** (1301 filtered out). |
| Focused suite: the deploy CLI contract | PASS | `cargo test -p forge --test deploy_contract` → **21 passed, 0 failed**. |
| Focused suite: the SSH adapter | PASS | `cargo test -p forge --test deploy_ssh_adapter_contract` → **12 passed, 0 failed**. |
| The SSH adapter is a runnable Python 3 program | PASS | `python3 -m py_compile adapters/ssh/forge-deployer-ssh` → no output, exit 0; `ls -l` → `-rwxr-xr-x`; `./adapters/ssh/forge-deployer-ssh --version` → `forge-deployer-ssh/0.1.0 (contract forge-deploy-executor/0.1.0)`. |
| Strict OpenSpec validation | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 131 passed, 0 failed (131 items)`, and `✓ change/deploy-target-ssh`. `openspec validate deploy-target-ssh --strict --no-interactive` → `Change 'deploy-target-ssh' is valid`. |
| Change names | PASS | `node scripts/check-openspec-change-names.mjs` → `check-openspec-change-names: PASS`. |
| Whitespace | PASS | `git diff --check` → no output, exit 0. |
| Formatting | PASS | `rustfmt --edition 2021 --check` on every changed and created Rust file → no output. `cargo fmt --all` was not run; the twelve pre-existing rustfmt-dirty files were not touched. |
| File size | PASS | `wc -l`: `src/deploy/model.rs` 785, `src/deploy/engine/stages.rs` 709, `src/deploy/engine/stages_tests.rs` 952, `tests/deploy_contract.rs` 922, `tests/deploy_ssh_adapter_contract.rs` 519, `src/extension/kinds.rs` 154, `src/extension/extension_tests.rs` 259, `src/core/manifest.rs` 794, `adapters/ssh/forge-deployer-ssh` 490. Every file is under 1000 physical lines. |
| The three `ssh` outcomes, against the adapter | PASS | `tests/deploy_ssh_adapter_contract.rs`: `the_apply_transfers_the_artifact_in_three_commands` (served; the three argv lines in order), `an_ssh_target_without_its_fields_is_refused_by_name` and `plan_refuses_an_ssh_target_without_its_fields` (refused for a missing field, in the shared sentence), `an_unreachable_host_is_unknown_not_failed` (unreachable → `unknown`). |
| Repository Gate | PASS | `forge gate --dry-run` resolved all 8 required checks with none unbound; the exemption budget reported `cap=2 declared=0 headroom=2 required=8 verdict=within`. The full `forge gate --timeout-secs 1800` run exited 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-20T14:25:03Z"` on revision `3ff3fac`, with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. The `source-file-size` check reported "evaluated 450 of 450 candidate file(s); maximum 1000 physical line(s)". The `security` check reported the pre-existing unmaintained-crate advisories for `paste` and `unsafe-libyaml` and still passed. |
| Real deploy | PASS | The main agent's run, against a fixture project with a committed `forge.yaml` declaring an `ssh` target with `host: home-server-01`, `user: deploy` and `path: /srv/app/docker-compose.yml`, a recording `ssh`/`scp` pair on `FORGE_SSH_BIN` / `FORGE_SCP_BIN`, the reference adapter on `FORGE_DEPLOYER_SSH_BIN`, and real Gate evidence for the project (`forge gate` → `aggregate: passed`, `profile: full`, bound to the project's HEAD). **Served:** `forge deploy plan` reported `target: vps (ssh)`; `forge deploy apply --target-name vps --confirm` reached the adapter and the recording programs logged exactly `ssh deploy@home-server-01 /bin/mkdir -p /srv/app`, `scp docker-compose.yml deploy@home-server-01:/srv/app/docker-compose.yml.forge-new`, `ssh deploy@home-server-01 /bin/mv -f /srv/app/docker-compose.yml.forge-new /srv/app/docker-compose.yml` — the three commands, in that order, with the staged name in between. The stage evidence reported `executor=forge-deployer-ssh/0.1.0@unknown` and the remote path appeared only as `<host-path>`. **Observed:** `forge deploy observe` with the host answering `Up 4 minutes` recorded `observe target=vps running: the host reports the service is up` with the evidence `docker ps --filter name=app: Up 4 minutes` and the host-side command `ssh deploy@home-server-01 docker ps --filter name=app --format '{{.Status}}'`. **Unreachable:** the same observe with `ssh` exiting 255 recorded `observe target=vps unknown: the host could not be reached; unknown is not offline proof`. **Refused:** a target with neither field gave `error[deploy-invalid]: deploy invalid: deployment target `vps` of kind `ssh` must declare `host` and `path``; a target with only a host gave `error[deploy-invalid]: deploy invalid: deployment target `vps` of kind `ssh` must declare `path``. Every refusal exited 1. |

### Notes on the implementation, for the reviewer

- **The refusal happens in the configuration validator**, in
  `DeployConfig::from_manifest_meta`, next to the kind check. The plan
  (`prepare_deploy`) does not re-check the fields, because a
  `DeployConfig` that reached it came through that validator. The legacy
  `deployment.type` form states no host and no path, so `type: ssh` is
  refused in the same words.
- **`DeployAdapterConfig::deployer_bin` is now empty when
  `FORGE_DEPLOYER_BIN` is unset**, because the resolution has to tell
  "unset" from "set to the default". The only reader outside the deploy
  lane is the admin API's scrub list (`src/api/admin/deploy.rs`), which
  skips empty secrets; a bare `forge-deployer` was never a local path, so
  nothing is lost there.
- **The adapter refuses a remote path, host or user outside a safe
  character set** (`[A-Za-z0-9._+/=,@-]`). The path, the host and the
  user travel as plain argv words to `ssh` and `scp`, and `scp` cannot
  address a path with a space unambiguously. The refusal happens before
  any command runs, so a declared value can never become a remote
  command. The design does not state this rule; it is the adapter's own
  boundary, and it is recorded here.
- **The `http` health check maps any non-zero remote exit to `failed`**
  on a reachable host, and `curl`'s exit 22 (a non-2xx answer) is the
  case the design names. A host that cannot be reached (`ssh` exits 255)
  is `unknown`, never `failed`. The `docker` check follows the design's
  output-driven rule: an empty `docker ps` answer is `unknown` (the
  service is not visible), a status containing `Up` is `running`, and a
  visible service that is not up is `failed`.
- **The executor contract's argv shape, envelope schema, classification
  table and discriminator are untouched.** The contract document's
  `--kind` list read `<local|docker-compose|mac-runtime>` and omitted the
  wired `jenkins` kind; it now names the wired kinds, including `ssh`.
  That is a correction to the document's enumeration, not a change to the
  argv's shape.
- **The canonical `openspec/specs/adapter-deployment/spec.md` was not
  edited.** The change package carries the `MODIFIED` requirement in
  `openspec/changes/deploy-target-ssh/specs/adapter-deployment/spec.md`,
  and the archive step (task 5.8) promotes it into the canonical spec.
