# cli-facade Specification

## Purpose
Thin `forge use <product>` facade that resolves a consolidated or external product binary and delegates to it in the caller's working directory, returning the product's exit code unchanged. Forge adds no AI and performs no rewriting; it only routes, so products keep their own accuracy and maturity evidence. Discovery surfaces are `forge use --list` and `forge help`.
## Requirements
### Requirement: CF-001 `forge use` resolves and delegates to a product binary

Forge SHALL provide a `use` subcommand that accepts a product id and forwards the remaining command-line arguments verbatim to that product's own binary, executing it in the caller's current working directory. Forge SHALL return the product process's exit code unchanged and SHALL perform no rewriting, interpretation, or AI transformation of the product's input or output. Forge SHALL NOT re-implement product behavior; it acts only as a resolver and delegator.

#### Scenario: CF-001-S1 Delegation preserves arguments and exit code

- **WHEN** a user runs `forge use driftwatchdog -- list`
- **THEN** Forge locates the `driftwatchdog` binary, spawns it with the argument `list`, and returns its exact exit code (0 for a successful listing)

#### Scenario: CF-001-S2 Flags are forwarded without interpretation

- **WHEN** a user runs `forge use cargo-sonar -- --help`
- **THEN** the `--help` flag is passed verbatim to the `cargo-sonar-boost` binary and its help text is printed unchanged

#### Scenario: CF-001-S3 No AI rewriting of product output

- **WHEN** a product writes structured or free-text output to stdout/stderr
- **THEN** Forge streams it through inherited stdio without modification and applies no AI summarization

### Requirement: CF-002 In-repo Rust crate resolution

For a product cataloged with kind `rust`, Forge SHALL resolve its binary from the workspace build output, relative to the running `forge` executable: `<workspace-root>/target/debug/<binary>` if present, otherwise `<workspace-root>/target/release/<binary>`. The workspace root SHALL be derived by walking up from the `forge` executable to the directory containing `Cargo.toml`. If neither built binary exists, Forge SHALL refuse with a message naming `cargo build -p <crate_name>` (or `cargo build --workspace`) and exit non-zero, and SHALL NOT fall through to PATH or network resolution.

#### Scenario: CF-002-S1 Debug binary preferred, then release

- **WHEN** both `target/debug/driftwatchdog` and `target/release/driftwatchdog` exist
- **THEN** Forge resolves and runs the debug binary

#### Scenario: CF-002-S2 Missing binary is refused with a build hint

- **WHEN** a `rust` product's binary is absent from both `target/debug` and `target/release`
- **THEN** Forge prints `product '<id>' is not built. Run cargo build -p <crate_name> ...` and exits 2, and does not attempt PATH or npx

#### Scenario: CF-002-S3 Resolution works from any working directory

- **WHEN** `forge use` is invoked from a directory other than the workspace root
- **THEN** Forge still locates the binary via the executable-relative workspace root

### Requirement: CF-003 External product resolution and npx restraint

For a product cataloged with kind `external`, Forge SHALL first resolve its binary from `PATH`; if absent, it SHALL attempt `npx -y <binary>`; if `npx` is unavailable, it SHALL refuse with an install hint (e.g. `npm i -g <binary>`). An arbitrary product id that is neither cataloged nor present on `PATH` SHALL be refused with the known-product list and SHALL NOT trigger `npx`, so a typo never causes a network install. Forge SHALL support aliases so a user may type either the canonical id or an alias (e.g. `grbk` for `gitrunbykaru`).

#### Scenario: CF-003-S1 External product on PATH is used directly

- **WHEN** `gitrunbykaru` is installed on `PATH`
- **THEN** Forge resolves and runs the PATH binary without invoking npx

#### Scenario: CF-003-S2 npx fallback only for cataloged externals

- **WHEN** `gitrunbykaru` is cataloged as `external` but absent from `PATH` and `npx` exists
- **THEN** Forge runs `npx -y gitrunbykaru` with the forwarded arguments

#### Scenario: CF-003-S3 Unknown id never triggers npx

- **WHEN** a user runs `forge use does-not-exist`
- **THEN** Forge lists the known products and exits 2, and performs no network install

#### Scenario: CF-003-S4 Alias resolves to the canonical binary

- **WHEN** a user runs `forge use grbk`
- **THEN** Forge resolves `gitrunbykaru` and delegates to it

### Requirement: CF-004 Discovery surfaces `use --list` and `help`

Forge SHALL provide `forge use --list` enumerating every cataloged product with its kind, and a `help` subcommand that prints the facade overview (resolution order, product catalog, and examples). Both surfaces SHALL read from the same in-code product catalog `PRODUCTS` in `src/cli/use_cmd.rs`, so adding a product in one place updates both. JSON format errors SHALL be emitted as `{"error":{"code":"product-not-found","message":"..."}}` on stderr.

#### Scenario: CF-004-S1 List shows cataloged products

- **WHEN** a user runs `forge use --list`
- **THEN** Forge prints each product id and its kind (e.g. `driftwatchdog  [rust]`)

#### Scenario: CF-004-S2 Help prints the facade overview

- **WHEN** a user runs `forge help`
- **THEN** Forge prints resolution order, the product list, and usage examples

#### Scenario: CF-004-S3 JSON error shape

- **WHEN** an unknown product is requested with `--format json`
- **THEN** Forge emits `{"error":{"code":"product-not-found","message":"..."}}` on stderr and exits 2
