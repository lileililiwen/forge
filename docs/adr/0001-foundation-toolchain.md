# ADR 0001: Foundation toolchain and persistence baseline

Date: 2026-09-16
Scope: `core-manifest-registry` (Roadmap order 1, target v0.1)
Status: accepted; planning context below is historical — the toolchain it records is implemented and validated in tree (see HANDOFF verification evidence). The historical wording is retained, not rewritten.

## Context

Forge is planning-only with no application source. The brief
([requirement.md](../../requirement.md) §37) recommends Rust for Core/CLI,
SQLite for the initial registry, YAML manifests and stdio MCP, without
pinning versions. [docs/architecture.md](../architecture.md) requires a
modular monolith with shared Core rules, thin transports, versioned
contracts and generated projects that work without Forge. This ADR records
the concrete choices before implementation so later changes inherit them
instead of re-deciding them.

## Decisions

### 1. Language and edition

Rust stable (validated with 1.98.1), edition 2021. Rationale: filesystem
work, process management, CLI ergonomics, single static binary, fast
startup — matching the brief's recommendation. Product contracts stay
stack-neutral: manifests are YAML, registry rows map to plain structs,
transports render Core outcomes without reinterpreting them.

### 2. Crate boundaries

Single Cargo package `forge` with a library plus a thin binary:

```text
src/lib.rs                  # Core + Registry public contracts
src/core/mod.rs             # error codes, shared types
src/core/manifest.rs        # schema-versioned forge.yaml parsing/validation
src/registry/mod.rs         # persistent project registry + operation journal
src/main.rs                 # CLI transport only (clap wiring, output rendering)
```

Core returns typed outcomes (`Result<T, ForgeError>`); the CLI maps them
to human text or JSON and exit codes. Later MCP/API transports must reuse
the same Core entry points.

### 3. Dependency pins (minimum versions, Cargo.lock committed)

- `clap` 4 with `derive`: CLI parsing, `--help`/`--version` generation.
- `serde` 1 with `derive`, `serde_yaml` 0.9, `serde_json` 1: manifest
  parsing (YAML) and machine-readable output (JSON).
- `rusqlite` 0.32 with `bundled`: SQLite without a system dependency
  (no `sqlite3` CLI or shared library required on the host).
- `thiserror` 2: typed Core errors with stable machine codes.
- `chrono` 0.4: UTC timestamps for observations and journal entries.
- dev: `tempfile` 3 for registry/manifest fixtures.

### 4. Manifest schema and parser

- `forge.yaml` is canonical. `platform.yaml` is accepted only as an
  explicit legacy import source (`--manifest platform.yaml`); when both
  files exist and no explicit source is given, loading fails with
  `ambiguous-manifest` and neither file is modified.
- Only schema version `1` is supported. Any other `schema` value fails
  with `unsupported-schema`; Forge never rewrites or downgrades it.
- Required: `schema`, `project.id`, `project.name`, `project.profile`.
  `project.id` must match `^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$`.
- `project.maturity` / `project.target_maturity` accept
  `L0`..`L4` when present. `runtime`, `features` (name → version map),
  `quality`, `ai`, `deployment`, `distribution` and `docs` sections are
  optional; absent future-integration sections mean "no enabled
  integrations", never an error.
- Manifest loads are read-only in this change: validation never mutates
  either manifest file. Atomic manifest writes are deferred to the first
  change that mutates manifests.

### 5. SQLite strategy

- One file database. Resolution order: `--registry <path>` flag,
  `FORGE_REGISTRY` environment variable, otherwise
  `$XDG_DATA_HOME/forge/registry.db` or
  `~/.local/share/forge/registry.db`. Tests use temporary paths.
- `rusqlite` bundled build; `busy_timeout` of 5s; single-writer updates
  inside explicit transactions. No shared Gate Runtime interaction.
- Tables: `projects` (one row per project, `id` unique, canonical
  absolute `path` unique, manifest-derived columns plus nullable
  observation columns where NULL means `unknown`) and `operations`
  (append-only journal: `pending` → `done`/`failed` with timestamps).
- Reconciliation rule: manifest files and the database can never share
  one transaction. `register` therefore writes the database only, in a
  single transaction that first records a `pending` journal entry and
  then the project row plus the terminal journal state. A stale
  `pending` entry (previous interrupted run) is marked `failed` by
  `reconcile_journal()` on open and is never reported as success.
- Observations: `runtime_status`, `last_commit`, `quality_status`,
  `agent_status`, `docs_status` are nullable. Missing path or missing
  observation renders as `unavailable`/`unknown`, never `healthy`.
  Git remote/commit capture is best-effort at register time; failure to
  read Git leaves `unknown` rather than failing registration.

### 6. CLI contract (this change)

`forge --help`, `forge --version`, `forge list`, `forge inspect
<ID-or-path>`, `forge register [path]`. All other v0.1 commands (`new`,
`import`, `doctor`) belong to later roadmap changes and are not stubbed
here. Errors print `error[CODE]: message` to stderr (JSON object with
`--format json`); exit `0` on success, `1` on operational failure,
`2` on usage failure.

## Consequences

- Later changes (`profile-registry`, `project-import`, …) inherit the
  toolchain, registry location rules and journal without re-deciding.
- PostgreSQL migration stays conditional on demonstrated need.
- Portal stack (ASP.NET Core / Next.js) remains deferred.

## Verification

`cargo fmt --check`, `cargo build`, `cargo test`, `cargo clippy -- -D
warnings`, `node scripts/check-openspec-change-names.mjs`,
`openspec validate --all --strict --no-interactive`, `git diff --check`.
