# Design: Self-service administrator password management

## Ownership and boundaries

- `src/identity/global.rs` owns the single `forge_admin` row and the
  `forge_admin_sessions` table in the Forge registry database. Both new
  operations live here so hashing, validation and session state stay in one
  place.
- `src/main.rs::IdentityCommands` / `cmd_identity` own the CLI surface and the
  TTY `read_secret` prompt, exactly as `Setup` does today.
- No new module, no new dependency: `argon2` and `rand` are already in the
  workspace manifest.

## Contracts

- `global::change_password(db_path, password) -> Result<(), String>`:
  validates the password with the same rule as `setup` (12–1024 chars),
  requires an existing administrator row (else a named refusal), re-hashes with
  Argon2id, and updates the singleton row's `password_hash` in place. On success
  it calls `revoke_all_sessions` before returning, so the rotation is atomic
  from the operator's point of view.
- `global::revoke_all_sessions(db_path) -> Result<(), String>`: sets
  `revoked = 1` for every not-yet-revoked session row.
- CLI `change-password`: reads the new password twice via `read_secret`, refuses
  a mismatch, delegates to `change_password`, and prints a confirmation without
  the value.
- CLI `generate-password [--length <n>]`: default length 20 (comfortably above
  the 12-char minimum), accepted range 12–128. Builds the password from
  `rand::thread_rng()` (OS-backed) over an unambiguous printable alphabet, and
  prints it once. It never opens the registry.

## Failure and security decisions

- A failed validation or a mismatched confirmation writes nothing and revokes
  nothing — the current password and sessions stay valid.
- Sessions are revoked on a *successful* change only, closing the window where
  an old stolen cookie would outlive a rotated password.
- Generated and entered passwords are never logged, never placed in argv, and
  never echoed; `generate-password` output is intended to be pasted, and the
  operator is told it is shown once.
- The alphabet excludes look-alike characters (`0/O`, `1/l/I`) to keep pasted
  passwords reliable, while remaining a superset of the policy's needs.

## Migration

Purely additive verbs and functions; no schema change and no data migration.
`change_password` requires the table `setup` already creates; against a registry
with no administrator it refuses rather than silently creating one.
