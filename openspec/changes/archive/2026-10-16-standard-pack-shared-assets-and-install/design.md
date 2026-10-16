# Design: standard-pack shared assets and installation into an existing project

## 1. Ownership decision

Owner-ruled on 2026-10-16: **`workspace-governance` authors the shared content,
Forge pins the bytes and distributes them.**

- Authoring stays where the authority already is: the OpenSpec name checker, the
  concern shims and the CI rehearsal script are governance/verification content,
  and the gate generator that binds them already lives in
  `workspace-governance/scripts/bind_gate_commands.py`.
- Forge bundles a pinned copy, exactly as the existing pack precedent does:
  `baseline-service@1.1.0` declares `external template source:
  workspace-governance/templates/runtime (bundled local fallback; never
  fetched)` and Forge ships the bytes.
- Nothing is fetched at run time. Forge keeps its "runs with no sibling
  checkout present" invariant; the pinned assets are the fallback.

## 2. Assets pinned by the change

All digests are of the canonical file as measured on 2026-10-16.

| Pack path (project-relative) | Source | Size | SHA-256 | Copies it replaces |
|---|---|---|---|---|
| `.standard/scripts/verify.sh` | `/home/paul/code/scripts/verify-local.sh` (the seed outside any repository, byte-identical to 74 repository copies) | 4,649 B | `b8dfa634621b45c9b0b5a5e6748e90aae494e2bc54a2ba2f93ff443c1ca51b50` | 74 identical copies; the pack's current 5-line stub is replaced |
| `scripts/check-openspec-change-names.mjs` | `workspace-governance/scripts/check-openspec-change-names.mjs` | 815 B | `c5003d0cf60ca24b524cb1d08325dc364dca71d5a17d6ee7a0ab5e003b70ea93` | 29 copies in 26 distinct contents |
| `scripts/check-accessibility.py` | `workspace-governance` shim shape (as in `actoria`, `citylens`, `mewo`, `trailCrew`) | 721 B | identical in all 4 copies | 4 identical copies |
| `scripts/check-privacy.py` | same shim shape, concern `privacy` | 703 B | identical in all 3 copies | 3 identical copies |

**Why these two shims and not the other three.** `check-provenance.py`,
`check-schemas.py` and `check-fixtures.py` exist once each
(`citylens`, `platform-contracts`, `platform-contracts`). The workspace's
two-consumer rule (`ARCHITECTURE.md` Ruling 6) bars promoting a
single-consumer artifact; the shim shape is parameterised by the concern word, so
a third concern joins the pack the moment a second repository needs it.

**Why they are stored as committed files.** The pinned bytes are committed under
`standards/baseline-service/1.2.0/<project-relative-path>` and embedded with
`include_str!`. The alternative — Rust string literals — was how the 5-line stub
happened; a 4.6 KB shell script inside a Rust source file is unreviewable and
would itself be a drift risk.

## 3. Why the shared scripts are not all under `.standard/`

`forge standard` guarantees "Forge owns exactly this subtree: every owned file
lives under it, so an upgrade can never touch unrelated project files"
(`src/standard/layout.rs:7-9`). That guarantee stays intact because ownership is
carried by the **receipt**, not by the directory prefix: `check`, `diff` and
`upgrade` operate on `receipt.files` paths relative to the project root
(`src/standard/snapshot.rs:379-470`, `:470-540`, `:541-636`), and `upgrade`
writes exactly the rendered paths plus the receipt, rolling back on failure.

Two of the four assets must live at repository-relative paths, because the Gate
generator binds those paths by name and would otherwise report the concern
`UNBOUND`:

- `workspace-governance/scripts/bind_gate_commands.py:240` —
  `OPENSPEC_NAMES_SCRIPT = "scripts/check-openspec-change-names.mjs"`.
- `workspace-governance/scripts/bind_gate_commands.py:1564` —
  `relative = f"scripts/check-{concern}{found[0]}"`.

Putting them under `.standard/` instead would require a change in
`workspace-governance` to teach the generator a second search root — a different
owner's change, and one that would leave the 14 Gate files that already bind
`scripts/check-openspec-change-names.mjs` broken in the meantime.

Consequence, stated plainly: the pack can now own files **outside** `.standard/`.
That is a deliberate widening of the pack's reach. The protective rule is
unchanged and is enforced in code: an existing pack-owned path whose bytes differ
is a conflict, and neither `install` nor `upgrade` writes anything without the
reviewed resolution.

## 4. Pack versioning

| Version | State | Change |
|---|---|---|
| `0.9.0` | deprecated | unchanged |
| `1.0.0`, `1.1.0` | supported | unchanged; projects pinned to them keep working |
| `2.0.0` | proposed | unchanged (unverified; not selectable) |
| **`1.2.0`** | **supported + verified** | the four assets in §2, on top of the `1.1.0` file set |

`1.2.0` is a superset of `1.1.0`; a project on `1.1.0` diffing against `1.2.0`
sees four `added` paths, `verify.sh` as `updated` (the stub replaced by real
content), and everything else `unchanged`. `2.0.0` is left proposed because it is
unverified; promoting it is not part of this change.

Evidence for `verified` is the new coverage in §9, not a claim: the rendered
`verify.sh` is asserted byte-equal to the pinned asset, and `forge standard
check` reports `rendered` for a pack-installed project.

## 5. `forge standard install <pack@version> [PATH]`

Semantics, in evaluation order. Every refusal writes nothing.

1. `--confirm` is required; without it, a typed `standard-invalid` refusal naming
   the flag (`upgrade` already uses this shape, `snapshot.rs:549-553`).
2. A receipt already present in `PATH` is refused, pointing at
   `forge standard diff`/`upgrade`. `install` never overwrites an owned snapshot.
3. The target pack is resolved with `inspect_pack`; an unknown selector is the
   existing typed refusal. A pack that does not support the resolved profile is
   refused by `render_snapshot` (`snapshot.rs:274-283`).
4. **Collisions.** For each rendered path, if a file exists on disk:
   - bytes equal to the rendered content → **adopted** (not rewritten, reported
     `adopted`); this is how a repository carrying the byte-identical
     `verify-local.sh` keeps it untouched;
   - bytes different → **conflict**: the whole install is refused, every
     conflicting path is named, and nothing is written. There is no `--force` on
     `install`: replacing a repository's own script is a separate, reviewed edit
     (`diff`/`upgrade` handle that case for snapshots).
5. Otherwise: write the rendered files and the receipt, creating parent
   directories, with the same rollback-on-failure path as `upgrade`
   (`snapshot.rs:600-625`). Unrelated files are never touched. Reported as
   `installed`.

Project identity and profile resolution: an existing `forge.yaml` supplies `id`
and `profile`; otherwise `--id` (default: the directory name, validated as a
kebab-case id) and `--profile` (required) are used. The resolved pair is
recorded in the receipt, which is what makes later `diff`/`upgrade` deterministic.

## 6. Interaction with the existing verbs

- `check` — unchanged; a pack-installed project reports `rendered` with four
  more file rows.
- `diff` — unchanged; installed projects now diff against `1.2.0` like any other.
- `upgrade` — unchanged. Once a repository is on `1.2.0`, asset updates arrive as
  a new pack version, which is the whole point of installing rather than copying.
- `forge new --standard-pack baseline-service@1.2.0` — works with no further
  change; `staged_files` already renders any selectable pack
  (`snapshot.rs:211-227`).

## 7. The no-sibling invariant

Forge still builds, tests and runs with no sibling repository present: the assets
are embedded bytes. Two of the installed files are *shims* that invoke
`../workspace-governance/scripts/check_concern.py` at run time — that is the
existing, deliberate fleet-layout contract (the same relative edge the generated
Gate commands already use: `python3 ../workspace-governance/scripts/...`), and it
only executes when a repository's Gate selects that concern. A repository outside
the fleet layout is not a target for these two shims and `install` refuses a
collision rather than guessing.

## 8. Risks and trade-offs

| Risk | Mitigation |
|---|---|
| The pack owning repository-relative paths widens what an upgrade may write | Receipt-scoped writes; divergent path = conflict; no `--force` on `install`; rollback on partial write |
| Installing the shims could change what a Gate binds | Only when `.ai-gate/gate.yaml` also selects that concern; installing a file never edits the Gate. A repository that does not want it simply does not install |
| The canonical `verify-local.sh` needs four toolchains to be useful | It degrades per detected language and exits non-zero only on a real failure; unchanged from the 74 copies that already run it |
| Duplicating the canonical script into Forge creates a second copy to maintain | That is the owner-ruled trade (author in `workspace-governance`, pin in Forge). The pinned bytes are digest-recorded per pack version, so drift is a diff, not a mystery |
| `verify.sh` in the snapshot vs `verify-local.sh` at the root could confuse operators | The pack path is the single owned one; the proposal states that repositories remove their root copy after installing. Documented in the pack descriptor and in the install report |

## 9. Verification plan

- **Unit** (`src/standard/snapshot_tests.rs`): `1.2.0` is selectable and a
  superset of `1.1.0`; the rendered `verify.sh` is byte-equal to the pinned
  asset; the rendered name checker is byte-equal to the canonical source bytes;
  install refuses without `--confirm`; install refuses an existing receipt;
  install refuses a divergent owned path and names it; install adopts a
  byte-identical path and leaves its bytes untouched; install writes the receipt
  and reports `installed`; `check` then reports `rendered`; `diff` to `1.2.0`
  from the installed snapshot is empty.
- **Contract** (`tests/standard_contract.rs`): the same through the built binary
  against a copy of a real repository layout (`verify-local.sh` present, identical
  → adopted; divergent → refusal naming the path), plus `forge standard check`
  after install and a `forge new --standard-pack baseline-service@1.2.0`
  regression.
- **Live**: install into a throwaway copy of a sibling under `/tmp`, run
  `.standard/scripts/verify.sh` there, and confirm the repository's own files are
  otherwise byte-identical (`git status` in the copy).
