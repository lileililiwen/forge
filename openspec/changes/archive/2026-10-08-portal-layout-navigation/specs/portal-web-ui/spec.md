# portal-web-ui (delta)

## ADDED Requirements

### Requirement: Responsive breakpoints cover small phones through wide desktop plus short landscape

The dashboard stylesheet SHALL systematize responsive behavior across
the 375 / 768 / 1024 / 1440 steps plus short-landscape orientation:
content tightening at or below 1024px, the existing 850px rule serving
the 768 tablet step, small-phone compaction at 375px, and hero
compaction gated on landscape orientation with short viewport height.
Desktop layout above 1024px SHALL keep its current feel with no
computed-value change.

#### Scenario: Open any dashboard route at 375px wide

- **WHEN** the operator views the login or any dashboard route at 375
  CSS pixels
- **THEN** content, sidebar, topbar, and cards compact with no
  page-level horizontal scroll and every control stays operable

#### Scenario: Open the portal in short landscape

- **WHEN** the operator views the portal in landscape orientation with
  a short viewport height
- **THEN** the auth hero compacts so the sign-in form stays reachable,
  while desktop landscape keeps its full layout

#### Scenario: Desktop above 1024px is unchanged

- **WHEN** the operator views any route on a desktop viewport wider
  than 1024px
- **THEN** layout, spacing, and type match the pre-change design

### Requirement: Small-phone sidebar keeps every destination reachable without page scroll

At and below the tablet step the sidebar SHALL present its 5
destinations in a scrollable in-row nav region (scrolling inside the
nav, never the page), wrapping brand-above/nav-below at 375px. Every
destination SHALL stay reachable and keyboard-focusable, the active
destination SHALL keep its visible active state plus `aria-current`,
and nav targets SHALL keep their 44px minima.

#### Scenario: Reach every destination at 375px

- **WHEN** the operator opens the dashboard at 375 CSS pixels
- **THEN** all 5 nav links are reachable with no horizontal page
  scroll, and the active link is visibly marked

### Requirement: Filter and search state survives reload, back/forward, and view switches

The fleet search box, source filter, and the six catalog predicate
inputs SHALL have their typed values preserved across dashboard boot
(reload), back/forward traversal, and switches back into the projects
view, re-applying the fleet render path on restore. The URL SHALL
remain the source of truth for `?project=` deep links: project
selection is never snapshotted and never restored from storage.

#### Scenario: Reload with fleet filters typed

- **WHEN** the operator types fleet search/filter values and reloads
- **THEN** the values are restored and the table re-filters as before

#### Scenario: Leave and return to the projects view

- **WHEN** the operator switches to another view and back
- **THEN** the typed search/filter values are still present and applied

#### Scenario: Deep links stay URL-owned

- **WHEN** the operator follows `/workbench?project=<id>` or
  `/management?project=<id>` through reload or back/forward
- **THEN** the URL's project boots exactly as before, unaffected by
  any preserved filter state

### Requirement: Unknown dashboard paths render an honest empty state

A client-side pathname outside the dashboard route allowlist SHALL
render an honest empty state naming the miss and linking the 5 real
destinations, instead of silently showing the fleet. No nav link SHALL
claim the active state there, and the crumb/title SHALL name the
missed page.

#### Scenario: Open an unknown path

- **WHEN** the dashboard router sees a pathname outside the route
  allowlist
- **THEN** the unknown section shows with an explanation and the 5
  destination links, and no sidebar link is marked active

### Requirement: Overlays, nav, and dropdowns layer on a named z-index scale

The stylesheet SHALL define a layered z-index scale ordering sticky
chrome, nav, dropdown wrappers, banners, overlays, and the skip link,
with each existing layered element assigned to its rung. The skip link
SHALL remain the topmost layer.

#### Scenario: Inspect the stacking order

- **WHEN** the shipped stylesheet is inspected
- **THEN** a named scale assigns every layered element to its rung
  with the skip link topmost, and sticky/nav/dropdown layers stack
  predictably
