# Proposal: Shared scripts travel in the standard pack, and an existing project can install it

## Why

The workspace pays for the same files once per repository, and Forge — the tool
that already owns versioned, receipt-tracked snapshots — ships a stub instead of
the real content.

Measured across the 83 sibling repositories at `/home/paul/code` (2026-10-16):

| Shared file | Copies | Divergence | Canonical source |
|---|---|---|---|
| `verify-local.sh` (171-line language-detecting CI rehearsal) | 77 | 74 byte-identical; 3 deliberate rewrites (`forge`, `labrys`, `rust-ecommerce`) | `/home/paul/code/scripts/verify-local.sh` — outside any repository |
| `scripts/check-openspec-change-names.mjs` | 29 | 26 distinct contents; the canonical 815-byte file exists in 4 repositories | `workspace-governance/scripts/check-openspec-change-names.mjs` |
| `scripts/check-accessibility.py`, `scripts/check-privacy.py` (thin shims into `workspace-governance/scripts/check_concern.py`) | 7 | byte-identical copies | `workspace-governance/scripts/check_concern.py` + the shim shape |

Three separate facts make this fixable inside Forge rather than as 113 edits:

1. **Forge already renders the right file and fills it with a stub.**
   `.standard/scripts/verify.sh` is a pack asset (`src/standard/layout.rs:21`), and
   for `baseline-service@1.1.0` it renders as
   `#!/usr/bin/env sh … echo "standard baseline-service@1.1.0"` followed by one
   `cargo test` (`src/standard/snapshot.rs:61-68`, rendered and read in
   `/tmp/stdprobe`). The fleet's real script detects Rust/Node/.NET/Python and
   runs format, clippy, build and test for each. The pack owns the slot and puts
   five lines in it.

2. **The snapshot mechanism already gives exactly the guarantees the fleet
   needs**: an ownership receipt with a per-file SHA-256
   (`src/standard/layout.rs:13`), a read-only `diff` that classifies each path
   (`src/standard/snapshot.rs:470-540`), and an `upgrade` that refuses to touch a
   modified or foreign file, rolls back a partial write, and never touches an
   unrelated file (`src/standard/snapshot.rs:541-636`). Only two repositories use
   it (`chronicleaf`, `devloom`), and both were created by Forge itself with
   `--standard-pack baseline-service@1.1.0`.

3. **An existing project cannot be given a snapshot at all.** `standard upgrade`
   requires `--confirm` **and** an existing receipt; measured on a copy of
   `hookit`:

   ```
   $ forge standard upgrade --to baseline-service@1.1.0 --confirm .
   error[standard-invalid]: '.' has no Forge-owned standard snapshot
   (.standard/receipt.json); nothing to diff; no standard snapshot files were changed
   ```

   So the 74 repositories that hand-copied `verify-local.sh` have no path to the
   single-source version. Adding assets to a pack without an install path would
   help only projects that do not exist yet — which is precisely the failure the
   two `.standard/` adopters already demonstrate.

## What Changes

- **A new supported pack version, `baseline-service@1.2.0`**, carrying the real
  shared scripts alongside the existing snapshot files:
  - `.standard/scripts/verify.sh` becomes the canonical 171-line
    language-detecting rehearsal script (authored by `workspace-governance`,
    pinned in Forge as a bundled asset with the pack digest).
  - `scripts/check-openspec-change-names.mjs` — the canonical 815-byte file, at
    the repository-relative path the Gate generator already binds
    (`workspace-governance/scripts/bind_gate_commands.py:240`).
  - `scripts/check-accessibility.py` and `scripts/check-privacy.py` — the two
    concern shims, at the paths the Gate generator binds when the concern is
    selected (`bind_gate_commands.py:1564`).
  Pack assets are committed as reviewable files under
  `standards/baseline-service/1.2.0/` and embedded with `include_str!`, so the
  pinned bytes are diffable instead of living inside a Rust string literal.

- **`forge standard install <pack@version> [PATH]`** — materialize a snapshot
  into a project that has none. It requires `--confirm`; it refuses when a
  receipt already exists (that is `diff`/`upgrade` territory); it refuses,
  naming every path, when a pack-owned path already exists on disk with
  different bytes; it adopts byte-identical existing files (the 74 identical
  `verify-local.sh` copies remain untouched content-wise); it writes the
  receipt and nothing else, with the same rollback discipline as `upgrade`. The
  project id and profile come from an existing `forge.yaml` when present,
  otherwise from `--id`/`--profile`.

## BFS Impact Map

| Surface | Impact |
|---|---|
| `src/standard/layout.rs` | New managed-path constants for the shared scripts and the pinned-asset source tree |
| `src/standard/snapshot.rs` | New `install_snapshot`; `all_packs` gains `1.2.0`; no change to `render`/`diff`/`upgrade` semantics |
| `src/standard/model.rs` | New `InstalledReport` (or reuse `UpgradeReport` shape with `installed:` naming) |
| `src/cli/commands_ops.rs`, `src/cli/semantic.rs` | New `standard install` subcommand and its rendering |
| `standards/baseline-service/1.2.0/**` | New committed assets (4 files) |
| `openspec/specs/standard-pack-registry-and-snapshots/spec.md` | Two added requirements (shared script assets; installation into an existing project) |
| `tests/standard_contract.rs`, `src/standard/snapshot_tests.rs` | New CLI and unit coverage, including byte-equality of the rendered verify script against the pinned asset |
| Callers of the pack | `forge new --standard-pack baseline-service@1.2.0` unchanged; `1.1.0` and `1.0.0` stay supported for pinned projects |
| Sibling repositories | No change in this package. Adoption is a separate, per-repository decision executed with `forge standard install` |

## Capabilities

- `standard-pack-registry-and-snapshots` (extended).

## Non-goals

- No installation into any sibling repository as part of this change. The pack
  and the install verb are the deliverable; rollout is a separate owner decision.
- No change to `.ai-gate/gate.yaml`, `driftwatch.toml` or `.project.json`
  generation: those are already "one generator, N outputs" and stay owned by
  `workspace-governance`.
- No adoption of `AGENTS.md`, `Directory.Build.props`, `global.json`,
  `compose.yaml`, `Dockerfile`, `Makefile`, CI workflows or per-project
  `scripts/*` — measured as intentional per-project values, not drift.
- No change to the `1.0.0`/`1.1.0`/`2.0.0` descriptors, and no new pack family.
- No second authoring home for the asset content: `workspace-governance`
  authors, Forge pins the bytes.
