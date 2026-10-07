# Design: Human-centered dashboard language and visual system

## 1. Implementation boundary

- **Repository / project:** this Forge repository, `frontend/` plus a
  bounded set of backend message literals. No sibling touched.
- **Modules changed:**
  - `frontend/styles.css` — full re-tokenization to the dark
    command-center system (all 236 existing selectors kept, values
    replaced; required tokens `min-width:320px`,
    `@media(max-width:560px)`, `:focus-visible`, `.skip-link`,
    `@media(prefers-reduced-motion:reduce)` preserved).
  - `frontend/app.js` — fleet row rendering (name-first, one status line),
    card copy (no hashes/digests/op-ids in display; values stay in JS
    state), onboarding result auto fleet re-fetch in place of full reload,
    plain-language preview/result sentences.
  - `frontend/index.html` — copy tweaks only where headings paraphrase
    programmer concepts; no id removed/renamed, heading hierarchy kept.
  - `src/api/project_management.rs`, `src/api/workspace.rs`,
    `src/api/admin.rs`, `src/api/delivery.rs` (only if it owns surfaced
    strings this change rewrites) — message literals only; codes,
    statuses and shapes untouched.
  - `tests/forge_portal_frontend_contract.rs` — pin token set, section
    order, no-hash-rendered markers.
  - `tests/browser/workspace-onboarding-check.mjs` (+ delivery drive if
    its copy changes) — readable-copy assertions, no-hash-rendered
    assertions, contrast re-verified on the new palette.
- **Must NOT change:** routes, codes, statuses, digests, idempotency,
  journal shapes, catalog rows, Core logic, heading hierarchy, control
  ids, live regions.

## 2. Language and runtime

- Browser HTML/CSS/JS served by `forge web serve`; no bundler, no webfont
  (offline-safe system stack), no new dependency. Rust 2021 toolchain only
  for message-literal edits and the test harness.
- Verify with the focused contract/browser suites; the Playwright contrast
  checks in the existing drives are the palette oracle.

## 3. Ownership and shared code

- `frontend/` owns all display copy and the visual system. Backend owns
  message strings but not their styling; code/status enums are shared
  vocabulary and stay byte-identical except the message text.
- The design language is adapted by hand (no package to install); contrast
  is verified, not assumed.

## 4. User experience and interface

Actor: operator. Entry: dashboard. Fleet rows: display name (manifest
`name`, fallback id parenthesized once), one plain status line per row
(healthy / needs attention: <reason> / unavailable), profile shown as-is
(it is already a plain word like `rust-web`? — render as “Rust web”
mapping for the six known profiles, raw value otherwise).
Workbench/delivery/onboarding/action cards: phase and next-action
sentences only; revisions shown as 12-char prefixes where a revision is
operationally needed (promote confirm still pastes full value from the
card? — NO: the full value travels in the digest-bound body the preview
returned; the operator never hand-copies hashes. Stage/promote inputs are
pre-filled from status data instead of typed).
Preview/result copy: sentences (“Onboard the 27 selected directories”),
per-item lines (“DemoApp → demo-custom (Rust web)”), failures with cause
and next step. Digests/op-ids/hashes: never rendered; JS memory + wire
only. Errors: plain instruction first, code in muted suffix where the
contract test needs it visible? — NO code on screen; code stays in the
JSON body for tooling.
States: loading/empty/unavailable/permission/error/confirm/success/
blocked/recovery all kept, reworded. Responsive/keyboard/focus/landmarks
unchanged. Palette: near-black canvas `#08090d`, panel `#0f1219`,
ink `#f2f4f8`, muted `#9aa3b5`, accent indigo `#5e6ad2`, states
green/amber/red/violet; every text pair ≥ 4.5:1, verified by the drives.

## 5. Behavioral model

Display is a pure function of the same JSON: identical requests,
identical wire bodies, identical journals; only rendered strings change.
Auto fleet re-fetch after onboard success: `GET /v1/admin/projects`,
re-render rows + counts, keep results panel visible; failure keeps the
manual Reload path. No new request types, no new timing assumptions.

## 6. Contract and compatibility

- Wire: unchanged. Catalog: unchanged. Journal: unchanged.
- Message literals: rewritten list lives in the tasks (one task per
  surface) with old→new mapping; contract tests updated to the new
  strings where they pinned them.
- CSS: same selectors, new values; the five required tokens pinned by
  the frontend contract test.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| Unknown profile value | raw value shown, never blank |
| Missing display name | id shown once, parenthesized as technical reference |
| Fleet re-fetch fails after onboard | results stay, manual Reload offered, error notice |
| Any refused action | plain instruction + what to do next; code stays in JSON |
| Contrast below 4.5 on any sampled text | build fails the drive; adjust the token, never the threshold |

## 8. Verification oracle

- **Browser drives** (onboarding extended + delivery): readable-copy
  assertions (name-first rows, sentence previews, no 40-hex/64-hex/op-id
  text on screen), contrast ≥ 4.5 on sampled text, keyboard, no-path —
  all green, 3/3 stability runs for the touched drive.
- **Frontend source contract**: token set, section order, no-hash
  markers, endpoint list unchanged.
- **Contract suites**: management/onboarding/catalog/execution/workbench/
  admin/fleet/portal suites green with updated message pins; `--bin
  forge`, `--lib api::` green.
- A task box is checked only with the command output for its assertion.

## 9. Decision ledger

- **Resolved:** hide, don't remove — identifiers stay in memory/wire,
  display shows words. Safety properties are untouched by construction.
- **Resolved:** stage/promote inputs pre-fill from status data instead of
  hand-copied hashes; the digest still binds the exact values.
- **Resolved:** one dark system (Linear-style), no theme switcher — one
  thing done well beats two half-done.
- **Resolved:** fleet auto re-fetch in place (results stay visible)
  instead of full-page reload.
- **Deferred to `forge-workspace-sync`:** CLI mechanism (parked,
  independent).
- **Blockers:** none.
