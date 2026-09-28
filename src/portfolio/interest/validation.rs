//! What may enter Forge.
//!
//! Every transport hands raw importer input to [`validate_snapshot`]
//! and nothing else validates an interest write, so a CLI file, an
//! HTTP body field and a future adapter payload are refused
//! identically and leave the same persisted finding.
//!
//! The load-bearing rule here is the **closed key set**: a snapshot
//! record may only carry [`SNAPSHOT_KEYS`], and its `metrics` object
//! may only carry [`InterestMetric`] labels. That is what keeps an
//! identity field, a raw event, a payment record or a credential out
//! of the store by construction. A permissive deserialization would
//! instead silently drop the extra field and report a clean import,
//! which is the failure mode this package exists to prevent.

use serde_json::Value;

use crate::portfolio::interest::{
    looks_like_email, normalize_timestamp, refusal, Coverage, InterestMetric, InterestRecord,
    InterestWrite, MetricValue, PrivacyMode, SnapshotRefusal, CREDENTIAL_KEYS, IDENTITY_KEYS,
    INTEREST_CONTRACT_VERSION, MAX_METRIC_VALUE, MAX_SNAPSHOTS_PER_IMPORT, MAX_SOURCE_CHARS,
    MAX_SOURCE_REVISION_CHARS, MAX_WINDOW_DAYS, PAYMENT_KEYS, RAW_EVENT_KEYS, SECRET_PREFIXES,
    SNAPSHOT_KEYS,
};

/// Whether a value carries a credential shape.
///
/// The shared policy owns the `key=value` and well-known token
/// detectors, so this reuses [`crate::policy::redact_credentials`] as
/// its first line rather than re-implementing the marker set. The
/// explicit prefix scan catches the token shapes an importer may carry
/// without a surrounding `=`.
pub fn looks_like_secret(value: &str) -> bool {
    if crate::policy::redact_credentials(value) != value {
        return true;
    }
    if value.contains("-----BEGIN") || value.contains("-----begin") {
        return true;
    }
    SECRET_PREFIXES.iter().any(|prefix| value.contains(prefix))
}

/// Describe a rejected value without echoing it. The refusal names
/// the field and the rule; the offending bytes never leave the
/// process, which is what makes it safe to surface in an audit trail.
fn secret_finding(field: &str) -> SnapshotRefusal {
    SnapshotRefusal::new(
        refusal::SECRET,
        format!(
            "{field} was refused because it carries a credential-shaped value; \
             the value itself was not recorded"
        ),
    )
}

/// Name the class of a refused key.
///
/// The four closed deny lists come first so the refusal says *why* the
/// field was dangerous — "identity" reads very differently to an
/// importer debugging a schema than "unknown field". A key on none of
/// the lists is simply outside the closed set.
pub fn classify_refused_key(key: &str) -> &'static str {
    let lower = key.trim().to_ascii_lowercase();
    if IDENTITY_KEYS.contains(&lower.as_str()) {
        return refusal::IDENTITY;
    }
    if RAW_EVENT_KEYS.contains(&lower.as_str()) {
        return refusal::RAW_EVENT;
    }
    if PAYMENT_KEYS.contains(&lower.as_str()) {
        return refusal::PAYMENT;
    }
    if CREDENTIAL_KEYS.contains(&lower.as_str()) {
        return refusal::CREDENTIAL;
    }
    refusal::FIELD_UNKNOWN
}

/// Validate a required, bounded, control-free provenance field.
///
/// Secret-shaped content is refused, not redacted: an evidence store
/// that silently rewrites an import would leave the importer believing
/// it stored what it sent. The two shape checks are the value-side
/// half of the closed-key rule — a raw referrer URL or an address
/// reaches a store through an allow-listed field name often enough to
/// be worth refusing on sight.
fn validate_text(field: &str, raw: &str, max_chars: usize) -> Result<String, SnapshotRefusal> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(SnapshotRefusal::new(
            refusal::PROVENANCE,
            format!("{field} is required"),
        ));
    }
    if looks_like_secret(trimmed) {
        return Err(secret_finding(field));
    }
    if is_raw_url_with_query(trimmed) {
        return Err(SnapshotRefusal::new(
            refusal::RAW_EVENT,
            format!(
                "{field} carries a raw URL query; a per-visitor referrer is not an aggregate \
                 observation and the value itself was not recorded"
            ),
        ));
    }
    if looks_like_email(trimmed) {
        return Err(SnapshotRefusal::new(
            refusal::IDENTITY,
            format!(
                "{field} carries an email address; an aggregate snapshot holds no visitor \
                 identity and the value itself was not recorded"
            ),
        ));
    }
    if trimmed.chars().count() > max_chars {
        return Err(SnapshotRefusal::new(
            refusal::PROVENANCE,
            format!("{field} is longer than {max_chars} characters"),
        ));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(SnapshotRefusal::new(
            refusal::PROVENANCE,
            format!("{field} must not contain control characters"),
        ));
    }
    Ok(trimmed.to_string())
}

/// Whether a string value carries a raw URL with a query string.
///
/// A referrer or a landing page arrives as `https://…?utm_source=…`,
/// and that query is per-visitor data by construction. Only a value
/// that actually parses as an absolute URL is treated this way, so a
/// source named `https-analytics` is not refused.
fn is_raw_url_with_query(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once("://") else {
        return false;
    };
    if scheme.is_empty()
        || !scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
    {
        return false;
    }
    !rest.is_empty() && rest.contains('?')
}

// --- document decoding --------------------------------------------------

/// Decode a versioned import document into unvalidated records.
///
/// The envelope itself is strict: an absent or wrong contract is a
/// document-level refusal rather than a per-record rejection, because
/// a Forge that accepted an unversioned batch could not tell an
/// importer which schema it actually applied. Each record inside is
/// left untouched so [`validate_snapshot`] stays the single gate.
pub fn parse_import_document(raw: &str) -> Result<Vec<(usize, InterestRecord)>, String> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|err| format!("import document is not valid JSON: {err}"))?;
    let Value::Object(mut document) = value else {
        return Err("import document must be a JSON object".to_string());
    };
    match document.remove("contract") {
        Some(Value::String(contract)) if contract == INTEREST_CONTRACT_VERSION => {}
        Some(Value::String(contract)) => {
            return Err(format!(
                "import document declares contract `{contract}`; Forge accepts only \
                 `{INTEREST_CONTRACT_VERSION}`"
            ))
        }
        Some(_) => return Err("import document `contract` must be a string".to_string()),
        None => {
            return Err(format!(
                "import document must declare `contract: {INTEREST_CONTRACT_VERSION}`"
            ))
        }
    }
    let Some(Value::Array(entries)) = document.remove("snapshots") else {
        return Err("import document must carry a `snapshots` array".to_string());
    };
    if entries.is_empty() {
        return Err("import document carries no snapshots".to_string());
    }
    if entries.len() > MAX_SNAPSHOTS_PER_IMPORT {
        return Err(format!(
            "import document carries more than {MAX_SNAPSHOTS_PER_IMPORT} snapshots"
        ));
    }
    if let Some(extra) = document.keys().next() {
        return Err(format!(
            "import document key `{extra}` is not allowed; expected only `contract` and `snapshots`"
        ));
    }
    let mut records = Vec::with_capacity(entries.len());
    for (index, entry) in entries.into_iter().enumerate() {
        let Value::Object(record) = entry else {
            return Err(format!(
                "snapshot {index} must be a JSON object; an aggregate record carries no free-form value"
            ));
        };
        records.push((index, record));
    }
    Ok(records)
}

// --- the single gate ----------------------------------------------------

/// Validate one raw record into a ready-to-write [`InterestWrite`].
///
/// The order of the checks is the policy:
///
/// 1. **The closed key set first.** An identity field, a raw event, a
///    payment record or a credential is refused before any value is
///    read, so nothing dangerous is ever parsed into a string the
///    caller could log.
/// 2. **Then provenance**: project, source, source revision and the
///    declared UTC half-open window.
/// 3. **Then the metric object**: allowlisted keys, non-negative
///    integer counts, bounded magnitude, and the zero rule — a metric
///    map of all zeros is accepted only when the source declares it
///    measured the complete window.
///
/// The returned write carries normalized timestamps and canonical
/// metric ordering, so persistence never re-derives them.
pub fn validate_snapshot(record: &InterestRecord) -> Result<InterestWrite, SnapshotRefusal> {
    // 1. The closed key set. Unknown keys are refused, not ignored:
    //    silently dropping an `email` field would report a clean
    //    import for a record Forge did not store as written.
    for key in record.keys() {
        if SNAPSHOT_KEYS.contains(&key.as_str()) {
            continue;
        }
        if looks_like_secret(key) {
            return Err(secret_finding(&format!("snapshot field `{key}`")));
        }
        return Err(SnapshotRefusal::new(
            classify_refused_key(key),
            format!(
                "snapshot field `{key}` is refused; a snapshot may carry only {}",
                SNAPSHOT_KEYS.join(", ")
            ),
        ));
    }

    // 2. Provenance.
    let project_id = required_project_id(record)?;
    let source = validate_text(
        "source",
        string_field(record, "source")?.unwrap_or_default(),
        MAX_SOURCE_CHARS,
    )?;
    let source_revision = validate_text(
        "source_revision",
        string_field(record, "source_revision")?.unwrap_or_default(),
        MAX_SOURCE_REVISION_CHARS,
    )?;
    let privacy_mode = match string_field(record, "privacy_mode")? {
        Some(raw) => PrivacyMode::parse(raw.trim())
            .map_err(|reason| SnapshotRefusal::new(refusal::PRIVACY_MODE, reason))?,
        None => PrivacyMode::Undeclared,
    };
    let coverage = match string_field(record, "coverage")? {
        Some(raw) => Coverage::parse(raw.trim())
            .map_err(|reason| SnapshotRefusal::new(refusal::COVERAGE, reason))?,
        None => Coverage::Partial,
    };
    let (window_start, window_end) = validate_window(record)?;
    let replaces_source_revision = validate_replacement(record)?;

    // 3. The allowlisted metric counts.
    let metrics = validate_metrics(record)?;

    Ok(InterestWrite {
        project_id,
        source,
        source_revision,
        window_start,
        window_end,
        privacy_mode,
        coverage,
        replaces_source_revision,
        metrics,
    })
}

/// Validate the declared UTC half-open window.
///
/// A timestamp without an offset is refused rather than assumed UTC:
/// the whole freshness rule is "how long ago did this window end",
/// and that question is unanswerable for an instant Forge has to
/// guess.
fn validate_window(record: &InterestRecord) -> Result<(String, String), SnapshotRefusal> {
    let window = |field: &str| -> Result<String, SnapshotRefusal> {
        let raw = string_field(record, field)?.unwrap_or_default();
        normalize_timestamp(field, raw)
            .map_err(|reason| SnapshotRefusal::new(refusal::WINDOW, reason))
    };
    let window_start = window("window_start")?;
    let window_end = window("window_end")?;

    let start = crate::portfolio::interest::parse_timestamp(&window_start)
        .expect("normalized timestamp is parseable");
    let end = crate::portfolio::interest::parse_timestamp(&window_end)
        .expect("normalized timestamp is parseable");
    if end <= start {
        return Err(SnapshotRefusal::new(
            refusal::WINDOW,
            format!(
                "window_end `{window_end}` must be later than window_start `{window_start}`; \
                 an empty or inverted window measures nothing"
            ),
        ));
    }
    let days = (end - start).num_days();
    if days > MAX_WINDOW_DAYS {
        return Err(SnapshotRefusal::new(
            refusal::WINDOW,
            format!(
                "window spans {days} days; an interest window may not exceed {MAX_WINDOW_DAYS}"
            ),
        ));
    }
    Ok((window_start, window_end))
}

/// Validate the optional replacement declaration.
fn validate_replacement(record: &InterestRecord) -> Result<Option<String>, SnapshotRefusal> {
    let declared = match record.get("replaces_source_revision") {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::String(raw)) if raw.trim().is_empty() => return Ok(None),
        Some(Value::String(raw)) => raw.clone(),
        Some(_) => {
            return Err(SnapshotRefusal::new(
                refusal::REPLACEMENT,
                "replaces_source_revision must be a string when it is present",
            ))
        }
    };
    let replaced = validate_text(
        "replaces_source_revision",
        &declared,
        MAX_SOURCE_REVISION_CHARS,
    )?;
    if replaced
        == string_field(record, "source_revision")?
            .unwrap_or_default()
            .trim()
    {
        return Err(SnapshotRefusal::new(
            refusal::REPLACEMENT,
            format!(
                "replaces_source_revision `{replaced}` is this record's own source revision; \
             a snapshot cannot replace itself"
            ),
        ));
    }
    Ok(Some(replaced))
}

/// Validate the `metrics` object against the closed allowlist.
fn validate_metrics(record: &InterestRecord) -> Result<Vec<MetricValue>, SnapshotRefusal> {
    let Some(Value::Object(entries)) = record.get("metrics") else {
        return Err(SnapshotRefusal::new(
            refusal::METRIC_INVALID,
            "snapshot requires a `metrics` object",
        ));
    };
    if entries.is_empty() {
        return Err(SnapshotRefusal::new(
            refusal::METRIC_INVALID,
            "snapshot `metrics` must declare at least one metric",
        ));
    }
    let mut values: Vec<MetricValue> = Vec::with_capacity(entries.len());
    for (key, raw) in entries {
        // The same closed key set applies one level down: `metrics` is
        // where an identity or a per-event payload would hide.
        let metric = match InterestMetric::parse(key) {
            Ok(metric) => metric,
            Err(_) if classify_refused_key(key) == refusal::FIELD_UNKNOWN => {
                return Err(SnapshotRefusal::new(
                    refusal::METRIC_UNKNOWN,
                    format!(
                        "metrics field `{key}` is not a metric; a metric may carry only {}",
                        InterestMetric::labels().join(", ")
                    ),
                ))
            }
            Err(_) => {
                return Err(SnapshotRefusal::new(
                    classify_refused_key(key),
                    format!(
                        "metrics field `{key}` is refused; a metric may carry only {}",
                        InterestMetric::labels().join(", ")
                    ),
                ))
            }
        };
        let value = match raw {
            Value::Number(number) => {
                // A negative count is refused by shape, not clamped:
                // `-1` is not "approximately zero", it is a sign that
                // the producer subtracted something it should not have.
                let Some(count) = number.as_i64() else {
                    return Err(SnapshotRefusal::new(
                        refusal::METRIC_INVALID,
                        format!("metrics `{key}` must be a whole number, not a fraction"),
                    ));
                };
                if count < 0 {
                    return Err(SnapshotRefusal::new(
                        refusal::METRIC_INVALID,
                        format!("metrics `{key}` must not be negative; a count is zero or more"),
                    ));
                }
                if count as u64 > MAX_METRIC_VALUE {
                    return Err(SnapshotRefusal::new(
                        refusal::METRIC_INVALID,
                        format!(
                            "metrics `{key}` exceeds {MAX_METRIC_VALUE}; a windowed count above \
                             that bound is a unit error, not an interest signal"
                        ),
                    ));
                }
                count as u64
            }
            Value::String(_) => {
                return Err(SnapshotRefusal::new(
                    refusal::METRIC_INVALID,
                    format!("metrics `{key}` must be a number, not a string"),
                ))
            }
            _ => {
                return Err(SnapshotRefusal::new(
                    refusal::METRIC_INVALID,
                    format!("metrics `{key}` must be a whole number, not a list or object"),
                ))
            }
        };
        values.push(MetricValue {
            metric: metric.label().to_string(),
            value,
        });
    }
    // Canonical metric order keeps the stored payload independent of
    // the order the importer happened to serialize its object in.
    values.sort_by_key(|value| {
        InterestMetric::parse(&value.metric)
            .ok()
            .map(|metric| metric as usize)
            .unwrap_or(usize::MAX)
    });

    // A zero is a claim. It is a claim the source made about a window
    // it says it measured; a partially measured window reporting all
    // zeros is a collection gap, and storing it would turn a broken
    // provider into a standing statement that nobody was interested.
    if is_partial_coverage(record) && values.iter().all(|value| value.value == 0) {
        return Err(SnapshotRefusal::new(
            refusal::ZERO_UNMEASURED,
            "metrics are all zero and the source declares only `partial` coverage; \
             a zero is only an observation when the source measured the complete window",
        ));
    }
    Ok(values)
}

/// Whether the record declared partial coverage. Read straight from
/// the raw object so the zero rule can run before the value is carried
/// into the write. An unparseable coverage is treated as partial,
/// which is the conservative reading.
fn is_partial_coverage(record: &InterestRecord) -> bool {
    match record.get("coverage") {
        Some(Value::String(raw)) => Coverage::parse(raw.trim())
            .map(|value| value == Coverage::Partial)
            .unwrap_or(true),
        _ => true,
    }
}

fn string_field<'a>(
    record: &'a InterestRecord,
    field: &str,
) -> Result<Option<&'a str>, SnapshotRefusal> {
    match record.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(raw)) => Ok(Some(raw.as_str())),
        Some(_) => Err(SnapshotRefusal::new(
            refusal::PROVENANCE,
            format!("snapshot `{field}` must be a string"),
        )),
    }
}

fn required_project_id(record: &InterestRecord) -> Result<String, SnapshotRefusal> {
    let Some(raw) = string_field(record, "project_id")?.map(str::to_string) else {
        return Err(SnapshotRefusal::new(
            refusal::PROJECT,
            "snapshot requires `project_id`",
        ));
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(SnapshotRefusal::new(
            refusal::PROJECT,
            "snapshot `project_id` is required",
        ));
    }
    if looks_like_secret(trimmed) {
        return Err(secret_finding("snapshot `project_id`"));
    }
    crate::core::validate_project_id(trimmed).map_err(|err| {
        SnapshotRefusal::new(refusal::PROJECT, format!("snapshot `project_id` {err}"))
    })?;
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record() -> InterestRecord {
        json!({
            "project_id": "alethefy",
            "source": "github-analytics",
            "source_revision": "a1b2c3",
            "window_start": "2026-09-01T00:00:00Z",
            "window_end": "2026-09-08T00:00:00Z",
            "privacy_mode": "exact-count",
            "coverage": "complete",
            "metrics": { "unique_visitors": 120, "outbound_cta_clicks": 9 },
        })
        .as_object()
        .expect("object")
        .clone()
    }

    #[test]
    fn the_document_envelope_is_strict() {
        let ok = json!({
            "contract": INTEREST_CONTRACT_VERSION,
            "snapshots": [record()],
        });
        let parsed = parse_import_document(&ok.to_string()).expect("valid document");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, 0);

        let wrong_contract = json!({
            "contract": "forge-portfolio-interest/9.9.9",
            "snapshots": [record()],
        });
        assert!(parse_import_document(&wrong_contract.to_string())
            .expect_err("wrong contract")
            .contains("Forge accepts only"));

        assert!(parse_import_document(r#"{"snapshots":[]}"#)
            .expect_err("absent contract")
            .contains("must declare"));
        assert!(
            parse_import_document(r#"{"contract":"forge-portfolio-interest/0.1.0"}"#)
                .expect_err("absent snapshots")
                .contains("snapshots")
        );
        assert!(parse_import_document(&format!(
            "{{\"contract\":\"{INTEREST_CONTRACT_VERSION}\",\"snapshots\":[]}}"
        ))
        .expect_err("empty batch")
        .contains("no snapshots"));
        let extra_key = serde_json::json!({
            "contract": INTEREST_CONTRACT_VERSION,
            "actor": "x",
            "snapshots": [record()],
        });
        assert!(parse_import_document(&extra_key.to_string())
            .expect_err("extra envelope key")
            .contains("not allowed"));
        assert!(parse_import_document("[]")
            .expect_err("array")
            .contains("JSON object"));
        assert!(parse_import_document("nope")
            .expect_err("garbage")
            .contains("not valid JSON"));
        assert!(parse_import_document(&format!(
            "{{\"contract\":\"{INTEREST_CONTRACT_VERSION}\",\"snapshots\":[1]}}"
        ))
        .expect_err("non-object record")
        .contains("must be a JSON object"));
    }

    #[test]
    fn an_import_is_normalized_to_utc_and_canonical_metric_order() {
        let mut raw = record();
        raw.insert(
            "window_start".to_string(),
            json!("2026-09-01T02:00:00+02:00"),
        );
        let validated = validate_snapshot(&raw).expect("valid snapshot");
        assert_eq!(validated.window_start, "2026-09-01T00:00:00Z");
        assert_eq!(validated.project_id, "alethefy");
        assert_eq!(validated.source, "github-analytics");
        assert_eq!(
            validated
                .metrics
                .iter()
                .map(|m| m.metric.as_str())
                .collect::<Vec<_>>(),
            vec!["unique_visitors", "outbound_cta_clicks"]
        );
        // A defaulted field is honest rather than optimistic.
        let mut minimal = record();
        minimal.remove("privacy_mode");
        minimal.remove("coverage");
        let defaulted = validate_snapshot(&minimal).expect("defaults");
        assert_eq!(defaulted.privacy_mode, PrivacyMode::Undeclared);
        assert_eq!(defaulted.coverage, Coverage::Partial);
    }

    #[test]
    fn identity_payment_event_and_credential_fields_are_refused_by_name() {
        for (field, expected) in [
            ("email", refusal::IDENTITY),
            ("visitor_id", refusal::IDENTITY),
            ("ip_address", refusal::IDENTITY),
            ("raw_events", refusal::RAW_EVENT),
            ("page_url", refusal::RAW_EVENT),
            ("card_number", refusal::PAYMENT),
            ("revenue", refusal::PAYMENT),
            ("api_key", refusal::CREDENTIAL),
            ("session_token", refusal::CREDENTIAL),
            ("engagement_score", refusal::FIELD_UNKNOWN),
        ] {
            let mut raw = record();
            raw.insert(field.to_string(), json!("anything"));
            let err = validate_snapshot(&raw).expect_err(field);
            assert_eq!(err.code, expected, "{field}: {}", err.detail);
            // The value is never echoed back to the caller.
            assert!(!err.detail.contains("anything"), "{field}: {}", err.detail);
        }
        // The same deny list applies inside the metric object, which
        // is where an address would most plausibly hide.
        let mut nested = record();
        nested.insert(
            "metrics".to_string(),
            json!({ "unique_visitors": 1, "email": "a@b.co" }),
        );
        let err = validate_snapshot(&nested).expect_err("metric identity");
        assert_eq!(err.code, refusal::IDENTITY, "{}", err.detail);
        assert!(!err.detail.contains("a@b.co"), "{}", err.detail);
        // A plausible-looking but unlisted metric is refused as an
        // unknown metric, not as an identity field.
        let mut unlisted = record();
        unlisted.insert("metrics".to_string(), json!({ "signups": 3 }));
        assert_eq!(
            validate_snapshot(&unlisted).expect_err("unlisted").code,
            refusal::METRIC_UNKNOWN
        );
    }

    #[test]
    fn a_metric_value_must_be_a_bounded_non_negative_count() {
        for value in [
            json!(-1),
            json!("120"),
            json!(1.5),
            json!([1, 2]),
            json!({"a": 1}),
        ] {
            let mut raw = record();
            raw.insert("metrics".to_string(), json!({ "unique_visitors": value }));
            assert_eq!(
                validate_snapshot(&raw).expect_err("bad value").code,
                refusal::METRIC_INVALID
            );
        }
        let mut huge = record();
        huge.insert(
            "metrics".to_string(),
            json!({ "unique_visitors": MAX_METRIC_VALUE + 1 }),
        );
        let err = validate_snapshot(&huge).expect_err("unit error");
        assert_eq!(err.code, refusal::METRIC_INVALID);
        assert!(err.detail.contains("unit error"), "{err}");

        let mut empty = record();
        empty.insert("metrics".to_string(), json!({}));
        assert!(validate_snapshot(&empty)
            .expect_err("empty")
            .detail
            .contains("at least one"));
        let mut absent = record();
        absent.remove("metrics");
        assert!(validate_snapshot(&absent)
            .expect_err("absent")
            .detail
            .contains("`metrics`"));
    }

    #[test]
    fn a_zero_is_only_an_observation_over_a_complete_window() {
        let mut complete = record();
        complete.insert(
            "metrics".to_string(),
            json!({ "unique_visitors": 0, "outbound_cta_clicks": 0 }),
        );
        assert!(validate_snapshot(&complete).is_ok());

        let mut partial = complete.clone();
        partial.insert("coverage".to_string(), json!("partial"));
        let err = validate_snapshot(&partial).expect_err("all-zero partial window");
        assert_eq!(err.code, refusal::ZERO_UNMEASURED);
        assert!(err.detail.contains("all zero"), "{err}");
        assert!(err.detail.contains("complete window"), "{err}");

        // A partial window with a non-zero value is still honest: it
        // is labelled partial, and the label travels with it.
        let mut partial_with_signal = complete;
        partial_with_signal.insert("coverage".to_string(), json!("partial"));
        partial_with_signal.insert(
            "metrics".to_string(),
            json!({ "unique_visitors": 0, "outbound_cta_clicks": 4 }),
        );
        assert!(validate_snapshot(&partial_with_signal).is_ok());
    }

    #[test]
    fn the_window_must_be_an_ordered_utc_half_open_interval() {
        let mut naive = record();
        naive.insert("window_start".to_string(), json!("2026-09-01T00:00:00"));
        assert_eq!(
            validate_snapshot(&naive).expect_err("naive").code,
            refusal::WINDOW
        );

        let mut inverted = record();
        inverted.insert("window_end".to_string(), json!("2026-08-24T00:00:00Z"));
        assert!(validate_snapshot(&inverted)
            .expect_err("inverted")
            .detail
            .contains("must be later"));

        let mut empty = record();
        empty.insert("window_end".to_string(), json!("2026-09-01T00:00:00Z"));
        assert!(validate_snapshot(&empty)
            .expect_err("empty window")
            .detail
            .contains("empty or inverted"));

        let mut long = record();
        long.insert("window_end".to_string(), json!("2028-09-01T00:00:00Z"));
        assert!(validate_snapshot(&long)
            .expect_err("lifetime total")
            .detail
            .contains("may not exceed"));

        for missing in ["window_start", "window_end"] {
            let mut raw = record();
            raw.remove(missing);
            assert_eq!(
                validate_snapshot(&raw).expect_err(missing).code,
                refusal::WINDOW
            );
        }
    }

    #[test]
    fn a_snapshot_cannot_replace_its_own_revision() {
        let mut raw = record();
        raw.insert("replaces_source_revision".to_string(), json!("a1b2c3"));
        let err = validate_snapshot(&raw).expect_err("self replacement");
        assert_eq!(err.code, refusal::REPLACEMENT);
        assert!(err.detail.contains("cannot replace itself"), "{err}");

        let mut named = record();
        named.insert("replaces_source_revision".to_string(), json!("a1b2c2"));
        assert_eq!(
            validate_snapshot(&named)
                .expect("replacement")
                .replaces_source_revision
                .as_deref(),
            Some("a1b2c2")
        );

        let mut wrong_type = record();
        wrong_type.insert("replaces_source_revision".to_string(), json!(7));
        assert_eq!(
            validate_snapshot(&wrong_type).expect_err("type").code,
            refusal::REPLACEMENT
        );
    }

    #[test]
    fn project_identity_and_provenance_are_validated() {
        let mut unknown = record();
        unknown.insert("project_id".to_string(), json!("Alethefy"));
        assert_eq!(
            validate_snapshot(&unknown).expect_err("project id").code,
            refusal::PROJECT
        );

        let mut no_project = record();
        no_project.remove("project_id");
        assert_eq!(
            validate_snapshot(&no_project).expect_err("project_id").code,
            refusal::PROJECT
        );
        for missing in ["source", "source_revision"] {
            let mut raw = record();
            raw.remove(missing);
            assert_eq!(
                validate_snapshot(&raw).expect_err(missing).code,
                refusal::PROVENANCE
            );
        }

        let mut credential = record();
        credential.insert(
            "source".to_string(),
            json!("ghp_abcdefghijklmnopqrstuvwxyz0123"),
        );
        let err = validate_snapshot(&credential).expect_err("credential source");
        assert_eq!(err.code, refusal::SECRET);
        assert!(!err.detail.contains("abcdef"), "{err}");

        for (field, value, code) in [
            ("privacy_mode", "anonymised", refusal::PRIVACY_MODE),
            ("coverage", "sampled", refusal::COVERAGE),
        ] {
            let mut raw = record();
            raw.insert(field.to_string(), json!(value));
            let err = validate_snapshot(&raw).expect_err(field);
            assert_eq!(err.code, code, "{err}");
            assert!(err.detail.contains("expected one of"), "{err}");
        }
    }

    #[test]
    fn a_credential_shaped_field_name_is_refused_without_echoing_it() {
        let mut raw = record();
        raw.insert("token=abcdef123456".to_string(), json!("x"));
        let err = validate_snapshot(&raw).expect_err("secret field name");
        assert_eq!(err.code, refusal::SECRET);
        assert!(!err.detail.contains("abcdef"), "{}", err.detail);
    }

    #[test]
    fn a_provenance_field_never_carries_a_referrer_or_an_address() {
        let mut referrer = record();
        referrer.insert(
            "source_revision".to_string(),
            json!("https://x.example/?utm=a"),
        );
        let err = validate_snapshot(&referrer).expect_err("raw referrer");
        assert_eq!(err.code, refusal::RAW_EVENT);
        assert!(err.detail.contains("raw URL query"), "{err}");
        assert!(!err.detail.contains("utm=a"), "{err}");

        let mut address = record();
        address.insert("source".to_string(), json!("ops@example.com"));
        let err = validate_snapshot(&address).expect_err("email");
        assert_eq!(err.code, refusal::IDENTITY);
        assert!(err.detail.contains("visitor identity"), "{err}");
        assert!(!err.detail.contains("ops@example.com"), "{err}");

        // A source name that merely looks URL-ish is still a name.
        let mut name = record();
        name.insert("source".to_string(), json!("https-analytics"));
        assert!(validate_snapshot(&name).is_ok());
    }

    #[test]
    fn the_url_shape_detector_only_fires_on_a_real_url_query() {
        assert!(is_raw_url_with_query("https://example.com/?utm_source=x"));
        assert!(!is_raw_url_with_query("https-analytics"));
        assert!(!is_raw_url_with_query("github-analytics"));
    }
}
