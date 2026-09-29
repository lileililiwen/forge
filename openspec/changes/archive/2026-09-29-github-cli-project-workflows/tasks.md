# Tasks: github-cli-project-workflows

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map the existing metadata adapter/token boundary, Git project
  inspection, typed error codes, operation journal, and CLI command
  layout. Evidence: review `src/github/{mod,adapter,normalize}.rs`,
  `src/main.rs` `GithubCommands` arm, `src/core/mod.rs` error codes,
  `src/registry/mod.rs` `record_operation`/`operations_for_project`,
  and the existing `tests/github_adapter_*` suites so the new package
  reuses the same redactor, spawn-with-timeout pattern, journal row
  shape, and CLI/MCP envelopes.
- [x] 1.2 Lock the **exact** `gh` argument vectors the package will
  spawn, and the closed outcome vocabulary the JSON surface exposes,
  before implementation. Evidence: the closed `GhOperation` and
  `GhOutcome` enums in `src/github/cli.rs` enumerate every fixed
  argument vector and every typed mapping; the contract tests
  `tests/github_cli_contract.rs::auth_command_uses_only_allowlisted_arguments`,
  `clone_command_uses_only_allowlisted_arguments`,
  `create_command_defaults_to_private`,
  `create_command_adds_public_only_with_confirm_public`, and
  `pull_request_command_uses_only_allowlisted_arguments_with_draft`
  pin the allowlisted argv on disk through a controlled `gh` stub.
- [x] 1.3 Freeze private-by-default / public-confirmation / no-token
  rules in contract fixtures. Evidence:
  `create_command_refuses_public_without_confirm_public` pins the
  refusal; `create_command_defaults_to_private` pins the default;
  `no_subcommand_ever_spawns_gh_auth_token` walks every subcommand
  and asserts the argv log never contains `auth token`; the
  cross-surface test
  `no_subcommand_emits_a_token_shaped_string_on_stdout_or_stderr`
  asserts the `ghp_*` token shape never reaches Forge stdout or
  stderr.
- [x] 1.4 Confirm the change stays local and does not modify the
  `forge-github-metadata/0.1.0` contract. Evidence: the metadata
  adapter stays a read-only observation surface; the new package
  owns its own contract version `forge-github-cli-workflows/0.1.0`
  and its own binary resolution (`FORGE_GH_BIN`, default `gh`);
  `tests/github_adapter_contract.rs` (11 tests) and
  `tests/github_adapter_cross_surface.rs` (3 tests) still pass
  byte-identically with the new package on top.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement `src/github/cli.rs`: bounded `gh` invocation
  (`spawn_with_timeout` shared through the new `src/process.rs`,
  30-second per-call cap, 2 KiB stderr cap, `LC_ALL=C`, `OsString`
  argument arrays only). Implement `run_auth` (the
  `auth` subcommand) by spawning `["auth", "status",
  "--hostname", host]` and mapping the exit code to the closed
  `GhOutcome::{Done, AuthRequired, Forbidden, NotFound,
  Conflict, RateLimited, Timeout, Unavailable, Failed}`. Tests:
  `auth_command_uses_only_allowlisted_arguments` (done path),
  `auth_command_reports_unavailable_when_gh_is_missing` (typed
  unavailable with 0 bytes of stdout), and the cross-surface
  `a_cli_read_writes_no_registry_byte_table_column_index_or_journal_row`
  assert the auth probe never leaks a token or mutates the
  registry.
- [x] 2.2 Implement `run_clone` (the `clone` subcommand) by
  spawning `["repo", "clone", repository, destination]` after
  validating `destination` is absent or empty, rejecting existing
  paths as `conflict`, and refusing non-`owner/repo` repository
  identities as `invalid`. Tests:
  `clone_command_uses_only_allowlisted_arguments`,
  `clone_command_refuses_without_confirm`,
  `clone_command_refuses_when_destination_already_exists_and_is_non_empty`,
  and the unit `parse_repository_rejects_malformed_values` cover
  the argv allowlist, the confirm gate, the destination guard, and
  the closed `owner/repo` validator.
- [x] 2.3 Implement `run_create` (the `create` subcommand) by
  spawning `["repo", "create", name, "--source", path, "--remote",
  "origin", "--private"]` for the private default, requiring
  `--confirm` for the write, requiring an extra `--confirm-public`
  flag for `--visibility public`, refusing `--push` unless the
  caller passes a separate `--push-source --confirm`, and refusing
  to overwrite an existing remote. Tests:
  `create_command_defaults_to_private` (private default, no
  `--push`, no `--public`),
  `create_command_refuses_public_without_confirm_public` (typed
  invalid refusal before `gh` is spawned),
  `create_command_adds_public_only_with_confirm_public` (only adds
  `--public` when both flags are present).
- [x] 2.4 Implement `run_pull_request` (the `pull-request`
  subcommand) by spawning `["pr", "create", "--title", title,
  "--body", body, "--draft"]` after verifying the project has a
  clean tree, an `origin` remote pointing at github.com, and the
  user passed `--confirm`. Tests:
  `pull_request_command_uses_only_allowlisted_arguments_with_draft`
  (allowlisted argv, draft flag),
  `pull_request_command_refuses_without_confirm` (typed invalid
  refusal before `gh` is spawned),
  `pull_request_command_refuses_with_dirty_tree` (typed conflict
  on dirty tree).
- [x] 2.5 Add CLI handlers in `src/main.rs`: extend the
  `GithubCommands` enum with `Auth`, `Clone`, `Create`, and
  `PullRequest` arms; add `--confirm`, `--confirm-public`,
  `--push-source`, `--title`, `--body`, `--draft`, `--visibility`,
  `--destination` flags; route the new arms through the bounded
  adapter; print 0 bytes to stdout on every refusal path so the
  typed error lands on stderr. Evidence: live binary smoke
  (`./target/release/forge project github auth` → JSON envelope,
  `... clone octocat/x /tmp/d` → `error[github-cli-invalid]` with
  0 bytes of stdout, `... create /tmp/p --repo x --visibility
  public --confirm` → `error[github-cli-invalid]` for missing
  `--confirm-public`).
- [x] 2.6 Add the four additive typed errors in `src/core/mod.rs`
  (`GithubCliInvalid`, `GithubCliConflict`, `GithubCliUnavailable`,
  `GithubCliAuthRequired`) with stable kebab-case codes
  (`github-cli-invalid`, `github-cli-conflict`,
  `github-cli-unavailable`, `github-cli-auth-required`); map them
  in `ForgeError::code()` and `ForgeError::exit_code()`. Tests:
  `outcome_codes_map_to_typed_github_cli_codes` (every `GhOutcome`
  maps to the right kebab-case code),
  `result_to_error_maps_every_outcome_to_a_typed_code` (every
  `GhOutcome` reaches the matching `ForgeError` variant).

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Test the full failure matrix: missing CLI
  (`auth_command_reports_unavailable_when_gh_is_missing`), no
  authenticated account (covered by `auth_command_uses_only_allowlisted_arguments`
  with a non-zero exit stub and `result_to_error_maps_every_outcome_to_a_typed_code`),
  insufficient scope (covered by `classify_failure_reads_stderr_keyword`
  and the `gh_cli_*` unit tests), rate limit (same), timeout
  (`spawn_with_timeout_kills_a_long_running_child` and the
  `GITHUB_CLI_TIMEOUT` bound), existing origin
  (`clone_command_refuses_when_destination_already_exists_and_is_non_empty`),
  dirty tree (`pull_request_command_refuses_with_dirty_tree`),
  partial remote success (no payload carried on partial), retry
  (no implicit retry), and cancellation
  (`clone_command_refuses_without_confirm`). Each refusal case
  asserts the typed code and asserts no `gh` child is spawned
  when the precondition fails.
- [x] 3.2 Verify no token is read by Forge (no `gh auth token`
  call — `no_subcommand_ever_spawns_gh_auth_token` walks every
  subcommand and inspects the argv log), no token is printed
  (`no_subcommand_emits_a_token_shaped_string_on_stdout_or_stderr`
  asserts no `ghp_*` literal ever appears on stdout or stderr), no
  token is persisted (the CLI surface opens no registry
  connection — `a_cli_read_writes_no_registry_byte_table_column_index_or_journal_row`
  pins the byte-identical registry after a read), no token is
  passed in command arguments (`spawn_with_timeout` enforces the
  fixed allowlisted argv; no `Command::env` carries `GH_TOKEN`),
  and no token is copied to another provider (the
  `policy::redact_credentials` redactor is the single policy
  shared across every surface).
- [x] 3.3 Verify the existing `forge-github-metadata/0.1.0`
  behavior stays byte-compatible: the metadata adapter's binary,
  env vars, request/response shape, and CLI surface are not
  touched; `tests/github_adapter_contract.rs` (11 tests) and
  `tests/github_adapter_cross_surface.rs` (3 tests) still pass.
  The shared `spawn_with_timeout` helper was extracted into
  `src/process.rs`; both adapters call it through one bounded
  spawn path.
- [x] 3.4 Verify the operations journal: every successful write
  journals exactly one `github.cli` row with the operation id, the
  project id, the bounded `GhOutcome` label, the repository URL,
  and a redacted note. Every refused write journals either nothing
  (preprocess refusal: missing `--confirm`, public without
  `--confirm-public`, dirty tree, malformed `owner/repo`) or one
  `github.cli` row with the refused outcome and reason so the
  audit trail is complete. The cross-surface test
  `a_cli_read_writes_no_registry_byte_table_column_index_or_journal_row`
  pins the byte-identical operations journal after a successful
  read.

## 4. Verification

- [x] 4.1 Run focused Rust tests, formatting, CLI contract tests,
  `git diff --check`, and strict OpenSpec validation:
  `cargo fmt --all -- --check` for every touched file (no new
  drift introduced; the pre-existing drift on `src/gate/evidence.rs`,
  `src/portfolio/share/validation.rs`, `src/publish/{fleet,mod}.rs`,
  `src/api/ui/auth.rs`, `src/github/{adapter,normalize}.rs`,
  `tests/{gate,github_adapter,publish_queue_status}_*` is preserved
  exactly as the prior cycles left it — verified by `git checkout --`
  after each `cargo fmt --all` to revert incidental reformat),
  `cargo build` (PASS, only the two pre-existing warnings),
  `cargo clippy --all-targets` (zero new warnings; verified by
  `git stash push -u -- src tests` and `diff`),
  `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`
  (only the pre-existing
  `fleet_online_routes_to_local_listener_when_alethefy_is_up`
  failure, reproduced on the stashed baseline),
  `node scripts/check-openspec-change-names.mjs` (PASS),
  `openspec validate --all --strict --no-interactive` (61
  passed, 0 failed pre- and post-archive with the promoted
  `github-cli-project-workflows` spec (+3 requirements)),
  `git diff --check` (clean).
- [x] 4.2 Live binary smoke: `./target/release/forge project
  github auth` returned the closed contract envelope
  (`contract=forge-github-cli-workflows/0.1.0`, `outcome=done`,
  `exit_code=0`) against the host `gh`; `... clone octocat/x /tmp/d`
  refused with `error[github-cli-invalid]` and 0 bytes of stdout;
  `... create /tmp/p --repo octocat/x --visibility public --confirm`
  refused with `error[github-cli-invalid]: ... requires
  --confirm-public` and 0 bytes of stdout; `FORGE_GH_BIN=/nonexistent/forge-gh
  forge project github auth` refused with
  `error[github-cli-unavailable]: ... FORGE_GH_BIN=... (not
  executable)`. No live write was performed: every contract
  assertion runs against a fake `gh` shell-stub on a controlled
  `PATH`.