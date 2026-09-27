# Tasks: forge-publish-observability-revision-containers

## 1. BFS — Baseline and impact coverage

- [ ] B1. Capture current provider request, progress, terminal response, and
  status output as compatibility fixtures.
- [ ] B2. Map build/run states and revision identity through Forge, Jenkins,
  Compose, registry, status CLI, and route verification.
- [ ] B3. Add fixtures for build failure, run failure, missing phase evidence,
  and explicit conflicting `container_name`.

## 2. DFS — Requirement-by-requirement implementation

- [ ] D1. Add build/run phase fields and evidence validation to the Forge
  provider contract and journal projection.
- [ ] D2. Emit build/run events and terminal evidence from the Jenkins Mac
  provider.
- [ ] D3. Use revision-qualified Compose project/container identity and verify
  the identity before run success.
- [ ] D4. Extend `forge deploy status` to display revision, build, run, and
  container identity details.

## 3. BFS — Cross-surface regression and completeness

- [ ] R1. Verify manual project publish, sequential fleet publish, and GitHub
  webhook publish use the same phase/revision semantics.
- [ ] R2. Verify cache reuse remains enabled and revision identity does not
  cause unnecessary base-layer downloads.
- [ ] R3. Verify partial fleet deployment, stale containers, rollback safety,
  and route refresh never report an old revision as current.
- [ ] R4. Verify no Mac source checkout or deployment script is introduced.

## 4. Verification

- [ ] V1. Run Jenkins provider unit tests and Forge provider/status tests.
- [ ] V2. Run Forge formatting, build, targeted tests, and applicable all-target
  tests.
- [ ] V3. Run strict OpenSpec validation and diff checks.
- [ ] V4. Capture real Mac Docker evidence showing the requested SHA in the
  container identity and separate build/run status.
