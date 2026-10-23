## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The translate report's aggregate verdict | `forge` | Yes. `src/docs/model.rs`. |
| The translate note | `forge` | Yes. `src/docs/translate.rs`. |
| The CLI's exit code and journal row | `forge` | Yes. `src/cli/release.rs`. |
| The release lane's docs stage | `forge` | Yes. `src/release/engine/stages.rs`. |
| The translate contract test | `forge` | Yes. `tests/docs_translate_review_truth_contract.rs` (new). |
| The `docs-translate-review-truth` capability | `forge` | Added. |
| The `documentation-translation` capability | `forge` | Modified. |
| The doctor's docs finding | `forge` | Read only. |
| The per-locale outcome | `forge` | Read only. |
| The derivative and its state file | `forge` | Read only. |

## The Gap

### The report's aggregate rule never looked at the review

```rust
// src/docs/model.rs, before
/// `true` when every locale is `translated` or `current`.
pub(super) fn healthy_check(&self) -> bool {
    if self.outcomes.is_empty() {
        return false;
    }
    self.outcomes
        .iter()
        .all(|o| o.status == STATUS_TRANSLATED || o.status == STATUS_CURRENT)
}
```

The outcome carries `review: String` and `review_reasons: Vec<String>`,
and the doctor reads the same state as a warning. The report's
aggregate rule read only `status`.

### The note was built from the failed count

```rust
// src/docs/translate.rs, before
let note = if failed == 0 {
    format!(
        "docs translate complete for project `{}`: {translated} translated, {current} current",
        request.project_id
    )
} else { … };
```

So the summary contradicted the three lines above it.

### The release lane's stage note dropped the review

```rust
// src/release/engine/stages.rs, before
note: format!("docs stage for locale `{locale}`: {}", outcome.note),
```

The outcome's note is the per-locale translation note; it does not
carry the review.

## The Rule

One typed verdict, computed from the outcomes, read by every surface.

```rust
/// The run's aggregate verdict over its locale outcomes.
pub enum TranslateVerdict {
    /// Every locale is translated or current, and every review is ok.
    Complete,
    /// Every locale is translated or current, and at least one review
    /// needs a human. A warning, not a failure.
    NeedsReview,
    /// At least one locale failed.
    Partial,
}
```

The rule is one sentence:

> A docs translate report SHALL carry one aggregate verdict over its
> outcomes. The verdict is `complete` when every locale is translated
> or current and every outcome's review is `ok`; `needs-review` when
> every locale is translated or current but at least one outcome's
> review is `needs-review`; and `partial` when at least one locale
> failed.

**The review status comes from the outcome's own `review` field.** The
outcome already carries it, and the doctor already reads it as a
warning. The report and the release lane now read the same fact.

**A needs-review verdict is a warning, not a failure.** The command
still exits 0 and the journal still records the operation as `done`,
because the derivative was written and the translation succeeded. What
changes is the claim: no surface calls the run complete.

**The report's `healthy` field keeps its name and its type.** It is
true only for `complete`. A needs-review run publishes
`healthy: false` and `verdict: "needs-review"`, so a reader that
checks either field learns the same thing — the same house rule change
24 established for the release lane's DriftWatch verdict: a warning is
never called healthy.

**The report publishes the verdict.** The report gains one always-
present field, `verdict`, so the JSON document states the aggregate
verdict rather than leaving the reader to re-derive it from the
outcomes.

**The CLI keys off the verdict, not off `healthy`.** The exit code and
the journal's state label follow the verdict:

| Verdict | Exit code | Journal state | Journal detail |
|---|---|---|---|
| `complete` | 0 | `done` | names the verdict |
| `needs-review` | 0 | `done` | names the verdict |
| `partial` | 1 | `partial` | names the verdict |

**The release lane names the review.** The docs stage's status is
unchanged — a needs-review locale is `delivered`, because the
derivative was written — and the stage's note gains the locale's review
status, so the release's evidence says what the report knows.

## Failure Boundaries

| Boundary | Behaviour |
|---|---|
| Every locale translated, every review ok | Verdict `complete`; `healthy: true`; exit 0. |
| A locale's review needs a human | Verdict `needs-review`; `healthy: false`; exit 0; journal state `done`. |
| A locale failed | Verdict `partial`; `healthy: false`; exit 1; journal state `partial`; the per-locale outcome JSON is printed to stdout, as today. |
| A locale failed and another needs review | Verdict `partial` — the definite failure outranks the warning. |
| No outcomes | Verdict `partial`; `healthy: false`; exit 1 — the request was meaningless, as today. |
| A disabled locale | No outcome, no change. |
| The release lane's docs stage | The stage's status is `delivered` for a translated or current locale, whatever its review; the note names the review. |

## Risks

- **The report's `healthy` field changes meaning for a needs-review
  run.** It becomes false. A reader that used `healthy` to mean "the
  translation succeeded" must read `verdict` instead. That is the
  point of the change: `healthy` was the field the report's own doc
  comment called "the verdict".
- **The note's wording changes for a needs-review run.** It says
  "needs review" and counts the locales that need one, instead of
  "complete".
- **The journal detail gains a field.** The state does not change for
  a needs-review run.
- **The contract version is unchanged.** The report gains a field; no
  field is removed or renamed.
- **The derivative is unchanged.** The recorded review status, the
  source hash and the written file are exactly as before.
