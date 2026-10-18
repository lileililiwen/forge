# {{project_name}} container runtime

This runtime publishes `{{app_project}}` in a multi-stage .NET image and starts
PostgreSQL before the application. The generated health check probes
`{{health_path}}` on the application container.

Set `POSTGRES_PASSWORD` in the environment before running `docker compose up`.
Secrets are intentionally supplied at runtime and are not written to this
repository.
