# Local runtime entry points

Forge provides two local development routes while keeping service topology
and process ownership with the existing runtime tools.

## Compose stack

```sh
forge run compose [--project-root DIR] [--file FILE] [--profile NAME ...] [--] [COMPOSE ARGS...]
```

Forge resolves the root and Compose file, then invokes `docker compose` from
the project root. With no Compose action/arguments, it runs `up`. Use `--` to
pass an explicit Compose action and its arguments; profiles are repeatable.
For example:

```sh
forge run compose --project-root . --profile dev -- up --build api web
forge run compose --file deploy/compose.yaml -- down
```

Docker Compose owns service dependencies, health checks, profile selection,
startup order, logs and teardown. A repository with an API, worker, UI and
databases should keep that graph in its Compose file; Forge does not create a
parallel service list.

## One local application process

```sh
forge run local [PATH] [--port PORT] [--no-open] [--keep] [--docker] [--json]
```

Forge requires the `gitrunbykaru` executable on `PATH` and delegates the
canonicalized app directory. GitRunByKaru owns framework detection, dependency
preparation, process launch, readiness and cleanup. Forge does not use `npx`
for this entry point. Separate app directories can be launched independently;
coordinated multi-service projects should use Compose as their service graph.

## Boundaries

- Forge does not parse or rewrite Compose YAML, alter project files, register
  projects, persist runtime sessions, or claim readiness based on process
  creation.
- Child stdout/stderr are inherited, and its exit code is propagated.
- Missing tools fail with an install hint. Forge does not install Docker or
  GitRunByKaru.
- The authenticated web command catalog lists `run.compose` and `run.local` as
  CLI-only, because a browser cannot safely manage processes on the developer's
  machine.
- These commands are local development entry points. They do not run CI,
  publish artifacts, deploy, or satisfy a release Gate precondition.
