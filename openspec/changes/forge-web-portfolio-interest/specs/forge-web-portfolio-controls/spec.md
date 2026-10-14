# forge-web-portfolio-controls Specification (delta)

## ADDED Requirements

### Requirement: Portfolio interest reads

Forge SHALL expose the stored interest evidence of the `interest-snapshots` Core store to the
authenticated operator through three read-only admin routes under `/v1/admin/portfolio/{id}/interest`
— `list` (the per-project snapshot list), `show` (the whole projection including refusals), and
`trend` (one metric's windowed history). The portal MUST read through the same
`forge::portfolio::interest_report` Core path the CLI `forge portfolio interest` uses; it MUST NOT
import evidence, MUST NOT expose a write or import control, and MUST NOT re-derive freshness with any
logic other than `interest::freshness_label` so the browser, the CLI and the JSON API report the same
`stale`/`current` label for the same window. `compare`, `audit` and `readiness` remain CLI and JSON-API
reads and are out of scope for this requirement.

#### Scenario: Per-project snapshot list with stale labels

- **WHEN** an operator reads `GET /v1/admin/portfolio/{id}/interest/list` for a managed project that
  has stored interest windows
- **THEN** Forge returns every stored snapshot with a per-snapshot `freshness` of `current` or `stale`
  computed by `interest::freshness_label(window_end, now, stale_after_days)`, a project-level
  `stale_label` of `no-evidence` (no snapshots), `stale` (at least one current window is older than
  the bound) or `current`, and `stale_snapshots`/`stale_after_days` so the portal never invents a
  `current` reading from a stale window

#### Scenario: Whole projection with refusals

- **WHEN** an operator reads `GET /v1/admin/portfolio/{id}/interest/show` for a managed project
- **THEN** Forge returns the whole projection including its `findings` (recorded refusals), each with
  its `field`, `code` and `detail`; the portal renders those refusals verbatim and MUST NOT synthesize
  a clean projection from a refusal

#### Scenario: One metric's windowed trend

- **WHEN** an operator reads `GET /v1/admin/portfolio/{id}/interest/trend?metric=<m>` for a managed
  project
- **THEN** Forge returns the metric's `Trend` with one point per reported window, each carrying its own
  `freshness` (`current`/`stale`), `unreported_windows` counted (never plotted as zero), and the
  `note` stating what the series does not establish

#### Scenario: A stale window is reported stale, never current

- **WHEN** a stored window ended more than `stale_after_days` before the read time
- **THEN** the `list`, `show` and `trend` reads all label that window `stale` exactly as the CLI
  reports it: the value is retained, the source is retained, and no window is dropped, refreshed or
  presented as the present

#### Scenario: Absent evidence is withheld, not healthy

- **WHEN** the operator reads `list`/`show`/`trend` for a managed project with no stored interest
  evidence
- **THEN** Forge returns `measured: false` and `stale_label: "no-evidence"` (or an empty `trend`),
  never a fabricated `current`/`ready` result

#### Scenario: Unmanaged or observed-only project is refused

- **WHEN** the operator reads any of the three interest routes for a project that is not managed by
  this registry
- **THEN** Forge returns the honest typed refusal the Core `require_project` gate produces, with no
  project id echoed
