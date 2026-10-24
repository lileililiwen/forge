## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The write gate | `forge` | Yes. `src/release/model.rs`: `ReleaseRequest::may_write()` and `ReleaseRequest::write_refusal()`. |
| The commit stage | `forge` | Yes. `src/release/engine/apply.rs`. |
| The tag stage | `forge` | Yes. `src/release/engine/apply.rs`. |
| The push, mirror, package, container, docs and notes stages | `forge` | Yes. `src/release/engine/stages.rs`. |
| The release note | `forge` | Yes. `src/release/engine/apply.rs`. |
| The registry journal state | `forge` | Yes. `src/cli/release.rs`. |
| The release engine tests | `forge` | Yes. `src/release/engine/engine_tests.rs`. |
| The dry-run contract test | `forge` | Added. `tests/release_dry_run_truth_contract.rs`. |
| The `release-dry-run-evidence-truth` capability | `forge` | Added. |
| The `release-publishing` capability | `forge` | Modified. |
| The release report | `forge` | Read only. |
| The stage status vocabulary | `forge` | Read only. |
| The release state file | `forge` | Read only. |
| The release contract version | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/release/engine/apply.rs, before
let note = if dry_run {
    "dry-run: no side effects were attempted".to_string()
} else if healthy {
```

The note is the release's own summary. It is read by the human
renderer, by the JSON document and by the CLI's failure reason, and it
is the one line a reader trusts about what the run did.

### The evidence that contradicts it

The owning agent's run of the release engine on its own fixture, with
`confirm: false` and `dry_run: true`:

```
NOTE=dry-run: no side effects were attempted
STAGE=tag STATUS=delivered NOTE=tag `v1.0.0` delivered for release `rel-scratch-1.0.0-aef2a7e9d820`
TAGS=v1.0.0
```

Three facts, one document:

- the note says no side effect was attempted;
- a stage outcome in the same report says a tag was delivered;
- the tag exists in the repository.

### Only three stages are gated

```rust
// src/release/engine/stages.rs, before
if !request.confirm {          // push, package, container
    return Ok(StageOutcome { status: STATUS_DISABLED.to_string(), ... });
}
```

The commit stage and the tag stage take no gate at all. `run_commit_stage`
has no request parameter and calls `commit_paths`; `run_tag_stage` takes
the request as `_request` and runs `git tag -a` unconditionally. The
docs stage takes no request and calls the translator, which writes the
translated files.

### The mirror stage claims a delivery it did not make

```rust
// src/release/engine/stages.rs, before
if !request.confirm {
    let plan = plan_distribution(&config, &mirror_request, &state, &state_path)?;
    return Ok(StageOutcome {
        status: if plan.healthy() {
            STATUS_DELIVERED.to_string()
        } else {
            STATUS_FAILED.to_string()
        },
        note: format!("mirror plan only: primary `{primary}`, ..."),
```

`plan_mirror` hard-codes `healthy: true` for a plan, so a plan-only
mirror is always "healthy" and the stage is always recorded as
`delivered` — while the note in the same outcome says "mirror plan
only". This is the only stage in the file whose `!request.confirm` path
does not return `disabled`.

## The Rule

One rule, in the one place every stage reads it:

```rust
// src/release/model.rs
impl ReleaseRequest {
    /// Whether a release stage may write to the host, the
    /// repository or a provider.
    ///
    /// A stage writes only when the release is confirmed and the
    /// run is not a dry run. A dry run states that no side effect
    /// was attempted, so a stage that writes would make the
    /// release report false.
    pub fn may_write(&self) -> bool {
        self.confirm && !self.dry_run
    }
}
```

The rule replaces three different ad-hoc checks — `!request.confirm`
in three stages, no check at all in two, and `plan.healthy()` in one —
with one decision every write-capable stage reads.

`write_refusal(stage)` names the reason in the stage's note:

| The request | The note |
|---|---|
| `dry_run: true` | `<stage> stage is a dry run; no side effect was attempted` |
| `confirm: false` | `<stage> stage requires --confirm; treating as disabled` |

### What each surface does with the rule

| Stage | Today | After |
|---|---|---|
| commit | commits the changelog | `disabled` + the refusal note |
| tag | creates an annotated git tag | `disabled` + the refusal note |
| push | `disabled` on `!confirm` | `disabled` + the shared refusal note |
| mirror | `delivered` for a plan | `disabled` + the refusal note, keeping the plan evidence |
| package | `disabled` on `!confirm` | `disabled` + the shared refusal note |
| container | `disabled` on `!confirm` | `disabled` + the shared refusal note |
| docs | translates and writes files | `disabled` + the refusal note |
| notes | renders the notes file | `disabled` + the refusal note |

### The report

- The dry-run note gains the planned stage count, so it reads
  `dry-run: no side effects were attempted; N stage(s) planned`.
- `healthy` is unchanged: it still means "no stage failed". A dry run
  that planned every stage is healthy because nothing failed, and the
  note says that nothing was written.
- The registry journal records a dry run as `planned` rather than
  `done`, so the append-only evidence does not claim that a release was
  applied.

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| `confirm: false, dry_run: false` | Unchanged: the apply is refused with `release-invalid` before any stage runs. |
| `confirm: false, dry_run: true` | Every stage is `disabled`; nothing is written; the note names the dry run. |
| `confirm: true, dry_run: false` | Unchanged: every stage runs and writes, exactly as before. |
| `confirm: true, dry_run: true` | Every stage is `disabled`; nothing is written. A dry run wins over a confirmation, because the dry run is the stronger promise the report makes. |
| A stage that fails for another reason | Unchanged: the stage records `failed` and the release is partial. |
| A retry over an already delivered stage | Unchanged: the stage records `skipped`. |

## Risks

| Risk | Mitigation |
|---|---|
| A confirmed release stops writing. | The gate opens for `confirm: true, dry_run: false`, and a contract test asserts that a confirmed apply still tags the repository. |
| The docs stage stops translating. | The docs stage is gated only when the run may not write; a confirmed release still translates. |
| The notes adapter loses `--dry-run`. | The notes adapter is untouched; the stage is gated before the adapter runs, so the adapter is called strictly less often. |
| A reader matching the old dry-run note breaks. | The phrase is kept and the planned stage count is appended. |
| The release becomes permanently partial. | `disabled` was already counted as healthy by the release's own verdict; the gate does not introduce a new status. |
| The release state file records a plan and a later apply skips the stage. | `already_delivered` only matches `delivered` and `skipped`; a `disabled` stage is never skipped on a later apply. |
