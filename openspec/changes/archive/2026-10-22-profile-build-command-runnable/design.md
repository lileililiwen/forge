## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The profile catalogue | `forge` | Yes. `src/profile/catalog.rs`. |
| The release-readiness document | `forge` | Yes. `docs/release-readiness.md`. |
| The pinned build-command table | `forge` | Yes. `tests/profile_build_command_truth_contract.rs`. |
| The build-command runnable check | `forge` | Yes. `tests/profile_build_command_runnable_contract.rs`. |
| The build lane | `forge` | Read only. It runs the declared command already. |
| The readiness matrix | `forge` | Read only. It runs the declared build command already. |
| The generator | `forge` | Read only. The README's native-command block is rendered from the descriptor. |
| The scaffold contract test | `forge` | Read only. |

## The Gap

**The build command needs a package the profile does not declare.**

```rust
// src/profile/catalog.rs, before this change
descriptor(
    "python-service",
    "0.1.0",
    "adapter-python",
    "python",
    "python3",            // the toolchain
    Some("3.12"),
    ...
    &["fastapi", "sqlalchemy", "alembic", "pytest"],  // the packages
    ...
    "python3 -m build",                            // the build command
    "python3 -m unittest discover -s tests -v",    // the test command
    ...
)
```

`python3 -m build` is the PyPA build frontend. It is a pip package. The
profile's toolchain is `python3` and the profile's packages do not name
it, so nothing the profile declares provides it.

**So the build lane fails on a stock Python.**

```console
$ forge build ./my-python-service
error[build-failed]: build command failed: build command `python3 -m build`
for project `my-python-service` exited with status 1; see evidence:
stderr: /usr/bin/python3: No module named build.__main__; 'build' is a
package and cannot be directly executed
```

**And the generated project says something false about itself.**

```text
## Native commands

```sh
python3 -m build
python3 -m unittest discover -s tests -v
```

FastAPI/SQLAlchemy dependencies from the profile descriptor are resolved
per-service; this scaffold ships dependency-free so native commands run
without network.
```

The test command honours the promise. The build command cannot: running it
needs `pip install build`, which is a network install of a package the
profile never declared. The scaffold is dependency-free on purpose.

**The repository already knows this.** Its own scaffold contract test
states it and proves the opposite command:

```rust
// tests/generate_contract.rs, before this change
// pytest/build backends are external and may be absent; the stdlib
// compile check proves the scaffold is syntactically portable without
// claiming a pytest run.
let compile = Command::new("python3")
    .args(["-m", "compileall", "-q", "app", "tests"])
```

The test command was moved to the standard library for exactly this
reason. The build command was left behind.

**And the requirement that is broken says so in plain words.** The
canonical `deterministic-project-generation` capability requires:

> Forge SHALL generate the five MVP profile projects with manifests and
> documented native build/test commands, without a Forge runtime
> dependency.
>
> #### Scenario: Portable generated projects success
>
> - **WHEN** a generated fixture is built using its stack toolchain with
>   Forge unavailable
> - **THEN** the native build and applicable tests succeed

A generated `python-service` project cannot be built with its stack
toolchain. The requirement does not hold.

## The Decision

### The build compiles the project's own sources

```rust
"python3 -m compileall -q app tests",
```

The standard library compile of the source roots the profile's own layout
declares:

| Path | In the profile's layout |
|---|---|
| `app/` | yes — the service package |
| `tests/` | yes — the test package |

`compileall` is in the standard library of every supported Python 3, it
needs no network, and it fails on a syntax error — which is what a build
should do. The output is byte-compiled modules; the generated project's
`.gitignore` already ignores `__pycache__/` and `*.pyc`.

This is the same choice the profile already made for its test command:
the standard library instead of a third-party runner. The profile's test
command runs `unittest`, not `pytest`, for exactly the reason its build
command now runs `compileall`, not `build`.

**The build does not become a packaging build.** The profile's generated
project deploys from source — its Dockerfile copies the tree and runs
`python3 -m app.main` — so the build's job is to prove the sources
compile. The `pyproject.toml` stays a PEP 517 package, so an operator who
installs `build` can still package the service; that is not the declared
build.

### The command is scaffold-shaped, like the others

`npm run build` names a script the scaffold ships (`scripts/build.mjs`);
it does not run in a project that has no `build` script. `python3 -m
compileall -q app tests` names the source roots the profile's layout
declares; it does not run in a project with a different layout. Both
commands describe the stack Forge generates, and both are honest about
the project they are run in.

### The declared build commands are runnable, and the repository checks it

A rule that only one profile follows is a convention, not a contract. The
new check holds every profile to it:

```rust
for descriptor in list_profiles() {
    // The command's program is the profile's declared toolchain.
    // The host's toolchain decides whether the command can be run here.
    // Generate a project for the profile and run the declared build
    // command through Forge's own build lane.
}
```

Two rules, one check:

| Rule | Fails when |
|---|---|
| The build command's program is the profile's declared toolchain | A profile's build runs something other than its own toolchain. |
| The declared build command succeeds in a project the profile generates | The command cannot run with the toolchain the profile declares — the defect this change closes. |

The check uses Forge's own build lane, so it exercises the command, the
runner, the working directory and the report together, not a hand-rolled
invocation.

**A profile whose toolchain the host lacks is skipped, in the house's
own way.** The repository's existing native checks do exactly this:

```rust
// tests/generate_contract.rs
if Command::new("dotnet").arg("--version").output().is_err() {
    return;
}
```

The check skips that profile, says nothing about it, and asserts every
profile whose toolchain is present. The skipped profile is not claimed
to pass; it is not claimed at all.

**The pinned table carries the new row.** Change 16's contract test pins
the catalogue's build commands, so the `python-service` row changes there
in the same change. That is the intended friction: a build command cannot
change without the table changing with it.

### The release-readiness document stops naming a package the profile does not declare

```text
- `python-service` (`python3 -m compileall -q app tests` /
  `python3 -m unittest discover -s tests -v`) needs only the `python3`
  toolchain; neither command needs a third-party runner.
```

The Flutter SDK stays a runner prerequisite, because it **is** the
profile's declared toolchain. The `build` package was not a toolchain;
it was an undeclared dependency dressed as a prerequisite.

## What Is Not Touched, And Why

| Surface | Why it is not in this package |
|---|---|
| The build lane (`src/build/mod.rs`) | It runs the profile's declared command, reports it and fails on a non-zero exit. That is already the contract; the catalogue was the part that was wrong. |
| The readiness matrix (`src/readiness/mod.rs`) | It runs the declared build command for the native evidence row and still reports `unverified` when a prerequisite is missing. It inherits the corrected command. |
| The test command | `python3 -m unittest discover -s tests -v` already runs with the declared toolchain alone. It stays. |
| The other five profiles' build commands | `cargo build`, `npm run build`, `flutter build bundle` and `dotnet build` already run with the toolchain their profile declares. The new check holds them to it; it does not change them. |
| The generated `pyproject.toml` | The project stays a PEP 517 package, so an operator who installs `build` can still package it. The declared build command is not that step. |
| The generated `.gitignore` | It already ignores `__pycache__/`, `*.pyc`, `dist/`, `build/` and `*.egg-info/`. The build output is already ignored. |
| The generator (`src/generate/engine.rs`) | The README's native-command block is rendered from the descriptor, so it names the new command without a source change. |
| The descriptor contract | No field is added, renamed or defaulted. The manifest stores the profile id, not the command. |
| The planned profiles | They are not judged by this change. |
| The scaffold contract test (`tests/generate_contract.rs`) | It keeps its offline compile check, which now proves the declared command's own superset. |

## User Experience and Interface

Before, `forge build` on a generated Python service failed:

```console
$ forge build ./my-python-service
error[build-failed]: build command failed: build command `python3 -m build`
for project `my-python-service` exited with status 1; see evidence:
stderr: /usr/bin/python3: No module named build.__main__; 'build' is a
package and cannot be directly executed
```

After, it builds with the toolchain the profile declares:

```console
$ forge build ./my-python-service
project: my-python-service
profile: python-service
command: python3 -m compileall -q app tests
target: host
status: passed
exit_code: 0
note: build command `python3 -m compileall -q app tests` passed for project
`my-python-service`
```

And the generated README names the command that runs:

```text
## Native commands

```sh
python3 -m compileall -q app tests
python3 -m unittest discover -s tests -v
```

FastAPI/SQLAlchemy dependencies from the profile descriptor are resolved
per-service; this scaffold ships dependency-free so native commands run
without network.
```

## Behavioral Model

| Situation | Outcome |
|---|---|
| A generated `python-service` project is built | The build compiles the project's own sources with `python3` alone, offline, and reports `passed`. |
| The project's sources have a syntax error | The compile fails and the build reports `build-failed`, naming the command. |
| A `python-service` project with a different layout is built | The command compiles the source roots the profile's layout names; a project without them reports the failure rather than a silent success. |
| The other five profiles | Unchanged: their declared build commands run with their own toolchains, and the new check holds them to it. |
| A host without a profile's toolchain | The check skips that profile and says nothing about it; the readiness matrix still reports the row `unverified` with the prerequisite named. |
| The catalogue's build command is changed on purpose | The pinned table in change 16's contract test fails until the table is updated in the same change. That is the intended friction. |
| The declared build command's program is not the toolchain | The new check fails, naming the profile. |

## Failure and Boundary Policy

- **The refusal is the lane's existing `build-failed`.** No new error
  code: a build command that exits non-zero is a failed build, and the
  lane already says so.
- **The build stays in the project directory.** The command runs where
  the project is, and it writes its output into the project.
- **The check never claims a pass it did not observe.** A profile whose
  toolchain is missing is skipped, not passed.
- **The check does not reach the planned profiles.** They are not
  selectable and not judged.
- **The check does not change the build lane's contract.** It observes it.
- **The build command's program is the toolchain.** A profile cannot run
  another toolchain's command and call it its own.

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

Three real runs:

1. `forge new --profile python-service` then `forge build` on the
   generated project: the build runs
   `python3 -m compileall -q app tests` and passes, with no network and
   no package beyond `python3`.
2. The generated project's README names that command, and the
   "dependency-free so native commands run without network" claim now
   holds for the build as well as the test.
3. `forge build` on the generated projects of the other five profiles
   still passes, with their own toolchains.

## Implementation Plan

**The catalogue.** `src/profile/catalog.rs`: the `python-service` build
command becomes `python3 -m compileall -q app tests`.

**The document.** `docs/release-readiness.md`: the `python-service`
prerequisite line names the two commands and says the row needs only the
`python3` toolchain.

**The pinned table.** `tests/profile_build_command_truth_contract.rs`:
the `python-service` row carries the new command.

**The check.** `tests/profile_build_command_runnable_contract.rs`: the
program rule and the runnable rule, for every supported profile whose
toolchain the host has.

## Decision Ledger

- **Resolved:** the `python-service` build command is
  `python3 -m compileall -q app tests` — the standard library compile of
  the source roots the profile's layout declares.
- **Resolved:** the build is not a packaging build. The project deploys
  from source; the `pyproject.toml` stays a PEP 517 package for an
  operator who wants to package it.
- **Resolved:** the test command is not changed. It already runs with the
  declared toolchain alone.
- **Resolved:** the check runs the declared command through Forge's own
  build lane, and skips a profile whose toolchain the host lacks, in the
  house's own way.
- **Resolved:** the build command's program is the profile's declared
  toolchain.
- **Resolved:** the pinned table in change 16's contract test carries the
  new row.
- **Resolved:** the release-readiness document stops naming the `build`
  package as a prerequisite; the Flutter SDK stays one, because it is the
  profile's declared toolchain.
- **Resolved:** the Node ecosystem's `node` and `npm` are one toolchain
  for the program rule. The catalogue pins `nextjs-web` to `node` and
  `react-web` to `npm`, and both build with `npm run build`. The
  repository already maps both names to the same ecosystem
  (`src/kit/registry.rs::Ecosystem::for_toolchain`) and gives both the
  same build target form (`src/target/mod.rs::target_form_for`), so the
  rule accepts either program for a Node toolchain. The catalogue was
  not re-pinned: that would move the kit registry's pin, the readiness
  row's toolchain evidence and the release-readiness document's
  `node`/`v24` line, all outside this change's boundary.
- **Implementation authorized (2026-10-22):** the owner directed the
  trunk repair programme and authorized this package in the approved
  order, after `import-profile-detection-truth` closed the import lane's
  profile gap.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| A profile's build command runs with the declared toolchain | The Python build works on a stock Python | The build passes | The standard library compile | `src/profile/catalog.rs` | The command runs; a missing toolchain is skipped, not claimed | 3.1, 3.4 | The contract test; the real build |
| The declared build commands are checked | A profile cannot declare an unrunnable build | The check names the profile | The program rule and the runnable rule | `tests/profile_build_command_runnable_contract.rs` | Every profile with a toolchain runs; a broken command fails the check | 3.4 | The contract test |
| The portable generated projects state the rule | The contract names the runnable build | The report is consistent | `deterministic-project-generation` gains a scenario | `src/profile/catalog.rs` | A generated project builds with its stack toolchain | 3.1 | The contract test |
| The catalogue's pinned build commands carry the new row | The build command cannot change unnoticed | The table names the row | The pinned `MATRIX` | `tests/profile_build_command_truth_contract.rs` | The table matches the catalogue | 3.3 | The contract test |
