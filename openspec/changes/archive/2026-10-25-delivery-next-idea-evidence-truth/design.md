# Design: delivery-next-idea-evidence-truth

## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The next-idea route | `forge` | Yes. `src/api/lifecycle_exec/delivery.rs`. |
| The lifecycle contract test | `forge` | Modified. `tests/web_lifecycle_execution_contract.rs`. |
| The next-idea contract test | `forge` | Added. `tests/delivery_next_idea_evidence_truth_contract.rs`. |
| The `delivery-next-idea-evidence-truth` capability | `forge` | Added. |
| The `web-lifecycle-execution` capability | `forge` | Modified. |
| The lifecycle journal helper | `forge` | Read only. |
| The other lifecycle routes | `forge` | Read only. |
| The publish lanes | `forge` | Read only. |
| The delivery publish lane | `forge` | Read only. |
| The frontend | `forge` | Read only. |
| The registry journal | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/api/lifecycle_exec/delivery.rs, before
let detail = if note.is_empty() {
    "delivery.next-idea: publish landed; next loop proposed".to_string()
} else {
    format!("delivery.next-idea: {note}")
};
```

### What it produced

```
delivery.next-idea|done|delivery.next-idea: publish landed; next loop proposed
```

### What the route knows

The route reads the request body, checks `confirm`, bounds the note and
records one journal row:

```rust
// src/api/lifecycle_exec/delivery.rs, before
let note = body.get("note").and_then(Value::as_str).map(str::trim).unwrap_or("").to_string();
if note.len() > 2000 { ... }
if !note.is_empty() && credential_shaped(&note) { ... }
let detail = if note.is_empty() { ... } else { ... };
record_journal(db_path, "delivery.next-idea", id, &detail);
```

It reads no publish, no publication, no journal row and no delivery
state. The word "publish landed" was a string in the source, not an
observation.

### The evidence lives elsewhere

| The evidence | Where it is recorded |
|---|---|
| A project publish | The publish lane's own journal row (kind `publish` or `publish.github`), with the verdict the publish lane computed. |
| A delivery publish | The delivery lane's publication attempt, with the status the publication lane recorded. |
| The loop transition | The `delivery.next-idea` row — this route's own record. |

The next-idea row is the transition record. Restating a publish inside
it, without reading one, is the programme's rule class: Forge states
something its own gathered evidence does not support.

### The pattern this lane already uses

The sibling route in the same lane records an operator-observed outcome
as the operator's statement, not as a Forge finding:

```rust
// src/api/delivery.rs
/// `POST /v1/admin/delivery/reconcile` — record the operator-observed
/// outcome of a publication attempt Core could not vouch for (`unknown`).
/// ... so that a reconciliation is recorded as the operator's explicit
/// statement rather than something Forge observed.
```

The change 41 rule is the same shape: a promotion receipt records the
operator's reason as the operator's reason.

## The Rule

**The next-idea row states the transition it recorded. It never claims
a publish landed, and a caller's note is recorded as the caller's
report.**

```rust
// src/api/lifecycle_exec/delivery.rs, after
// The route states what it did: it recorded the next-idea transition.
// It read no publish, so it claims none. A caller's note is the
// caller's report, recorded as such.
let detail = if note.is_empty() {
    "delivery.next-idea: transition=proposed".to_string()
} else {
    format!("delivery.next-idea: transition=proposed reported={note}")
};
```

The detail follows the sibling routes' shape — `kind subject key=value`
— so the row reads like the rest of the lifecycle journal:

| Detail | Meaning |
|---|---|
| `delivery.next-idea: transition=proposed` | The route recorded the next-idea transition. It read no publish, so it says none. |
| `delivery.next-idea: transition=proposed reported=publish landed` | The route recorded the transition and kept the caller's report, marked as the caller's. |

The row's kind, project id and state do not change: the route did
record the transition, so the row's state is still `done`.

## The options, decided

| Option for the row's detail | Verdict |
|---|---|
| Keep the default "publish landed" | Rejected. The route read nothing; the words were a string in the source. The operations table would carry a publish claim no surface ever made. |
| Keep the claim and add the caller's note when there is one | Rejected. The claim is the defect, not its position in the string. |
| Make the route observe the publish (a journal row, a publication attempt) | Rejected. The publish lanes already write their own evidence; reading it here would put a second copy of the publish verdict vocabulary in the lifecycle lane, and the row it reads could be weeks old — a stale `done` row would read as a fresh publish. |
| Make the route refuse when it observed no publish | Rejected. The route has no observation to judge, so a refusal would be a guess too; and the canonical requirement asks for the row when the publish succeeds, which is the caller's moment to call the route. |
| State the transition and record the caller's note as the caller's report | **Chosen.** The route states what it did, keeps the caller's information, and marks whose statement it is. The publish's evidence stays in the publish lanes' own rows. |
| Drop the note entirely | Rejected. The note is the operator's context; dropping it loses information the caller supplied. |
| Rewrite the rows written before this change | Rejected. Nothing can know what an old row described. The rule applies to new rows. |

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A confirmed request with no note | The row says `transition=proposed`. It claims no publish. |
| A confirmed request with a note | The row says `transition=proposed reported=<the caller's note>`. The claim stays the caller's. |
| A request without `confirm` | Unchanged: refused with `409`, nothing journalled. |
| A note over 2000 characters | Unchanged: refused with `400`. |
| A credential-shaped note | Unchanged: refused with `400`. |
| The response body | Unchanged: `200` with `accepted`, `project_id`, `operation_id`, `effect` and the contract version. |
| The row's kind, project and state | Unchanged: `delivery.next-idea`, the project id, `done`. |
| The publish lanes | Unchanged: they keep their own evidence rows. |
| The frontend's prompt | Unchanged: the browser's "the publish landed" follows the publish response the browser received. |
| The other lifecycle routes | Unchanged: they state what they observed and keep their own wording. |

## Risks

| Risk | Mitigation |
|---|---|
| The row no longer carries the publish claim the canonical requirement's scenario names. | The requirement is about the row's kind and the workbench's announcement, both of which stay. The publish's evidence stays in the publish lanes' own rows, which is where a reader looks for it. |
| A reader that searched the operations table for "publish landed" now finds it only in the caller's report. | That is the truth: the server never observed a publish. The publish lanes' rows are the place to look for the publish itself. |
| The note is now prefixed, so a reader parsing the detail as a whole sees a different string. | The detail is a human string; the row's kind, project, state and length are unchanged, and no reader in the repository parses this detail. |
| The caller's note could still carry an unverified claim. | It is marked as the caller's report. The server's own statement — the transition it recorded — is stated separately and is true. |
