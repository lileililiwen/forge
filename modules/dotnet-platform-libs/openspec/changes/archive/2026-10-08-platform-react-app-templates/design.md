# Design: reusable React application templates

## 1. Implementation boundary

Implement only under `ui/` in `dotnet-platform-libs`. Add `ui/packages/react-templates` as the single new React package with public exports for a theme adapter, public content layout, signed-in user dashboard layout, admin dashboard layout, and documented data-state composition. Extend `ui/scripts/generate-tokens.mjs`, `ui/scripts/verify-ui.mjs`, `ui/scripts/pack-ui.mjs`, and `ui/feed/feed.json` only where needed to emit, verify, and ship the new artifact. Update `ui/README.md` with install/adoption and AI-agent rules. Do not change `src/Platform.UI.Razor`, any .NET package, consumer repository, or Flutter source.

## 2. Language and runtime

Use TypeScript/TSX with React `>=18`. Declare `react`, `antd` (supported major range `>=5 <7`), and `@platform/react-ui`/`@platform/react-shell` as peer dependencies where required; use the platform feed for platform packages and do not embed vendor source. The repository's UI artifacts are currently source-distributed `.tgz` packages and verified with Node ESM scripts; do not add a build system or package registry. Consumers compile the TSX through their own React toolchain. Browser-level evidence must use a runnable consumer fixture with its own React build setup, not claim that the source-contract harness rendered JSX.

## 3. Ownership and shared code

`ui/tokens/tokens.json` remains the only value source. `ui/packages/react-ui` remains the only platform implementation of overlapping base primitives and `ui/packages/react-shell` remains the navigation/auth/permission boundary. The new package composes these exports and Ant Design for advanced tables, filters, and layout primitives that are not already represented by platform components. It must not re-export duplicate Button, Field, Card, Badge, or State components. Ant Design is a peer dependency, not copied into the platform feed; consumer `package.json` must explicitly install a supported peer version. Feed metadata pins the platform tarball bytes/digest only.

Generate `tokenValuesByTheme` from the DTCG tree for `light` and `dark`, preserving source aliases and avoiding hard-coded hex values. Export a `createPlatformAntTheme(mode)` function that maps semantic values to Ant Design's global token names and a small set of component tokens. Theme values must be concrete generated colors, not CSS `var()` strings, because Ant Design's token algorithms may derive colors. Shared CSS tokens still style platform primitives. The exported theme object is deterministic and has no runtime environment or DOM dependency.

## 4. User experience and interface

Expose three compositions from `@platform/react-templates`:

- `PublicPageLayout`: product identity, content navigation, main landmark, and footer; no authenticated shell requirement.
- `UserDashboardLayout`: product identity, user navigation, current page heading, account actions, and main landmark.
- `AdminDashboardLayout`: admin navigation, current page heading, breadcrumb/section context, and main landmark; permission filtering is presentation only and never authorization.

Each layout receives explicit children, title, navigation, current route, and optional branding props; admin/user layouts receive a typed platform shell user object. Keep actions/content in slots rather than inventing product data. Templates are responsive: at narrow widths navigation collapses to a reachable menu or wraps into a labeled scroll region; content columns stack; controls fill available width; tables alone may scroll horizontally in an explicitly labeled region. Use a skip link, focus target for route changes, semantic landmarks/headings, `aria-current`, and stable focus after route changes. Loading, empty, error, forbidden, and success feedback use existing platform state conventions and caller-provided plain-language action text. Reduced motion uses the existing token/media conventions.

Use Ant Design only for advanced data entry/display/layout where it provides a real gap (for example Table, DatePicker, Select, and Grid); wrap those uses in named platform patterns with accessible names and labels. Agent docs require existing platform primitives for base controls. Do not use decorative charts without a text summary and equivalent table/download action. Do not add hover-only instructions, icon-only actions, arbitrary gradients, arbitrary token values, or nested cards as default composition.

## 5. Behavioral model

Templates are stateless composition components. Callers own routes, data fetching, mutations, authorization, persistence, and state transitions. The layout filters navigation only when a permission label is supplied, and never treats hidden navigation as a security decision. The page-data state order is caller-owned and mutually exclusive: `loading` → `success` with nonempty or empty data, or `error`; `forbidden` is an authorization response rendered by the caller. Retry buttons invoke caller callbacks and are absent when no recovery callback exists.

## 6. Contract and compatibility

Export only typed React props and theme helpers from `ui/packages/react-templates/src/index.ts`. Use explicit `href` navigation with optional `onNavigate`, retaining normal anchor behavior when no handler is supplied. `package.json` declares React and Ant Design peer ranges and exact platform feed dependencies; no provider or router is required. Additive package addition keeps existing package names/APIs and the existing 0.1.0 kit version unless the current feed contract requires a version increment for changed package bytes; in that case increment the kit/package version and regenerate all digests consistently. Do not silently mutate an existing versioned tarball's bytes.

## 7. Failure and boundary policy

Missing children or required labels are compile-time/API errors. Empty navigation is valid. Missing optional branding falls back to plain text identity; no remote asset is fetched. Missing permission metadata leaves navigation visible and does not grant backend access. Error states must not expose exception details; callers provide safe copy. Ant Design peer absence is a package installation/build error with a documented diagnostic; there is no runtime fallback to a second UI framework. Unsupported theme mode is a TypeScript error.

## 8. Verification oracle

Before checking off tasks, require: generated light/dark values exactly map to declared DTCG semantic values; the verifier rejects undeclared color literals and duplicate base primitives; React package metadata and feed manifest agree; two packs produce byte-identical tarballs; static contract checks prove each template's landmarks, headings, focus/skip navigation, state slots, responsive hooks, and no forbidden duplicated primitives; a temporary consumer resolves the package and its Ant Design peer at an unrelated repository path. A runnable consumer must be rendered at 320, 375, 768, 1024, and 1280 CSS-pixel widths and reviewed for page overflow, table-only scroll, keyboard operation, reduced motion, semantic tree, contrast, and chart alternatives. This work order does not authorize running tests without the user's request; record commands and evidence in HANDOFF after execution is authorized.

## 9. Decision ledger

| Decision | Resolution |
|---|---|
| React design system | Ant Design peer for advanced controls; platform primitives remain the only base primitive layer |
| Flutter | Separate Dart/Material 3 follow-up because it has a different runtime, artifact, and acceptance oracle |
| Token mapping | Generate concrete theme maps from the single DTCG source for both light and dark modes |
| Public vs dashboard pages | Three explicit layouts with distinct navigation and information density; no universal admin shell |
| Distribution | Existing deterministic vendored feed, no new registry or credential |
| Consumer migrations | Excluded; platform sample/consumer fixture is the adoption proof |
| Browser verification | Required for completion; not claimed from source checks alone |
