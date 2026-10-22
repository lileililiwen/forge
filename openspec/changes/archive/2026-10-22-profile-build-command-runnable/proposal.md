# Change: profile-build-command-runnable

## Why

`forge build` builds a project with the build command its profile
declares. The `python-service` profile declares:

```rust
// src/profile/catalog.rs, before this change
descriptor(
    "python-service",
    ...
    "python3 -m build",                     // the build command
    "python3 -m unittest discover -s tests -v", // the test command
    ...
)
```

`python3 -m build` is the PyPA build frontend. It is a **pip package**,
not part of the Python toolchain: `python3` alone cannot run it. The
profile declares the toolchain `python3` (version `3.12`), the profile's
package list (`fastapi`, `sqlalchemy`, `alembic`, `pytest`) does not name
it, and the scaffold does not ship it — the generated project is
dependency-free on purpose.

So the declared build command cannot run with the toolchain the profile
declares, and the build lane fails on a stock Python:

```console
$ forge build ./my-python-service
error[build-failed]: build command failed: build command `python3 -m build`
for project `my-python-service` exited with status 1; see evidence:
stderr: /usr/bin/python3: No module named build.__main__; 'build' is a
package and cannot be directly executed
```

The generated project also says something false about itself. Its README
lists the native commands and then states:

> FastAPI/SQLAlchemy dependencies from the profile descriptor are resolved
> per-service; this scaffold ships dependency-free so native commands run
> without network.

The test command honours that promise. The build command cannot: making
it run needs `pip install build`, which is a network install of a package
the profile never declared.

The repository already knows this. The scaffold's own contract test says
so in a comment and proves the opposite command instead:

```rust
// tests/generate_contract.rs, before this change
// pytest/build backends are external and may be absent; the stdlib
// compile check proves the scaffold is syntactically portable without
// claiming a pytest run.
let compile = Command::new("python3")
    .args(["-m", "compileall", "-q", "app", "tests"])
```

The test command was moved to the standard library for exactly this
reason. The build command was left declaring a package that is not the
toolchain.

And nothing held the profile to its promise. The readiness matrix runs
the declared build command and reports `unverified` when a prerequisite is
missing, so the row stayed honest about its own environment while the
catalogue kept declaring a build the toolchain does not provide. The
release-readiness document records the extra package as a runner
prerequisite — as if it were the Flutter SDK, which *is* the profile's
declared toolchain.

## What Changes

- **The `python-service` build command runs with the toolchain the
  profile declares.** It becomes `python3 -m compileall -q app tests` —
  the standard library compile of the source roots the profile's own
  layout names. It needs nothing beyond `python3`, and no network.
- **The generated project's README becomes true.** Its native-command
  block is rendered from the descriptor, so it names the command that
  runs; the "dependency-free so native commands run without network"
  claim now holds for the build as well as the test.
- **The release-readiness document stops naming a package the profile
  does not declare.** The `python-service` row needs the `python3`
  toolchain and nothing else.
- **The declared build commands are runnable, and that is checked.** A
  repository contract check generates a project for every supported
  profile and runs the profile's declared build command through Forge's
  own build lane; the check fails, naming the profile, when the command
  cannot run. The check skips a profile whose toolchain the host does
  not have, in the house's own way, and asserts the rest.
- **The build command's program is the declared toolchain.** A profile
  whose build command ran something other than its own toolchain would
  fail the same check.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The profile catalogue | `forge` | Yes | `src/profile/catalog.rs`: the `python-service` build command. |
| The build lane | `forge` | Read only | `src/build/mod.rs` already runs the declared command and reports it; it inherits the corrected command. |
| The readiness matrix | `forge` | Read only | `src/readiness/mod.rs` already runs the declared build command for the native evidence row. |
| The generated python scaffold | `forge` | Read only | `src/generate/engine.rs`: the README's native-command block is rendered from the descriptor. The `.gitignore` already ignores `__pycache__/` and `*.pyc`. |
| The release-readiness document | `forge` | Yes | `docs/release-readiness.md`: the `python-service` prerequisite line. |
| The pinned build-command table | `forge` | Yes | `tests/profile_build_command_truth_contract.rs`: the `python-service` row. |
| The build-command runnable check | `forge` | Yes | `tests/profile_build_command_runnable_contract.rs`: the new guard. |
| The scaffold contract test | `forge` | Read only | `tests/generate_contract.rs` keeps its compile check and its other assertions. |
| The manifest, the release lane, the deploy lane | `forge` | Read only | Their contracts do not change. |
| The `profile-build-command-truth` capability | `forge` | Read only | Its requirements already hold; no wording moves. |
| The `deterministic-project-generation` capability | `forge` | Modified | The portable-project requirement names the runnable build command. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md:701` (§15 Deterministic Generation) requires a generated project to build with its native toolchain; `requirement.md:413` (§9 Profiles) makes the profile's declared commands the profile's contract. |
| Contracts | The descriptor's `build_command` field keeps its shape. No field is added, renamed or defaulted. The build lane's contract does not change. |
| Callers | `forge build` (CLI), the readiness matrix, the release lane and the API all read the descriptor through `inspect_profile`. |
| Persistence | Nothing is persisted. The manifest stores the profile id, not the command. |
| Integrations | The Python toolchain only. `python3 -m compileall` is in the standard library of every supported Python 3. |
| Failures | No new error code. The `build-failed` refusal the lane already raises is what a failing build command produces. |
| Tests | A new `tests/profile_build_command_runnable_contract.rs` runs every profile's declared build command; the pinned table in `tests/profile_build_command_truth_contract.rs` carries the new `python-service` row. |
| Dependencies | None added. The change removes a dependency on an undeclared package. |
| Concerns | Quality and compatibility: the guard runs the declared command for every profile whose toolchain the host has. Security: no new file, network or credential surface; the build lane still runs the command in the project directory. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `profile-build-command-runnable` | forge | Rust | added | this package |
| `deterministic-project-generation` | forge | Rust | modified | the portable-project requirement names the runnable build command |

## Non-Goals

- **The profile does not gain a field for extra tooling.** The rule is
  that the build command runs with the declared toolchain, not that the
  descriptor can declare a second prerequisite.
- **The build does not become a packaging build.** The profile's build
  compiles the project's Python sources; it does not produce a wheel or
  an sdist. The scaffold deploys from source — its Dockerfile copies the
  tree and runs `python3 -m app.main`.
- **The `pyproject.toml` is untouched.** The generated project stays a
  PEP 517 package, so an operator who installs `build` can still package
  it. The declared build command is not that packaging step.
- **The test command is not changed.** `python3 -m unittest discover -s
  tests -v` already runs with the declared toolchain alone, and it stays.
- **The planned profiles are not promoted.** Their build commands are
  not judged by this change.
- **The other profiles' build commands are untouched.** `cargo build`,
  `npm run build`, `flutter build bundle` and `dotnet build` already run
  with the toolchain their profile declares; the new check holds them to
  it.
- **The build lane's contract does not change.** It still runs the
  profile's declared command, reports it and fails on a non-zero exit.
- **The readiness matrix's contract does not change.** It still runs the
  declared build command for the native evidence row and still reports
  `unverified` when a prerequisite is missing.
