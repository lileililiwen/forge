# Design: Profile-aware Forge Gate

## Implementation boundary

Implement in Forge's Rust workspace: `src/main.rs`, `src/cli/functions_13.rs`, `src/gate/{model,engine,projection,rehearsal}.rs`, `crates/driftwatchdog/src/{cli.rs,commands/gate.rs,gate/manifest.rs}`, `src/delivery/handlers.rs`, hosted CI validation, and `src/standard` plus new `standards/baseline-{service,client}` snapshots. Update only the canonical OpenSpec deltas and CI/CD model. Do not edit sibling repositories, old snapshots, or Forge's own CI job graph.

## Language and runtime

Rust stable 2021 for Forge and vendored Driftwatchdog, Clap for CLI, Serde JSON evidence, shell/YAML for generated packs, SQLite remains unchanged. Test with focused Rust integration/unit tests, snapshot contract tests, CI YAML/script checks, and the local Forge Gate. Generated projects do not need Forge except when invoking the installed hook/CI tool as already required by their standard pack.

## Ownership and shared code

Forge owns profile semantics and Gate evidence. The vendored Driftwatchdog engine receives a typed profile parameter from Forge's private engine dispatch. The project Gate manifest remains the source for full extension checks. Standard snapshots are immutable and consumed by project repositories through pinned generated files. No shared package extraction is appropriate: no other repository consumes Gate evidence for delivery approval, and the `platform.gate-result` contract lacks this authority tier.

## User experience and interface

`forge gate [TARGET] [--profile fast|full] [--dry-run] [--timeout-secs N]`. `full` is the default. Fast output names the selected profile, executes the mandatory baseline and exits nonzero on baseline failure. Full output additionally executes every selected extension. Status output includes the saved profile. Invalid profile values are rejected by Clap; profile is invalid with `status` and `evidence` forms. The generated hook calls `forge gate . --profile fast`; CI's `.standard/scripts/verify.sh` calls `forge gate . --profile full`. Terminal text and JSON carry the profile; no browser surface changes.

## Behavioral model

| Input | Planned checks | Evidence | Full-only consumers |
|---|---|---|---|
| omitted profile | Same as `full` | `profile=full` | accepted if fresh, passed, revision/project matched |
| `full` | mandatory baseline + every declared enabled extension | `profile=full` | accepted if fresh, passed, revision/project matched |
| `fast` | mandatory in-process `source-file-size` only | `profile=fast` | refused with recovery instruction |
| unknown profile | no checks run | prior evidence unchanged | refused |
| malformed declaration | no checks run; fail closed | prior evidence unchanged | refused |

Profile selection filters only the resolved extension plan after the manifest has been parsed and validated. Therefore fast cannot use malformed configuration to evade validation or write a pass. The baseline is injected after selection and cannot be removed, exempted or marked optional. Full remains the default for direct command compatibility. No persistence schema migration is needed.

## Contract and compatibility

- Add `GateProfile` (`Fast`, `Full`) to Forge Gate evidence as a serialized lowercase string. Deserialization defaults missing profile to `Full`; this is safe because legacy runtime always ran the whole resolved plan.
- Bump `GATE_CONTRACT_VERSION` to `0.2.0`. Update hosted CI envelope validation to accept `0.1.0` as legacy-full and `0.2.0` with explicit profile; reject legacy records that claim fast (they cannot express it).
- `forge gate` direct default is full; `forge gate status` reflects profile but otherwise keeps current response semantics.
- Delivery requires `profile=full`; refusal names observed profile and exact rerun command.
- Hosted CI uploader refuses fast evidence before network transmission. Forge ingestion rejects fast evidence even if a producer bypasses the bundled uploader.
- Add new service/client snapshots (service `1.6.0`, client `1.3.0`), update their provenance and default constants. Pin both installers to Forge `0.1.1`, the package version introduced with this profile CLI. Keep prior snapshots byte-immutable. Pre-commit runs fast, hosted CI runs full. Both retain the existing hosted evidence upload behavior.

## Failure and boundary policy

Missing profile defaults full only when reading legacy evidence. Unsupported CLI profile is an input error. Invalid Gate manifest, unavailable Forge runtime, timeout, or baseline failure is nonzero; no false evidence pass is persisted. Fast extension checks are explicitly omitted, not recorded as passed. Full check failures remain blocking. Delivery and hosted upload treat fast evidence as refusal and report how to obtain full evidence. Existing CI snapshots retain prior behavior.

## Verification oracle

- Engine tests: fast plan includes exactly mandatory source-size; full preserves all selected declarations; malformed declarations fail before execution; full remains default.
- CLI tests: profile reaches private engine, output records profile, dry-run is profile-aware, invalid action/profile pair fails without overwriting evidence.
- Evidence tests: round-trip both profiles; legacy JSON decodes as full; hosted envelope accepts legacy-full and current full; rejects fast.
- Delivery tests: fast fresh passing evidence is rejected without provider/journal side effects; full fresh passing evidence retains existing success path.
- Standard tests: latest generated service/client snapshots call fast in pre-commit and full in hosted CI, preserve uploader/install behavior and provenance, and become generation defaults. Existing snapshots are unchanged.
- Run `cargo test --lib gate:: -- --test-threads=1`, focused Gate/delivery/hosted/standard integration contracts, `cargo check --all-targets`, changed Rust rustfmt, relevant shell/Python/Node syntax checks, name preflight, strict OpenSpec validation, `git diff --check`, `forge gate --dry-run`, and bounded full `forge gate`.
- Record elapsed full and fast Gate durations in this checkout. Do not infer cross-project timing from Forge's result.

## Decision ledger

- Fast scope is only the Forge-mandated built-in baseline; extension checks are full-only and visibly omitted from fast plans. This gives a stable, language-neutral local floor without inventing project-specific check classifications.
- Legacy evidence is full because the earlier runtime had no profile selection. New evidence is explicit.
- Delivery and hosted ingestion are the approval boundaries. A fast result cannot be upgraded into full by a wrapper or caller.
- New immutable standard versions are required because existing snapshots and receipts are versioned artifacts. Their installer pins target Forge `0.1.1`; actual remote release publication remains user-controlled.
- Forge's own workflow currently has separate duplicate test-bearing Gate checks; this package does not change them because they are a separate hosted job graph concern.
- UI/UX is terminal-only; no browser route is affected, so browser/device verification does not apply.
