# portal-web-ui (delta)

## ADDED Requirements

### Requirement: Portal icons are inline SVG from one stroke set

Every UI glyph SHALL be an inline SVG from a single consistent stroke
set (24 viewBox, `currentColor`, round caps/joins, uniform
stroke-width token, sm/md/lg size tokens) instead of a unicode glyph.
Decorative icons beside visible text SHALL carry `aria-hidden`;
accessible names on labelled controls SHALL stay intact; no emoji
SHALL appear anywhere in the portal.

#### Scenario: Scan nav, search, summary, and empty states

- **WHEN** the operator views any dashboard route or the login page
- **THEN** sidebar, search, summary-card, empty-state, story-check,
  lock, submit, and disclosure icons render from the one stroke set
  at uniform weight and aligned sizes, with no unicode-glyph icon
  remaining

#### Scenario: Assistive technology meets an icon

- **WHEN** a screen reader encounters a decorative icon beside visible
  text
- **THEN** the icon is hidden (`aria-hidden`) and the visible label
  (or `sr-only`/`aria-label` name on labelled controls) announces as
  before

### Requirement: Portal type declares its base, scale, figures, and prose guard

The stylesheet SHALL declare an explicit 16px base with body
line-height in the 1.5–1.75 band, a named type scale, a 65–75ch
line-length guard on prose, tabular figures for counts/ids/timestamps,
and long-token wrapping via `overflow-wrap:anywhere` (never
`word-break:break-all`). Every normal-text pair SHALL keep meeting
4.5:1.

#### Scenario: Read body copy and figures

- **WHEN** the operator reads dashboard or login copy at the default
  size
- **THEN** body text renders at 16px with 1.6 line-height, prose lines
  wrap at or under 70ch, and counts/ids/digests/timestamps render in
  tabular figures

#### Scenario: A long token meets a narrow card

- **WHEN** a digest or token exceeds its container width
- **THEN** it wraps sanely (`overflow-wrap:anywhere`) without clipping
  or breaking the layout

#### Scenario: Contrast after the type pass

- **WHEN** any portal text/background pair is measured
- **THEN** normal-text contrast still meets 4.5:1 (no color token
  changed)

### Requirement: Portal motion runs on shared enter/exit tokens

Interactive motion SHALL use shared duration/easing tokens with exits
at 60–70% of enters (no one-duration-everywhere), SHALL stay
transform/opacity-only, SHALL keep the `prefers-reduced-motion` guard,
and SHALL animate no layout property.

#### Scenario: Hover and press a control

- **WHEN** the operator hovers (enter) and presses (exit) a button,
  nav link, or action head
- **THEN** the enter runs at the shared enter duration and the press
  confirms at the faster shared exit duration, with no sibling
  movement

#### Scenario: Reduced-motion operator uses the portal

- **WHEN** reduced motion is preferred
- **THEN** all transitions/animations collapse as before and
  programmatic scrolls stay instant
