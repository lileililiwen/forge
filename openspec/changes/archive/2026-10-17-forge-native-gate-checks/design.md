# Design: Forge-owned native gate

## 1. Implementation boundary

The implementation is the Forge Rust workspace. `src/gate/engine.rs` is the only public execution path for `forge gate`; it invokes a hidden subcommand on the same Forge executable, which dispatches to the already vendored `crates/driftwatchdog` implementation. The internal crate is compiled into Forge, not separately installed or resolved. Its Forge-specific entry point returns the existing machine-readable result while using an in-memory store and writing no DriftWatch project history. `src/cli/functions_13.rs` persists Forge evidence and operation-journal outcomes.

The default plan is constructed by the internal gate engine when neither `gate.toml` nor `.ai-gate/gate.yaml` exists. When either file exists, its explicitly configured checks extend the mandatory baseline. Update Forge's CLI help, README, `docs/architecture.md`, `docs/provider-evidence.md`, `requirement.md` section 24, and the CI gate job to remove the separate runtime requirement. Do not modify sibling repositories or implement the dependent CI/pre-commit templates.

## 2. Language and runtime

Rust 2021, root package `forge` (MSRV 1.88), and the existing vendored Rust gate-engine workspace member. The gate executor remains synchronous and local. Project commands use the existing bounded execution adapter; the built-in scanner runs in process. No shell, network request, or external gate binary is needed for the built-in baseline. Build/test commands are `cargo fmt --all -- --check`, `cargo build --workspace --all-targets --all-features`, focused gate contract tests, and `cargo test --workspace --all-targets`.

## 3. Ownership and shared code

Forge Core owns check selection, baseline version, execution orchestration, evidence, status, and exit code. Reuse the vendored engine's parser, command adapter, aggregation, source scanner, redaction, and result vocabulary through a Rust API. The public caller is `forge gate`; internal crate and legacy type names do not establish an external product dependency. No sibling checkout or separate DriftWatchdog installation is used. Project extension commands stay in project-owned gate declarations and run only when explicitly bound.

## 4. User experience and interface

The CLI remains `forge gate [TARGET] [--dry-run] [--timeout-secs N] [--format human|json]` and `forge gate status [TARGET]`. Dry-run lists the mandatory baseline first, then project extensions, including check id, required status, source, timeout, and effective baseline pack version. A real run reports every result and remediation in stable check-id order. Passing exits 0; a blocked result exits nonzero; malformed configuration and incomplete required execution are explicit failures. JSON stdout is one parseable document; operational diagnostics use stderr. No interactive input or color is required. This is not a browser workflow; browser/device testing and responsive layout do not apply.

## 5. Behavioral model

| Input state | Selected plan | Side effects | Result |
|---|---|---|---|
| No gate manifest | Mandatory `source-file-size` baseline at 1,000 physical lines | Read-only scan; Forge writes one evidence record and one journal row only for a real run | pass or blocked based on scan completeness/violations |
| Valid gate manifest | Mandatory baseline union explicit manifest checks | Each explicit extension executes once; one Forge evidence record and journal row on real run | Aggregate uses the existing blocking policy and preserves per-check states |
| Invalid gate manifest | No plan | No check executes; prior evidence and journal remain unchanged | typed configuration failure |
| Dry-run | Resolve baseline plus extensions; execute none | No evidence or journal write | plan/rehearsal only; never a verdict |
| Required extension has no command | Baseline plus declared extension | No command execution for that row | explicit `UNBOUND`/review result under the existing binding policy; never silently omitted |

The baseline check is mandatory and cannot be removed, exempted, or weakened by the project manifest. The existing source scanner uses its safe repository boundary and 1,000-line default. Project manifests may add explicit check ids and command bindings using the existing `[[checks]]` and `[project_commands]` format. Duplicate ids are rejected or collapsed to the one baseline row before execution; the final plan contains exactly one `source-file-size` row.

## 6. Contract and compatibility

Keep `GATE_CONTRACT_VERSION` at `0.1.0` and keep the serialized evidence schema additive. Old evidence with runtime `driftwatchdog` remains readable byte-for-byte. New evidence records `runtime: "forge"`, Forge's version, the same revision and timestamp fields, and an effective manifest digest that includes `forge-baseline/1.0.0` plus any project manifest bytes. `rule_pack_version` remains the project-declared identity when present and is `forge-baseline/1.0.0` when no project manifest exists. The source-file-size row identifies Forge baseline provenance.

`verification.gate_runtime` and `FORGE_GATE_BIN` are retained as legacy metadata/environment inputs for compatibility outside this invocation but no longer alter `forge gate` resolution. `forge gate` never probes `PATH` for `driftwatchdog` or `driftwatch`. Existing `.ai-gate/gate.yaml` and `gate.toml` extension declarations remain readable. No migration is required to receive the default baseline.

## 7. Failure and boundary policy

- A missing target directory, malformed manifest, invalid extension declaration, or unsupported policy is a typed failure before checks and before evidence/journal writes.
- A baseline enumeration/read failure is `REVIEW_REQUIRED`; incomplete source coverage cannot pass.
- An oversized source file is `FAIL`, names a project-relative path and observed/configured counts, and blocks the run.
- A required extension with no binding or a timeout is non-passing under the existing policy. Other checks still run when one check fails.
- Secret-bearing output is redacted and bounded by existing helpers. Project paths are projected as relative paths or `<project>`.
- Dry-run and status reads are side-effect-free. Real runs atomically persist one Forge evidence record and the existing operation row. No external runtime history is written.
- Existing unknown/custom check IDs remain extensions; they cannot override the built-in baseline id or use a duplicate id to hide it.

## 8. Verification oracle

- Internal gate-engine tests prove default-manifest resolution selects one required `source-file-size` row and uses the documented baseline version.
- Forge integration tests run against a temporary repository with no gate manifest, with a passing and oversized fixture, and assert JSON rows, exit codes, relative paths, and persisted evidence.
- Extension tests prove a registered project command runs once alongside the baseline, preserves a blocked verdict, and does not replace baseline policy.
- Runtime-resolution tests prove an empty `PATH`, an unset or invalid `FORGE_GATE_BIN`, and stale `.project.json` runtime metadata do not prevent the Forge-owned engine from running.
- Dry-run and invalid-manifest tests assert evidence, `.forge` and operation-journal bytes are unchanged.
- Historical fixture tests prove old `runtime: driftwatchdog` records remain readable and unchanged.
- `cargo fmt --all -- --check`; focused Gate tests; full workspace tests; `cargo build --workspace --all-targets --all-features`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; name preflight; strict OpenSpec; `git diff --check`; and the required `forge gate --dry-run` plus bounded real `forge gate` run.

## 9. Decision ledger

- The first built-in baseline is deliberately narrow and universal: repository-aware physical source-file line count, default maximum 1,000. Broader security, style, build, and test rules require language/profile applicability and are not claimed by this package.
- Existing project commands are additive extensions; this change does not auto-discover scripts or make any sibling policy checker mandatory.
- Forge owns the gate contract even though implementation code is reused from an already vendored workspace member. It is linked into the Forge execution path and is not installed or resolved separately.
- CI and pre-commit templates, generator integration, and legacy adoption are the single dependent follow-up package and remain unimplemented by this change.
