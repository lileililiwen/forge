# Proposal: Projects declare their interfaces, and the audit charges an undeclared edge

## Why

The portfolio has a boundary language for *capabilities* and none for
*interfaces*, so every cross-project call is invisible to governance.

Measured on 2026-10-17 across 82 declarations and 40 cross-project edges:

- `.project.json` can declare `capabilities.<name>` (`owner`, `evidence_state`,
  `evidence_ref`, `consumes[]`), `integrations[]` (`id`, `required`,
  `evidence_ref`), `declines[]`, `adoption_grant`, `standalone`, `publication`
  and `release_evidence` — but it cannot say **how** anything is reached.
  Nothing declares a mechanism, an entry point, or a contract version.
- Adoption of what exists is 1–3 %: `capabilities` 3 of 82, a capability
  carrying `consumes[]` 1, `integrations[]` 2, `declines[]` 1, `adoption_grant`
  0.
- The registry cannot charge anyone: all **9** entries in `capabilities.json`
  are `posture: exempt`, `reason: provider-unavailable`, all nine hosted by
  `dotnet-platform-libs`.
- Meanwhile cooperation happens over unstandardised mechanisms. The largest is
  67 `.ai-gate/gate.yaml` files binding
  `python3 ../workspace-governance/scripts/workspace_check.py --root .. --project X --json`
  — no contract version, no digest, and only resolvable when the sibling
  checkout sits at `../workspace-governance`. That generator is this
  repository's own: `scripts/bind_gate_commands.py:1116-1124`.
- Contract versions exist on both sides for only four edges
  (`driftwatch-checker/0.1.0`, `platform.gate-result/1.0.0`,
  `forge-deploy-executor/0.1.0`, `forge-publish-provider/0.1.0`); four more have
  one side or none (`forge-delivery-hermora/0.1.0` names an adapter binary that
  exists in neither repository; `ariadex`, `sisyphusfy`, the docs translator and
  the studio runner declare nothing); and every generated gate binding has none.

Three consequences follow directly:

1. **A boundary cannot be reviewed.** Whether a project obeys the workspace's
   own rule — `AGENTS.md:48`, "A checker must not depend on an environment
   variable being set… Prefer a committed, repo-relative input" — is not
   checkable, because no declaration states what the project reaches for.
2. **Duplication is invisible.** Three copies of the secret-field vocabulary,
   two capability vocabularies with zero overlap, and Forge's hardcoded plugin
   table all read as legitimate because nothing contradicts them.
3. **An undeclared edge is never a finding.** The audit already reports
   `CAPABILITY_ADOPTION_UNRESOLVED`, `ADOPTION_GRANT_UNSATISFIED`,
   `STANDALONE_ABSOLUTE_PATH` and `STANDALONE_PATH_ESCAPE` — the vocabulary for
   charging a boundary exists; the input it would charge does not.

This repository owns `schemas/project.schema.json` and the audit, so the
interface vocabulary belongs here: decision **D3** splits the capability
*entry schema* to `platform-contracts` and the *instances* to this repository,
and an interface block inside the project declaration is an instance.

## What Changes

- **`schemas/project.schema.json` gains `interfaces[]`.** Each entry declares one
  reachable boundary of the project: a stable `id`, whether the project
  `provides` or `consumes` it, the `mechanism` (closed set: `invoke`, `consume`,
  `observe`, `sibling`), the `entry` (command, URL, or repository-relative path),
  the `contract` identifier with version, an optional `digest`, and `required`.
  A `consumes` entry names its `provider`; the provider must declare a matching
  `provides` entry.
- **The audit validates declarations and charges undeclared edges.** New
  findings, declared with stable codes in the existing vocabulary:
  `INTERFACE_DECLARATION_INVALID`, `INTERFACE_MECHANISM_UNKNOWN`,
  `INTERFACE_CONTRACT_UNVERSIONED`, `INTERFACE_PROVIDER_UNDECLARED`,
  `INTERFACE_CONSUMED_UNUSED`, `INTERFACE_SIBLING_PATH_UNPINNED`,
  `INTERFACE_VENDORED_WITHOUT_DIGEST`.
- **The mechanism rule becomes checkable.** `mechanism: sibling` — the
  relative-path invocation that 67 gate files use today — is lawful only when
  the consuming project also declares the provider `required: true` **and** the
  edge is digest-pinned; otherwise the finding is
  `INTERFACE_SIBLING_PATH_UNPINNED` with the remediation naming the two lawful
  alternatives (a digest-pinned vendored asset, or a published artifact).
- **A vendored copy becomes lawful only with provenance.** Any byte copy of
  another project's artifact that a project keeps must be declared as
  `mechanism: consume` with its `contract`, `version` and `digest`; a copy
  without a digest is `INTERFACE_VENDORED_WITHOUT_DIGEST`. This makes the two
  existing copies in Forge (`scripts/governance/product_code_quality.py`,
  `scripts/vocabulary.json`) lawful by declaration instead of by omission.
- **The registry unfreeze precondition is stated.** A capability entry stays
  `exempt / provider-unavailable` until its provider declares a `verified`
  entry; the audit reports the unfrozen blockers
  (`CAPABILITY_PROVIDER_UNVERIFIED`) so the nine current exemptions have a named
  owner and a visible next action instead of silently absorbing every consumer.

## BFS Impact Map

| Surface | Impact |
|---|---|
| `schemas/project.schema.json` | New `interfaces` property and `interface*` `$defs`; additive and optional, so 82 existing declarations stay valid unchanged |
| `scripts/workspace_check.py` | Declaration validation plus the seven findings; the audit already joins registry + manifest + declaration and gains a fourth input |
| `capabilities.json` | No entry changes here; the exemption rule gains a reporter |
| `scripts/bind_gate_commands.py`, `bind_ui_gate.py`, `bind_verification_command.py` | Must emit `mechanism: sibling` declarations (or a declared `consume`) alongside the commands they already write, so the fleet's 67 existing bindings become declared rather than unlawful |
| `docs/capabilities.md`, `docs/` | The declaration model is documented with the mechanism set and the two lawful alternatives to a sibling path |
| Tests | `tests/test_workspace_check*.py` gains declaration/finding cases; the schema change is exercised against a fixture declaring each mechanism |
| Consumers | Forge reads the same block (its own change, `interface-declaration-consumption`); no consumer is forced to declare |

## Capabilities

- `platform-capability-manifest` (extended): the declaration is no longer intent
  only; it can name an interface and its contract.
- `capability-adoption-gate` (extended): findings for undeclared, unversioned and
  unpinned edges, plus the provider-unverified reporter that unfreezes the
  registry.

## Non-goals

- No new registry. The registry of interfaces is the declarations themselves;
  this change adds no second list to keep in sync.
- No mechanism is blessed beyond the four; in particular no new environment
  variable is introduced as an escape, and no absolute path becomes lawful.
- No consumer is required to declare in this change. A project with no
  `interfaces[]` reports nothing; a project that declares one is held to it.
- No change to the gate runtime, its vocabulary, or `driftwatchdog`'s
  execution semantics.
- No change to `capabilities.json` postures by hand; the nine exemptions lift
  only when their provider declares verified evidence.
- The wire envelope for interfaces travelling *between* systems
  (`platform.*` families) stays with `platform-contracts`.
