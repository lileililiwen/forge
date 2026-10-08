# Proposal: Portal icon type motion (slice 5)

## Why

Prior UI/UX audit of `frontend/` (vanilla HTML/CSS/JS, no framework) found
six gap slices; slices 1–4 (`portal-focus-route-contrast`,
`portal-touch-responsive-targets`, `portal-form-error-feedback`,
`portal-layout-navigation`) are delivered and archived. This change
implements ONLY slice 5 (consistent icon set, readable type scale,
shared motion tokens). Slice 6 stays QUEUED and is not authored as an
active change by this package.

Measured defects in slice 5 scope (current `main`):

- Icons are ad-hoc unicode glyphs, each from a different block with its
  own metrics: sidebar nav (`index.html:19` — `▦ ⚙ ＋ ◈ ⇪`), search
  (`index.html:31` — `⌕`), eight summary cards (`index.html:35-44` —
  `▦ ◈ ⌁ ✔ ! ↻ ⌀ Σ`), two empty states (`▦`, `◈`), the login lock
  (`login.html:45` — `●`) and submit arrow (`login.html:43` — `→`),
  the workbench disclosure caret (`app.js:2068` — `▸`), and the login
  story check (`styles.css:9` — `.story-points li:before`,
  `content:"✓"`). Glyph rendering varies by platform font; stroke
  weight, optical size, and alignment are uncontrolled, and there is no
  size/width token for any of them.
- Typography has no declared base: neither `:root` nor `html` sets a
  `font-size` (the 16px base is browser default, not a decision), `body`
  carries no `line-height` (falls back to `normal`, ~1.2–1.4, below the
  1.5–1.75 readable band), sizes are scattered literals with no scale
  (`10/11/12/13/15/16/20/22/24/27/32/36/38/43/62px`), prose has no
  line-length guard on several selectors (`.wb-action-summary`,
  `.wb-plan-result`, `.notice`, `.auth-card>.muted`), counts/ids/
  digests render in proportional figures so columns jitter, and
  `.wb-digest` (`styles.css:80`) uses `word-break:break-all`, which
  breaks long tokens at arbitrary points instead of wrapping them.
- Motion is one-duration-everywhere: every interactive transition is a
  bare `120ms ease` (`styles.css:109-112,120`, caret `61`), with no
  shared duration/easing tokens, no enter/exit distinction, and no
  named easing. Slices 1–4 behaviors that must not regress: 44px
  targets, visible labels, error summaries, focus/contrast tokens,
  `prefers-reduced-motion` guard, responsive/unknown-route/filter-state
  handling, and all deep-link behaviors.

## What Changes

- `frontend/index.html`: the 5 sidebar glyphs, the search glyph, the 8
  summary-card glyphs, and the 2 empty-mark glyphs become inline SVGs
  from one stroke set (24 viewBox, `currentColor`, round caps/joins,
  uniform `--icon-stroke`, sizes via `--icon-sm/md/lg`). Decorative
  SVGs beside visible text keep `aria-hidden="true"`; every visible
  label and `aria-current` stays byte-identical.
- `frontend/login.html`: the 3 story-check items gain an inline check
  SVG inside a `.check-disc` span (replacing the CSS `✓`), the lock
  `●` becomes a lock SVG, the submit `→` becomes an arrow SVG.
- `frontend/app.js`: the workbench caret `▸` becomes an inline
  chevron-right SVG (the existing 90° open-rotation rule keeps working
  on the wrapper span); the login submit-button restore string uses the
  same arrow SVG as `login.html`. No router, fetch, payload, or
  deep-link logic touched.
- `frontend/styles.css`: one appended slice-5 override block plus three
  minimal base edits (`li:before` glyph rule removed, `.wb-digest`
  `break-all` replaced, `.lock-icon` sized for SVG). Block contents:
  icon size/stroke tokens + per-site sizing; explicit `html 16px`
  base, `body 1rem/1.6`, `--text-*` scale wired to body/panel/summary
  type at identical computed sizes; `tabular-nums` for counts/ids/
  digests/timestamps; `70ch` prose guard; shared `--dur-enter 140ms /
  --dur-exit 90ms` (exit ≈64% of enter) with `--ease-standard /
  --ease-out`, asymmetric press/close exits, caret on tokens,
  transform/opacity-only, reduced-motion guard kept.
- `tests/forge_web_navigation_contract.rs`: one new static token test
  covering icons, type, and motion.
- No API/registry/journal/CLI/catalog change. 44px targets, visible
  labels, error summaries, focus/contrast colors, reduced-motion
  behavior, responsive/unknown-route/filter-state/deep-link behaviors
  unchanged.

## BFS Impact Map

- **Capabilities:** `portal-icon-type-motion` (new, slice-scoped).
- **Users / flows:** every operator reading the dashboard or login
  page; screen-reader operators (decorative SVGs hidden, names kept);
  reduced-motion operators (guard kept, transform/opacity-only);
  small-phone/desktop operators (no size/target/layout change).
- **Contracts / data / persistence:** none. No JSON API, registry,
  journal, schema, catalog row/count, or CLI change. No storage change.
- **Integrations / configuration:** none. `scripts/web.sh` staging picks
  up `frontend/` unchanged. No external font/CDN fetch — system stacks
  only, inline SVG only.
- **Callers:** `el()` unchanged (caret site builds the span then sets
  `innerHTML`); login submit restore string; `renderRoute`,
  `navigateTo`, `viewForPath`, filter snapshot/restore,
  `applyWorkbenchProjectParam`, `applyManagementProjectParam`,
  `loginNextTarget`, `lastRouteView` guard, scroll helper untouched.
  CSS cascade: slice-5 block is later-equal-specificity, so it wins
  only where it intends (type/motion/digest/icon sizing); slice 1–4
  values keep their computed results except the intended type/motion
  upgrades.
- **Failure / boundary:** SVG is inline markup — no fetch to fail, no
  fallback needed; a missing icon element degrades to text-only label
  (labels are real text, never icon-only except none exist); forced
  colors: icons use `currentColor`, active states keep text plus
  `aria-current`, never color alone.
- **Tests:** `forge_web_navigation_contract` (+1 token test),
  `forge_web_manage_deep_link_browser`,
  `forge_web_workbench_deep_link_browser` must stay green; rendering
  verified by static token assertions over the shipped files (no new
  browser harness; markup-only claims are not reported as browser
  verification).
- **Privacy / security:** no new data collected, stored, or sent; no
  origin/cookie/session handling touched.

## Capabilities

- `portal-icon-type-motion`: every UI glyph is an inline SVG from one
  stroke set with uniform stroke-width and size tokens, decorative
  icons hidden from assistive technology with accessible names intact
  and no emoji anywhere; type declares a 16px base with body
  line-height in the 1.5–1.75 band, a named scale, a 65–75ch prose
  guard, tabular figures for counts/ids/timestamps, and sane long-token
  wrapping, keeping all 4.5:1 pairs; motion runs on shared
  duration/easing tokens with exits at ~60–70% of enters,
  transform/opacity-only, under the kept reduced-motion guard, with no
  layout-shift animation.

## Non-goals

- Slice 6 (data-table virtualize/sort/export/skeleton) — a separate
  future change.
- No light theme, no new modules, no framework or build step, no
  visual redesign beyond the scoped icon/type/motion tokens; desktop
  feel and all slice 1–4 contracts preserved.
- No workbench health latency or project-retire work (HANDOFF defects
  1–2 stand).
