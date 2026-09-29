# Design: project-semantic-description-review

Status: implementation-ready planning package. No code is written by this
change.

## Implementation boundary

Repository `forge`, Rust 1.87+, existing spec-remediation and agent-runtime
stack.

Files to **add**:

- `src/semantic/mod.rs` — module root and re-exports.
- `src/semantic/proposal.rs` — the proposal record, its closed state machine
  and provenance.
- `src/semantic/review.rs` — suggest/approve/reject/supersede operations over
  the shared remediation model.
- `tests/semantic_review_contract.rs`, `tests/semantic_review_cross_surface.rs`.

Files to **change** (additive only):

| File | Change |
|---|---|
| `src/lib.rs` | `pub mod semantic;` |
| `src/main.rs` | `Describe` and `Classify` command trees: `suggest`, `list`, `show`, `approve`, `reject` |
| `src/spec/mod.rs` | route an unresolved/conflicting suggestion to bounded OpenSpec or manual review |

Do **not** touch: deterministic remediation writes (owned by
`project-local-remediation-plans`), GitHub transport (owned by the adapter
package), deployment, billing or the interest store.

## Language and runtime

Rust 1.87+, `rustfmt` defaults, no new dependency. An optional provider is
reached through the existing `src/agent/` adapter boundary; the Core contract is
provider-neutral. Commands: `cargo test --lib -- semantic`,
`cargo test --test semantic_review_contract`, `cargo clippy --all-targets -- -D
warnings`.

## Ownership and shared code

Forge owns proposal and approval state. The package **extends**
`src/spec/mod.rs` for bounded follow-up routing and **adapts** `src/agent/`
for optional generation; it **keeps local** rather than reusing the analytics or
portfolio stores, because semantic metadata is neither interest nor user-owned
portfolio state. No generated text is authority until an operator approves it.

## Behavioral model

`forge-semantic-proposal/0.1.0`.

```rust
pub enum ProposalKind { Description, Domain, PortfolioTags, Profile, Lifecycle }
pub enum ProposalState { Suggested, Approved, Rejected, Superseded, Conflicted }
```

| Field | Rule |
|---|---|
| `current_value` | the approved/observed value now in force, or `null` |
| `suggested_value` | bounded, control-free, credential-scrubbed text |
| `evidence` | source paths, revisions, excerpts or hashes, redacted; never raw secrets |
| `confidence` | `low\|medium\|high`, explicitly not truth |
| `provider` | generator identity, or `operator` |
| `state` | closed machine; only an operator moves `Suggested` to `Approved`/`Rejected` |
| `revision` | bound to the catalog/evidence revision it saw; a later revision supersedes |

## Contract and compatibility

Proposal and review state are project-scoped, revision-bound JSON. Errors are
typed: `semantic-invalid` (malformed suggestion, unlisted kind), and the
existing `unknown-project`/conflict codes where applicable. A conflicted
proposal (evidence disagrees) is `Conflicted`, never auto-approved.

## Failure and boundary policy

| Case | Result |
|---|---|
| Provider unavailable | `Suggested` is not created; a typed unavailable state is reported, prior proposals untouched |
| Conflicting evidence | `Conflicted` with each source retained; no approval |
| Approval of a stale proposal | refused, because a newer revision superseded it |
| Malformed/provider-injected text | refused and scrubbed; never becomes authority automatically |
| Suggestion with no operator review | stays `Suggested`; nothing is written to the project or a provider |

## Verification oracle

`tests/semantic_review_contract.rs`: closed state machine, provenance capture,
confidence labelling, approve/reject/supersede transitions, stale-revision
refusal and conflict retention; `tests/semantic_review_cross_surface.rs`:
suggest is read-only, nothing is auto-applied, credentials are redacted, and no
provider or project file is written without an approval. No checkbox without
its named test and captured output.

## Decision ledger

- Confidence is a label, not truth and not deployment evidence.
- Generated suggestions are never auto-published and never treated as observed
  fact.
- Deterministic repairs stay in the remediation package; this package only
  produces and reviews proposals.
