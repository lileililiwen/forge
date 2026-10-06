# Proposal: Self-service administrator password management

## Why

The delivered global admin portal offers only one-time `forge identity setup`,
which refuses to run twice. Once an administrator exists there is no supported
way to rotate or replace the password, so an operator who cannot use the
initial password is locked out and the only "fix" is deleting the registry row
by hand. There is also no way to mint a strong password that satisfies the
12–1024 character policy without leaving it in shell history or a file.

## What Changes

- Add `forge identity change-password`: non-destructive rotation of the single
  administrator password, reading the new value twice without echo, applying the
  existing strength/confirmation policy, re-hashing with Argon2id, and revoking
  all outstanding sessions.
- Add `forge identity generate-password`: prints one OS-entropy random password
  of a bounded, configurable length, touching no registry and requiring no TTY.
- Extend `forge::identity::global` with `change_password` and
  `revoke_all_sessions`; reuse the existing Argon2id hashing and validation.

## Capabilities

- `forge-admin-login` (modified): two new self-service requirements.

## Non-goals

- No email change, no multi-user accounts, no self-service sign-up, no password
  reset by email, no per-project credential management, and no change to the
  browser login/session API contract. The web UI still has no password form;
  rotation stays a CLI operator action.
