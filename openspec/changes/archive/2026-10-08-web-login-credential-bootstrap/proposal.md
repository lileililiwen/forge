# Proposal: Operator login credentials on `scripts/web.sh start`

## Why

An operator runs `scripts/web.sh start` and cannot log into the dashboard
because no Forge administrator is configured, or the password is unknown and
unrecoverable. The store keeps only an Argon2id hash (`forge identity
global`, table `forge_admin`), so an existing password can never be printed.
Today the operator must already know the internal sequence —
`forge identity generate-password`, `forge identity setup --email …` — and
even then `setup` and `change-password` read the password from a terminal and
refuse when stdin is not a terminal, so the start script cannot drive them.
The start command must end that: bring up the services and print the login
URL, the account email, and — when it just created or reset the account — the
password, so the operator can sign in immediately.

## What Changes

- Add `forge identity status`, a read-only verb that reports `configured`
  and the administrator email (never the password) in both human and JSON
  form. Human output is two trivially parseable lines: `configured: true` /
  `email: x@y`, or the single line `configured: false`.
- Add `--password-stdin` to `forge identity setup` and `forge identity
  change-password`: read exactly one line from stdin, skip the interactive
  confirmation, and never accept the secret from argv. The interactive
  terminal path is unchanged.
- Extend `scripts/web.sh` with `--admin-email` (default
  `operator@example.com`), credential bootstrap inside `start`, and a new
  `reset-password` subcommand. `start` ensures an administrator exists before
  the services bind and prints the login URL, the account email, and the
  password when it created the account; when an account already exists it
  prints the URL and email and points at `reset-password` instead of
  inventing a password. `reset-password` generates a fresh password, sets it
  non-interactively, prints email + URL + password once, and documents that
  it revokes existing sessions.
- Add the `identity.status` row to `src/api/command_catalog.rs` so the CLI
  parity oracle stays green, and bump the pinned row count with a comment.
- New `tests/web_login_credentials_contract.rs` drives the real binary and
  the real script: status fresh/configured, `--password-stdin` setup without
  the secret in argv, rotation invalidating a prior session, short-password
  refusal, `reset-password` printing once without writing the password to
  `.forge/run/state`, and `start` bootstrap behavior.

## BFS Impact Map

- **Capabilities:** `web-operator-credential-bootstrap` (new); `forge-admin-login`
  (extended: non-interactive input + status).
- **Users / flows:** the operator runs one command and gets a working login.
- **Contracts / data / persistence:** no schema change. `forge_admin` and
  `forge_admin_sessions` are reused unchanged; `change-password` already
  revokes every session, so rotation stays session-invalidating.
- **Integrations / configuration:** new script flag `--admin-email`; no new
  environment key. The script relies on `forge identity *` and `forge api
  serve` both resolving `default_registry_path` when no `--registry` is
  passed (verified: `src/main.rs` builds one `db_path` for both).
- **Callers:** `scripts/web.sh`; `src/main.rs` identity dispatch; the catalog
  row; the new contract test.
- **Failure / boundary behavior:** `start` never prints a password it did not
  set; a short/generated-out-of-bounds password is refused by the existing
  Core validation and surfaces as a non-zero script exit; `reset-password`
  refuses cleanly when the binary is missing; a `start` when an account
  already exists never resets it implicitly.
- **Tests:** `tests/web_login_credentials_contract.rs` (6 tests).
- **Privacy / security:** the password is printed to stdout once, never to
  `argv`, never to `.forge/run/state`, never to logs; `identity status` never
  reads or prints the hash.

## Capabilities

- `web-operator-credential-bootstrap`: `scripts/web.sh start` /
  `reset-password` ensure an operator login and print the credentials the
  operator needs, exactly once.

## Non-goals

- No `--password <value>` flag (it would leak the secret into argv/history).
- No attempt to recover or print an existing password.
- No new browser surface, route, or registry/journal schema.
- No change to the interactive `identity setup` / `change-password` UX.
- No implicit reset on `start` when an account already exists.
