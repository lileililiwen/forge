//! Authenticated list and Gate workflow for Forge source modules.
use super::super::{ApiConfig, ApiRequest, ApiResponse, Route};
use super::gateway::{cors, error, guarded, is_json, scrub_json, scrub_text};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

const CONTRACT: &str = "forge-module-workflow/0.1.0";
static RUNNING: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

struct ModuleRunGuard(String);

impl ModuleRunGuard {
    fn acquire(id: &str) -> Result<Self, ()> {
        let running = RUNNING.get_or_init(|| Mutex::new(HashSet::new()));
        let mut active = running.lock().map_err(|_| ())?;
        if !active.insert(id.to_string()) {
            return Err(());
        }
        Ok(Self(id.to_string()))
    }
}

impl Drop for ModuleRunGuard {
    fn drop(&mut self) {
        if let Some(running) = RUNNING.get() {
            if let Ok(mut active) = running.lock() {
                active.remove(&self.0);
            }
        }
    }
}

pub(in crate::api) fn route_modules(method: &str, segments: &[&str]) -> Option<Route> {
    match (method, segments) {
        ("GET", ["v1", "admin", "modules"]) => Some(Route::AdminModules),
        ("GET", ["v1", "admin", "modules", id, "gate", "status"]) => {
            Some(Route::AdminModuleGateStatus {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "modules", id, "gate", "run"]) => {
            Some(Route::AdminModuleGateRun {
                id: (*id).to_string(),
            })
        }
        _ => None,
    }
}

pub(in crate::api) fn handles(route: &Route) -> bool {
    matches!(
        route,
        Route::AdminModules
            | Route::AdminModuleGateStatus { .. }
            | Route::AdminModuleGateRun { .. }
    )
}

pub(in crate::api) fn handle(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    route: &Route,
) -> ApiResponse {
    if !super::gateway::allowed_origin(config, request.header("origin")) {
        return cors(
            config,
            request,
            error(
                403,
                "admin-origin-rejected",
                "request origin is not allowed",
            ),
        );
    }
    if request.method == "POST" && !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "module Gate run requires application/json",
            ),
        );
    }
    let response = guarded(db_path, request, |_| match route {
        Route::AdminModules => list_modules(),
        Route::AdminModuleGateStatus { id } => with_module(id, status_module),
        Route::AdminModuleGateRun { id } => run_module(db_path, request, id),
        _ => error(404, "module-route-not-found", "module route was not found"),
    });
    cors(config, request, response)
}

fn with_module(
    id: &str,
    f: impl FnOnce(&crate::module_catalog::ModuleEntry) -> ApiResponse,
) -> ApiResponse {
    match crate::module_catalog::resolve(id) {
        Ok(module) => f(&module),
        Err(err) => typed_error(err),
    }
}

fn typed_error(err: crate::core::ForgeError) -> ApiResponse {
    let code = err.code();
    let status = match code {
        "module-not-found" => 404,
        "module-catalog-unavailable" | "module-catalog-invalid" => 503,
        _ => 400,
    };
    ApiResponse::json(
        status,
        json!({"contract": CONTRACT, "error":{"code":code,"message":"module catalog could not resolve the requested module"}}),
    )
}

fn list_modules() -> ApiResponse {
    match crate::module_catalog::discover() {
        Ok(modules) => {
            let rows: Vec<Value> = modules.iter().map(|m| json!({"id":m.id,"stack":m.stack,"profile":m.profile,"path":m.path,"gate":latest(&m.absolute_path,m.id.as_str())})).collect();
            ApiResponse::json(200, json!({"contract":CONTRACT,"modules":rows}))
        }
        Err(err) => typed_error(err),
    }
}

fn latest(root: &Path, id: &str) -> Value {
    match crate::gate::load_latest_evidence(root) {
        Ok(Some(evidence)) => {
            let revision = crate::gate::capture_revision(root);
            let freshness = crate::gate::evidence_freshness(&evidence, revision.as_deref());
            let mut envelope = serde_json::to_value(evidence).unwrap_or(Value::Null);
            envelope["freshness"] = json!(freshness.label());
            scrub_json(envelope, &[root.display().to_string()])
        }
        Ok(None) => {
            json!({"project_id":id,"aggregate":"unverified","freshness":"absent","checks":[]})
        }
        Err(_) => {
            json!({"project_id":id,"aggregate":"unavailable","freshness":"unavailable","checks":[]})
        }
    }
}

fn status_module(module: &crate::module_catalog::ModuleEntry) -> ApiResponse {
    ApiResponse::json(
        200,
        json!({"contract":CONTRACT,"module":{"id":module.id,"path":module.path},"gate":latest(&module.absolute_path,&module.id)}),
    )
}

fn run_module(db_path: &Path, request: &ApiRequest, id: &str) -> ApiResponse {
    let body = request.json_body();
    let Some(fields) = body.as_object() else {
        return profile_error("body must be a JSON object");
    };
    if fields.keys().any(|key| key != "profile") {
        return profile_error("only the profile field is accepted");
    }
    let profile = match body
        .get("profile")
        .and_then(Value::as_str)
        .unwrap_or("full")
    {
        "fast" => crate::gate::GateProfile::Fast,
        "full" => crate::gate::GateProfile::Full,
        _ => return profile_error("profile must be fast or full"),
    };
    let module = match crate::module_catalog::resolve(id) {
        Ok(m) => m,
        Err(err) => return typed_error(err),
    };
    let _run = match ModuleRunGuard::acquire(id) {
        Ok(guard) => guard,
        Err(()) => {
            return error(
                409,
                "module-gate-in-progress",
                "a Gate run is already active for this module",
            )
        }
    };
    let mut cfg = crate::gate::GateConfig::from_env();
    cfg.profile = profile;
    match crate::gate::run_gate(&module.absolute_path, id, &cfg, false) {
        crate::gate::GateOutcome::Evidence(evidence) => {
            if let Err(err) = crate::gate::save_evidence(&module.absolute_path, &evidence) {
                return ApiResponse::json(
                    503,
                    json!({"contract":CONTRACT,"error":{"code":err.code(),"message":"Gate evidence could not be saved"}}),
                );
            }
            if let Ok(registry) = crate::registry::Registry::open(db_path) {
                let outcome = crate::gate::GateOutcome::Evidence(evidence.clone());
                let verdict = crate::gate::journal_verdict(&outcome);
                let summary = crate::gate::evidence_summary(&evidence);
                let _ = registry.record_operation("gate", id, verdict, &summary);
            }
            let gate = latest(&module.absolute_path, id);
            let payload = scrub_json(
                json!({"contract":CONTRACT,"module":{"id":id,"path":module.path},"gate":gate}),
                &[module.absolute_path.display().to_string()],
            );
            ApiResponse::json(200, payload)
        }
        crate::gate::GateOutcome::Unavailable { reason } => ApiResponse::json(
            503,
            json!({"contract":CONTRACT,"error":{"code":"module-gate-unavailable","message":scrub_text(&reason,&[module.absolute_path.display().to_string()])}}),
        ),
        _ => error(
            503,
            "module-gate-unavailable",
            "Gate did not produce persisted evidence",
        ),
    }
}

fn profile_error(message: &str) -> ApiResponse {
    ApiResponse::json(
        400,
        json!({"contract":CONTRACT,"error":{"code":"gate-profile-invalid","message":message}}),
    )
}

#[cfg(test)]
mod tests {
    use super::ModuleRunGuard;

    #[test]
    fn module_run_guard_rejects_duplicates_and_releases_on_drop() {
        let first = ModuleRunGuard::acquire("guard-contract-module").unwrap();
        assert!(ModuleRunGuard::acquire("guard-contract-module").is_err());
        assert!(ModuleRunGuard::acquire("independent-contract-module").is_ok());
        drop(first);
        assert!(ModuleRunGuard::acquire("guard-contract-module").is_ok());
    }
}
