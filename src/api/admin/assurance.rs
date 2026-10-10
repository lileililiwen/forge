//! Read-only assurance browser (`web-assurance-browser`).
//!
//! Seventeen session-gated GET handlers over the assurance reads the CLI
//! already serves: `spec::{list_specs, read_spec, route_finding}`,
//! `remediation::{scan, build_plan, diff}`,
//! `semantic::{list, read}` (describe + classify),
//! `contract::{load_manifest, family_schema_path, emit_*}`,
//! `governance::{list_providers, evaluate_project}` (never
//! `check_project`), registry-only
//! `analytics::aggregate_project_metrics` (never
//! `inspect_external_planes`), and `studio::{load_session,
//! envelope_from_session}` (never start/stop/spawn). No write, no
//! provider probe, no adapter run, no shell, no journal row, no summary
//! write, no observation persist, no browser-supplied path on any path;
//! every response carries the admin contract version and project-bound
//! bodies are scrubbed of the absolute project directory.

use super::super::{ApiConfig, ApiRequest, ApiResponse, Route, API_CONTRACT_VERSION};
use serde_json::{json, Value};
use std::path::Path;

use super::deploy::deploy_id_gate;
use super::gateway::{cors, error, guarded, scrub_json, scrub_text};

/// Assurance sections served beside the creation catalog on the shared
/// validated-key triple. Disjoint from every creation registry key, so the
/// two matchers never claim the same shape.
const SECTIONS: &[&str] = &[
    "contracts",
    "specs",
    "spec",
    "remediate",
    "describe",
    "classify",
    "governance",
    "analytics",
    "studio",
];

/// True for the nine assurance sections; anything else belongs to the
/// creation catalog (or to no beside-table route at all).
pub(in crate::api) fn is_assurance_registry(registry: &str) -> bool {
    SECTIONS.contains(&registry)
}

fn list_route(section: &str) -> Route {
    Route::AdminCreation {
        registry: section.to_string(),
        item: String::new(),
        action: "list".to_string(),
    }
}

fn inspect_route(section: &str, item: &str, action: &str) -> Route {
    Route::AdminCreation {
        registry: section.to_string(),
        item: item.to_string(),
        action: action.to_string(),
    }
}

/// Beside-table entry point for the shared triple: assurance sections run
/// here, every other registry keeps the creation-catalog behavior, and an
/// unknown registry is a static 404. One call site keeps the deploy
/// dispatch arm (and ) at a constant line count.
pub(in crate::api) fn dispatch_creation(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    registry: &str,
    item: &str,
    action: &str,
) -> ApiResponse {
    if is_assurance_registry(registry) {
        dispatch(config, db_path, request, registry, item, action)
    } else {
        super::creation::dispatch(config, db_path, request, registry, item, action)
    }
}

/// Match the seventeen read-only assurance routes before the main table
/// runs. The literal `contracts` arm never collides with the
/// `projects`/`creation`/`portfolio`/`delivery`/`fleet`/`workspace` arms;
/// the project-bound literals (`specs`, `spec/route`, `remediate/scan`,
/// `remediate/diff`, `describe/proposals`, `classify/proposals`,
/// `contracts/emit`, `governance`, `governance/status`,
/// `governance/inspect`, `analytics/metrics`, `studio/preview`) never
/// collide with the existing `feature`/`spec`/`deploy`/`release`/
/// `publish`/`delivery`/`agents`/`identity`/`releases`/`deploys`/
/// `catalog`/`standard`/`intent` arms, so trying these shapes first
/// shadows no existing route. Kept beside the handlers (not in
/// `router.rs`) so the route table stays under the source-file-size cap.
/// The `OPTIONS` arm keeps the browser preflight beside the GETs it
/// covers.
pub(in crate::api) fn route_assurance(method: &str, segments: &[&str]) -> Option<Route> {
    match (method, segments) {
        ("GET", ["v1", "admin", "contracts"]) => Some(list_route("contracts")),
        ("GET", ["v1", "admin", "contracts", family]) => {
            Some(inspect_route("contracts", family, "inspect"))
        }
        ("GET", ["v1", "admin", "projects", id, "specs"]) => {
            Some(inspect_route("specs", id, "list"))
        }
        ("GET", ["v1", "admin", "projects", id, "specs", spec]) => {
            Some(inspect_route("specs", &format!("{id}/{spec}"), "inspect"))
        }
        ("GET", ["v1", "admin", "projects", id, "spec", "route"]) => {
            Some(inspect_route("spec", id, "route"))
        }
        ("GET", ["v1", "admin", "projects", id, "remediate", "scan"]) => {
            Some(inspect_route("remediate", id, "scan"))
        }
        ("GET", ["v1", "admin", "projects", id, "remediate", "diff"]) => {
            Some(inspect_route("remediate", id, "diff"))
        }
        ("GET", ["v1", "admin", "projects", id, "describe", "proposals"]) => {
            Some(inspect_route("describe", id, "list"))
        }
        ("GET", ["v1", "admin", "projects", id, "describe", "proposals", proposal]) => Some(
            inspect_route("describe", &format!("{id}/{proposal}"), "show"),
        ),
        ("GET", ["v1", "admin", "projects", id, "classify", "proposals"]) => {
            Some(inspect_route("classify", id, "list"))
        }
        ("GET", ["v1", "admin", "projects", id, "classify", "proposals", proposal]) => Some(
            inspect_route("classify", &format!("{id}/{proposal}"), "show"),
        ),
        ("GET", ["v1", "admin", "projects", id, "contracts", "emit"]) => {
            Some(inspect_route("contracts", id, "emit"))
        }
        ("GET", ["v1", "admin", "projects", id, "governance"]) => {
            Some(inspect_route("governance", id, "list"))
        }
        ("GET", ["v1", "admin", "projects", id, "governance", "status"]) => {
            Some(inspect_route("governance", id, "status"))
        }
        ("GET", ["v1", "admin", "projects", id, "governance", "inspect"]) => {
            Some(inspect_route("governance", id, "inspect"))
        }
        ("GET", ["v1", "admin", "projects", id, "analytics", "metrics"]) => {
            Some(inspect_route("analytics", id, "metrics"))
        }
        ("GET", ["v1", "admin", "projects", id, "studio", "preview"]) => {
            Some(inspect_route("studio", id, "preview"))
        }
        ("OPTIONS", ["v1", "admin", "contracts", ..]) => Some(Route::AdminOptions),
        ("OPTIONS", ["v1", "admin", "projects", ..]) => None,
        _ => None,
    }
}

/// Validate one `{id}`-family path segment before any store lookup. A
/// blank value, a path separator (raw or percent-encoded), a backslash,
/// a `..` traversal or any other percent-encoding is a static typed
/// `400` that never echoes the offending input.
fn assurance_id_gate(raw: &str) -> Result<String, ApiResponse> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.contains('%')
    {
        return Err(error(
            400,
            "admin-invalid-assurance-id",
            "that assurance id is not valid; use the id shown in the list.",
        ));
    }
    Ok(trimmed.to_string())
}

/// Split a `"{project}/{entry}"` item back into its validated halves.
/// Both halves pass the same gate; a malformed item is a static `404`
/// (the matcher built it, so this is unreachable in practice).
fn split_item(item: &str) -> Result<(String, String), ApiResponse> {
    match item.split_once('/') {
        Some((project, entry)) => Ok((assurance_id_gate(project)?, assurance_id_gate(entry)?)),
        None => Err(error(
            404,
            "admin-unknown-assurance",
            "no assurance route matches the request",
        )),
    }
}

fn query_values(query: Option<&str>, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    for pair in query
        .unwrap_or_default()
        .split('&')
        .filter(|p| !p.is_empty())
    {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k.trim() == key {
            let decoded = super::super::percent_decode(v);
            if !decoded.trim().is_empty() {
                out.push(decoded);
            }
        }
    }
    out
}

fn query_single(query: Option<&str>, key: &str) -> Option<String> {
    query_values(query, key).into_iter().next()
}

fn require_query(query: Option<&str>, key: &str, what: &str) -> Result<String, ApiResponse> {
    match query_single(query, key) {
        Some(value) => Ok(value),
        None => Err(error(400, "admin-invalid-assurance-query", what)),
    }
}

/// Assurance status mapping: the shared table plus the reachable
/// assurance user errors, which the shared table would report as `500`.
/// Kept beside the handlers (not in `err_status`) so `router.rs` stays
/// under the source-file-size cap.
pub(in crate::api) fn assurance_error_status(err: &crate::core::ForgeError) -> u16 {
    match err.code() {
        "spec-invalid"
        | "remediation-invalid"
        | "semantic-invalid"
        | "contract-invalid"
        | "governance-invalid"
        | "analytics-invalid"
        | "studio-invalid-spec" => 400,
        _ => super::super::err_status(err),
    }
}

fn typed_assurance_error(err: &crate::core::ForgeError) -> ApiResponse {
    ApiResponse::json(
        assurance_error_status(err),
        json!({
            "error": { "code": err.code(), "message": err.to_string() },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

fn typed_project_assurance_error(project_dir: &Path, err: &crate::core::ForgeError) -> ApiResponse {
    let secrets = [project_dir.display().to_string()];
    ApiResponse::json(
        assurance_error_status(err),
        json!({
            "error": {
                "code": err.code(),
                "message": scrub_text(&err.to_string(), &secrets),
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// Dispatch one assurance read. The `(registry, action)` pair is
/// re-validated here so only the matcher's seventeen combos run;
/// anything else is a static `404`, never a guess.
pub(in crate::api) fn dispatch(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    section: &str,
    item: &str,
    action: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let query = req.query.as_deref();
            match (section, action) {
                ("contracts", "list") => contract_list(),
                ("contracts", "inspect") => match assurance_id_gate(item) {
                    Ok(family) => contract_inspect(&family),
                    Err(response) => response,
                },
                ("specs", "list") => match deploy_id_gate(db_path, item) {
                    Ok(dir) => spec_list(&dir, item),
                    Err(response) => response,
                },
                ("specs", "inspect") => match split_item(item) {
                    Ok((project, spec)) => match deploy_id_gate(db_path, &project) {
                        Ok(dir) => spec_inspect(&dir, &project, &spec),
                        Err(response) => response,
                    },
                    Err(response) => response,
                },
                ("spec", "route") => match deploy_id_gate(db_path, item) {
                    Ok(dir) => match require_query(
                        query,
                        "finding",
                        "the spec `finding` query value is required, e.g. ?finding=<finding-id>.",
                    ) {
                        Ok(finding) => spec_route(&dir, item, &finding),
                        Err(response) => response,
                    },
                    Err(response) => response,
                },
                ("remediate", "scan") => match deploy_id_gate(db_path, item) {
                    Ok(dir) => remediate_scan(&dir, item),
                    Err(response) => response,
                },
                ("remediate", "diff") => match deploy_id_gate(db_path, item) {
                    Ok(dir) => {
                        let finding = match require_query(
                            query,
                            "finding",
                            "the remediate `finding` query value is required, e.g. ?finding=<finding-id>.",
                        ) {
                            Ok(v) => v,
                            Err(response) => return response,
                        };
                        let pack = query_single(query, "pack");
                        remediate_diff(&dir, item, &finding, pack.as_deref())
                    }
                    Err(response) => response,
                },
                ("describe", "list") => match deploy_id_gate(db_path, item) {
                    Ok(dir) => semantic_list(&dir, item, "describe"),
                    Err(response) => response,
                },
                ("describe", "show") => match split_item(item) {
                    Ok((project, proposal)) => match deploy_id_gate(db_path, &project) {
                        Ok(dir) => semantic_show(&dir, &project, &proposal),
                        Err(response) => response,
                    },
                    Err(response) => response,
                },
                ("classify", "list") => match deploy_id_gate(db_path, item) {
                    Ok(dir) => semantic_list(&dir, item, "classify"),
                    Err(response) => response,
                },
                ("classify", "show") => match split_item(item) {
                    Ok((project, proposal)) => match deploy_id_gate(db_path, &project) {
                        Ok(dir) => semantic_show(&dir, &project, &proposal),
                        Err(response) => response,
                    },
                    Err(response) => response,
                },
                ("contracts", "emit") => match deploy_id_gate(db_path, item) {
                    Ok(dir) => match require_query(
                        query,
                        "family",
                        "the contract `family` query value is required, e.g. ?family=platform.gate-result.",
                    ) {
                        Ok(family) => contract_emit(&dir, item, &family),
                        Err(response) => response,
                    },
                    Err(response) => response,
                },
                ("governance", "list") => match deploy_id_gate(db_path, item) {
                    Ok(dir) => governance_list(&dir, item),
                    Err(response) => response,
                },
                ("governance", "status") | ("governance", "inspect") => {
                    match deploy_id_gate(db_path, item) {
                        Ok(dir) => governance_observe(&dir, item),
                        Err(response) => response,
                    }
                }
                ("analytics", "metrics") => match deploy_id_gate(db_path, item) {
                    Ok(dir) => {
                        let _ = &dir;
                        analytics_metrics(db_path, item, query)
                    }
                    Err(response) => response,
                },
                ("studio", "preview") => match deploy_id_gate(db_path, item) {
                    Ok(dir) => studio_preview(&dir, item),
                    Err(response) => response,
                },
                _ => error(
                    404,
                    "admin-unknown-assurance",
                    "no assurance route matches the request",
                ),
            }
        }),
    )
}

/// `GET /v1/admin/contracts`: every versioned surface and its platform
/// mapping — the same payload as `forge contract list --format json`.
fn contract_list() -> ApiResponse {
    let manifest = match crate::contract::load_manifest() {
        Ok(manifest) => manifest,
        Err(err) => return typed_assurance_error(&err),
    };
    let contracts: Vec<Value> = crate::contract::CONTRACTS
        .iter()
        .map(|c| {
            json!({
                "module": c.module, "constant": c.constant,
                "discriminator": c.discriminator, "version": c.version,
                "platform_family": c.platform_family, "doc": c.doc,
            })
        })
        .collect();
    ApiResponse::json(
        200,
        json!({
            "contracts": contracts,
            "manifest": manifest,
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// `GET /v1/admin/contracts/{family}`: one platform family and its
/// schema — the same `required` projection as `forge contract inspect`.
fn contract_inspect(family: &str) -> ApiResponse {
    let schema_path = match crate::contract::family_schema_path(family) {
        Some(path) => path,
        None => {
            return error(
                400,
                "admin-unknown-contract-family",
                "unknown contract family; use the family shown in the contract list.",
            )
        }
    };
    let schema_text = match std::fs::read_to_string(
        crate::contract::contracts_dir().join(schema_path),
    ) {
        Ok(text) => text,
        Err(err) => {
            return ApiResponse::json(
                500,
                json!({
                    "error": { "code": "contract-unavailable", "message": format!("cannot read schema {schema_path}: {err}") },
                    "contract": API_CONTRACT_VERSION,
                }),
            )
        }
    };
    let schema: Value = match serde_json::from_str(&schema_text) {
        Ok(schema) => schema,
        Err(err) => {
            return ApiResponse::json(
                500,
                json!({
                    "error": { "code": "contract-unavailable", "message": format!("invalid schema: {err}") },
                    "contract": API_CONTRACT_VERSION,
                }),
            )
        }
    };
    let manifest = match crate::contract::load_manifest() {
        Ok(manifest) => manifest,
        Err(err) => return typed_assurance_error(&err),
    };
    ApiResponse::json(
        200,
        json!({ "family": family, "schema": schema, "manifest": manifest, "contract": API_CONTRACT_VERSION }),
    )
}

/// `GET /v1/admin/projects/{id}/specs`: every generated spec under
/// `.forge/specs/` — the same entries as `forge spec list`.
fn spec_list(project_dir: &Path, id: &str) -> ApiResponse {
    let secrets = [project_dir.display().to_string()];
    match crate::spec::list_specs(project_dir) {
        Ok(entries) => {
            let entries = scrub_json(json!(entries), &secrets);
            ApiResponse::json(
                200,
                json!({ "project": id, "specs": entries, "contract": API_CONTRACT_VERSION }),
            )
        }
        Err(err) => typed_project_assurance_error(project_dir, &err),
    }
}

/// `GET /v1/admin/projects/{id}/specs/{spec}`: the full bounded
/// proposal — the same draft as `forge spec inspect`. The `{spec}`
/// segment accepts the full `<project>-<hash>` dir name or an
/// unambiguous `<hash>` prefix, mirroring the CLI.
fn spec_inspect(project_dir: &Path, id: &str, raw: &str) -> ApiResponse {
    let secrets = [project_dir.display().to_string()];
    let spec_id = match resolve_spec_id(project_dir, id, raw) {
        Ok(spec_id) => spec_id,
        Err(response) => return response,
    };
    match crate::spec::read_spec(project_dir, &spec_id) {
        Ok(Some(draft)) => {
            let draft = scrub_json(json!(draft), &secrets);
            ApiResponse::json(
                200,
                json!({ "project": id, "spec": draft, "contract": API_CONTRACT_VERSION }),
            )
        }
        Ok(None) => error(
            404,
            "admin-unknown-spec",
            "no spec with that id exists for this project.",
        ),
        Err(err) => typed_project_assurance_error(project_dir, &err),
    }
}

fn resolve_spec_id(
    project_dir: &Path,
    project_id: &str,
    raw: &str,
) -> Result<crate::spec::SpecId, ApiResponse> {
    if let Some((prefix, hash)) = raw.split_once('-') {
        let candidate = crate::spec::SpecId {
            project_id: prefix.to_string(),
            hash: hash.to_string(),
        };
        if let Ok(Some(_)) = crate::spec::read_spec(project_dir, &candidate) {
            return Ok(candidate);
        }
    }
    let entries = match crate::spec::list_specs(project_dir) {
        Ok(entries) => entries,
        Err(err) => return Err(typed_project_assurance_error(project_dir, &err)),
    };
    let matches: Vec<&crate::spec::SpecListEntry> = entries
        .iter()
        .filter(|e| e.id.project_id == project_id && e.id.hash.starts_with(raw))
        .collect();
    match matches.len() {
        1 => Ok(matches[0].id.clone()),
        0 => Err(error(
            404,
            "admin-unknown-spec",
            "no spec with that id exists for this project.",
        )),
        _ => Err(error(
            400,
            "admin-ambiguous-spec",
            "that spec prefix matches more than one spec; use the full <project>-<hash> id.",
        )),
    }
}

/// `GET /v1/admin/projects/{id}/spec/route?finding=`: classify one
/// finding — the same decision as `forge spec route`. The finding source
/// mirrors the CLI: a `driftwatch-` prefix routes as a policy finding,
/// a `semantic-` prefix as a conflict handoff, anything else as a
/// doctor fail input. Never applies.
fn spec_route(project_dir: &Path, id: &str, finding: &str) -> ApiResponse {
    let trimmed = finding.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.contains('%')
        || trimmed.len() > 256
    {
        return error(
            400,
            "admin-invalid-assurance-query",
            "the spec `finding` query value is not valid.",
        );
    }
    let manifest = match crate::core::manifest::Manifest::load_from_dir(project_dir, None) {
        Ok((manifest, _)) => manifest,
        Err(err) => return typed_project_assurance_error(project_dir, &err),
    };
    let request = crate::spec::SpecRequest {
        project_path: project_dir.to_path_buf(),
        finding_ids: vec![trimmed.to_string()],
        reason: None,
    };
    let source = if let Some(stripped) = trimmed.strip_prefix("driftwatch-") {
        crate::spec::FindingSource::Policy(crate::policy::PolicyFinding {
            id: stripped.to_string(),
            category: "spec".to_string(),
            severity: crate::policy::PolicySeverity::Fail,
            applicable: true,
            message: format!("policy finding `{stripped}`"),
            evidence: Vec::new(),
            reason: None,
        })
    } else if let Some(stripped) = trimmed.strip_prefix("semantic-") {
        crate::spec::FindingSource::Conflict(crate::upgrade::SemanticConflict {
            project_id: manifest.project.id.clone(),
            feature: stripped.to_string(),
            owned_file: format!(".forge/features/{stripped}.receipt"),
            reason: "drifted receipt reported by the browser".to_string(),
            suggested_spec: format!("forge spec route --finding {trimmed}"),
        })
    } else {
        crate::spec::FindingSource::Doctor(crate::spec::DoctorFindingInput {
            id: trimmed.to_string(),
            status: crate::doctor::FindingStatus::Fail,
            remediation: crate::doctor::Remediation::Manual,
            category: "spec".to_string(),
            detail: format!("finding `{trimmed}` routed by the browser"),
        })
    };
    match crate::spec::route_finding(&request, &source) {
        Ok(decision) => ApiResponse::json(
            200,
            json!({ "project": id, "route": decision, "contract": API_CONTRACT_VERSION }),
        ),
        Err(err) => typed_project_assurance_error(project_dir, &err),
    }
}

/// `GET /v1/admin/projects/{id}/remediate/scan`: automatic findings —
/// the same report as `forge remediate scan`.
fn remediate_scan(project_dir: &Path, id: &str) -> ApiResponse {
    let secrets = [project_dir.display().to_string()];
    match crate::remediation::scan(project_dir) {
        Ok(report) => {
            let report = scrub_json(json!(report), &secrets);
            ApiResponse::json(
                200,
                json!({ "project": id, "scan": report, "contract": API_CONTRACT_VERSION }),
            )
        }
        Err(err) => typed_project_assurance_error(project_dir, &err),
    }
}

/// `GET /v1/admin/projects/{id}/remediate/diff?finding=&pack=`: the
/// files a plan would change — the same entries as
/// `forge remediate diff`. The plan is rebuilt server-side from the
/// finding (+ optional pack); the `--plan` file path is not reproduced.
fn remediate_diff(project_dir: &Path, id: &str, finding: &str, pack: Option<&str>) -> ApiResponse {
    let trimmed = finding.trim();
    if trimmed.is_empty() || trimmed.len() > 256 {
        return error(
            400,
            "admin-invalid-assurance-query",
            "the remediate `finding` query value is not valid.",
        );
    }
    if let Some(pack) = pack {
        let pack = pack.trim();
        if pack.is_empty() || pack.len() > 256 || pack.contains('/') || pack.contains("..") {
            return error(
                400,
                "admin-invalid-assurance-query",
                "the remediate `pack` query value is not valid.",
            );
        }
    }
    let plan = match crate::remediation::build_plan(project_dir, trimmed, pack) {
        Ok(plan) => plan,
        Err(err) => return typed_project_assurance_error(project_dir, &err),
    };
    let diff = crate::remediation::diff(&plan);
    ApiResponse::json(
        200,
        json!({
            "project": id,
            "plan": plan,
            "diff": diff,
            "contract": crate::remediation::REMEDIATION_CONTRACT_VERSION,
        }),
    )
}

/// `GET .../describe|classify/proposals`: every recorded proposal —
/// the same entries as `forge describe|classify list`.
fn semantic_list(project_dir: &Path, id: &str, kind: &str) -> ApiResponse {
    match crate::semantic::list(project_dir) {
        Ok(entries) => ApiResponse::json(
            200,
            json!({ "project": id, "kind": kind, "proposals": entries, "contract": API_CONTRACT_VERSION }),
        ),
        Err(err) => typed_project_assurance_error(project_dir, &err),
    }
}

/// `GET .../describe|classify/proposals/{proposal}`: one proposal
/// manifest — the same record as `forge describe|classify show`. The id
/// is `<kind>-<hash>`, mirroring the CLI.
fn semantic_show(project_dir: &Path, id: &str, raw: &str) -> ApiResponse {
    let (kind_label, hash) = match raw.rsplit_once('-') {
        Some((kind, hash)) => (kind, hash),
        None => {
            return error(
                400,
                "admin-invalid-assurance-query",
                "that proposal id is not valid; use the <kind>-<hash> id shown in the list.",
            )
        }
    };
    let kind = match crate::semantic::parse_kind(kind_label) {
        Ok(kind) => kind,
        Err(err) => return typed_project_assurance_error(project_dir, &err),
    };
    let manifest = match crate::core::manifest::Manifest::load_from_dir(project_dir, None) {
        Ok((manifest, _)) => manifest,
        Err(err) => return typed_project_assurance_error(project_dir, &err),
    };
    let proposal_id = crate::semantic::ProposalId {
        project_id: manifest.project.id,
        kind,
        hash: hash.to_string(),
    };
    match crate::semantic::read(project_dir, &proposal_id) {
        Ok(Some(proposal)) => ApiResponse::json(
            200,
            json!({ "project": id, "proposal": proposal, "contract": API_CONTRACT_VERSION }),
        ),
        Ok(None) => error(
            404,
            "admin-unknown-proposal",
            "no proposal with that id exists for this project.",
        ),
        Err(err) => typed_project_assurance_error(project_dir, &err),
    }
}

/// `GET .../contracts/emit?family=`: project a Core record into a
/// platform envelope — the same document as `forge contract emit`.
/// `platform.job-outcome` has no source record (CLI errors); the browser
/// reports it honestly as unavailable instead of a 500.
fn contract_emit(project_dir: &Path, id: &str, family: &str) -> ApiResponse {
    let family = family.trim();
    if !crate::contract::supported_families().contains(&family) {
        return error(
            400,
            "admin-unknown-contract-family",
            "unknown contract family; use the family shown in the contract list.",
        );
    }
    if family == "platform.job-outcome" {
        return ApiResponse::json(
            200,
            json!({
                "project": id,
                "family": family,
                "unavailable": true,
                "reason": "platform.job-outcome has no source record in this release; run `forge contract emit` in a terminal for the typed refusal.",
                "contract": API_CONTRACT_VERSION,
            }),
        );
    }
    let doc: Result<Value, crate::core::ForgeError> = match family {
        "platform.gate-result" => crate::contract::emit_gate_result(project_dir, id).map_err(|e| {
            crate::core::ForgeError::ContractInvalid {
                reason: e.to_string(),
            }
        }),
        "platform.readiness" => match crate::contract::emit_readiness(project_dir, None) {
            Ok(mut docs) => match docs.into_iter().next() {
                Some(doc) => Ok(doc),
                None => {
                    return ApiResponse::json(
                        200,
                        json!({
                            "project": id, "family": family, "unavailable": true,
                            "reason": "no readiness data exists for this project yet.",
                            "contract": API_CONTRACT_VERSION,
                        }),
                    )
                }
            },
            Err(e) => Err(crate::core::ForgeError::ContractInvalid {
                reason: e.to_string(),
            }),
        },
        "platform.release-evidence" => {
            let manifest = match crate::core::manifest::Manifest::load_from_dir(project_dir, None) {
                Ok((manifest, _)) => manifest,
                Err(err) => return typed_project_assurance_error(project_dir, &err),
            };
            let releases =
                match crate::release::engine::list_releases(project_dir, &manifest.project.id) {
                    Ok(releases) => releases,
                    Err(err) => return typed_project_assurance_error(project_dir, &err),
                };
            let latest = match releases.first() {
                Some(latest) => latest,
                None => {
                    return ApiResponse::json(
                        200,
                        json!({
                            "project": id, "family": family, "unavailable": true,
                            "reason": "no release state exists for this project yet.",
                            "contract": API_CONTRACT_VERSION,
                        }),
                    )
                }
            };
            crate::contract::emit_release_evidence(
                project_dir,
                &manifest.project.id,
                &latest.release_id,
            )
            .map_err(|e| crate::core::ForgeError::ContractInvalid {
                reason: e.to_string(),
            })
        }
        "platform.capability" => {
            let manifest = match crate::core::manifest::Manifest::load_from_dir(project_dir, None) {
                Ok((manifest, _)) => manifest,
                Err(err) => return typed_project_assurance_error(project_dir, &err),
            };
            match crate::contract::emit_capability(project_dir, &manifest.project.id) {
                Ok(Some(doc)) => Ok(doc),
                Ok(None) => {
                    return ApiResponse::json(
                        200,
                        json!({
                            "project": id, "family": family, "unavailable": true,
                            "reason": "no capabilities block exists in .project.json for this project.",
                            "contract": API_CONTRACT_VERSION,
                        }),
                    )
                }
                Err(e) => Err(crate::core::ForgeError::ContractInvalid {
                    reason: e.to_string(),
                }),
            }
        }
        "platform.audit-event" => {
            return ApiResponse::json(
                200,
                json!({
                    "project": id, "family": family, "unavailable": true,
                    "reason": "platform.audit-event projects the registry journal, not one project; run `forge contract emit --family platform.audit-event` in a terminal.",
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
        _ => {
            return error(
                400,
                "admin-unknown-contract-family",
                "unknown contract family; use the family shown in the contract list.",
            )
        }
    };
    match doc {
        Ok(doc) => {
            let secrets = [project_dir.display().to_string()];
            let doc = scrub_json(doc, &secrets);
            ApiResponse::json(
                200,
                json!({ "project": id, "family": family, "envelope": doc, "contract": API_CONTRACT_VERSION }),
            )
        }
        Err(err) => typed_project_assurance_error(project_dir, &err),
    }
}

/// `GET .../governance`: the local + configured providers — the same
/// descriptors as `forge governance list`.
fn governance_list(project_dir: &Path, id: &str) -> ApiResponse {
    match crate::governance::list_providers(project_dir) {
        Ok(providers) => ApiResponse::json(
            200,
            json!({
                "project": id, "providers": providers,
                "contract": crate::governance::GOVERNANCE_CONTRACT_VERSION,
            }),
        ),
        Err(err) => typed_project_assurance_error(project_dir, &err),
    }
}

/// `GET .../governance/status` + `.../governance/inspect`: the current
/// observation via the read-only `evaluate_project` — never
/// `check_project`, so nothing is persisted and no journal row is
/// written. An enabled external provider is reported honestly as
/// unavailable (its adapter never runs in the browser).
fn governance_observe(project_dir: &Path, id: &str) -> ApiResponse {
    let providers = match crate::governance::list_providers(project_dir) {
        Ok(providers) => providers,
        Err(err) => return typed_project_assurance_error(project_dir, &err),
    };
    if providers
        .iter()
        .any(|p| p.provider != crate::governance::LOCAL_PROVIDER_ID && p.enabled)
    {
        let name = providers
            .iter()
            .find(|p| p.provider != crate::governance::LOCAL_PROVIDER_ID && p.enabled)
            .map(|p| p.provider.clone())
            .unwrap_or_else(|| "external".to_string());
        return ApiResponse::json(
            200,
            json!({
                "project": id, "unavailable": true,
                "reason": format!("governance provider `{name}` is external; its adapter runs only in a terminal via `forge governance status`. The browser never runs provider adapters."),
                "providers": providers,
                "contract": crate::governance::GOVERNANCE_CONTRACT_VERSION,
            }),
        );
    }
    match crate::governance::evaluate_project(project_dir) {
        Ok(observation) => {
            let secrets = [project_dir.display().to_string()];
            let observation = scrub_json(json!(observation), &secrets);
            ApiResponse::json(
                200,
                json!({
                    "project": id, "observation": observation,
                    "contract": crate::governance::GOVERNANCE_CONTRACT_VERSION,
                }),
            )
        }
        Err(err) => typed_project_assurance_error(project_dir, &err),
    }
}

/// `GET .../analytics/metrics?window_days=`: registry-only metrics —
/// the same aggregates as `forge analytics metrics` minus the provider
/// plane, the summary write and the journal row. The browser never
/// probes providers.
fn analytics_metrics(db_path: &Path, id: &str, query: Option<&str>) -> ApiResponse {
    let window_days = match query_single(query, "window_days") {
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
        Err(err) => return typed_assurance_error(&err),
    };
    match crate::analytics::aggregate_project_metrics(
        &registry,
        Some(crate::analytics::DoctorSummary::default()),
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
        Err(err) => typed_assurance_error(&err),
    }
}

/// `GET .../studio/preview`: the current session envelope — the same
/// record as the CLI `studio preview` status read. A project with no
/// saved session answers `state: none`, never a 404. Never starts,
/// stops or spawns.
fn studio_preview(project_dir: &Path, id: &str) -> ApiResponse {
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
        Err(err) => typed_project_assurance_error(project_dir, &err),
    }
}
