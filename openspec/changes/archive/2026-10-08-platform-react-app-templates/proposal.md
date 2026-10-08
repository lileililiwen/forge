# Proposal: reusable React application templates

## Why

AI-generated product UIs vary widely in layout, visual hierarchy, and accessibility. The platform already ships design tokens, accessible React primitives, a shell, auth forms, and a portable feed, but it does not give agents product-level page patterns or a curated React theme. Adding those patterns at the shared owner gives consumers a dependable starting point without letting each agent invent a design system.

## What Changes

- Add a React template package for public content pages, normal-user dashboards, and administration dashboards.
- Integrate Ant Design for complex data and enterprise controls while retaining `@platform/react-ui` as the sole source for overlapping platform primitives (button, field, card, badge, and feedback state).
- Map the existing DTCG semantic tokens into an Ant Design `ConfigProvider` theme for light and dark modes.
- Add an AI usage guide and deterministic conformance checks for approved components, token use, responsive layout, and accessibility contracts.
- Distribute the new package through the existing deterministic UI feed. Ant Design remains a declared peer dependency resolved by consumers from their package source.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `platform-react-app-templates` (this change) | React consumers can compose consistent public, user-dashboard, and admin screens from a platform theme and templates | `dotnet-platform-libs/ui`, TypeScript/React | `@platform/react-templates`, exported layouts/theme, versioned UI feed | Existing `@platform/design-tokens`, `@platform/react-ui`, `@platform/react-shell`; Ant Design peer | Template contract checks, theme-token checks, deterministic feed check, and rendered sample browser/accessibility review |
| `platform-flutter-ui-foundation` (follow-up) | Flutter consumers can apply equivalent semantic tokens through native Material 3 themes | `dotnet-platform-libs/ui`, Dart/Flutter | Generated Dart token/theme artifact and Flutter sample | Existing `ui/tokens/tokens.json`; independent Flutter toolchain | Dart generation determinism, Flutter build/widget checks, and device accessibility review |

The outcomes are independently verifiable and target different runtimes and artifact formats, so they are separate OpenSpec changes. The React package is first because the platform already ships React token and component artifacts; the Flutter change follows without making React depend on Dart. No consumer migration is bundled.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Existing platform UI | `ui/tokens/tokens.json`, `ui/packages/react-ui`, `ui/packages/react-shell`, `ui/packages/react-auth`, `ui/feed` | Semantic tokens, accessible primitives, shell/auth contracts, deterministic vendored tarballs | No product-level layouts or Ant Design mapping; current primitives are deliberately small | `dotnet-platform-libs/ui`; committed versioned feed | **extend shared owner** |
| Existing platform UI capability | `openspec/specs/platform-ui-design-system/spec.md` | Cross-client token and interaction contract; React and Razor boundaries | Does not define React app templates or Flutter delivery | Same shared owner; additive package contract | **extend shared owner** |
| Workspace Flutter consumers | `ui/legacy-divergent.json` and `openspec/changes/archive/2026-10-05-ui-shared-layer-reachability/proposal.md` | Existing list of Flutter consumers and explicit `tokens: none` baseline | No Dart output exists today | Follow-up remains owned by `dotnet-platform-libs/ui`; each app adopts separately | **extend shared owner** |
| Workspace governance | `/home/paul/code/workspace-governance/ui-adoption-migration.json` | Adoption boundary and consumer migration guidance | Does not own UI artifacts or React runtime | Governance owns portfolio policy, not these package bytes | **keep local** |

The shared design system belongs in `dotnet-platform-libs`; no sibling copy is needed. The package does not edit consumers, Razor assets, or workspace governance.

## User Experience and Interface Impact

The direct actor is a product developer or AI coding agent creating a screen; the end users are public visitors, signed-in users, and administrators. Developers select a template and supply product-owned content, routes, navigation, permissions, and brand configuration. Public pages prioritize content and a clear primary action; user dashboards prioritize current account tasks and status; admin dashboards prioritize navigation, filters, data tables, and safe operations. All templates include explicit loading, empty, failure, and permission-denied states where data is asynchronous or protected.

Templates reflow at 320 CSS pixels with no page-level two-dimensional scrolling. Wide data tables may use a labeled, keyboard-reachable scroll region. Keyboard focus remains visible and unobscured, active navigation is announced, controls have accessible names, status is not color-only, reduced motion is respected, and charts require text and data alternatives. Browser evidence covers 320, 375, 768, 1024, and 1280 CSS-pixel viewports, keyboard-only navigation, screen-reader semantics, light/dark themes, and reduced-motion preference.

## BFS Impact Map

| Requirement | Actor/journey and state | Implementation surface | Verification evidence |
|---|---|---|---|
| One platform design contract | Agent chooses a light/dark theme; existing and advanced controls stay visually aligned | `ui/tokens/tokens.json`, generated `tokens.ts`, `react-templates/src/theme.tsx` | Every semantic source value maps into both generated theme modes; no package-owned palette literal |
| Distinct public/user/admin hierarchy | Visitor, signed-in user, or administrator enters the appropriate route | `react-templates/src/layouts.tsx`, existing `react-shell` | Each layout exposes its correct navigation, landmarks, heading, and shell user boundary |
| Responsive layouts | All actors navigate and use controls across phone/tablet/desktop | `react-templates/styles.css`, shell responsive CSS, Ant Design table wrapper | Browser widths 320/375/768/1024/1280; no page-level overflow and table-only horizontal scroll |
| Accessible interaction states | Keyboard/screen-reader user navigates and recovers from loading/empty/error/forbidden states | Existing `react-ui` primitives plus template state composition | Keyboard and accessibility-tree review; required text cues, labels, visible focus, and retry callback behavior |
| Chart alternatives | User reviews any consumer-provided chart | Consumer chart/content slot; no chart library added | Consumer review requires summary and exact data view before adoption |
| Versioned artifact | Consumer vendors the platform package and installs its Ant Design peer | `react-templates/package.json`, packer, feed manifest | Feed verifier confirms package bytes/digest/members and explicit peer metadata |
| Agent guide and quality checks | Agent creates a new public, user, or admin route | `ui/README.md`, `ui/scripts/verify-ui.mjs` | Verifier checks theme completeness, package peers, named layouts, base-primitive ownership, and accessible structure |

| Surface | Impact |
|---|---|
| People and workflows | Product developers/agents select public, user, or admin templates; end users read, navigate, submit forms, filter data, and recover from failures |
| Contracts/data | Adds React exports and a theme adapter; no network or domain data contract changes |
| Packages and integrations | Adds a TypeScript package with React and Ant Design peers; consumes existing token, UI, and shell packages; adds one deterministic feed tarball |
| Configuration | Product-owned branding, theme mode, nav entries, and optional permission labels are explicit props/config; no environment-variable inference |
| Failures and boundaries | Missing required props fail through TypeScript; denied admin routes remain backend-protected; loading/error/empty states require explicit caller data and recovery action |
| Verification | Static source conformance, generated theme/token parity, packaging determinism, sample rendering, viewport and keyboard/accessibility review |
| Compatibility and privacy | Existing React primitives and packages remain compatible; no telemetry, storage, authentication provider, or product data is introduced |
| Sibling scope | Extend the shared UI owner; no sibling projects are modified. Flutter is a separate follow-up artifact. |

## Capabilities

- `platform-react-app-templates`

## Non-goals

- No Flutter/Dart artifact in this React change; it is a separate follow-up OpenSpec package.
- No replacement of existing `@platform/react-ui` primitives and no parallel platform-owned button/field/card implementation.
- No React Router, application state manager, charting library, backend, authentication provider, or product-specific schema.
- No marketing-site content, brand design exercise, mass consumer migration, npm publication, or live release workflow change.
- No claim that framework defaults alone establish WCAG conformance; consumer content and rendered flows still require review.
