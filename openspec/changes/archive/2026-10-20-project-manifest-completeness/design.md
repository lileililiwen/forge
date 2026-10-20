## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The default `release:` and `deployment:` manifest blocks | `forge` | Yes. New `src/core/manifest_blocks.rs`. |
| The generated project manifest | `forge` | Yes. `src/generate/request.rs`. |
| The adopted project manifest | `forge` | Yes. `src/import/detect.rs`. |
| The registry observation of a deployment target | `forge` | Yes. `src/registry/model.rs`. |
| The `forge.yaml` contract and its parser | `forge` | No. Read only. |
| The release and deploy contracts and their engines | `forge` | No. Read only. |
| The profile catalog | `forge` | No. Read only. |
| Every sibling module | its owner | No. Not edited. |

## Language and Runtime

- **Forge Core** is Rust, in the root `forge` crate.
- The new module is a small, dependency-free module under `src/core/`, next to
  the manifest contract it serves. It is text rendering plus one pure
  derivation function; it touches no I/O at all, because the caller hands it
  the file names it already knows.
- `src/core/manifest.rs` is 788 lines and the repository caps a source file at
  1000 physical lines, so the new code lands in its own file rather than
  growing the contract module.

## Ownership and Shared Code

Two writers write `forge.yaml`:

| Writer | File | Used by |
|---|---|---|
| Generation | `src/generate/request.rs::manifest_text` | `forge new` |
| Import | `src/import/detect.rs::build_manifest_text` | `forge import --accept`, and the graduation import |

Both now render the two blocks from the same module. That is the point of the
change: one rule, two call sites, no drift. The blocks are rendered as YAML
text in the same style the existing writers already use, and a test parses the
rendered result back through the real manifest parser, so the rendered shape
cannot drift from the parser either.

`src/import/graduation.rs`'s import path also calls `build_manifest_text`. It
adopts an artifact that Forge did not generate, so it declares no deployment
artifact; the release block is still complete.

## User Experience and Interface

`forge new` and `forge import --accept` write a manifest that already carries
the release and deployment blocks. No verb is added and no existing verb
changes shape. `forge inspect` shows the deployment target of a project whose
manifest uses the typed `targets[]` form.

## Behavioral Model

| Actor | Block | Boundary |
|---|---|---|
| A new project | `release:` with `versioning: semver` and the checks `doctor` and `test`. | `doctor` is Forge's own assessment. `test` runs the profile's own test command. Both are the checks a source project can satisfy with its own code. `gate` and `driftwatch` are not declared by default: they need evidence the owner produces by running them, and a declared check that cannot run blocks the release honestly. |
| A new project | `deployment:` with one target named `local` of kind `local`, plus an artifact when the project has one. | `local` needs no artifact, so the block is always complete. The artifact is declared only when the file exists, because a declared artifact that is missing is refused at deploy time. |
| An adopted project | The same release block. | The same rule. |
| An adopted project | A `deployment:` block derived from the files the repository actually has. | A Compose file gives a `docker-compose` target and that file as the artifact. A Dockerfile alone gives a `local` target with that file as the artifact. Neither gives a `local` target and no artifact. |
| A project with its own manifest | Untouched. | Forge never rewrites a manifest it did not write in this operation. |

The deployment block never mixes the legacy `type`/`target` form with the typed
`targets[]` form; `DeployConfig::from_manifest_meta` refuses a manifest that
does (`src/deploy/model.rs:109`).

## Contract and Compatibility

The `forge.yaml` contract keeps schema version 1. The two blocks are additive:
a manifest written before this change has no `release:` and no `deployment:`,
and it keeps parsing exactly as before. Nothing in the parser changes.

The default release block:

```yaml
release:
  versioning: semver
  checks:
    - kind: doctor
    - kind: test
```

`versioning: semver` is the only scheme the release contract accepts
(`src/release/model.rs:39`). `doctor` and `test` are two of the four known
check kinds (`src/release/model.rs:54`).

The default deployment block, for a project with a container file:

```yaml
deployment:
  artifact: Dockerfile
  targets:
    - name: local
      kind: local
```

`local` is a supported target kind (`src/deploy/contract.rs:41`) and `local` is
a valid kebab-case target name. A `local` target does not require an artifact
(`src/deploy/engine/stages.rs:53`), so the block stays complete when the
project has no artifact to declare.

Compatibility rules that hold for the change:

- A hand-written manifest is never rewritten.
- A generated manifest parses through `Manifest::parse` unchanged.
- `release_config_from_manifest` and `deploy_config_from_manifest` succeed on a
  freshly generated manifest.
- A declared artifact is a file the project actually has, so
  `load_artifact` never refuses the generated default.

## Failure and Boundary Policy

| Situation | Outcome |
|---|---|
| The generated tree ships no container file | The deployment block declares a `local` target and no artifact. |
| The imported repository has no container file | The same. |
| The imported repository has a Compose file | A `docker-compose` target with that file as the artifact. |
| The imported repository has a Dockerfile only | A `local` target with that file as the artifact. |
| A project already has a `release:` or `deployment:` block | The block is kept exactly as the owner wrote it. This package only adds blocks to manifests Forge writes. |
| The profile is unknown | Generation refuses before any file is written, as it does today. |
| A manifest cannot be parsed after rendering | Generation refuses before promotion; the staged manifest is validated before the tree is written. |

## Verification Oracle

```sh
# The Forge Core test suite
cargo test --workspace --all-targets -- \
  --skip rust_scaffold_builds_and_tests_with_native_toolchain \
  --skip rust_scaffold_builds_with_native_toolchain_without_forge \
  --skip declaration_is_inert_for_native_builds_present_and_absent \
  --test-threads=1

# The repository Gate
forge gate --dry-run
forge gate --timeout-secs 1800

# Strict OpenSpec validation and whitespace
openspec validate --all --strict --no-interactive
git diff --check
```

The oracle is the behaviour, not the build:

- A generated project parses, and its release and deployment configurations
  resolve without a refusal.
- The generated deployment artifact is a file the generated tree actually
  contains.
- An adopted project gets the same blocks, with an artifact that matches the
  repository.
- A project whose manifest uses the typed `targets[]` form shows a deployment
  target in `forge inspect`.
- A manifest Forge did not write is not modified.

## Implementation Plan

**The new module.** `src/core/manifest_blocks.rs` owns three things:

- `DEFAULT_RELEASE_CHECKS` — the two check kinds a fresh project declares.
- `release_block(checks)` — renders the `release:` YAML text.
- `deployment_defaults(present_files)` — returns the default kind and the
  optional artifact for a set of file names that exist.
- `deployment_block(kind, artifact)` — renders the `deployment:` YAML text.

**The generation writer.** `manifest_text` takes the names of the files the
generated tree will contain and renders both blocks. `src/generate/engine.rs`
builds the file list first and then renders the manifest, so the deployment
defaults come from the tree the generator really writes. The profile layout
describes the profile in general and is not always the full file list: two
profiles ship a container file that their layout does not name, and a
deployment block that ignored the tree would declare no artifact for them.

**The import writer.** `build_manifest_text` gains the repository's container
files and renders both blocks. The graduation import passes none.

**The registry observation.** `deployment_target` falls back to the declared
`deployment.default`, then to the first typed target's name, then to the
legacy `deployment.target`.

**The tests.** The generation and import tests assert the new content, and a
new test parses a rendered manifest and resolves both configurations.

## Decision Ledger

- **Resolved:** the release block declares `doctor` and `test`. They are the
  two checks a project satisfies with its own source. `gate` and `driftwatch`
  are not declared by default because they need evidence the owner produces
  by running them, and a declared check that cannot run blocks the release.
- **Resolved:** the deployment target is `local`. It needs no artifact, so
  the block is complete whatever the project contains.
- **Resolved:** the artifact is declared only when the file exists. A
  declared artifact that is missing is refused at deploy time
  (`src/deploy/prepare.rs:195`), so the default must not be a lie.
- **Resolved:** a generated project's artifact comes from the files the
  generator writes, not from the profile layout. The layout is a description
  of the profile and is not the full file list, so two profiles that ship a
  container file would otherwise declare no artifact.
- **Resolved:** only a project-root container file counts. The standard
  pack's own Compose file lives under `.standard/` and is a Forge-owned
  standard asset, so pointing a deployment at it would couple the deployment
  to a pack's internal layout. The project's own Dockerfile is the artifact.
- **Resolved:** the import path derives the artifact from the repository's
  files rather than from the profile layout, because an adopted repository is
  not the profile's scaffold.
- **Resolved:** the blocks are rendered as text, not serialized from the
  contract types, because the contract types are `Deserialize`-only. A test
  parses the rendered result back through the real parser, so the rendered
  shape cannot drift from the contract.
- **Deferred:** refusing a release whose checks are all inapplicable. That is
  the `release-evidence-non-vacuous` package.
- **Deferred:** the scaffold still writes no changelog, so a fresh project
  needs a `CHANGELOG.md` before `forge release prepare` can run. That is a
  project file, not a manifest block: the release contract has always
  required a present changelog, and the refusal now names the changelog
  instead of a missing manifest section. Writing an initial changelog is a
  separate change to the generator.
- **Implementation authorized (2026-10-19):** the owner directed the trunk
  repair programme and authorised this package first. The authorized surfaces
  are the files listed in the proposal.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| A generated or imported project carries a complete release block | Both writers render the block | `forge new` → `forge release prepare` works | `semver` plus `doctor` and `test` | `src/core/manifest_blocks.rs`, both writers | The block parses and resolves; a hand-written block is untouched | 2.1–2.4 | Generation and import tests, the repository suite |
| A generated or imported project carries a complete deployment block | Both writers render the block | `forge new` → `forge deploy plan` works | One `local` target; artifact only when the file exists | `src/core/manifest_blocks.rs`, both writers | The block resolves; the declared artifact exists | 2.5–2.8 | Generation and import tests, the repository suite |
| The defaults are derived, never fabricated | The derivation is one pure function | The owner sees only what the project actually has | Layout for a new project, repository files for an adopted one | `deployment_defaults` | A profile with no container file declares no artifact; a repository with a Compose file declares a compose target | 2.5–2.6 | Unit tests of the derivation |
| The registry observation reflects the completed manifest | `forge inspect` shows the deployment target | `forge inspect` names the target | `default`, then the first typed target, then the legacy field | `src/registry/model.rs` | A typed-only manifest shows its target; a manifest with no deployment block shows none | 2.9 | Registry test |
| An existing manifest is never rewritten | No rewrite path is added | The owner's files stay theirs | The change only affects manifests Forge writes | Both writers | A manifest with its own blocks keeps them byte for byte | 2.4, 2.8 | Generation and import tests |
