# Design: Jenkins deploy executor contract

## Ownership and boundaries

jenkins-local owns deployment execution (sync, compose, job registration,
host paths, secrets). Forge owns the invocation contract, the plan/apply
journaling, confirm gates and health observation vocabulary. The reference
adapter is a translation layer, ~150 lines, with no business logic of its
own.

## Executor contract `forge-deploy-executor/0.1.0`

A single executable invoked with argv arrays (as Forge's `src/deploy`
already does today):

- `apply --target <name> --kind <local|docker-compose|ssh> --project <id>
  --revision <rev> [--dry-run]`
- `observe --target <name> --project <id> --deploy-id <id>`

stdout is one JSON envelope:

```json
{"contract": "forge-deploy-executor/0.1.0",
 "status": "applied|blocked|failed|observed|unavailable|unknown",
 "detail": "...", "evidence": ["..."], "observed_at": "..."}
```

Non-zero exit with a parseable envelope means the named stage failed;
non-zero without one is `unavailable` (prior DeployState untouched) —
exactly the existing classification, now pinned for third parties.

## Reference adapter mapping

`adapters/jenkins/forge-deployer-jenkins` (POSIX shell or Python) maps:

- `apply --dry-run` → `deploy-all.sh --dry-run` filtered to the project, or
  the Jenkins job's build-with-parameters preview if configured;
- `apply` → `project.sh <project>` (Jenkins triggers the pipeline job);
- `observe` → `project.sh`-equivalent status output.

Mapping facts (verb names, env for job URLs) live in one
`jenkins-adapter.md` doc file so the jenkins-local repo can later adopt or
fork the adapter without contract archaeology.

## Health vocabulary

Status text is matched against an explicit table (running→healthy,
exited/stopped→down, unreachable/unknown→unknown); no regex-optimistic
healthy defaults. `unknown` keeps the last good observation per the
existing rule.

## Provider matrix

The `deploy` row runs the adapter with `--dry-run` only. Live labeling
follows existing sandbox rules; the row's source names the adapter +
jenkins-local revision. Production promotion is jenkins-local's decision,
tracked in docs as a downstream adoption checklist.

## Security

The adapter is an operator-installed binary path (same trust model as
`FORGE_DEPLOYER_BIN`); argument arrays only; host strings passing through
Forge remain credential-redacted; Forge never stores Jenkins credentials,
URLs go in provider config as `scheme://` refs per manifest rules.

## Verification

- Contract fixtures: stub adapter emitting each status and malformed/
  timeout cases; assert existing deploy classifications still hold and
  prior state survives failures.
- Integration (evidence-qualified): run the reference adapter against a
  disposable Jenkins-less host with `--dry-run` stubs; production evidence
  deferred explicitly.
