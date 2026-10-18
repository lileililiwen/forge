# Proposal: Align local and hosted Gate profiles

## Why

Generated pre-commit hooks currently run every declared Gate extension. This can repeat expensive builds and test suites on each commit. Hosted CI also runs the full Gate, while Forge's own workflow has named checks that overlap the Gate's `tests` and `declared-verification` checks. The developer cannot see from a hook template which checks are baseline requirements and which are extended verification.

## What Changes

- Add explicit `fast` and `full` profiles to `forge gate`. Both profiles always execute Forge's mandatory in-process `source-file-size` baseline. `fast` omits project-declared extension checks; `full` runs all declared checks and remains the default.
- Record the selected profile in Gate evidence and its JSON projections. Missing profile in legacy evidence means `full`, since older Forge versions only ran the full configured plan.
- Require fresh passing `full` evidence for delivery preflight. Fast evidence remains useful locally but cannot approve delivery or be uploaded as hosted full-verification evidence.
- Update generated pre-commit templates to use `fast`; generated CI continues to run `full`. Add new immutable service/client standard snapshots and make them the new-project defaults.
- Update the CI/CD model with profile semantics, evidence handling and explicit remaining measurement limitations.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `gate-local-hosted-profiles` | Make the local hook fast while preserving the mandatory baseline and full hosted/delivery verification | Forge, Rust CLI/Core plus versioned shell/YAML standard snapshots | `forge gate --profile fast|full`; Gate evidence profile; immutable baseline-service/client snapshots | Existing `gate-runtime-evidence` and `standard-pack-registry-and-snapshots` | CLI contract tests prove check selection, evidence tier and delivery refusal; snapshot tests prove hook/CI parity and generated defaults |

This is one acceptance journey: the standard hook invokes the Forge profile, the same runtime records its scope, and downstream approval accepts only full evidence. Splitting runtime/evidence from the generated templates would leave an unsafe or unusable intermediate state. Forge's own duplicate test runs are a separate CI workflow concern and remain a later gap.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge bundled Gate | `crates/driftwatchdog/src/gate/manifest.rs::resolve`; `src/gate/engine.rs`; `openspec/specs/gate-runtime-evidence/spec.md` | In-process mandatory source-size baseline, extension plan, bounded execution and Forge-owned revision-bound evidence | Current public Forge invocation has no selection profile or evidence tier | Forge owns CLI contract and the vendored runtime; versioned `forge` release | **keep local** |
| Forge baseline packs | `src/standard/layout.rs`; `standards/baseline-service/1.5.0`; `standards/baseline-client/1.2.0` | Pinned generated CI, pre-commit, installer and hosted-evidence assets | Existing immutable snapshots call the full Gate from local hooks | Forge standard registry; new immutable pack versions | **keep local** |
| Platform Gate result contract | `contracts/`, `src/contract/`; `platform.gate-result/1.0.0` | Portable aggregate/check summary | Does not encode Forge's evidence profile or project delivery authority; changing it would unnecessarily alter the shared contract | Forge consumes the published contract; no sibling edits in this change | **adapt through a generic adapter** |

No sibling repository provides a compatible fast/full evidence tier. Forge keeps this policy local; other products consume generated snapshots and do not depend on Forge source or runtime during their own native builds.

## User Experience and Interface Impact

The developer's entry points are the pre-commit hook and `forge gate [TARGET] --profile fast|full`. The default remains `full` for direct/manual invocations and existing CI. The generated hook names its fast baseline run; on failure, Forge prints the blocking check and states that full CI runs the configured extensions. `--dry-run` displays the selected profile and planned checks without creating evidence. JSON evidence includes the profile. Delivery refusal identifies fast evidence and instructs the user to run `forge gate --profile full`.

This is a terminal workflow; no browser route or portal layout changes. Keyboard behavior and screen-reader output remain terminal-native, with profile and failure details in text and JSON. The verifier oracle is CLI/hook execution, not a browser test.

## BFS Impact Map

| Surface | Impact and required outcome |
|---|---|
| Gate planning/execution | Fast selects only the mandatory Forge baseline; full selects baseline plus all valid declared checks. Malformed declarations still fail closed for either profile. |
| CLI | Add validated profile option; reject incompatible use with status/evidence subcommands; retain `full` as default. Dry run is profile-aware. |
| Evidence and consumers | Persist profile; old evidence without the field is interpreted as full. Status/API/portal projections preserve it. Delivery and hosted upload accept only full. |
| Standard packs/generation | Add immutable service/client versions whose hooks use fast and CI uses full; preserve historical snapshots; advance defaults after contract and template tests. |
| Forge CI | Do not run the same test suite twice within Gate. Preserve standalone workflow test jobs and all distinct Gate baseline/security/governance requirements. |
| Data/persistence | No database migration; evidence JSON gains an optional backward-compatible field. |
| Failure/boundaries | Unknown profile, malformed gate config, missing Forge binary, timeout and failed baseline remain blocking; fast evidence cannot satisfy full-only consumers. |
| Verification | Unit/CLI tests, delivery and hosted-evidence contracts, immutable pack/provenance tests, generated asset checks, strict OpenSpec, and local Gate. |
| Siblings/shared ownership | No sibling edits. Versioned snapshots are the portable adoption boundary; `platform.gate-result` remains unchanged and is reached only through an adapter. |
| Security/privacy | Preserve argv-based CLI parsing, existing bounded check execution and credential redaction; profile never disables the in-process baseline or changes extension command authorization. |

## Capabilities

- `gate-runtime-evidence`: profile-aware planning, execution evidence and full-only approval.
- `standard-pack-registry-and-snapshots`: generated hooks select fast and hosted workflows select full in new immutable snapshots.

## Non-goals

- Do not change sibling repositories, existing generated projects, or archived standard snapshots.
- Do not make project extensions mandatory in the fast hook; full CI remains authoritative for extensions.
- Do not change Forge's own CI job graph or the duplicate test-bearing Gate checks in this package.
- Do not change the shared `platform.gate-result/1.0.0` contract, release policy, provider behavior, or CI status ingestion trust model.
- Do not claim runtime improvement from estimates; record local timing evidence for the Forge repository and define that generated-project timing needs consumer measurements.
