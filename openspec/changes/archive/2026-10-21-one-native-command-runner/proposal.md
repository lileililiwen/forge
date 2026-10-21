# Change: one-native-command-runner

## Why

`forge test` runs the project's test command through a private `sh -c`
spawner with no timeout. A test that hangs hangs the lane with it: the
release lane's test check, the registry row the operation was writing, and
the operator waiting for a result that never comes.

The code even says so, and then does not do it. `src/gitops/mod.rs`
exports `SUBPROCESS_WAIT_TIMEOUT_SECS`, documented as "Wait timeout for a
single test/subprocess invocation ... an unresponsive tool surfaces as
`TestFailed` rather than hanging the registry". Nothing reads it. The
constant is a promise the runner does not keep.

The shared native runner is in the same state. `src/native/mod.rs` calls
itself "The shared native-command runner" and its own module doc says "The
runner has no timeout. `forge test` has none either; a bounded runner with a
process-group kill is a recorded follow-up." So the build step, the
generator's post-generation verification and the readiness fixtures are all
unbounded too.

And the codebase has learned how to kill a process tree, in one place only.
`src/studio/preview.rs` puts the child in its own process group and signals
the whole group before reaping, because "the direct child is often a package
manager whose dev server runs as a grandchild; killing only the direct child
would leak a descendant". Every other runner kills the direct child only.

## What Changes

- **One native command runner.** The shared runner in `src/native/mod.rs`
  owns the native command contract: the argument array or the shell, the
  working directory, the environment layering, the redaction and the bounded
  capture. `forge test` converges onto it instead of running its own
  `sh -c` spawner. The shell semantics stay, because a profile's test
  command may rely on them.
- **The shared runner is bounded.** Every native command — the build step,
  the generator's verification, the readiness fixtures and `forge test` —
  runs under one wall-clock bound and is killed when it overruns.
- **The kill takes the tree with it.** The child is spawned as its own
  process-group leader and the whole group is signalled before the child is
  reaped, so a test command that spawned descendants leaves nothing behind.
- **The dead constant goes.** `SUBPROCESS_WAIT_TIMEOUT_SECS` is removed. The
  shared runner's own constant owns the number.

## Package Boundary

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `one-native-command-runner` | forge | Rust | implemented | this package |
| `agent-runtime-workflows` | forge | Rust | modified | the `test` operation's runner contract |
| `profile-and-release-readiness` | forge | Rust | modified | the native evidence rows inherit the bound |
| `target-addressed-build` | forge | Rust | read only | the build step keeps its contract and inherits the bound |
| The Gate, the doctor, the publish and deploy lanes | forge | Rust | read only | their contracts are unchanged |

## Non-Goals

- **The adapter runners are not touched.** The release notes adapter, the
  driftwatch adapter, the analytics adapter, the GitHub CLI and the
  governance adapter each keep their own private bounded spawner. They run
  Forge's own adapters, not the project's native commands, and this change
  does not alter their contracts. Their remediation path is recorded in the
  design.
- **The Gate's own adapter runner is not touched.** It belongs to the same
  family as the adapter runners above.
- **The native command contract does not change shape.** The build command
  and the test command are still the profile's declared commands, still run
  as an argument array or through the shell, and the outcome still reports
  the exit code, the bounded output and the start error.
- **The manifest contract does not change.** No new field, no new default.
- **The test command keeps its shell semantics.** A profile's test command
  may use pipes, redirections or `&&`, and it still runs through `sh -c`.
