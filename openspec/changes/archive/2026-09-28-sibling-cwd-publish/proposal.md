# Proposal: sibling cwd publish via Forge

## Why

70+ sibling repositories (`alethefy`, `hypora`, `crossify`, `openpanel`, …) each own a `.project.json` identity and — when deployable — a `docker-compose.yml`, but Forge only publishes siblings that appear in a central inventory file (`forge publish fleet --inventory <path>`) or the legacy `workspace-governance/projects.json` handoff. A sibling cannot publish itself by running `forge publish` from its own checkout without first being registered centrally, inventing an inventory document, or knowing the `--folder`/`--project` plumbing. `jenkins-local` is Forge's publish plugin, not a sibling-facing API — siblings should never call `project-action.sh` directly — yet today the only documented single-project path forces the sibling to name its folder explicitly. The result is a central-inventory bottleneck: every adoptable sibling must be wired into Forge's repo before it can be delivered.

## What Changes

- Add cwd-discovered single-project publish: `forge publish` with no `--project`/`--folder` discovers the project from the current working directory (`.project.json` `id` > `forge.yaml` `project.id` > directory name), validates the id, captures `HEAD` as the 40-hex revision, and invokes the selected provider through the existing `forge-publish-provider/0.1.0` contract. `--folder` and `--project` remain as explicit overrides; a bare `forge publish` is a cwd shorthand, not a replacement.
- Keep the inventory fleet path unchanged (`forge publish fleet --inventory <path>` / legacy `--fleet-registry`) for batch publishing. Single-project cwd publish and fleet publish are two modes of the same engine, sharing provider selection, revision discipline, phase evidence (`revision`/`build_status`/`run_status`/`container_identity`), queue/journals, and `forge deploy status` projection.
- Preserve the plugin boundary: `jenkins-local` stays a Forge-side provider (`adapters/forge-publish-provider.py` → Mac Docker/Caddy/Cloudflare). Siblings configure only the Forge provider (`FORGE_PUBLISH_PROVIDER` / `.forge/providers.yaml`); Forge selects and invokes the plugin. No sibling calls `jenkins-local` scripts directly.
- Report missing runtime contracts explicitly: a sibling without a Compose file is `compose_missing` in `forge inventory show` / fleet reports, not silently dropped — later auto-scaffolding can remediate it without changing this package's scope.
- **BREAKING**: none. Bare `forge publish` previously failed with `publish requires --project or --folder`; it now succeeds when cwd discovery yields a valid project. All existing flags keep their semantics.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `sibling-cwd-publish` (this change) | A sibling checkout publishes itself through Forge via cwd discovery without central registration | Forge Rust; adapters external | `forge-publish-provider/0.1.0` + `.project.json`/`forge.yaml` identity; `FORGE_PUBLISH_PROVIDER` / `.forge/providers.yaml` | `forge-publish-plugin-orchestration`, `forge-independent-project-inventory-fleet` | CLI contract tests for cwd discovery + existing fleet tests unchanged |

Siblings own their `.project.json`, Dockerfile and Compose file. Forge owns cwd discovery, provider selection, bounded provider invocation, journaling, and status projection. Provider binaries and `jenkins-local` remain out of the sibling checkout — the sibling depends only on having `forge` on `PATH`.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge publish providers | `src/publish/providers.rs`, `src/main.rs::cmd_publish_provider` | External executable contract, bounded `1800s` invocation, secret redaction, phase evidence | No cwd discovery; requires explicit `--project`/`--folder` | Forge | **Extend shared owner** |
| Forge inventory fleet | `src/publish/inventory.rs`, `src/main.rs::cmd_publish_fleet`, `tests/inventory_contract.rs` | Versioned `forge-project-inventory/0.1.0`, `compose_ready`/`compose_missing`/`invalid`/`source_unavailable` classification, serial queue | Fleet requires an explicit inventory document; single publish bypasses it | Forge | **Reuse; fleet mode unchanged** |
| Workspace Governance | `.project.json` schema_version 1, `scripts/workspace_check.py` | 70+ sibling `.project.json` identities (profile, verification, deployment), provider-neutral ids | Siblings have no `forge.yaml`; Forge must read `.project.json` as primary identity | workspace-governance | **Consume read-only; do not depend on install** |
| jenkins-local | `adapters/forge-publish-provider.py`, `project-action.sh`, `deploy-all.sh` | Mac Docker, Compose identity `forge-<project>-<sha12>`, `providers.rs` phase vocab | Direct script use violates plugin boundary; sibling must go through Forge provider | jenkins-local | **Adapt through generic provider (no sibling change)** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| CLI | `forge publish` bare mode discovers cwd; `--folder`/`--project`/`--provider`/`--revision`/`--dry-run` keep semantics; `--cwd` optional explicit override for scripting |
| Identity | Cwd discovery reads `.project.json` `id` first, then `forge.yaml:project.id`, then directory name; validates kebab/snake, `1..=128` chars |
| Revision | Bare publish captures `git rev-parse HEAD` in discovered directory; must be 40-hex (same `validate_revision` gate); on failure reports `publish-invalid` before provider spawn |
| Provider | Selection unchanged (`FORGE_PUBLISH_PROVIDER` or `--provider`, config at `FORGE_PUBLISH_PROVIDER_CONFIG` or `<cwd>/.forge/providers.yaml`); `jenkins-local` stays the default Mac runtime behind the contract |
| Journal/status | Bare publish journals through the same `publish` kind, queue-id-optional path (`queue_id=None` for single), additive `revision`/`build`/`run`/`container_identity`; visible via `forge deploy status --project <id>` as today |
| Fleet | Unchanged: `forge publish fleet --inventory <path>` still classifies every entry; `forge inventory show <path>` still reports `compose_missing` explicitly |
| Failure | Ambiguous/missing identity, non-hex revision, unknown/disabled provider, missing compose (via inventory) all typed `publish-invalid`/`fleet-registry-invalid`; no silent omission or fallback scan |
| Unaffected | `workspace-governance` install/adapter, OpenPanel provider, GitHub push idempotency, `forge deploy status --queue --watch` queue mode |

## Capabilities

### New Capabilities

- `sibling-cwd-publish`: a sibling checkout publishes itself through Forge from its own directory via cwd discovery, using the existing provider contract and journal, without central inventory registration.

### Modified Capabilities

(none — `forge-publish-plugin-orchestration`, `forge-independent-project-inventory-fleet`, and `publish-commands` keep their requirements; this package adds a new cwd mode to the same CLI surface without changing their SHALLs)

## Non-goals

- Gating publish on OpenSpec/spec-first evidence (sibling workflow decision, not a Forge CI/CD precondition).
- Auto-generating `docker-compose.yml`/`Dockerfile` for the ~60 `deployable=false` siblings — this package reports `compose_missing` explicitly so a later scaffolding change can remediate it.
- Fleet parallelism, local Docker dev watch loop, or API publish routes — not required for cwd single-publish.
- Changing `jenkins-local` Mac routing, Caddy/Cloudflare wildcard, or workspace-governance registry shape.
- A shared completion Gate for this repository.

## Dependencies

- Requires `forge-publish-plugin-orchestration` (provider engine + `publish --project`/`--folder`) — archived.
- Requires `forge-independent-project-inventory-fleet` (portable inventory + fleet classification) — archived.
- No sibling provider implementation required; existing `jenkins-local` adapter is exercised as the optional live round-trip when present.
