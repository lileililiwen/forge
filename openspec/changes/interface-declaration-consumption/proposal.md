# Proposal: Forge reads declared interfaces instead of a hardcoded builtin table

## Why

Forge keeps a second, private description of the workspace's boundaries, and it
cannot see the declarations the workspace already owns.

- `src/plugins/mod.rs:313-341` resolves a plugin's kind and capabilities from a
  table **written in Rust**: `driftwatchdog=gate`, `cargo-*=quality`,
  `sisyphusfy|ariadex|mnemora=agent`, `platform-contracts=contract`,
  `labrys|openpanel|jenkins-local=delivery`, `argoscope|devloom=metadata`. The
  capability set (`src/plugins/mod.rs:44`, 11 values) and the kind set (`:63`, 6
  values) are likewise compile-time constants. The only input it consults first
  is the optional `kits/manifest.json` `plugins:` block.
- The workspace declares the same facts elsewhere. `workspace-governance` owns
  `schemas/project.schema.json` with `capabilities`, `integrations[]`,
  `declines[]` — and its change
  `interface-declaration-and-boundary-findings` adds `interfaces[]` (mechanism,
  entry, contract version, digest). Measured adoption of what already exists is
  1–3 % of 82 declarations, and Forge reads none of it: `src/contract/mod.rs:837`
  reads only `capabilities` to project a contract envelope.
- So Forge's registry, the workspace registry and `platform-contracts`'
  capability schema describe the same boundaries three times. A provider that
  changes its capability list must be edited in Rust, and nothing reports the
  disagreement.
- Forge also holds copies it does not declare: `scripts/governance/product_code_quality.py`
  and `vocabulary.py` (byte-identical to `workspace-governance/scripts/*`),
  `contracts/` (a digest-pinned vendored copy of `platform-contracts`, pinned in
  `contracts/manifest.json`), and `contracts/vocabulary/secret-field-substrings.json`
  — the **third** copy of a vocabulary whose other copies live in
  `workspace-governance/vocabulary.json` and `platform-contracts/schemas/registry.json`.
  The digest-pinned ones are lawful once declared; the third copy is not
  distinguished from a consumption edge anywhere.

Two concrete costs of the current state, both measured: a provider's capability
change requires a Forge source edit and a release, and a reviewer reading
`forge plugins list` cannot tell a declaration from a builtin guess.

## What Changes

- **The declaration becomes the registry source.** Forge SHALL resolve a plugin's
  kind and capabilities from the workspace declaration when one exists, and SHALL
  report which source answered (`descriptor`, `declared`, `manifest`,
  `builtin`). The hardcoded table survives only as a last-resort fallback for an
  id nothing declares, and SHALL never override a declaration or a descriptor.
  A declared value that contradicts a configured `providers.yaml` descriptor
  SHALL be a typed refusal naming both, not a silent precedence.
- **A declaration Forge cannot read is refused, not ignored.** An unknown
  mechanism, a version-less contract on a non-sibling interface, or a
  declaration that fails to parse SHALL produce a typed refusal naming the field,
  so a malformed boundary is never treated as an absent one.
- **Forge declares its own interfaces.** `forge/.project.json` gains an
  `interfaces[]` block enumerating what Forge exposes (gate invocation, publish
  provider, deploy executor, contract emit, agent runtimes, governance provider,
  MCP surface, HTTP API) and what it consumes (driftwatchdog's gate and check
  protocols, the platform-contracts schemas, the kit feed), each with its
  mechanism, entry and contract version. The values SHALL be derived from the
  existing versioned-surface inventory (`CONTRACTS`, `src/contract/mod.rs:44`)
  rather than a second hand-maintained list, so the two cannot drift.
- **Vendored copies become declared consumption edges.** Forge's retained copies
  (`scripts/governance/*`, `contracts/**`, `kits/**`) SHALL each be named as a
  `consume` interface with provider, contract, version and the digest already
  recorded in `contracts/manifest.json` / `kits/manifest.json`. The third copy
  of the secret-field vocabulary SHALL be reduced to one declared edge against
  its owner's artifact rather than an undeclared sibling copy.

## BFS Impact Map

| Surface | Impact |
|---|---|
| `src/plugins/mod.rs` | Resolution order gains the declaration source; `builtin_hardcoded` becomes fallback-only; provenance is reported per row |
| `src/contract/mod.rs` | `CONTRACTS` stays the single versioned-surface inventory and feeds the published declaration values |
| `.project.json` (Forge's own) | New `interfaces[]` block; no other project file changes |
| `src/cli/plugins.rs`, `src/cli/cap.rs` | Render the provenance column and the refusal text |
| `forge docs/architecture.md`, `docs/provider-evidence.md` | Boundary tables updated to name the declaration as the source |
| `contracts/manifest.json`, `kits/manifest.json` | Unchanged as digest sources; their entries are now referenced by declarations |
| Tests | New cases: declared row wins over builtin, descriptor wins over declaration, contradicting pair refuses, unknown mechanism refuses, absent declaration falls back and reports `builtin` |
| Callers | `forge plugins list`, `forge cap`, the API shipping-reads section, and any test asserting the builtin mapping keep working; a row that used to say `builtin delivery plugin` may now say `declared` |

## Capabilities

- `forge-publish-plugin-orchestration` (modified): the builtin-resolution
  requirement becomes declaration-first with reported provenance.

## Non-goals

- No new registry in Forge and no new document format: the declaration is the
  workspace's `.project.json`, whose schema `workspace-governance` owns.
- No requirement that any sibling declares anything: an unregistered provider id
  keeps resolving through the fallback exactly as today.
- No change to adapter invocation, contract envelopes, the MCP surface, the gate
  or the publish pipeline — only to how a plugin's kind and capabilities are
  resolved and reported.
- No deletion of the digest-pinned vendored copies: they are what makes Forge run
  with no sibling checkout, and this change makes them declared instead of
  anonymous.
- No dependency on `workspace-governance` at run time: a declaration is read from
  a project directory, not fetched from a sibling.
