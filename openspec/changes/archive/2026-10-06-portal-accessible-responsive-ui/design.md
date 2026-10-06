# Design: portal-accessible-responsive-ui

## Implementation boundary

Repository `/home/paul/code/forge`, Rust 1.87+, Maud 0.27, existing HTML/CSS output from `src/api/ui/render.rs`. Change only shared UI document shells, CSS, semantic markup, and tests under `tests/portal_ui_contract.rs` and `tests/browser/`. No route, authorization, data, identity, database, CLI, API JSON, MCP, or sibling project changes. Keep the no-client-JavaScript model and Maud escaping.

## Existing browser/test runtime

The repository already has a pinned Playwright harness: `tests/browser/package.json`, lockfile, `tests/browser/render-check.mjs` (the React-preview smoke) and the installed Chromium. This change adds `tests/browser/portal-a11y-check.mjs` under the same pinned package — no new frontend dependency — and drives it from `tests/portal_browser_a11y.rs`. Because the portal stylesheet is inline, the harness renders the shipped page families through the real HTTP surface, writes them to a temp directory and loads them from disk; no running server is required. If `node`, the pinned `playwright` install or a Chromium engine is unavailable, the harness exits 2 and the Rust test reports `UNVERIFIED` rather than a pass. Rust code remains synchronous server-rendered HTML. Rust checks use `cargo test --test portal_ui_contract`, `cargo test --test portal_browser_a11y`, `cargo fmt --check` and `cargo build`.

## Shared page structure

Every complete portal document SHALL render `html lang="en"`, UTF-8, viewport metadata, one `<header>` with a named `<nav>`, one skip link targeting the page's unique `<main id="main-content">`, exactly one page-level `<h1>`, and one `<footer>`. All page-level headings are ordered without skipped levels. Each form control has a persistent associated `<label>`; error text is associated with its field and announced with `role="alert"` where appropriate. Tables retain `<caption>` or an accessible name, `<thead>`, scoped column headers and `<tbody>`; on narrow screens the table is placed in a labelled, keyboard-scrollable region rather than forcing page-level horizontal scrolling.

## Responsive and visual rules

- Use a small set of CSS custom properties for surface, text, muted text, border, interactive/focus, success, warning, and failure colors. Support light and dark system preference; do not encode pass/fail meaning by color alone (include text labels/icons with accessible names).
- Page content is fluid, bounded by a readable max width, and has no fixed minimum wider than 320 CSS px. At 320 CSS px and 400% zoom, content reflows into one column; only intrinsically two-dimensional tables may scroll within their own labelled region.
- Navigation wraps; metadata field rows collapse to one column; buttons and form controls have at least 24×24 CSS px hit boxes, with a preferred 44px block size for primary controls. No interactive target is clipped or obscured.
- Focus indicators are persistent and at least 3:1 against adjacent colors; focus is not covered by sticky headers. Hover is never the only affordance. Honor `prefers-reduced-motion`; avoid animation unless it conveys state.
- Text contrast: normal text at least 4.5:1; large text and non-text UI boundaries/focus indicators at least 3:1. Record measured computed pairs from browser output for light/dark and default/focus/hover states; do not claim conformance from token names or source-color inspection alone.

## Accessibility semantics and status

Keep native links/buttons/forms. Use headings/landmarks rather than ARIA substitutes. Label filter fieldsets with legends. Status and evidence labels remain explicit text (`done`, `failed`, `pending`, `unavailable`, etc.) and remain understandable with CSS disabled or grayscale. Do not put essential information in generated CSS content. Ensure tab order follows reading order and table scroll regions are keyboard reachable with a visible name.

## Contract and compatibility

Rendered text/data and route URLs remain unchanged except adding semantically necessary captions/labels and landmarks. JSON/API response bytes remain unchanged. No cookie/auth assumptions are introduced in this package. Existing Maud automatic escaping remains mandatory for all project-controlled content. The CSS remains inline and local so pages work offline and no asset hosting/CSP change is needed.

## Failure and boundary behavior

- Empty states and error pages use the same document shell, skip link, heading, focus behavior, and landmarks.
- Long project IDs, URLs, notes, and tokens wrap or scroll inside bounded code/pre/table regions; they must not enlarge the viewport.
- Missing optional fields render an explicit text fallback, not an unlabeled blank.
- Unknown browser support for newer CSS falls back to readable one-column/block layout and native controls.

## Verification oracle

1. `portal_ui_contract` (always-run markup layer) checks all page families for language, landmarks, unique main target, heading level, associated labels, named tables/status text, existing escaping, and inline CSS hooks for responsive/focus/reduced-motion behavior. `portal_browser_a11y` drives the pinned Playwright harness for the rendered-behavior layer.
2. `tests/browser/portal-a11y-check.mjs` captures the fleet, detail, portfolio, plan/result, error, and Studio pages at 320 and 1280 CSS px in light and dark colour schemes; it asserts no document-level horizontal overflow at 320, controls are visible/reachable, and no element obscures keyboard focus. Screenshots of each page family are written to the run's temp directory for clipping/overlap review.
3. Walk every page with keyboard only; verify skip link, nav, filters, mutation forms, project links, and table scroll region. Inspect the rendered DOM for named landmarks, level-one heading, form labels, table headers, and status text.
4. Measure all computed text and focus/UI contrast combinations in light and dark modes. Every sampled relevant pair passes WCAG 2.2 AA thresholds above. Automated contrast output supplements but cannot replace reflow, keyboard, or manual semantics checks.
5. Run `cargo test --test portal_ui_contract`, `cargo test --test portal_browser_a11y`, `cargo fmt --check`, `cargo build`, name preflight, strict OpenSpec validation and `git diff --check`. No external provider or network access is part of this UI package.

## Decision ledger

- Resolved: retain server-rendered Maud and inline CSS; no front-end framework or JavaScript.
- Resolved: use existing Playwright harness, pinned in repository; do not add dependencies.
- Resolved: responsive changes cover every page using the shared frames, including error and Studio pages.
- Resolved: tests measure actual rendered contrast/focus and viewport behavior; source-string tests are not conformance evidence.
- Deferred: branding-specific illustration/imagery and a broader product design system are out of scope.
