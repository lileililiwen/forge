# Proposal: Thin `forge use` facade over consolidated and sibling products

## Why

Forge consolidated the sibling dev-tooling crates (driftwatchdog, cargo-audit-boost, cargo-matrix-boost, cargo-sonar-boost) into `crates/*` so the workspace is the single home for dev tooling, and it may also route to external sibling products (e.g. the npm package gitrunbykaru). But each product keeps its own CLI, its own release path, and (for npm) its own publish path. Re-implementing their surfaces inside `forge` would duplicate behavior and drift from the products' own accuracy and maturity evidence.

The outer Forge workspace should therefore be a *thin* control plane: locate the right binary and delegate, running it in the user's current directory ("relative use"). No AI, no rewriting. This makes the consolidated products trivially reachable while preserving their first-hand evidence (Gate runs, audit/sonar reports) that `forge doctor`/`forge gate` surface.

## What Changes

- Add `forge use <product> [--] [ARGS...]` that resolves a product binary and delegates, returning its exit code unchanged.
- Add `forge use --list` and `forge help` discovery surfaces backed by one in-code catalog `PRODUCTS` in `src/cli/use_cmd.rs`.
- Resolve in priority order: (1) in-repo Rust crate binary under `target/{debug,release}/<binary>` (relative to the running `forge` executable); (2) a binary on `PATH`; (3) `npx -y <binary>` for cataloged `external` products only.
- Refuse a missing in-repo binary with a `cargo build -p <crate_name>` hint; refuse an unknown id with the known-product list and never run npx for an arbitrary name.
- Document the facade in README.md and docs/facade.md.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `cli-facade` | Forge resolves and delegates to consolidated/external product binaries with no AI rewriting | Forge; Rust 2021 CLI (`src/cli/use_cmd.rs`, `src/main.rs`) | Subprocess spawn with inherited stdio; in-code `PRODUCTS` catalog | Workspace `target/` build output; host `PATH`/`npx` | Each product's own binary runs unchanged; exit code and output are observable |

The resolver and the products meet only at the dispatch edge; the products remain independently built, tested, and (for npm) published. Splitting would mean either duplicating product behavior in Forge or losing the single reachable entry point.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Consolidated crates | `crates/{driftwatchdog,cargo-audit-boost,cargo-matrix-boost,cargo-sonar}/` built binaries in `target/` | Existing workspace build output; original crate names preserved | Forge had no single entry to reach them | Each crate owns its binary/release | **adopt** — resolve from `target/`, do not reimplement |
| gitrunbykaru (npm sibling) | `gitrunbykaru` bin on PATH or via `npx -y` | External package with its own MCP server | Forge should not vendor it | npm package owns publish | **route** — resolve via PATH/npx |
| `forge gate` / `forge doctor` | `src/gate`, `src/doctor` | Consume product evidence | No change; facade only dispatches | Forge | **keep separate** — routing vs assessment stay distinct |

## User Experience and Interface Impact

Actors are maintainers who want one command to reach any consolidated or sibling product from any directory. Entry point is `forge use <product>`, `forge use --list`, and `forge help`. The product's own CLI/exit/output are unchanged; Forge adds only resolution and a `--` passthrough. No new configuration file is introduced; the catalog lives in code.

## BFS Impact Map

| Area | Impact |
|---|---|
| Capability/state | New `cli-facade` delta; `use` and `help` subcommands; in-code `PRODUCTS` catalog |
| Desired configuration | None new; products already workspace members (`members = ["crates/*"]`) |
| Observed persistence | None; delegation is stateless |
| Runtime boundary | One subprocess spawn per `use`; inherited stdio; exit code returned |
| Attribution/filesystem | Walk up from `forge` executable to workspace root; read `target/{debug,release}/<binary>`; no write |
| External boundary | `PATH` lookup; `npx -y` for cataloged externals only; no implicit network install |
| Shared callers | `src/main.rs` dispatch; `cli/use_cmd.rs` resolver; README + docs/facade.md |
| Generation/import | None |

## Capabilities

- `cli-facade` (added): `forge use` resolver/delegator and discovery surfaces.

## Non-goals

- Re-implementing any product's behavior inside Forge.
- Adding AI summarization or rewriting of product output.
- Introducing a new configuration file for the product catalog (kept in code).
- Vendoring external npm siblings into the workspace.
