# Pinned assets: `baseline-service@1.2.0`

These files are the pinned, bundled payload of this pack version. They are
**authored by `workspace-governance`** and pinned here verbatim: Forge bundles
the bytes so a snapshot renders with no sibling checkout present, and never
fetches anything at run time.

A pinned asset is immutable for a given pack version. To change one, add a new
pack version with the new bytes; do not edit a file that an existing version
lists, or the recorded `asset_digest` of that version stops describing what it
renders.

| Project-relative path | Source | Bytes | SHA-256 |
| --- | --- | --- | --- |
| `verify-local.sh` | `<workspace>/scripts/verify-local.sh` (the seed outside any repository, byte-identical to 74 repository copies at 2026-10-16) | 4649 | `b8dfa634621b45c9b0b5a5e6748e90aae494e2bc54a2ba2f93ff443c1ca51b50` |
| `scripts/check-openspec-change-names.mjs` | `workspace-governance/scripts/check-openspec-change-names.mjs` | 815 | `c5003d0cf60ca24b524cb1d08325dc364dca71d5a17d6ee7a0ab5e003b70ea93` |
| `scripts/check-accessibility.py` | `workspace-governance` concern-shim shape (`workspace-governance/scripts/check_concern.py` is the logic) | 721 | `20a0a357fdad9cebd622b71059dcd155f8ff83436c3c7266d85ff184e4a6bdcf` |
| `scripts/check-privacy.py` | same shim shape, concern `privacy` | 703 | `1dcddb2f091c84b86428f694f3eefefc969902cb5ccb69380371cfcc3d98ff20` |

Paths are project-relative on purpose: the change-name checker and the concern
shims are bound by the workspace Gate generator at exactly these paths
(`workspace-governance/scripts/bind_gate_commands.py`), so installing them
anywhere else would leave the concern `UNBOUND`.

The two shims are included because at least two real repositories carry each of
them. `check-provenance.py`, `check-schemas.py` and `check-fixtures.py` exist
once each and stay out until a second repository needs them.

Refresh procedure when the governance owner updates a source file:

1. Copy the new bytes into this directory at the same project-relative path.
2. Update the digest in this table.
3. Add a new pack version in `src/standard/snapshot.rs` that lists the file; leave
   the existing version's list untouched.
