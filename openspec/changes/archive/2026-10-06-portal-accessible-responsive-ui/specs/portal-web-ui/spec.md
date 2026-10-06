## ADDED Requirements

### Requirement: Responsive portal reflow

Forge SHALL render every portal page so the content reflows at 320 CSS pixels and 400% zoom without page-level horizontal scrolling, except for bounded two-dimensional data tables that may scroll within their own named region.

#### Scenario: Narrow viewport

- **WHEN** an operator views fleet, project detail, portfolio, publish plan/result, error, or Studio pages at 320 CSS pixels
- **THEN** navigation and metadata reflow, controls remain visible, and the document has no horizontal overflow; any wide table scrolls only inside its labelled table region

#### Scenario: Zoom and long content

- **WHEN** text is enlarged to 200% or the page is viewed at 400% zoom with long project identifiers, URLs, or notes
- **THEN** text and controls remain available without overlap, clipping, or loss of content

### Requirement: Semantic structure and keyboard operation

Forge SHALL give every portal document a language, landmark structure, skip link, unique main target, logical heading order, labelled controls, named data tables, and visible keyboard focus. Native semantic HTML SHALL be preferred over redundant ARIA roles.

#### Scenario: Keyboard-only operation

- **WHEN** a user navigates any portal page using only a keyboard
- **THEN** the skip link, navigation, filters, project links, table region, and mutation controls are reachable in reading order and every focused item has a visible, unobscured indicator

#### Scenario: Form and table semantics

- **WHEN** assistive technology inspects a portal form or data table
- **THEN** each input has an associated label, each table exposes a name/caption and scoped headers, and each status is conveyed as text rather than color alone

### Requirement: Portal visual contrast and interaction targets

Forge SHALL meet WCAG 2.2 AA contrast requirements for rendered text and interactive boundaries/focus indicators, provide pointer targets of at least 24 by 24 CSS pixels unless a WCAG exception applies, and honor reduced-motion preference.

#### Scenario: Light and dark presentation

- **WHEN** any portal page renders in light or dark system color scheme, including default, hover, and keyboard-focus states
- **THEN** measured normal-text contrast is at least 4.5:1, large text and non-text UI/focus contrast is at least 3:1, and status remains understandable without color

#### Scenario: Reduced motion and target size

- **WHEN** a user enables reduced motion or uses pointer/touch input
- **THEN** nonessential motion is removed and interactive targets meet the minimum target size without clipping or overlap

### Requirement: Portal accessibility verification

Forge SHALL verify representative rendered portal page families at narrow and desktop viewports with browser automation and record measured contrast and keyboard/accessibility-tree evidence; markup-only tests SHALL NOT be reported as full accessibility verification.

#### Scenario: Browser evidence covers all page families

- **WHEN** the portal UI verification suite runs
- **THEN** it exercises fleet, detail, portfolio, publish plan/result, error, and Studio pages for reflow, keyboard focus, semantic names, and light/dark contrast, and reports each failed page/state
