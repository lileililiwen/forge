# platform-react-app-templates Specification

## Purpose
TBD - created by archiving change platform-react-app-templates. Update Purpose after archive.
## Requirements
### Requirement: React templates SHALL use one platform design contract

The package SHALL expose light and dark Ant Design theme configuration generated from the platform DTCG semantic tokens. The templates SHALL use the existing platform React primitives for buttons, fields, cards, badges, and feedback states and SHALL NOT provide competing platform implementations of those primitives.

#### Scenario: Consumer selects a theme

- **WHEN** a consumer selects light or dark mode
- **THEN** platform primitives and Ant Design controls use values derived from the corresponding generated semantic token map
- **AND** no color literal is introduced in the theme adapter

#### Scenario: Agent composes a form

- **WHEN** an AI agent adds a button, field, card, badge, or feedback state
- **THEN** the agent guide directs it to the existing `@platform/react-ui` component
- **AND** the template verifier rejects a duplicated platform base primitive

### Requirement: Public, user, and admin layouts SHALL provide distinct hierarchy

The package SHALL expose separate public content, signed-in user dashboard, and admin dashboard layouts. Each SHALL expose a labeled main landmark, a skip link, a page heading/focus target, and explicit navigation configuration. Admin navigation visibility SHALL be treated only as presentation and SHALL NOT imply authorization.

#### Scenario: Public visitor opens a product page

- **WHEN** a visitor opens a public page
- **THEN** the content-first layout presents product identity, primary navigation, a main landmark, and footer without requiring an authenticated shell user

#### Scenario: Signed-in user opens their dashboard

- **WHEN** a user opens an account page
- **THEN** the user layout presents user navigation, current page context, account actions, and main content

#### Scenario: Administrator lacks a feature permission

- **WHEN** the shell's permission metadata omits an admin navigation permission
- **THEN** that navigation entry is not presented
- **AND** the backend remains responsible for denying unauthorized requests

### Requirement: Templates SHALL reflow across mobile and desktop viewports

Templates SHALL remain usable at 320, 375, 768, 1024, and 1280 CSS-pixel widths without page-level horizontal scrolling. Data that inherently requires two dimensions MAY scroll inside a named, keyboard-reachable region.

#### Scenario: Admin opens a table on a phone

- **WHEN** the viewport is 320 CSS pixels wide
- **THEN** navigation and content controls remain reachable, the page itself does not scroll horizontally, and a wide table scrolls only inside its labeled data region

#### Scenario: Dashboard opens on a tablet

- **WHEN** the viewport is 768 CSS pixels wide
- **THEN** multi-column dashboard sections collapse or reflow without clipping controls or hiding page actions

### Requirement: Interactive content SHALL expose accessible names and states

Templates SHALL provide visible keyboard focus, a focus target after route changes, meaningful labels for Ant Design controls, text cues in addition to color, reduced-motion support, and explicit caller-provided loading, empty, error, and forbidden state content. A retry action SHALL invoke a supplied callback and SHALL NOT be shown when none exists.

#### Scenario: Keyboard user changes route

- **WHEN** a keyboard user activates a navigation link
- **THEN** focus remains visible, the current item is identified with `aria-current`, and focus can move to the new page heading/main target

#### Scenario: Data request fails

- **WHEN** a caller renders an error state with a recovery callback
- **THEN** the template displays safe failure copy and an actionable retry control
- **AND** it does not expose raw exception details

#### Scenario: Status color changes

- **WHEN** a status is success, warning, danger, stale, selected, or neutral
- **THEN** its meaning is available as text or another non-color cue

### Requirement: Charts SHALL have nonvisual and non-hover alternatives

Any chart pattern supplied by the template SHALL include a concise text summary and a table or downloadable data alternative. The initial package SHALL NOT introduce a charting dependency.

#### Scenario: User reviews a dashboard chart

- **WHEN** a page presents a chart in a supported composition
- **THEN** the chart purpose and key result are available as text and exact values can be accessed without hover

### Requirement: React templates SHALL be consumable as a versioned platform artifact

The new package SHALL be included in the deterministic UI feed with exact name, version, digest, and package membership. Ant Design SHALL be declared as a peer dependency and SHALL NOT be copied into the feed artifact.

#### Scenario: Consumer vendors the platform feed

- **WHEN** a consumer installs the platform React template tarball from its repository-local vendor path
- **THEN** the declared platform dependencies and Ant Design peer contract are explicit
- **AND** no sibling checkout, absolute path, registry credential, or environment-derived package path is required for the platform tarball

### Requirement: AI agents SHALL have a constrained template guide

The package documentation SHALL identify the correct template for public, user, and admin screens; the component ownership rules; allowed customization through branding and semantic tokens; and prohibited patterns that bypass the shared design contract.

#### Scenario: Agent creates a new admin page

- **WHEN** an agent starts a new admin route from the template guide
- **THEN** it can select an admin layout, compose data controls from approved platform/Ant Design components, supply required accessible states, and avoid inventing a second token palette

### Requirement: UI quality checks SHALL reject template contract regressions

The UI verifier SHALL identify missing template exports, ungenerated theme mappings, undeclared color literals, duplicated base primitives, missing peer metadata, and feed-manifest mismatches with actionable diagnostics.

#### Scenario: Required theme mapping is missing

- **WHEN** a semantic color exists in the token source but has no generated Ant Design mapping
- **THEN** the verifier fails with the source token and missing mapping named

#### Scenario: A package is absent from the feed manifest

- **WHEN** the package exists on disk but its tarball or digest is absent from `feed.json`
- **THEN** feed verification fails and names the missing package or digest
