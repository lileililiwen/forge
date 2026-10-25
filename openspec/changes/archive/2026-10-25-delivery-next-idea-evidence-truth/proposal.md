# Change: delivery-next-idea-evidence-truth

## Why

The next-idea route journals a publish it never observed.

`POST /v1/admin/projects/{id}/delivery/next-idea` records the loop
transition after a delivery publish. When the caller sends no note, the
route writes this into the journal:

```rust
// src/api/lifecycle_exec/delivery.rs, before
let detail = if note.is_empty() {
    "delivery.next-idea: publish landed; next loop proposed".to_string()
} else {
    format!("delivery.next-idea: {note}")
};
```

The route read nothing. It did not look at a publish, a publication, a
journal row or a delivery state. It wrote the words "publish landed"
because those words were the default.

Reproduced on the current tree:

```
$ forge-web api POST /v1/admin/projects/p1/delivery/next-idea '{"confirm":true}'
200 OK
$ sqlite3 registry.db "SELECT kind, state, detail FROM operations WHERE kind='delivery.next-idea'"
delivery.next-idea|done|delivery.next-idea: publish landed; next loop proposed
```

A reader of the operations table — the portal's activity view, an
operator asking what Forge did — reads a row that says a publish landed
and names no publish.

The claim is not a harmless summary either. The canonical
`web-lifecycle-execution` capability requires the row as the record of
the loop transition:

> Delivery publish success SHALL record a `delivery.next-idea` journal
> row and render it in `#delivery-next-idea` with the `&step=idea` link
> plus the maintain refresh shortcut.

The publish's own evidence lives in the publish lane's own row (kind
`publish`, `publish.github`) or in the delivery lane's publication
attempt. The next-idea row is the *transition* record. Restating a
publish inside it, without having read one, is the programme's rule
class exactly: Forge states something its own gathered evidence does
not support.

This is the same repair as changes 37 to 42, one lane over: the
surface states only what it observed, and what it did not observe it
does not claim.

## What Changes

- **The route states the transition it recorded.** The journal detail
  says the next-idea transition was proposed. It never says a publish
  landed.
- **A caller's note is the caller's report.** The note the caller
  sends is recorded as the caller's report, not as the server's
  finding — the pattern the delivery reconcile route already uses
  ("records the operator-observed outcome ... as the operator's
  explicit statement rather than something Forge observed").
- **The canonical requirement states the rule.** The
  `web-lifecycle-execution` capability says the row names the
  transition and never claims an unobserved publish.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The next-idea route | `forge` | Yes | `src/api/lifecycle_exec/delivery.rs`: the detail names the transition and the caller's report. |
| The lifecycle journal helper | `forge` | Read only | `record_journal` keeps writing the row with the same kind, project and state. |
| The next-idea response | `forge` | Read only | The route still answers `200` with the operation id; only the detail changes. |
| The request contract | `forge` | Read only | `confirm` and `note` keep their meaning, their length limit and the credential-shaped refusal. |
| The other lifecycle routes | `forge` | Read only | `graduation.import`, `intent.apply` and `remediate.apply` state what they observed and are unchanged. |
| The publish lanes | `forge` | Read only | They keep their own evidence rows; the next-idea row no longer restates them. |
| The delivery publish lane | `forge` | Read only | `POST /v1/admin/delivery/publish` and the reconcile route are unchanged. |
| The frontend | `forge` | Read only | The workbench's Next-idea prompt says "the publish landed" because the browser observed the publish response; that is the browser's own statement and it stays. |
| The lifecycle contract test | `forge` | Modified | `tests/web_lifecycle_execution_contract.rs`: the detail assertions follow the new rule. |
| The next-idea contract test | `forge` | Added | `tests/delivery_next_idea_evidence_truth_contract.rs`. |
| The `delivery-next-idea-evidence-truth` capability | `forge` | Added | this package |
| The `web-lifecycle-execution` capability | `forge` | Modified | the transition requirement states the general rule. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §11 owns the semantic component registry and §45-§47 own the lifecycle surfaces; the `web-lifecycle-execution` capability owns the next-idea transition. |
| Contracts | No field is removed or renamed. The journal row keeps its kind, project and state; its detail no longer contains an invented claim. |
| Callers | `POST /v1/admin/projects/{id}/delivery/next-idea` and the workbench's Next-idea prompt. |
| Persistence | The row's detail text changes. The row's kind, project id and state do not. |
| Integrations | None. No adapter, provider or repository is touched. |
| Failures | No new error code and no new refusal. The route keeps answering `200` for a confirmed request; only the statement in the detail changes. |
| Tests | A new contract test `tests/delivery_next_idea_evidence_truth_contract.rs`; the existing lifecycle contract test gains the detail assertions. |
| Dependencies | None added. |
| Concerns | Quality: the operations table no longer carries a publish claim the server never read. Compatibility: the row's kind, state and response are unchanged, so no reader breaks. Security: the caller's note keeps its length limit and its credential-shaped refusal. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `delivery-next-idea-evidence-truth` | forge | Rust | added | this package |
| `web-lifecycle-execution` | forge | Rust | modified | the transition requirement states the general rule. |

## Non-goals

- **The publish lanes are unchanged.** They keep their own evidence
  rows; the next-idea row does not restate them.
- **The route does not start observing publishes.** Reading the
  publish lanes' verdicts here would put a second copy of the publish
  vocabulary in the lifecycle lane, and a stale row would read as a
  fresh publish.
- **The response is unchanged.** The route still answers `200` with
  the operation id, and a request without `confirm` is still refused
  with `409`.
- **The note keeps its contract.** Its length limit and its
  credential-shaped refusal are unchanged.
- **The frontend is unchanged.** The browser's "the publish landed"
  follows the publish response the browser received.
- **No journal row is rewritten.** Rows written before this change
  stay as they are.
