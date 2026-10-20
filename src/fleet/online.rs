//! Read-only per-project liveness verdicts for a managed fleet
//! (`fleet-liveness-status`).
//!
//! `forge deploy status` answers "what did Forge publish" from the
//! `operations` journal; nothing answered "is the service online".
//! The `decoupled-remote-publish` rollout made the gap load-bearing:
//! journal-healthy projects served fallback 404s (no Caddy rule),
//! Cloudflare 502s (rule without upstream), and crash loops
//! (missing target secrets) — all invisible to the journal.
//!
//! This module joins three live facts per publishable roster entry
//! into one typed verdict:
//!
//! - **container**: the target's `docker ps` names+status over the
//!   existing SSH transport.
//! - **route**: the host rules parsed from the *served*
//!   `platform/Caddyfile` (ground truth, not a local projection).
//! - **reachability**: one bounded HTTPS GET per routed host with a
//!   bounded body read.
//!
//! The whole surface is read-only: no journal rows, no registry
//! writes, no target writes, and every captured string passes
//! [`crate::policy::redact_credentials`] before it reaches a report
//! or the operator.
//!
//! ## The roster rule
//!
//! An entry is probed when the fleet would have published it:
//! compose-ready **and** an own-process runtime kind. That is the
//! fleet's publishing rule, read from the same place the publish
//! lane reads it ([`crate::publish::inventory::is_publishable`]).
//! A shared-host entry is a module the fleet never started, so
//! probing it would report a false alarm; it is reported as skipped
//! with the reason that names the host that owns it.
//!
//! ## Contract (`forge-fleet-liveness/0.1.0`)
//!
//! ```json
//! {
//!   "contract": "forge-fleet-liveness/0.1.0",
//!   "generated_at": "RFC3339",
//!   "inventory_source": "inventory:/path/to/inventory.json",
//!   "target": "mac",
//!   "domain": "tooosall.uk",
//!   "entries": [{
//!     "id": "alethefy",
//!     "classification": "compose_ready",
//!     "host_model": "own_process",
//!     "container": "forge-alethefy-0123456789ab (Up 5m)",
//!     "route": "http://alethefy.tooosall.uk",
//!     "http_status": 200,
//!     "verdict": "ONLINE",
//!     "detail": null
//!   }],
//!   "summary": {"online": 1, "down": 0, "no_route": 0, "not_deployed": 0}
//! }
//! ```

use std::fmt;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::publish::inventory::{is_publishable, InventoryClassification};
use crate::runtime::HostModel;

/// Versioned contract for the fleet-liveness surface.
pub const LIVENESS_CONTRACT_VERSION: &str = "forge-fleet-liveness/0.1.0";

/// Environment variable naming the SSH target the probes ride on.
/// Defaults to `mac` (matches the publish adapter's existing default
/// so `forge fleet online` and `forge publish fleet` agree on which
/// host the probes contact).
pub const SSH_TARGET_ENV: &str = "FORGE_PUBLISH_SSH_TARGET";

/// Environment variable naming the served Caddyfile path on the
/// target. The renderer writes this file under
/// `<remote_root>/platform/Caddyfile`; the env override exists for
/// tests and for hosts that mount the platform tree elsewhere.
pub const CADDYFILE_PATH_ENV: &str = "FORGE_PUBLISH_CADDYFILE_PATH";

/// Default SSH target — kept aligned with
/// [`crate::publish::jenkins::DEFAULT_SSH_TARGET`].
pub const DEFAULT_SSH_TARGET: &str = "mac";

/// Default path to the served Caddyfile, relative to the operator's
/// platform tree on the target.
pub const DEFAULT_CADDYFILE_PATH: &str = "/srv/platform/Caddyfile";

/// Per-probe HTTP timeout, in seconds. The design caps `--timeout-secs`
/// at `1..=120`; the default of 12 keeps a slow origin from holding up
/// an entire fleet but still allows TLS handshakes + small bodies.
pub const DEFAULT_HTTP_TIMEOUT_SECS: u64 = 12;

/// Maximum number of seconds any single HTTP probe may wait.
pub const MAX_HTTP_TIMEOUT_SECS: u64 = 120;

/// Hard upper bound on the body bytes a probe will read from the
/// origin. 64 KiB is enough for the unknown-hostname fallback
/// detection and any reasonable 404/503 page; anything bigger is
/// truncated so a chatty origin cannot pin the reader.
pub const MAX_PROBE_BODY_BYTES: u64 = 65_536;

/// Body text that the router's unknown-hostname fallback serves on
/// `404`. A matching body is always `DOWN` (the router rule is
/// stale or the app is not mounted) — never `ONLINE`, whatever the
/// status code.
const ROUTER_UNKNOWN_HOST_BODY: &str = "Unknown application hostname";

/// HTTP statuses that read `DOWN` regardless of body — the origin is
/// unreachable from the public face and the journal would say
/// `healthy` while the operator sees a Cloudflare 502.
const GATEWAY_ERROR_STATUSES: &[u16] = &[502, 503, 504];

/// One roster entry's outcome. The verdict is the only field an
/// operator typically reads; the others support an evidence trail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LivenessEntry {
    pub id: String,
    /// The fleet classification this entry had when selected.
    /// Only publishable entries are ever probed; the others appear
    /// in the report with their own classification so the operator
    /// can see what was skipped.
    pub classification: String,
    /// How this entry's runtime kind is hosted. An own-process kind
    /// runs as a workload of its own; a shared-host kind is a module
    /// hosted by the project that adopts it, so the fleet never
    /// started it and it is never probed. `None` when the entry's
    /// runtime kind could not be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_model: Option<HostModel>,
    /// First matched `docker ps` row, or `None` when the target has
    /// no container for this project. Container strings are redacted
    /// and bounded so a leaky Docker label never reaches the report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    /// Resolved public host (the served Caddyfile rule), or `None`
    /// when no rule exists for this project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    /// HTTP status the probe saw, or `None` when the probe was
    /// skipped (no route, no container, or the host was not probed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    /// Verdict derived from the joined facts.
    pub verdict: Verdict,
    /// Bounded cause string for `DOWN` / `NOT-DEPLOYED` /
    /// `NO-ROUTE` / `UNAVAILABLE`; never carries secrets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Typed fleet verdict. The four operational values plus
/// `UNAVAILABLE` (probe couldn't run) and `SKIPPED` (entry was
/// not eligible for probing) are the only stable strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// Routed host answered with anything other than the router
    /// fallback or a gateway error.
    #[serde(rename = "ONLINE")]
    Online,
    /// Route exists but the origin is unreachable or answers
    /// 502/503/504 or the router fallback body.
    #[serde(rename = "DOWN")]
    Down,
    /// No router rule for this project — the served Caddyfile is
    /// the ground truth, whatever the local registry predicted.
    #[serde(rename = "NO-ROUTE")]
    NoRoute,
    /// Route exists but no Compose container is running on the
    /// target — the publish either never happened or the container
    /// has crashed.
    #[serde(rename = "NOT-DEPLOYED")]
    NotDeployed,
    /// The probe could not run (binary absent, timeout, malformed
    /// Caddyfile). Distinct from `DOWN` — the operator needs to
    /// know whether to fix the probe or the app.
    #[serde(rename = "UNAVAILABLE")]
    Unavailable,
    /// The entry was not eligible for probing (an entry the fleet
    /// never published: not compose-ready, or a shared-host module).
    /// Reported with its own classification and host model; never
    /// probed, never counted as a probe outcome.
    #[serde(rename = "SKIPPED")]
    Skipped,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Online => "ONLINE",
            Verdict::Down => "DOWN",
            Verdict::NoRoute => "NO-ROUTE",
            Verdict::NotDeployed => "NOT-DEPLOYED",
            Verdict::Unavailable => "UNAVAILABLE",
            Verdict::Skipped => "SKIPPED",
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Aggregate counts. The summary is the first field an operator
/// reads; the per-entry `entries` array is the evidence trail.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LivenessSummary {
    pub online: usize,
    pub down: usize,
    pub no_route: usize,
    pub not_deployed: usize,
    pub unavailable: usize,
    /// Entries that were reported but never probed (an entry the
    /// fleet never published). Carried as a count so a fleet summary
    /// can answer "did every roster entry get a verdict?" without
    /// walking the entries.
    pub skipped: usize,
}

impl LivenessSummary {
    /// True iff every probed host is `ONLINE` and no entry is
    /// `UNAVAILABLE`. Skipped entries do not contribute to the
    /// exit code — they were already reported by the fleet
    /// classification step.
    pub fn all_probed_online(&self) -> bool {
        self.down == 0
            && self.no_route == 0
            && self.not_deployed == 0
            && self.unavailable == 0
            && self.online > 0
    }
}

/// Full fleet-liveness report. The `contract`, `generated_at`, and
/// `entries` shape are the wire contract; `summary` is derived.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LivenessReport {
    pub contract: String,
    pub generated_at: String,
    pub inventory_source: String,
    pub target: String,
    pub domain: String,
    pub entries: Vec<LivenessEntry>,
    pub summary: LivenessSummary,
}

/// What the verdict classifier needs to make one decision. Built by
/// the probe runners; never constructed directly outside this
/// module so every entry flows through `classify`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeFacts {
    pub container: Option<String>,
    pub route: Option<String>,
    pub http_status: Option<u16>,
    pub body: Option<String>,
    pub probe_error: Option<String>,
}

/// One roster entry the liveness report is built from: the
/// inventory's identity for it, the classification the inventory gave
/// it, the host model its runtime kind declares, and the live facts
/// the probes gathered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivenessCandidate {
    /// The inventory's id for the entry.
    pub id: String,
    /// The classification the inventory gave the entry.
    pub classification: InventoryClassification,
    /// How the entry's runtime kind is hosted.
    pub host_model: Option<HostModel>,
    /// The live facts the probes gathered.
    pub facts: ProbeFacts,
}

/// Compute one entry's verdict from the joined probe facts. The
/// state machine mirrors the design table:
///
/// | Container | Route     | HTTP                                    | Verdict        |
/// | --------- | --------- | --------------------------------------- | -------------- |
/// | any       | absent    | skipped                                 | `NO-ROUTE`     |
/// | absent    | present   | skipped                                 | `NOT-DEPLOYED` |
/// | present   | present   | 000 / timeout / 502 / 503 / 504         | `DOWN`         |
/// | present   | present   | router fallback body (any status)       | `DOWN`         |
/// | present   | present   | any other 2xx / 3xx / 4xx               | `ONLINE`       |
///
/// A `probe_error` is anything the probe could not attribute to an
/// HTTP status (binary absent, parse failure, timeout before
/// handshake). It always reads `UNAVAILABLE` so the operator can
/// distinguish "the app is down" from "I couldn't probe it".
pub fn classify(facts: ProbeFacts) -> (Verdict, Option<u16>, Option<String>) {
    if let Some(err) = facts.probe_error {
        return (Verdict::Unavailable, None, Some(bound_detail(err)));
    }
    let route = facts.route.as_deref().map(str::to_owned);
    let container = facts.container.as_deref().map(str::to_owned);
    match (container, route) {
        (_, None) => (Verdict::NoRoute, None, None),
        (None, Some(_)) => (
            Verdict::NotDeployed,
            None,
            Some("no compose container running on target".to_string()),
        ),
        (Some(_), Some(_)) => {
            let status = facts.http_status;
            let body = facts.body.unwrap_or_default();
            let body_lower = body.to_ascii_lowercase();
            let is_fallback = body_lower.contains(&ROUTER_UNKNOWN_HOST_BODY.to_ascii_lowercase());
            if is_fallback {
                return (
                    Verdict::Down,
                    status,
                    Some("router fallback body matched".to_string()),
                );
            }
            match status {
                Some(code) if GATEWAY_ERROR_STATUSES.contains(&code) => (
                    Verdict::Down,
                    Some(code),
                    Some(format!("gateway error {code}")),
                ),
                Some(0) | None => (
                    Verdict::Down,
                    status,
                    Some("connection failed or timed out".to_string()),
                ),
                // Any other 2xx / 3xx / 4xx — including 404 — is the
                // application answering. An API-only service that 404s
                // its own root is still online.
                Some(code) => (Verdict::Online, Some(code), None),
            }
        }
    }
}

fn bound_detail(detail: String) -> String {
    const MAX_DETAIL: usize = 200;
    let truncated = if detail.len() > MAX_DETAIL {
        let mut end = MAX_DETAIL;
        while !detail.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &detail[..end])
    } else {
        detail
    };
    redact_credentials(&truncated)
}

/// Parse the served `platform/Caddyfile` and return every routed
/// host (e.g. `alethefy.tooosall.uk`). The nav host
/// (`apps.<domain>`) and the `:80` fallback are excluded — neither
/// answers for a single project, so probing them would always
/// `DOWN`. Malformed rules are surfaced as an error so the operator
/// never gets a fabricated "no route" answer when the truth is
/// "the file is broken".
pub fn parse_caddyfile_hosts(
    content: &str,
    domain: &str,
    nav_host: &str,
) -> Result<Vec<String>, ForgeError> {
    let mut hosts = Vec::new();
    let mut found_any_rule = false;
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Accept the rendered shape `http://<host> {` only. Anything
        // else is either a directive inside an existing block or a
        // file we don't understand; either way, not a host rule.
        let Some(rest) = line.strip_prefix("http://") else {
            continue;
        };
        let Some(host_part) = rest.split_whitespace().next() else {
            continue;
        };
        // Strip the optional trailing `{` so `host_part` is a clean host.
        let host = host_part.trim_end_matches('{');
        if host.is_empty() {
            continue;
        }
        found_any_rule = true;
        if host == ":80" {
            continue;
        }
        if host == nav_host || host == format!("{nav_host}.{domain}") {
            continue;
        }
        // The rendered file writes hosts scoped to the configured
        // domain. Hosts in some other domain are left in the list —
        // a multi-domain router is intentional, not malformed.
        hosts.push(host.to_string());
    }
    // An empty file (or one with only comments / a bare `:80`)
    // reads as "no rules" — distinct from a file we couldn't
    // parse. The caller decides what to do with that.
    if !found_any_rule {
        return Ok(Vec::new());
    }
    Ok(hosts)
}

/// Look up the routed host for one project id. The convention is
/// `<id>.<domain>` (matches the renderer), so an entry is `None`
/// unless the served Caddyfile contains exactly that host. This is
/// deliberately narrower than a substring match: the operator who
/// publishes `foo` and sees `foo-bar` as online would be reading
/// someone else's app.
pub fn route_for_project(hosts: &[String], project_id: &str, domain: &str) -> Option<String> {
    let wanted = format!("{project_id}.{domain}");
    hosts.iter().find(|host| host.as_str() == wanted).cloned()
}

/// SSH argv used to read the served Caddyfile from the target.
/// Returned as a `Vec<OsString>` so the call site can hand it to
/// `SubprocessTransport::run` unchanged. Public so the CLI
/// command's `--dry-run` can render the exact argv.
///
/// When the target looks like a local path (starts with `/`), it
/// is invoked directly — the test stub uses this path to redirect
/// the probe to a fake `ssh` script without `ssh` itself trying to
/// resolve the path as a hostname.
pub fn cat_caddyfile_command(ssh_target: &str, caddyfile_path: &str) -> Vec<std::ffi::OsString> {
    if ssh_target.starts_with('/') {
        [
            std::ffi::OsString::from(ssh_target),
            std::ffi::OsString::from("cat"),
            std::ffi::OsString::from(caddyfile_path),
        ]
        .to_vec()
    } else {
        [
            std::ffi::OsString::from("ssh"),
            std::ffi::OsString::from(ssh_target),
            std::ffi::OsString::from("cat"),
            std::ffi::OsString::from(caddyfile_path),
        ]
        .to_vec()
    }
}

/// SSH argv used to read Compose container names from the target.
/// Returned as a `Vec<OsString>` so the call site can hand it to
/// `SubprocessTransport::run` unchanged. Mirrors the
/// `remote_compose::docker_command` argv shape so the same PATH
/// override reaches Docker Desktop's helper. When the target is a
/// local path (a test stub), it is invoked directly so the stub
/// answers the same argv shape production does.
pub fn docker_ps_command(ssh_target: &str) -> Vec<std::ffi::OsString> {
    if ssh_target.starts_with('/') {
        [
            std::ffi::OsString::from(ssh_target),
            std::ffi::OsString::from("env"),
            std::ffi::OsString::from(format!(
                "PATH={}",
                crate::publish::remote_compose::DOCKER_PATH
            )),
            std::ffi::OsString::from(crate::publish::remote_compose::DOCKER_BIN),
            std::ffi::OsString::from("ps"),
            std::ffi::OsString::from("--no-trunc"),
            std::ffi::OsString::from("--format"),
            std::ffi::OsString::from("{{.Names}}\t{{.Status}}"),
        ]
        .to_vec()
    } else {
        [
            std::ffi::OsString::from("ssh"),
            std::ffi::OsString::from(ssh_target),
            std::ffi::OsString::from("env"),
            std::ffi::OsString::from(format!(
                "PATH={}",
                crate::publish::remote_compose::DOCKER_PATH
            )),
            std::ffi::OsString::from(crate::publish::remote_compose::DOCKER_BIN),
            std::ffi::OsString::from("ps"),
            std::ffi::OsString::from("--no-trunc"),
            std::ffi::OsString::from("--format"),
            std::ffi::OsString::from("{{.Names}}\t{{.Status}}"),
        ]
        .to_vec()
    }
}

/// Run `curl` for one HTTPS GET with bounded body size and timeout.
/// Returns `(status, body)` where `status` is the HTTP status (or
/// `0` for connection-level failure) and `body` is the bounded
/// response body. Every captured string is redacted.
pub fn probe_http(url: &str, timeout_secs: u64) -> Result<(u16, String), ForgeError> {
    use std::io::Read;
    let timeout_secs = timeout_secs.clamp(1, MAX_HTTP_TIMEOUT_SECS);
    let mut cmd = Command::new("curl");
    cmd.args([
        "-s",
        "-m",
        &timeout_secs.to_string(),
        "--max-filesize",
        &MAX_PROBE_BODY_BYTES.to_string(),
        "-o",
        "-",
        "-w",
        "\n%{http_code}",
        url,
    ]);
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    let mut child = cmd.spawn().map_err(|err| ForgeError::PublishInvalid {
        reason: format!("cannot spawn `curl` for HTTPS probe: {err}"),
    })?;
    let mut out = Vec::new();
    if let Some(mut stdout) = child.stdout.take() {
        let mut limited = (&mut stdout).take(MAX_PROBE_BODY_BYTES + 32);
        let _ = limited.read_to_end(&mut out);
    }
    let mut err = Vec::new();
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_end(&mut err);
    }
    let status = child.wait().map_err(|err| ForgeError::PublishInvalid {
        reason: format!("curl wait failed: {err}"),
    })?;
    let _ = status; // status is read from the trailing line of stdout
    let stdout_text = String::from_utf8_lossy(&out).into_owned();
    let stderr_text = String::from_utf8_lossy(&err).into_owned();
    if !stderr_text.trim().is_empty() {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "HTTPS probe to `{url}` failed: {}",
                redact_credentials(stderr_text.trim())
            ),
        });
    }
    // The trailing line carries the status code; everything before
    // it is the response body (bounded by `--max-filesize`).
    let (body_part, status_part) = match stdout_text.rsplit_once('\n') {
        Some((body, status)) => (body.to_string(), status.trim().to_string()),
        None => (String::new(), String::new()),
    };
    let bounded_body: String = if body_part.len() > MAX_PROBE_BODY_BYTES as usize {
        let mut end = MAX_PROBE_BODY_BYTES as usize;
        while end > 0 && !body_part.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &body_part[..end])
    } else {
        body_part
    };
    let http_status: u16 = status_part.parse().unwrap_or(0u16);
    Ok((http_status, redact_credentials(&bounded_body)))
}

/// Parse `docker ps` output (`<name>\t<status>` per line) and
/// return the first row whose Compose project name starts with
/// `forge-<project>-`. The container string is bounded and
/// redacted before it reaches the report.
pub fn match_container(ps_output: &str, project_id: &str) -> Option<String> {
    const MAX_CONTAINER_CHARS: usize = 120;
    let prefix = format!("forge-{project_id}-");
    for raw in ps_output.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let (name, status) = match line.split_once('\t') {
            Some((n, s)) => (n, s),
            None => (line, ""),
        };
        if !name.starts_with(&prefix) {
            continue;
        }
        let combined = if status.is_empty() {
            name.to_string()
        } else {
            format!("{name} ({status})")
        };
        let bounded = if combined.len() > MAX_CONTAINER_CHARS {
            let mut end = MAX_CONTAINER_CHARS;
            while end > 0 && !combined.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}…", &combined[..end])
        } else {
            combined
        };
        return Some(redact_credentials(&bounded));
    }
    None
}

/// Read the SSH target from the environment, falling back to the
/// publish adapter's default. Two surfaces share the env so a
/// probe never silently diverges from a publish.
pub fn ssh_target_from_env() -> String {
    std::env::var(SSH_TARGET_ENV)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_SSH_TARGET.to_string())
}

/// Read the served Caddyfile path from the environment, falling
/// back to the default `/srv/platform/Caddyfile`. The renderer
/// writes this file under the operator's platform tree; the env
/// override exists for hosts that mount the tree elsewhere.
pub fn caddyfile_path_from_env() -> String {
    std::env::var(CADDYFILE_PATH_ENV)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_CADDYFILE_PATH.to_string())
}

/// Build a `LivenessReport` from one inventory classification
/// result and one set of probe facts. Carries every roster
/// entry (probed and skipped) so the operator can see what was
/// reported versus what was probed. The `target`, `domain`, and
/// `inventory_source` labels travel with the report so a JSON
/// consumer can tell two runs apart.
///
/// The rule is the fleet's publishing rule: an entry is probed when
/// the fleet would have published it. A shared-host entry is a module
/// the fleet never started, so it is skipped with the reason that
/// names the host that owns it — probing it would report a false
/// alarm.
pub fn build_report(
    entries: Vec<LivenessCandidate>,
    inventory_source: String,
    target: String,
    domain: String,
    generated_at: String,
) -> LivenessReport {
    let mut report_entries = Vec::with_capacity(entries.len());
    let mut summary = LivenessSummary::default();
    for candidate in entries {
        let LivenessCandidate {
            id,
            classification,
            host_model,
            facts,
        } = candidate;
        let (verdict, http_status, detail) = if is_publishable(classification, host_model) {
            classify(facts.clone())
        } else {
            // Entries the fleet never published are reported with
            // their own classification and host model and never
            // probed — never run them through the verdict classifier,
            // which would otherwise downgrade every one of them to
            // `NO-ROUTE` or `UNAVAILABLE` even when the inventory
            // said the real reason was `compose_missing` /
            // `source_unavailable`, or when the entry is a module
            // the fleet never started.
            (
                Verdict::Skipped,
                None,
                host_model
                    .and_then(|model| model.hold_back_reason())
                    .map(str::to_string),
            )
        };
        match verdict {
            Verdict::Online => summary.online += 1,
            Verdict::Down => summary.down += 1,
            Verdict::NoRoute => summary.no_route += 1,
            Verdict::NotDeployed => summary.not_deployed += 1,
            Verdict::Unavailable => summary.unavailable += 1,
            Verdict::Skipped => summary.skipped += 1,
        }
        let entry = LivenessEntry {
            id,
            classification: classification.as_str().to_string(),
            host_model,
            container: facts.container,
            route: facts.route,
            http_status,
            verdict,
            detail,
        };
        report_entries.push(entry);
    }
    LivenessReport {
        contract: LIVENESS_CONTRACT_VERSION.to_string(),
        generated_at,
        inventory_source,
        target,
        domain,
        entries: report_entries,
        summary,
    }
}
