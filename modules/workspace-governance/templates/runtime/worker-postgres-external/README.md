# {{project_name}} container runtime (external PostgreSQL)

This runtime installs `requirements.txt` and starts the worker with
`{{worker_command}}` against a PostgreSQL service supplied at runtime.

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

If the endpoint cannot be reached, the worker exits with its existing
connectivity error; the renderer never starts a fallback database.

Set `POSTGRES_*` outside the repository and review the worker command
before promotion.
