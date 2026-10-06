# Tasks: forge-identity-password-management

## BFS baseline

- [x] 1.1 Map `identity::global` (setup/authenticate/session tables), `cmd_identity`, `read_secret`, and the Argon2id/rand dependency availability.
- [x] 1.2 Record baseline `cargo build` and focused `identity` test results.

## DFS requirement implementation

- [x] 2.1 Implement `global::revoke_all_sessions` and `global::change_password` with the existing strength/confirmation policy and a no-administrator refusal.
- [x] 2.2 Implement `forge identity change-password` (double hidden prompt, mismatch refusal, session revocation, no echo).
- [x] 2.3 Implement `forge identity generate-password --length` (OS entropy, bounded length, default meets policy, registry-free, printed once).

## BFS regression and completeness

- [x] 3.1 Verify change success re-hashes and old password stops authenticating while the new one works; email unchanged.
- [x] 3.2 Verify weak/mismatched input and change-before-setup leave hash and sessions untouched; a successful change revokes outstanding sessions.
- [x] 3.3 Verify generated length/charset/bounds and that generation never touches the registry.

## Verification

- [x] 4.1 Add unit/contract coverage in `src/identity/global.rs` tests and the identity contract test; run `cargo fmt --check`, `cargo build`, `cargo test --lib identity::global`, `cargo test --test identity_contract`, name preflight, `openspec validate --all --strict --no-interactive`, `git diff --check`.
- [x] 4.2 Archive with promoted specs and update the HANDOFF pointer.
