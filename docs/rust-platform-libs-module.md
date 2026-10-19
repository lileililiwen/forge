# Rust platform libraries module

The former Rust platform libraries repository is maintained as ordinary
source under [`modules/rust-platform-libs/`](../modules/rust-platform-libs/).
The import preserves source revision
`4762746e35b3886b21e2c87bf602aed14bc9dea8` and its 30 commits in Forge
history. The module keeps its nested Cargo workspace, eight crates, Rust
1.95.0 toolchain pin, MSRV, Apache-2.0 license, consumer fixture and native
verification scripts. It is not a Git submodule and does not require the old
sibling checkout.

## Local verification

Run these commands from `modules/rust-platform-libs/`:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
cargo build --workspace
cargo test --workspace
./scripts/conformance.sh
```

The contract parity tests use Forge's canonical `contracts/` directory by
default. They validate the five envelope families in `contracts/registry.json`,
validate the standalone public-portfolio-manifest schema and fixture pair, and
assert the four unsupported families recorded in `family-support.json`. Set
`PLATFORM_CONTRACTS_DIR` when running the module against another contract tree;
the tests support the legacy nine-family `schemas/registry.json` layout as
well as Forge's layout. The selected path is authoritative: missing or
malformed data fails the tests without falling back to another checkout.

The root `forge gate` assesses the Forge Git repository. It does not treat the
nested Cargo workspace as a second Git project. Root Rust quality CI runs the
module-native commands from the module root and also checks import provenance
and Governance registration.

## CI and release

`.github/workflows/platform-rust-quality.yml` runs read-only checks when the
module, its canonical contract inputs, Forge project registration, or its
workflow changes. `.github/workflows/platform-rust-publish.yml` is a separate
crates.io publisher. Tag releases use
`rust-platform-libs-v<SemVer>` to avoid colliding with Forge release tags;
manual dispatch requires the exact version and typed owner confirmation.

Publication remains blocked until the module revision is available from the
published Forge repository, the owner tracks
`modules/rust-platform-libs/PUBLICATION-APPROVED`, and
`CARGO_REGISTRY_TOKEN` is configured. This import creates none of those
preconditions and publishes no crates.

## Ownership

Forge owns source history, root workflows, internal project registration and
the canonical contract schemas. The Rust module owns its crate APIs, Cargo
workspace, tests, conformance fixture, package metadata and crates.io policy.
Consumers keep their own dependency and migration decisions. The archived
source checkout is not a build or runtime dependency.
