# Design: github-project-metadata-adapter

Status: implementation-ready planning package. No code is written by this
change, and no GitHub request is made by the package.

## Implementation boundary

Repository `forge`, Rust 1.87+, existing provider/governance stack.

Files to **add**:

- `src/github/mod.rs` — module root and re-exports.
- `src/github/adapter.rs` — the bounded, argument-array invocation of the
  adapter executable and its versioned request/response contract.
- `src/github/normalize.rs` — mapping of GitHub values into the catalog record
  while retaining source, revision and freshness.
- `tests/github_adapter_contract.rs`, `tests/github_adapter_cross_surface.rs`.

Files to **change** (additive only):

| File | Change |
|---|---|
| `src/lib.rs` | `pub mod github;` |
| `src/catalog/mod.rs` | accept the GitHub adapter as one optional source |
| `src/main.rs` | `--source github` selection plus `forge project github observe|propose` where the adapter is configured |

Do **not** touch: local project source, deployment, CI execution, release
publication, or any Forge deployability verdict. GitHub is optional.

## Language and runtime

Rust 1.87+, `rustfmt` defaults, no new dependency; the adapter is invoked as an
external executable through the existing bounded-process pattern (argument
array, `LC_ALL=C`, credential redaction, bounded timeout). Commands: `cargo test
--lib -- github`, `cargo test --test github_adapter_contract`, `cargo clippy
--all-targets -- -D warnings`.

## Ownership and shared code

Forge owns the adapter contract and normalization; GitHub owns the API and its
data. The package **extends** the existing provider selection/redaction
machinery and **adapts** through a generic executable so another Git host can
replace GitHub without changing Core. Tokens are referenced from the
environment, never stored in a manifest or printed, exactly as the existing
credential boundary requires.

## Behavioral model

`forge-github-metadata/0.1.0`.

```rust
pub enum GithubState { Current, Stale, Unavailable, Unauthorized, Forbidden, NotFound, RateLimited }
```

| Operation | Behaviour |
|---|---|
| Observe | read description, topics, languages, default branch, archived, workflows, releases, tags, custom properties |
| Normalize | each value becomes a catalog record field with `source = github`, its revision and freshness |
| Propose (PR mode) | an approved change becomes a reviewable pull request; the default mode |
| Direct mode | refused unless an explicit separate mode plus confirmation is supplied |

Rate-limit, pagination and partial responses are represented as states, not as
truncated success. GitHub topics, release tags and Forge portfolio tags remain
three separate namespaces.

## Contract and compatibility

Versioned request/response documents; typed errors `github-invalid`
(malformed/unknown selection), `github-adapter-unavailable` (missing or
non-executable adapter), and the mapping above for provider states. Forge
Core contracts do not gain any GitHub-specific field.

## Failure and boundary policy

| Case | Result |
|---|---|
| No token configured | observation is `Unauthorized`; local-only Forge stays fully functional |
| Rate limited | `RateLimited` with reset hint; no partial data presented as complete |
| Not found / forbidden | explicit state; no record invented |
| Network unavailable | `Unavailable`; prior catalog records untouched |
| Adapter output carries a token/body secret | refused and redacted; never rendered |
| Any request without an approved proposal | no mutation; observation only |

## Verification oracle

`tests/github_adapter_contract.rs`: normalization into catalog records with
provenance, closed state mapping, PR-mode vs direct-mode gating, and namespace
separation; `tests/github_adapter_cross_surface.rs`: a **local executable stub**
captures the exact request, proves no token reaches a report, proves no request
is sent without configuration, and proves a missing/non-executable adapter is a
typed unavailable state. No live GitHub host is contacted by the tests. No
checkbox without its named test and captured output.

## Decision ledger

- GitHub is optional and never authoritative for local evidence or deployability.
- PR mode is the default mutation path; direct mutation is explicit and gated.
- GitHub language statistics are not build evidence.
