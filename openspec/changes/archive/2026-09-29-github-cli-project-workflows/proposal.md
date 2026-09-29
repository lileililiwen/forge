# Proposal: Reuse the user's authenticated GitHub CLI

## Why

Forge already has a GitHub metadata adapter, but its documented configuration requires `FORGE_GITHUB_TOKEN`. The user already signs in with `gh`; requiring a second token setup adds friction and duplicates authentication.

## What Changes

- Add bounded local repository operations through the installed GitHub CLI: check its auth state, clone or attach a repository, create a private repository for a project, and open a pull request.
- Keep local Git operations in `git`; use `gh` only for GitHub-hosted operations.
- Require explicit confirmation for remote writes and never read, persist, print, or forward the token from the `gh` credential store.

## Package Boundary and Split Assessment

This package is only the Forge-to-`gh` local process adapter. It does not change the existing GitHub metadata contract or the remote deployment provider. The separate publish workflow consumes this package as an optional repository step.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `github-cli-project-workflows` | Operate on GitHub repositories using the user's existing `gh` session | Forge, Rust | fixed `gh` argument vectors and typed outcomes | local project registry and installed `gh` | fake-process suite plus optional authenticated read-only smoke |
| `project-to-production-workflow` | Guide a reviewed project through staging and promotion | Forge, Rust | existing Forge publish journal/provider protocol | this package, OpenPanel provider, Hermora onboarding | staged lifecycle conformance |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge GitHub metadata | `src/github/adapter.rs`, `src/github/normalize.rs` | bounded external process, typed GitHub outcomes, redaction, repository discovery | separate metadata adapter currently requires `FORGE_GITHUB_TOKEN`; it is not a user repository workflow | Forge | **extend shared owner** without replacing its contract |
| Forge local Git | `src/` project inspection and existing publish revision handling | repository path and revision validation | no `gh`-based repository setup/PR flow | Forge | **extend shared owner** |
| Workspace Governance | `projects.json`, registry metadata | repository inventory and declarations | does not execute GitHub operations | Workspace Governance | **adapt through a generic adapter** |
| GitHub auth | user's installed `gh` configuration | existing login, hosts, scopes, and credential storage | must remain on the local Forge execution host | GitHub CLI owns credentials | **adopt** |

## BFS Impact Map

- **Commands:** `forge project github auth|clone|create|pull-request` with explicit project/repository arguments.
- **State:** project remote is unset/connected; remote write operation is planned/confirmed/succeeded/failed. Existing project registry remains authoritative for local project identity.
- **Security:** invoke `gh` using fixed subcommands and an argument array; no shell interpolation, `gh auth token`, token env export, or token persistence. Repository creation defaults private. Public visibility requires a separate explicit confirmation.
- **Failures:** `gh` missing, not authenticated, insufficient scope, rate limited, remote not found, dirty worktree, existing origin, timeout, or push failure is reported with a bounded typed reason and recovery action.
- **Compatibility:** existing GitHub metadata adapter remains supported; its token-based automation mode is not silently changed.
- **Unchanged:** no repository is created, cloned, pushed, or made public without an explicit operation confirmation.

## Capabilities

- **New:** `github-cli-project-workflows`.

## Non-goals

- No GitHub login form, OAuth app, personal-token wizard, or token scraping.
- No automatic commits, pushes, repository visibility changes, or PR merges.
- No hosted/server-side `gh` execution in this package; it is a local developer-host capability.
- No change to OpenPanel or Hermora deployment/management state.
