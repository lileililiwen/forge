# github-project-metadata-adapter (delta)

## ADDED Requirements

### Requirement: gh-backed observe fallback when the adapter binary is missing

When no GitHub metadata adapter binary is configured but the user's `gh` CLI is available, Forge SHALL observe repositories through a vendored read-only `gh repo view --json` path and synthesize the same versioned observation shape, SHALL label the envelope `adapter_source=gh-cli-fallback`, and SHALL keep every other adapter behavior unchanged.

#### Scenario: Observe via gh when the adapter is absent

- **WHEN** `forge project github observe <owner/repo>` runs with no adapter binary on `PATH`/`FORGE_GITHUB_BIN` and an executable `gh` on `PATH`/`FORGE_GH_BIN`
- **THEN** Forge runs the bounded `gh repo view <owner/repo> --json nameWithOwner,description,repositoryTopics,defaultBranchRef,isArchived,primaryLanguage` read, returns a `current` observation with topics/description/default-branch/archived/language, and reports `adapter_source=gh-cli-fallback`

#### Scenario: No adapter and no gh stays unavailable

- **WHEN** neither the adapter binary nor `gh` is available
- **THEN** Forge returns the original typed `error[github-adapter-unavailable]` and performs no write

#### Scenario: Fallback never needs the adapter token

- **WHEN** the fallback path runs without `FORGE_GITHUB_TOKEN` set
- **THEN** the observation still succeeds off `gh` auth and no unauthenticated adapter request is attempted

### Requirement: gh-backed direct single-topic propose with confirm echo

When no adapter binary is configured but `gh` is available, Forge SHALL accept a direct-mode single-`topic=` propose with a non-empty `--confirm` token, SHALL apply it via `gh repo edit <owner/repo> --add-topic <topic>`, SHALL verify the confirmation locally without logging its value, and SHALL leave pull-request mode and every other field/mode on the adapter-only path.

#### Scenario: Direct topic propose via gh

- **WHEN** `forge project github propose <owner/repo> --mode direct --confirm <token> --set topic=<value>` runs with no adapter binary and an executable `gh`
- **THEN** Forge runs `gh repo edit <owner/repo> --add-topic <value>`, returns `mode=direct state=current`, and the token value appears in no note, log, or JSON envelope

#### Scenario: Direct propose without confirmation is refused

- **WHEN** direct mode is requested without a non-empty `--confirm` token
- **THEN** Forge refuses with `error[github-invalid]` and performs no `gh` mutation

#### Scenario: PR mode still requires the adapter

- **WHEN** `--mode pull-request` (default) is requested with no adapter binary
- **THEN** Forge returns `error[github-adapter-unavailable]` even when `gh` is installed, and performs no write

#### Scenario: Credentials never reach a report

- **WHEN** any fallback observe/propose output is rendered
- **THEN** the token and any credential-shaped value are absent and redacted
