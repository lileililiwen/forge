# Proposal: Close the three failing matrix rows so a full-matrix claim becomes possible

## Why

`docs/release-readiness.md:53-75` records four non-passing rows and then states
the consequence plainly: "Until those gaps are remediated, **no full-matrix gate
passes**; a qualified subset gate … is the release-eligible verdict, and the
unselected rows remain explicitly non-passing."

The recorded causes, verbatim from that file:

- `flutter-app` **failed**: "`flutter build appbundle` cannot find
  `android/app/build.gradle` in the generated fixture (`extended-profile-catalog`
  template gap; an Android SDK would additionally be required on the runner)."
  The descriptor's commands are `flutter build appbundle` (build) and
  `flutter test` (test) at `src/profile/mod.rs:490-491`, so the row is asking the
  generated tree to produce an Android app bundle from a tree that has no
  Android host platform in it.
- `react-web` **failed**: "`npm test` (`node --test`) fails with `ReferenceError:
  document is not defined` in the generated fixture (`extended-profile-catalog`
  template gap)." A scaffold whose own test command cannot run in a bare Node
  process is not a tested template by this repository's own definition of
  `Supported` (`src/profile/mod.rs:40`: "Fully implemented, has a tested
  template").
- `python-service` **failed** on hosts without `pytest`: "`python3 -m build`
  passes, `python3 -m pytest` reports `No module named pytest` (runner
  prerequisite gap)." The descriptor test command is `python3 -m pytest`
  (`src/profile/mod.rs:577`), which makes a third-party module a hidden
  prerequisite of a row that is supposed to be reproducible on a clean runner.

Two consequences make this more than a CI hygiene item.

**The readiness matrix is the single source of truth for what Forge claims about
profiles.** `forge readiness matrix` reports every supported profile and
classifies each row `passed` / `failed` / `unverified`
(`docs/release-readiness.md:15-21`), and the same descriptor test command is the
value Forge writes into a generated project's `.project.json`
`verification.command` (recorded as "one source of truth with the readiness
matrix" in the archived `workspace-metadata-emission` evidence). So a row that
cannot run is simultaneously a profile Forge advertises as `Supported`, a
template that cannot verify itself, and — for any project generated from it — a
declared verification command that does not work on the operator's own machine.

**Every downstream claim inherits the gap.** The v0.1 acceptance line requires
that "all five MVP profile fixtures generate and build using native toolchains
without Forge" (`ROADMAP.md:59`); `flutter-app` and `python-service` are two of
those five, and `react-web` is the v0.2 addition (`ROADMAP.md:26`). The
`artifact-and-ci-baseline` CI gate can only qualify the rows its runner can
actually run, so this package determines how much of that gate is honest.

## What Changes

- Give `flutter-app` a template that matches the command it declares. Either
  stage the Android host platform files the appbundle build needs, or change the
  descriptor's build/test commands to ones the shipped tree can perform
  (`flutter analyze`, `flutter test`, or a web/desktop target that needs no
  Android SDK). The choice is a design decision with a recorded rationale, not a
  quiet substitution, and the rejected option is written down with its reason.
- Make `react-web`'s test command pass in a bare Node process: the shipped
  `src/app.test.mjs` must test what the template actually contains without
  requiring a browser DOM, and the template must not imply a test environment it
  does not provision.
- Remove `pytest` as a hidden prerequisite for `python-service` by moving the
  declared test command to the standard library runner the generated tree already
  satisfies, keeping `python3 -m build` as the build command.
- Re-qualify all three rows against their real native toolchains with the
  **disposable-fixture, Forge-absent** procedure the checklist already describes
  (`docs/release-readiness.md:15-18`), recording the toolchain version, source
  SHA-256 and timestamp per row as the matrix captures them.
- Re-open the full-matrix gate only when every supported row reports `passed`,
  and keep `unverified` for any row whose toolchain is genuinely absent from the
  runner — which is the standing rule (`:33`: "A profile whose toolchain is
  absent is `unverified`, never passing").
- Synchronise every document that currently states the subset-only rule:
  `docs/release-readiness.md`, the runner/profile contract table, `README.md`'s
  MVP claim, `ROADMAP.md`'s v0.1/v0.2 acceptance wording, and any evidence block
  that cites the three failures.

## BFS Impact Map

- **Capabilities:** `profile-and-release-readiness` (the runner/profile contract
  and the full-matrix claim), `extended-profile-catalog` (the three descriptors
  and their templates), `deterministic-project-generation` (the generated tree
  and the emitted `verification.command`), and indirectly
  `gate-runtime-evidence`/`artifact-and-ci-baseline` (what CI may qualify).
- **Users and flows:** `forge new --profile react-web|flutter-app|python-service`
  produces a project whose own declared verification command runs;
  `forge readiness check` without `--profile` stops being a guaranteed failure;
  a governed project's Workspace Governance `VERIFICATION_COMMAND` check sees a
  command that works.
- **Contracts/data/persistence:** descriptor `build_command`/`test_command`
  values may change (`src/profile/mod.rs:68-69`, `:722` validation), which
  changes generated `.project.json` `verification.command` and therefore changes
  declaration content — the receipt/refresh path from
  `workspace-metadata-emission` must handle the stale-unedited case per profile.
  No schema or registry migration.
- **Integrations/configuration:** the matrix's per-row native invocations; CI
  runner toolchains (`flutter`/Android SDK availability decides whether the
  `flutter-app` row is `passed` or honestly `unverified` there); `python3 -m
  unittest` needs no pip step, which keeps the row reproducible on a network-free
  runner.
- **Callers:** `forge readiness matrix|check`, `forge doctor` profile findings,
  `forge profile inspect` (which advertises the commands), `forge new`, the
  portal readiness section, and the `docs/requirements-coverage.md` account of
  §9/§15.
- **Failure/boundary behavior:** a toolchain the host lacks stays `unverified`,
  never coerced into `passed`; a template whose native build fails is `failed`
  with the exact command named; a descriptor with an empty `build_command` is
  already refused (`src/profile/mod.rs:722`); a changed command never silently
  rewrites a user-edited declaration — it refuses with the ownership conflict.
- **Tests:** the existing per-profile native build/test matrix evidence; template
  content tests asserting the shipped test file needs no DOM; descriptor tests
  asserting the command pair a profile advertises is the pair its template can
  satisfy; contract tests that the emitted `.project.json` command matches the
  descriptor and the matrix row; upgrade-path tests for the rehashed receipt.
- **Dependencies:** soft on `artifact-and-ci-baseline`, which decides which rows
  CI qualifies; no dependency on any sibling repository.
- **Compatibility/security/privacy:** no CLI surface, document format or exit code
  changes; the generated tree must still build and test with Forge absent from
  `PATH`; the Python change removes a third-party import rather than adding one.

## Capabilities

- (extends `profile-and-release-readiness`) every supported profile's declared
  native build and test commands execute successfully against its own generated
  tree, so the readiness matrix can report a full-matrix pass instead of a
  permanently qualified subset.

## Non-goals

- Adding, removing or re-scoping profiles. Planned specialist profiles stay
  planned and stay out of the matrix (`docs/release-readiness.md:68-70`).
- Installing an Android SDK or any other heavyweight toolchain to force a row to
  pass. If the row cannot run on a given runner it is reported `unverified`, and
  the choice of what a runner qualifies belongs to `artifact-and-ci-baseline`.
- Weakening the `Supported` status to hide a gap, or converting a failure into a
  skip.
- Changing the readiness contract version or the matrix output shape.
- Reaching a full-matrix pass by dropping rows from the matrix.
- Any maturity promotion, publication or release claim; requirement.md §25 keeps
  advancement evidence-based and optional.

Source: requirement.md §9, §14, §15, §23, §34, §39, §40.
