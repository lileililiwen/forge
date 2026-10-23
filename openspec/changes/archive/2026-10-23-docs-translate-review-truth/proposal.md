# Change: docs-translate-review-truth

## Why

`forge docs translate` writes a derivative per locale and then states
what the run achieved. The canonical capability says what the run owes
the operator:

> #### Scenario: Canonical and derivative documentation success
>
> - **WHEN** translation of an enabled locale completes against an
>   unchanged source
> - **THEN** the derivative records its source hash and review status

So the derivative records a **review status**. The capability's own
purpose names the case: "needs-review marking on link or terminology
violations". The doctor already reads that state and judges it as a
warning, not a failure:

```rust
// src/doctor/freshness.rs
FreshnessStatus::NeedsReview { .. } => FindingStatus::Warn,
```

The translate report read the same state and ignored it. Its aggregate
rule looked only at the per-locale *status*:

```rust
// src/docs/model.rs, before
/// `true` when every locale is `translated` or `current`.
/// A run with no outcomes is not healthy: the request was
/// meaningless.
pub(super) fn healthy_check(&self) -> bool {
    if self.outcomes.is_empty() {
        return false;
    }
    self.outcomes
        .iter()
        .all(|o| o.status == STATUS_TRANSLATED || o.status == STATUS_CURRENT)
}
```

and its note was built from the failed count alone:

```rust
// src/docs/translate.rs, before
let note = if failed == 0 {
    format!(
        "docs translate complete for project `{}`: {translated} translated, {current} current",
        request.project_id
    )
} else { … };
```

So a locale whose translation dropped a link — a derivative the
report itself marks `needs-review`, with the reason attached — was
reported as **complete**, with `healthy: true` and exit code 0.

Driven on this host, with a translator that drops the link destination
the source carried:

```text
project: docs-app
source: README.md (en)
locales: zh-CN
state: …/docs-app/.forge/docs
  - zh-CN translated derivative=docs/README.zh-CN.md: locale `zh-CN` translated to `docs/README.zh-CN.md` at source hash `82dbd281ea67` (4 segment(s) translated, 0 reused)
      review: needs-review
      review-reason: link destination `https://example.com/manual` was altered or dropped
      evidence: source_hash: 82dbd281ea67a316ffc9d421da8b7fe8f4b5d29f43cfe2e0948d1693d85a4898
summary: docs translate complete for project `docs-app`: 1 translated, 0 current
```

and the JSON document carried `"healthy": true` with the same
`"note"`, and the command **exited 0**.

**The same output contradicted itself.** Three lines above the
summary, the report said `review: needs-review` and named the reason;
the summary said `complete`. The report's own aggregate field said
`healthy: true`.

And the release lane, which runs the same translation as a stage, hid
it too:

```rust
// src/release/engine/stages.rs, before
let status = if outcome.status == crate::docs::STATUS_TRANSLATED
    || outcome.status == crate::docs::STATUS_CURRENT
{
    STATUS_DELIVERED.to_string()
} else {
    STATUS_FAILED.to_string()
};
```

The stage's status is the locale's *translation* status, so a
needs-review locale is `delivered` — that part is right, the derivative
was written. But the stage's note carried the outcome's note, which
does not mention the review, so the release's own evidence for that
locale never said that a human still had to look at it.

Two surfaces, one missing rule: the report's aggregate verdict and the
release's docs stage evidence both dropped a review status the report
had already gathered.

## What Changes

- **One typed translate verdict.** The report's aggregate verdict names
  the review status of its outcomes: `complete`, `needs-review` or
  `partial`.
- **A needs-review verdict is a warning, not a failure.** The command
  still exits 0 and the journal still records the operation as `done`,
  because the translation succeeded and a human must look at it. But no
  surface calls the run complete, and the report's `healthy` field is
  true only for `complete`.
- **The release lane names the review.** The docs stage's note names
  the locale's review status, so the release's evidence says what the
  report knows. The stage's status is unchanged: a needs-review locale
  is still `delivered`, because the derivative was written.
- **The claims are pinned.** A repository contract check drives the
  real CLI with a translator that drops a link, and drives the release
  lane over the same project.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The translate report's aggregate verdict | `forge` | Yes | `src/docs/model.rs`: the typed verdict and the rule. |
| The translate note | `forge` | Yes | `src/docs/translate.rs`: the note names the verdict. |
| The CLI's exit code and journal row | `forge` | Yes | `src/cli/release.rs`: the verdict decides both. |
| The release lane's docs stage | `forge` | Yes | `src/release/engine/stages.rs`: the stage note names the review. |
| The doctor's docs finding | `forge` | Read only | The doctor already judges a needs-review derivative as a warning; the change makes the report agree with it. |
| The per-locale outcome | `forge` | Read only | The outcome already carries `review` and `review_reasons`. |
| The derivative and its state file | `forge` | Read only | The derivative and the recorded review status are unchanged. |
| The translate contract test | `forge` | Yes | `tests/docs_translate_review_truth_contract.rs` (new). |
| The `docs-translate-review-truth` capability | `forge` | Added | this package |
| The `documentation-translation` capability | `forge` | Modified | The derivative requirement names the verdict. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §24 (documentation) owns the translation lane; the `documentation-translation` capability owns the derivative and its review status. |
| Contracts | The report gains one always-present field (`verdict`). The `healthy` field keeps its name and its type but becomes true only for `complete`; a needs-review run now publishes `healthy: false` with `verdict: "needs-review"`. The contract version is unchanged. |
| Callers | `forge docs translate` (human and JSON), its exit code and its journal row; the release lane's docs stage. |
| Persistence | The derivative and the recorded review status are unchanged. The journal row keeps the state `done` for a needs-review run; its detail gains the verdict. |
| Integrations | None. The translator adapter is unchanged. |
| Failures | No new error code. A needs-review run still exits 0. |
| Tests | A new `tests/docs_translate_review_truth_contract.rs`. |
| Dependencies | None added. |
| Concerns | Quality: the report and the release lane stop hiding a review status they already hold. Compatibility: a reader that checked `healthy` for "the translation succeeded" must now read `verdict`; a needs-review run publishes `healthy: false` and `verdict: "needs-review"`. Security: no new surface; the review reasons are the report's own. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `docs-translate-review-truth` | forge | Rust | added | this package |
| `documentation-translation` | forge | Rust | modified | the derivative requirement names the verdict. |

## Non-Goals

- **A needs-review run does not fail.** The translation succeeded; a
  human must review it. The exit code stays 0 and the journal state
  stays `done`.
- **The per-locale outcome is unchanged.** It already carries `review`
  and `review_reasons`, and the human renderer already prints them.
- **The derivative is unchanged.** The recorded review status and the
  source hash are written exactly as before.
- **The doctor's finding is unchanged.** It already judges a
  needs-review derivative as a warning; this change makes the report
  agree with it rather than replacing the doctor's rule.
- **The release's docs stage status is unchanged.** A needs-review
  locale is still `delivered`; only the stage's note changes.
- **The contract version is unchanged.**
