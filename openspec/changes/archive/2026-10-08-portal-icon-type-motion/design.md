# Design: Portal icon type motion

## 1. Decisions

- **Inline SVG, one stroke set, no sprite sheet (chosen).** Every icon
  is inline `<svg viewBox="0 0 24 24" fill="none"
  stroke="currentColor" stroke-linecap="round"
  stroke-linejoin="round">` with Lucide/feather-style paths, sized by
  `--icon-sm:14px / --icon-md:16px / --icon-lg:20px` and stroked by
  `--icon-stroke:1.8` (applied as a CSS `stroke-width` rule so the
  token, not a per-icon attribute, owns the weight). Inline over a
  `<symbol>` sprite: no cross-file reference to break under the static
  server's exact-allowlist, zero fetch, and each usage stays greppable
  for the contract test. Alternative (unicode glyphs kept) rejected:
  per-block metrics make weight/size/alignment uncontrollable across
  platforms. Alternative (external icon font/CDN) rejected: the portal
  ships zero external fetches by policy.
- **Icon mapping keeps meaning, not codepoints.** grid→projects,
  settings→workbench, plus→management, diamond→portfolio,
  upload→delivery, search→search, activity→evidence, check→healthy,
  alert-triangle→issues, refresh→stale, ban→unavailable,
  layers→registered-total, lock→security note, arrow→submit,
  chevron→disclosure caret. The `▸` caret becomes a chevron-right path
  so the existing `.is-open{transform:rotate(90deg)}` rule keeps its
  closed→open meaning with zero logic change.
- **Decorative-hidden, names intact (chosen).** All replaced icons sit
  beside visible text, so each SVG (and its wrapper span) carries
  `aria-hidden="true"`; every `sr-only` label, `aria-label`,
  `aria-current`, and visible text node is untouched. There are no
  icon-only controls in the portal (the password toggle is text
  "Show/Hide"), so no new accessible-name work is needed. `·` middot
  separators and `—`/`→`-in-prose (version ranges) are punctuation,
  not icons, and stay.
- **Appended slice-5 CSS block + three surgical base edits (chosen).**
  Same convention as slices 2–4: one commented block at the end of
  `frontend/styles.css`, equal-specificity later rules. Three base
  edits remove what an override cannot cleanly erase without leaving
  dead glyph text in the file: the `.story-points li:before`
  `content:"✓"` rule (replaced by real `.check-disc` markup),
  `.wb-digest` `word-break:break-all` (replaced by
  `overflow-wrap:anywhere`), and `.lock-icon` font sizing (SVG needs
  box sizing, not a 9px font). Everything else is additive override.
- **Type scale tokens mapped to existing sizes (chosen).**
  `--text-xs:10px through --text-3xl:24px` name the sizes already in
  use; the block wires body (`1rem`), panel headings, and summary
  figures to tokens at byte-identical computed values, then adds the
  genuinely new behavior: `body{line-height:1.6}` (inside 1.5–1.75),
  `font-variant-numeric:tabular-nums` on counts/ids/digests/timestamps,
  and `max-width:70ch` (inside 65–75ch) on prose selectors that lack a
  guard. Selectors that already cap narrower (`.story-description`
  370px) are left alone — a guard caps length, it never widens text.
  No color token is touched, so every 4.5:1 pair from slice 1 stands.
- **Asymmetric enter/exit from two duration tokens (chosen).**
  `--dur-enter:140ms` (stays inside the slice-2 80–150ms press-feedback
  band) and `--dur-exit:90ms` (≈64% of enter, inside the required
  60–70%); `--ease-standard` for enters, `--ease-out` available for
  exits. Base interactive rules use the enter token; `:active` press
  states drop to `transition-duration:var(--dur-exit)` so confirmation
  lands fast; the caret rotates on the enter token. Properties stay
  `opacity/background-color/border-color/transform` — paint and
  compositor only, never layout (no width/height/margin/padding
  animation, no keyframes). The existing
  `prefers-reduced-motion:reduce` guard (which already zeroes
  transition/animation durations) is kept verbatim.

## 2. Implementation boundary

- **Repository / project:** this Forge repository, vendored `frontend/`
  only. No sibling touched.
- **Files changed:**
  - `frontend/index.html` — 16 glyph sites → inline SVG (5 nav, 1
    search, 8 summary cards, 2 empty marks); labels/structure/attrs
    otherwise identical.
  - `frontend/login.html` — 3 story items gain `.check-disc` check
    SVGs, lock `●` → lock SVG, submit `→` → arrow SVG.
  - `frontend/app.js` — caret span sets chevron `innerHTML`; login
    submit restore string uses the same arrow SVG. No other JS change.
  - `frontend/styles.css` — three base edits + appended slice-5 block
    (icon/type/motion tokens and wiring).
  - `tests/forge_web_navigation_contract.rs` — one new static token
    test for slice 5.
  - OpenSpec package (`proposal.md`, `design.md`, `tasks.md`,
    `specs/portal-web-ui/spec.md` delta).
- **Modules reused unchanged:** entire router (`VIEW_BY_PATH`,
  `viewForPath`, `renderRoute`, `navigateTo`, `popstate`,
  `isRoutePath`), filter snapshot/restore, deep-link params,
  `loginNextTarget`, focus guard, scroll helper, all fetch routes and
  payload shapes, slices 1–4 tokens and behaviors.
- **Must NOT change:** JSON API, auth/session server behavior, registry
  or journal, `command_catalog` rows, static-server allowlist, typed
  payload shapes, 44px minima, labels, summaries, contrast colors,
  slice-6 surfaces.

## 3. Language and runtime

Vanilla HTML + vanilla CSS in `frontend/` (14 new custom properties;
no other new tokens), one ES2021 `innerHTML` assignment for the
caret, served by `forge web serve` and staged by `scripts/web.sh`
symlinks. No build step, no external fetch, no font load.
Verification: Rust integration oracles (`cargo test --test <name>`)
plus static token assertions over the shipped files; no new browser
harness in this slice.

## 4. User experience and interface

Actor: any operator on login or any dashboard route, including
screen-reader, keyboard-only, reduced-motion, and small-phone
operators.

| Action | Before | After |
|---|---|---|
| Scan nav/summary/empty-state icons | Mixed unicode blocks, platform-dependent weight and alignment | One stroke set, uniform 1.8 width, sm/md/lg sizes aligned to text |
| Read the login story checks | CSS `✓` glyph in a disc | Same disc, real check SVG, same green pair |
| Open a workbench action card | `▸` glyph rotates 90° | Chevron SVG rotates 90°, same motion, same meaning |
| Read counts/ids/digests | Proportional figures, digest mid-token breaks | Tabular figures, long tokens wrap sanely |
| Read long prose | Some blocks unbounded in width | Capped at 70ch |
| Hover/press a control | Bare `120ms ease` everywhere | Enter 140ms standard-ease, press exits at 90ms; motion-tolerant only |

## 5. Behavioral model

- Icons: pure markup swap. Computed layout is unchanged (SVG boxes at
  sm/md/lg match the glyph boxes they replace); the 560px rule that
  hides `.nav-link span` keeps hiding icon+text-span identically.
- Type: `html{font-size:16px}` and body `1rem` restate the existing
  computed base explicitly; `line-height:1.6` is the one intended
  vertical change. Tabular figures and `overflow-wrap:anywhere` alter
  glyph advances/wrapping only, never content. The 70ch guard only
  narrows blocks that were wider.
- Motion: hover/enter at 140ms, press/active at 90ms, caret rotate at
  140ms; reduced-motion collapses all to ~instant as before. No
  property that triggers layout is transitioned; no animation runs on
  load.
- Unknown-route, filter-state, deep-link, focus, scroll, label,
  summary, target, and breakpoint behaviors are byte-for-byte the same
  code paths as slice 4.

## 6. Contract and compatibility

No wire, storage, or CLI contract changes. Browser-only deltas: icon
markup, caret markup, submit-button markup, type/motion CSS.
Backward compatible: all existing `app.js` token assertions
(`VIEW_BY_PATH`, `renderRoute`, `history.pushState`, `popstate`,
`applyManagementProjectParam`, `applyWorkbenchProjectParam`,
`loginNextTarget`, `nothing was selected/loaded`,
`match.state === "registered"`, slice-4 filter/z/unknown tokens)
remain literally present; bare `/workbench`, `/management`,
`login.html`, and the `next` allowlist behave as before.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| SVG unsupported (ancient agent) | Labels are real text beside the icon; meaning never icon-only |
| Forced-colors / high-contrast | Icons use `currentColor`; active states keep text + `aria-current` |
| Reduced motion | Guard zeroes all durations; scroll helper already instant |
| Missing `.check-disc` styling (foreign shell) | Inline SVG still renders at 24px default; text label intact |
| Long digest/token | `overflow-wrap:anywhere` wraps without breaking layout or clipping |

## 8. Verification oracle

- `cargo fmt --check`, `cargo build` clean (no Rust touched; confirms
  no collateral).
- `cargo test --test forge_web_navigation_contract`,
  `--test forge_web_manage_deep_link_browser`,
  `--test forge_web_workbench_deep_link_browser` green (tokens +
  behavior intact).
- Static assertions by inspection over shipped files: zero remaining
  icon glyphs (`▦⚙＋◈⇪⌕⌁✔↻⌀Σ✓●▸` absent from icon sites; `→` absent
  from button markup); `svg.icon` present at all 16+5+2 sites with
  `aria-hidden`; `--icon-sm/md/lg` + `--icon-stroke` defined and
  wired; `html{font-size:16px}`, body `line-height:1.6`,
  `--text-*` scale, `tabular-nums`, `70ch`, `overflow-wrap:anywhere`,
  `break-all` gone; `--dur-enter:140ms/--dur-exit:90ms` with exit
  ≈64% of enter, named easings, no bare `120ms ease` on interactive
  rules, transform/opacity-only, reduced-motion guard kept; 44px
  minima, labels, summaries, focus/contrast, deep-link, filter-state,
  unknown, z-scale tokens intact.
- `node scripts/check-openspec-change-names.mjs` PASS;
  `openspec validate --all --strict --no-interactive` 0 failures;
  `git diff --check` clean.
- `forge gate --dry-run` then `forge gate --timeout-secs 600`; verdict
  recorded in HANDOFF with attribution.

## 9. Decision ledger

- **Resolved:** inline SVG over sprite sheet (no allowlist/fetch
  risk, greppable usages).
- **Resolved:** chevron SVG over kept `▸` (same rotation rule, set
  membership complete).
- **Resolved:** three surgical base edits over pure-append (dead glyph
  text must leave the file).
- **Resolved:** 140/90ms over kept 120ms-everywhere (enter stays in
  the 80–150ms press band; exit hits 60–70% of enter).
- **Resolved:** 70ch guard that only narrows (already-narrow prose
  untouched).
- **Resolved:** no new browser harness; automation stays the
  full-audit concern, not this slice's.
- **Deferred (non-goals):** slice 6; light theme; new modules.
- **Blockers:** none.
