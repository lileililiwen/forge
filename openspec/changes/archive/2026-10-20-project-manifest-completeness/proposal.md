## Why

A product cannot reach "released" or "deployed" without a human hand-editing
`forge.yaml`.

`forge new` writes only `schema`, `project`, `runtime`, `features` and `kit`
(`src/generate/request.rs`). `forge import --accept` writes only `schema`,
`project` and `runtime` (`src/import/detect.rs:687`). After that:

- `forge release` refuses the project, because there is no `release` section
  (`src/release/prepare.rs:50`).
- `forge deploy` refuses the project, because there is no `deployment` section
  (`src/deploy/prepare.rs:65`).

The refusal is correct — the blocks are missing. The gap is that nothing in
Forge ever writes them. The owner has to know the manifest schema, look up the
right field names, and hand-write two blocks before the product can ship. That
is exactly the case the platform rule names: "if every app independently
reinvents the same manifest block, fold it into a component/template".

The lifecycle the product is built around is idea → develop → test → product →
maintain, and it is supposed to be a paved path. Today the path has a hole in
it that only a human can cross, and the hole sits between "develop" and
"product" — the two phases the product exists for.

## What Changes

- **One owner for the two blocks.** A new `src/core/manifest_blocks.rs` owns
  the default `release:` and `deployment:` blocks and the rule that derives
  them. `forge new` and `forge import --accept` both call it, so the two
  writers cannot drift.
- **`forge new` writes both blocks.** A generated project comes out of the
  generator with a release block and a deployment block, so `forge release
  prepare` and `forge deploy plan` work on it without an edit.
- **`forge import --accept` writes both blocks.** An adopted repository gets
  the same completeness as a generated one.
- **The defaults are derived, never fabricated.** The release block names
  `semver` versioning and the two checks a source project can satisfy with its
  own code (`doctor`, `test`). The deployment block names one `local` target,
  and declares an artifact only when the project actually has one.
- **The registry observation follows.** `forge inspect` reads the legacy
  `deployment.target` field only, so a project with the modern typed
  `deployment.targets[]` form shows no deployment target. The observation now
  falls back to the declared default or the first typed target.
- **Nothing else moves.** A hand-written manifest is never rewritten. The
  release and deploy contracts, the profile catalog, the release check
  semantics, the delivery lanes and the plugin/provider mechanisms are all
  untouched. Those are separate changes.

## Package Boundary and Split Assessment

| Package | Module | Platform/Stack | Status | Decision |
|---|---|---|---|---|
| `project-manifest-completeness` | `forge` | Rust (Forge Core) | implemented | This package. The default manifest blocks and the two manifest writers. |
| `release-evidence-non-vacuous` | `forge` | Rust (Forge Core) | planned | Next. Refuses a release whose checks are all inapplicable. It depends on this package: without a default check set there is nothing to make non-vacuous. |
| `single-delivery-lane` | `forge` | Rust (Forge Core) | planned | Later. Folds the four delivery lanes into one. |
| `target-addressed-build` | `forge` | Rust (Forge Core) | planned | Later. A `forge build` verb with an os/arch/RID. |
| `extension-point-contract` | `forge` | Rust (Forge Core) | planned | Later. One descriptor, discovery, version and state rule for every extension point. |

One package, one owner. The Forge Core manifest writers and the manifest
contract live in this package; the sibling modules own nothing here.

## Sibling and Shared Architecture Reconnaissance

| Neighbor | Shared surface | Boundary decision |
|---|---|---|
| `forge/modules/workspace-governance` | The `.project.json` governance declaration and the `deployment.deployable` field. | A different file with a different contract. The governance declaration is not touched. The `forge.yaml` manifest has no JSON schema; it is validated by the Rust parser alone. |
| `forge/modules/dotnet-platform-libs` | The `forge.yaml` `features:` and `kit:` blocks a generated project pins. | The generated manifest keeps both; this package only adds the two missing blocks. The .NET module is not edited. |
| `forge/modules/rust-platform-libs` | Nothing in this change. | Not edited. |
| `forge/modules/identity-providers` | The `auth:` block in the same manifest. | Unrelated block, untouched. |
| The `platform-contracts` contract set | `forge.yaml` is not in it; the manifest is a local, versioned contract owned by this repository. | Not edited. |

## User Experience and Interface Impact

Before:

```
$ forge new ./shop --profile aspnet-web
$ forge release prepare --version 1.0.0
error[release-invalid]: project `shop` has no `release` section; declare one with at least `versioning: semver`
```

After:

```
$ forge new ./shop --profile aspnet-web
$ forge release prepare --version 1.0.0
... the release plan names the checks it will run ...
```

The generated `forge.yaml` carries:

```yaml
release:
  versioning: semver
  checks:
    - kind: doctor
    - kind: test
deployment:
  artifact: Dockerfile
  targets:
    - name: local
      kind: local
```

A Flutter project, whose scaffold ships no container file, carries the same
blocks without the `artifact` line. Nothing about the owner's workflow changes
except that the hand-edit disappears.

`forge import --accept` behaves the same way, and the deployment block it
writes follows the files the repository actually has: a repository that ships
a Compose file gets a `docker-compose` target, one that ships only a
`Dockerfile` gets a `local` target with that file as its artifact, and one
with neither gets a `local` target and no artifact.

## BFS Impact Map

- **`src/core/manifest_blocks.rs`** — new. The default blocks and the rule
  that derives them.
- **`src/core/mod.rs`** — register the new module.
- **`src/generate/request.rs`** — the generated manifest carries the blocks.
- **`src/generate/engine.rs`** — build the file list first and pass it to
  the manifest writer.
- **`src/import/detect.rs`** — the adopted manifest carries the blocks.
- **`src/registry/model.rs`** — the observation reads the typed deployment
  target.
- **Tests** — the generation and import tests, and the registry observation
  test.
- **Unchanged on purpose** — the release and deploy contracts, the profile
  catalog, the doctor, the delivery lanes, the plugin and provider
  mechanisms, every sibling module, and the `forge.yaml` schema version.

## Capabilities

- New capability: `project-manifest-completeness`.
- No existing capability changes. The manifest contract, the release
  contract and the deploy contract keep their current requirements.

## Non-goals

- No change to the `forge.yaml` schema version, and no change to how a
  hand-written manifest is read.
- No rewrite of an existing project manifest. A project that already has a
  `release:` or `deployment:` block keeps it, byte for byte.
- No change to the release check semantics. A release whose checks are all
  inapplicable is still allowed to pass vacuously; that is the
  `release-evidence-non-vacuous` package, and it is not smuggled in here.
- No change to the generated file set. The scaffold still writes no
  changelog, so a fresh project needs a `CHANGELOG.md` before a release can
  be prepared. The release contract has always required one; this change
  only moves the refusal from a missing manifest block to the missing file.
- No change to the delivery lanes, the deploy target kinds, or the plugin and
  provider mechanisms.
- No new CLI verb. `forge new` and `forge import` keep their surface.
- No change to the doctor, the gate, the release engine or the deploy engine.
- No change to any sibling module.
