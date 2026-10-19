# Facade: `forge use <product>`

The outer Forge workspace is a *thin* control plane. Its job is **not** to
re-implement the consolidated dev tooling, but to **route** the user to each
product's own binary and let it run in the user's current directory ("relative
use"). Forge adds no AI and performs no rewriting — it only locates the right
binary and delegates. Because the product's own code runs, the product keeps its
native accuracy and any maturity evidence it produces is first-hand.

## Why a facade

The consolidation moved the sibling crates (`driftwatchdog`,
`cargo-audit-boost`, `cargo-matrix-boost`, `cargo-sonar-boost`) into
`crates/*` so Forge is the single home for dev tooling. But each crate is
still a standalone product with its own CLI and (for the npm sibling) its own
publish path. Rather than duplicating their surface inside `forge` and risking
drift, `forge use` is a stable, tiny dispatcher:

- **In-repo products** are resolved from the workspace build output
  (`target/{debug,release}/<binary>`), relative to the running `forge` binary,
  so it works from any working directory.
- **External products** (e.g. an npm package such as `gitrunbykaru`) resolve
  from `PATH`, then `npx -y <product>` — but only for products *cataloged* as
  external, so an arbitrary typo never triggers a network install.

## Usage

```sh
# Run a product in the current directory, forwarding every argument.
forge use driftwatchdog -- --help
forge use cargo-audit-boost -- .
forge use cargo-sonar -- report        # `cargo-sonar` is an alias for cargo-sonar-boost

# External npm sibling (resolved from PATH, then npx -y).
forge use gitrunbykaru -- --help
forge use grbk -- run                   # `grbk` is an alias for gitrunbykaru

# Discovery.
forge use --list                        # routable products
forge help                              # facade overview
```

`forge use <product> -- [ARGS...]` forwards `ARGS` verbatim to the product's
binary (the `--` separator and `allow_hyphen_values` let you pass flags that
start with `-`). The product's own exit code is returned unchanged, so scripts
that already depend on `driftwatchdog`/`cargo-audit-boost`/etc. keep working
through `forge use`.

For local development runs, use the narrower runtime entry points instead of
the generic product facade: `forge run compose` delegates a full service graph
to Docker Compose, while `forge run local <PATH>` delegates one app directory
to an installed GitRunByKaru. This run path never uses the generic facade's
`npx` fallback. See [local runtime entry points](local-runtime-entrypoints.md).

## Catalog

Products are enumerated in `src/cli/use_cmd.rs` (`PRODUCTS`). Each entry has:

| field       | meaning                                                        |
|-------------|----------------------------------------------------------------|
| `id`        | the name you type: `forge use <id>`                            |
| `aliases`   | alternative ids (e.g. `cargo-sonar` -> `cargo-sonar-boost`)     |
| `binary`    | the compiled binary name (also the npm bin name for externals) |
| `crate_name`| Cargo package name, used for the `cargo build -p` hint          |
| `kind`      | `rust` (vendored in-repo) or `external` (PATH/npx)             |
| `summary`   | one-line description for `forge help` / `forge use --list`      |

To add a product, append a `Product` to `PRODUCTS` and rebuild. In-repo crates
must already be members of the workspace (`members = ["crates/*"]` in
`Cargo.toml`); external entries only need a resolvable binary.

## Error behavior

- `forge use` with no product id prints a hint and exits `2`.
- A known `rust` product that is **not built** suggests
  `cargo build -p <crate_name>` (or `cargo build --workspace`) and exits `2`.
- An unknown id (not in the catalog and not on `PATH`) lists the known products
  and exits `2`; it never attempts `npx` for an unrecognized name.
- With `--format json`, errors are emitted as
  `{"error":{"code":"product-not-found","message":"..."}}` on stderr.

## Relationship to maturity

The facade is the *delivery* half of Forge's role as dev boss: it makes the
consolidated products trivially reachable. The *maturity* half is the products'
own evidence (Gate runs, audit/sonar reports) which `forge doctor`/`forge gate`
surface without re-running them. Routing and assessment stay separate on purpose.
