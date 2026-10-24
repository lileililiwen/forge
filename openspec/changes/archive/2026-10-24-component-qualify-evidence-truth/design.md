# Design: component-qualify-evidence-truth

## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The certified evidence gate | `forge` | Yes. `src/component/engine.rs`. |
| The promotion evidence type | `forge` | Yes. `src/component/model.rs`. |
| The promotion outcome | `forge` | Yes. `src/component/model.rs`. |
| The receipt writer | `forge` | Yes. `src/component/engine.rs`. |
| The qualify command | `forge` | Yes. `src/cli/component.rs`. |
| The command help | `forge` | Yes. `src/cli/commands_ops.rs`. |
| The component contract test | `forge` | Modified. `tests/component_contract.rs`. |
| The component unit tests | `forge` | Modified. `src/component/engine_tests.rs`. |
| The promotion contract test | `forge` | Added. `tests/component_qualify_evidence_truth_contract.rs`. |
| The `component-qualify-evidence-truth` capability | `forge` | Added. |
| The `semantic-component-registry` capability | `forge` | Modified. |
| The catalog and its descriptors | `forge` | Read only. |
| The resolver | `forge` | Read only. |
| The deprecated promotion path | `forge` | Read only. |
| The registry journal | `forge` | Read only. |
| The catalog browser and the web surfaces | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/cli/component.rs, before
let last_verified_ts = match last_verified.as_deref() {
    Some(value) => parse_rfc3339_for_qualify(value).ok_or_else(|| {
        ForgeError::ComponentInvalid {
            reason: format!("last_verified '{value}' is not a valid RFC 3339 timestamp"),
        }
    })?,
    None => chrono::Utc::now(),
};
let evidence = ComponentQualifyEvidence {
    test_coverage: coverage.unwrap_or(0.95),
    last_verified: last_verified_ts,
    known_issues: known_issues.clone(),
    security_review: security_review.unwrap_or(target == ComponentQuality::Certified),
};
```

### What it produced

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

A component that nobody tested, nobody reviewed and nobody verified was
promoted to `certified`, and the receipt says so in numbers.

### The three gates could not fail

The certified gate is three checks:

```rust
// src/component/engine.rs, before
fn evidence_meets_certified_gate(evidence: &ComponentQualifyEvidence) -> Result<(), String> {
    if !evidence.security_review { return Err("missing security review".to_string()); }
    if evidence.test_coverage < CERTIFIED_TEST_COVERAGE { ... }
    if evidence.known_issues.len() > MAX_KNOWN_ISSUES { ... }
    if !evidence_is_fresh(evidence.last_verified) { ... }
    Ok(())
}
```

Each check reads a field the command had already defaulted:

| Gate | The default | Why it always passed |
|---|---|---|
| `security_review` | `unwrap_or(target == Certified)` | The default is `true` exactly for the level that asks for it. |
| `test_coverage >= 0.85` | `unwrap_or(0.95)` | 0.95 is above the minimum. |
| `last_verified` within 180 days | `chrono::Utc::now()` | The default is the current instant. |

So the certified gate was unreachable code: a promotion to `certified`
could not be refused for want of evidence, and the receipt always
carried the three numbers that made it look complete.

### The command advertised the defaults

```
--coverage <COVERAGE>                   Override test coverage (default: 0.95 for `certified`)
--last-verified <LAST_VERIFIED>         Override the last-verified timestamp (RFC 3339). Default: now
--security-review <SECURITY_REVIEW>     Set security review status (default: `true` for `certified`)
```

The help text made the fabrication a documented feature.

### The damage goes past the receipt

`Certified` is the level the resolver prefers. The canonical
requirement says it plainly: *"WHEN compatible certified and
experimental candidates satisfy the same request THEN the resolver
prefers the certified candidate and reports its evidence"*. A
fabricated receipt puts a component into the preferred slot on the
strength of numbers nobody produced, and the resolver reports those
numbers back to the caller as the component's evidence.

## The Rule

**Forge records a promotion only from evidence the operator supplied.
An absent field is absent, not a default.**

### One gate, in the core promotion path

```rust
// src/component/engine.rs, after
/// The evidence a `Certified` promotion requires. An absent field
/// is never a default: the gate refuses the promotion and names
/// what is missing.
pub fn missing_certified_evidence(evidence: &ComponentQualifyEvidence) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if evidence.test_coverage.is_none() {
        missing.push("test_coverage");
    }
    if evidence.last_verified.is_none() {
        missing.push("last_verified");
    }
    if evidence.security_review.is_none() {
        missing.push("security_review");
    }
    missing
}
```

The gate refuses before it judges:

```rust
fn evidence_meets_certified_gate(evidence: &ComponentQualifyEvidence) -> Result<(), String> {
    let missing = missing_certified_evidence(evidence);
    if !missing.is_empty() {
        return Err(format!(
            "missing required evidence: {}; a certified promotion requires the operator to \
             supply it",
            missing.join(", ")
        ));
    }
    // Every field is present from here on; the checks below judge
    // the value, and each fallback keeps a later field from
    // inventing one.
    if !evidence.security_review.unwrap_or(false) {
        return Err("missing security review".to_string());
    }
    let coverage = evidence.test_coverage.unwrap_or(0.0);
    if coverage < CERTIFIED_TEST_COVERAGE {
        return Err(format!(
            "test coverage {} is below the certified minimum {}",
            coverage, CERTIFIED_TEST_COVERAGE
        ));
    }
    ...
}
```

The rule lives in the core module, so the command and every future
surface read it. The command no longer decides what a certification
needs; it only passes the operator's evidence through.

### The evidence type holds absence

```rust
// src/component/model.rs, after
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ComponentQualifyEvidence {
    /// Measured test coverage (`0.0`-`1.0`), when the operator has one.
    pub test_coverage: Option<f32>,
    /// When the component was last independently verified, when the operator knows it.
    pub last_verified: Option<DateTime<Utc>>,
    /// Known issues the operator listed.
    pub known_issues: Vec<String>,
    /// Whether a security review was recorded.
    pub security_review: Option<bool>,
}
```

`Option` is the whole point: the type can now say "no coverage was
supplied", which the previous type could not.

### The command passes the flags through

```rust
// src/cli/component.rs, after
let last_verified_ts = match last_verified.as_deref() {
    Some(value) => Some(parse_rfc3339_for_qualify(value).ok_or_else(|| {
        ForgeError::ComponentInvalid {
            reason: format!("last_verified '{value}' is not a valid RFC 3339 timestamp"),
        }
    })?),
    None => None,
};
let evidence = ComponentQualifyEvidence {
    test_coverage: coverage,
    last_verified: last_verified_ts,
    known_issues: known_issues.clone(),
    security_review,
};
```

The three defaults are gone. A flag that was not supplied stays
unsupplied.

### The refusal names what is missing

The outcome gains the evidence a refusal was missing, so a caller does
not have to parse the note:

```rust
// src/component/model.rs, after
pub struct ComponentQualifyOutcome {
    pub component_id: String,
    pub prior_quality: ComponentQuality,
    pub target_quality: ComponentQuality,
    pub promoted: bool,
    pub files_written: Vec<String>,
    /// The evidence a refused promotion was missing, named by the
    /// receipt's field names. Empty when nothing was missing.
    pub missing_evidence: Vec<String>,
    pub note: String,
}
```

The command publishes it:

```json
{
  "component_id": "toast",
  "prior_quality": "verified",
  "target_quality": "certified",
  "promoted": false,
  "missing_evidence": ["test_coverage", "last_verified", "security_review"],
  "files_written": [],
  "note": "promotion refused: missing required evidence: test_coverage, last_verified, security_review; a certified promotion requires the operator to supply it; component 'toast' remains at quality 'verified'"
}
```

The prior quality level is in the output, so the canonical requirement
— *"promotion fails and the prior quality level remains"* — is
visible to the caller, not implied.

### The receipt carries only what was supplied

```rust
// src/component/engine.rs, after
let mut evidence_json = serde_json::Map::new();
if let Some(coverage) = evidence.test_coverage {
    evidence_json.insert("test_coverage".to_string(), serde_json::json!(coverage_for_receipt(coverage)));
}
if let Some(last_verified) = evidence.last_verified {
    evidence_json.insert("last_verified".to_string(), serde_json::json!(last_verified.to_rfc3339()));
}
evidence_json.insert("known_issues".to_string(), serde_json::json!(evidence.known_issues));
if let Some(security_review) = evidence.security_review {
    evidence_json.insert("security_review".to_string(), serde_json::json!(security_review));
}
```

A field that was not supplied is absent from the receipt. There is no
way for the receipt to carry a number nobody produced.

### The float artifact

The coverage ratio is held as `f32` (the catalog's evidence is `f32`
too, and one measure should have one type). Widening it to `f64` for
JSON shows the binary artifact — `0.949999988079071` for the operator's
`0.95`. The receipt rounds to four decimals, which is the ratio the
operator typed:

```rust
/// Coverage ratios are held as `f32`; widening one to `f64` for
/// JSON shows the binary artifact (`0.949999988079071`). The
/// receipt rounds to four decimals so the stored number is the
/// ratio the operator supplied.
fn coverage_for_receipt(coverage: f32) -> f64 {
    (f64::from(coverage) * 10_000.0).round() / 10_000.0
}
```

## The options, decided

| Option for a certified promotion without complete evidence | Verdict |
|---|---|
| Keep the defaults (coverage 0.95, security review true, timestamp now) | Rejected. The three gates cannot fail, the receipt states measurements nobody made, and the resolver prefers the component because of them. |
| Keep the defaults and mark them "unverified" in the receipt | Rejected. The resolver reports the receipt's evidence as the component's evidence; a receipt that says "0.95, unverified" still carries a measurement that was never made, and the level is still `certified`. |
| Refuse the promotion and name the missing evidence | **Chosen.** The canonical requirement's own shape: the promotion fails, the prior quality level remains, and no receipt is written. The refusal names every missing field, so the operator knows what to measure. |
| Refuse with a typed CLI error and a non-zero exit code | Rejected. The command already returns a refusal outcome for the same class of outcome (a negative security review, a stale timestamp), and the canonical requirement gives one outcome for one condition — "certification lacks required verification or security evidence". A second shape for the same class would split the contract, and a non-zero exit would need the CLI to own its exit code for one command while the outcome already says plainly that nothing was promoted. |
| Require the same evidence for every promotion level | Rejected here. The canonical requirement names certification. A promotion to `experimental` or `verified` records the evidence the operator supplied and no more; inventing a rule for those levels is a separate change with its own contract. |
| Let Forge measure the coverage itself (run the component's tests) | Rejected. The catalog is a static catalog: it names a test surface (`forge component resolve --profile rust-web --component api-mutation contract`), it does not own a test harness it could run. Forge has nothing to measure, so the operator's measurement is the evidence. |
| Drop the receipt entirely | Rejected. The receipt is the promotion's audit record and the artifact the resolver reports. Removing it would lose the record of what was promoted and on what evidence. |
| Back-fill the receipts already written | Rejected. The change cannot know what the evidence of an old promotion really was. The old receipts stay as they are; the rule applies to new promotions. |

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A certified promotion with coverage, timestamp and security review | The promotion is accepted; the receipt records exactly those three values. |
| A certified promotion with no evidence at all | The promotion is refused. The outcome names `test_coverage`, `last_verified` and `security_review` as missing, the prior quality level remains, and no receipt is written. |
| A certified promotion with a security review of `false` | The promotion is refused with "missing security review" — the wording the existing contract test reads. |
| A certified promotion with coverage below 0.85 | The promotion is refused with the coverage and the minimum, as before. |
| A certified promotion with a stale timestamp | The promotion is refused with the freshness window, as before. |
| A promotion to `deprecated` | Unchanged: no evidence is required and no receipt is written. |
| A promotion to a level the component already has | Unchanged: a no-op, no receipt. |
| A promotion to `verified` or `experimental` | The promotion is accepted and the receipt carries only the evidence the operator supplied; absent fields are omitted. |
| An invalid `--last-verified` value | Unchanged: a typed `component-invalid` error naming the value. |
| An unknown target quality | Unchanged: a typed `component-invalid` error naming the value. |
| The catalog | Unchanged: the descriptors keep their evidence, their qualities and their timestamps. |
| The resolver | Unchanged: it still prefers a certified candidate, and now prefers only candidates that earned the level. |
| The registry journal | Unchanged: a refused promotion is journalled as `blocked`, an accepted one as `done`. |
| A receipt written before this change | Unchanged on disk. Nothing rewrites it. |

## Risks

| Risk | Mitigation |
|---|---|
| A caller that relied on the defaults can no longer promote to `certified`. | That is the point of the change: the promotion needed evidence it never had. The refusal names every missing field and the help text says which flag supplies it. |
| A script that only checks the exit code sees 0 for a refused promotion. | The exit code is unchanged from the command's existing refusal contract; the outcome says `promoted: false` and names the missing evidence. Changing the exit code would need the CLI to own its exit code for one command and would split one refusal class in two. |
| The receipt's evidence object can now be smaller. | For a certified promotion it always holds the three required fields. A smaller object is the honest one: it holds what the operator supplied. |
| The coverage is rounded to four decimals in the receipt. | Four decimals on a `0.0`-`1.0` ratio keeps the operator's number and removes the `f32` widening artifact. The gate compares the unrounded value. |
| The catalog's `f32` evidence and the qualify evidence now differ in optionality. | They were always different things: the catalog's evidence is what the catalog asserts about a descriptor, the qualify evidence is what the operator supplied about one promotion. Only the latter can be absent. |
