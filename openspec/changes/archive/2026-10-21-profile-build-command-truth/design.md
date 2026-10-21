## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The profile catalogue | `forge` | Yes. `src/profile/catalog.rs`. |
| The descriptor validation and the catalogue door | `forge` | Yes. `src/profile/resolve.rs`. |
| The generated Flutter scaffold | `forge` | Yes. `src/generate/engine.rs`. |
| The release-readiness document | `forge` | Yes. `docs/release-readiness.md`. |
| The build lane | `forge` | Read only. It runs the declared command already. |
| The readiness matrix | `forge` | Read only. It runs the declared build command already. |
| The manifest and the release lane | `forge` | Read only. |

## The Gap

**The profile declares an analyser as its build.**

```rust
// src/profile/catalog.rs, before this change
descriptor(
    "flutter-app",
    "0.1.0",
    "adapter-flutter",
    "dart",
    "flutter",
    Some("3.22"),
    ...
    "flutter analyze",
    "flutter test",
    ...
)
```

The build lane's contract is one sentence: *"Forge Core builds a project with
the build command its profile declares."* The `flutter-app` row breaks that
sentence. `flutter analyze` is `dart analyze` with a Flutter front end: it
reads the sources, prints a report and exits zero. It writes no bundle, no
kernel and no package. The build lane reports the command it ran, so the
report was never a lie about *what ran* — it was a lie about *what that
does*.

**The scaffold's README repeats it.**

```rust
// src/generate/engine.rs, before this change
"Verified with `flutter analyze` + `flutter test` (no platform host or \
 Android SDK needed). The app bundle is a release-stage command that \
 requires `android/` and the Android SDK."
```

A generated project therefore told its own reader that the way to verify it
was an analysis, and that the actual build was something it could not run.

**Nothing checked the shape of a build command.** `validate_descriptor`
checks that a required field is present and non-empty:

```rust
// src/profile/resolve.rs, before this change
} else if profile.build_command.trim().is_empty() {
    Some("build_command")
}
```

Any non-empty string passes. `flutter analyze` passed. The one guard that
would have caught it — a pinned table of the catalogue's build commands —
did not exist.

**And the reason is honest.** `flutter build apk` and `flutter build
appbundle` need the Android SDK and an `android/` directory that the Flutter
scaffold does not have. The analysis command was the only Flutter command
that ran anywhere, so it was declared and called a build. The mistake was
not picking `flutter analyze`; it was that no build was available in the
catalogue that the scaffold could actually run.

## The Decision

### The build is `flutter build bundle`

```rust
// src/profile/catalog.rs, after this change
descriptor(
    "flutter-app",
    ...
    "flutter build bundle",
    "flutter test",
    ...
)
```

`flutter build bundle` compiles the Dart sources and writes the Flutter
asset bundle to `build/flutter_assets/`. It needs:

| Prerequisite | `flutter build bundle` | `flutter build appbundle` |
|---|---|---|
| The `flutter` toolchain | yes | yes |
| The Android SDK | no | yes |
| An `android/` directory | no | yes |
| A platform host | no | yes |

The profile already declares the `flutter` toolchain, and the scaffold
already has no `android/` directory. So `flutter build bundle` is the
build the scaffold can run today, with no new prerequisite and no new
file.

`flutter build bundle` is in the build family `requirement.md` already
names: the canonical families are `dotnet build`, `cargo build`,
`npm run build`, `flutter build` and `python ...`.

The app-store package is not demoted. It is a release-stage command that
needs `android/` and the Android SDK, and the README says so. The planned
`flutter-client` profile already declares `flutter build appbundle` for
exactly that stage, and it is not touched.

### The rule: a named list of analysis verbs

```rust
/// The build-command verbs that only analyse, never build.
///
/// A named list, not an oracle. Forge refuses the verbs it knows and
/// accepts every other command: a rule that classified every command
/// would block legitimate builds, and Forge does not know every
/// toolchain's vocabulary.
pub const ANALYSIS_BUILD_VERBS: [&str; 6] =
    ["analyze", "lint", "clippy", "fmt", "format", "vet"];
```

The verb of a command is its **last token that is not a flag**:

```rust
pub fn build_command_verb(command: &str) -> Option<&str> {
    command.split_whitespace().filter(|t| !t.starts_with('-')).last()
}
```

That definition is what makes the rule work on real commands:

| Command | Verb | Verdict | Why |
|---|---|---|---|
| `flutter analyze` | `analyze` | refused | the analyser itself |
| `cargo clippy --all-targets` | `clippy` | refused | the linter, behind a flag |
| `flutter analyze --fatal-infos` | `analyze` | refused | the flag does not hide the verb |
| `npm run lint` | `lint` | refused | a lint script is not a build |
| `npm run build` | `build` | accepted | a build |
| `npm run build -- --mode=production` | `build` | accepted | the flag is not the verb |
| `python3 -m build` | `build` | accepted | `-m` is a flag; `build` is the verb |
| `flutter build bundle` | `bundle` | accepted | a build |
| `flutter build appbundle` | `appbundle` | accepted | a build |
| `dotnet build` | `build` | accepted | a build |
| `cargo build --release` | `build` | accepted | a build |

The last-token rule is deliberate and its limit is recorded: it reads the
command, not the shell. `flutter analyze && true` has the verb `true` and is
accepted. A rule that parsed the shell would be a second shell parser, and
the profile's build command is a single command by contract.

### The two doors

The rule is one function, and it is called at the two places a build
command is declared.

```rust
fn check_build_command(profile: &ProfileDescriptor) -> Result<(), ForgeError>
```

**Door one — an external descriptor document.**
`validate_descriptor` is the door an external document comes through, and
it already refuses a missing field with `invalid-profile`. The rule refuses
there too, before any project tree is staged:

```console
$ forge ... (external descriptor with `build_command: cargo clippy`)
error[invalid-profile]: profile descriptor 'custom-web' declares the
analysis command `cargo clippy` as its build command; `clippy` analyses, it
does not build; declare a command that produces build output; no files were
changed
```

**Door two — the built-in catalogue.** `inspect_profile` is where every
consumer reads a descriptor: the build lane, the test lane, the resolver,
the preflight, the CLI, the API and the doctor. A catalogue row whose build
command only analyses is refused there, so a catalogue edit that reintroduces
the problem is caught at the door rather than at the build:

```console
$ forge profile inspect flutter-app
error[invalid-profile]: profile descriptor 'flutter-app' declares the
analysis command `flutter analyze` as its build command; ...
```

Both doors refuse with the same code, `invalid-profile`, and the same
reason shape: the profile, the command, the verb.

**The list is not a door.** `list_profiles` returns the catalogue as it is
compiled in. A row that fails the rule stays visible in the list, and the
pinned check below fails, naming the profile. The list is a catalogue, not
a declaration; a broken row stays visible so the operator can see it, and
the check that refuses it says which row and why.

### The catalogue is pinned

A refusal rule catches a bad declaration at the door. A pinned table stops
the bad declaration from being written in the first place, and it makes a
change to a build command a deliberate, reviewed edit.

```rust
/// `(profile, declared build command)` for every supported profile.
const MATRIX: [(&str, &str); 6] = [
    ("aspnet-web", "dotnet build"),
    ("flutter-app", "flutter build bundle"),
    ("nextjs-web", "npm run build"),
    ("python-service", "python3 -m build"),
    ("react-web", "npm run build"),
    ("rust-web", "cargo build"),
];
```

The contract test reads the catalogue and compares it with the table. A
changed row fails the test and names the profile. The test also asserts
that no supported row carries an analysis verb, so the table and the rule
agree.

This is the house pattern: `tests/workspace_metadata_contract.rs` pins the
governance profile and the native test command per profile in exactly this
shape.

## What Is Not Touched, And Why

| Surface | Why it is not in this package |
|---|---|
| The build lane (`src/build/mod.rs`) | It runs the profile's declared command, reports it and fails on a non-zero exit. That is already the contract; the catalogue was the part that was wrong. |
| The readiness matrix (`src/readiness/mod.rs`) | It runs the declared build command for the native evidence row. It inherits the corrected command. |
| The test command | A test command that analyses is a different question, and the test lane's contract does not change. |
| The planned profiles | `flutter-client` keeps `flutter build appbundle`; the others keep their own builds. None of them is a supported profile, and none of them declares an analysis command. |
| The manifest | No field is added, renamed or defaulted. The manifest stores the profile id; the command comes from the catalogue. |
| The release and deploy lanes | Their contracts do not change. |

## User Experience and Interface

Before, a Flutter build reported a build that built nothing:

```console
$ forge build ./my-flutter-app
project: my-flutter-app
profile: flutter-app
command: flutter analyze
status: passed
exit_code: 0
```

After, the same command builds the project:

```console
$ forge build ./my-flutter-app
project: my-flutter-app
profile: flutter-app
command: flutter build bundle
status: passed
exit_code: 0
```

And a descriptor that declares an analysis command is refused before anything
runs:

```console
error[invalid-profile]: profile descriptor 'custom-web' declares the
analysis command `cargo clippy` as its build command; `clippy` analyses, it
does not build; declare a command that produces build output; no files were
changed
```

The generated scaffold's README now says what the project actually does:

```text
Built with `flutter build bundle` and verified with `flutter test` (no
platform host or Android SDK needed). The app store package
(`flutter build appbundle`) is a release-stage command that requires
`android/` and the Android SDK.
```

## Behavioral Model

| Situation | Outcome |
|---|---|
| A profile declares a build command with an unnamed verb | The descriptor is accepted. |
| A profile declares a build command with a named analysis verb | The descriptor is refused with `invalid-profile`, naming the profile, the command and the verb. No file is written. |
| A profile declares a build command whose verb is followed only by flags | The verb is the last non-flag token, and the refusal names it. |
| A built-in catalogue row declares an analysis command | Inspecting that profile is refused with the same reason. The catalogue row stays visible in the list, and the pinned check fails naming the profile. |
| The catalogue's build command is changed on purpose | The pinned check fails until the table is updated in the same change. That is the intended friction. |

## Failure and Boundary Policy

- **The refusal is `invalid-profile`.** The same code the descriptor
  validation already uses, so a caller does not need a new branch.
- **The refusal happens before any file is written.** A refused descriptor
  stages no project tree.
- **The reason names three things.** The profile, the command and the
  verb. A refusal that names only the code is not actionable.
- **The rule never guesses.** An unknown verb is accepted. Forge refuses
  what it knows and does not pretend to classify the rest.
- **The rule does not reach the test command.** A test command is a
  different contract.
- **The list is not filtered.** A broken catalogue row stays visible, and
  the pinned check names it.

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

1. `forge new --profile flutter-app` names `flutter build bundle` as the
   build command, and the generated README names it too.
2. `forge build` on that scaffold runs `flutter build bundle` and passes —
   the Flutter SDK is on this host, so the build really runs.
3. A descriptor that declares `cargo clippy` as its build command is
   refused with `invalid-profile`, naming the profile, the command and the
   verb.

## Implementation Plan

**The catalogue.** `src/profile/catalog.rs`: the `flutter-app` build
command becomes `flutter build bundle`.

**The rule.** `src/profile/resolve.rs` gains `ANALYSIS_BUILD_VERBS`,
`build_command_verb` and `check_build_command`. `validate_descriptor`
calls it after the missing-field check; `inspect_profile` calls it for both
catalogues.

**The scaffold.** `src/generate/engine.rs`: the README sentence names the
build and the release-stage package.

**The document.** `docs/release-readiness.md`: the Flutter prerequisite line
names the build command the profile now declares.

**The tests.** `src/profile/resolve.rs` gains unit tests for the verb
reading and the refusal at both doors. The new
`tests/profile_build_command_truth_contract.rs` pins the catalogue's build
commands and checks the refusal through the library.

## Decision Ledger

- **Resolved:** the Flutter build is `flutter build bundle`. It is the
  build the scaffold can run with the toolchain the profile already
  declares; the app-store package stays a release-stage command.
- **Resolved:** the rule is a named list of six analysis verbs, and an
  unknown verb is accepted. Forge refuses what it knows.
- **Resolved:** the verb is the last non-flag token of the command.
- **Resolved:** the rule is applied at the two declaration doors —
  `validate_descriptor` for an external document and `inspect_profile` for
  the built-in catalogue.
- **Resolved:** `list_profiles` is not a door. The list is a catalogue; a
  broken row stays visible and the pinned check names it.
- **Resolved:** the catalogue's build commands are pinned by a contract
  test, in the house's `MATRIX` shape.
- **Resolved:** the test command is not governed.
- **Implementation authorized (2026-10-21):** the owner directed the trunk
  repair programme and authorized this package in the approved order, after
  `one-native-command-runner` closed the runner gap the build lane shared.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| A profile's build command builds | The catalogue stops declaring an analyser | The build produces output | `flutter build bundle`; a named analysis verb is refused | `src/profile/catalog.rs`, `src/profile/resolve.rs` | Refusal at both doors; an unknown verb is accepted | 3.1–3.4 | The contract test; the real build |
| The catalogue's build commands are pinned | A build command cannot change unnoticed | The table names the row | The pinned `MATRIX` | `tests/profile_build_command_truth_contract.rs` | The table matches; a changed row fails | 3.5 | The contract test |
| The catalog does not present an analysis command as a build | Inspection refuses | The operator reads why | `profile-registry` gains a scenario | `src/profile/resolve.rs` | A catalogue row that analyses is refused | 3.2 | The contract test |
