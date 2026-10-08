# portal-web-ui (delta)

## ADDED Requirements

### Requirement: Fleet table columns sort with an announced sort state

The fleet `Project`, `Details`, and `Status` columns SHALL be sortable
through keyboard-reachable header controls that expose the current sort
state with `aria-sort` on the header cell. Sorting SHALL be stable and
SHALL compose with the existing free-text, source, and catalog-predicate
filters (filter code, the catalog `limit:1000` query, and the JSON shape
stay unchanged). The `Open` action column SHALL stay unsorted.

#### Scenario: Sort the fleet by a column

- **WHEN** the operator activates the `Project` header control
- **THEN** rows order by project name (stable for ties), the header
  cell reports `aria-sort="ascending"`, and activating again reverses
  to `descending`

#### Scenario: Sort composes with filters

- **WHEN** free-text, source, or catalog-predicate filters are active
  and a sort is applied
- **THEN** the table shows the filtered set in the requested order —
  never unfiltered rows, never a filter reset

#### Scenario: Keyboard and screen-reader operation

- **WHEN** the operator tabs to a fleet header control or inspects it
  with assistive technology
- **THEN** the control is reachable by keyboard, labelled
  `Sort by <column>`, and its header cell announces the current sort
  state via `aria-sort`

### Requirement: Filtered fleet rows export as CSV in the browser

The dashboard SHALL offer an `Export filtered CSV` control that
downloads the currently filtered and sorted fleet rows (pre-page) as
an RFC-4180-escaped CSV through a plain browser download. No new
endpoint SHALL be called and no server state SHALL be created.

#### Scenario: Export the current view

- **WHEN** the operator activates `Export filtered CSV` with filters
  and a sort applied
- **THEN** the downloaded file contains a header row plus one row per
  filtered project in the displayed order, with commas, quotes, and
  newlines escaped

#### Scenario: Empty filtered set

- **WHEN** no project matches the current filters
- **THEN** the export control is disabled and no download is offered

### Requirement: Large fleets render windowed with honest counts

The fleet table SHALL mount at most one page of rows
(`FLEET_PAGE_SIZE = 50`) and SHALL report the honest window
(`Showing X of Y filtered (Z total)`). Paging SHALL apply after
filtering and sorting so filter composition stays exact, and any
filter or sort change SHALL reset to the first page.

#### Scenario: Browse a large filtered fleet

- **WHEN** the filtered set exceeds one page
- **THEN** only the current page's rows are mounted, the count names
  the window and the filtered/total figures, and Previous/Next move
  the window without changing the set

#### Scenario: Shrink the set while paged

- **WHEN** the operator is past page one and narrows the filters so
  fewer pages remain
- **THEN** the view clamps to a valid page (first page on filter or
  sort change) and the count stays exact

### Requirement: Async regions show skeleton placeholders and reserve count space

While the fleet fetch is in flight the table region SHALL show
skeleton/shimmer placeholder rows (not text alone) with
`aria-busy="true"`, and delivery/maintain loaders SHALL use the same
skeleton treatment. Summary counts SHALL reserve space so their
arrival does not shift layout. Shimmer SHALL collapse under
`prefers-reduced-motion` and SHALL animate no layout property.

#### Scenario: Load the dashboard on a slow connection

- **WHEN** `/v1/admin/projects` has not yet resolved
- **THEN** the fleet region shows placeholder rows with `aria-busy`,
  and summary counts keep their reserved space instead of reflowing
  on arrival

#### Scenario: Reduced-motion operator loads the dashboard

- **WHEN** reduced motion is preferred
- **THEN** placeholders render statically with no shimmer animation
