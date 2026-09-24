# Tasks: Execute the declared gate runtime and journal its evidence

## 1. BFS — Baseline and impact coverage

- [x] Capture real Driftwatchdog `gate --format json` document shapes
  (passed/blocked/not-applicable) from the sibling repo as fixtures.
  (`tests/fixtures/gate/` — four verbatim captures at `25811ed` plus the
  dry-run plan; divergences from the design recorded in `NOTES.md`.)
- [x] Map operations-table kinds, evidence-file precedents
  (release/deploy/analytics), doctor finding registry, provider-matrix row
  machinery and readiness check capture.
- [x] Define GateEvidence type, verdict mapping table and exit-code rule.
  (`src/gate/mod.rs`; aggregate `passed|blocked|failed|unknown`, check
  state `pass|fail|skip|not_applicable|unresolved`, exit 0 only for a
  passed aggregate; the real third top-level status `REVIEW_REQUIRED` is
  classified through `blocked`/`unknown`, never `passed`.)
- [x] Add fixture set and test skeletons for every classification branch.
  (18 `src/gate` unit tests; every branch has a driving test below.)

## 2. DFS — Requirement-by-requirement implementation

- [x] Implement runtime resolution chain with shared binary probe.
  (`FORGE_GATE_BIN` exact → `.project.json verification.gate_runtime`
  name-mapped (unknown name refused by name) → ordered
  `driftwatchdog`/`driftwatch` PATH probe shared with the policy plane;
  attempts listed on every refusal.)
- [x] Implement bounded gate invocation and status-document parsing with
  `unresolved` for unknown states. (Argument array, null stdin, 256 KiB
  bound, default 600s `--timeout-secs` 1..=86400; parseable document is
  evidence whatever the exit code; contradictory PASS-on-nonzero-exit
  downgrades to `unknown`; `gate --dry-run` reports the real
  side-effect-free plan preview — the design's JSON dry-run assumption
  was disproved live, see design.md corrections.)
- [x] Implement evidence persistence (atomic), journaling with real
  project ids, and revision capture.
  (`.forge/gate/<project-id>/evidence.json` tmp+rename; operations kind
  `gate` with `done|blocked|failed`; rehearsals and reads never
  persist/journal; revision = git HEAD at invocation.)
- [x] Implement CLI `forge gate [TARGET] [--dry-run]` and `gate status`
  (human/JSON) with aggregate-mirroring exit codes.
  (16 `tests/gate_contract.rs` cases.)
- [x] Implement the doctor `gate-evidence` finding (pass/warn-stale/fail-
  blocked/unverified). (Declared-but-never-run and unreadable records are
  UNAVAILABLE-applicable; a project with no declaration, manifest or
  evidence is not-applicable so absence never reads as health and the
  checker stays quiet.)
- [x] Add the provider-matrix `gate-runtime` row honoring opt-in rules.
  (8 `tests/gate_provider_contract.rs` cases; probe runs only the
  dry-run plan surface and never claims a gate pass.)
- [x] Wire release checks to accept a gate kind bound to captured
  revision. (`gate` joins the `release.checks` vocabulary; never-run →
  unavailable, revision-moved → `stale`, blocked/failed → fail;
  `tests/gate_cross_surface.rs` proves a stale passing record cannot
  back a ready plan.)

## 3. BFS — Cross-surface regression and completeness

- [x] Re-verify no surface treats absent gate evidence as pass (doctor,
  portal, readiness, release).
  (`tests/gate_cross_surface.rs`: doctor absent/declared/blocked/stale
  matrix; `gate status` absent → unverified exit 1; checker alert matrix;
  release never-run → unavailable → plan not ready.)
- [x] Re-verify stale-after-revision behavior and history preservation on
  unavailable runs. (Status fresh→stale after a new commit with identical
  observed_at/aggregate bytes; missing-runtime refusal names attempts,
  leaves prior evidence byte-identical and journals `failed` beside the
  earlier `done`.)
- [x] Re-verify journal independence (gate rows do not alter other kinds),
  redaction, and that MCP tool registry/API/portal gains no new write
  surface. (Foreign provider row byte-preserved; credential-shaped
  diagnostics redacted through stdout, evidence file and journal; MCP
  tools/list never advertises gate tools; no `/gate` API route; portal
  refuses `view gate` as `portal-invalid`.)
- [x] Update docs (architecture integration line, HANDOFF evidence wording
  for future cycles, provider-evidence rows) to reflect the runtime
  exists. (`docs/architecture.md` Gate Runtime Consumption row;
  `docs/provider-evidence.md` `gate-runtime` row; `.project.json`
  vocabulary note in `tests/fixtures/workspace-metadata/NOTES.md`;
  HANDOFF/AGENTS evidence wording at close.)

## 4. Verification

- [x] `cargo fmt --all -- --check`; `cargo build`; `cargo test
  --all-targets`; `cargo clippy --all-targets -- -D warnings`.
  (fmt/build/name-preflight/`git diff --check` green; clippy
  `--all-targets -- -D warnings` green; full battery green.)
- [x] Dogfood `forge gate .` on this repository with driftwatchdog
  installed; commit the snapshot summary or the exact failed command and
  next action in HANDOFF evidence.
  (Release build, installed sibling `driftwatch 0.1.0`:
  `forge gate . --dry-run` exit 0 through this repo's real
  `.ai-gate/gate.yaml` — plan preview listing build/repository/security/
  tests as required, `persisted: false, journaled: false`; the real run
  fails honestly at the runtime's own store boundary
  (`gate-runtime-unavailable: ... hint: run `driftwatch doctor`; if the
  DB is corrupt, delete `.driftwatch/state.db` and run `driftwatch init`
  again`), empty stdout, no repository writes, evidence untouched.
  Next action (sibling-owned): initialize this checkout's `.driftwatch`
  store (`driftwatch init`) before claiming a real gate pass here; none
  is claimed. The full lifecycle was then proven live against a scratch
  gate project with the real installed binary: passing run exit 0 with
  revision-bound persisted evidence + `done` journal; blocked run exit
  1 with `blocked` aggregate + journal; stale status exit 1 after the
  revision moved; doctor pass→fail and the checker error alert
  `doctor/gate-evidence`; dry-run preview persisted nothing.)
- [x] `node scripts/check-openspec-change-names.mjs`; `openspec validate
  --all --strict --no-interactive`; `git diff --check`. (All green,
  pre- and post-archive.)
