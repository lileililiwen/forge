# Change: profile-build-command-truth

## Why

`forge build` builds a project with the build command its profile declares.
The `flutter-app` profile declared `flutter analyze`:

```rust
// src/profile/catalog.rs, before this change
descriptor(
    "flutter-app",
    ...
    "flutter analyze",   // the build command
    "flutter test",      // the test command
    ...
)
```

`flutter analyze` is a static analyser. It reads the Dart sources, prints
"No issues found!" and exits zero. It produces no build output: no asset
bundle, no kernel, no app package. So `forge build` on a Flutter project
reported a passed build that built nothing, and the report named the command
it ran — the truth was visible, but the truth was wrong.

The reason it was chosen is recorded in the repository itself. `flutter
build apk` and `flutter build appbundle` need the Android SDK and an
`android/` directory, and the Flutter scaffold has neither. An analysis
command was the only Flutter command that ran without them, so the profile
declared the analysis command and called it a build.

There is a build that needs only the toolchain the profile already declares.
`flutter build bundle` compiles the Dart sources and writes the Flutter
asset bundle (`build/flutter_assets/`) with no platform host, no Android SDK
and no `android/` directory. The planned `flutter-client` profile already
declares `flutter build appbundle` for the release-stage package; nothing
declared the build a Flutter project can actually run today.

And the rule that let a lying build command through is not written down
anywhere. A descriptor may declare any string as its build command, and
nothing checks that the command builds. The only guard is the pinned
catalogue table, and the pinned table did not exist.

## What Changes

- **The Flutter profile declares a build.** `flutter-app`'s build command
  becomes `flutter build bundle`. The app-store package stays a
  release-stage command and the README says so.
- **A build command that only analyses is refused.** A declared build
  command whose verb is a named analysis verb — `analyze`, `lint`,
  `clippy`, `fmt`, `format`, `vet` — is refused with `invalid-profile`,
  naming the profile, the command and the verb. The refusal happens at
  both declaration doors: an external descriptor document and the
  built-in catalogue.
- **The catalogue's build commands are pinned.** A repository contract
  check pins every supported profile's build command, so a build command
  cannot change in the catalogue without that check failing and naming
  the profile.
- **The verb is read past the flags.** The verb of a command is its last
  token that is not a flag, so `cargo clippy --all-targets` is refused and
  `npm run build -- --mode=production` is not.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The profile catalogue | `forge` | Yes | `src/profile/catalog.rs`: the `flutter-app` build command. |
| The descriptor validation | `forge` | Yes | `src/profile/resolve.rs`: the rule and the two doors. |
| The generated Flutter scaffold | `forge` | Yes | `src/generate/engine.rs`: the README sentence. |
| The release-readiness document | `forge` | Yes | `docs/release-readiness.md`: the prerequisite line. |
| The build lane | `forge` | Read only | `src/build/mod.rs` already runs the declared command and reports it. |
| The readiness matrix | `forge` | Read only | `src/readiness/mod.rs` already runs the declared build command. |
| The manifest, the release lane, the deploy lane | `forge` | Read only | Their contracts do not change. |
| The `target-addressed-build` and `profile-and-release-readiness` capabilities | `forge` | Read only | Their requirements already hold; no wording moves. |
| The external profile documents | `forge` | Yes | The declaration door. |
| The doctor's profile checks | `forge` | Read only | It reads descriptors through the same door. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md:90` lists `flutter build` as a canonical build family; `flutter build bundle` is that family. |
| Contracts | The descriptor's `build_command` field keeps its shape. No field is added, renamed or defaulted. |
| Callers | The build lane, the readiness matrix, the CLI and the API all read the descriptor through `inspect_profile`. |
| Persistence | Nothing is persisted. The catalogue is compiled in; the manifest stores the profile id, not the command. |
| Integrations | The Flutter SDK is the only toolchain involved. `flutter build bundle` needs no Android SDK and no platform host. |
| Failures | The refusal is `invalid-profile`, the code the descriptor validation already uses. |
| Tests | A new `tests/profile_build_command_truth_contract.rs` pins the catalogue and the refusal. |
| Dependencies | None added. The deny-list is a named constant in `src/profile/resolve.rs`. |
| Concerns | Quality and compatibility: the rule is a named list, not an oracle, so an unknown verb is accepted. Security: no new file, network or credential surface. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `profile-build-command-truth` | forge | Rust | added | this package |
| `profile-registry` | forge | Rust | modified | the catalog no longer presents an analysis command as a build |

## Non-Goals

- **The test command is not governed.** A test command that analyses is a
  different question, and the test lane's contract does not change.
- **The deny-list is not an oracle.** Forge refuses the six verbs it
  names and accepts every other command. A rule that classified every
  command would block legitimate builds, and Forge cannot know every
  toolchain's vocabulary.
- **The release-stage package is not the profile's build.**
  `flutter build appbundle` still needs the Android SDK and an `android/`
  directory, so it stays a release-stage command and is named as such in
  the README.
- **The planned profiles are not promoted.** `flutter-client` stays
  planned and keeps `flutter build appbundle`.
- **The build lane's contract does not change.** It still runs the
  profile's declared command, reports it and fails on a non-zero exit.
- **The catalogue's other rows do not change.** `dotnet build`,
  `npm run build`, `cargo build` and `python3 -m build` were already
  builds and stay exactly as they are.
