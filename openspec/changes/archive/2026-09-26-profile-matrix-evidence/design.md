# Design: Profile matrix evidence

## Ownership and boundaries

| Concern | Owner | This package's role |
| --- | --- | --- |
| The `passed` / `failed` / `unverified` row vocabulary and the runner/profile contract | Forge's readiness plane (`forge readiness`, contract `0.1.0`) | Unchanged. The verdict rules are already correct; the evidence behind them is not |
| What a runner qualifies | `.github/workflows/ci.yml`, i.e. `artifact-and-ci-baseline` | This package only states which rows are *possible* to qualify and refuses to claim the rest |
| Template content per profile | `src/generate/mod.rs` and `src/profile` descriptors | Changed where a declared command does not match the shipped tree |
| A generated project's own verification command | the descriptor, by the single-source-of-truth rule established in `workspace-metadata-emission` | Kept consistent with the matrix row, and the receipt/refresh path re-proved per profile |
| Adoption, registration, deployment of governed projects | Workspace Governance / Jenkins-local | Not touched. A passing matrix is not adoption and not deployability |

The rule that keeps this honest: **a row may only move from `failed` to
`passed` because the generated tree changed or the declared command changed to
one the tree satisfies.** A row may never move because the matrix stopped
reporting it, because the failure was silenced, or because the command was
replaced by a no-op.

## The three rows, separately

### `react-web` — DOM-free test in a bare Node process

The generated `src/app.test.mjs` runs under `node --test` and reads
`document`, which Node does not provide. The template ships `index.html`,
`src/main.js`, `src/app.test.mjs`, `scripts/build.mjs`, `vite.config.js` and
`package.json`.

Two honest options:

1. **Scope the shipped test to what is actually testable without a browser.**
   Keep `node --test` as the declared command and assert on the pure surface the
   template already contains (the app's state/derivation functions, the module's
   exported contract, the build script's output paths), removing the `document`
   read entirely. The template makes no claim about DOM behaviour, so nothing is
   lost — it is simply not asserted by a scaffold test.
2. **Provision a DOM.** Either add `jsdom` as a dev dependency with an install
   step, or switch to a runner that supplies one. This makes `npm test` depend on
   a package fetch, which contradicts the deterministic-first rule and adds a
   network prerequisite to a row meant to run on a clean runner.

Option 1 is chosen. Option 2 is recorded as rejected here with its reason:
a scaffold whose verification depends on a package registry is not reproducible
in the way `requirement.md` §3.1 and the checklist's Forge-absent procedure
require. The test file must still fail if the app contract breaks, so the
assertions have to be real rather than deleted — the diff is reviewed against
"does this test still detect a broken template".

### `python-service` — no third-party test runner

`python3 -m pytest` presumes `pytest` is importable; on a bare host it is not,
and the row fails with `No module named pytest`. The build command
(`python3 -m build`) already passes.

Change the descriptor test command to the standard library runner that the
generated tree satisfies without any install:

```text
build_command: python3 -m build
test_command:  python3 -m unittest discover -s tests -v     (design decision; exact form fixed by the template's layout)
```

and ship the template's tests so they are `unittest`-compatible. Two consequences
are tracked rather than waved through:

- The same value is written into a generated project's `.project.json`
  `verification.command`, so the emitted declaration changes content for the
  profile, which re-enters the stale-unedited refresh path and the receipt hash.
- `python3 -m build` is itself a third-party module. Verified on this host it is
  present, but the same hidden-prerequisite class of failure applies to it, so
  the row's runner prerequisite is recorded explicitly and the matrix reports
  `unverified` when it is absent instead of `failed`-as-if-a-code-defect. That
  distinction is part of this package's deliverable.
- Keeping `pytest` as an *optional* richer runner is acceptable only if the
  declared command works without it; making the declared command work is the
  requirement.

### `flutter-app` — the command does not match the tree

`flutter build appbundle` requires an Android host platform
(`android/app/build.gradle`) plus an Android SDK. The template ships Dart source
and `pubspec.yaml`, not a platform host.

Options:

1. **Ship the platform host files.** `flutter create --platforms=android`
   generates a large, version-coupled tree (Gradle wrapper, manifest, Kotlin
   entry point, `build.gradle` pair). Vendoring that into a deterministic
   template ties the scaffold to a Flutter/AGP/Gradle version matrix the
   repository cannot test on every runner, and it still needs an Android SDK to
   verify.
2. **Declare a build command the shipped tree performs.** `flutter analyze`
   (static contract, no platform host) as build, `flutter test` as test — both
   run against a Dart-only tree and both fail loudly when the scaffold breaks.
   A real platform bundle then belongs to the project's own release pipeline,
   where the operator has the SDK, which is exactly how Forge's release plane
   treats stage commands.

Option 2 is chosen, with option 1 recorded as rejected for the version-coupling
and SDK-prerequisite reasons. The consequence is stated instead of hidden:
**`forge new --profile flutter-app` no longer claims to produce a
buildable app bundle; it produces a verified Dart project.** The descriptor's
`capabilities`/`deployment` notes and `docs/requirements-coverage.md` §9 must say
the same thing, and `flutter build appbundle` stays visible as the project-level
command an operator wires into their own release stage, not as the scaffold's
verification.

## Runner truthfulness

After the template work, the matrix rows are:

| Profile | Declared commands | CI qualification on `ubuntu-latest` |
| --- | --- | --- |
| `rust-web` | `cargo build`, `cargo test` | qualified (toolchain provisioned) |
| `nextjs-web` | `npm run build`, `npm test` | qualified |
| `aspnet-web` | `dotnet build`, `dotnet test` | qualified only if the runner installs the version the evidence names (current CI installs `dotnet 8.0.x`; the local qualification recorded `dotnet 10.0.400`) |
| `react-web` | `npm run build`, `npm test` | qualified |
| `python-service` | `python3 -m build`, `python3 -m unittest …` | qualified only where `build` is importable; otherwise `unverified` with the prerequisite named |
| `flutter-app` | `flutter analyze`, `flutter test` | `unverified` on a runner with no Flutter SDK — an honest row, not a failure and not a skip |

So the intended end state is: **no row reports `failed`.** `unverified` may
legitimately remain on hosts lacking a toolchain, and the full-matrix *pass*
claim is only available on a runner that qualifies every supported row. This
package does not claim that a runner exists which does so; it removes the
defects that guarantee failure everywhere, and records which rows still depend on
runner prerequisites.

## Compatibility and migration

- Descriptor command changes alter generated trees, so `tests/generate_contract.rs`
  and the per-profile contract suites need their expected values refreshed —
  deliberately, with the diff reviewed profile by profile.
- Existing projects keep their pinned versions and receipts; a declaration whose
  content is stale but unedited refreshes through the existing path, and one a
  user edited refuses with `feature-ownership-conflict`. No silent rewrite.
- `forge profile inspect` output changes in the command fields only; the shape
  and `support_status` semantics are untouched.
- No readiness contract bump: the matrix already reports these fields, and this
  package changes evidence, not interface.
- If `flutter-app`'s command change is judged to alter its *contract* rather than
  its evidence, the alternative is to demote the profile's `Supported` status —
  rejected here because the template is genuinely verified under the declared
  commands; the review decision belongs in the proposal/design review, not in an
  implementation shortcut.

## Failure boundaries to prove, not assume

- A generated tree is built and tested with **Forge absent from `PATH`** — the
  checklist's own condition (`docs/release-readiness.md:15-18`).
- A missing toolchain yields `unverified` with the toolchain named, and the gate
  still refuses a full-matrix pass.
- A template whose declared test command exits non-zero yields `failed` with the
  exact command; the run is recorded, never converted to a skip.
- The disposable fixture is removed after the row, and the repository working
  tree is left unchanged by the matrix run.
- Row evidence keeps carrying toolchain version, source SHA-256 and timestamp per
  row, and those values are what get quoted into HANDOFF.

## Verification

- Per profile: `forge new`, then the native commands from the descriptor, then
  the readiness row's recorded verdict with its captured values.
- `forge readiness matrix` output quoted verbatim into `HANDOFF.md`, and
  `forge readiness check` run with no `--profile` to show the full-matrix result
  honestly (passing only if every row passes).
- `scripts/release-check.sh --gate-profile …` with the profile set the CI job
  actually qualifies, recorded as the local reproduction of CI.
- `.project.json` emission tests for the changed commands, plus an upgrade round
  trip on a pre-change project proving the refresh and conflict paths.
- Docs sweep: `docs/release-readiness.md` known-gaps section either emptied with
  evidence or narrowed to the rows that remain runner-prerequisite-dependent, and
  `README.md`/`ROADMAP.md` acceptance wording adjusted to the recorded truth.
