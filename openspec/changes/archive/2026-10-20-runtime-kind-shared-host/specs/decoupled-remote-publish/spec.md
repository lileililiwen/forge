## MODIFIED Requirements

### Requirement: Caddy host rendering in Forge
After successful per-project deploys, Forge SHALL render the host's `platform/Caddyfile` (and `platform/site/index.html` when the template exists) from `RUNTIME_ROOT/port-registry.json` and ship it to the target followed by `docker compose -f PLATFORM_ROOT/compose.yml up -d --force-recreate`.

#### Scenario: Caddy renders after fleet
- **WHEN** `forge publish fleet` finishes with 20/20 healthy
- **THEN** `PLATFORM_ROOT/Caddyfile` contains one host rule per published project at `<project>.tooosall.uk` and the `apps` nav host

### Requirement: Contract preservation for roster, journal, and wildcard routing
`forge publish fleet` SHALL continue to derive the roster from `workspace-governance/projects.json` (or explicit `--inventory` / `--fleet-registry`), filter to the entries the fleet may publish — compose-ready entries whose runtime kind is an own-process kind — report every entry it did not publish explicitly, persist the existing queue/phase `operations` journal without new columns, and route public HTTP hosts through the existing wildcard Cloudflare → Caddy boundary.

#### Scenario: Fleet eligibility unchanged
- **WHEN** `forge publish fleet --dry-run --format json` is run against the current `workspace-governance/projects.json`
- **THEN** it reports `publishable=20`, `skipped=48` with each skipped entry's typed classification, and only the entries it publishes receive a journal row

#### Scenario: A shared-host module is held back
- **WHEN** the roster declares an entry whose runtime kind is a shared-host kind
- **THEN** the fleet reports it with its host model and the reason the adopting project owns it, and does not publish it or write a journal row for it
