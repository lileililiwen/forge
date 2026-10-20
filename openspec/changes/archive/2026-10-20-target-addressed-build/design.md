## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The build operation and its outcome | `forge` | Yes. `src/build/`. |
| The operating system, architecture and runtime-identifier vocabulary | `forge` | Yes. `src/target/`. |
| The shared native-command runner | `forge` | Yes. `src/native/`. |
| The generator's native verification | `forge` | Yes. It moves onto the shared runner. |
| The readiness fixtures' native runner | `forge` | Yes. It moves onto the shared runner. |
| The `forge test` shell runner | `forge` | No. Read only. Its convergence is a follow-up. |
| The profile catalog and its descriptors | `forge` | No. Read only. |
| The Gate's own `build` check | `forge` | No. Read only. |
| The manifest contract | `forge` | No. Read only. |
| Every sibling module | its owner | No. Not edited. |

## Language and Runtime

- **Forge Core** is Rust, in the root `forge` crate.
- The build runs the profile's build command as an argument array in the
  project directory, exactly as the generator's verifier already does, with
  the target's environment variables layered on top of the inherited
  environment.
- The command is not run through a shell, so a profile's build command
  cannot inject shell syntax.
- The files it touches stay well under the repository's 1000-line cap:
  `src/generate/engine.rs` is 851 lines, `src/readiness/mod.rs` is 522 and
  `src/cli/gitops.rs` is 261.

## Ownership and Shared Code

Three runners run a native command today:

| Runner | Style | Used by |
|---|---|---|
| `src/generate/engine.rs:768` | argument array, no shell | the generator's post-generation verification |
| `src/readiness/mod.rs:226` | argument array, `PATH` override | the readiness fixtures |
| `src/gitops/mod.rs:454` | `sh -c` | `forge test` |

The first two are the same mechanism with a different evidence shape. This
change moves them onto one runner in `src/native/`:

```rust
/// Run a native command as an argument array in a working directory.
pub fn run_native(
    dir: &Path,
    program: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<NativeOutcome, ForgeError>
```

`NativeOutcome` carries the exit code, the success flag and the redacted
evidence. The runner redacts the captured output through
`crate::policy::redact_credentials`, which the Gate already uses and the
gitops runner does not.

The gitops shell runner keeps its shell semantics, because a profile's test
command may rely on them, and `forge test` keeps its current behaviour. Its
convergence onto the shared runner, with a bounded process-group kill, is a
separate change.

`split_command` (`src/generate/engine.rs:735`) moves with the runner; the
readiness module already calls it from there.

## Ownership of the Target Vocabulary

A target is three possible inputs and one resolution:

```
os + arch  ──▶ canonical os, canonical arch ──▶ the toolchain's target form
rid        ──▶ the toolchain's target form
(nothing)  ──▶ the host
```

`src/target/` owns the canonical list, the aliases and the per-toolchain
resolution:

| Toolchain | Target form | Mechanism |
|---|---|---|
| `cargo` | Rust target triple | `CARGO_BUILD_TARGET` |
| `dotnet` | runtime identifier | `DOTNET_RUNTIME_ID` |
| `node`, `npm` | platform and architecture | `npm_config_platform`, `npm_config_arch` |
| anything else | none | the request is refused |

The canonical operating systems are `linux`, `darwin` and `windows`. The
canonical architectures are `x86_64` and `aarch64`. The accepted aliases are
`macos` and `osx` for `darwin`, `win` for `windows`, `amd64` and `x64` for
`x86_64`, and `arm64` for `aarch64`.

The resolutions:

| os | arch | cargo | dotnet | node |
|---|---|---|---|---|
| linux | x86_64 | `x86_64-unknown-linux-gnu` | `linux-x64` | `linux` / `x64` |
| linux | aarch64 | `aarch64-unknown-linux-gnu` | `linux-arm64` | `linux` / `arm64` |
| darwin | x86_64 | `x86_64-apple-darwin` | `osx-x64` | `darwin` / `x64` |
| darwin | aarch64 | `aarch64-apple-darwin` | `osx-arm64` | `darwin` / `arm64` |
| windows | x86_64 | `x86_64-pc-windows-msvc` | `win-x64` | `win32` / `x64` |
| windows | aarch64 | `aarch64-pc-windows-msvc` | `win-arm64` | `win32` / `arm64` |

The table is deliberately in Forge Core and not borrowed from the vendored
`driftwatchdog` distribution list, which describes the sibling's own release
artifacts and holds three values Forge does not need.

## User Experience and Interface

A new verb, next to `forge test`:

```
forge build <PROJECT> [--os <OS>] [--arch <ARCH>] [--rid <RID>]
```

`PROJECT` defaults to the current directory, as `forge test` does. The three
target flags are exclusive in the sense the spec states: an operating system
needs an architecture and the reverse, and a runtime identifier cannot be
combined with them.

The outcome mirrors `TestOutcome`:

| Field | Meaning |
|---|---|
| `contract` | `forge-build/0.1.0` |
| `project_id` | the project's id |
| `profile` | the profile the build command came from |
| `command` | the command that ran |
| `target` | the resolved target, or `host` |
| `status` | `passed` or `failed` |
| `exit_code` | the command's exit code |
| `note` | one line describing what happened |
| `evidence` | the redacted, truncated stdout and stderr |

A failed build is an error, as a failed test is: `error[build-failed]` with
the reason and the evidence.

## Behavioral Model

| Actor | Block | Boundary |
|---|---|---|
| A project whose profile declares a build command, built for the host | The build runs and the outcome reports the command, the host and the status. | The build writes the toolchain's own output into the project directory. |
| The same project, built for a target its toolchain can address | The build runs with the target bound and the outcome names the resolved target. | The target is bound through the toolchain's own environment variable. |
| A project whose toolchain has no target mechanism, built for the host | The build runs. | Unaffected by the target mechanism. |
| The same project, built for a target | The build is refused and the refusal names the toolchain and the profile. | Forge never builds for the host and reports it as the requested target. |
| A project whose profile declares no build command | The build is refused and the refusal names the profile. | |
| A build command that exits non-zero | The build fails with the evidence. | No build is reported. |
| An operating system without an architecture, or the reverse | The build is refused and the refusal names the missing half. | |
| A runtime identifier together with an operating system and an architecture | The build is refused. | |
| A runtime identifier for a toolchain that does not read runtime identifiers | The build is refused and the refusal names the toolchain. | |
| An unknown operating system or architecture | The build is refused and the refusal names the value and the accepted values. | |
| A documented alias | The build resolves the alias to the canonical value. | |

## Contract and Compatibility

No existing contract changes. The manifest is untouched, the profile catalog
is untouched, the Gate's own build check is untouched, and the test lane keeps
its current behaviour.

Two error variants are added: `build-invalid` for a request Forge cannot
honour, and `build-failed` for a build command that failed. They follow the
`test-invalid`/`test-failed` pattern already in `src/core/mod.rs`.

The build writes nothing outside the project directory: it runs the toolchain,
and the toolchain writes its own output. Forge writes no manifest field, no
registry row and no journal row.

## Failure and Boundary Policy

| Situation | Outcome |
|---|---|
| The path is not a directory or has no manifest | The build is refused, as `forge test` refuses. |
| The profile is unknown | The build is refused, as `forge test` refuses. |
| The profile declares an empty build command | The build is refused and the refusal names the profile. |
| The build command cannot start | The build fails with the reason. |
| The build command exits non-zero | The build fails with the reason and the evidence. |
| The requested target is unknown for the toolchain | The build is refused. |
| The evidence contains a credential | The evidence is redacted before it reaches the report. |

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

- A build for the host runs the profile's build command and reports the host.
- A build with `--os linux --arch aarch64` for a `cargo` project binds
  `CARGO_BUILD_TARGET=aarch64-unknown-linux-gnu` and reports that target.
- A build with `--rid linux-arm64` for a `dotnet` project binds
  `DOTNET_RUNTIME_ID=linux-arm64` and reports that target.
- A build with `--os linux --arch aarch64` for a `python3` project is refused
  and names the toolchain.
- An unknown operating system is refused and names the accepted values.
- A failed build command fails the build with its evidence.

## Implementation Plan

**The vocabulary.** A new `src/target/` module: the canonical operating
systems and architectures, the aliases, the per-toolchain resolution, and the
refusals.

**The runner.** A new `src/native/` module: `run_native`, `NativeOutcome`,
and `split_command` moved from the generator. The generator's verifier and the
readiness runner move onto it.

**The build.** A new `src/build/` module: `run_build`, `BuildOutcome`, and the
resolution of the profile's build command plus the requested target into the
command and the environment.

**The verb.** `src/cli/gitops.rs` gains `cmd_build` next to `cmd_test`, and
`src/main.rs` gains the `Build` command and its dispatch.

**The errors.** `src/core/mod.rs` gains `BuildInvalid` and `BuildFailed` with
the codes `build-invalid` and `build-failed`.

**The tests.** Unit tests for the vocabulary and the resolution, unit tests
for the build operation against a fake project whose build command is a shell
script the test writes, and a contract test for the verb.

## Decision Ledger

- **Resolved:** the target is a command-line choice, not a manifest field.
  The manifest is the project's identity and its deployment plan; a build
  target is a decision of one build, and the owner states it when they build.
- **Resolved:** the vocabulary lives in Forge Core, in `src/target/`, and not
  in the vendored sibling's distribution list. That list is about the
  sibling's own release artifacts.
- **Resolved:** the per-toolchain mechanism is a table in the build module
  for now. The extension-point contract turns it into a declared descriptor
  later; a table is honest about being a table.
- **Resolved:** the build fails on a non-zero exit, like `forge test`. A build
  that half-succeeded is not a build.
- **Resolved:** no timeout. `forge test` has none, and a bounded runner with
  a process-group kill is a separate change.
- **Resolved:** the evidence is redacted. The gitops runner is not, and the
  Gate is; the new runner follows the Gate.
- **Resolved:** the profile catalog is not corrected. The `flutter-app`
  profile declares `flutter analyze` as its build command; the build reports
  the command it ran, so the report is honest, and correcting the catalog is a
  separate change with its own blast radius.
- **Implementation authorized (2026-10-20):** the owner directed the trunk
  repair programme and authorized this package in the approved order. The
  authorized surfaces are the files listed in the proposal.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| A project has a build step | A new `forge build` verb runs the profile's build command and reports the outcome | The owner builds and reads the command, status, exit code and evidence | The build mirrors `forge test` and uses the shared runner | `src/build/`, `src/cli/gitops.rs`, `src/main.rs` | A build succeeds; a failed build fails; a profile without a build command | 2.5–2.7, 3.1 | Unit tests, a contract test, a real run |
| A build may address a target | The operating system, architecture and runtime identifier resolve into the toolchain's form | The owner passes `--os`, `--arch` or `--rid` and the outcome names the target | One table, one resolution, and a refusal instead of a silent host build | `src/target/`, `src/build/` | A target build; a runtime identifier; a toolchain without a mechanism; a partial target; two targets; a host build | 2.1–2.4, 3.2 | Unit tests, a real run |
| The target vocabulary is canonical | One canonical list with documented aliases | The owner is refused by name for an unknown value | The list lives in Forge Core | `src/target/` | An alias is accepted; an unknown value is refused | 2.2, 3.3 | Unit tests |
| The build reports only the target it used | The outcome names the resolved target or the host | The owner can tell what was built | The report carries the used form, never a requested one | `src/build/` | The reported target; the host is reported as the host | 2.6, 3.4 | Unit tests, a real run |
