# Design: standard-pack-registry-and-snapshots

## Implementation boundary

Repository: `forge`, Rust Core/CLI, existing SQLite registry and deterministic
asset renderer. Inspect/change `src/profile/`, `src/generate/`, registry
migrations, CLI dispatch in `src/main.rs`, and contract tests. Do not modify
Workspace Governance policy code, product repositories, or the active portal
routes.

## Language and runtime

Rust 1.87+ using the existing Cargo workspace and SQLite strategy. Primary
commands are `cargo fmt --check`, `cargo build`, focused `cargo test`, full
`cargo test --all-targets` with the repository's native-scaffold skip where
needed, and strict OpenSpec validation.

## Ownership and shared code

Forge owns pack descriptors, compatibility, receipts, and upgrade planning.
Workspace Governance runtime templates may be consumed through an explicit
versioned asset adapter, but Forge must retain a local fallback pack and must
not require the sibling checkout. CSS tokens remain asset/package inputs, not
Forge runtime code.

## Behavioral model

Pack lifecycle: `proposed -> supported -> deprecated`; only `supported` packs
with verified render/build fixtures are selectable. Project snapshot lifecycle:
`absent -> rendered -> modified -> upgrade-planned -> upgraded`.

Required receipt fields:

```json
{
  "profile": "rust-service",
  "pack": "baseline-service",
  "version": "1.0.0",
  "asset_digest": "<sha256>",
  "generated_at": "<timestamp>",
  "files": [{"path": ".standard/profile.yaml", "digest": "<sha256>"}]
}
```

Rendering is deterministic for identical pack, inputs, and tool version.
Existing user files are never overwritten unless an explicit upgrade plan is
confirmed. A modified owned file becomes a conflict, not an implicit merge.

## Contract and compatibility

Add CLI contracts:

```text
forge standard list
forge standard inspect <pack>@<version>
forge standard check [PATH]
forge standard diff [PATH] --against <pack>@<version>
forge standard upgrade [PATH] --to <pack>@<version> --confirm
```

The registry stores pack identity, version, supported profiles, asset digest,
support state, and compatibility range. Existing `forge new` behavior remains
valid: no pack is ever selected implicitly, so without an explicit
`--standard-pack <pack>@<version>` the output stays byte-identical to
pre-standard releases (the prior pinned tree digests still pass). A selected
pack must be supported with verified fixtures and compatible with the
profile; anything else refuses during request normalization, before any file
change. There is no implicit `latest`.

## Failure and boundary policy

| Case | Result |
|---|---|
| Unknown pack/version | typed `standard-invalid`, no files changed |
| Unsupported profile mapping | refusal naming profile and pack |
| Missing asset/digest mismatch | unavailable/error, no partial snapshot |
| Modified owned file on upgrade | conflict plan, no overwrite |
| Existing unrelated file | preserved |
| Pack deprecated but pinned in project | inspect/check works; new generation refuses unless explicitly allowed |
| External runtime template unavailable | local fallback or typed unavailable; never silently fetch |

## Verification oracle

Add unit tests for descriptor parsing, digest validation, deterministic output,
receipt ownership, compatibility, conflict detection, and refusal atomicity.
Add CLI contract tests for list/inspect/check/diff/upgrade, generated fixture
build/test commands, and no-sibling-checkout execution. Verify rendered Compose
with `docker compose config` where Docker is available; otherwise record the
exact unavailable check. Run format/build/clippy/tests, name preflight, strict
OpenSpec validation, and `git diff --check`.

## Decision ledger

- Generated projects own a snapshot; they do not mount a template repository.
- Standard upgrades are explicit and conflict-safe.
- CI invokes the project-local `scripts/verify.sh`; Forge does not need to run
  inside project CI.
- A design-token package is deferred until two consumers demonstrate a stable
  shared implementation.
