# Design: independent project inventory and container fleet

## Implementation boundary

Forge owns `src/publish/inventory.rs`, inventory source selection, normalized
snapshot persistence, and fleet eligibility. `jenkins-local` owns the external
Mac provider and route refresh. workspace-governance may ship an adapter but
Forge imports no workspace-governance code. Project repositories own their
Dockerfiles and Compose files.

## Language and runtime

Forge: Rust 1.87+, Cargo, serde, SQLite, clap. Provider: Python 3 JSON
executable. Runtime: Mac Docker Desktop, Docker Compose, Caddy, and one
Cloudflare wildcard tunnel. Linux orchestrates only; builds and containers run
on Mac.

## Ownership and shared code

Forge consumes a provider-neutral inventory response. A local file provider is
always available. External providers receive a JSON request over stdin and
return JSON stdout; they are configured by executable path and explicit
invocation arguments. workspace-governance is one adapter, not a dependency.

## Contract

Inventory contract `forge-project-inventory/0.1.0`:

```json
{
  "contract": "forge-project-inventory/0.1.0",
  "provider": "local",
  "generated_at": "RFC3339",
  "projects": [{
    "id": "example",
    "repository": "https://github.com/org/example.git",
    "revision": "40-hex-sha",
    "profile": "rust-product",
    "runtime": "web|worker|job|library",
    "compose_file": "docker-compose.yml",
    "source_path": "/invocation/only/example",
    "public_http": true,
    "public_port": 8080
  }]
}
```

`source_path` is never persisted as the project identity and is optional for a
remote Git source. Project IDs and revisions are validated; duplicate IDs,
missing repository/revision, malformed JSON, and unsupported runtime values are
rejected. A project without Compose is returned as `compose_missing`, not
silently dropped.

## Behavioral model

1. Resolve one explicitly selected inventory source.
2. Validate and snapshot the normalized inventory.
3. Classify every entry: `compose_ready`, `compose_missing`, `invalid`, or
   `source_unavailable`.
4. Publish `compose_ready` entries serially through the selected provider.
5. Read the Mac runtime port registry after a healthy run.
6. Generate Caddy routes only for `public_http=true` HTTP services.
7. Keep the wildcard Cloudflare tunnel unchanged.

Runtime classes determine exposure: `web` may route HTTP; `worker`, `job`, and
`library` run without a public route unless an explicit public HTTP declaration
exists. No database or private service is routed.

## Failure and compatibility

- No explicit source: use only a Forge-local configured inventory file; never
  search sibling directories.
- External adapter disabled/unavailable: typed failure; no stale inventory
  unless an explicit stale override is supplied.
- Missing Compose/Dockerfile: per-project failure; other projects continue
  unless `--fail-fast` is selected.
- Invalid public port or duplicate hostname: route generation fails closed for
  that route and preserves other valid routes.
- Existing seven-project `deployment/projects.json` remains readable through a
  compatibility adapter during migration.
- Mac source remains temporary and is removed after each provider operation.

## Verification oracle

- Rust unit tests validate schema, IDs, SHA, runtime classes, duplicates, and
  missing Compose classification.
- Adapter contract tests cover valid output, malformed output, timeout,
  disabled provider, and unavailable workspace-governance path.
- Fleet tests prove all inventory entries are reported and only valid Compose
  entries are invoked serially.
- Provider tests prove public/private route selection and wildcard-safe Caddy
  rendering.
- Mac evidence must include container health, port registry, Caddy route, and
  one Cloudflare URL check for every public HTTP project.

## Decision ledger

- Local inventory is the standalone Forge baseline.
- workspace-governance is optional and selected by explicit adapter config.
- Profiles describe runtime generation/requirements but do not silently imply
  deployability.
- Every accepted runtime requires a real Compose contract; placeholder
  containers are invalid.
- Cloudflare remains one wildcard route to Caddy, not N project routes.
