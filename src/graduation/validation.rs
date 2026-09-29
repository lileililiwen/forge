//! What may enter Forge.
//!
//! Every transport hands a raw artifact to [`parse_artifact`] and
//! [`validate_graduation`], so a CLI file, a stdin stream and a future
//! adapter payload are refused identically and leave no project state.
//!
//! The load-bearing rules are:
//!
//! 1. **The bounded read happens first.** [`read_artifact`] refuses an
//!    oversized source before it is fully read, and a non-UTF-8 or
//!    non-object source before any value is parsed.
//! 2. **The closed key set.** Every object level may carry only its
//!    named keys, so an identity field, a raw event, a payment record
//!    or a credential is refused before a value is read into a string
//!    the caller could log. A permissive deserialization would instead
//!    silently drop the extra field and report a clean import, which is
//!    the failure this package exists to prevent.
//! 3. **The value shape.** Under an allowed name, a credential, an
//!    email address or a raw URL query is still refused, because that
//!    is how such data actually arrives.
//! 4. **`validated: true` is the gate.** An idea that was not validated
//!    is not a graduation and seeds no project.

use std::io::Read;

use serde_json::Value;

use super::{
    is_raw_url_with_query, looks_like_email, looks_like_secret, normalize_timestamp, refusal,
    GraduationBrief, GraduationEvidence, GraduationExperiment, GraduationImport, GraduationRefusal,
    GraduationSource, GraduationSuccessMetric, ARTIFACT_KEYS, BRIEF_KEYS, CREDENTIAL_KEYS,
    EVIDENCE_KEYS, EXPERIMENT_KEYS, IDEA_GRADUATION_CONTRACT, IDENTITY_KEYS, MAX_BRIEF_FIELD_CHARS,
    MAX_EVIDENCE_EXCERPT_CHARS, MAX_EVIDENCE_ITEMS, MAX_EVIDENCE_KIND_CHARS, MAX_GRADUATION_BYTES,
    MAX_METRIC_FIELD_CHARS, MAX_METRIC_WINDOW_CHARS, MAX_PROVENANCE_CHARS, MAX_REQUIREMENTS,
    MAX_REQUIREMENT_CHARS, MAX_SUCCESS_METRICS, MAX_TITLE_CHARS, METRIC_KEYS, PAYMENT_KEYS,
    RAW_EVENT_KEYS, SUPPORTED_IDEA_GRADUATION_MAJOR, SUPPORTED_IDEA_GRADUATION_REVISIONS,
};
use crate::core::ForgeError;

/// A decoded artifact before it is validated: a JSON object whose keys
/// and values are still untrusted.
pub type GraduationRecord = serde_json::Map<String, Value>;

/// Read at most [`MAX_GRADUATION_BYTES`] from a path or stdin.
///
/// An absent or unreadable path is [`ForgeError::PathUnavailable`]; a
/// source over the byte bound or not valid UTF-8 is
/// [`ForgeError::GraduationInvalid`]. The distinction matters: the
/// operator can fix a path, but a 2 MiB artifact is a producer bug.
pub fn read_artifact(source: &str) -> Result<String, ForgeError> {
    let trimmed = source.trim();
    if trimmed == "-" {
        // One byte past the bound so an oversized stream is detected
        // rather than silently truncated into a parse error that would
        // read like malformed JSON.
        let mut reader = std::io::stdin()
            .lock()
            .take(MAX_GRADUATION_BYTES as u64 + 1);
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(|err| ForgeError::GraduationInvalid {
                reason: format!("graduation artifact could not be read: {err}"),
            })?;
        if bytes.len() > MAX_GRADUATION_BYTES {
            return Err(ForgeError::GraduationInvalid {
                reason: format!("graduation artifact is larger than {MAX_GRADUATION_BYTES} bytes"),
            });
        }
        return String::from_utf8(bytes).map_err(|_| ForgeError::GraduationInvalid {
            reason: "graduation artifact is not valid UTF-8".to_string(),
        });
    }
    let metadata = std::fs::metadata(trimmed).map_err(|_| ForgeError::PathUnavailable {
        path: trimmed.to_string(),
    })?;
    if metadata.len() > MAX_GRADUATION_BYTES as u64 {
        return Err(ForgeError::GraduationInvalid {
            reason: format!("graduation artifact is larger than {MAX_GRADUATION_BYTES} bytes"),
        });
    }
    let bytes = std::fs::read(trimmed).map_err(|_| ForgeError::PathUnavailable {
        path: trimmed.to_string(),
    })?;
    String::from_utf8(bytes).map_err(|_| ForgeError::GraduationInvalid {
        reason: "graduation artifact is not valid UTF-8".to_string(),
    })
}

/// Decode the artifact into an unvalidated closed-map record.
///
/// Only the JSON shape and the contract discriminator are read here: an
/// unsupported family, major or revision is refused before a single
/// content value is examined.
pub fn parse_artifact(raw: &str) -> Result<GraduationRecord, GraduationRefusal> {
    let value: Value = serde_json::from_str(raw).map_err(|err| {
        GraduationRefusal::new(
            "artifact",
            refusal::SHAPE,
            format!("artifact is not valid JSON: {err}"),
        )
    })?;
    let Value::Object(record) = value else {
        return Err(GraduationRefusal::new(
            "artifact",
            refusal::SHAPE,
            "artifact must be a JSON object",
        ));
    };
    contract_parts(&record)?;
    Ok(record)
}

/// Parse, family-check, major-check and revision-check the contract
/// discriminator. Returns the full discriminator, the parsed major and
/// the accepted revision.
fn contract_parts(record: &GraduationRecord) -> Result<(String, u32, String), GraduationRefusal> {
    let contract = match record.get("contract") {
        Some(Value::String(contract)) => contract.clone(),
        Some(_) => {
            return Err(GraduationRefusal::new(
                "contract",
                refusal::CONTRACT,
                "`contract` must be a string",
            ))
        }
        None => {
            return Err(GraduationRefusal::new(
                "contract",
                refusal::CONTRACT,
                format!(
                    "artifact must declare `contract: {IDEA_GRADUATION_CONTRACT}/<major>.<minor>.<patch>`"
                ),
            ))
        }
    };
    let Some((family, version)) = contract.split_once('/') else {
        return Err(GraduationRefusal::new(
            "contract",
            refusal::CONTRACT,
            "contract must be `platform.<family>/<major>.<minor>.<patch>`",
        ));
    };
    if family != IDEA_GRADUATION_CONTRACT {
        return Err(GraduationRefusal::new(
            "contract",
            refusal::CONTRACT,
            format!("contract family `{family}` is not `{IDEA_GRADUATION_CONTRACT}`"),
        ));
    }
    let parts: Vec<&str> = version.split('.').collect();
    let well_formed = parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
    if !well_formed {
        return Err(GraduationRefusal::new(
            "contract",
            refusal::CONTRACT,
            format!("contract revision `{version}` is not `<major>.<minor>.<patch>`"),
        ));
    }
    let revision = version.to_string();
    let major: u32 = parts[0].parse().map_err(|_| {
        GraduationRefusal::new(
            "contract",
            refusal::CONTRACT,
            "contract major is not a number",
        )
    })?;
    if major != SUPPORTED_IDEA_GRADUATION_MAJOR {
        return Err(GraduationRefusal::new(
            "contract",
            refusal::MAJOR,
            format!(
                "contract major {major} is unsupported; Forge accepts major {SUPPORTED_IDEA_GRADUATION_MAJOR}"
            ),
        ));
    }
    if !SUPPORTED_IDEA_GRADUATION_REVISIONS.contains(&revision.as_str()) {
        return Err(GraduationRefusal::new(
            "contract",
            refusal::REVISION,
            format!(
                "contract revision `{revision}` is unsupported; Forge accepts {}",
                SUPPORTED_IDEA_GRADUATION_REVISIONS.join(", ")
            ),
        ));
    }
    Ok((contract, major, revision))
}

/// The one gate: deny lists, closed key sets, provenance, brief,
/// experiment, `validated == true`, bounds and scrubbing. Every
/// transport goes through it.
pub fn validate_graduation(
    record: &GraduationRecord,
) -> Result<GraduationImport, GraduationRefusal> {
    // 1. The closed key set at the artifact level.
    refuse_unknown_keys(record, &ARTIFACT_KEYS, "artifact")?;

    // 2. The contract and provenance.
    let (contract, major, schema_revision) = contract_parts(record)?;
    let hypora_project_id = required_text(
        "hypora_project_id",
        string_field(record, "hypora_project_id")?,
        MAX_PROVENANCE_CHARS,
        refusal::PROVENANCE,
    )?;
    let hypora_revision = required_text(
        "hypora_revision",
        string_field(record, "hypora_revision")?,
        MAX_PROVENANCE_CHARS,
        refusal::PROVENANCE,
    )?;
    let graduated_at = timestamp_field(record, "graduated_at")?;

    // 3. The brief.
    let brief = object_field(record, "brief", refusal::SHAPE)?;
    refuse_unknown_keys(brief, &BRIEF_KEYS, "brief")?;
    let title = required_text(
        "brief.title",
        string_field(brief, "title")?,
        MAX_TITLE_CHARS,
        refusal::VALUE,
    )?;
    let problem = required_text(
        "brief.problem",
        string_field(brief, "problem")?,
        MAX_BRIEF_FIELD_CHARS,
        refusal::VALUE,
    )?;
    let audience = required_text(
        "brief.audience",
        string_field(brief, "audience")?,
        MAX_BRIEF_FIELD_CHARS,
        refusal::VALUE,
    )?;
    let solution = required_text(
        "brief.solution",
        string_field(brief, "solution")?,
        MAX_BRIEF_FIELD_CHARS,
        refusal::VALUE,
    )?;
    let requirements = validate_requirements(brief)?;
    let success_metrics = validate_success_metrics(brief)?;

    // 4. The experiment, including the `validated == true` gate.
    let experiment_record = object_field(record, "experiment", refusal::SHAPE)?;
    refuse_unknown_keys(experiment_record, &EXPERIMENT_KEYS, "experiment")?;
    let validated = match experiment_record.get("validated") {
        Some(Value::Bool(true)) => true,
        Some(Value::Bool(false)) => {
            return Err(GraduationRefusal::new(
                "experiment.validated",
                refusal::NOT_VALIDATED,
                "artifact declares the experiment was not validated; Forge imports only a validated idea",
            ))
        }
        Some(_) => {
            return Err(GraduationRefusal::new(
                "experiment.validated",
                refusal::SHAPE,
                "`experiment.validated` must be a boolean",
            ))
        }
        None => {
            return Err(GraduationRefusal::new(
                "experiment.validated",
                refusal::NOT_VALIDATED,
                "artifact must declare `experiment.validated: true`",
            ))
        }
    };
    let summary = required_text(
        "experiment.summary",
        string_field(experiment_record, "summary")?,
        MAX_BRIEF_FIELD_CHARS,
        refusal::VALUE,
    )?;
    let evidence = validate_evidence(experiment_record)?;

    Ok(GraduationImport {
        source: GraduationSource {
            contract,
            major,
            schema_revision,
            hypora_project_id,
            hypora_revision,
            graduated_at,
        },
        brief: GraduationBrief {
            title,
            problem,
            audience,
            solution,
            requirements,
            success_metrics,
        },
        experiment: GraduationExperiment {
            summary,
            validated,
            evidence,
        },
    })
}

/// Refuse a value that carries a credential shape without echoing it.
fn secret_finding(field: &str) -> GraduationRefusal {
    GraduationRefusal::new(
        field,
        refusal::SECRET,
        format!(
            "{field} was refused because it carries a credential-shaped value; \
             the value itself was not recorded"
        ),
    )
}

/// Name the class of a refused key. The four deny lists come first so
/// the refusal says *why* a field is dangerous; a key on none of them
/// is simply outside the closed set.
fn classify_refused_key(key: &str) -> &'static str {
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

/// Refuse any key outside the closed set for this object level.
fn refuse_unknown_keys(
    object: &GraduationRecord,
    allowed: &[&str],
    scope: &str,
) -> Result<(), GraduationRefusal> {
    for key in object.keys() {
        if allowed.contains(&key.as_str()) {
            continue;
        }
        if looks_like_secret(key) {
            return Err(secret_finding(&format!("{scope} field `{key}`")));
        }
        return Err(GraduationRefusal::new(
            format!("{scope} field `{key}`"),
            classify_refused_key(key),
            format!(
                "{scope} field `{key}` is refused; {scope} may carry only {}",
                allowed.join(", ")
            ),
        ));
    }
    Ok(())
}

/// A required, bounded, control-free, shape-checked text field.
fn required_text(
    field: &str,
    raw: Option<&str>,
    max_chars: usize,
    code: &str,
) -> Result<String, GraduationRefusal> {
    let trimmed = raw.unwrap_or_default().trim();
    if trimmed.is_empty() {
        return Err(GraduationRefusal::new(
            field,
            code,
            format!("{field} is required"),
        ));
    }
    if looks_like_secret(trimmed) {
        return Err(secret_finding(field));
    }
    if is_raw_url_with_query(trimmed) {
        return Err(GraduationRefusal::new(
            field,
            refusal::RAW_EVENT,
            format!(
                "{field} carries a raw URL query; a per-visitor URL is not an aggregate field \
                 and the value itself was not recorded"
            ),
        ));
    }
    if looks_like_email(trimmed) {
        return Err(GraduationRefusal::new(
            field,
            refusal::IDENTITY,
            format!(
                "{field} carries an email address; a graduation brief holds no participant \
                 identity and the value itself was not recorded"
            ),
        ));
    }
    if trimmed.chars().count() > max_chars {
        return Err(GraduationRefusal::new(
            field,
            refusal::BOUNDS,
            format!("{field} is longer than {max_chars} characters"),
        ));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(GraduationRefusal::new(
            field,
            refusal::VALUE,
            format!("{field} must not contain control characters"),
        ));
    }
    Ok(trimmed.to_string())
}

fn timestamp_field(object: &GraduationRecord, field: &str) -> Result<String, GraduationRefusal> {
    let raw = string_field(object, field)?.unwrap_or_default();
    normalize_timestamp(field, raw)
        .map_err(|reason| GraduationRefusal::new(field, refusal::VALUE, reason))
}

fn validate_requirements(brief: &GraduationRecord) -> Result<Vec<String>, GraduationRefusal> {
    let Some(Value::Array(entries)) = brief.get("requirements") else {
        return Err(GraduationRefusal::new(
            "brief.requirements",
            refusal::SHAPE,
            "brief requires a `requirements` array",
        ));
    };
    if entries.len() > MAX_REQUIREMENTS {
        return Err(GraduationRefusal::new(
            "brief.requirements",
            refusal::BOUNDS,
            format!("brief.requirements carries more than {MAX_REQUIREMENTS} entries"),
        ));
    }
    let mut out = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let Value::String(raw) = entry else {
            return Err(GraduationRefusal::new(
                format!("brief.requirements[{index}]"),
                refusal::SHAPE,
                "every requirement must be a string",
            ));
        };
        out.push(required_text(
            &format!("brief.requirements[{index}]"),
            Some(raw),
            MAX_REQUIREMENT_CHARS,
            refusal::VALUE,
        )?);
    }
    Ok(out)
}

fn validate_success_metrics(
    brief: &GraduationRecord,
) -> Result<Vec<GraduationSuccessMetric>, GraduationRefusal> {
    let Some(Value::Array(entries)) = brief.get("success_metrics") else {
        return Err(GraduationRefusal::new(
            "brief.success_metrics",
            refusal::SHAPE,
            "brief requires a `success_metrics` array",
        ));
    };
    if entries.len() > MAX_SUCCESS_METRICS {
        return Err(GraduationRefusal::new(
            "brief.success_metrics",
            refusal::BOUNDS,
            format!("brief.success_metrics carries more than {MAX_SUCCESS_METRICS} entries"),
        ));
    }
    let mut out = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let Value::Object(item) = entry else {
            return Err(GraduationRefusal::new(
                format!("brief.success_metrics[{index}]"),
                refusal::SHAPE,
                "every success metric must be a JSON object",
            ));
        };
        refuse_unknown_keys(item, &METRIC_KEYS, "success metric")?;
        let name = required_text(
            &format!("brief.success_metrics[{index}].name"),
            string_field(item, "name")?,
            MAX_METRIC_FIELD_CHARS,
            refusal::VALUE,
        )?;
        let target = required_text(
            &format!("brief.success_metrics[{index}].target"),
            string_field(item, "target")?,
            MAX_METRIC_FIELD_CHARS,
            refusal::VALUE,
        )?;
        let window = required_text(
            &format!("brief.success_metrics[{index}].window"),
            string_field(item, "window")?,
            MAX_METRIC_WINDOW_CHARS,
            refusal::VALUE,
        )?;
        out.push(GraduationSuccessMetric {
            name,
            target,
            window,
        });
    }
    Ok(out)
}

fn validate_evidence(
    experiment: &GraduationRecord,
) -> Result<Vec<GraduationEvidence>, GraduationRefusal> {
    let Some(Value::Array(entries)) = experiment.get("evidence") else {
        return Err(GraduationRefusal::new(
            "experiment.evidence",
            refusal::SHAPE,
            "experiment requires an `evidence` array",
        ));
    };
    if entries.len() > MAX_EVIDENCE_ITEMS {
        return Err(GraduationRefusal::new(
            "experiment.evidence",
            refusal::BOUNDS,
            format!("experiment.evidence carries more than {MAX_EVIDENCE_ITEMS} entries"),
        ));
    }
    let mut out = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let Value::Object(item) = entry else {
            return Err(GraduationRefusal::new(
                format!("experiment.evidence[{index}]"),
                refusal::SHAPE,
                "every evidence entry must be a JSON object",
            ));
        };
        refuse_unknown_keys(item, &EVIDENCE_KEYS, "evidence")?;
        let kind = required_text(
            &format!("experiment.evidence[{index}].kind"),
            string_field(item, "kind")?,
            MAX_EVIDENCE_KIND_CHARS,
            refusal::VALUE,
        )?;
        let excerpt = required_text(
            &format!("experiment.evidence[{index}].excerpt"),
            string_field(item, "excerpt")?,
            MAX_EVIDENCE_EXCERPT_CHARS,
            refusal::VALUE,
        )?;
        let observed_at = timestamp_field(item, "observed_at").map_err(|refusal| {
            GraduationRefusal::new(
                format!("experiment.evidence[{index}].observed_at"),
                refusal.code,
                refusal.detail,
            )
        })?;
        out.push(GraduationEvidence {
            kind,
            excerpt,
            observed_at,
        });
    }
    Ok(out)
}

/// Read an optional string field, refusing a non-string value rather
/// than coercing it.
fn string_field<'a>(
    object: &'a GraduationRecord,
    field: &str,
) -> Result<Option<&'a str>, GraduationRefusal> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(raw)) => Ok(Some(raw.as_str())),
        Some(_) => Err(GraduationRefusal::new(
            field,
            refusal::SHAPE,
            format!("`{field}` must be a string"),
        )),
    }
}

fn object_field<'a>(
    object: &'a GraduationRecord,
    field: &str,
    code: &str,
) -> Result<&'a GraduationRecord, GraduationRefusal> {
    match object.get(field) {
        Some(Value::Object(inner)) => Ok(inner),
        Some(_) => Err(GraduationRefusal::new(
            field,
            code,
            format!("`{field}` must be a JSON object"),
        )),
        None => Err(GraduationRefusal::new(
            field,
            code,
            format!("artifact requires `{field}`"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn artifact() -> GraduationRecord {
        json!({
            "contract": "platform.idea-graduation/0.1.0",
            "hypora_project_id": "prj_01H",
            "hypora_revision": "rev-2026-09-20-3",
            "graduated_at": "2026-09-20T00:00:00Z",
            "brief": {
                "title": "GPA Simulator",
                "problem": "Students cannot see how a term changes their GPA.",
                "audience": "University students planning a term.",
                "solution": "A planner that projects cumulative GPA per course set.",
                "requirements": ["Model terms, courses, credits and grades."],
                "success_metrics": [
                    { "name": "graded_course_sets_saved", "target": ">= 100", "window": "30d" }
                ],
            },
            "experiment": {
                "summary": "40 students completed the projection task in the probe.",
                "validated": true,
                "evidence": [
                    { "kind": "probe-completion", "excerpt": "Aggregate: 40 of 52 completed.", "observed_at": "2026-09-18T00:00:00Z" }
                ],
            },
        })
        .as_object()
        .expect("object")
        .clone()
    }

    #[test]
    fn a_valid_artifact_maps_to_source_brief_and_experiment() {
        let import = validate_graduation(&artifact()).expect("valid artifact");
        assert_eq!(import.source.contract, "platform.idea-graduation/0.1.0");
        assert_eq!(import.source.major, SUPPORTED_IDEA_GRADUATION_MAJOR);
        assert_eq!(import.source.schema_revision, "0.1.0");
        assert_eq!(import.source.hypora_project_id, "prj_01H");
        assert_eq!(import.brief.title, "GPA Simulator");
        assert_eq!(import.brief.requirements.len(), 1);
        assert_eq!(import.brief.success_metrics[0].window, "30d");
        assert!(import.experiment.validated);
        assert_eq!(import.experiment.evidence.len(), 1);
        assert_eq!(
            import.experiment.evidence[0].observed_at,
            "2026-09-18T00:00:00Z"
        );
    }

    #[test]
    fn an_unsupported_major_and_an_unknown_revision_are_named() {
        let mut major = artifact();
        major.insert("contract".into(), json!("platform.idea-graduation/1.0.0"));
        let err = validate_graduation(&major).expect_err("major");
        assert_eq!(err.code, refusal::MAJOR);
        assert!(err.detail.contains("major 0"), "{err}");

        let mut revision = artifact();
        revision.insert("contract".into(), json!("platform.idea-graduation/0.2.0"));
        let err = validate_graduation(&revision).expect_err("revision");
        assert_eq!(err.code, refusal::REVISION);
        assert!(err.detail.contains("0.1.0"), "{err}");
    }

    #[test]
    fn a_malformed_contract_is_refused() {
        for contract in [
            "forge-graduation-import/0.1.0",
            "platform.idea-graduation",
            "platform.idea-graduation/1",
            "platform.idea-graduation/0.1",
            "platform.idea-graduation/0.1.x",
        ] {
            let mut raw = artifact();
            raw.insert("contract".into(), json!(contract));
            let err = validate_graduation(&raw).expect_err(contract);
            assert_eq!(err.code, refusal::CONTRACT, "{contract}: {err}");
        }
        let mut missing = artifact();
        missing.remove("contract");
        assert_eq!(
            validate_graduation(&missing).expect_err("absent").code,
            refusal::CONTRACT
        );
        assert_eq!(
            parse_artifact("[]").expect_err("array").code,
            refusal::SHAPE
        );
    }

    #[test]
    fn the_deny_lists_classify_each_dangerous_key_class() {
        for (field, expected) in [
            ("email", refusal::IDENTITY),
            ("participant_id", refusal::IDENTITY),
            ("ip_address", refusal::IDENTITY),
            ("raw_events", refusal::RAW_EVENT),
            ("answers", refusal::RAW_EVENT),
            ("card_number", refusal::PAYMENT),
            ("revenue", refusal::PAYMENT),
            ("api_key", refusal::CREDENTIAL),
            ("session_token", refusal::CREDENTIAL),
            ("profile", refusal::FIELD_UNKNOWN),
            ("deployment", refusal::FIELD_UNKNOWN),
        ] {
            let mut raw = artifact();
            raw.insert(field.to_string(), json!("anything"));
            let err = validate_graduation(&raw).expect_err(field);
            assert_eq!(err.code, expected, "{field}: {}", err.detail);
            assert!(!err.detail.contains("anything"), "{field}: {}", err.detail);
        }
    }

    #[test]
    fn the_closed_key_sets_apply_at_every_level() {
        // Brief level.
        let mut brief_key = artifact();
        brief_key["brief"]
            .as_object_mut()
            .unwrap()
            .insert("email".into(), json!("a@b.co"));
        assert_eq!(
            validate_graduation(&brief_key).expect_err("brief key").code,
            refusal::IDENTITY
        );
        // Metric level.
        let mut metric_key = artifact();
        metric_key["brief"]["success_metrics"][0]["visitor_id"] = json!("v1");
        assert_eq!(
            validate_graduation(&metric_key)
                .expect_err("metric key")
                .code,
            refusal::IDENTITY
        );
        // Evidence level.
        let mut evidence_key = artifact();
        evidence_key["experiment"]["evidence"][0]["page_url"] = json!("https://x.example/?utm=a");
        assert_eq!(
            validate_graduation(&evidence_key)
                .expect_err("evidence key")
                .code,
            refusal::RAW_EVENT
        );
    }

    #[test]
    fn a_credential_shaped_value_is_refused_and_never_echoed() {
        let mut raw = artifact();
        raw["brief"]["problem"] = json!("token=ghp_abcdefghijklmnopqrstuvwxyz0123456789");
        let err = validate_graduation(&raw).expect_err("credential");
        assert_eq!(err.code, refusal::SECRET);
        assert!(!err.detail.contains("ghp_"), "{}", err.detail);
        assert!(err.detail.contains("credential-shaped"), "{}", err.detail);
    }

    #[test]
    fn an_email_or_query_url_value_is_refused() {
        let mut email = artifact();
        email["brief"]["audience"] = json!("ops@example.com");
        let err = validate_graduation(&email).expect_err("email");
        assert_eq!(err.code, refusal::IDENTITY);
        assert!(!err.detail.contains("ops@example.com"), "{err}");

        let mut url = artifact();
        url["brief"]["solution"] = json!("https://example.com/?utm_source=x");
        let err = validate_graduation(&url).expect_err("query url");
        assert_eq!(err.code, refusal::RAW_EVENT);
        assert!(!err.detail.contains("utm_source"), "{err}");
    }

    #[test]
    fn an_unvalidated_artifact_is_refused() {
        let mut absent = artifact();
        absent["experiment"]
            .as_object_mut()
            .unwrap()
            .remove("validated");
        let err = validate_graduation(&absent).expect_err("absent");
        assert_eq!(err.code, refusal::NOT_VALIDATED);

        let mut falsey = artifact();
        falsey["experiment"]["validated"] = json!(false);
        let err = validate_graduation(&falsey).expect_err("false");
        assert_eq!(err.code, refusal::NOT_VALIDATED);
        assert!(err.detail.contains("not validated"), "{err}");
    }

    #[test]
    fn the_bounds_are_enforced_per_field() {
        let mut title = artifact();
        title["brief"]["title"] = json!("x".repeat(MAX_TITLE_CHARS + 1));
        let err = validate_graduation(&title).expect_err("title");
        assert_eq!(err.code, refusal::BOUNDS);
        assert!(err.detail.contains(&MAX_TITLE_CHARS.to_string()), "{err}");

        let mut project = artifact();
        project["hypora_project_id"] = json!("x".repeat(MAX_PROVENANCE_CHARS + 1));
        assert_eq!(
            validate_graduation(&project).expect_err("provenance").code,
            refusal::BOUNDS
        );

        let mut requirements = artifact();
        requirements["brief"]["requirements"] = json!(vec!["x".to_string(); MAX_REQUIREMENTS + 1]);
        assert_eq!(
            validate_graduation(&requirements)
                .expect_err("requirements")
                .code,
            refusal::BOUNDS
        );

        let mut evidence = artifact();
        evidence["experiment"]["evidence"] = json!(vec![
            json!({"kind": "k", "excerpt": "e", "observed_at": "2026-09-18T00:00:00Z"});
            MAX_EVIDENCE_ITEMS + 1
        ]);
        assert_eq!(
            validate_graduation(&evidence).expect_err("evidence").code,
            refusal::BOUNDS
        );
    }

    #[test]
    fn a_control_character_in_a_field_is_refused() {
        let mut raw = artifact();
        raw["brief"]["problem"] = json!("before\u{0001}after");
        let err = validate_graduation(&raw).expect_err("control");
        assert_eq!(err.code, refusal::VALUE);
    }

    #[test]
    fn a_missing_or_wrongly_typed_field_is_refused() {
        let mut missing = artifact();
        missing["brief"].as_object_mut().unwrap().remove("problem");
        assert_eq!(
            validate_graduation(&missing).expect_err("missing").code,
            refusal::VALUE
        );

        let mut wrong = artifact();
        wrong["brief"]["title"] = json!(7);
        assert_eq!(
            validate_graduation(&wrong).expect_err("type").code,
            refusal::SHAPE
        );

        let mut bad_time = artifact();
        bad_time["graduated_at"] = json!("2026-09-20");
        assert_eq!(
            validate_graduation(&bad_time).expect_err("naive time").code,
            refusal::VALUE
        );
    }

    #[test]
    fn read_artifact_distinguishes_path_from_payload() {
        let err = read_artifact("/definitely/not/here/artifact.json").expect_err("absent");
        assert_eq!(err.code(), "path-unavailable");
    }
}
