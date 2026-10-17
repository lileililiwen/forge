# Design: Thin `forge use` facade

## Ownership

`src/cli/use_cmd.rs` owns the product catalog (`PRODUCTS`), resolver, delegation, and the `help`/`--list` rendering. `src/main.rs` owns the `Commands::Use`/`Commands::Help` variants and dispatch: `Use` returns an `ExitCode` directly via an early-return arm (like `Gate`/`Workspace`) because it streams the child's stdio and must return the child's exit code.

## Contract

`forge use <product> [--] [ARGS...]`
- `product`: catalog id or alias; optional at parse time (required at runtime, else error).
- `--list`: list cataloged products and exit.
- `args`: everything after `--` (or after the product) forwarded verbatim, including flags (`allow_hyphen_values = true`, `trailing_var_arg = true`).

Exit codes:
- 0: successful delegation (product's own 0) or `--list`.
- 2: missing product id, not-built in-repo product, unknown product, or unavailable external tool.
- child's code (1..=255): propagated unchanged from the product.
- 127: spawn failure.

## Failure boundaries

1. No product id -> error hint, exit 2.
2. `rust` product not built -> `cargo build -p <crate_name>` hint, exit 2; never falls through to PATH/npx.
3. Unknown id (not cataloged, not on PATH) -> known-product list, exit 2; never npx (no network on typo).
4. `external` product not on PATH and no `npx` -> install hint, exit 2.
5. Spawn failure -> stderr message, exit 127.
6. `--format json` errors -> `{"error":{"code":"product-not-found","message":"..."}}` on stderr.

## Resolution algorithm

```
resolve(product):
  if cataloged rust:
     bin = first existing of target/debug/<binary>, target/release/<binary> (workspace root from exe)
     if bin: delegate(bin, args) else error(not-built)
  if cataloged external:
     bin = PATH lookup(<binary>)
     if bin: delegate(bin, args)
     elif npx: delegate_npx(npx, <binary>, args)
     else error(install-hint)
  if PATH lookup(product): delegate            # allow unlisted local binaries
  else error(unknown)
```

## Migration / compatibility

No migration. The consolidated crates already build into `target/`; their original names are preserved. `cargo-sonar` is an alias for the `cargo-sonar-boost` package/binary. Existing direct invocations of the crate binaries continue to work; `forge use` is purely additive.

## Verification

- `cargo build --workspace` green; `cargo check --workspace` green.
- Manual: `forge help`, `forge use --list`, `forge use driftwatchdog -- list` (rc 0), `forge use cargo-sonar -- --help`, `forge use grbk` alias, unknown-id error (no npx), JSON error shape.
- `git diff --check` clean.
