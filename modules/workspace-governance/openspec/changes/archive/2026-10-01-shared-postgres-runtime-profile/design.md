# Design: Runtime templates for an external shared PostgreSQL service

## 1. Implementation boundary

Repository `/home/paul/code/workspace-governance`, Python 3 runtime-template renderer and static Docker/Compose assets under `templates/runtime/`. Extend `templates/runtime/registry.json`, `scripts/runtime_templates.py`, existing template contract specs/tests, and add four `*-postgres-external/` families with Dockerfile/Compose/.dockerignore/README/.env.example. Reuse the renderer's placeholder validation and safe generated-file refusal. Do not modify project application source or any PostgreSQL service.

## 2. Language and runtime

Python 3 using repository-pinned workspace tooling; Docker Compose v2 is an optional validation executable (absence reports `TOOL_UNAVAILABLE`). Rendered variants cover .NET ASP.NET, Rust web, Python ASGI, and Python worker. Validation uses repository unittest suite, runtime-template self-check, Jenkins manifest check, and strict OpenSpec validation.

## 3. Ownership and shared code

Workspace Governance owns versioned runtime template IDs and renderer behavior. Forge and OpenPanel consume generated files/deployment metadata; they do not duplicate template source. No common database service or workspace daemon is introduced. Existing container-backed templates remain the default and unchanged.

## 4. Behavioral model

Add template IDs `dotnet-web-postgres-external`, `rust-web-postgres-external`, `python-web-postgres-external`, and `worker-postgres-external`. Each renders one app/worker service only; no `db` service, volume, health dependency, or init command is emitted. Runtime config uses required `PGHOST`, optional `PGPORT` default 5432, `PGDATABASE`, `PGUSER`, and `PGPASSWORD` values loaded from a local `.env` or host secret injection. `.env.example` has blank password and non-secret placeholder values. Template variables include project/runtime-specific source paths and non-secret database name/user, never a password.

The renderer creates `.env.example` and, idempotently, adds a root `.env` ignore entry to `.gitignore` while preserving existing bytes/line endings. If `.gitignore` cannot be safely parsed or contains a conflicting unignore rule, rendering refuses with `SECRET_IGNORE_CONFLICT` before writing any template file. `.dockerignore` excludes `.env`, `.env.*` except `.env.example`, and database dumps. Generated docs instruct operators to provision one database and one least-privilege DB role per project, create `.env` locally with restricted file permissions, and pass the connection host for the target network. Current Mac Docker users may set host to `host.docker.internal`; this value is documentation example only. Cloud runtime may select a private service DNS name or return to existing container-backed profile.

## 5. Contract and compatibility

Registry declares required variables and generated files for each new ID. CLI remains `runtime_templates.py list|inspect|render|check`; `--type <new-id>` selects external mode. `template_type` and version are written to `.project.json`; no credentials or endpoint secret are stored there. Runtime health checks verify the app only and do not claim database backup/readiness beyond the app's own health endpoint. Existing IDs and render output are byte-identical.

Errors: missing host/database/user values fail Compose interpolation with a named variable; blank/missing password fails at startup; missing `.env` yields documented configuration failure; unknown template ID remains the existing typed not-found result; unsafe `.gitignore` state refuses before partial output. `docker compose config` must not be used in captured logs containing real `.env` values.

## 6. Failure and boundary policy

External database unavailable → application health reports its existing readiness failure; renderer does not substitute a local container. No server or volume cleanup is ever attempted. Different projects with duplicate database/user values are not automatically detected across hosts; docs and tests enforce that generated IDs/variables are project-scoped and warn that operators must provision unique principals. Network and TLS policy are passed through to app configuration; production credentials are not rendered or committed. Switching back to isolated-container mode is a new template render with generated-file replacement confirmation and explicit data migration outside this change.

## 7. Verification oracle

For each of four template IDs, render a fixture and assert Compose has exactly one application service, no `db`, no PostgreSQL volume, correct environment-variable mapping, `.env.example` blank password, `.env` ignored, and non-secret `.project.json` metadata. Verify existing templates compare byte-for-byte to golden snapshots. Test missing/unsafe `.gitignore`, preexisting generated-file refusal, idempotent rerender, and secret-shaped template argument rejection. Run `docker compose config` against a fixture `.env` with dummy credentials when Compose exists; otherwise report `TOOL_UNAVAILABLE` without claiming validation.

## 8. Decision ledger

Resolved: external service is opt-in; current per-project PostgreSQL container remains default; each project owns a distinct DB/user; only a blank `.env.example` is committed; endpoint selection is environment-specific; no Mac hostname is default. Deferred: OpenPanel provisioning of PostgreSQL users, automated quotas/backups, TLS enforcement policy, and provider-specific remote secret stores. Priority is intentionally below product creation/auth/publishing work. Blockers: none.
