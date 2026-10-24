# Change: component-qualify-evidence-truth

## Why

`forge component qualify` promotes a component to a higher quality
level and writes the promotion receipt. The receipt is the record the
resolver and every later reader trust. Forge writes it with an
evidence block it invented.

Reproduced on the current tree, in a directory with no test run, no
security review and no verification:

```
$ forge component qualify toast --to certified --reason "looks fine" --path /tmp/comp-probe
$ cat /tmp/comp-probe/.forge/components/toast/qualify.json
{
  "contract": "0.1.0",
  "component_id": "toast",
  "target_quality": "certified",
  "reason": "looks fine",
  "evidence": {
    "test_coverage": 0.949999988079071,
    "known_issues": [],
    "last_verified": "2026-10-24T15:00:16.482913Z",
    "security_review": true
  }
}
```

`toast` was promoted from `verified` to `certified`. Nothing measured
the coverage, nobody reviewed the security, and nobody verified the
component. The three numbers in that receipt are defaults the command
made up:

```rust
// src/cli/component.rs, before
let last_verified_ts = match last_verified.as_deref() {
    Some(value) => parse_rfc3339_for_qualify(value).ok_or_else(...)?,
    None => chrono::Utc::now(),
};
let evidence = ComponentQualifyEvidence {
    test_coverage: coverage.unwrap_or(0.95),
    last_verified: last_verified_ts,
    known_issues: known_issues.clone(),
    security_review: security_review.unwrap_or(target == ComponentQuality::Certified),
};
```

`coverage.unwrap_or(0.95)` is above the certified minimum
(`CERTIFIED_TEST_COVERAGE = 0.85`), so the coverage gate always passes.
`security_review.unwrap_or(target == Certified)` makes the security
gate always pass for the one level that asks for it. `last_verified`
falls back to `now`, so the freshness gate always passes. **The
certified gate could not fail.** The three checks in
`evidence_meets_certified_gate` are unreachable with the defaults the
command supplies.

The command's own help text advertised the defaults:

```
--coverage <COVERAGE>          Override test coverage (default: 0.95 for `certified`)
--last-verified <LAST_VERIFIED> Override the last-verified timestamp (RFC 3339). Default: now
--security-review <SECURITY_REVIEW>  Set security review status (default: `true` for `certified`)
```

This is the rule the repair programme is built on: Forge states
something its own gathered evidence does not support. The canonical
`semantic-component-registry` capability already requires the honest
behaviour — *"WHEN certification lacks required verification or
security evidence THEN promotion fails and the prior quality level
remains"* — and the command breaks it: it never lacks the evidence,
because it makes it up.

The damage does not stop at the receipt. `Certified` is the level the
resolver prefers ("the resolver prefers the certified candidate and
reports its evidence"), so a fabricated receipt promotes a component
into the preferred slot on the strength of numbers nobody produced.

## What Changes

- **Forge does not invent evidence.** The qualify command passes the
  operator's evidence through as it was supplied. An absent flag
  stays absent.
- **A certified promotion needs its evidence.** A promotion to
  `certified` without a test coverage, a last-verified timestamp and
  a security review is refused. The refusal names the missing
  evidence, the prior quality level stays, and no receipt is written.
- **The receipt records only what was supplied.** The evidence block
  carries the fields the operator gave and no others: no default
  coverage, no default security review, no default timestamp.
- **The rule lives in the core promotion path.** The command and the
  library read the same certified evidence gate, so there is one
  place that decides what a certification needs.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The certified evidence gate | `forge` | Yes | `src/component/engine.rs`: one rule names the evidence a certification needs and refuses an absent one. |
| The promotion evidence type | `forge` | Yes | `src/component/model.rs`: `ComponentQualifyEvidence` holds optional evidence; an absent field is absent, not a default. |
| The promotion outcome | `forge` | Yes | `src/component/model.rs`: the outcome names the evidence a refusal was missing. |
| The receipt writer | `forge` | Yes | `src/component/engine.rs`: the receipt carries only the supplied evidence. |
| The qualify command | `forge` | Yes | `src/cli/component.rs`: the flags are passed through; no default coverage, timestamp or security review. |
| The command help | `forge` | Yes | `src/cli/commands_ops.rs`: the help no longer advertises defaults that do not exist. |
| The component contract test | `forge` | Modified | `tests/component_contract.rs`: the complete-evidence case supplies its evidence explicitly. |
| The component unit tests | `forge` | Modified | `src/component/engine_tests.rs`: the evidence constructions follow the optional fields; new cases cover the refusal. |
| The promotion contract test | `forge` | Added | `tests/component_qualify_evidence_truth_contract.rs`. |
| The `component-qualify-evidence-truth` capability | `forge` | Added | this package |
| The `semantic-component-registry` capability | `forge` | Modified | the quality requirement states the general rule. |
| The catalog and its descriptors | `forge` | Read only | The catalog's own evidence is unchanged; the defect is in the promotion path, not in the catalog. |
| The resolver | `forge` | Read only | The resolver still prefers a certified candidate; it now prefers only candidates that earned the level. |
| The receipt layout | `forge` | Modified | The evidence object keeps its field names; absent fields are omitted instead of filled in. |
| The registry journal | `forge` | Read only | A refused promotion is still journalled as `blocked`. |
| The catalog browser and the web surfaces | `forge` | Read only | They read the catalog, not the qualify receipt. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §11 and §12 own the component registry and its quality levels; the `semantic-component-registry` capability owns the promotion gate. |
| Contracts | No field is removed or renamed. `ComponentQualifyEvidence`'s three measured fields become optional; `ComponentQualifyOutcome` gains `missing_evidence`; the receipt's evidence object keeps its field names and omits the ones that were not supplied. |
| Callers | `forge component qualify` is the only writer of the receipt. |
| Persistence | The receipt under `.forge/components/<id>/qualify.json` is written less often: a refused promotion writes nothing. Its layout is unchanged for a complete promotion. |
| Integrations | None. No adapter, provider or repository is touched. |
| Failures | No new error code. A certified promotion without complete evidence is refused like every other refused promotion: the outcome says what is missing, the prior quality level stays, no receipt is written. |
| Tests | A new contract test `tests/component_qualify_evidence_truth_contract.rs`; the component unit tests gain the refusal cases; the existing complete-evidence case supplies its evidence. |
| Dependencies | None added. |
| Concerns | Quality: a certified promotion now needs three real observations, so the certified level means something. Compatibility: a caller that relied on the defaults can no longer promote to `certified` without evidence; the refusal names what is missing. Security: the receipt stops claiming a security review that never happened. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `component-qualify-evidence-truth` | forge | Rust | added | this package |
| `semantic-component-registry` | forge | Rust | modified | the quality requirement states the general rule. |

## Non-goals

- **The catalog is unchanged.** The catalog's own descriptors keep
  their evidence, their qualities and their timestamps. The defect is
  in the promotion path.
- **The resolver is unchanged.** It still prefers a certified
  candidate. It now prefers only candidates that earned the level.
- **The deprecated path is unchanged.** A deprecation needs no
  evidence and writes no receipt, exactly as before.
- **A promotion below `certified` is not gated.** This change does
  not invent an evidence rule for `experimental` or `verified`; the
  canonical requirement names certification. Such a promotion writes
  a receipt that carries only the evidence the operator supplied.
- **The receipt layout is unchanged.** The same fields, the same
  names; a field that was not supplied is omitted.
- **No receipt is back-filled.** The receipts written before this
  change stay on disk; nothing rewrites them.
