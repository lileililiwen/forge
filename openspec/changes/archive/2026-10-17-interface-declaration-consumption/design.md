# Design: Declaration-first plugin resolution and Forge's own interface block

## Ownership and boundaries

- The declaration format is owned by `workspace-governance`
  (`schemas/project.schema.json`). This change **reads** it and adds no field of
  its own to the format; Forge's own `.project.json` is data in that format.
- Forge remains a consumer of governance, not its authority: this change makes
  Forge read a declaration that already exists, and never writes one into another
  project.
- The versioned-surface inventory stays `CONTRACTS` (`src/contract/mod.rs:44`).
  The published declaration values are derived from it at build time rather than
  restated, so `forge contract list` and the declaration cannot disagree.
- The no-sibling invariant is preserved: a declaration is read from the project
  directory being inspected. Nothing is fetched, and an absent declaration is not
  an error.

## Resolution order and provenance

| Rank | Source | Meaning | Reported as |
|---|---|---|---|
| 1 | `providers.yaml` `plugins:` descriptor | operator configuration for this installation | `descriptor` |
| 2 | project declaration (`interfaces[]`, `capabilities`, `integrations`) | the provider's own statement of its boundary | `declared` |
| 3 | `kits/manifest.json` `plugins:` block | bundled kit metadata | `manifest` |
| 4 | builtin table (`src/plugins/mod.rs`) | last-resort fallback for an id nothing declares | `builtin` |

Invariants:

1. A lower rank never overrides a higher one. In particular `builtin` never wins
   over `declared`.
2. Every rendered row names its provenance, so a reviewer can see whether a
   capability claim came from a declaration or from a guess compiled into Forge.
3. A contradiction between rank 1 and rank 2 is a typed refusal naming both
   values and the id; it is never resolved silently. Rationale: an operator
   descriptor is deliberate local configuration, and a provider declaration is a
   deliberate statement — when they disagree, one of them is wrong and only a
   human can say which.
4. An id nothing declares still resolves through rank 4, unchanged. This change
   adds a source; it removes no capability.

## Declaration reading

- Read `interfaces[]` when present. For plugin kind/capability resolution, use
  the entries the project declares as `provides`; the mechanism does not change a
  plugin's kind (a plugin is still `gate`/`quality`/`agent`/`contract`/
  `delivery`/`metadata`), it only makes the claim attributable.
- Read `capabilities` and `integrations` for the existing projections
  (`contract::emit_capability`, `contract::emit_declaration_vocabulary`) — no
  behaviour change.
- Refusals, each typed and naming the offending field:
  - declaration present but unparseable → refuse, naming the file and the parse
    error;
  - `mechanism` outside the closed set → refuse, naming the accepted set;
  - non-`sibling` interface with a version-less `contract` → refuse;
  - `consume`/`observe` interface with no `digest` → refuse.
  An empty or absent `interfaces` array is valid and silent.

## Forge's own declaration

`forge/.project.json` gains an `interfaces[]` block. Values derived from
`CONTRACTS` and the adapter contract constants rather than retyped:

| Interface | Direction | Mechanism | Contract |
|---|---|---|---|
| `gate-invocation` | provides | invoke | `platform.gate-result/1.0.0` |
| `gate-evidence-export` | consumes | invoke | `forge-gate-evidence-export` (driftwatchdog) |
| `checker-document` | provides | observe | `forge-checker/0.1.0` |
| `publish-provider` | consumes | invoke | `forge-publish-provider/0.1.0` |
| `deploy-executor` | consumes | invoke | `forge-deploy-executor/0.1.0` |
| `governance-observation` | consumes | invoke | `forge-governance-provider/0.1.0` |
| `contract-envelope` | provides | observe | `platform.*` families (`src/contract/mod.rs`) |
| `platform-schemas` | consumes | consume | `platform-contracts@<revision>` + digest |
| `platform-kit-feed` | consumes | consume | `dotnet-platform-libs@0.2.0` + digest |
| `governance-checkers` | consumes | consume | `workspace-governance` checker bytes + digest |
| `agent-runtime` | consumes | invoke | `ariadex`/`sisyphusfy` (no version today — declared as such, which is the finding this change makes visible) |
| `mcp-surface` | provides | invoke | `MCP_CONTRACT_VERSION` |

Where a contract version does not exist today (the two agent runtimes), the
declaration records the interface **without** a version and the governance audit
reports it — Forge does not invent a version to look complete.

## Failure behaviour

- Unparseable or invalid declaration: typed refusal naming file and field; the
  command exits non-zero and reports nothing as resolved from that declaration.
- Contradiction between descriptor and declaration: typed refusal naming both
  values; no row is rendered for that id.
- Absent declaration: rank 4 fallback, row reported as `builtin`, exit unchanged.
- Unreadable project directory: unchanged from today (the existing
  project-resolution refusals apply before declaration reading).

## Verification oracle

- A project declaring `driftwatchdog` as `gate[gate]` resolves as `declared`, with
  the same kind and capabilities the builtin table would have produced — proving
  the declaration path is behavioural-equal, not merely accepted.
- Removing the declaration returns the row to `builtin`, with identical kind and
  capabilities — proving no capability was lost.
- A contradicting descriptor + declaration pair refuses by name.
- An unknown mechanism, a version-less contract on a non-sibling interface, and a
  `consume` entry without a digest each refuse with the field named.
- `forge plugins list` shows a provenance column; `forge cap list` counts are
  unchanged for the same set of ids.
- `forge contract list` and the declaration agree on every contract version: a
  test asserts the derived values equal the `CONTRACTS` table.
- The full workspace suite, `cargo fmt --check`, `cargo clippy -D warnings`, and
  the local gate stay green; the only expected diff is the added provenance field
  in plugin rows and the new declaration file block.
