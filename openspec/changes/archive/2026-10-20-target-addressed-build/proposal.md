## Why

The spine of Forge has no build step.

`forge test` runs the profile's test command, the Gate has a `build` check
bound to a manifest-declared command, and the profile descriptor already
carries a `build_command` for every profile. Nothing in Forge builds a
project. A project that has to ship a binary for another platform has no Forge
operation for it, and the target it was built for is never recorded anywhere.

Forge Core also has no vocabulary for a target at all. The only os/arch lists
in the repository are in the vendored `driftwatchdog` crate, and they describe
the sibling's own release artifacts, not a project's build target.

Three near-duplicate native-command runners already exist: the generator's
verifier (`src/generate/engine.rs:768`), the readiness runner
(`src/readiness/mod.rs:226`) and the gitops shell runner
(`src/gitops/mod.rs:454`). A fourth copy inside a build command would make
the spine's build step behave differently from its test step for no reason, so
the build gets one shared runner instead.

## What Changes

- **A build step.** A new `forge build` command runs the profile's build
  command in the project directory and reports the outcome: the command, the
  target, the status, the exit code and the evidence. It mirrors
  `forge test`, the sibling step of the same spine.
- **A target vocabulary.** A new `src/target/` module owns one canonical
  operating system and architecture list, the documented aliases, and the
  resolution of an operating system and architecture into the form each
  toolchain reads.
- **Three toolchains address a target.** `cargo` reads
  `CARGO_BUILD_TARGET` and takes a Rust target triple; `dotnet` reads
  `DOTNET_RUNTIME_ID` and takes a runtime identifier; `node` and `npm` read
  `npm_config_platform` and `npm_config_arch`. A toolchain with no mechanism
  refuses a requested target instead of silently building for the host.
- **The reported target is the used target.** The outcome names the resolved
  target in the form the toolchain read, and names the host when no target was
  bound.
- **One native-command runner.** The new runner lives in `src/native/`, takes
  an argument array, environment overrides and a working directory, and
  redacts the captured output. The generator's verifier and the readiness
  runner move onto it. The gitops shell runner keeps its shell semantics for
  `forge test` for now; its convergence is a recorded follow-up.
- **Nothing else moves.** The manifest contract, the profile catalog, the
  Gate's own build check, the release lane, the deploy lane and every sibling
  module are untouched.

## Package Boundary and Split Assessment

| Package | Module | Platform/Stack | Status | Decision |
|---|---|---|---|---|
| `target-addressed-build` | `forge` | Rust (Forge Core) | implemented | This package. The build step of the spine and the target vocabulary. |
| `one-native-command-runner` | `forge` | Rust (Forge Core) | planned | Later. The gitops shell runner for `forge test` converges onto the shared runner, with a bounded process-group kill. |
| `extension-point-contract` | `forge` | Rust (Forge Core) | planned | Later. The per-toolchain target mechanism becomes one declared descriptor instead of a table in the build module. |

One package, one owner. The build step belongs to Forge Core; the target
vocabulary belongs with it because the build is its only reader today.

## Sibling and Shared Architecture Reconnaissance

| Neighbor | Shared surface | Boundary decision |
|---|---|---|
| `forge/modules/workspace-governance` | The Gate runtime and the profile catalog's shared-layer references. | Read only. The build command is read from the profile descriptor, as `forge test` reads the test command. |
| `forge/modules/dotnet-platform-libs` | The .NET toolchain's runtime-identifier vocabulary. | Read only. The runtime identifiers used here are the .NET portable ones the toolchain already understands. |
| `forge/modules/rust-platform-libs` | The Rust target triples. | Read only. The triples used here are the ones `cargo` already understands. |
| `forge/modules/identity-providers` | Nothing in this change. | Not edited. |
| The `platform-contracts` contract set | The Gate evidence contract and the build contract are local to this repository. | Not edited. |

## User Experience and Interface Impact

Before: there is no way to ask Forge to build a project.

```
$ forge build ./my-app --os linux --arch aarch64
error: unrecognized subcommand 'build'
```

After:

```
$ forge build ./my-app
project: my-app
profile: rust-web
command: cargo build
target: host
status: passed
exit_code: 0
note: build command `cargo build` passed for project `my-app`
evidence: stdout: Compiling my-app v0.1.0
```

and with a target:

```
$ forge build ./my-app --os linux --arch aarch64
project: my-app
profile: rust-web
command: cargo build
target: aarch64-unknown-linux-gnu
status: passed
exit_code: 0
note: build command `cargo build` passed for project `my-app` at target `aarch64-unknown-linux-gnu`
evidence: stdout: Compiling my-app v0.1.0
```

A toolchain that cannot address a target refuses instead of building for the
host:

```
$ forge build ./my-app --os linux --arch aarch64
error[build-invalid]: build invalid: profile `python-service` uses toolchain `python3`, which has no target-addressed build; build for the host instead
```

## BFS Impact Map

- **`src/target/`** (new) — the canonical operating system and architecture
  vocabulary, the aliases, and the per-toolchain resolution into a target
  triple, a runtime identifier or a platform/architecture pair.
- **`src/native/`** (new) — the shared native-command runner: an argument
  array, environment overrides, a working directory, and redacted evidence.
- **`src/build/`** (new) — the build operation and its outcome.
- **`src/generate/engine.rs`** and **`src/readiness/mod.rs`** — their local
  runners move onto the shared runner.
- **`src/cli/gitops.rs`** — the `forge build` handler, next to `forge test`.
- **`src/main.rs`** — the `build` verb and its dispatch.
- **`src/api/command_catalog/rows_project.rs`** and
  **`src/api/command_catalog/catalog.rs`** — the command catalog carries a row
  for every Clap path, so the new verb needs a row and the catalog's row count
  moves with it.
- **`src/core/mod.rs`** — the `build-invalid` and `build-failed` error
  variants.
- **Unchanged on purpose** — the manifest contract, the profile catalog, the
  Gate's own build check, the release lane, the deploy lane, the test lane's
  shell runner, and every sibling module.

## Capabilities

- New capability: `target-addressed-build`.
- No existing capability changes. The `profile-and-release-readiness` and
  `forge-native-gate` contracts keep their current requirements.

## Non-goals

- No change to the profile catalog, including the `flutter-app` profile whose
  declared build command is `flutter analyze`. The build reports the command
  it ran, so nothing is hidden, and correcting that profile is a separate
  change.
- No change to the Gate's `build` check, which is bound to a manifest-declared
  command and is not the same surface as `forge build`.
- No change to the manifest contract. The target is a command-line choice,
  not a manifest field.
- No change to the release lane, the deploy lane or the delivery lanes.
- No new artifact concept. Nothing in Forge produces a compiled artifact
  today, and the deploy lane's artifact is a container file.
- No timeout on the build. `forge test` has none either; a bounded runner with
  a process-group kill is the recorded follow-up.
- No change to any sibling module.
