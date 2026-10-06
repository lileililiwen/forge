# Forge handoff

current_spec: forge-web-portfolio-controls

## Current state

`forge-web-project-workbench` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-project-workbench`, promoting
four `forge-web-project-workbench` requirements into a new canonical spec. The
workbench deep-dives a single managed project from the dashboard: session-gated
detail, side-effect-free plan and confirmed apply, where every triggered
workflow goes through the crate's own typed in-process functions or a strict
fixed allowlist of design-scoped command ids — never `sh -c`, a generic shell
or interpolated user-controlled argv — and anything unsafe or not yet web
available renders the catalog's honest disposition instead of a fake
execution. The next eligible change from `openspec list` and the roadmap is
`forge-web-portfolio-controls`.

Before that, `forge-web-command-catalog` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-command-catalog`, promoting three
`forge-web-command-catalog` requirements into a new canonical spec. Every
top-level and nested Rust CLI command (225 rows, including `help`) is now
discoverable through the authenticated `GET /v1/admin/commands` JSON endpoint
and a searchable Commands section of the standalone frontend, each mapped to a
truthful availability state (`web`, `cli_only`, `provider_required`,
`project_capability_required`, `not_yet_web`) with plain-language guidance; no
shell/eval route exists anywhere and Clap-tree parity is a hard test failure on
drift.

Before that, `forge-web-project-fleet` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-project-fleet`, promoting three
`forge-web-project-fleet` requirements into a new canonical spec. The dashboard's
`GET /v1/admin/projects` now returns a normalized fleet envelope that combines the
local registry, an explicitly selected portable inventory source and the workspace
fleet observer, and always includes a guaranteed Forge-self record; the standalone
`frontend/` renders the sources and per-project rows with honest
healthy-empty/stale/malformed/unavailable/conflict states and no fake actions.

Before that, `forge-global-admin-portal` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-global-admin-portal`, promoting four
`forge-admin-login` requirements (new canonical spec) and four `portal-web-ui`
requirements.

The delivered portal gives one Forge-wide administrator email/password login and
an all-project dashboard, independent of each project's `forge.yaml` `identity:`
block. Browser HTML/CSS/JS live under `frontend/` and are served by the
standalone Rust `forge web serve` listener; the separate `forge api serve`
listener serves admin auth and the fleet as JSON only. The pointer above names
the next active change.

`scaffold-prewires-shared-layer` is implemented, verified and archived. Its
requirements were promoted into
[openspec/specs/scaffold-prewires-shared-layer/spec.md](openspec/specs/scaffold-prewires-shared-layer/spec.md).

At the prior archive checkpoint, no active changes remained. The three portal
packages authored after the queue closed were implemented and archived. The
seven earlier in-flight packages
were consolidated into `runtime-hardening-and-contract-closure` and archived on
2026-10-05 as
`openspec/changes/archive/2026-10-05-runtime-hardening-and-contract-closure`,
promoting 11 requirements and modifying 1 across five canonical specs. At that
archive checkpoint, `openspec list` reported none active and the repository held
71 archived changes and 63 canonical specs; the portal packages below were
authored afterward.

### forge-web-command-catalog delivered and archived (2026-10-06)

`forge-web-command-catalog` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-command-catalog`, promoting three
`forge-web-command-catalog` requirements into a new canonical spec (3 added, 0
modified). The complete CLI surface is now browser-discoverable as static
metadata:

- `src/api/command_catalog.rs` (new): typed rows
  `{id,parent_id,label,summary,category,scope,risk,availability,route,cli_invocation,reason,capabilities}`
  — 225 rows, one per Clap path (47 top-level + 138 second-level + 39
  third-level) plus the explicit top-level `help` row clap only generates at
  build time. IDs are Clap dot paths (`project.github.observe`); categories
  are the nine derived groups; availability/risk/scope are closed vocabularies
  enforced by `problems()`. Contract is versioned separately as
  `forge-command-catalog/0.1.0`. Web availability is evidence-based: exactly
  four rows (`list`, `fleet.list`, `fleet.status`, `inventory.show`) point at
  the implemented `GET /v1/admin/projects`; every other row carries a
  plain-language reason and next step (transports, native build/test, hidden
  TTY input, local git, provider credentials and manual OIDC callbacks stay
  `cli_only`/`provider_required`/`project_capability_required`; planned web
  workflows are honestly `not_yet_web` tracked gaps, never "supported").
  `disabled` is kept in the vocabulary but unused today.
- `src/api/mod.rs` + `src/api/admin.rs`: `Route::AdminCommands` behind the
  same global-session cookie gate and exact-origin CORS as the other admin
  routes; `GET /v1/admin/commands` returns the envelope or 401/403/503 JSON.
  POST → 405 via the existing alt-method rule. No shell/eval route exists or
  is added anywhere; the catalog is descriptive and authorization stays with
  each invoked operation.
- `src/main.rs`: parity oracle `command_catalog_tests` walks the real
  `Cli::command()` tree (plus `help`) and fails if the catalog and the CLI
  ever drift — the enumeration cannot go stale silently.
- `frontend/index.html`, `app.js`, `styles.css`: a Commands section with
  literal search (search terms only — never shell input), category and state
  filters, availability/risk badges rendered via `textContent`, the CLI
  invocation shown as inert `<code>` guidance, an empty-result state and an
  honest "Command catalog unavailable" state. No new files; the `forge web
  serve` allowlist is unchanged.

Evidence at archive (commands run, real output):

| Check | Result |
|---|---|
| `cargo build` | clean (0 errors; only the same three pre-existing warnings — none added) |
| `cargo fmt --check` | clean |
| `cargo test --bin forge command_catalog_tests` | **2 passed / 0 failed** (full Clap-tree set equality; `problems()` empty) |
| `cargo test --lib api::command_catalog` | **4 passed / 0 failed** (integrity, 225-row count, web-route allowlist, id/parent shape) |
| `cargo test --test forge_web_command_catalog_contract` | **8 passed / 0 failed** (anon 401 without data leak; hostile origin 403 with valid session; authenticated 200 envelope: unique ids, valid parents, reason on every non-web row, vocabulary closure, all 48 top-level names incl. `help`, nested spot checks, exactly 4 web rows; POST → 405; seven exec/shell/run-style paths → 404; shell-metacharacter query never echoed or applied; real-binary `forge --help`/`forge project --help`/`forge portfolio --help` name parity; frontend source contract: catalog fetch, textContent labels, no `eval(`, no markup in Rust) |
| `cargo test --lib` (full) | **1204 passed / 0 failed / 1 ignored** (`generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain` ran and passed here, not skipped) |
| `cargo test --test forge_admin_api_contract` / `forge_web_fleet_contract` / `identity_contract` / `portal_ui_contract` / `forge_portal_frontend_contract` | **4 / 11 / 19 / 39 / 5 passed, 0 failed** (compatibility intact) |
| `cargo test --test api_contract` | **11 passed / 0 failed** on rerun (first run: 10/11 — `healthz` failed to bind an ephemeral loopback port, the documented sandbox flake, not a regression) |
| live loopback round-trip | real `api serve` (18777) + `web serve` (18778) with a PTY-seeded admin: anonymous `GET /v1/admin/commands` → `401`; login → `200`; catalog → `200` with **225 rows / 9 categories / 4 web rows / reason on every non-web row**; POST → `405`; `/v1/admin/exec` → `404`; `/index.html` → `200`; smoke servers stopped afterwards; the pre-existing `api serve` on 8765 was left untouched |
| `node scripts/check-openspec-change-names.mjs` | PASS (before and after archive) |
| `openspec validate --all --strict --no-interactive` | **69 passed / 0 failed** (before and after archive) |
| `git diff --check` | PASS |

Honest scope of verification: catalog DOM rendering, filtering and the empty
state are pinned by the frontend source contract and the live HTTP smoke, but
were **not** exercised in an interactive headless browser here; that visual
pass remains for the operator. Pre-existing conditions re-captured during this
delivery (none added by it): `cargo clippy --all-targets -- -D warnings` still
fails only on untouched files — `src/gate/evidence.rs` (5),
`src/publish/fleet.rs` (2), `src/publish/mod.rs` (2), `src/api/ui/auth.rs` (2),
`src/portfolio/share/validation.rs` (1) and `src/api/fleet.rs` (2) — with
`command_catalog.rs` and its contract tests clippy-clean;
`tests/artifact_baseline_contract.rs::reported_manifest_changelog_versions_agree`
still fails pre-existing (5 passed / 1 failed). Per-user role filtering is
deferred (single global administrator).

### forge-web-project-fleet delivered and archived (2026-10-06)

`forge-web-project-fleet` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-web-project-fleet`, promoting three
`forge-web-project-fleet` requirements into a new canonical spec (3 added, 0
modified). The dashboard's authenticated JSON endpoint now composes one normalized
fleet from several sources and always includes Forge itself:

- `src/api/fleet.rs` (new, private, JSON-only, no markup): resolves the local
  registry, an explicitly selected portable inventory source and the workspace
  fleet observer through bounded read-only adapters, then merges them with
  provenance. A guaranteed Forge-self record is always present at index 0; a
  registry row carrying the self identity is merged into that single record
  (`has_registry_ref`) rather than duplicated, so Forge appears exactly once.
  Each row keeps its legacy keys (`id/profile/state/lifecycle/confidence/tags/
  evidence/updated_at`) plus normalized fields, and mutates only Forge-managed
  rows (capability labels). Cross-source identity collisions are retained and
  conflict-marked with capabilities cleared — never silently dropped.
- Provenance is exposed per source as a `SourceDescriptor` (`id/kind/status/
  count/malformed/reason/observed_at/provider`) that never carries an absolute
  filesystem path. Adapter failures emit a fixed safe reason; the path-bearing
  error text and `report.source`/`snapshot.source` are never serialized. Healthy-
  empty, fresh, stale, unconfigured, unavailable, malformed (named, not dropped)
  and conflict states are all preserved.
- `src/api/admin.rs`: `GET /v1/admin/projects` delegates to `fleet::load(db)` after
  the session check, returning the envelope or a neutral unavailable response;
  auth, CORS and the no-markup portal contract are unchanged. `src/api/mod.rs`
  adds `mod fleet;`.
- `frontend/index.html`, `app.js`, `styles.css`: a source panel (`role="status"
  aria-live="polite"`) plus a source filter, and a seven-column project table
  (Project/Source/Profile/Latest state/Evidence/Lifecycle/Access). Everything is
  rendered via `textContent` only; capabilities show as labels with no links, so
  there are no fake actions. Deep project navigation is intentionally deferred to
  the future `forge-web-project-workbench`.

Evidence at archive (commands run, real output):

| Check | Result |
|---|---|
| `cargo build` | clean (0 errors; only pre-existing warnings: `ShareSurface` in `src/portfolio/share/validation.rs`, `journal` field / `Published` variant in `src/main.rs`, none added by this change) |
| `cargo fmt --check` | clean |
| `cargo test --lib api::fleet` | **8 passed / 0 failed** (self merge, conflict detection, source classification, summary counts, ordering) |
| `cargo test --test forge_web_fleet_contract` | **11 passed / 0 failed** (anon 401 no data; self on empty registry; self merges registered same-id; managed registry rows distinct; inventory + workspace observed rows; stale inventory; malformed named not dropped; configured-but-unreadable → unavailable safe reason with no path; cross-source conflict retains row and disables links; no absolute path serialized with all sources active) |
| `cargo test --test forge_admin_api_contract` | **4 passed / 0 failed** (empty-registry assertion updated to expect the single self row with `registered:0`, `self_present:true`) |
| `cargo test --test forge_portal_frontend_contract` | **5 passed / 0 failed** |
| `cargo test --test api_contract` / `identity_contract` / `portal_ui_contract` | **11 / 19 / 39 passed, 0 failed** (compatibility intact) |
| live loopback round-trip | real `api serve` (18777) + `web serve` (18778) with the exact frontend origin: `/healthz` 200; anonymous `GET /v1/admin/projects` → `401 api-unauthorized`; hostile origin → `403`; `/index.html` 200; `/app.js` 200 `text/javascript; charset=utf-8`; path traversal → `404` |
| `node scripts/check-openspec-change-names.mjs` | PASS (before and after archive) |
| `openspec validate forge-web-project-fleet --strict` | valid (before archive) |
| `openspec validate --all --strict --no-interactive` | **69 passed / 0 failed** (before and after archive) |
| `git diff --check` | PASS |

Honest scope of verification: the source-panel and project-row DOM rendering and
the search/source-filter re-render are covered by the standalone frontend source
contract and were exercised at the HTTP/JSON level against the running listeners,
but were **not** rendered in an interactive headless browser here; that visual pass
remains for the operator. Source selection stays strictly env-driven
(`FORGE_INVENTORY_SOURCE`, `FORGE_WORKSPACE_REGISTRY`, plus `FORGE_SELF_ID` and
`FORGE_FLEET_MAX_AGE_SECONDS` overrides) — no sibling-directory scanning, no
absolute path leakage, no mutation of unmanaged rows.

### portal-browser-sign-in delivered and archived (2026-10-06)

`portal-browser-sign-in` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-portal-browser-sign-in`, promoting one
`central-admin-identity` requirement and four `portal-web-ui` requirements into
the canonical specs. The browser round trip runs through the `openidconnect`
crate's discovery, PKCE exchange and ID-token verification behind a
`BrowserAuthVerifier` trait (`LibraryBrowserAuthVerifier` in production, a
fake verifier in tests). The pin moved from the drafted `3.5.0` to `4.0.1`
because the 3.x `reqwest` 0.11 / `rustls` 0.21 stack carries open
`rustls-webpki`/`h2` advisories; the unavoidable `rsa` timing advisory is a
reasoned exception in `deny.toml` and `.cargo/audit.toml`.

Evidence at archive: `cargo test --test portal_ui_contract` (31 passed),
`cargo test --test identity_contract` (19 passed), `cargo build`,
`cargo fmt --check`, `cargo deny check` (all four policies ok), `cargo audit`,
`openspec validate --all --strict --no-interactive`,
`node scripts/check-openspec-change-names.mjs` and `git diff --check`.

Pre-existing conditions, not regressions (all present on `HEAD` before this
change, in files it does not touch):

- `cargo clippy --workspace --all-targets --all-features -- -D warnings` fails
  under the host's newer clippy on `src/gate/evidence.rs`,
  `src/publish/fleet.rs`, `src/publish/mod.rs`, `src/api/ui/auth.rs` and
  `src/portfolio/share/validation.rs`. Every file this change touches is clean.
- `tests/artifact_baseline_contract.rs::reported_manifest_changelog_versions_agree`
  fails because `c2ef70b` opened `CHANGELOG.md` with `## [Unreleased]` while the
  test asserts the newest heading equals the `Cargo.toml` version (`0.1.0`). The
  test reads only `CHANGELOG.md` (unmodified) and the `Cargo.toml` version, so the
  outcome is independent of this change. Not repaired here: the fix belongs with
  the changelog convention, not this package.
- Live-provider OIDC acceptance is unavailable in this environment.
  Deterministic coverage sits at the `BrowserAuthVerifier` seam plus the real
  verifier's pre-network refusals (provider error, empty code, expired
  challenge).

### portal-accessible-responsive-ui delivered and archived (2026-10-06)

`portal-accessible-responsive-ui` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-portal-accessible-responsive-ui`,
promoting four `portal-web-ui` requirements. Every portal page now renders one
accessible document shell (`chrome()`): language, skip link, one named
`<header>`/`<nav>`, exactly one `<main id="main-content">`, one page-level
`<h1>` and one `<footer>`. The inline stylesheet is token-driven with a
light/dark preference, reduced-motion support, a visible high-contrast focus
ring, minimum pointer targets and 320px reflow; data tables are captioned,
scoped and wrapped in named keyboard-reachable scroll regions. Only
`src/api/ui/render.rs` changed. No client JavaScript, external asset or new
dependency was added.

Evidence at archive: `cargo test --test portal_ui_contract` (39 passed) and
`cargo test --test portal_browser_a11y` (1 passed) — the latter drives
`tests/browser/portal-a11y-check.mjs` over the shipped page families at
320/375/640/768/1280 CSS px in light and dark and reported
`portal-a11y-check: ok (7 pages, light+dark, 5 viewports, 7 screenshots)`;
`cargo fmt --check`, `cargo build`, `openspec validate --all --strict`,
`node scripts/check-openspec-change-names.mjs` and `git diff --check` all pass.
The full workspace run has no new failure (only the pre-existing
`reported_manifest_changelog_versions_agree` below). Clippy is clean for every
file this change touches; the same pre-existing lints remain elsewhere. The
browser harness requires `node`, the pinned `tests/browser/node_modules`
install and a Chromium engine; when any is absent it reports `UNVERIFIED` and
the markup contract layer remains the always-run evidence.

### portal-login-entry-flow delivered and archived (2026-10-06)

`portal-login-entry-flow` is implemented and archived as
`openspec/changes/archive/2026-10-06-portal-login-entry-flow`, promoting the
unauthenticated portal-entry behavior into `portal-web-ui`. Anonymous HTML
requests to `/ui` now redirect to `/ui/sign-in`; that route renders a required
project-id form and continues through the existing project-scoped OIDC flow.
The form does not enumerate the registry or accept credentials.

Evidence: `cargo test --test portal_ui_contract` (39 passed),
`cargo test --test identity_contract` (19 passed),
`cargo test --test api_contract` (11 passed; run outside the sandbox because
the suite binds ephemeral loopback ports), `cargo fmt --check`, `cargo build`
(0 errors; three pre-existing warnings), `openspec validate --all --strict
--no-interactive` (64 passed), `node scripts/check-openspec-change-names.mjs`,
and `git diff --check`.

### forge-global-admin-portal delivered and archived (2026-10-06)

`forge-global-admin-portal` is implemented, verified and archived as
`openspec/changes/archive/2026-10-06-forge-global-admin-portal`, promoting four
`forge-admin-login` requirements into a new canonical spec and four
`portal-web-ui` requirements into the existing one (8 added, 0 modified). One
Forge-wide administrator account and browser session now authenticate the fleet
independently of per-project OIDC:

- `src/identity/global.rs`: Argon2id PHC hashes, opaque 256-bit CSPRNG tokens,
  12-hour bounded sessions persisted only as SHA-256 digests, plus revoke/expiry.
- `src/api/admin.rs` and the `Route::Admin*` routing in `src/api/mod.rs`: JSON
  `GET/POST/DELETE /v1/admin/session` and authenticated `GET /v1/admin/projects`,
  with exact-`frontend_origin` CORS and Origin checks. The session cookie is
  `forge_admin_session` (distinct from project `forge_session`), `HttpOnly;
  SameSite=Lax; Path=/`, `Secure` only for an `https` frontend origin. Existing
  `/v1` bearer and project OIDC behavior is unchanged.
- `src/web.rs` + `forge web serve`: Rust static listener serving only the
  allowlisted `frontend/` assets (no directory traversal, GET/HEAD only).
- `forge identity setup --email`: interactive hidden-password CLI (libc termios),
  refusing non-TTY, weak password (<12 chars), bad email and duplicate setup.
- `frontend/`: standalone `login.html`, `index.html`, `styles.css`, `config.js`,
  `app.js` and a README documenting the separate API and web preview commands. No
  page markup, style or script was added to Rust.

Evidence at archive (commands run, real output):

| Check | Result |
|---|---|
| `cargo build` | clean (0 errors; three pre-existing warnings: `ShareSurface` in `src/portfolio/share/validation.rs`, `journal` field / `Published` variant in `src/main.rs` `FleetEntryOutcome`, none added by this change) |
| `cargo fmt --check` | clean |
| `cargo test --test forge_admin_api_contract` | **4 passed / 0 failed** (login+cookie+fleet+revoke, bad creds/untrusted origin, anonymous+expired+OIDC-cookie isolation, logout-origin-mismatch does not revoke) |
| `cargo test --test forge_portal_frontend_contract` | **5 passed / 0 failed** (standalone assets, login has only email/password and no project-id, dashboard drives JSON API with honest empty/unavailable states, a11y/responsive stylesheet, no markup in Rust) |
| `cargo test --lib identity::global` | **3 passed / 0 failed** (lifecycle, weak/bad-email refusal, expiry) |
| `cargo test --lib web::tests` | **1 passed / 0 failed** (allowlist, root serves login) |
| `cargo test --test identity_contract` / `portal_ui_contract` / `api_contract` | **19 / 39 / 11 passed, 0 failed** (compatibility intact; `api_contract` run outside the sandbox for loopback binds) |
| live loopback round-trip | real `api serve` + `web serve`: configured/unconfigured session JSON, bad login → generic `401`, successful login → `HttpOnly; SameSite=Lax; Max-Age=43200` cookie (no `Secure` on loopback), `GET /v1/admin/projects` returns the real registered `forge-demo-proj` row plus empty/`registered:0` states, DELETE → `Max-Age=0` then `401`, hostile origin → `403`; web server serves `/`,`index.html`,`app.js` and rejects traversal/`POST`/unknown with `404`/`405`; `identity setup` refused on non-TTY stdin and succeeded through a pty without echoing the password |
| `node scripts/check-openspec-change-names.mjs` | PASS (before and after archive) |
| `openspec validate --all --strict --no-interactive` | **69 passed / 0 failed** (before and after archive) |
| `cargo deny check advisories licenses` | **advisories ok, licenses ok** (argon2 0.5.3); only benign unmatched-license-allowance warnings |
| `git diff --check` | PASS |

Honest scope of verification: the browser keyboard-navigation and 320px-reflow
visual experience and the login/dashboard DOM rendering are covered by the
markup/source contract above and were exercised at the HTTP/JSON level against
the running listeners, but were **not** rendered in an interactive headless
browser here; that visual pass remains for the operator. No shared Gate Runtime
is configured; no Gate pass is claimed.

### Fresh clones did not build

`.gitignore` excluded `/templates/`, filed under "local agent runtime state" by
`b85f980` alongside `/.ariadex/`, `/.claude/` and `/.commandcode/`. It is not agent
state: `src/generate/mod.rs:662` embeds `templates/rust-web-main-rs.txt` with
`include_str!`, so the `rust-web` scaffold body could never be committed. Proven by
moving the directory aside and building —
`error: couldn't read src/generate/../../templates/rust-web-main-rs.txt`,
`could not compile forge (lib)`. Every clone since `b85f980` was incomplete; only the
working machine had the file. The rule is removed and the template is tracked.

Local agent state for this CLI (`/.qoder/`) is now ignored on the same terms as the
other agent directories, and the root `driftwatch.toml` is tracked, which is what the
`.gitignore` comment next to `.driftwatch/` already said should happen.

## Seven in-flight changes consolidated, then archived

Seven change packages were implemented, verified and committed on `main`, and none
was archived. Each one recorded `openspec archive` as "deliberately not run: the
other changes are parked" — which is inverted, because archive is the per-change
step that removes a change from flight. The queue had jammed on a blocker in
`contract-parity-gate-real-digests` §4.4 (`check-spec-governance.mjs` failing on the
`scaffold-prewires-shared-layer` spec), and that blocker had already been fixed by
`5019d12` without anyone re-running the check. That check reports PASS today.

The seven are merged into one archived package and their directories are removed.
Their full text is preserved in git from `c138f08` to `1c3e1f5`.

| Absorbed change | Code commit | Concern |
|---|---|---|
| `contract-parity-gate-real-digests` | `c138f08` | vendored contract bytes |
| `manifest-wire-contract-shape` | `619b945` | the emitted manifest's wire shape |
| `governance-adapter-request-write-race` | `21a9566` | a governance adapter that never reads its request |
| `governance-adapter-bounded-process-run` | `5d5f103` | the adapter subprocess can deadlock; the revision lookup had no deadline |
| `studio-preview-contract-port-range` | `5d5f103` | a test port range inside the OS ephemeral window |
| `adapter-request-write-boundary` | `e3a156b` | the same broken-pipe request write at the publish, translate and hermora boundaries |
| `studio-test-port-range` | `f078a4c` | three more hardcoded test port ranges inside the OS ephemeral window |

Merging was necessary, not cosmetic: `src/governance.rs` was edited by three of the
five code commits, and `f078a4c` deleted ~100 lines `5d5f103` had just written into
`tests/studio_preview_contract.rs` to move them into the shared
`tests/support/studio_ports.rs`. Archiving separately would have promoted two
near-duplicate port requirements into `runtime-hardening-and-test-isolation` and
written the broken-pipe rule twice at two scopes. After the merge there is **one
requirement per mechanism**:

- the request-write rule in `governance-provider-contract` only, widened to all four
  boundaries and carrying all four unioned scenarios;
- the Studio test port window in `runtime-hardening-and-test-isolation` only,
  carrying the union of six scenarios;
- the allocator refusal, the bounded adapter run, the bounded revision lookup, the
  parity gate and the manifest wire shape carried verbatim.

`openspec archive` ran with spec promotion, no `--skip-specs`:
**11 requirements added, 1 modified**, across `governance-provider-contract` (3),
`platform-contract-consumption` (5 added, 1 modified), `portfolio-share` (1),
`runtime-hardening-and-test-isolation` (1) and `site-studio-preview-refinement` (1).
Verification re-run at archive time rather than inherited: whole workspace
**2305 passed / 0 failed / 3 ignored**, `scripts/contract-parity.sh` **exit 0**
comparing 13 files and 10 family digests, manifest schema acceptance against the
sibling contract **1 passed**, `cargo fmt --check` clean, clippy exit 0,
`check-openspec-change-names` and `check-spec-governance` PASS.

The narrative sections below still name the seven absorbed packages, because each one
records a distinct mechanism and its measured evidence. They are history, not
work-in-flight: every requirement they describe is now in a canonical spec.

## What governance-adapter-bounded-process-run fixes

Three defects in `run_adapter` / `git_revision`, all recorded as outstanding in
`governance-adapter-request-write-race`'s `design.md` §5 and all reachable from
`forge governance check`.

1. **A large stdout deadlocked.** `MAX_ADAPTER_OUTPUT_BYTES` is 256 KiB, a pipe
   buffer is 64 KiB, and the old code read a pipe only *after* `try_wait`
   reported the child gone. Any adapter writing more than one buffer blocked in
   `write(2)` on every run and was reported `Unavailable` —
   `adapter exceeded timeout of 5000 ms`. Arithmetic, not a race. Both pipes
   are now drained on their own threads while the parent waits, bytes past the
   cap counted and discarded so the cap stays a real memory bound.
2. **`git_revision` had no deadline.** `Command::output()` blocks until the
   child exits, forever, and it runs on *every* check including the local
   provider. It is now bounded by the selected provider's existing `timeout_ms`
   — the constant `validate_config` already constrains — so no new number
   enters the boundary. Measured against the pre-fix expression: `timeout 8` →
   **exit 124**, never returned.
3. **The wait polled every 10 ms.** It is now a blocking `recv_timeout` that
   wakes on a pipe reaching end-of-file or at the deadline.

Two further findings came out of the implementation and are documented rather
than papered over. `/bin/sh -c '…'` forks on this host, so killing a child does
not close a pipe a descendant inherited (**measured**: the pipe stayed open the
full 30 s), which is why the drain threads are detached rather than joined — a
join there took the full 30 s, an unbounded wait in the exact case the bound
exists for. And end-of-file is **not** an exit event, so a waiter that woke
only on end-of-file reported `exceeded timeout` for adapters that had answered
completely (**measured**: 2–4 failures per run, at exact multiples of the 5 s
and 10 s test budgets; 0 failures and 0.02 s after the fix). That one narrow
window — both pipes done, child not yet reaped — is the only place a short
re-check remains, and it has its own deterministic guard.

`21a9566` is not regressed: the `BrokenPipe`-is-not-a-failure rule and the
kill-and-reap on a genuine write failure are carried through untouched, and
both of its guards still pass.

### Verification of these two changes (2026-10-05)

| Check | Result |
|---|---|
| `cargo test --test studio_preview_contract` × 30 | **30/30 passed**, `7 passed; 0 failed` every run |
| `cargo test --test studio_preview_contract -- --test-threads=8` × 10 | **10/10 passed** |
| `cargo test --test governance_contract` × 30 | **30/30 passed** |
| `cargo test --test governance_contract -- --test-threads=8` / `=1` | **10/10** and **10/10** passed |
| `cargo test --lib governance::tests` | 17 passed |
| Full suite, 6 runs with the change **stashed** | 4 of 6 **failed**; run c failed `preview_port_collision_is_refused_without_killing_a_listener` with `left: 56, right: 64` |
| Full suite, 5 runs with the change | 2 clean (`2303 passed / 0 failed`), 3 with failures all drawn from the pre-existing pool |
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets` | exit 0; 12 warnings stashed, 12 applied — none added |
| `git diff --check` | PASS |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | 68 passed / 0 failed |

Guard-by-guard, against the pre-fix mechanism, with the guard reverted rather
than the fix assumed:

| Guard | Against the pre-fix mechanism |
|---|---|
| `an_adapter_that_writes_more_than_one_pipe_buffer_still_answers` | **fails**: `left: Unavailable, right: Pass` |
| `an_adapter_that_writes_past_the_output_cap_is_refused` | **fails**: `Ok(... status: Unavailable, detail: "adapter exceeded timeout of 5000 ms")` |
| `a_revision_lookup_that_never_answers_gives_up_within_its_bound` | the pre-fix `Command::output()` expression, rebuilt and run, **never returned**: `timeout 8` → exit **124** |
| `a_child_that_closed_its_pipes_and_keeps_running_is_not_reported_as_a_timeout` | **fails**: `elapsed 5.004397229s` of a 5 s budget |
| Studio liveness loop, one listener removed | **fails**: `listener on 4100 no longer accepts: Connection refused (os error 111)` |

The first full-suite runs with an early version of the fix failed 2–4 tests per
run at exact multiples of the 5 s/10 s test budgets. That was this change's own
bug — end-of-file is not an exit event — and it is documented in the change's
`design.md` §3.4 rather than quietly dropped.

## Whole-suite flakes: four fixed here, four still open and classified

| Test | Signature | State |
|---|---|---|
| `studio_preview_contract::preview_port_collision_is_refused_without_killing_a_listener` | `left: 56, right: 64` | **fixed** by `studio-preview-contract-port-range` |
| `studio_api_contract::api_owns_the_live_preview_between_start_and_stop` | `studio port unavailable: no free port in range 47100..=47163` | **fixed** by `studio-test-port-range` — mechanism reproduced and proved, see below |
| `studio_cli_contract::preview_start_probe_reaches_ready_and_leaves_no_live_process` | same, for `47300..=47363` | **fixed** by `studio-test-port-range` |
| `delivery_cross_surface::a_successful_preflight_writes_a_journal_row_visible_on_both_transports` | `publish provider 'openpanel': Broken pipe (os error 32)` | **fixed** by `adapter-request-write-boundary`; 30/30 target runs |
| `docs_contract::provider_failure_keeps_prior_derivative_and_redacts_secrets` | `translator stdin write failed: Broken pipe (os error 32)` | **fixed** by `adapter-request-write-boundary`; 30/30 target runs |
| `api_contract::{healthz_route_returns_200_without_authorization, unknown_route_returns_404, wrong_method_returns_405}` | `ConnectionReset (os error 104)` | **pre-existing, reproduced stashed**: 3 failures in 8 target runs — untouched here |
| `mcp_contract::mcp_repeated_isolated_round_trip_is_stable` | two invocations, `observed_at` one second apart | **pre-existing, reproduced stashed** — untouched here |
| `docs_contract::ordering_is_stable_by_project_then_source_whatever_the_selection_order` | — | **pre-existing, reproduced stashed** — untouched here |
| `Text file busy (os error 26)` at **three** lib sites — `portfolio::share::publish::tests::publish_timeout_is_bounded`, `policy::tests::unrecognized_json_documents_never_report`, `gate::tests::real_run_executes_gate_surface_and_records_evidence` | `spawn failed: Text file busy (os error 26)` | **pre-existing, root-caused, not fixable in scope** — see "The Text file busy flake, root-caused" below |

The three `BrokenPipe` boundaries named as "the strongest candidate for the next
change" are now fixed; the rule they share lives in `process::write_request`.

## The `Text file busy` flake, root-caused

It is **not** a gate-runner bug, **not** an environmental quirk, and **not** a
collision between parallel test targets. It is a harness race, and it is
reproducible on demand. Four sites now, and the first three are all in the lib
target: whichever one writes a script happens to be racing whichever spawner.

Measured rate on this machine: 1 failure in 30 `cargo test --lib` runs, 1 in 40,
and 1 on the first `strace` attempt. **0 failures in 200 runs of the single test
in isolation** — which is exactly why the previous worker recorded it as
"passes 5/5 in isolation both ways" and could not place it.

The three sites, all in the lib target, all the same shape: write a script into
a temp directory with `fs::write`, `chmod +x`, then `Command::new(that path)`:

| Site | Fixture |
|---|---|
| `src/portfolio/share/publish.rs:444` | `/tmp/forge-share-timeout-<pid>/wedged.sh` |
| `src/policy/mod.rs` (`executable()`) | `<TempDir>/proj/dw-junk.sh` |
| `src/gate/mod.rs:1072` (`write_fixture_script`) | `<TempDir>/gate-ok.sh` |
| `tests/inventory_contract.rs:281` | `<TempDir>/inventory-adapter.sh`, staged with `fs::copy` |

The syscall trace of a real failure shows the mechanism exactly:

```
2445580 openat("…/proj/dw-junk.sh", O_WRONLY|O_CREAT|O_TRUNC|O_CLOEXEC) = 27
…
2445600 execve("…/proj/dw-junk.sh", [… "--version"]) = -1 ETXTBSY
```

`2445600` is a **forked child**, and so are `2445597`, `2445598`, `2445599` and
`2445603` — other tests' spawns, interleaved. `fs::write` holds the
`O_WRONLY` descriptor across its `open`/`write`/`close`, and Rust's spawn path
is `fork` + `execvp`, so **a child forked inside that window inherits a copy of
the write descriptor**. The child then execs the very file that descriptor is
open for. `O_CLOEXEC` does not save it: the kernel checks `ETXTBSY` while
opening the new executable, which is *before* it closes the close-on-exec
descriptors. The exec of an executable that another live descriptor has open
for writing is refused, which is the ordinary "you cannot overwrite a running
executable" rule.

That accounts for every observation, including the ones that looked like
evidence for other explanations:

- **0/200 in isolation** — one thread, so no second spawn can fork inside the
  write window.
- **A different test each time** — it is whichever script-writer happens to be
  racing whichever spawner.
- **Not a shared artifact** — each fixture lives in its own `TempDir`; what is
  shared is the process's descriptor table, not a file.
- **Not the filesystem** — `/tmp` is tmpfs here, but the rule is inode-wide and
  behaves the same on ext4.
- **The leaked directories** (`/tmp/forge-share-timeout-*` on disk right now)
  are a *consequence*: the panic skips the `remove_dir_all` at the end of the
  test, so a failed run leaves its fixture behind.

**Why it is not fixed here.** Closing the window needs every `spawn` in the
process to be excluded from the write window, which means a lock around *all*
process creation in a 1180-test binary plus every integration-test binary.
One of the three sites (`src/portfolio/share/**`) is explicitly outside this
change's scope, and the other two are lib-crate unit tests whose spawn sites are
product code. It is therefore left **named and owned here rather than left
unowned**: the honest statement is that it is a genuine race with a known
mechanism and no fix that fits one change. Scheduling it means picking one of:

1. a crate-wide spawn/write lock used by every test that stages an executable
   (touches product code, or needs the tests moved behind a helper), or
2. replacing staged-script fixtures with `sh -c '<body>'`, which removes the
   writable executable entirely — not possible where the code under test
   resolves a *path*, which is all three sites.

## What adapter-request-write-boundary fixes

`21a9566` fixed a `BrokenPipe` on the *governance* adapter's request write and
recorded the other three as mechanical. Those three are now fixed by one shared
rule instead of three more copies of it:

```rust
// src/process.rs
pub fn write_request(stdin: &mut impl Write, request: &[u8]) -> std::io::Result<()>
```

`BrokenPipe` is `Ok(())` — the peer closed its input, so the caller goes on to
the child's real exit status and output. Every other error is returned for the
caller to map to its own typed refusal.

| Site | Pre-fix | Post-fix |
|---|---|---|
| `src/publish/providers.rs:529` | bare `write_all` → `PublishInvalid`, **child left running** | shared rule, kill + reap, same message |
| `src/docs/mod.rs:676` | bare `write_all` → translator error | shared rule; its existing kill + reap unchanged |
| `src/delivery/hermora.rs:173` | `let _ = stdin.write_all(&payload)` — **every** error discarded | shared rule, kill + reap, typed `DeliveryUnavailable` |
| `src/governance.rs:898` | already correct (`21a9566`) | its mapper delegates to the shared rule; both landed guards untouched |

Hermora was the odd one: its comment already stated the correct rule ("a broken
pipe here means the adapter exited early") and the code under it threw the
error away, so a genuine `EIO`/`ENOMEM` while writing a request was silently
swallowed and Forge then reported the adapter's own exit status for a request
that was never delivered.

**Why one shared function rather than three more edits.** The decision "`BrokenPipe`
reports on the child, so it is not Forge's failure" is policy, not a detail of
one module, and four copies is how Hermora's `let _ =` was written in the first
place — three sites spelled the rule and the fourth had nothing to copy. The
helper takes a `&mut impl Write`, not a `&mut Child`, precisely so the
*non*-`BrokenPipe` arm stays unit-testable with a `Write` impl that always fails
`PermissionDenied`. That guard is what catches a reintroduced `let _ =`.

Two guards in `src/process.rs::tests`, both deterministic:

| Guard | Against the pre-fix behaviour |
|---|---|
| `a_request_write_to_a_child_that_already_exited_is_not_a_failure` | **fails 1/1**: `Os { code: 32, kind: BrokenPipe }` |
| `a_request_write_failure_that_is_not_a_broken_pipe_is_returned` | passes both ways by design — it pins the *retained* refusal |

### The deadlock check on the three new sites

Asked to check each site for the pipe-buffer deadlock `5d5f103` fixed in
`governance.rs`. **The shape is present at all three, and worse**: stdout is
drained only *after* `try_wait` reports exit, and none of the three caps stdout
at all.

| Site | stdout drained while the child runs? | cap | a child writing > 64 KiB to stdout |
|---|---|---|---|
| `src/publish/providers.rs:589-607` | no (stderr *is*, on its own thread, `:546`) | none | blocks in `write(2)`, killed at `provider_timeout()`, reported `timed out` |
| `src/docs/mod.rs:684-693` | no | none | blocked, killed at the translator timeout |
| `src/delivery/hermora.rs:180-200` | no (stderr is `/dev/null`) | `RESPONSE_BYTES_MAX` applies *after* the wait, so it bounds the retained response, not the read | blocked, killed at `adapter_timeout()` |

**It is a misclassification, not a hang**: each loop has an existing deadline it
returns from, so the wrong verdict is "timed out", not "wedged". It is left
unfixed deliberately — closing it means re-deriving the bounded-run machinery
three more times, including its measured `/bin/sh` fork hazard where drain
threads must be *detached* because joining one took the full 30 s. No test in
this repository produces more than ~10 KiB on these channels, so nothing
attributes a failure to it. Recorded in the change's `design.md` §5.

## What studio-test-port-range fixes

Three more targets hardcoded bases inside this host's ephemeral window
(`/proc/sys/net/ipv4/ip_local_port_range` = `32768 60999`):
`studio_api_contract.rs:193` (`47100`), `studio_cli_contract.rs:505,549`
(`47300`) and `react_web_native_preview.rs:153` (`48200`). One shared helper,
`tests/support/studio_ports.rs`, now serves all four Studio targets including
the one the previous change fixed, so there is one search rather than four.

**Mechanism proved both ways.** With an unrelated python process holding all
256 ports of `45800-45863`, `47100-47163`, `47300-47363` and `48200-48263`:

- with the fix, all four targets pass;
- with the pre-fix bases restored, `studio_api_contract` fails
  `studio port unavailable: no free port in range 47100..=47163` and
  `studio_cli_contract` fails `no free port in range 47300..=47363`.

**A defect this change introduced and then fixed.** Its own first full-suite run
failed, because a `react_web_native_preview` run was executing at the same time
and all four targets preferred the *same* window:

```
studio-start-timeout: profile runner exited before binding the reserved port (port 4100)
```

`cargo test` runs test binaries one at a time; two `cargo test` invocations do
not. Each target now starts its search at its own index, so it prefers `4100`,
`4228`, `4356` or `4484` — rotated, not filtered, so every candidate stays
reachable. Measured with `ss -ltn` while three targets ran: `4228` and `4356` in
use at once, from two different targets. The underlying gap is the allocator's
own bind/drop window, which is a product defect and is recorded as such.

`preview_port_collision_is_refused_without_killing_a_listener` bound a fixed
range `45800–45863`, inside this host's ephemeral window
(`/proc/sys/net/ipv4/ip_local_port_range` = `32768 60999`). One unrelated
outbound connection took one port and the test failed `left: 63, right: 64`.
The test now reads the ephemeral window, considers only candidates lying wholly
outside it, and **keeps the listeners** that verified its range was free, so
nothing can change between choosing the range and occupying it. Measured: with
`4100..4163` held by an unrelated process, the whole target still passes 7/7.

The test also had a second, hidden defect. The "listeners are all still alive"
assertion compared a `Vec` length with the value it was built from and **could
not fail**. It now proves a listener still accepts on every port and that no
port was released. The record states plainly that neither half can prove
*ownership*, because nothing observable from outside a process can.

`manifest-wire-contract-shape` is implemented, verified and archived — promoted on
2026-10-05 as part of `runtime-hardening-and-contract-closure`, together with the six
other packages that had been committed without promotion.

## Verification of adapter-request-write-boundary and studio-test-port-range

Every run of every target, 2026-10-05. **400 consecutive runs, all clean.**

| Check | Result |
|---|---|
| 7 targets × 30 runs at default parallelism | **210/210 clean** |
| `cargo test --lib` × 30 | **30/30** `1180 passed; 0 failed` |
| 8 targets × 10 at `--test-threads=8` | **80/80 clean** |
| 8 targets × 10 at `--test-threads=1` | **80/80 clean** |
| held-range experiment (256 ports of the four old bases held) | 4 targets × 3 runs, **12/12 clean** |
| held-range **negative control** (pre-fix bases restored) | `studio_api_contract` fails `no free port in range 47100..=47163`; `studio_cli_contract` fails `no free port in range 47300..=47363` |
| concurrency control (3 Studio targets at once, 5 rounds) | **15/15 clean** with distinct slots; with every slot forced to `0` the same control failed inside five rounds |
| `FORGE_NATIVE_REACT_WEB_PREVIEW=1 cargo test --test react_web_native_preview` | **6/6 passed**, each a real `npm install`, a real Vite dev server on the run-time base and a Playwright render (`VERIFIED: rendered "hello from native-react-preview"`) |
| new guard vs. pre-fix behaviour | `a_request_write_to_a_child_that_already_exited_is_not_a_failure` **fails 1/1** with `Os { code: 32, kind: BrokenPipe }` |
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets` | exit 0; zero diagnostics in any file either change touches, and the per-target warning set is **identical** stashed vs applied |
| `git diff --check` | PASS |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **70 passed / 0 failed** |

### Whole suite, 6 runs each, with and without the two changes

| Run | with the change | change **stashed** |
|---|---|---|
| 1 | 2305 passed / 0 failed / 3 ignored | 2302 / **1 failed** / 3 — `delivery_cross_surface::a_successful_preflight_writes_a_journal_row_visible_on_both_transports` |
| 2 | 2304 / **1 failed** / 3 — `inventory_show_consumes_external_adapter_executable` | 2303 / 0 failed / 3 |
| 3 | 2304 / **1 failed** / 3 — `flutter_scaffold_tests_pass_without_forge` | 2302 / **1 failed** / 3 — `format_json_does_not_alter_checker_stdout` |
| 4 | 2305 / 0 failed / 3 | 2302 / **1 failed** / 3 — the `delivery_cross_surface` BrokenPipe test again |
| 5 | 2304 / **1 failed** / 3 — `flutter_scaffold_tests_pass_without_forge` | 2302 / **1 failed** / 3 — the `delivery_cross_surface` BrokenPipe test again |
| 6 | 2305 / 0 failed / 3 | 2302 / **1 failed** / 3 — `docs_contract::provider_failure_keeps_prior_derivative_and_redacts_secrets` |

**3 of 6 applied runs clean, against 1 of 6 stashed.** Four of the five stashed
failures are the two `BrokenPipe` tests this work fixes, and neither appears in
any applied run. The two-test difference in the totals is the two new unit
guards.

The host matters for reading these numbers: it is shared, and during this work
another tenant had a load average above 30 with several processes pinned at
100 % CPU. Every number above was measured under that load, not on an idle
machine.

### The two applied failures that remain, and their status

| Test | Status |
|---|---|
| `inventory_show_consumes_external_adapter_executable` — `cannot start inventory adapter /tmp/.tmpcyR0pz/inventory-adapter.sh: Text file busy (os error 26)` | **Pre-existing class, not reproduced stashed.** The failure is a *spawn* failure: `tests/inventory_contract.rs` does `fs::copy` of a fixture script and `src/publish/inventory.rs:382` execs it, which is the write-descriptor race above. Both files are untouched by this work and the failure happens before any request write, so it cannot come from it. **Not observed in the 6 stashed runs, and not reproduced in 240 stashed runs of that target** (120 serial + 120 under 6-way parallel load); it needs the whole-suite's process pressure |
| `flutter_scaffold_tests_pass_without_forge` | **Environmental, not reproduced stashed.** It shells out to `flutter test` → `dart pub get`, which needs the network and the whole machine; it failed 2 of 6 applied runs and 0 of 6 stashed. `tests/generate_contract.rs` is untouched by this work |
| `format_json_does_not_alter_checker_stdout` (stashed run 3 only) | **Pre-existing**, seen only with the change stashed |

## What governance-adapter-request-write-race does

`tests/governance_contract.rs` was flaky: ~50 % failure rate, 1–2 tests failing
per run, varying every time. It was reported as a shared-state problem and it
was **not** one. Two measurements settle it:

```
$ for i in $(seq 1 20); do cargo test --test governance_contract -- --test-threads=1; done
7 runs FAILED (1-3 tests each), 13 runs ok
```

Serialising the target does not fix it, which is what a per-test race does and
what shared state does not. The four tests that ever failed are exactly the
four whose adapter script never reads standard input:

```sh
#!/bin/sh
printf '%s' '{"provider":"external", ...}'
```

Such an adapter exits immediately and closes the read end of its stdin pipe,
so Forge's own request write can lose the race and come back
`Broken pipe (os error 32)`. `run_external_provider` maps that to
`ProviderStatus::Unavailable` and **throws the adapter's answer away** — the
answer was already complete on stdout. Measured detail, captured through the
real `save_provider_selection` + `check_project` path:

```
status=Unavailable detail=Some("cannot write adapter request: Broken pipe (os error 32)")
```

The fix is in `src/governance.rs`, not in the test. `BrokenPipe` is the one
write error that says something about the *adapter* rather than about Forge's
plumbing, so `write_adapter_request` treats it as a completed write and Forge
reads the exit status, stdout and stderr the adapter actually produced. Every
other write error keeps its typed unavailable refusal, and now kills and reaps
the child before returning. No test was ignored, serialised, slept or retried.

The shared-state hypothesis was tested and cleared rather than assumed: no
`set_var`/`remove_var` or `set_current_dir` on this path, no `static` /
`OnceLock`, per-test `TempDir`, pid-keyed temp file names that cannot collide
across distinct directories, no socket, and read-only fixtures.

### Verification (2026-10-05)

- `cargo test --test governance_contract`, **245 consecutive runs** at default
  parallelism across five blocks: **244 passed, 1 failed**, and the final
  contiguous block of 20 was **20/20 passed** (see the open item below). A
  further 20 runs at `--test-threads=8`: 20/20 passed. A further 20 at
  `--test-threads=1`: 20/20 passed.
- Before the fix, for comparison: 13 failures across 8 of 20 default runs, and
  7 failures across 4 of 20 serialised runs.
- The deterministic guard was checked against the **pre-fix** code: with the
  `BrokenPipe` arm removed,
  `a_request_write_to_an_adapter_that_already_exited_is_not_a_failure` fails
  1/1 with `cannot write adapter request: Broken pipe (os error 32)`. It is not
  a vacuous test. The end-to-end guard
  `an_adapter_that_never_reads_its_request_still_answers` is a *weaker*
  instrument — it detected the pre-fix defect in 0/10 whole-target runs,
  because `run_adapter` writes immediately after `spawn()` and the child must
  win a sub-millisecond race. It is kept as the only place in the suite that
  names the scenario as a requirement, not as the deterministic pin. A guard
  that raised the odds with CPU pressure or repetition was rejected: that is the
  nondeterminism this change exists to remove.
- `cargo test --workspace --all-targets --no-fail-fast -- --skip
  generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`,
  `cargo fmt --check`, `cargo clippy --workspace --all-targets` (zero findings
  in either file this change touches), `git diff --check`,
  `node scripts/check-openspec-change-names.mjs`,
  `openspec validate --all --strict --no-interactive` — see the totals in the
  change's `tasks.md` §4.
- No shared Gate Runtime is configured; no Gate pass is claimed.

### Outstanding

- **One unexplained failure in 245 runs.** A single run of the 245 (default
  parallelism) reported `20 passed; 1 failed` and the capture did not record
  which test or why. It was not reproduced in the 20 runs at
  `--test-threads=8`, the 20 at `--test-threads=1`, or 220 further default
  runs, and it is **not** claimed as fixed. It is most likely a different,
  environmental flake — see the studio port note below — but that is a
  hypothesis, not a measurement, and it remains open: the capture never
  recorded which test failed, so nothing links it to anything later measured.
- ~~**Three adjacent defects found while mapping `run_adapter`, not fixed here**~~
  **Since fixed** by `governance-adapter-bounded-process-run` (concurrent pipe
  drain, a `git_revision` deadline, a blocking wait instead of a 10 ms poll).
  These three were **not** the cause of this flake — no adapter in this
  repository produces more than ~10 KiB.
- ~~**`tests/studio_preview_contract.rs::preview_port_collision_is_refused_without_killing_a_listener`
  is a separate, pre-existing flake**~~ **Since fixed** by
  `studio-preview-contract-port-range`, which chooses the range at run time
  outside the host's ephemeral window and proves the occupied listeners survive
  the refusal with an assertion that can fail.

## What manifest-wire-contract-shape does

Forge is the **producer** of the manifest a separate static Hugo site consumes.
Three fields it emitted were rejected by the consumed schema, so the first real
export would have failed the consumer's build. The schema is
`platform-contracts/schemas/public-portfolio-manifest.schema.json`
(`platform.public-portfolio-manifest/1.0.0`, digest
`07a3c47769da8923998273cda602ddffb195f983e3dcf344fb1d86540c6bc986`).

| Field | Was | Now |
|---|---|---|
| `schema_family` | `"public-portfolio-manifest"` | `"platform.public-portfolio-manifest"` |
| `schema_version` | JSON number `1` | string `"1.0.0"` |
| `manifest_revision` | JSON number (`u32`) | string `"rev_<revision>"` |

The internal revision is **unchanged everywhere it is stored**: the
`manifest_revision INTEGER` column, the `i64` approval, audit, publication
report and adapter envelope, and `build_manifest(records, u32)`. Only the
serialized field is a string, produced by one function,
`wire_manifest_revision`, so no call site can invent a second spelling.

The encoding is a pure, injective function of the integer, so an unchanged
catalog still hashes identically. One honest consequence: an approval made
before this change is bound to the old hash, so `publish` now refuses it until
the operator previews and approves again. No stored approval or audit entry was
rewritten — that would forge an approval nobody gave.

## Verification (2026-10-05)

- `cargo build`: clean. One `unused import: ShareSurface` warning in
  `src/portfolio/share/validation.rs`, **verified pre-existing** (present with
  this change stashed).
- `cargo test --workspace --all-targets --no-fail-fast -- --skip
  generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`:
  **2294 passed / 0 failed / 3 ignored**. The skip is the pre-existing hang
  described under "Pre-existing conditions" below. The three ignored tests are
  the pre-existing `contract::tests::parity_walk`, the pre-existing
  `packing_leaves_the_sibling_checkout_byte_identical`, and this change's own
  acceptance test, run explicitly.
- `cargo test --test manifest_wire_contract -- --ignored --nocapture`:
  **1 passed** — `jsonschema` accepted a Forge-produced manifest.
- **Acceptance, consumer's own oracle**: a scratch registry, a real
  `target/debug/forge portfolio share set → preview → approve → publish` cycle,
  and then
  `python3 lileililiwen.github.io/scripts/validate_manifest.py --strict`
  against `/home/paul/code/platform-contracts/schemas/public-portfolio-manifest.schema.json`:
  `manifest OK … (schema 1.0.0, 1 project(s), revision rev_1)`, **exit 0**.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate --all --strict --no-interactive`: 65 passed / 0 failed.
- `cargo fmt --check`: clean (it was already clean on `main` before this
  change; the "five dirty files" note further down is stale).
- `cargo clippy --workspace --all-targets`: exit 0, and **zero findings in any
  file this change touches** (`src/portfolio/share/mod.rs`,
  `src/portfolio/share/manifest.rs`, `tests/portfolio_share_cli_contract.rs`,
  `tests/manifest_wire_contract.rs`). The remaining warnings are all in files
  this change does not own.
- `git diff --check`: PASS.
- No shared Gate Runtime is configured; no Gate pass is claimed.

## Three further contract mismatches found, and not fixed

Found while mapping the document, reproduced, and deliberately left as
non-goals of this change rather than silently widening its scope. All three are
reachable from the command line today, and each produces a published manifest
the consumer rejects.

1. **`visibility`** — the schema's enum is `["public"]`; Forge's `Visibility::ALL`
   also admits `unlisted`. `share set --visibility unlisted` publishes a
   document rejected with `projects/0/visibility: 'unlisted' is not one of
   ['public']`.
2. **`status_evidence`** — the schema is `additionalProperties: false` and
   permits only `observed_at`, `source`, `note`; Forge's `EVIDENCE_KEYS` admits
   `observed_at`, `source_system`, `source_revision`, `state`, `source`. A
   record with `--evidence '{"observed_at":"…","source_system":"…"}'` is
   rejected with `Additional properties are not allowed ('source_system' was
   unexpected)`.
3. **`id` length** — the schema caps `id` at 64 characters.
   `validate_project_id` already pins the schema's *pattern* exactly but has no
   length bound, so a 70-character id registers, publishes and is rejected with
   `projects/0/id: … is too long`.

Fixing these means deciding whether Forge narrows its vocabularies or the
contract widens them. That is a product decision, not a mechanical one, and it
belongs to its own change.

## The mirror gap is closed, but its test still points at the sibling

The schema that was missing from Forge's mirror —
`contracts/schemas/public-portfolio-manifest.schema.json` — **is now present**, and
`scripts/contract-parity.sh` verifies it and the twelve other retained files
byte-for-byte against the resolved `platform-contracts` source (13 files, 10 family
digests, exit 0, re-run 2026-10-05). The reason nothing in this repository could
catch the wire-shape defect no longer holds.

One piece of that change's workaround is still standing, and it is now unnecessary
rather than wrong: `tests/manifest_wire_contract.rs::pinned_schema_path` resolves
only `PLATFORM_CONTRACTS_DIR` or a sibling `platform-contracts/` checkout, never the
vendored mirror, so its decisive test stays `#[ignore]`d and is run explicitly.
Repointing it at `contracts/schemas/` would let the schema check run in the ordinary
suite — it would still need Python with `jsonschema`. That is a test behaviour
change, so it is left as open work for its own package rather than folded into a
documentation refresh.

The work landed on `main`. It was originally committed on a
`feat/scaffold-prewires-shared-layer` branch; `main` was fast-forwarded onto it
and the branch deleted, per owner direction that Forge work goes straight to
`main`. See "Why a branch appeared" below.

## What this change does

- A profile may declare a **versioned shared-layer kit** as a compiled-in
  descriptor resolved offline. `forge new` renders that reference into the
  generated project's own native manifest — the `kit` block in `forge.yaml` —
  with no feed access, no sibling checkout and no network (`src/kit/`,
  `src/profile/mod.rs`, `src/generate/mod.rs`).
- A profile declares a **minimum consumption floor**. Unmet is a typed refusal
  before anything is written; there is no warn-and-continue path. An operator
  can override with `--kit-exception <reason>`, which is recorded visibly in
  `forge.yaml` and the README. A profile with no registered kit records a
  **declared zero** with a `zero_reason`, which is a distinct state from a floor
  failure (`src/kit/floor.rs`).
- The `aspnet-web`, `react-web` and `nextjs-web` scaffolds **pre-wire** the
  shared layer. .NET restores from a **committed, project-relative feed** — a
  `NuGet.config` with `<clear />` and one named source at
  `packages/platform-feed` — so a fresh clone restores with no sibling checkout,
  no environment variable and no secret. The Node profiles get digest-pinned
  vendored tokens under an owned `.platform/` subtree with an ownership receipt
  (`src/kit/feed.rs`, `src/kit/assets.rs`, `kits/`).
- `forge kit` is the one new top-level verb: `pack` regenerates the feed,
  `verify` catches drift, `upgrade` is the single sanctioned way a pinned
  project moves between kit versions.
- The pinned kit id and version are observed additively on the existing project
  row. No new SQLite table (`src/registry/mod.rs`).

## The floor, and the evidence it came from

`6` for `aspnet-web`, `1` for the two Node profiles, `0`-with-reason elsewhere.
Derived from per-package distinct external consumer counts, frozen in
`kit::PLATFORM_PACKAGE_EVIDENCE`; a test fails when a classification disagrees
with the fixture.

- Confirmed (6): `Platform.Core` (7), `Platform.AspNetCore` (6),
  `Platform.Testing` (4), `Platform.RateLimiting` (2), `Platform.Idempotency`
  (2), `Platform.Observability` (2).
- Withheld despite clearing the bar, because they need a store:
  `Platform.Persistence.EfCore` (4), `Platform.Identity.AspNetCore` (3),
  `Platform.Tenant.Lifecycle.AspNetCore` (0).
- Provisional (below the bar, rendered commented, never restoring): everything
  else, each comment naming its own consumer count.

The **restore closure is 9 packages, not 6**. `Platform.Billing.Contracts`,
`Platform.Eventing` and `Platform.Web.Telemetry` arrive as project references of
the confirmed set, so the feed carries all nine. The floor still counts only the
six; the three transitive members are never counted and never directly
referenced.

## Verification (2026-10-03)

- `cargo test --workspace --all-targets --no-fail-fast -- --skip generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`:
  **2291 passed / 0 failed**. The one skipped test is discussed under
  "Pre-existing conditions".
- `cargo test --test kit_contract`: **60 passed / 0 failed**, stable across
  three consecutive parallel runs.
- `cargo clippy --workspace --all-targets`: **zero findings in any file this
  change touches** (`src/kit/*`, `src/generate/mod.rs`, `src/main.rs`,
  `tests/kit_contract.rs`). Three findings my own code introduced during review
  were fixed rather than recorded.
- `rustfmt --check`: clean on every file this change touches.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate --all --strict --no-interactive`: 63 passed / 0 failed.
- `git diff --check`: PASS.
- No shared Gate Runtime is configured; no Gate pass is claimed.

## The sibling is now provably read-only

This was the most serious defect found on review, and it is worth stating
plainly because the code *claimed* the opposite.

The spec says no file in the sibling checkout is written, moved or removed, and
`feed.rs` repeated the claim in its own doc comment. It was false.
`dotnet pack -o <dir>` redirects only the final `.nupkg`; it also wrote restore
assets into `<project>/obj/` and build output into `<project>/bin/`, inside the
checkout. Those paths are gitignored, so `git status` stayed clean and the
mutation was invisible to every check that looks at version control.

Proved by running it: the original invocation rewrote **6 file entries** under
`dotnet-platform-libs/src/*/obj/`.

The fix packs from a **scratch copy** of the sibling — `src/` without any
build-output directory, plus every regular file at the root — so the requirement
is true by construction. Redirecting MSBuild's output roots was implemented
first and **rejected on evidence**: it does not work, because the default
`**/*.cs` glob still reads the sibling's own `obj/`, and on a sibling carrying a
stale `net8.0` output the build compiles two copies of the same generated
assembly attributes and fails.

After the fix, a full pack of all nine packages leaves the sibling
**byte-identical across all 10,051 files** under `src/`.

## Defects found and fixed on review

The package's 25 tasks were all ticked before this review. Four of them were
ticked against something that did not hold up:

1. **The explicit kit upgrade had no operator entry point.** `diff_kit_snapshot`
   and `upgrade_kit_snapshot` existed and were tested, but `forge kit`
   implemented only `pack` and `verify`. The spec scenario is "*when an
   operator runs* the explicit kit upgrade"; its only caller was `#[cfg(test)]`.
   A capability nobody can invoke is not an explicit path. Now `forge kit
   upgrade <path> --to <kit@version>`, review-only by default, applied on
   `--confirm`.
2. **The upgrade did not move the declared pin.** It rewrote the owned files
   and the receipt but not `kit.version` in the project's `forge.yaml` — and
   `forge kit verify <path>` deliberately reads what the project *says* it pins.
   A successful upgrade therefore produced a project that failed its own drift
   gate. Now recorded as a targeted line edit, never a YAML round-trip, reusing
   the generator's own `yaml_scalar` so the escaping cannot drift.
3. **The source-mode refusal was dead code.** `refuse_source_reference` existed
   and `FeedRejection::SourceReference` was never constructed outside a test;
   `validate_feed_value` could not produce it. "Forge SHALL refuse a source-mode
   project reference" was unimplemented while looking implemented. Now
   `find_source_reference` scans every rendered manifest before anything is
   staged. A reference that stays *inside* the generated project is deliberately
   allowed — a solution with its own test project is portable.
4. **Two tests asserted nothing.** The pattern-catalog colour check searched
   for the literal string `#[0-9a-fA-F]{6}`, which no hex colour contains. The
   completeness check asserted `matches!(class, Confirmed | Provisional)` against
   a two-variant enum, so "every package is classified" could never fail. Both
   now parse and compare for real.

A fifth, smaller one: the feed's byte-level digest check was gated on
`feed_dir.starts_with(kits_dir())` with no reporting, so a generated project's
own committed feed was version-checked and the byte check was **silently
skipped** — a green result that never ran the check. `FeedVerificationReport`
now carries `digests_verified`, the digest record is a parameter
(`verify_committed_feed_with_digests`), and the CLI prints a note when the check
did not run.

## Pre-existing conditions (not regressions)

- `cargo fmt --all -- --check` used to fail on five files this change does not
  own. **This is now stale and was re-checked on 2026-10-05: the tree is
  rustfmt-clean on `main` at `b8b0fb5`, so that earlier failure has been fixed
  by intervening work rather than by this change.** The claim below is kept as
  written history: at the time, the parent-commit versions of
  `src/gate/evidence.rs`, `src/github/normalize.rs`,
  `src/portfolio/share/validation.rs` and `src/publish/fleet.rs` were already
  rustfmt-dirty, so that failure was not drift from that change.
- `scripts/release-check.sh` blocks at its first gate, `cargo fmt --check`, for
  the reason above. It never reaches its test, clippy, audit or readiness
  stages. `cargo-deny` and `cargo-audit` are both installed.
- `generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`
  **hangs indefinitely** in this environment, at 0% CPU, blocked on the cargo
  package-cache lock taken by the outer `cargo test`. **Verified pre-existing**:
  it hangs identically with this change stashed, in isolation, and was killed by
  a 240s timeout both times. It is the only test excluded from the run above.

## Verification gaps left open

- `packing_leaves_the_sibling_checkout_byte_identical` is `#[ignore]`d, and the
  reason is measured rather than guessed. A real pack is ~15 seconds of heavy
  parallel CPU; in a whole-suite run that load pushed
  `governance_contract::valid_external_response_is_normalized_and_redacted` past
  its 5-second external-adapter timeout (`left: Unavailable, right: Pass`) and
  turned an unrelated green suite red. The control run — same tree, that test
  skipped — is 2291 passed / 0 failed, and the pack test passes on its own, so
  the assertion is sound and the interference is real. Run it explicitly:

  ```sh
  cargo test --test kit_contract packing_leaves_the_sibling_checkout_byte_identical -- --ignored
  ```

  Last explicit run: **passed**.
- The floor-refusal scenario ("WHEN a floor refusal occurs … the destination
  directory, the staging area and the project registry are byte-identical") is
  proven for a real pre-staging refusal through the CLI, but **no shipped
  profile has an unmet floor**, so that exact trigger is unreachable from the
  command line. The typed refusal itself is covered. Closing it properly needs a
  way to induce an unmet floor end to end, which is a product decision.
- `tests/workspace_metadata_contract.rs::opt_out_is_byte_identical_to_pre_release`
  pins the exact bytes `--no-workspace-metadata` produces, per profile. All six
  digests were re-captured because this change alters what a scaffold contains
  by design. The guard's intent is unchanged and it still fails on future drift.
- `forge new`'s `notes` field was already asserted empty for a fully mapped
  profile. A declared-zero warning is not an omission note, so it moved to a new
  additive `kit_warning` field rather than overloading `notes`.
- `tests/generate_contract.rs::dotnet_scaffold_builds_offline_without_forge`
  asserted that an `aspnet-web` scaffold builds with **no** `PackageReference` at
  all. That was true before and is false now, by design. The test is left intact
  and still asserts a successful `dotnet build`; it now needs a resolvable feed,
  which the default test environment does not provide. The equivalent assertion
  added by this change, `the_generated_dotnet_project_operates_without_forge`,
  reports `unverified` rather than a pass when no feed is configured.

## Why a branch appeared

The work was committed on `feat/scaffold-prewires-shared-layer` rather than
straight onto `main`, contrary to the owner's standing direction.

**At the time, AGENTS.md contained no branch instruction.** Its history then was
three commits (`5a2d272`, `d069d59`, `290832f`), and neither AGENTS.md nor
`.ai-rules/` nor README.md mentioned "branch" or "main" anywhere. So the rule the
owner believed was written down was not in the repository, and the previous agent
was not working from a written instruction to override.

The most likely driver is the agent harness rather than the repository: the
default commit guidance in this environment is "if on the default branch, branch
first", which fires even when a project says nothing. The reflog shows the
branch created at `2420e2c` and the single commit landing on it, with `main`
left at `2420e2c`.

Corrective action taken: `main` was fast-forwarded to the commit, the branch
deleted, and all subsequent work committed on `main`. Nothing was lost — the
branch tip and `main` are the same commit.

**The rule has since been written down.** `AGENTS.md` now carries a "Git workflow"
section: work on `main`, no branch or worktree unless the owner explicitly asks,
never push, stage explicit paths and never `git add -A`. That closes the gap this
section recorded; the rest of it stays as the reason the rule is stated where an
agent will read it before committing.

## Native evidence (real, not claimed)

| Profile | Result |
|---|---|
| `aspnet-web` | `dotnet restore` + `dotnet build` on `net10.0` succeeded, 0 warnings / 0 errors, at a path the project was not generated at, with `NUGET_PLATFORM_FEED` unset and no sibling present — the feed bytes are committed in the project |
| `react-web` | `npm run build` and `npm test` offline succeeded; vendored `node .platform/tokens/verify-tokens.mjs` passed |
| `nextjs-web` | `npm run build` and `npm test` offline succeeded |
| `rust-web` | `cargo build --offline` succeeded |
| `flutter-app` | `flutter analyze` — "No issues found!"; `flutter test` — all tests passed |
| `python-service` | rendered; not built |

The portability proof, which is the oracle for the owner's requirement: the
global NuGet package cache was emptied of all 59 `Platform.*` entries first, a
fresh `aspnet-web` scaffold was rendered, copied to an unrelated path, and
restored and built there with the feed variable removed and no sibling library
present. All nine `Platform.*` libraries resolved at `0.1.0` into the emptied
cache, and `obj/project.assets.json` lists exactly 9. The control experiment —
the same project with its committed `packages/` deleted — fails with
`error NU1301`, naming the relative feed as the missing local source. So the
committed bytes are what supply the packages: not the cache, not a sibling, not
an environment variable.
