## MODIFIED Requirements

### Requirement: Caddy host rendering in Forge
After successful per-project deploys, Forge SHALL render the host's `platform/Caddyfile` (and `platform/site/index.html` when the template exists) from `RUNTIME_ROOT/port-registry.json` and ship it to the target followed by `docker compose -f PLATFORM_ROOT/compose.yml up -d --force-recreate`. The publish summary SHALL name the address the deploy published, and SHALL NOT claim that the project is reachable, because no stage of a publish observes a reachability.

#### Scenario: Caddy renders after fleet
- **WHEN** `forge publish fleet` finishes with 20/20 healthy
- **THEN** `PLATFORM_ROOT/Caddyfile` contains one host rule per published project at `<project>.tooosall.uk` and the `apps` nav host

#### Scenario: The publish summary names the published address

- **WHEN** a publish deploy completes and the report carries a subdomain
- **THEN** the summary names the published address, and does not claim that the project is reachable
