## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map the end-user auth setup requirement to the operator identity surface, the project manifest, the provider boundary and the sibling packages.
- [x] 1.2 Confirm the ownership boundaries: Forge Core owns the contract and the CLI, `modules/identity-providers` owns the packs, the .NET module owns the ASP.NET Core host adapter, the governance module owns the D9 policy. Record no implementation authority over a sibling.
- [x] 1.3 Confirm that the publish-provider code, the `platform-contracts` contract set, DriftWatch, the PTY manager and the content and analytics providers are outside the boundary.
- [x] 1.4 Baseline the current state: no `forge auth` command, no provider pack, no `auth:` block in the project manifest, and no identity-provider contract in this repository.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add the `src/auth/` module: the contract types, the pack catalog and the setup runner, with Core's own copy of the wire types.
- [x] 2.2 Validate every answer: contract major, provider, operation, project, project revision, plan digest, secret material and URL safety. Scan the raw answer, before the typed response drops unknown fields.
- [x] 2.3 Add the `auth` CLI surface to `src/cli/commands.rs`, `src/cli/auth.rs` and `src/main.rs`, with `ForgeError::AuthSetupRefused`.
- [x] 2.4 Implement the preview, the confirmation and the digest-bound apply. A preview and a declined setup write nothing.
- [x] 2.5 Add the `modules/identity-providers` module: its own Cargo workspace, the `forge-identity-provider` contract crate, the two packs, the pack descriptors, the module identity and its Gate.
- [x] 2.6 Write the contract fixtures: the refusals and the one accepted answer.
- [x] 2.7 Implement the Keycloak pack: one realm and one public client per project on the pinned image, a generated admin password in an owner-only file, and a reference instead of a value.
- [x] 2.8 Implement the Supabase pack: refuse a shared project, refuse to apply, and stay blocked until the OIDC and registration capability is verified.
- [x] 2.9 Derive the population from the project id, so two projects never share a realm, a client or an issuer.
- [x] 2.10 Refuse a hosted project that another product already uses.
- [x] 2.11 Record a standard OIDC configuration: issuer, client id, callback path, scopes, registration link, secret references only.
- [x] 2.12 Enforce URL safety: HTTPS or plain HTTP on loopback, and a registration link on the issuer's own origin.
- [x] 2.13 Report the outcome as `preview`, `applied`, `blocked` or `declined`, with a reason and a recovery instruction.
- [x] 2.14 Write the `auth:` block into the project manifest only on a confirmed setup, and never twice for the same provider.
- [x] 2.15 Write the CLI contract tests in `tests/auth_setup_contract.rs`.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Check that the operator identity surface, the publish providers and the shared contract set are unchanged.
- [x] 3.2 Check that the .NET module, the governance module and the Rust platform module are unchanged.
- [x] 3.3 Check that the module carries its own identity, its own Gate and its own documentation, and that `forge module list` resolves it.
- [x] 3.4 Check that no secret value reaches the manifest, the log or the command output, and that the only credential in the local runtime is an owner-only file.

## 4. Verification

- [x] 4.1 Run the Forge Core contract tests: `cargo test --test auth_setup_contract`.
- [x] 4.2 Run the module's own tests: `cargo test --workspace --manifest-path modules/identity-providers/Cargo.toml`.
- [x] 4.3 Run the module's own Gate: `forge module verify identity-providers`.
- [x] 4.4 Run the repository Gate: the `forge gate --dry-run` rehearsal and the full `forge gate`.
- [x] 4.5 Run `openspec validate --all --strict --no-interactive` and `git diff --check`.
- [x] 4.6 Collect the real Keycloak evidence: two projects, two realms, two clients, two issuers, and a cross-instance token refusal.

## 5. Implementation — authorized surfaces (2026-10-19)

- [x] 5.1 Forge Core: `src/auth/`, `src/cli/auth.rs`, `src/cli/commands.rs`, `src/cli/mod.rs`, `src/core/mod.rs`, `src/lib.rs`, `src/main.rs`, `tests/auth_setup_contract.rs`.
- [x] 5.2 The new module `modules/identity-providers/` with its crates, its pack descriptors, its `.project.json`, its `.ai-gate/gate.yaml`, its `README.md` and its `HANDOFF.md`.
- [x] 5.3 The project manifest `auth:` block, written only on a confirmed setup.
- [x] 5.4 Record the exact output of every verification command in the evidence table below.
- [x] 5.5 Archive without `--skip-specs` so the capability is promoted into `openspec/specs/forge-auth-provider-extensions/spec.md`, commit the implementation with its specs, then update the handoff pointer. The archive promoted seven requirements into `openspec/specs/forge-auth-provider-extensions/spec.md`, and strict validation still reports 125 of 125.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Forge Core contract tests | PASS | `cargo test --test auth_setup_contract` ran 12 tests, all passed. They cover the preview that writes nothing, the confirmed setup that records one population, the repeated setup that does not duplicate the entry, two projects that get two populations, the answer for another project that is refused, the answer that carries a secret that is refused, the provider failure that leaves the project unchanged, the uninstalled provider that is blocked, the installed pack list, and the JSON envelope. |
| Module contract fixtures | PASS | `cargo test --workspace --manifest-path modules/identity-providers/Cargo.toml` ran 13 tests, all passed: the unknown contract major, another project, another operation, a stale revision, secret material, a value that pretends to be a reference, a registration URL on another host, a plain HTTP issuer that is not loopback, a plan without a digest, an empty request field, a secret in the summary, a secret in the plan changes, and the one accepted answer. |
| Module Gate | PASS | `forge module verify identity-providers` returned `aggregate: passed` for `build`, `declared-verification`, `format`, `repository`, `security`, `source-file-size` and `tests`, observed `2026-10-19T15:44:15Z` at revision `d49e836bf091`. Evidence in `modules/identity-providers/.forge/gate/identity-providers/evidence.json`. |
| Repository test suite | PASS | `scripts/check-tests.sh` completed with no failing test target: the Rust workspace suite, the .NET and Rust import checks, and the .NET and Rust sibling import verifiers. |
| Repository Gate | PASS | The dry run resolved all eight required checks with none unbound. The full `forge gate --timeout-secs 1800` returned `aggregate: passed` for `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests`, observed `2026-10-19T16:09:52Z` at revision `d49e836bf091`. Evidence in `.forge/gate/forge/evidence.json`. |
| Strict OpenSpec validation | PASS | `node scripts/check-openspec-change-names.mjs` reports `PASS`; `openspec validate --all --strict --no-interactive` reported `Totals: 125 passed, 0 failed (125 items)`. |
| Whitespace | PASS | `git diff --check` reported no findings. The new files carry no trailing whitespace. |
| Real Keycloak evidence | PASS | Two projects (`evidence-one`, `evidence-two`) on one runtime produced two realms, two clients and two issuers: `http://127.0.0.1:18095/realms/forge-evidence-one` and `http://127.0.0.1:18096/realms/forge-evidence-two`. The administrator token of the first instance was refused by the second with HTTP 401. A preview left the project manifest byte-identical; a confirmed setup wrote one `auth:` block; the repeated confirmed setup left one block. A runtime that did not accept the stored credential was refused with `provider_credential_mismatch` and the exact reset command, the manifest kept its working configuration, and the documented reset command recovered the setup. |
