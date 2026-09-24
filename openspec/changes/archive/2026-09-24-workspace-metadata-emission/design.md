# Design: Workspace metadata emission

## Ownership and boundaries

Workspace Governance owns the `.project.json` schema and adoption
semantics. Forge owns generation of an initial, honest instance. After
generation the file belongs to the project: users edit it, the sibling
reads it, Forge's doctor treats it as informational evidence, and Forge's
upgrade never silently overwrites user edits (ownership receipt rules
apply).

## Schema mapping

Target schema (sibling, version 1):

```json
{"schema_version": 1, "id": "...", "kind": "product",
 "profile": "rust-product", "lifecycle": "active",
 "verification": {"command": "cargo test --workspace",
   "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
 "deployment": {"deployable": false, "jenkins_job": null,
   "compose_file": null}}
```

Profile table (lives in each profile descriptor, not hardcoded in the
generator):

| Forge profile | governance profile | verification |
| --- | --- | --- |
| rust-web | rust-product | profile's native test command |
| python-service | python-product | pytest command |
| nextjs-web / react-web | node-product | npm test |
| aspnet-web | dotnet-product | dotnet test |
| flutter-app | flutter-product | flutter test |

Profiles whose mapping is unknown emit **no** file (with a printed note);
a guessed profile string is refused because the sibling's checker validates
its own vocabulary and a wrong value would poison adoption.

The `profile` values in this table must be confirmed against the sibling's
current `PROFILES` constant at implementation time (that repo's vocabulary
may extend `rust-product`, `dotnet-library`, `deployment`, …); the table is
data, not a Forge product concept, and drift in it is a docs/fixture fix,
never a schema migration.

## Honesty rules

- `evidence_status` starts `planned` always; Forge never writes `passed`.
- `gate_runtime` only when the profile descriptor names one.
- `deployment.*` stays null/false at creation.
- `verification.command` is the same native command the profile's readiness
  matrix uses — one source of truth in the descriptor.

## Generation flow

1. Existing staged generation adds `.project.json` to the ownership
   manifest with a content hash.
2. `--no-workspace-metadata` or profile without mapping → file simply
   absent; output byte-identical to today's release.
3. Import: file presence is recorded as an observation; never written.
4. Upgrades: a modified file produces the standard ownership-conflict
   refusal, identical to other managed files.

## Determinism & validation

Emission is pure function of (manifest, profile descriptor). The
readiness/native-build matrix gains a per-profile assertion that the file
is valid JSON, matches the sibling schema's required keys, and carries the
mapped values.

## Verification

Fixtures for each MVP+extended profile; opt-out parity hash test; import
preservation test; a validation test parsing the emitted file against the
documented schema; ownership conflict on user-edited file.
