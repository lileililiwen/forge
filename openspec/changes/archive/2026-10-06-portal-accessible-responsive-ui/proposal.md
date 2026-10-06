# Proposal: Make the Forge portal responsive and accessible

## Why

Forge's Maud portal renders fleet, project, portfolio, publish, and Studio pages through one shared frame, but its current stylesheet is a minimal generic table layout with weak responsive behavior, small controls, limited focus treatment, and status colors that are difficult to distinguish. Operators need a usable interface at mobile widths and with keyboard, screen-reader, zoom, and contrast needs.

## What Changes

- Establish a shared responsive visual system for all existing portal pages without changing route, data, identity, or publish behavior.
- Add semantic landmarks, skip navigation, logical headings, accessible form labels, table semantics/overflow handling, visible focus, non-color status cues, and reduced-motion support.
- Meet WCAG 2.2 AA requirements in scope: text contrast 4.5:1, large text/UI contrast 3:1, keyboard/focus visibility and not-obscured, reflow at 320 CSS px, text resize/zoom, and 24×24 CSS px minimum pointer targets unless a documented WCAG exception applies.
- Verify rendered pages at narrow/mobile and desktop widths, light/dark color schemes, keyboard-only navigation, accessibility tree, and actual contrast measurements.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `portal-browser-sign-in` | Secure provider-backed browser sign-in to the Forge portal. | Forge / Rust. | OIDC callback/session and portal authorization. | Existing central admin identity. | Fake OIDC issuer and session contract tests. |
| `portal-accessible-responsive-ui` | Existing pages work responsively and pass the portal accessibility baseline. | Forge / Rust + Maud HTML/CSS. | Shared portal page rendering and static assets only. | Existing `portal-web-ui`; no dependency on the auth change. | Browser render, viewport/reflow, keyboard/AX-tree, and measured contrast tests. |

The outcomes have separate failure domains and acceptance oracles; this package is visual/presentation-only. It may run before or after browser sign-in and must not modify identity or transport semantics.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge portal | `src/api/ui/render.rs`; `openspec/specs/portal-web-ui/spec.md` | Existing Maud layouts, server-rendered pages, escaping, one inline stylesheet, browser contract tests. | Current shared CSS lacks responsive/accessibility guarantees. | Forge owns this interface and release. | **keep local**; extend the existing shared renderer. |
| Devloom | `openspec/specs/browser-provider-acceptance/spec.md`; `openspec/specs/owner-settings-console/spec.md` | Browser test/accessibility evidence and server-rendered settings patterns. | ASP.NET product console and identity ownership differ; no shared CSS or Rust renderer exists. | Devloom owns its application UI. | **keep local**; import no source or design runtime. |
| Chronicleaf | Active product checkout; no Forge-compatible shared UI component or design token package was established by the targeted search. | No shared presentation contract established. | Separate application, runtime, and product lifecycle. | Chronicleaf owns its UI. | **keep local**; no sibling dependency. |
| Platform Contracts | `README.md` explicitly excludes UI ownership. | Cross-project wire schemas only. | Visual presentation is not an inter-project wire contract. | Platform Contracts owns versioned data contracts. | **keep local**. |

## BFS Impact Map

- **Pages:** fleet `/ui`; detail, portfolio edit, publish plan/result, errors, and Studio views under `/ui/...`.
- **Renderer:** `src/api/ui/render.rs` shared document head, fleet/project frames, components, tables, forms, status classes, and inline CSS. Preserve Maud default escaping.
- **Contracts:** modifies only `portal-web-ui` presentation requirements. No API, CLI, MCP, identity, persistence, or publication contract changes.
- **Users/input:** keyboard and assistive-technology users, low-vision users, mobile/narrow viewport users, zoomed text, dark/light mode, reduced-motion preference; all project text remains untrusted.
- **Verification:** rendering unit/contract tests plus the existing Playwright browser harness in `tests/browser`; measure computed foreground/background pairs rather than asserting named colors alone.
- **Compatibility/security:** zero JavaScript requirement; no new web dependency; do not hide focus, rely on color alone, introduce fixed-width content, weaken HTML escaping, or expose credential values in the UI.
- **Unaffected:** authentication flow, session/cookies, backend route behavior, project data, mutation confirmations, static publication, and all sibling project source/runtime.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `portal-web-ui`: responsive layout, semantic structure, keyboard/focus behavior, contrast, target sizing, and browser verification requirements.

## Non-goals

- Browser OIDC/session sign-in (separate `portal-browser-sign-in` package).
- Replacing server-rendered Maud with React/SPA/JavaScript, changing API envelopes, or adding new functionality to the portal.
- Reworking project information architecture or changing any existing workflow/data semantics.
