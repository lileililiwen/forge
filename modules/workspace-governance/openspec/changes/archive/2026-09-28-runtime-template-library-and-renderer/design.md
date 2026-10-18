# Design: Add reusable runtime Docker and Compose templates

## Implementation boundary

Implementation is local to `/home/paul/code/workspace-governance`:

- `templates/runtime/<type>/template.json` — template metadata and variable
  contract.
- `templates/runtime/<type>/{Dockerfile,compose.yaml,.dockerignore,README.md}`
  — versioned render sources.
- `scripts/runtime_templates.py` — CLI and renderer.
- `schemas/project.schema.json` — deployment-template metadata extension.
- `scripts/workspace_check.py` — declaration validation for rendered metadata.
- `tests/test_runtime_templates.py` — unit, fixture, failure, and portability
  tests.
- `README.md` and `docs/deployment-templates.md` — operator documentation.

Do not change product repositories, `jenkins-local`, or DriftWatchdog in this
package. Do not use absolute workspace paths in templates or generated files.

## Language and runtime

Python 3.11-compatible standard-library implementation, using `argparse`,
`json`, `pathlib`, and `string.Template` or an equivalent bounded renderer.
Verification commands:

```bash
python3 -m unittest discover -s tests -v
python3 scripts/workspace_check.py --self-check
python3 scripts/jenkins_manifest.py --check --root ..
openspec validate --all --strict --no-interactive
git diff --check
```

## Ownership and shared code

Workspace Governance owns template contracts and rendering. The renderer may
read the target repository and its `.project.json`, but it writes only the
explicit target files and declaration metadata. Jenkins consumes
`deployment.compose_file`; DriftWatchdog consumes the generated Gate command.
Neither sibling is imported as source code.

## Template type contract

| Type | Runtime evidence required | Generated runtime |
|---|---|---|
| `dotnet-web-postgres` | `.sln`/`.csproj`, API project, port, health path, database key | multi-stage SDK/ASP.NET image + PostgreSQL service |
| `rust-web-postgres` | `Cargo.toml`, binary/package name, port, health path, database key | multi-stage Rust image + PostgreSQL service |
| `python-web-postgres` | `pyproject.toml`/requirements, entrypoint, port, health path, database key | Python image + PostgreSQL service |
| `node-web` | `package.json`, build/start scripts, port | dependency/build image + runtime image |
| `static-site` | static output directory and serving strategy | Nginx runtime image |
| `worker-postgres` | worker command, package manifest, database key | worker image + PostgreSQL service |

Each type declares required variables in `template.json`. Unsupported or
ambiguous detection returns a structured diagnostic listing the missing
evidence and the explicit `--type`/variable needed.

## Renderer behavior

Commands:

```text
runtime_templates.py list
runtime_templates.py inspect --root <repo>
runtime_templates.py render --root <repo> --type <type> --var key=value ...
runtime_templates.py check --root <repo> [--type <type>]
```

`render` behavior:

1. Resolve and confine the target root.
2. Load `.project.json` if present and validate its identity.
3. Inspect manifests and derive suggestions; never silently select a type when
   more than one type is plausible.
4. Validate all required variables against bounded names and repository-
   relative values.
5. Render to memory, reject unresolved placeholders, absolute paths, secret
   literals, and output paths outside the target root.
6. Refuse if any target file exists unless `--replace <relative-path>` names
   that exact file. `--force` is not supported.
7. Write files atomically and update only the deployment-template metadata in
   `.project.json`; preserve all unrelated declaration fields.
8. Set `deployment.deployable` to `false` unless it was already true; render
   state is configuration evidence, not deployment promotion.

Generated metadata:

```json
{
  "template_type": "dotnet-web-postgres",
  "template_version": 1,
  "rendered_files": ["Dockerfile", "compose.yaml", ".dockerignore", "docs/DEPLOYMENT.md"],
  "compose_file": "compose.yaml",
  "state": "configured"
}
```

`deployment.compose_file` remains the existing Jenkins handoff field. The
additional object is additive and schema-validated.

## Generated file contract

Every runtime template MUST:

- use a multi-stage build when the runtime supports compilation;
- run as a non-root user where the base image permits it;
- expose a project variable for the application port;
- define a healthcheck or document why an external probe is required;
- use environment substitution for secrets with no committed defaults;
- define PostgreSQL health ordering when PostgreSQL is part of the type;
- include a `.dockerignore` that excludes Git state, local state, secrets,
  build output, dependencies, and test artifacts;
- include a README describing build, config, health, migration, and rollback
  boundaries;
- remain valid under `docker compose config` after rendering with fixture
  variables.

Templates may not claim production readiness, publish images, or embed
credentials.

## Failure and boundary policy

- Missing manifest evidence: `INSUFFICIENT_EVIDENCE`, no write.
- Ambiguous type: `TYPE_REQUIRED`, no write.
- Unknown type or variable: `TEMPLATE_INVALID`, no write.
- Existing output without explicit per-file replacement: `TARGET_EXISTS`, no
  partial write.
- Placeholder, secret, traversal, or absolute path: `RENDER_REJECTED`, no write.
- Invalid target declaration: `DECLARATION_INVALID`, no write.
- Docker unavailable during `check`: report `TOOL_UNAVAILABLE`; do not claim
  Compose validity from static text alone.
- Template render succeeds but Docker build/health is not run: metadata remains
  `configured`, never `verified`.

## Verification oracle

- Every template metadata file parses and references exactly the declared
  render files.
- Each type renders a fixture project and has no unresolved placeholders,
  absolute paths, secret literals, or path traversal.
- Each rendered fixture passes `docker compose config` when Docker Compose is
  installed; otherwise the test records an unavailable tool boundary.
- Existing-file refusal proves no partial writes.
- Ambiguous and missing-variable fixtures prove fail-closed behavior.
- Declaration schema and workspace self-check accept generated metadata.
- Jenkins manifest consumes the generated relative Compose path without
  changing deployment behavior.

## Decision ledger

- Common ownership is Workspace Governance because it already owns project
  profiles, bootstrap declarations, schemas, and deployment metadata.
- Template types are explicit and versioned; detection suggests but never
  silently chooses an architecture.
- Product migration is deferred to separate adoption changes after the
  renderer is verified.
- Existing DriftWatchdog scaffold files are not imported into the renderer;
  the renderer emits the established contract directly.
