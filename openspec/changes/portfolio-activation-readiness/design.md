# Design: portfolio-activation-readiness

## Implementation boundary

Repository: `forge`, Rust 1.87+, existing Cargo workspace and SQLite
registry. Files to **add**:

- `src/portfolio/interest/activation.rs` — the whole domain: vocabulary,
  selection rule, evaluation, threshold and window validation, unit tests.
- `tests/portfolio_activation_cli_contract.rs` — CLI and in-process API
  contract.
- `tests/portfolio_activation_cross_surface.rs` — read-only and
  no-billing-surface regression.

Files to **change** (additive only):

| File | Change |
|---|---|
| `src/portfolio/interest/mod.rs` | `pub mod activation;` plus re-exports of `Readiness`, `NotReadyReason`, `ReadinessReason`, `ReadinessEvidence`, `ReadinessVerdict`, `ActivationReport`, `build_readiness`, `validate_threshold`, `parse_window`, `ACTIVATION_CONTRACT_VERSION` |
| `src/portfolio/interest_report.rs` | add `pub fn activation_readiness(...)` as a second read-only entry point beside `compare_projects` and `interest_trend` |
| `src/registry/interest/mod.rs` | add `pub fn interest_snapshot_counts(&self, project_id: &str) -> Result<(usize, usize), ForgeError>` returning `(total, current)` by two `COUNT(*)` queries |
| `src/core/mod.rs` | add `ForgeError::PortfolioActivationNotReady { reason: String }` and its `code()` arm |
| `src/main.rs` | add `PortfolioActivationCommands`, the `Activation` variant on `PortfolioCommands`, `cmd_portfolio_activation`, `render_activation`, and `parse_activation_window` |
| `src/api/mod.rs` | add `Route::InterestReadiness`, its matcher, its `required_permission` arm, its fleet `authorize` arm, its dispatch arm, `handle_interest_readiness`, and an `err_status` row mapping `portfolio-activation-not-ready` to `409` with a comment that the route never constructs it (it answers `200`), so a future caller cannot silently become a `500` |

Do **not** touch: the interest snapshot schema or contract, the share or
publication packages, generation, publish, `src/registry/mod.rs`
migrations, the MCP tool list, or the portal. Do not add a payment,
subscription, entitlement, customer, price or revenue type anywhere.

## Language and runtime

Rust 1.87+, `rustfmt` defaults, no new dependency (the readiness
projection needs `chrono` and `serde`, both already present). Primary
commands:

```text
cargo fmt --all -- --check
cargo build
cargo test --lib -- portfolio::interest registry::interest
cargo test --test portfolio_activation_cli_contract
cargo test --test portfolio_activation_cross_surface
cargo clippy --all-targets -- -D warnings
cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain
openspec validate --all --strict --no-interactive
node scripts/check-openspec-change-names.mjs
git diff --check
```

`cargo fmt` and `cargo clippy` must be diffed against the pre-change
baseline, not run blind: this repository carries pre-existing
formatting drift in `src/gate/evidence.rs`,
`src/publish/{fleet,jenkins}.rs`,
`src/portfolio/share/validation.rs`, `tests/gate_contract.rs`,
`tests/gate_cross_surface.rs` and
`tests/publish_queue_status_contract.rs`, and pre-existing clippy
locations in `src/api/ui/auth.rs`, `src/gate/evidence.rs`,
`src/portfolio/share/validation.rs`, `src/publish/fleet.rs` and
`src/publish/mod.rs`. Preserve both baselines exactly.

## Ownership and shared code

Forge owns the readiness verdict, its threshold vocabulary and its
refusal reasons. A selected product owns entitlement, pricing, checkout
and conversion measurement. The analytics provider owns collection and
identity policy. This package contacts no provider, persists nothing,
and reads only snapshots another package already imported.

## Domain model

### Contract version

```rust
pub const ACTIVATION_CONTRACT_VERSION: &str = "forge-portfolio-activation/0.1.0";
```

### Verdict vocabulary

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Readiness { Ready, NotReady }
```

`label()` → `"ready"` | `"not-ready"`. `parse()` accepts exactly those
two labels and refuses anything else. `is_ready()` is true only for
`Ready`. `ALL` is in declaration order.

### Reason vocabulary

Eight reasons, and this declaration order **is** the report order:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotReadyReason {
    ThresholdNotDeclared, // "threshold-not-declared"
    NoEvidence,           // "no-evidence"
    SupersededOnly,       // "superseded-only"
    NoCurrentWindow,      // "no-current-window"
    StaleWindow,          // "stale-window"
    InexactPrivacyMode,   // "inexact-privacy-mode"
    PartialCoverage,      // "partial-coverage"
    BelowThreshold,       // "below-threshold"
}
```

Each reason has `label()` and a `detail(facts) -> String` that names the
condition using only already-validated values (window, source, privacy
mode, coverage, value, threshold), never an unvalidated input. Exact
detail strings, which tests assert on:

| Reason | Detail |
|---|---|
| `ThresholdNotDeclared` | `no threshold was declared; readiness requires an operator-declared threshold` |
| `NoEvidence` | `project `{id}` has no stored interest snapshot; Forge reports no figure rather than a zero` |
| `SupersededOnly` | `every stored snapshot for `{id}` has been superseded; history is not current evidence` |
| `NoCurrentWindow` | `no current window reports `{metric}`` (append ` matching `{start}`..`{end}`` when a window was declared, and ` from source `{source}`` when a source was declared) |
| `StaleWindow` | `window `{start}`..`{end}` ended {n} day(s) ago and reads as stale against a {d} day bound` |
| `InexactPrivacyMode` | `privacy mode `{mode}` is not an exact count; the figure is a floor or an undeclared measurement, not a headcount` |
| `PartialCoverage` | `coverage is `partial`; the source measured only part of the window, so the figure is not a complete measurement` |
| `BelowThreshold` | `value {value} is below the declared threshold {threshold}` |

### Verdict shapes

```rust
/// A declared period, both sides normalized to UTC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequestedWindow {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessReason {
    pub reason: NotReadyReason,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessEvidence {
    pub snapshot_id: i64,
    pub window_start: String,   // normalized UTC
    pub window_end: String,     // normalized UTC
    pub source: String,
    pub source_revision: String,
    pub privacy_mode: String,   // PrivacyMode::label()
    pub coverage: String,       // Coverage::label()
    pub freshness: String,      // Freshness::label()
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessVerdict {
    pub project_id: String,
    pub readiness: Readiness,
    pub metric: String,
    pub threshold: Option<u64>,
    pub reasons: Vec<ReadinessReason>, // fixed order, empty iff Ready
    pub evidence: Option<ReadinessEvidence>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActivationReport {
    pub metric: String,
    pub threshold: Option<u64>,
    pub stale_after_days: i64,
    pub requested_window: Option<RequestedWindow>,
    pub requested_source: Option<String>,
    pub verdicts: Vec<ReadinessVerdict>, // project id ascending
    pub ready_count: usize,
    pub not_ready_count: usize,
}
```

`ActivationReport::is_ready()` is `!verdicts.is_empty() && not_ready_count == 0`.
An empty verdict list is never `ready`; the orchestration refuses an
empty evaluation set before it can occur (see Failure policy).
`ActivationReport::ready_count()` and `not_ready_count()` are derived
from `verdicts`, and `ActivationReport::verdict_reason_labels()` returns
`` `{project_id}:{reason}` `` for every reason in every verdict,
deduplicated and sorted, for the bounded stderr line.

### Candidate selection rule

Given `project_id`, `metric`, optional `source` and optional
`(window_start, window_end)`:

1. Collect the project's **current** snapshots (`state == Accepted`).
   Persisted evidence order is `(window_start, window_end, id)` ascending.
2. Keep only snapshots whose `metrics` contain `metric` with a value.
3. If `source` was declared, keep only snapshots whose `source` equals it.
4. If a window was declared, keep only snapshots whose
   `(window_start, window_end)` equals the **normalized** declared pair.
5. If the candidate list is empty → `NoCurrentWindow`.
6. Otherwise select the **maximum by `(window_end, id)`**. When several
   snapshots share that maximum `window_end` — different sources, or a
   re-import — the highest `id` wins. Selection never falls back to an
   older window.

When the selected window is also reported by a different source,
`notes` gains one entry per additional source, formatted verbatim as:

```text
another source `{source}` also reports this window; the verdict rests on source `{chosen_source}` revision `{chosen_revision}`
```

The selected snapshot is the `evidence`. Every remaining condition is
evaluated against that one snapshot. **There is no fallback to an older
window on failure** — falling back would present older evidence as
current readiness, which is the failure mode this package exists to
prevent. Rationale is recorded in the decision ledger.

### Evaluation

`reasons` is built in the fixed declaration order, each condition
independent, so multiple conditions are reported together:

| # | Condition | Reason appended |
|---|---|---|
| 1 | `threshold.is_none()` | `ThresholdNotDeclared` |
| 2 | `(total, current) == (0, _)` | `NoEvidence` |
| 3 | `total > 0 && current == 0` | `SupersededOnly` |
| 4 | no candidate selected (conditions 2 and 3 excluded) | `NoCurrentWindow` |
| 5 | candidate exists and is `Freshness::Stale` | `StaleWindow` |
| 6 | candidate exists and `privacy_mode != "exact-count"` | `InexactPrivacyMode` |
| 7 | candidate exists and `coverage != "complete"` | `PartialCoverage` |
| 8 | candidate exists, `threshold = Some(t)`, `value < t` | `BelowThreshold` |

The exact control flow, which an implementer should follow literally:

```rust
let mut reasons = Vec::new();
if threshold.is_none() {
    reasons.push(not_ready_reason(ThresholdNotDeclared, project_id, metric));
}
let candidate = if counts.0 == 0 {
    reasons.push(not_ready_reason(NoEvidence, project_id, metric));
    None
} else if counts.1 == 0 {
    reasons.push(not_ready_reason(SupersededOnly, project_id, metric));
    None
} else {
    match select_candidate(snapshots, metric, source, requested_window) {
        Some(snapshot) => Some(snapshot),
        None => {
            reasons.push(not_ready_reason(NoCurrentWindow, project_id, metric));
            None
        }
    }
};
if let Some(snapshot) = candidate {
    if snapshot.freshness(now, stale_after_days) == Freshness::Stale {
        reasons.push(stale_reason(snapshot, now, stale_after_days));
    }
    if snapshot.privacy_mode != PrivacyMode::ExactCount.label() {
        reasons.push(not_ready_reason(InexactPrivacyMode, project_id, metric));
    }
    if snapshot.coverage != Coverage::Complete.label() {
        reasons.push(not_ready_reason(PartialCoverage, project_id, metric));
    }
    if let Some(threshold) = threshold {
        let value = snapshot.metric(metric).expect("candidates report the metric");
        if value < threshold {
            reasons.push(below_threshold_reason(value, threshold));
        }
    }
}
// reasons is already in the fixed declaration order by construction.
let readiness = if reasons.is_empty() { Readiness::Ready } else { Readiness::NotReady };
```

`readiness` is `Ready` iff `reasons` is empty. `evidence` is `None`
for the `NoEvidence`, `SupersededOnly` and `NoCurrentWindow` paths, and
`Some` for every path where a candidate was selected. Because the
pushes happen in the table's order, `reasons` needs no sort — but the
implementation must not reorder the pushes, and the
`the_reason_vocabulary_is_closed_and_in_report_order` test pins that.

Note that a threshold of `0` is meaningful and legal: it asserts "any
exact, fresh, complete measurement counts", and an all-zero complete
window then satisfies it. A `partial` source reporting all zeros can
never reach readiness because `portfolio-interest-snapshots` refuses to
store such a snapshot in the first place.

### Calibration for an implementer

These worked examples pin the behaviour; use them as fixtures.

| Evidence (one current window unless stated) | Threshold | Verdict |
|---|---|---|
| `exact-count`, `complete`, `current`, `unique_visitors = 120`, window `2026-09-01..2026-09-08`, `now = 2026-09-20` (12 days) | `100` | `ready`, no reasons |
| same, `unique_visitors = 40` | `100` | `not-ready`, `[below-threshold]` |
| `lower-bound`, `complete`, `current`, `120` | `100` | `not-ready`, `[inexact-privacy-mode]` |
| `exact-count`, `partial`, `current`, `120` | `100` | `not-ready`, `[partial-coverage]` |
| `exact-count`, `complete`, `stale`, `120` (window ended 28 days before `now`) | `100` | `not-ready`, `[stale-window]` |
| `exact-count`, `complete`, `current`, `120` | absent | `not-ready`, `[threshold-not-declared]` |
| `lower-bound`, `partial`, window ended 28 days before `now`, `120` | absent | `not-ready`, `[threshold-not-declared, stale-window, inexact-privacy-mode, partial-coverage]` |
| no snapshots | `100` | `not-ready`, `[no-evidence]` |
| one superseded snapshot only | `100` | `not-ready`, `[superseded-only]` |
| one current window reporting only `outbound_cta_clicks` | `100` | `not-ready`, `[no-current-window]` |

## Contract

### Domain function signatures

```rust
// src/portfolio/interest/activation.rs
pub fn validate_threshold(threshold: u64) -> Result<u64, String>;
pub fn parse_window(raw: &str) -> Result<(String, String), String>;
pub fn build_readiness(
    project_id: &str,
    metric: InterestMetric,
    threshold: Option<u64>,
    source: Option<&str>,
    requested_window: Option<(&str, &str)>,
    counts: (usize, usize),                 // (total, current)
    snapshots: &[InterestSnapshot],         // current snapshots only
    now: DateTime<Utc>,
    stale_after_days: i64,
) -> ReadinessVerdict;

// src/portfolio/interest_report.rs
pub fn activation_readiness(
    registry: &Registry,
    project_ids: &[String],
    metric: InterestMetric,
    threshold: Option<u64>,
    source: Option<&str>,
    requested_window: Option<(&str, &str)>,
    stale_after_days: i64,
    now: DateTime<Utc>,
) -> Result<ActivationReport, ForgeError>;

// src/registry/interest/mod.rs
pub fn interest_snapshot_counts(&self, project_id: &str) -> Result<(usize, usize), ForgeError>;
```

`activation_readiness` bound-checks `stale_after_days` with the existing
`bound_stale_after_days`, calls `Registry::require_project`
(`src/registry/portfolio.rs`, already `pub(crate)`) for every id before reading
anything (so a typo cannot produce a silently narrowed report), and
refuses an empty `project_ids` with
`PortfolioInterestInvalid { reason: "readiness needs at least one project to evaluate" }`.

### CLI grammar

```text
forge portfolio activation readiness [PROJECT] \
    --metric <METRIC> \
    [--min-value <COUNT>] \
    [--source <SOURCE>] \
    [--window <START>..<END>] \
    [--stale-after-days <DAYS>]
```

| Argument | Type | Required | Default | Bound / refusal |
|---|---|---|---|---|
| `PROJECT` | positional `String` | no | `""` = every registered project | unknown id → `unknown-project`; empty registry → `portfolio-interest-invalid` |
| `--metric` | `String` | **yes** | — | unlisted → `portfolio-interest-invalid` naming the allowlist |
| `--min-value` | `u64` | no | none | `> 1_000_000_000` → `portfolio-interest-invalid`; absent is the `threshold-not-declared` verdict, not an error |
| `--source` | `String` | no | none | empty/whitespace-only after trimming → `portfolio-interest-invalid` |
| `--window` | `String` | no | none | must be `<START>..<END>` with each side RFC 3339; unparseable, missing `..`, or `END <= START` → `portfolio-interest-invalid` |
| `--stale-after-days` | `i64` | no | `DEFAULT_STALE_AFTER_DAYS` (14) | outside `1..=365` → `portfolio-interest-invalid` |

`--metric` is required: readiness is always about one declared signal.
`--min-value` is optional because its absence is a *verdict*, not an
input error — this asymmetry is deliberate and test-pinned.

Both `--window` timestamps normalize through the existing
`interest::normalize_timestamp`, so an offset form such as
`2026-09-01T02:00:00+02:00` matches a stored `2026-09-01T00:00:00Z`.

### Exit code

Mirror `cmd_fleet_online` exactly: build the `Output`, print it, then
return the typed error when the gate is not met.

```rust
let output = as_output(format, human, json);
if !report.is_ready() {
    match &output {
        Output::Human(text) => println!("{text}"),
        Output::Json(value) => println!("{}", serde_json::to_string_pretty(value).unwrap()),
    }
    return Err(ForgeError::PortfolioActivationNotReady {
        reason: format!(
            "activation readiness: {} of {} project(s) are not ready ({})",
            report.not_ready_count,
            report.verdicts.len(),
            report.verdict_reason_labels().join(", "),
        ),
    });
}
Ok(output)
```

- Exit `0`: every evaluated project is `ready`.
- Exit `1`: at least one is `not-ready`. The report **is** printed to
  stdout (this is the documented `fleet online` exception to "a refusal
  prints nothing on stdout") and the typed error goes to stderr.
- Exit `1`, stdout empty: any input error in the table above.

`report.verdict_reason_labels()` is a small helper returning
`` `{project_id}:{reason}` `` for each distinct reason across all
verdicts, deduplicated and sorted, so the stderr line is bounded and
deterministic.

### Human output

Exactly this shape, one block per verdict in `verdicts` order:

```text
activation readiness (forge-portfolio-activation/0.1.0): metric=unique_visitors threshold=100 stale_after_days=14
  alethefy  ready      window=2026-09-01T00:00:00Z..2026-09-08T00:00:00Z source=github-analytics revision=wk-36-a privacy=exact-count coverage=complete freshness=current value=120
  beta      not-ready
      stale-window: window 2026-08-25T00:00:00Z..2026-09-01T00:00:00Z ended 28 day(s) ago and reads as stale against a 14 day bound
      below-threshold: value 40 is below the declared threshold 100
summary: ready=1 not-ready=1
```

When `threshold` is `None` the header prints `threshold=none`. A verdict
with no evidence prints only its `project`/`readiness` line and its
reasons. A verdict with evidence prints `window=… source=… revision=…
privacy=… coverage=… freshness=… value=…`. `notes` are printed as
additional four-space-indented lines after the reasons.

### JSON output (`--format json`)

```json
{
  "contract": "forge-portfolio-activation/0.1.0",
  "generated_at": "<rfc3339>",
  "activation": {
    "metric": "unique_visitors",
    "threshold": 100,
    "stale_after_days": 14,
    "requested_window": { "start": "2026-09-01T00:00:00Z", "end": "2026-09-08T00:00:00Z" },
    "requested_source": null,
    "ready": false,
    "ready_count": 1,
    "not_ready_count": 1,
    "verdicts": [
      {
        "project_id": "alethefy",
        "readiness": "ready",
        "metric": "unique_visitors",
        "threshold": 100,
        "reasons": [],
        "evidence": {
          "snapshot_id": 1,
          "window_start": "2026-09-01T00:00:00Z",
          "window_end": "2026-09-08T00:00:00Z",
          "source": "github-analytics",
          "source_revision": "wk-36-a",
          "privacy_mode": "exact-count",
          "coverage": "complete",
          "freshness": "current",
          "value": 120
        },
        "notes": []
      }
    ]
  }
}
```

`threshold` and `requested_window` are `null` when not declared.
`evidence` is `null` for `no-evidence`, `superseded-only` and
`no-current-window`.

The envelope is composed explicitly with `serde_json::json!` rather than
`serde_json::to_value(&report)`, exactly as the other portfolio commands
do. That is where `generated_at`, `contract` and the derived `ready`
boolean come from: `ready` is `activation_readiness(..)?.is_ready()`, and
it is deliberately not a field on `ActivationReport`, so the report
cannot carry a `ready` flag that disagrees with `not_ready_count`.

### API contract

```text
GET /v1/interest/readiness?project=<id>&metric=<m>&min_value=<n>&source=<s>&window=<start>..<end>&stale_after_days=<d>
GET /v1/interest/readiness?projects=<a,b,c>&metric=<m>...
```

| Parameter | Behaviour |
|---|---|
| `project` | one id |
| `projects` | comma-separated ids, each trimmed, blanks dropped |
| neither | every registered project |
| both | `400 api-invalid` |
| `metric` | **required**; missing or unlisted → `400 api-invalid` |
| `min_value` | optional `u64`; non-integer or `> 1_000_000_000` → `400 api-invalid`. The CLI flag stays `--min-value`; the query parameter follows the existing underscore convention (`stale_after_days`) |
| `source` | optional |
| `window` | optional `<start>..<end>`; decoded with the existing `percent_decode` helper in `src/api/mod.rs`, because the value contains `..` |
| `stale_after_days` | optional `i64`, bounded as in the CLI |
| anything else | `400 api-invalid` naming the parameter |

Response: `200` with

```json
{ "contract": "forge-portfolio-activation/0.1.0",
  "interest": { "activation": <the object above, without "generated_at"> } }
```

The route returns `200` for **both** verdicts and never constructs
`PortfolioActivationNotReady`; only the CLI has a gate exit code. An
unknown project is `400 unknown-project`, matching the other interest
read routes. Every other status comes from the existing `err_status`
mapping.

## Failure and boundary policy

| Case | Result |
|---|---|
| No snapshots for the project | `not-ready`, reason `no-evidence`; never a zero |
| Only superseded revisions | `not-ready`, reason `superseded-only` |
| Current windows exist but none reports the metric | `not-ready`, reason `no-current-window` |
| Declared `--window` matches no current window | `not-ready`, reason `no-current-window`, detail naming the requested window |
| Declared `--source` reports nothing for the metric | `not-ready`, reason `no-current-window`, detail naming the source |
| Selected window is stale | `not-ready`, reason `stale-window` |
| `privacy_mode` is `lower-bound` or `undeclared` | `not-ready`, reason `inexact-privacy-mode` |
| `coverage` is `partial` | `not-ready`, reason `partial-coverage` |
| Value below the declared threshold | `not-ready`, reason `below-threshold` |
| No threshold declared | `not-ready`, reason `threshold-not-declared` |
| Several conditions hold | every reason reported, in the fixed order |
| Threshold `> 1_000_000_000` | `portfolio-interest-invalid`, stdout empty |
| Malformed or inverted `--window` | `portfolio-interest-invalid`, stdout empty |
| Blank `--source` | `portfolio-interest-invalid`, stdout empty |
| Unknown or unlisted metric | `portfolio-interest-invalid` naming the allowlist |
| Unknown project | `unknown-project` |
| `--stale-after-days` out of `1..=365` | `portfolio-interest-invalid` |
| Every registered project, registry empty | `portfolio-interest-invalid`, stdout empty |
| Any caller expecting a plan, price or entitlement | absent by construction |

## Verification oracle

Add unit tests in `src/portfolio/interest/activation.rs`:

`the_readiness_vocabulary_is_closed`,
`the_reason_vocabulary_is_closed_and_in_report_order`,
`thresholds_are_bounded_and_zero_is_legal`,
`a_declared_window_parses_normalizes_and_requires_order`,
`no_evidence_is_not_ready_and_invents_no_zero`,
`superseded_only_evidence_is_not_ready`,
`a_current_window_without_the_metric_is_no_current_window`,
`the_latest_reported_window_is_the_one_evaluated`,
`a_declared_window_narrows_the_evidence`,
`a_declared_source_narrows_the_evidence`,
`a_stale_window_is_not_ready_even_above_threshold`,
`a_lower_bound_figure_is_never_a_headcount`,
`an_undeclared_privacy_mode_is_not_ready`,
`partial_coverage_is_not_ready`,
`an_all_zero_complete_window_can_be_ready_at_threshold_zero`,
`below_threshold_is_not_ready_at_the_exact_boundary`,
`every_withholding_condition_is_reported_not_collapsed`,
`an_absent_threshold_is_a_verdict_not_an_error`,
`the_two_sources_sharing_a_window_add_a_note_and_settle_on_the_latest`,
`the_verdict_is_deterministic_for_repeated_reads`.

Also add to `src/registry/interest/mod.rs`:
`snapshot_counts_distinguish_no_evidence_from_superseded_only`.

Add `tests/portfolio_activation_cli_contract.rs`:

`activation_help_advertises_the_command_and_its_flags`,
`a_ready_project_reports_ready_prints_the_report_and_exits_zero`,
`a_not_ready_project_prints_the_report_and_exits_non_zero`,
`every_not_ready_reason_is_reachable_and_named`,
`an_absent_threshold_prints_a_verdict_and_still_exits_zero_when_ready`,
`an_out_of_range_threshold_is_a_typed_refusal_with_empty_stdout`,
`a_malformed_or_inverted_window_is_a_typed_refusal`,
`a_blank_source_is_a_typed_refusal`,
`an_unknown_project_and_an_unknown_metric_are_typed_refusals`,
`an_empty_registry_is_a_typed_refusal`,
`the_fleet_form_evaluates_every_project_in_id_order`,
`readiness_json_carries_the_verdict_in_both_cases`,
`every_readiness_route_demands_an_admin_session`,
`the_api_answers_200_with_the_verdict_for_ready_and_not_ready`,
`readiness_query_parameters_are_bounded_and_typed`,
`the_stale_after_days_bound_is_validated_rather_than_clamped`.

Add `tests/portfolio_activation_cross_surface.rs`:

`readiness_leaves_every_table_and_the_journal_unchanged`,
`no_activation_or_billing_field_reaches_the_share_manifest_the_portfolio_projection_or_the_fleet_list`,
`no_price_plan_subscription_entitlement_checkout_or_revenue_field_exists_in_any_projection`,
`paid_interest_events_is_an_aggregate_signal_and_grants_no_access`,
`a_registry_written_before_this_package_is_read_as_is`,
`existing_interest_surfaces_are_unchanged`.

Evidence procedure: run the commands above, diff `cargo fmt` and
`cargo clippy` output against the stashed pre-change baseline
(`git stash push -u -- src tests`, capture, restore), and run the CLI
and loopback-HTTP smoke paths. Record the exact counts and the exact
deferred claims. No analytics provider is contacted, no product is
activated, and no billing work is begun.

## Decision ledger

- **Forge owns the gate; the product owns the monetization.** Moving
  either across that line is a separate, product-owned proposal.
- **Readiness evaluates the latest reported window and never falls back.**
  Falling back to an older window when the newest one is stale, inexact
  or partial would present older evidence as current readiness, which is
  the failure mode this package exists to prevent. An operator who wants
  a specific period declares `--window`.
- **The threshold has no default.** A default would be Forge inventing a
  commercial judgement it does not own; absence is a reported reason.
- **A threshold of `0` is legal and meaningful** — it asks only whether
  exact, fresh, complete evidence exists.
- **Absence of evidence is `not-ready`, never an optimistic default.** A
  project nobody measured is not a project nobody wanted.
- **The verdict is a closed set of named reasons, not a score.** No
  percentage, ranking or "best project" output exists, because any of
  those would invite a decision the evidence does not support.
- **`paid_interest_events` remains an aggregate signal.** It is not a
  payment record, not a customer identity, and grants nothing.
- **The API always answers `200`.** A gate exit code is a CLI concept;
  an HTTP client reads the verdict from the body.
- **This package adds no stored state.** It reads the existing tables,
  so a registry written before it is read as-is.
