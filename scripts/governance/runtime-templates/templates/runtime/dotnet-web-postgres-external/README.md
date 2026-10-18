# {{project_name}} container runtime (external PostgreSQL)

This runtime publishes `{{app_project}}` in a multi-stage .NET image and
connects to a PostgreSQL service supplied at runtime. The generated
health check probes `{{health_path}}` on the application container.

## Shared database expectations

- One **database** and one **least-privilege DB role** for this project on
  the shared server. Do not reuse a role across projects.
- The renderer only writes the connection defaults to `.env.example`. Real
  values are loaded from a local `.env` (gitignored) or a host secret
  source at runtime.

## Connection variables

| Variable | Source | Notes |
|---|---|---|
| `PGHOST` | `.env` | Endpoint of the shared PostgreSQL service. |
| `PGPORT` | `.env` | Defaults to `5432` if unset. |
| `PGUSER` | `.env` | The least-privilege role for `{{database_name}}`. |
| `PGDATABASE` | `.env` | The database provisioned for this project. |
| `PGPASSWORD` | `.env` | Never commit. The renderer never reads, stores, or substitutes a password. |

## Local development

```bash
cp .env.example .env
chmod 600 .env
docker compose up --build
```

If the endpoint cannot be reached, the application reports its existing
readiness failure; the renderer never starts a fallback database.

Secrets are intentionally supplied at runtime and are not written to this
repository.

## Switching back to a local container

Render `dotnet-web-postgres` (the existing per-project container profile)
with `--replace compose.yaml --replace .env.example` and migrate any data
outside this change.
