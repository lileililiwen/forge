# Tasks: shared-postgres-runtime-profile

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Map runtime template registry, variable substitution, file overwrite rules, `.project.json` metadata, `.gitignore` behavior, and Docker/Compose checks.
- [ ] 1.2 Add four render fixtures and assert generated service/resource topology before template implementation.
- [ ] 1.3 Verify user-provided secrets never enter CLI render arguments, project metadata, Git, docs, or test snapshots.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Add external PostgreSQL template IDs and variable validation for web/worker runtimes.
- [ ] 2.2 Add app-only Compose assets and blank `.env.example` files; omit DB containers and volumes.
- [ ] 2.3 Add idempotent `.env` ignore insertion with fail-before-write conflict handling.
- [ ] 2.4 Add documentation for distinct per-project databases/roles, local env secrets, Mac host example, and portable cloud endpoints.
- [ ] 2.5 Update runtime template registry/schema/spec and preserve existing default types.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Verify four variants' Compose topology/env mappings, secret ignore, project metadata, and renderer idempotence.
- [ ] 3.2 Compare all existing template outputs against baseline snapshots and check deployment manifest integration remains compatible.
- [ ] 3.3 Verify missing credentials, unsafe ignore rules, unavailable Compose, and external DB failure never claim a database was provisioned or healthy.

## 4. Verification

- [ ] 4.1 Run `python3 -m unittest discover -s tests -v`, `python3 scripts/workspace_check.py --self-check`, `python3 scripts/jenkins_manifest.py --check`, strict OpenSpec validation, and `git diff --check`.
- [ ] 4.2 Render all four fixture templates and run `docker compose config` with dummy values when available; report Compose unavailability separately.
