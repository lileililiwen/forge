# Design: Remove the in-process /ui HTML portal

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge` crate (library + binary). No sibling touched.
- **Modules deleted:**
  - `src/api/ui/` (entire directory: `mod.rs`, `auth.rs`, `data.rs`, `render.rs`, `routes.rs`, plus `static/`).
- **Files modified:**
  - `src/api/mod.rs`: drop any `use crate::api::ui::*;` and any registration that mounted the `/ui/*` routes.
  - `src/lib.rs`: drop `pub mod api::ui;` (if it is declared there — verify during implementation).
  - `tests/identity_contract.rs`: drop the OIDC round-trip cases that hit the deleted routes.
- **Tests deleted:**
  - `tests/portal_ui_contract.rs` (1,431 lines)
  - `tests/portal_browser_a11y.rs`
  - `tests/forge_portal_frontend_contract.rs`
  - The `/ui/*` cases in `tests/portal_contract.rs` and
    `tests/portal_cross_surface.rs` (kept cases are the
    non-`/ui/*` ones; if none remain, the file is
    deleted outright).
- **Specs deleted from `openspec/specs/`:** the three
  specs named in `proposal.md` § What Changes.

## 2. Language and runtime

- Rust 2021, `rustc 1.87` floor. No new language features.
- The OIDC machinery in `src/identity/` stays — its
  only consumers in the deleted `src/api/ui/auth.rs`
  are gone, but its CLI consumers
  (`forge identity challenge / callback / list / inspect /
  terminate`) and the OIDC round-trip cases in
  `tests/identity_contract.rs` stay.

## 3. Ownership and shared code

`src/api/ui/` is exclusively consumed by:
- The `forge api serve` listener in `src/api/mod.rs` (one
  route table entry).
- The deleted test files.

The OIDC machinery in `src/identity/` is shared with the
`forge identity` CLI subcommands and
`tests/identity_contract.rs`; it stays.

## 4. User experience and interface

- `forge web serve` (port 4173) — unchanged.
- `forge api serve` (port 8765) — `/ui/*` returns 404;
  every `/v1/...` route is unchanged.
- `forge portal dashboard` / `forge portal view` — unchanged.
- `forge identity init` / `validate` / `challenge` /
  `callback` / `list` / `inspect` / `terminate` — unchanged.

## 5. Behavioral model

- A browser that hits `/ui` on the API listener sees 404.
- The CLI subcommands behave exactly as before.
- A `forge.yaml` carrying an `identity:` block parses as
  before; the `forge identity validate` subcommand
  reports the block's contents exactly as before.

## 6. Contract and compatibility

- JSON API contract (`src/api/mod.rs` minus the deleted
  routes): unchanged.
- Registry schema: unchanged.
- Journal schema: unchanged.
- `forge.yaml` schema: unchanged.
- CLI surface: unchanged (only the deleted HTTP routes
  are removed; the CLI subcommands are all kept).

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| `GET /ui/*` on the API listener | 404 |
| `forge portal dashboard` | Unchanged (the CLI is kept) |
| `forge identity challenge` | Unchanged (the OIDC CLI stays) |

## 8. Verification oracle

- `cargo build --workspace` — 0 errors.
- `cargo test --workspace` — every test that survives the
  deletion is green; deleted tests are no longer in the
  output.
- `forge web serve` on port 4173 — the web UI loads and
  its API calls (which hit port 8765) return the same
  data as before.
- `forge api serve` on port 8765 — `/healthz` returns
  200; `/v1/...` routes return the same data; `GET /ui/*`
  returns 404.
- `forge portal dashboard` — unchanged behavior.
- `forge identity validate` — unchanged behavior.

## 9. Decision ledger

- **Resolved:** the in-process HTML portal at `/ui/*` is
  removed; the standalone web UI is the only browser path.
- **Resolved:** the OIDC machinery and CLI stay; only
  the HTTP routes that lived in `src/api/ui/` are removed.
- **Resolved:** the `forge portal` CLI stays.
- **Resolved:** the global admin password surface stays.
- **Blockers:** none.
