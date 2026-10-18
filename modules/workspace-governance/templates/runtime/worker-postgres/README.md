# {{project_name}} container runtime

This runtime installs `requirements.txt` and starts the worker with
`{{worker_command}}` after PostgreSQL reports healthy. The database URL is
provided through the runtime environment.

Set `POSTGRES_PASSWORD` outside the repository and review the worker command
before promotion.
