# Tasks: Close the three failing matrix rows so a full-matrix claim becomes possible

## 1. BFS — Baseline and impact coverage

- [ ] Reproduce the recorded failures before changing anything: generate each of
  `react-web`, `python-service` and `flutter-app` into a disposable fixture, run
  the descriptor's declared build and test commands with Forge absent from
  `PATH`, and capture the exact error text and toolchain versions. Record whether
  each still fails as `docs/release-readiness.md:58-67` describes.
- [ ] Inventory every place the three command pairs are asserted:
  `src/profile/mod.rs` (descriptor construction, the `build_command` emptiness
  validation at `:722`), `src/generate/mod.rs` template file lists,
  `src/generate/workspace.rs` `verification.command` emission,
  `src/readiness` row capture, `tests/generate_contract.rs`,
  `tests/profile_contract.rs`, `tests/readiness_contract.rs`,
  `tests/workspace_metadata_contract.rs` (per-profile digests), and
  `docs/requirements-coverage.md` §9/§15 text.
- [ ] Capture the pre-change readiness matrix output verbatim, including each
  row's toolchain version, source SHA-256 and timestamp, so the before/after
  comparison is evidence rather than recollection.
- [ ] Decide each profile's command pair against the tree it actually ships, and
  write the rejected alternative with its reason into design.md before editing a
  template (Flutter: host-platform vendoring versus a Dart-only verified build;
  Python: third-party runner versus the standard library; React: provisioning a
  DOM versus a DOM-free test).
- [ ] Confirm the Python build prerequisite honestly: check whether
  `python3 -m build` is importable on this host and on the CI runner image, and
  decide whether that row is `passed` or `unverified` with the prerequisite
  named — no guessing.

## 2. DFS — Requirement-by-requirement implementation

- [ ] Rewrite the `react-web` generated `src/app.test.mjs` so it asserts the
  template's real contract under `node --test` without touching `document`,
  keeping assertions strong enough that a broken template still fails; update the
  template content tests to require a DOM-free test and to forbid a bare
  `document` read.
- [ ] Change the `python-service` descriptor test command to the standard
  library runner and ship the template's tests in a form that runner executes
  without any install; keep `python3 -m build` as the build command, and record
  `build` as a declared runner prerequisite rather than an assumed one.
- [ ] Change the `flutter-app` descriptor build command to one the shipped Dart
  tree performs (`flutter analyze`) with `flutter test` as the test command, and
  keep `flutter build appbundle` documented as the operator-level release command
  that needs a platform host and an Android SDK — not as scaffold verification.
- [ ] Where a command change alters a descriptor's advertised capabilities or
  deployment defaults, update those fields and the `forge profile inspect` text so
  no field implies a guarantee the template cannot meet.
- [ ] Re-verify the emitted `.project.json` `verification.command` still matches
  the descriptor test command for every supported profile, and that
  `forge new` → sibling `workspace_check.py` reports no `VERIFICATION_COMMAND`
  finding for the changed profiles.
- [ ] Refresh the per-profile expected file lists, digests and contract
  assertions deliberately, reviewing the diff profile by profile rather than
  accepting a blanket digest update.
- [ ] Prove the upgrade path for each changed profile: a pre-change project with
  an unedited declaration refreshes and re-hashes its receipt naming both files;
  one with a user-edited declaration refuses with the ownership conflict leaving
  both files byte-preserved; a foreign declaration is never rewritten.
- [ ] Re-run the readiness matrix and `forge readiness check` without
  `--profile`, recording whether the full-matrix gate now passes and naming any
  row that remains `unverified` for a runner prerequisite.

## 3. BFS — Cross-surface regression and completeness

- [ ] Re-run the full native-toolchain evidence for the unaffected profiles
  (`rust-web`, `nextjs-web`, `aspnet-web`) to confirm the template changes did
  not disturb rows that already passed, and that the `aspnet-web` row's
  qualification still names the toolchain version the runner actually installs.
- [ ] Re-run `forge doctor` findings and the portal readiness section against the
  regenerated projects so profile status surfaces consistently with the matrix.
- [ ] Re-check that `Supported` still means what `src/profile/mod.rs:40` says it
  means for all six supported profiles, and that planned specialist profiles remain
  `unsupported-profile`, discoverable, and excluded from the matrix.
- [ ] Re-check that no row became a skip, that no failure was reclassified as
  `passed`, and that a missing toolchain still yields `unverified` with the
  toolchain named.
- [ ] Update `docs/release-readiness.md` (known gaps narrowed to what genuinely
  remains, the runner/profile contract table refreshed), `README.md` MVP status,
  `ROADMAP.md` v0.1/v0.2 acceptance wording, and
  `docs/requirements-coverage.md` §9/§15 to the recorded evidence.

## 4. Verification

- [ ] `cargo fmt --all -- --check`; `cargo build`; `cargo test --all-targets`
  with the long-running native scaffold test excluded; then run that native test
  separately and record its duration and result.
- [ ] `cargo clippy --all-targets -- -D warnings`; `node
  scripts/check-openspec-change-names.mjs`; `node
  scripts/check-spec-governance.mjs`; `openspec validate --all --strict
  --no-interactive`; `git diff --check`.
- [ ] Per-profile native evidence quoted into `HANDOFF.md`: generated tree,
  build command, test command, exit status, toolchain version, and the Forge-
  absent condition.
- [ ] `sh scripts/release-check.sh` with the profile set the current CI job
  qualifies, and once with no `--gate-profile`, recording which of the two the CI
  gate can honestly claim.
- [ ] Archive without `--skip-specs` only after every scoped row has current
  evidence, and state plainly whether a full-matrix pass is now available or
  still bounded by runner prerequisites.
