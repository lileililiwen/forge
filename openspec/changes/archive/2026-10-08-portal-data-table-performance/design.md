# Design: Portal data table performance (slice 6)

## Ownership

- View state owner: `frontend/app.js` fleet module (sort key/dir, page
  cursor). No other module reads or writes it.
- Markup owner: `frontend/index.html` project panel (sort buttons,
  export button, pager region, skeleton-compatible tbody).
- Style owner: `frontend/styles.css` appended slice-6 block only.
- Contract owner: `tests/forge_web_navigation_contract.rs` slice-6
  static token test; no server contract changes.

## Contracts

- Sort: exactly one sorted column at a time; the active `<th>` carries
  `aria-sort="ascending"|"descending"`, the others `aria-sort="none"`.
  Each header control is a native `<button type="button">` inside the
  `<th>` (keyboard-reachable, 44px minima kept), labelled
  `Sort by <column>`. Click cycles `asc → desc → none` (none restores
  API order). Sort keys: `name` (`project.name || identity`,
  case-insensitive), `details` (profile words + management suffix —
  the same string the cell renders), `status` (the same
  `projectStatusLine` text the cell renders). Stable: ties break by
  pre-sort index. Sort applies AFTER the existing
  free-text/source/catalog-predicate filter — filter code and the
  `limit:1000` catalog query are untouched.
- Export: `#fleet-export` (`button.button-quiet`, `Export filtered
  CSV`) serializes the current filtered+sorted (pre-page) row set:
  header `name,identity,profile,details,status,source,management` +
  one RFC-4180-escaped row per project (quotes doubled, fields with
  `,"` or newline quoted). Download is a `Blob` (`text/csv`) object
  URL on a temp `<a download="forge-projects.csv">`, revoked after
  click. Disabled while the filtered set is empty. No fetch, no
  endpoint, no server state.
- Pagination: `FLEET_PAGE_SIZE = 50`. `renderProjects` mounts at most
  one page; `#project-count` reports the honest window
  (`Showing 1–50 of 132 filtered (410 total)`; short fleets
  `Showing 7 of 7 filtered (7 total)`; empty stays `0`). `#fleet-prev` /
  `#fleet-next` (44px, labelled `Previous/Next fleet page`) plus
  `#fleet-page-info` (`Page 1 of 3`, `aria-live="polite"`) live in
  `#fleet-pager` (nav-labelled region). Any filter/sort change resets
  to page 1 (clamped when the set shrinks). Page state is in-memory
  only — deep links (`?project=`) and the slice-4 filter snapshot are
  unaffected.
- Loading: while `/v1/admin/projects` is in flight, `renderFleetSkeleton`
  mounts 8 skeleton rows (a `<td colspan>`-free row of three cells each
  carrying `span.skeleton`, `aria-hidden`) inside the visible table
  region with `aria-busy="true"`; `#project-count` keeps its text node
  (now `Loading your projects…` alongside skeleton rows, not instead of
  content). Delivery/maintain loaders render the same `.skeleton` span
  instead of bare text. Summary counts (`#summary-*`, `#fleet-*`) get
  reserved space (`inline-block`, `min-width:3ch`, `min-height:1.2em`)
  so arrival does not shift layout. Shimmer is a `background-position`
  animation only, collapsed under the existing
  `prefers-reduced-motion` guard; no layout property animates.

## Failures

- Fleet fetch fails: existing `showDashboardError` path unchanged;
  skeleton rows are replaced by the error, never left spinning.
- Catalog predicate read fails: existing lift-constraint + `role="alert"`
  path unchanged; sort/page still apply to the unfiltered set.
- Empty filtered set: table region hides (existing rule), pager and
  export disable, `No matching projects` empty state shows (unchanged).
- CSV with zero rows: export button is `disabled`; no empty-file
  download is offered.
- `Blob`/object-URL unavailable (non-browser harness): the click
  no-ops after the disabled check; no exception escapes (guarded).

## Migrations

None. No stored format changes: slice-4 `forge.filter-state.v1`
snapshot keeps its keys (sort/page intentionally not persisted).
No canonical spec changes beyond appending 4 requirements to
`portal-web-ui`. No API/registry/journal/CLI/catalog migration.
