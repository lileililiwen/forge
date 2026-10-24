use super::*;

/// Registry-only metrics; the browser never probes external providers or
/// writes a summary or journal row.
pub(super) fn metrics(db_path: &Path, id: &str, query: Option<&str>) -> ApiResponse {
    let window_days = match super::query_single(query, "window_days") {
        None => crate::analytics::DEFAULT_WINDOW_DAYS,
        Some(raw) => match raw.trim().parse::<u32>() {
            Ok(window) => window,
            Err(_) => {
                return error(
                    400,
                    "admin-invalid-assurance-query",
                    "the analytics `window_days` query value must be a number.",
                )
            }
        },
    };
    if !(crate::analytics::MIN_WINDOW_DAYS..=crate::analytics::MAX_WINDOW_DAYS)
        .contains(&window_days)
    {
        return error(
            400,
            "admin-invalid-assurance-query",
            "the analytics `window_days` query value is outside the allowed range.",
        );
    }
    let registry = match crate::registry::Registry::open(db_path) {
        Ok(registry) => registry,
        Err(err) => return super::typed_assurance_error(&err),
    };
    match crate::analytics::aggregate_project_metrics(
        &registry,
        None,
        &crate::analytics::MetricsAggregateOptions {
            default_window_days: window_days,
            external_observations: Vec::new(),
        },
    ) {
        Ok(report) => ApiResponse::json(
            200,
            json!({
                "project": id, "metrics": report,
                "contract": crate::analytics::ANALYTICS_CONTRACT_VERSION,
            }),
        ),
        Err(err) => super::typed_assurance_error(&err),
    }
}

/// Read the current session envelope without starting, stopping, or spawning.
pub(super) fn studio_preview(project_dir: &Path, id: &str) -> ApiResponse {
    let secrets = [project_dir.display().to_string()];
    match crate::studio::load_session(project_dir) {
        Ok(Some(session)) => {
            let envelope = crate::studio::envelope_from_session(&session);
            let envelope = scrub_json(json!(envelope), &secrets);
            ApiResponse::json(
                200,
                json!({ "project": id, "preview": envelope, "contract": API_CONTRACT_VERSION }),
            )
        }
        Ok(None) => {
            let envelope = crate::studio::PreviewEnvelope::from_session(
                id,
                "r0",
                &crate::studio::state::SessionPreviewState::default(),
            );
            ApiResponse::json(
                200,
                json!({ "project": id, "preview": envelope, "contract": API_CONTRACT_VERSION }),
            )
        }
        Err(err) => super::typed_project_assurance_error(project_dir, &err),
    }
}
