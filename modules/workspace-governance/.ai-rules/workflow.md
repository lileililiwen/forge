# Governance Workflow

## BFS → DFS → BFS

1. BFS: map affected projects, registry entries, profiles, consumers, deployment paths, evidence, and exceptions.
2. Structural pass: update schemas, declarations, registry references, templates, and test skeletons. This is `SKELETON_READY`, not completion.
3. DFS: implement one registry/check/deployment requirement at a time.
4. BFS: re-check all registered projects, generated manifests, security boundaries, OpenSpec names, CI parity, and evidence status.

## Git state

Work directly on `main` by default; do not create a branch or worktree unless the user asks for one or designates one.

## OpenSpec lifecycle

Run `openspec list`, confirm there is at most one active change, set `current_spec` in `HANDOFF.md`, and run `node scripts/check-openspec-change-names.mjs` before status or instructions. Do not create a new change until the current one is complete and archived. When none remain, set `current_spec: none`. Implement one change, run the local checker and tests, validate strictly, archive without `--skip-specs`, and inspect the staged diff.

Each completed OpenSpec spec/change requires exactly two commits: first the implementation/tests/archive commit, then a HANDOFF.md-only pointer/evidence commit. Stop after commit 2.

## Existing-project adoption

Run `python3 scripts/init_project.py <path> --id <id> --profile <profile> --kind <kind>`. Review all generated files, fill the real verification command, and run `workspace_check.py --project <id>`. Do not copy shared gate implementations, installers, caches, or credentials.

## Jenkins boundary

The governance checker must pass before `jenkins_manifest.py` emits an eligible deployment entry. Jenkins owns secrets, host paths, synchronization, Compose, database provisioning, job registration, and deployment.
