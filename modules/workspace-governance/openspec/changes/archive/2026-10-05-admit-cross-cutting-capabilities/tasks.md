# Tasks: admit-cross-cutting-capabilities

## 0. Caveats and constraints

- [x] **The checker is not modified.** `validate_capabilities` keeps emitting
  `CAPABILITY_UNSUPPORTED` for an id outside the profile's list. Only the
  profile data and the vocabulary paragraph change; `scripts/workspace_check.py`
  is byte-identical before and after.
- [x] **Only the four cross-cutting ids** (`telemetry`, `rate-limiting`,
  `idempotency`, `feature-flags`) are admitted. The registry's role-shaped ids
  stay out by decision (`design.md` §3), not by oversight.
- [x] **No sibling repository is written and `capabilities.json` is not
  edited.** The registry already binds the four ids to `dotnet-platform-libs`.
- [x] **No commit sweeps unrelated work.** The pre-existing untracked work
  (`driftwatch.toml`, `scripts/check_concern.py`, `scripts/close_gate_debt.py`,
  `openspec/changes/drive-legacy-gate-debt-to-zero/`) is not staged. `main`
  only, no branch, no `git add -A`, no attribution trailers.

## 1. BFS — Baseline and blast radius

- [x] 1.1 Recorded the baseline before any edit: `--self-check` `0 errors /
  0 warnings`; test suite `Ran 812 tests` with exactly one pre-existing
  failure, `test_bind_gate_commands.GateOracleTest.test_a_fresh_run_reproduces_the_gate_set`,
  caused by fleet gate-debt drift in sibling repositories and **not** by this
  change; `openspec validate --all --strict` failed only on the untracked
  `drive-legacy-gate-debt-to-zero`.
- [x] 1.2 Located the single reporting site: `validate_capabilities`
  (`scripts/workspace_check.py:1034`), one WARNING emission, called once from
  `validate_project` (`:3341`) with `supported` read from the profile by
  `load_profile_spec` (`:3335-3339`).
- [x] 1.3 Reproduced the defect without editing anything: calling
  `validate_capabilities` with `dotnet-product`'s supported set and a
  `telemetry` declaration yields exactly `CAPABILITY_UNSUPPORTED`.
- [x] 1.4 Mapped every profile that declares a `supported_capabilities` list:
  `dotnet-library`, `dotnet-product`, `governance`, `platform`, `product`,
  `python-product`, `python-runtime`, `rust-platform`, `rust-product`,
  `tooling`, `typescript-monorepo`, `typescript-product` — twelve files.
  `deployment`, `dotnet-template`, and `inventory` declare none and are out of
  scope.
- [x] 1.5 Confirmed no schema constrains the list: `schemas/` has no profile
  schema, and `supported_capabilities` is referenced only by
  `docs/capabilities.md` and `scripts/workspace_check.py`.

## 2. DFS — One coherent edit

- [x] 2.1 Added `telemetry`, `rate-limiting`, `idempotency`, `feature-flags`
  to `supported_capabilities` in all twelve profiles, preserving each file's
  existing ordering (product ids first, then the cross-cutting four).
- [x] 2.2 Updated `docs/capabilities.md` §"Capability states" to name the
  cross-cutting shared capabilities and state why they are declarable in every
  profile while the role-shaped registry ids are admitted on demand.
- [x] 2.3 Left `capabilities.json`, the schema, and `workspace_check.py`
  untouched.

## 3. BFS — Prove the rule and its negative control

- [x] 3.1 Added `tests/test_capability_profiles.py`: every profile with a
  `supported_capabilities` list contains all four ids (fails naming the
  profile and id).
- [x] 3.2 Asserted the measured reproduction is green: `validate_capabilities`
  with `dotnet-product`'s set and a `telemetry` declaration emits no finding
  (`test_cc002`).
- [x] 3.3 Asserted the negative control still bites: `dotnet-library` with a
  capability it does not list still emits `CAPABILITY_UNSUPPORTED`
  (`test_cc003`).
- [x] 3.4 Suite: `python3 -m unittest discover -s tests` — `Ran 815 tests`
  (812 prior + 3 new); the new module passes and the pre-existing fleet-drift
  failure is unchanged and still the only failure.
- [x] 3.5 `python3 scripts/workspace_check.py --self-check` — `0 errors /
  0 warnings`. `python3 scripts/auto_gate.py --self-check --no-deployment` —
  `PASS errors=0 warnings=0`.

## 4. BFS — Spec and validation

- [x] 4.1 Spec delta at `specs/platform-capability-manifest/spec.md`, MODIFIES
  the existing `Declare reusable capabilities explicitly` requirement — the
  requirement `validate_capabilities` implements.
- [x] 4.2 `openspec validate admit-cross-cutting-capabilities --strict`
  reports *valid*; `node scripts/check-openspec-change-names.mjs` passes;
  `openspec validate --all --strict` is `25 passed, 1 failed`, the failure
  being the untracked `drive-legacy-gate-debt-to-zero` unchanged from
  baseline.
- [x] 4.3 Archived the change; the target spec already carried a real
  `## Purpose`, so no archiver `TBD` marker was introduced.
- [x] 4.4 `git diff --check` clean; two commits on `main` — profile data,
  docs, tests and archive first, then `HANDOFF.md` only.

## 5. Deliberately not done

- [x] **`persistence-postgres`, `health-checks`, `web-application-factory`,
  and `postgres-test-fixture` are not admitted.** They are role-shaped, not
  cross-cutting, and no consumer records an adoption of them today
  (`design.md` §3).
- [x] **`workspace_check.py` is not made registry-aware.** Rejected in
  `design.md` §4; the profile list stays the intentional statement.
- [x] **No sibling repository is edited.** `argoscope`'s adoption of
  `Platform.Observability`/`Platform.Web.Telemetry` is a separate change in
  that repository, enabled by this one.
- [x] **`capabilities.json` postures are unchanged.** Every capability stays
  `exempt`; this change admits declaration, not adoption pressure.
