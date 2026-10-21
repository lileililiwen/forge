## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The native command runner | `forge` | Yes. `src/native/mod.rs`. |
| The process-group lifecycle | `forge` | Yes. `src/process.rs`. |
| The `forge test` operation | `forge` | Yes. `src/gitops/mod.rs`. |
| The build step, the generator's verification, the readiness fixtures | `forge` | Read only; they inherit the bound through the shared runner. |
| The adapter runners | `forge` | Read only. |
| The profile catalog, the manifest | `forge` | Read only. |

## The Gap

Three facts, in the order they matter.

**`forge test` is unbounded.** The whole operation is one call:

```rust
// src/gitops/mod.rs, before this change
fn run_shell_command(dir: &Path, command: &str) -> Result<std::process::Output, ForgeError> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd.current_dir(dir);
    cmd.output().map_err(|err| ForgeError::TestFailed { /* ... */ })
}
```

`Command::output` waits for the child, and for every descendant that keeps
the pipes open, with no deadline. A test command that never returns hangs
the operation, and the operation hangs whatever called it.

**The contract already promises a bound that does not exist.**

```rust
// src/gitops/mod.rs, before this change
/// Wait timeout for a single test/subprocess invocation. Mirrors
/// the policy adapter pattern: an unresponsive tool surfaces as
/// `TestFailed` rather than hanging the registry.
pub const SUBPROCESS_WAIT_TIMEOUT_SECS: u64 = 600;
```

Nothing reads it. The comment describes a behaviour the code does not have.

**The shared runner is unbounded too, and says so.**

```rust
// src/native/mod.rs, before this change
//! The runner has no timeout. `forge test` has none either; a bounded
//! runner with a process-group kill is a recorded follow-up.
```

So the build step, the generator's post-generation verification and the
readiness fixtures — every Core operation that runs a native command as an
argument array — are unbounded as well.

And the codebase already knows how to kill a process tree, in one place:

```rust
// src/studio/preview.rs
unsafe {
    libc::kill(-(self.child.id() as i32), libc::SIGKILL);
}
```

with the reason written down: "the direct child is often a package manager
whose dev server runs as a grandchild; killing only the direct child would
leak a descendant that keeps the reserved port bound."

## The Decision

### One runner owns the native command

`src/native/mod.rs` is already named "The shared native-command runner". It
keeps that name and gains the responsibility the name implies.

It gains a shell entry point beside the argument-array one:

```rust
pub fn run_native(dir, program, args, env) -> Result<NativeOutcome, ForgeError>
pub fn run_native_shell(dir, command, env) -> Result<NativeOutcome, ForgeError>
```

Both share one implementation. The shell entry point runs `sh -c <command>`
and nothing else; the argument-array entry point runs the program with its
arguments. The shell form exists for one reason: a profile's test command
may rely on shell semantics — pipes, redirections, `&&` — and `forge test`
has always honoured them.

Each form has a bound-explicit sibling —
`run_native_with_timeout`, `run_native_shell_with_timeout` and
`gitops::run_test_with_timeout`. They exist for one reason: the bound is
thirty minutes, and a test cannot wait thirty minutes to prove the bound
works. Every product path binds `NATIVE_COMMAND_TIMEOUT`; no product path
passes another bound. The shape follows the house precedent in
`crate::process::spawn_with_timeout`, which has always taken its bound as a
parameter.

`forge test` then converges:

```rust
// src/gitops/mod.rs, after this change
let outcome = crate::native::run_native_shell(dir, &command, &[])?;
```

and its private `run_shell_command` is deleted. The release lane's test
check calls `crate::gitops::run_test`, so it inherits the bound too, with
no change to that lane.

The runner's capture contract does not move. The captured streams stay
redacted through the policy plane and bounded to the last
[`MAX_CAPTURE_CHARS`] characters, because a failure is reported at the end
of the output and a bound never changes the last characters a reader sees.

One thing does change, and it is a fix: `forge test`'s output is now
redacted. The gitops runner never redacted; the native runner has always
redacted. A test command that printed a credential-looking string used to
put it in the registry row.

### The bound

```rust
/// The wall-clock bound for one native command.
pub const NATIVE_COMMAND_TIMEOUT: Duration = Duration::from_secs(1800);
```

Thirty minutes. The house already uses this number for a long external
operation — `PUBLISH_DEPLOY_TIMEOUT` and `PUBLISH_PROVIDER_TIMEOUT` are both
1800 seconds. It is long enough for a real build or test of a real project
on a cold machine, and short enough that a hang is a failure rather than a
weekend.

The bound is a wall-clock deadline on the command, not on Forge's patience
with it. It is not configurable through the manifest: a bound the project
can raise is not a bound.

### The kill takes the tree

The child is spawned as its own process-group leader, and on the deadline the
whole group is signalled before the child is reaped:

```rust
// the pattern src/studio/preview.rs already uses
command.process_group(0);            // Unix
libc::kill(-(child.id() as i32), libc::SIGKILL);
```

The child is its own group leader, so its group id equals its pid, and a
negative pid targets exactly the tree the runner created. The direct child
is killed and reaped afterwards, so a platform without process groups keeps
the previous behaviour.

The lifecycle helpers live in `src/process.rs`, which is already "the
bounded child-process spawn helper shared by every adapter and workflow that
delegates to an external executable":

```rust
pub fn spawn_in_own_group(command: &mut Command) -> std::io::Result<std::process::Child>
pub fn kill_process_tree(child: &mut std::process::Child)
```

`src/process.rs::spawn_with_timeout` uses both, so every adapter that
already uses that helper — the GitHub CLI, the GitHub adapter, the shared
analytics helper, the Studio preview — gains the tree kill at no contract
change. Their timeouts, their output caps and their error wording are
untouched.

### The capture is bounded too

A command that exits can still leave a descendant behind that holds the
capture pipes open. The old `command.output()` waited for end-of-file on
those pipes, so a descendant that held them open hung the runner forever —
the same hang this change exists to remove.

So the capture has a grace period. The captures are given two seconds to
drain after the command exits; if they have not, the group is killed and
the runner goes on. A descendant that outlived the command does not get to
hold the lane.

This is a behaviour change beyond the literal wording of the timeout: the
runner now kills a process group that outlived a *successful* command. It
is recorded here because it is a real change, not a detail. The alternative
was the hang.

### The outcome names the timeout

`NativeOutcome` gains a `timed_out` flag. A command that overran the bound
is not a command that exited non-zero, and the two are reported differently:

```rust
pub struct NativeOutcome {
    pub exit_code: Option<i32>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub start_error: Option<String>,
    pub timed_out: bool,   // new
}
```

The build step's failure reason names the timeout when the flag is set, and
the exit status otherwise. `forge test` reports a timeout as
`error[test-failed]` with the command, the profile and the bound in the
reason.

## What Is Not Touched, And Why

The codebase has more spawn paths than this change reaches:

| Runner | Owner | Why it is not in this package |
|---|---|---|
| `src/release/engine/operations.rs::run_with_timeout` | the release notes adapter | It runs Forge's own adapter, not the project's native command. |
| `src/policy/driftwatch.rs::run_with_timeout` | the driftwatch adapter | Same. |
| `src/analytics/projections.rs::spawn_with_timeout` | the analytics adapter | Same. |
| `src/provider/runner.rs::run_bounded` | the provider probes | Same. |
| `src/governance/engine.rs::run_adapter` | the Gate's adapter | Same. |

Each of them already has a bounded spawn with a child-only kill. The
remediation path is recorded and short: each can adopt
`crate::process::spawn_in_own_group` and `crate::process::kill_process_tree`
the next time its own contract is touched. What this change does is make the
**native command** path — the one the project's own build and test commands
run through — a single bounded runner.

## User Experience and Interface

Before, a hung test command hung Forge:

```console
$ forge test --project my-app
# ... never returns
```

After, the bound is reached, the tree is killed and the failure names it:

```console
$ forge test --project my-app
error[test-failed]: test command `cargo test` for profile `rust-web` exceeded
the 1800s timeout; the command's process group was killed
```

A command that fails the way it always did still fails the way it always
did:

```console
error[test-failed]: test command `cargo test` for project `my-app` exited with
status 101; see evidence: stdout: ... | stderr: ...
```

The build step's reason gains the same distinction:

```console
native command `npm run build` failed for profile `react-web`: ...
native command `npm run build` exceeded the 1800s timeout for profile `react-web`
```

## Behavioral Model

| Situation | Outcome |
|---|---|
| The command finishes in time | The outcome is what it always was: the exit code, the redacted bounded output, the start error. |
| The command overruns the bound | The whole process group is killed, the child is reaped, and the outcome reports `timed_out`. The caller names the timeout. |
| The command cannot start | The outcome reports the start error, exactly as before. |
| The command spawns descendants | On the deadline the descendants die with it; they are in the group the runner created. |
| A descendant outlives a successful command | The captures are given a grace period; if they do not drain, the group is killed and the runner goes on. |
| The command's output is long | It stays bounded to the last 8000 characters per stream, and redacted. |
| The command uses shell syntax | `forge test` still runs it through `sh -c`. |

## Contract and Compatibility

The `test` operation's contract gains a bound and a tree kill. Nothing about
its inputs changes: the same profile, the same declared test command, the
same working directory, the same error code for a failure.

`forge test`'s output is now redacted. That is a change in the recorded
evidence, and it is the change the shared runner already makes everywhere
else.

`SUBPROCESS_WAIT_TIMEOUT_SECS` is removed. It is a public constant, so a
caller outside this repository could name it; nothing in this repository
does. Its number, 600 seconds, was never read, so removing it changes no
behaviour. The bound that replaces it is 1800 seconds and it is named
`crate::native::NATIVE_COMMAND_TIMEOUT`.

The canonical capability `agent-runtime-workflows` is modified where the
`test` operation's contract is stated, and `profile-and-release-readiness` is
modified where the native evidence rows are stated. Their canonical scenario
names are kept.

## Failure and Boundary Policy

- **A timeout is a failure, never a pass.** The outcome's `success` is
  false, the exit code is absent, and the caller refuses.
- **The group kill is not optional.** The runner puts the child in its own
  group; there is no configuration that leaves the tree running.
- **A platform without process groups keeps the child-only kill.** The
  helper falls back to `child.kill()`.
- **The runner never writes anything.** It runs a command, captures its
  output and reports. It does not persist evidence and it does not journal.
- **The bound does not depend on the project.** It is not a manifest field,
  so a project cannot raise it.

## Verification Oracle

```bash
# The Forge Core test suite
cargo test --workspace --all-targets -- \
  --skip rust_scaffold_builds_and_tests_with_native_toolchain \
  --skip rust_scaffold_builds_with_native_toolchain_without_forge \
  --skip declaration_is_inert_for_native_builds_present_and_absent \
  --test-threads=1

# The repository Gate
./target/debug/forge gate --dry-run
./target/debug/forge gate --timeout-secs 1800

# Strict OpenSpec validation and whitespace
openspec validate --all --strict --no-interactive
node scripts/check-openspec-change-names.mjs
git diff --check
```

A real run: a project whose test command sleeps longer than the bound is
refused with a timeout reason, and the process it spawned is gone with it.

## Implementation Plan

**The lifecycle.** `src/process.rs` gains `spawn_in_own_group` and
`kill_process_tree`, and `spawn_with_timeout` uses them.

**The runner.** `src/native/mod.rs` gains the bound, the group spawn, the
group kill, the shell entry point and the `timed_out` flag.

**The operation.** `src/gitops/mod.rs` deletes `run_shell_command` and the
dead constant, and calls the shared runner.

**The fixtures.** The tests that pin the runner's behaviour — the native
runner's own unit tests, the gitops tests, the build lane's tests — are
updated to the new outcome shape.

**The tests.** New tests pin the timeout, the tree kill and the convergence:
a command that overruns the bound is killed and reported as a timeout; a
command that spawns a descendant leaves no descendant behind; `forge test`
goes through the shared runner.

## Decision Ledger

- **Resolved:** the shared runner in `src/native/mod.rs` is the one native
  command runner. The name was already there; the responsibility now is too.
- **Resolved:** the shell form stays. A profile's test command may rely on
  shell semantics, and `forge test` has always honoured them.
- **Resolved:** the bound is 1800 seconds, the house's value for a long
  external operation. It is not configurable.
- **Resolved:** the kill is a process-group kill, using the pattern
  `src/studio/preview.rs` already established.
- **Resolved:** the process-group lifecycle lives in `src/process.rs`, so
  the adapters that already use that helper gain the tree kill for free.
- **Resolved:** `forge test`'s output is now redacted. The shared runner
  redacts; the gitops runner did not.
- **Resolved:** the adapter runners are out of scope. They run Forge's own
  adapters, and their contracts are recorded in the design with the
  remediation path.
- **Implementation authorized (2026-10-21):** the owner directed the trunk
  repair programme and authorized this package in the approved order, after
  `release-gate-mandatory` closed the release lane's Gate gap.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| One runner runs every native command | `forge test` and the build share one runner | The same bound everywhere | The shared runner owns the contract | `src/native/mod.rs`, `src/gitops/mod.rs` | Every native command | 3.1–3.3 | The native and gitops suites |
| The runner is bounded | A hung command fails | The owner gets a timeout, not a hang | 1800 seconds, not configurable | `src/native/mod.rs` | A command that overruns | 3.2 | The timeout test |
| The kill takes the tree | No orphaned descendants | The host is left clean | Process-group kill, the studio pattern | `src/process.rs` | A command that spawns a child | 3.1–3.2 | The tree-kill test |
| The outcome names the timeout | The failure says "timed out" | The owner reads what happened | `timed_out` on the outcome | `src/native/mod.rs` | A timeout and an ordinary failure | 3.2 | The outcome tests |
