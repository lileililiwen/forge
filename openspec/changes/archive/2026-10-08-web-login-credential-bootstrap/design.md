# Design: Operator login credentials on `scripts/web.sh start`

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge` crate
  (library + binary) plus `scripts/web.sh`. No sibling touched.
- **Files changed:**
  - `src/identity/global.rs` — add `pub fn email(db_path) -> Result<Option<String>, String>`.
  - `src/main.rs` — `IdentityCommands::Status`; `--password-stdin` on `Setup`
    and `ChangePassword`; `read_password_stdin`; dispatch/rendering.
  - `src/api/command_catalog.rs` — one `identity.status` row; pinned count
    233 → 234 with a comment.
  - `scripts/web.sh` — `--admin-email`, `ensure_admin`, `reset-password`,
    banner printing; run-dir/binary env overrides for isolation (see §4).
  - `tests/web_login_credentials_contract.rs` — new oracle.
- **Modules reused unchanged:** `identity::global::{setup, change_password,
  generate_password, is_configured, authenticate, session_valid}`,
  `default_registry_path`, `ApiConfig`, the existing login route.
- **Must NOT change:** the `forge_admin` / `forge_admin_sessions` schema,
  `API_CONTRACT_VERSION`, any web route, any other CLI path, the interactive
  password UX.

## 2. Language and runtime

- Rust 2021, `rustc 1.87` floor. `scripts/web.sh` stays POSIX `sh` with
  `set -eu`. Linux; ports are loopback by default.

## 3. CLI contract

### 3.1 `forge identity status`

New leaf, no arguments:

- Human (default `--format human`/`table`), stdout, exactly:
  - configured: `configured: true\nemail: <lowercased-admin-email>`
  - not configured: `configured: false`
- JSON (`--format json` / `--format ndjson`): `{"contract":
  "forge-admin-login/1.0.0", "configured": <bool>, "email": <string|null>}`.
- It reuses `is_configured` + a new `email` reader; it never selects or
  prints `password_hash`.

### 3.2 `--password-stdin` on `setup` and `change-password`

Both gain `#[arg(long)] password_stdin: bool`. When set:

- read exactly one line from stdin (`read_password_stdin`), stripping a
  trailing `\r`/`\n`; EOF or an empty line is a typed `identity-invalid`
  error ("no password was provided on stdin");
- skip the second confirmation entry;
- hand the value straight to the existing Core function, where the 12–1024
  length policy still applies.

When unset, the existing `read_secret` double-entry interactive path is
byte-for-byte unchanged. No `--password <value>` flag is added; the secret
never enters `argv`.

## 4. Script contract (`scripts/web.sh`)

New parsing: `--admin-email <e>` (default `operator@example.com`), accepted
for `start`, `restart`, `reset-password` (and harmlessly ignored elsewhere).

New subcommand: `reset-password`.

New helper `ensure_admin`, called at the front of `cmd_start` (after the
binary check, before `save_state`/`start_one`):

1. `status="$("$BIN" identity status 2>/dev/null || true)"`; parse
   `configured:` and `email:` with `sed`/`head` (no `jq` dependency).
2. If `configured: true`:
   - print the login URL (`http://$WEB_HOST:$WEB_PORT/`) and the parsed
     existing email;
   - print an instruction to run `scripts/web.sh reset-password` for a new
     password;
   - print **no** password.
3. Else (fresh registry):
   - `password="$("$BIN" identity generate-password --length 20)"`;
   - `printf '%s\n' "$password" | "$BIN" identity setup --email "$ADMIN_EMAIL"
     --password-stdin`; a non-zero exit aborts `start` (non-zero);
   - print the login URL, `$ADMIN_EMAIL`, and the password, with a one-time
     warning.

`cmd_reset_password`:

1. Requires the binary (same guard as `start`).
2. Reads status. If configured, uses the stored email and
   `identity change-password --password-stdin`; if not, creates the account
   with `identity setup --email "$ADMIN_EMAIL" --password-stdin`.
3. Generates a fresh password via `identity generate-password --length 20`,
   feeds it on stdin, then prints URL + email + password once and states that
   existing sessions were revoked.

Isolation seams (minimal, documented in the header): `RUN_DIR`, `LOG_DIR`,
`WEB_ROOT_DIR` and `BIN` honour `FORGE_WEB_RUN_DIR`, `FORGE_WEB_LOG_DIR`,
`FORGE_WEB_ROOT_DIR` and `FORGE_BIN` when set, defaulting to today's values.
These let the contract test drive the real script against a throwaway run
directory without touching the operator's `.forge/run` or the currently
bound ports.

## 5. Security and no-leak rules

- The password is written only to stdout by `printf`/`echo`; it is never an
  `argv` element, never appended to `STATE_FILE`, never redirected into
  `$API_LOG`/`$WEB_LOG`, and `save_state` continues to write only the four
  host/port/root assignments.
- `identity status` never reads `password_hash`; the JSON/human shapes carry
  only `configured` and `email`.
- Errors never echo the password (`setup`/`change-password` Core errors are
  fixed strings).

## 6. Failure and boundary policy

| Case | Behavior |
|---|---|
| No administrator, fresh start | create with generated password, print once, continue |
| Administrator exists, `start` | print URL + email + reset hint, print no password |
| `reset-password`, no administrator | create with generated password, print once |
| `reset-password`, administrator exists | rotate, revoke sessions, print once |
| Password line empty / EOF on stdin | typed `identity-invalid`, non-zero, nothing persisted |
| Password under 12 chars | Core refuses (`identity-invalid`), non-zero, no row written |
| Binary missing (`scripts/web.sh reset-password`) | usage-style error, non-zero, no state change |
| `identity status` on a fresh registry | creates the DB schema, reports `configured: false`, writes no admin row |

## 7. Compatibility

- `identity setup` / `change-password` with no flag keep the exact interactive
  behavior, messages and exit codes.
- `identity generate-password` is unchanged.
- The catalog gains one CLI-only read row; the `--bin forge` parity oracle and
  the `command_catalog` pinned count are updated in the same change.
- `scripts/web.sh`'s existing flags, subcommands and state format are
  unchanged; defaults for the new env seams reproduce today's paths.

## 8. Verification oracle

`tests/web_login_credentials_contract.rs`, in the `plugins_contract.rs` /
`forge_web_command_catalog_contract.rs` house style (`forge_bin()`,
`env_remove("FORGE_REGISTRY")`, `tempfile` fixtures, `run_json`):

1. `identity status` fresh → `configured: false`; after `identity setup
   --email … --password-stdin` → `configured: true` with the email.
2. `identity setup --email … --password-stdin` succeeds with the password on
   stdin, and the same invocation never places the password in `argv` (drive
   a real login against `authenticate`; assert the process args carry no
   password).
3. `identity change-password --password-stdin` rotates: the old password no
   longer authenticates, the new one does, and a session minted before the
   change is invalidated.
4. A password under 12 characters piped on stdin is refused (non-zero,
   `identity-invalid`), leaving `configured` unchanged.
5. `scripts/web.sh reset-password` (with `FORGE_REGISTRY`, `FORGE_BIN`,
   `FORGE_WEB_RUN_DIR`, `FORGE_WEB_LOG_DIR`, `FORGE_WEB_ROOT_DIR` pointed at a
   temp dir) prints an email and a password; the password authenticates; and
   `$RUN_DIR/state` does not contain the password.
6. `scripts/web.sh start` on a fresh registry prints the account and a
   password; after the account exists it prints the email and no password.
   The test fixes free ports, starts the real services, asserts the banner,
   and calls `stop`.

Existing suites that must stay green: `cargo test --lib command_catalog`,
`cargo test --bin forge`, `plugins_contract`, `catalog_contract`,
`classify_derive_contract`, `classify_apply_contract`,
`forge_web_command_catalog_contract`, `forge_web_maintainer_surface_contract`.

## 9. Decision ledger

- **Resolved:** add `--password-stdin`, not `--password`, so the secret never
  reaches process arguments or shell history.
- **Resolved:** `identity status` is CLI-only (reads the local registry file);
  it gets an honest `cli_only` catalog row.
- **Resolved:** `start` never resets an existing account; the explicit
  `reset-password` verb is the only path that rotates, and it is documented as
  session-revoking.
- **Resolved:** the script parses the trivial human status lines rather than
  requiring `jq`; `--format json` remains available for other tooling.
- **Blockers:** none.
