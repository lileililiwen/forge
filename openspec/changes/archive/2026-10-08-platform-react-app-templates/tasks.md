## 1. BFS — Baseline and impact coverage

- [x] Record baseline exports, package metadata, token output, feed membership, and current verifier behavior for the React packages.
- [x] Map each requirement to its package source, token-generation rule, consumer-visible state, responsive case, accessibility assertion, and verification oracle.
- [x] Freeze the compatibility boundary: existing React package exports remain intact; `@platform/react-ui` owns base primitives; Ant Design is a peer only.
- [x] Confirm the Flutter follow-up is recorded as a separate package and is not pulled into this change's dependency or acceptance boundary.

## 2. DFS — Requirement-by-requirement implementation

- [x] Extend `ui/scripts/generate-tokens.mjs` to emit concrete light/dark semantic value maps for theme adapters from `ui/tokens/tokens.json` without hand-enumerated color values.
- [x] Extend `ui/scripts/verify-ui.mjs` to validate generated theme completeness and reject missing semantic mappings or source-unreachable literals.
- [x] Add `ui/packages/react-templates/package.json` with explicit React, Ant Design, and platform peer contracts and package files.
- [x] Add `ui/packages/react-templates/src/theme.tsx` exporting deterministic `createPlatformAntTheme(mode)` from generated semantic token values.
- [x] Add `ui/packages/react-templates/src/layouts.tsx` with separate public, user-dashboard, and admin-dashboard layouts composing `@platform/react-ui` and `@platform/react-shell` without duplicate base primitives.
- [x] Add responsive template styles in `ui/packages/react-templates/styles.css`, using only declared design tokens and documented breakpoint values.
- [x] Add `ui/packages/react-templates/src/index.ts` exports and verifier checks for landmarks, page headings, navigation, state handling, table scroll regions, and peer declarations.
- [x] Extend deterministic packing and `ui/feed/feed.json` to include the React templates tarball and matching digest/membership record.
- [x] Update `ui/README.md` with React peer installation, layout selection, branding boundaries, AI-agent composition rules, and accessibility/responsive checklist.

## 3. BFS — Cross-surface regression and completeness

- [x] Confirm existing `@platform/react-ui`, `@platform/react-shell`, and `@platform/react-auth` package contracts remain unchanged and no second platform Button/Field/Card/Badge/State is exported.
- [x] Confirm token generation, theme mapping, CSS, verifier, package metadata, and feed manifest all point to the same declared semantic values and package version.
- [x] Confirm public, user, and admin journeys each expose the required landmarks, navigation, focus target, loading/empty/error/forbidden behavior, and recovery affordances.
- [x] Confirm narrow layout has no page-level overflow at 320, 375, and 768 CSS pixels; wide data remains scrollable inside its labeled table region.
- [x] Review keyboard and screen-reader behavior, reduced motion, light/dark contrast, and chart alternatives in a runnable consumer fixture.
- [x] Confirm no unrelated .NET, Razor, Flutter, consumer, or untracked files were changed.

## 4. Verification

- [x] Run the focused token generator and `node ui/scripts/verify-ui.mjs`; retain output showing all generated theme mappings resolve to source tokens.
- [x] Run `node ui/scripts/pack-ui.mjs` twice and verify identical package digests and feed membership.
- [x] Run focused React template/package checks and the repository's applicable quality gate after explicit test/verification authorization.
- [x] Install the vendored platform packages in a temporary consumer at an unrelated path and resolve the documented Ant Design peer.
- [x] Render all three layouts in a browser at 320, 375, 768, 1024, and 1280 CSS-pixel widths; record keyboard, accessibility-tree, reduced-motion, and overflow evidence.
- [x] Run `openspec validate platform-react-app-templates --strict --no-interactive` and `git diff --check`.
- [x] Update task state and `HANDOFF.md` only for completed evidence; do not archive or claim completion while browser/accessibility verification remains open.
