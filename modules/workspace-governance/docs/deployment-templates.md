# Runtime deployment templates

Workspace Governance owns the reusable runtime template library in
`templates/runtime`. The renderer creates a reviewed starting point for a
project; it does not promote the project to deployable and it never writes
credentials.

List and inspect without changing a project:

```bash
python3 scripts/runtime_templates.py list
python3 scripts/runtime_templates.py inspect --root ../my-project
```

Render only after choosing the type and supplying project-specific values:

```bash
python3 scripts/runtime_templates.py render \
  --root ../my-project \
  --type dotnet-web-postgres \
  --var project_name=my-project \
  --var port=8080 \
  --var health_path=/healthz \
  --var app_project=src/MyProject/MyProject.csproj \
  --var database_name=my_project
```

The command refuses existing generated files unless each file is named with
`--replace`. It records `template_type`, `template_version`,
`rendered_files`, `compose_file`, and `template_state: configured` in
`.project.json`, while preserving `deployment.deployable`.

The six version-one families are:

- `dotnet-web-postgres`: multi-stage ASP.NET runtime and PostgreSQL.
- `rust-web-postgres`: locked release build, non-root Debian runtime, and PostgreSQL.
- `python-web-postgres`: Uvicorn web runtime and PostgreSQL.
- `node-web`: npm install/build production runtime without a database assumption.
- `static-site`: nginx runtime for a prebuilt output directory.
- `worker-postgres`: Python worker runtime and PostgreSQL.

Each family includes a Dockerfile, Compose contract, Docker build exclusions,
and a family-specific README. Adapt generated commands and image versions as a
normal project change, then run the project verification and Compose check:

```bash
python3 scripts/runtime_templates.py check --root ../my-project
```

`check` reports `TOOL_UNAVAILABLE` when Docker Compose is not installed. That
is configuration evidence, not runtime verification.
