# Design: Fleet registry observation

## Ownership and boundaries

Workspace Governance owns `projects.json`: its schema, adoption decisions
and any writes. Forge owns a read-only parser and projection of that
document into fleet observations. The local SQLite registry remains the
only source of truth for managed projects; fleet data is never copied into
it.

## Input contract

Accepted document shape (schema_version 1):

```json
{"schema_version": 1,
 "discovery": {"mode": "...", "exclude": ["..."]},
 "projects": [{"id", "path", "profile", "lifecycle", "adoption"}]}
```

Unknown `schema_version` → typed refusal. Duplicate ids, blank paths, paths
resolving outside `<registry-dir>` after canonicalization, and symlinked
project dirs that escape the root are malformed-entry refusals naming the
entry. Other valid entries still report (per-entry isolation, the same rule
the distribution adapter uses).

## Query model

`fleet::observe(registry_path, max_age) -> FleetReport`:

- entries with `id`, declared fields, `forge.yaml_present`,
  `locally_registered` (join against the local registry by id),
  `state: managed|unmanaged`;
- report-level `source` (absolute path), `observed_at`, `freshness`
  (`fresh|stale|unconfigured`) from file mtime vs `max-age` (default 86400s,
  matching the docs freshness vocabulary);
- never invents profiles: the WG `profile` vocabulary (`rust-product`,
  `dotnet-library`, …) is surfaced verbatim and not coerced to Forge
  profiles.

## Surfaces

- CLI: `forge fleet list [--registry R] [--max-age S]`,
  `forge fleet status` (registry health only), `forge fleet inspect <ID>`.
  Human + `--format json`.
- Portal: a fleet block on the projects section for `scope: fleet` views
  when a registry is configured; entries marked `unmanaged` cannot be
  drilled into operations, only inspected. Status vocabulary reuses the
  portal's `ok|warn|unknown|unavailable` roll-up; a stale registry is
  `warn`, an absent one `unavailable` — never masked as ok.

## Why no mirror table

Persisting fleet entries would create a second fleet truth that drifts from
WG's registry and tempts mutation. Re-reading a bounded JSON file is cheap;
staleness is then expressed honestly by `observed_at`.

## Failure and security

Read size bounded (registry files above a cap refuse as invalid); strings
redacted before display; no execution of registry content; no writes outside
`.forge/` read caches (there are none) and the SQLite journal (none — fleet
reads are not journaled operations).

## Verification

Fixtures copied from WG's real schema; mutation-detection tests comparing
fixture-tree hashes before/after a fleet round trip; portal parity tests;
CLI error-code tests for every malformed-entry class.
