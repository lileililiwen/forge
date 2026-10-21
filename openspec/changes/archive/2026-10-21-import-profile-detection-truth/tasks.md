# Tasks: import-profile-detection-truth

## 1. BFS baseline

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Read the import detection: `src/import/detect.rs` (`top_level_candidates`, `profile_language`, `profile_package_manager`, `detect_framework`, `inspect_import`).
- [x] 1.3 Read the import proposal model and display: `src/import/model.rs`, `src/import/display.rs`.
- [x] 1.4 Read the import contract test: `tests/import_contract.rs`.
- [x] 1.5 Read the supported profile catalogue: `src/profile/catalog.rs` and the generator's per-profile markers in `src/generate/engine.rs`.
- [x] 1.6 Read the canonical capability this change modifies: `project-import`.
- [x] 1.7 Read `requirement.md` §8 (Project Import) and §9 (Profiles).

## 2. DFS requirement implementation

- [x] 2.1 Decide the shape of the lane's profile knowledge: one declared table of `(profile, language, package manager)`.
- [x] 2.2 Decide the npm markers: the `next` dependency or a `next.config.*` file; the `react` dependency, a `vite.config.*` file or a root `index.html`.
- [x] 2.3 Decide the precedence: Next.js wins when both marker sets are present, because a Next.js application depends on React.
- [x] 2.4 Decide the guard: the lane's profile list is pinned against the supported catalogue in both directions, and every supported profile is detectable.

## 3. Implement

- [x] 3.1 `src/import/detect.rs`: `IMPORT_PROFILES`, and `profile_language`, `profile_package_manager` and `import_profiles()` read it; `react-web` is `typescript`/`npm`.
- [x] 3.2 `src/import/detect.rs`: the `package.json` detector proposes `nextjs-web` or `react-web` from the markers, with the Next.js precedence and the existing scoring shape.
- [x] 3.3 `src/import/detect.rs`: `detect_framework` learns `react-web`, detecting `react` from the `react` dependency.
- [x] 3.4 `tests/import_profile_detection_contract.rs`: the pinned profile list against the catalogue, and a detection run for every supported profile.

## 4. BFS regression and completeness

- [x] 4.1 `cargo build` succeeds and the binary is produced.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 `forge import` on a React project names `react-web`, `typescript`, `npm` and the framework `react`.
- [x] 4.4 `forge import --accept` on that project writes `runtime.language: typescript`, and `forge doctor` on it reports no dependency drift.
- [x] 4.5 `forge import` on a generated Next.js project names `nextjs-web` whatever the project is called.
- [x] 4.6 `forge import` on a Rust, Flutter, Python and .NET project is unchanged.
- [x] 4.7 Every touched Rust file is rustfmt-clean, and no pre-existing rustfmt-dirty file was touched.
- [x] 4.8 `cargo clippy --workspace --all-targets --all-features` reports no new warning.
- [x] 4.9 Every touched file stays under the 1000 physical line limit.

## 5. Verification

- [x] 5.1 `forge gate --dry-run` resolves every required check.
- [x] 5.2 `forge gate --timeout-secs 1800` passes locally, before the archive.
- [x] 5.3 `openspec validate --all --strict --no-interactive` passes.
- [x] 5.4 `node scripts/check-openspec-change-names.mjs` passes.
- [x] 5.5 `git diff --check` reports no finding.

## 6. Archive

Closed by the owning agent.

- [x] 6.1 Archive without `--skip-specs`, so the new capability is promoted.
- [x] 6.2 Replace the archive's `TBD` Purpose with the real one, and update the modified capability's purpose.
- [x] 6.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

The change was archived with `openspec archive import-profile-detection-truth
--yes` (no `--skip-specs`), so
`openspec/specs/import-profile-detection-truth/spec.md` was created and
`openspec/specs/project-import/spec.md` was updated with every canonical
scenario kept. The archive's `TBD` Purpose was replaced by hand with the
paragraph the capability's own requirements describe, and the modified
capability's Purpose now states that the detection covers every supported
profile. The implementation and its specs are committed together, and the
handoff update is a separate commit.

## 7. Evidence

Every row below is a command that was run in this worktree, with its
observed output quoted. Nothing is claimed that was not observed.

| Check | Result | Evidence |
|---|---|---|
| 4.1 `cargo build --bin forge` | pass | `Compiling forge v0.1.1 (/home/paul/code/forge)`<br>`Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 16.51s` (exit 0) |
| 4.2 the repository test suite | pass | `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` → 213 test binaries, `4057 passed; 0 failed; 1 ignored` in total (exit 0) |
| 3.4 `cargo test --test import_profile_detection_contract` | pass | `running 5 tests` — `the_lane_knows_exactly_the_supported_profiles ... ok`, `a_plain_npm_project_is_not_guessed ... ok`, `a_generated_nextjs_project_is_detected_whatever_it_is_called ... ok`, `an_accepted_react_import_carries_the_profiles_language ... ok`, `every_supported_profile_is_detected ... ok`; `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`<br>The unchanged `cargo test --test import_contract` → `test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| 4.3 `forge import` on a React project | pass | Before: `profile = None`, `lang = unknown None`, `framework = unknown None`, `pkgmgr = unknown None`, `confidence = none`<br>After (`./target/debug/forge import /tmp/dbdt18/react-app --format json`): `profile = react-web`, `lang = detected typescript`, `framework = detected react`, `pkgmgr = detected npm`, `alts = []`, `confidence = high` |
| 4.4 `forge import --accept` then `forge doctor` | pass | Before: `runtime:` / `  language: unknown` and `[WARN] dependency-drift (automatic)` / `evidence: manifest runtime language 'unknown' disagrees with profile language 'typescript'`<br>After: `imported react-app (/tmp/dbdt18/react-app)`; the written `forge.yaml` carries `runtime:` / `  language: typescript`; `forge doctor` reports `[PASS] dependency-drift (automatic)` / `evidence: manifest runtime agrees with on-disk dependency evidence` / `detail: no dependency drift detected` |
| 4.5 `forge import` on a generated Next.js project | pass | Before: `profile = None`, `lang = unknown None`<br>After (`./target/debug/forge import /tmp/dbdt18/webapp --format json`, a `forge new --profile nextjs-web` project): `profile = nextjs-web`, `lang = detected typescript`, `framework = detected next.js`, `pkgmgr = detected npm`, `alts = []` |
| 4.6 `forge import` on the other four profiles | pass | rust-app: `rust-web` / `rust` / `axum` / `cargo` before and after; flutter-app: `flutter-app` / `dart` / `flutter` / `pub` before and after; python-app: `python-service` / `python` / `fastapi` / `pip` before and after; aspnet-app: `aspnet-web` / `csharp` / `asp.net core` / `dotnet` before and after. Every one: `alts = []`, `confidence = high` |
| 4.7 `rustfmt --edition 2021 --check` on every touched Rust file | pass | `rustfmt --edition 2021 --check src/import/detect.rs tests/import_profile_detection_contract.rs` → exit 0, no diff<br>`git status --short` → `M HANDOFF.md`, `M src/import/detect.rs`, `?? openspec/changes/import-profile-detection-truth/`, `?? tests/import_profile_detection_contract.rs`: no pre-existing rustfmt-dirty file was touched |
| 4.8 `cargo clippy --workspace --all-targets --all-features` | pass | exit 0; 39 warnings, the baseline count; none of them is in `src/import/detect.rs` or `tests/import_profile_detection_contract.rs` |
| 4.9 the source file line counts of every touched file | pass | `src/import/detect.rs` 952 lines, `tests/import_profile_detection_contract.rs` 312 lines (limit 1000) |
| 5.1 `forge gate --dry-run` | pass | `checks (8 planned):` with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size`, `tests`; `unbound required checks: none`; `exemption budget: cap=2 declared=0 headroom=2 required=8 verdict=within` (exit 0) |
| 5.2 `forge gate --timeout-secs 1800` | pass | `aggregate: passed`, `freshness: fresh`, `observed_at: 2026-10-21T23:03:00Z`, `revision: bfb11b2c338c74e98af8e736770181d145934289`, `manifest_digest: sha256:2c456f0e82a570fb`, `rule_pack_version: forge-baseline/1.0.0`; all eight checks `pass`; `persisted: .forge/gate/forge/evidence.json` (exit 0) |
| 5.3 `openspec validate --all --strict --no-interactive` | pass | `✓ change/import-profile-detection-truth`, `Totals: 140 passed, 0 failed (140 items)` (exit 0) |
| 5.4 `node scripts/check-openspec-change-names.mjs` | pass | `check-openspec-change-names: PASS` (exit 0) |
| 5.5 `git diff --check` | pass | exit 0, no output |
