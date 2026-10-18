# {{project_name}} container runtime

This runtime builds the `{{app_project}}` Cargo package in release mode and
runs it as a non-root user. PostgreSQL is a separate Compose service and the
application health check probes `{{health_path}}`.

Set `POSTGRES_PASSWORD` in the environment before running Compose. Do not put
credentials in the generated files.
