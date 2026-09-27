# Tasks: jenkins-publish-integration

## 1. BFS — Baseline and impact coverage

- [x] Confirm the existing deploy-executor contract and release container stage.
- [x] Confirm the jenkins-local `mac-runtime-only-deployment` dependency contract.
- [x] Map runtime-only requirements to Forge modules and contract tests.

## 2. DFS — Requirement-by-requirement implementation

- [x] Add failing Forge test for `mac-runtime` target validation.
- [x] Implement `TARGET_MAC_RUNTIME` and preserve existing target compatibility.
- [x] Add failing Forge test for executor working-directory propagation.
- [x] Implement project-root working-directory handoff.
- [x] Add failing adapter contract fixtures for dry-run/apply/observe.
- [x] Consume the Linux-side adapter through `FORGE_DEPLOYER_BIN`.
- [x] Document Forge local container build and runtime deployment commands.

## 3. BFS — Cross-surface regression and completeness

- [x] Prove no new path invokes rsync, Jenkins API, or Mac deployment scripts.
- [x] Prove Mac-local env-file values never enter Forge output.
- [x] Prove unavailable runtime preserves prior evidence.
- [x] Prove existing local/docker-compose deploy targets remain compatible.

## 4. Verification

- [x] Run focused and full Forge tests, format, and build.
- [x] Run strict OpenSpec validation and name checks.
- [x] Run `git diff --check` and inspect the uncommitted user changes.
- [x] Record real Mac execution as a separate unrun runtime step.
