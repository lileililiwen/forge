//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::catalog;
use crate::core::ForgeError;
use crate::registry::{Registry, ReservationOutcome};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{self, BufRead, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::contract::{API_CONTRACT_VERSION, HANDLER_TIMEOUT, READ_TIMEOUT};
use super::model::{
    ApiConfig, ApiError, ApiRequest, ApiResponse, CatalogQueryParams, ShutdownSignal,
};
use super::router::{bad_request, handle};

/// `GET /v1/interest/audit`
pub(super) fn handle_interest_audit(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let limit = match parse_interest_limit(request.query.as_deref().unwrap_or_default()) {
        Ok(value) => value,
        Err(reason) => return bad_request(&reason),
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match registry.interest_findings(limit) {
        Ok(findings) => ApiResponse::json(
            200,
            serde_json::json!({
                "interest": { "refusals": findings },
                "contract": crate::portfolio::interest::INTEREST_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// `GET /v1/interest/readiness`
/// Read-only verdict on whether aggregate evidence justifies the
/// product-owned activation follow-up. Always answers `200` for both
/// verdicts: the gate exit code is a CLI concept and an HTTP client
/// reads the verdict from the body.
pub(super) fn handle_interest_readiness(
    db_path: &Path,
    request: &ApiRequest,
    now: DateTime<Utc>,
) -> ApiResponse {
    use crate::portfolio::interest::{InterestMetric, ACTIVATION_CONTRACT_VERSION};
    let query = request.query.as_deref().unwrap_or_default().to_string();
    let mut project: Option<String> = None;
    let mut projects: Option<Vec<String>> = None;
    let mut metric: Option<String> = None;
    let mut min_value: Option<u64> = None;
    let mut source: Option<String> = None;
    let mut window: Option<String> = None;
    let mut stale_after_days = crate::portfolio::interest::DEFAULT_STALE_AFTER_DAYS;
    for pair in query.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value);
        match key.trim() {
            "project" => project = Some(value),
            "projects" => {
                projects = Some(
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|part| !part.is_empty())
                        .map(str::to_string)
                        .collect(),
                );
            }
            "metric" => metric = Some(value),
            "min_value" => match value.trim().parse::<u64>() {
                Ok(parsed) => min_value = Some(parsed),
                Err(_) => return bad_request("min_value must be an integer"),
            },
            "source" => source = Some(value),
            "window" => window = Some(value),
            "stale_after_days" => match value.trim().parse::<i64>() {
                Ok(parsed) => stale_after_days = parsed,
                Err(_) => return bad_request("stale_after_days must be an integer"),
            },
            other => {
                return bad_request(&format!(
                    "unknown interest readiness query parameter `{other}`"
                ))
            }
        }
    }
    if project.is_some() && projects.is_some() {
        return bad_request("interest readiness takes `project` or `projects`, not both");
    }
    let Some(raw_metric) = metric else {
        return bad_request("interest readiness requires a `metric` parameter");
    };
    let metric = match InterestMetric::parse(raw_metric.trim()) {
        Ok(metric) => metric,
        Err(reason) => return bad_request(&reason),
    };
    let threshold = match min_value {
        Some(value) => match crate::portfolio::interest::validate_threshold(value) {
            Ok(bound) => Some(bound),
            Err(reason) => return bad_request(&reason),
        },
        None => None,
    };
    let source = match source {
        Some(raw) if raw.trim().is_empty() => {
            return bad_request("source must not be blank");
        }
        Some(raw) => Some(raw.trim().to_string()),
        None => None,
    };
    let parsed_window = match window {
        Some(raw) => match crate::portfolio::interest::parse_window(raw.trim()) {
            Ok((start, end)) => Some((start, end)),
            Err(reason) => return bad_request(&reason),
        },
        None => None,
    };
    let window_pair = parsed_window
        .as_ref()
        .map(|(start, end)| (start.as_str(), end.as_str()));
    if let Err(reason) = crate::portfolio::interest::bound_stale_after_days(stale_after_days) {
        return bad_request(&reason);
    }
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_ids: Vec<String> = match (project, projects) {
        (Some(one), None) => vec![one.trim().to_string()],
        (None, Some(many)) => many,
        (None, None) => match registry.list() {
            Ok(records) => records.into_iter().map(|record| record.id).collect(),
            Err(err) => return ApiResponse::from_error(&err),
        },
        (Some(_), Some(_)) => {
            return bad_request("interest readiness takes `project` or `projects`, not both");
        }
    };
    match crate::portfolio::interest_report::activation_readiness(
        &registry,
        &project_ids,
        metric,
        threshold,
        source.as_deref(),
        window_pair,
        stale_after_days,
        now,
    ) {
        Ok(report) => ApiResponse::json(
            200,
            serde_json::json!({
                "contract": ACTIVATION_CONTRACT_VERSION,
                "interest": {
                    "activation": {
                        "metric": report.metric,
                        "threshold": report.threshold,
                        "stale_after_days": report.stale_after_days,
                        "requested_window": report.requested_window,
                        "requested_source": report.requested_source,
                        "ready": report.is_ready(),
                        "ready_count": report.ready_count,
                        "not_ready_count": report.not_ready_count,
                        "verdicts": report.verdicts,
                    },
                },
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

/// Parse the `stale_after_days` query parameter. An absent parameter
/// is the house default; a present but unusable one is a typed refusal
/// rather than a silently clamped bound.
pub(super) fn parse_interest_stale_after_days(query: &str) -> Result<i64, ApiResponse> {
    let Some(pair) = query.split('&').find(|part| !part.is_empty()) else {
        return Ok(crate::portfolio::interest::DEFAULT_STALE_AFTER_DAYS);
    };
    let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
    if key.trim() != "stale_after_days" {
        return Err(bad_request(&format!(
            "unknown interest query parameter `{key}`"
        )));
    }
    let parsed = value
        .trim()
        .parse::<i64>()
        .map_err(|_| bad_request("stale_after_days must be an integer"))?;
    crate::portfolio::interest::bound_stale_after_days(parsed)
        .map_err(|reason| bad_request(&reason))
}

/// Parse the `limit` query parameter of the interest audit route.
fn parse_interest_limit(query: &str) -> Result<usize, String> {
    let Some(pair) = query.split('&').find(|part| !part.is_empty()) else {
        return Ok(50);
    };
    let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
    if key.trim() != "limit" {
        return Err(format!("unknown interest audit query parameter `{key}`"));
    }
    let parsed = value
        .trim()
        .parse::<usize>()
        .map_err(|_| "limit must be an integer".to_string())?;
    if !(1..=500).contains(&parsed) {
        return Err("limit must be between 1 and 500".to_string());
    }
    Ok(parsed)
}

/// Parse the query string of a `/v1/projects/catalog` request into a
/// [`CatalogQueryParams`]. Unknown keys are a typed `api-invalid`
/// refusal — the same shape the interest routes use — so a typo
/// never silently disables a filter.
pub(super) fn parse_catalog_query_params(
    raw: Option<&str>,
) -> Result<CatalogQueryParams, ApiResponse> {
    let mut params = CatalogQueryParams::default();
    let query = raw.unwrap_or_default();
    for pair in query.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value);
        match key.trim() {
            "source" => params.sources.push(value),
            "tag" => params.tags.push(value),
            "language" => params.languages.push(value),
            "profile" => params.profiles.push(value),
            "lifecycle" => params.lifecycles.push(value),
            "repository" | "repo" => params.repositories.push(value),
            "ci" => params.ci.push(value),
            "compose" => params.compose.push(value),
            "evidence" => params.evidence.push(value),
            "filter" => params.filters.push(value),
            "limit" => match value.trim().parse::<usize>() {
                Ok(parsed) => params.limit = parsed,
                Err(_) => {
                    return Err(bad_request("limit must be a positive integer"));
                }
            },
            "cursor" => {
                if !value.trim().is_empty() {
                    params.cursor = Some(value);
                }
            }
            "max-age" | "max_age" => match value.trim().parse::<i64>() {
                Ok(parsed) => params.max_age = parsed,
                Err(_) => return Err(bad_request("max-age must be an integer")),
            },
            "workspace-registry" | "workspace_registry" => {
                if !value.trim().is_empty() {
                    params.workspace_registry = Some(PathBuf::from(value));
                }
            }
            "inventory" => {
                if !value.trim().is_empty() {
                    params.inventory = Some(PathBuf::from(value));
                }
            }
            "git-repository" | "git_repository" => {
                if !value.trim().is_empty() {
                    params.git_repositories.push(PathBuf::from(value));
                }
            }
            "github-repository" | "github_repository" => {
                if !value.trim().is_empty() {
                    params.github_repositories.push(value);
                }
            }
            other => {
                return Err(bad_request(&format!(
                    "unknown catalog query parameter `{other}`"
                )));
            }
        }
    }
    Ok(params)
}

/// Build a `CatalogSourceSelection` from the request params.
/// Unknown source kinds are a typed `api-invalid` refusal so the
/// transport surfaces a 400 rather than a silent no-op.
pub(super) fn build_catalog_selection(
    params: &CatalogQueryParams,
) -> Result<catalog::CatalogSourceSelection, ApiResponse> {
    let mut kinds: Vec<catalog::SourceKind> = Vec::new();
    if params.sources.is_empty() {
        kinds.push(catalog::SourceKind::Local);
    } else {
        for raw in &params.sources {
            let kind = catalog::SourceKind::parse(raw).ok_or_else(|| {
                bad_request(&format!(
                    "unknown --source `{raw}`; expected one of \
                     local|git|workspace-registry|inventory|github"
                ))
            })?;
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
    }
    let workspace_registry = params
        .workspace_registry
        .clone()
        .or_else(|| crate::fleet::resolve_registry_path(None));
    let inventory = params
        .inventory
        .clone()
        .or_else(|| crate::publish::inventory::resolve_source(None));
    Ok(catalog::CatalogSourceSelection {
        kinds,
        git_repositories: params.git_repositories.clone(),
        workspace_registry,
        inventory,
        github_repositories: params.github_repositories.clone(),
    })
}

/// Flatten the typed filters plus the generic `key=value` filters
/// into one ordered pair list. `CatalogQuery::from_pairs` refuses
/// an unknown key, so the surface contract — the same wire form
/// the CLI uses — stays the only place filter keys are named.
pub(super) fn catalog_filter_pairs_from(params: &CatalogQueryParams) -> Vec<String> {
    let mut pairs: Vec<String> = Vec::new();
    for value in &params.tags {
        pairs.push(format!("tag={value}"));
    }
    for value in &params.languages {
        pairs.push(format!("language={value}"));
    }
    for value in &params.profiles {
        pairs.push(format!("profile={value}"));
    }
    for value in &params.lifecycles {
        pairs.push(format!("lifecycle={value}"));
    }
    for value in &params.repositories {
        pairs.push(format!("repository={value}"));
    }
    for value in &params.ci {
        pairs.push(format!("ci={value}"));
    }
    for value in &params.compose {
        pairs.push(format!("compose={value}"));
    }
    for value in &params.evidence {
        pairs.push(format!("evidence={value}"));
    }
    pairs.extend(params.filters.iter().cloned());
    pairs
}

/// The interest routes take a comma-separated project list, so a
/// caller whose ids ever need escaping could not otherwise express
/// them. Only the three characters that actually change a query's
/// meaning are decoded; anything else is left verbatim rather than
/// guessed at.
pub(in crate::api) fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok();
            if let Some(byte) = hex.and_then(|value| u8::from_str_radix(value, 16).ok()) {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        if bytes[index] == b'+' {
            out.push(b' ');
            index += 1;
            continue;
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// Reservation helper. Reserves a pending operation,
/// invokes the closure, then finalizes the operation with/// `done` or `failed`. Returns the closure's value plus
/// the operation id.
///
/// The closure receives the reserved `op_id` and a
/// read-only [`Registry`] handle. Handlers that need to
/// mutate the registry (for example `apply_upgrade`)
/// should re-open the registry inside the closure
/// because [`Registry::open`] consumes the path and the
/// original handle is borrowed for the duration of the
/// reservation.
pub(in crate::api) fn run_with_operation<T, F>(
    db_path: &Path,
    kind: &str,
    project_id: &str,
    request: &ApiRequest,
    f: F,
) -> Result<(T, i64, String), ApiResponse>
where
    F: FnOnce(i64, &Registry) -> Result<(T, i64, String), ForgeError>,
{
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return Err(ApiResponse::from_error(&err)),
    };
    let request_hash = request_hash(request);
    let key_owned;
    let key = match request.idempotency_key.as_deref() {
        Some(value) => {
            key_owned = value.to_string();
            Some(key_owned.as_str())
        }
        None => None,
    };
    let outcome = match key {
        Some(value) => {
            match registry.reserve_idempotent_operation(kind, project_id, value, &request_hash) {
                Ok(value) => value,
                Err(err) => return Err(ApiResponse::from_error(&err)),
            }
        }
        None => {
            // No key: insert a regular pending row and
            // always go through finalize so the journal is
            // consistent.
            let detail = format!("{kind} initiated by api");
            if let Err(err) = registry.record_operation(kind, project_id, "pending", &detail) {
                return Err(ApiResponse::from_error(&err));
            }
            let entries = registry.journal_entries().ok();
            let op_id = entries
                .as_ref()
                .and_then(|list| {
                    list.iter()
                        .rev()
                        .find(|e| e.kind == kind && e.project_id == project_id)
                        .map(|e| e.op_id)
                })
                .unwrap_or(0);
            ReservationOutcome::Reserved { op_id }
        }
    };
    let op_id = match outcome {
        ReservationOutcome::Reserved { op_id } => op_id,
        ReservationOutcome::Reused { op_id } => {
            // Replay path: return the existing operation
            // without re-running the closure.
            return Err(pending_or_done_response(&registry, op_id, kind, project_id));
        }
    };
    match f(op_id, &registry) {
        Ok((value, op_id, project_id)) => {
            let detail = format!("{kind} completed");
            let _ = registry.finalize_operation(op_id, "done", &detail);
            Ok((value, op_id, project_id))
        }
        Err(err) => {
            let detail = err.to_string();
            let _ = registry.finalize_operation(op_id, "failed", &detail);
            Err(ApiResponse::from_error(&err))
        }
    }
}

fn pending_or_done_response(
    registry: &Registry,
    op_id: i64,
    kind: &str,
    project_id: &str,
) -> ApiResponse {
    // Try to fetch the existing entry; if we can, return
    // its current state with `200 OK` so the retry sees
    // the same operation identity the boundary scenario
    // requires.
    if let Ok(Some(entry)) = registry.operation(op_id) {
        let status = if entry.state == "pending" { 202 } else { 200 };
        return ApiResponse::json(
            status,
            serde_json::json!({
                "operation": entry,
                "contract": API_CONTRACT_VERSION,
                "replay": true,
                "kind": kind,
                "project_id": project_id,
            }),
        );
    }
    ApiResponse::json(
        500,
        serde_json::json!({
            "error": {
                "code": "api-internal",
                "message": "operation reservation succeeded but the record could not be re-read"
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

pub(super) fn request_hash(request: &ApiRequest) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(request.method.as_bytes());
    hasher.update(b"\n");
    hasher.update(request.path.as_bytes());
    hasher.update(b"\n");
    hasher.update(&request.body);
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Parse one HTTP/1.1 request from the wire. The
/// transport is intentionally minimal: it accepts the
/// request line plus headers, reads `Content-Length`
/// bytes, and refuses anything larger than
/// `max_body_bytes`. The body is read into a `Vec<u8>` so
/// the handler can interpret it.
pub fn parse_request(
    raw: &[u8],
    remote_addr: Option<SocketAddr>,
    max_body_bytes: usize,
) -> Result<ApiRequest, ApiError> {
    let header_end = find_header_end(raw).ok_or_else(|| {
        ApiError::Parse("no \\r\\n\\r\\n separator between headers and body".to_string())
    })?;
    let body_offset = header_end + 4;
    let body = if body_offset < raw.len() {
        raw[body_offset..].to_vec()
    } else {
        Vec::new()
    };
    if body.len() > max_body_bytes {
        return Err(ApiError::BodyTooLarge {
            limit: max_body_bytes,
            got: body.len(),
        });
    }
    let header_text = std::str::from_utf8(&raw[..header_end])
        .map_err(|err| ApiError::Parse(format!("non-UTF8 header: {err}")))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| ApiError::Parse("request line missing".to_string()))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| ApiError::Parse("method missing".to_string()))?
        .to_string();
    let target = parts
        .next()
        .ok_or_else(|| ApiError::Parse("request target missing".to_string()))?;
    let version = parts
        .next()
        .ok_or_else(|| ApiError::Parse("HTTP version missing".to_string()))?;
    if !version.starts_with("HTTP/") {
        return Err(ApiError::Parse(format!(
            "unsupported protocol `{version}`; HTTP/1.1 only"
        )));
    }
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), Some(q.to_string())),
        None => (target.to_string(), None),
    };
    let mut headers = BTreeMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| ApiError::Parse(format!("malformed header `{line}`")))?;
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }
    let idempotency_key = headers
        .get("idempotency-key")
        .filter(|value| !value.is_empty())
        .cloned();
    let bearer_token = headers.get("authorization").and_then(|value| {
        let (scheme, token) = value.split_once(' ')?;
        if scheme.eq_ignore_ascii_case("Bearer") {
            Some(token.trim().to_string())
        } else {
            None
        }
    });
    let cookies = headers
        .get("cookie")
        .map(|raw| parse_cookie_header(raw))
        .unwrap_or_default();
    Ok(ApiRequest {
        method,
        path,
        query,
        headers,
        body,
        idempotency_key,
        bearer_token,
        cookies,
        remote_addr,
        started_at: Utc::now(),
    })
}

/// Parse a `Cookie:` header into a name→value map. The
/// header is a `;`-separated list of `name=value` pairs;
/// whitespace is trimmed and the value is returned
/// undecoded because session cookies are hex tokens that
/// never need URL escaping. Duplicate names keep the first
/// occurrence so a forged `Cookie:` header cannot smuggle
/// a second `forge_session` value past the dispatch.
pub fn parse_cookie_header(raw: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for pair in raw.split(';') {
        let trimmed = pair.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (name, value) = match trimmed.split_once('=') {
            Some((n, v)) => (n.trim(), v.trim()),
            None => continue,
        };
        if name.is_empty() {
            continue;
        }
        out.entry(name.to_string())
            .or_insert_with(|| value.to_string());
    }
    out
}

fn find_header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Render a response back to the socket. Single-shot
/// write: status line, headers, body. Connection: close
/// is the safe default for a low-throughput control
/// plane; clients open a fresh connection per request.
pub fn write_response(stream: &mut TcpStream, response: &ApiResponse) -> io::Result<()> {
    let reason = reason_phrase(response.status);
    let mut buffer = Vec::with_capacity(128 + response.body.len());
    buffer.extend_from_slice(
        format!("HTTP/1.1 {status} {reason}\r\n", status = response.status).as_bytes(),
    );
    buffer.extend_from_slice(b"connection: close\r\n");
    for (name, value) in &response.headers {
        buffer.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    buffer.extend_from_slice(format!("content-length: {}\r\n", response.body.len()).as_bytes());
    buffer.extend_from_slice(b"\r\n");
    buffer.extend_from_slice(&response.body);
    stream.write_all(&buffer)?;
    stream.flush()?;
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

/// Bind a TCP listener and serve the API until the
/// shutdown signal fires. The function returns the
/// number of accepted connections on graceful shutdown.
/// The listener is bound to `config.socket_addr()` so a
/// non-loopback bind is the operator's choice, never the
/// default.
pub fn serve(
    config: &ApiConfig,
    db_path: &Path,
    shutdown: ShutdownSignal,
) -> Result<usize, ForgeError> {
    let listener =
        TcpListener::bind(config.socket_addr()).map_err(|err| ForgeError::ApiInvalid {
            reason: format!(
                "cannot bind api listener on {}: {err}",
                config.socket_addr()
            ),
        })?;
    listener
        .set_nonblocking(false)
        .map_err(|err| ForgeError::ApiInvalid {
            reason: format!("cannot configure api listener: {err}"),
        })?;
    let mut accepted = 0usize;
    for stream in listener.incoming() {
        if shutdown.is_set() {
            break;
        }
        let mut stream = match stream {
            Ok(value) => value,
            Err(err) => {
                // Transient socket error: log and continue.
                eprintln!("forge api: accept error: {err}");
                continue;
            }
        };
        accepted += 1;
        let db_path = db_path.to_path_buf();
        let shutdown = shutdown.clone();
        let max_body = config.max_body_bytes;
        let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
        let _ = stream.set_write_timeout(Some(HANDLER_TIMEOUT));
        // Inline handler so the test can drive the
        // server with a single read. The accept loop is
        // single-threaded by design (the contract is
        // "low-throughput control plane"), but a future
        // revision can move the per-connection logic
        // into a thread pool without changing the wire
        // contract.
        let response = handle_one(config, &mut stream, &db_path, max_body, shutdown);
        let _ = write_response(&mut stream, &response);
    }
    Ok(accepted)
}

fn handle_one(
    config: &ApiConfig,
    stream: &mut TcpStream,
    db_path: &Path,
    max_body: usize,
    _shutdown: ShutdownSignal,
) -> ApiResponse {
    let mut buffer = Vec::with_capacity(2048);
    let mut chunk = [0u8; 4096];
    let start = Instant::now();
    loop {
        if start.elapsed() > READ_TIMEOUT {
            return ApiError::Timeout.to_response();
        }
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buffer.extend_from_slice(&chunk[..n]);
                if find_header_end(&buffer).is_some() {
                    // Once we have the header terminator,
                    // read Content-Length more bytes for
                    // the body. Cap at max_body_bytes
                    // before allocating.
                    let header_end = find_header_end(&buffer).unwrap();
                    let body_offset = header_end + 4;
                    let content_length = header_content_length(&buffer[..header_end]).unwrap_or(0);
                    let desired = body_offset + content_length;
                    if content_length > max_body {
                        let limit = max_body;
                        let got = content_length;
                        return ApiError::BodyTooLarge { limit, got }.to_response();
                    }
                    while buffer.len() < desired {
                        if start.elapsed() > READ_TIMEOUT {
                            return ApiError::Timeout.to_response();
                        }
                        match stream.read(&mut chunk) {
                            Ok(0) => break,
                            Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                            Err(err) if err.kind() == io::ErrorKind::WouldBlock => continue,
                            Err(err) => return ApiError::Io(err.to_string()).to_response(),
                        }
                    }
                    break;
                }
                if buffer.len() > max_body + 8192 {
                    return ApiError::BodyTooLarge {
                        limit: max_body,
                        got: buffer.len(),
                    }
                    .to_response();
                }
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => continue,
            Err(err) => return ApiError::Io(err.to_string()).to_response(),
        }
    }
    let remote_addr = stream.peer_addr().ok();
    let request = match parse_request(&buffer, remote_addr, max_body) {
        Ok(value) => value,
        Err(err) => return err.to_response(),
    };
    handle(config, db_path, &request, Utc::now())
}

fn header_content_length(header_text: &[u8]) -> Option<usize> {
    let text = std::str::from_utf8(header_text).ok()?;
    for line in text.split("\r\n").skip(1) {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                return value.trim().parse().ok();
            }
        }
    }
    None
}

/// Read a full request from a `BufRead` source and
/// return the response. Exposed for tests so the
/// in-process server can be driven without a socket.
pub fn handle_buffered<R: BufRead, W: Write>(
    config: &ApiConfig,
    db_path: &Path,
    reader: &mut R,
    writer: &mut W,
    max_body_bytes: usize,
) -> io::Result<ApiResponse> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    let request = match parse_request(&buffer, None, max_body_bytes) {
        Ok(value) => value,
        Err(err) => {
            let response = err.to_response();
            let body = serde_json::to_string(
                &serde_json::from_slice::<Value>(&response.body).unwrap_or(Value::Null),
            )
            .unwrap_or_else(|_| "{}".to_string());
            writeln!(writer, "{}", body)?;
            return Ok(response);
        }
    };
    let response = handle(config, db_path, &request, Utc::now());
    writeln!(
        writer,
        "{}",
        serde_json::to_string(
            &serde_json::from_slice::<Value>(&response.body).unwrap_or(Value::Null)
        )
        .unwrap_or_else(|_| "{}".to_string())
    )?;
    Ok(response)
}
