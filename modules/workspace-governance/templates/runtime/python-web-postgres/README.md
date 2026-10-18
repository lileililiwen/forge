# {{project_name}} container runtime

This runtime installs `requirements.txt`, starts `{{entrypoint}}` with
Uvicorn, and waits for PostgreSQL. The Compose health check probes
`{{health_path}}`.

If the project uses another dependency manager, adapt the generated Dockerfile
as a reviewed project change. Set `POSTGRES_PASSWORD` only through the runtime
environment.
