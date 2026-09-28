//! Stable host-port allocation for independent Docker Compose
//! projects (`decoupled-remote-publish`).
//!
//! This module is a faithful Rust port of the target-hosted
//! `port_allocator.py` / `compose-ports.py` pair so Forge can own
//! port allocation from Linux while the target keeps only Docker and
//! a file tree. The Python implementation remains the oracle: the
//! unit tests in this module pin the same blocks, the same legacy-port
//! reuse, and the same override bytes the sibling produced.
//!
//! ## Registry
//!
//! `port-registry.json` stays authoritative *on the target* at
//! `RUNTIME_ROOT/port-registry.json`; Forge reads it over SSH,
//! allocates in Rust, and ships it back atomically. The document is
//! the same shape the sibling used:
//!
//! ```json
//! {"projects": {"local:alethefy": {"base": 12540, "services": {"alethefy:8000/tcp": 12540}}}}
//! ```
//!
//! Serialisation is sorted and two-space indented with a trailing
//! newline so a fleet run that allocates nothing new rewrites
//! byte-identical bytes.
//!
//! ## Allocation rules
//!
//! 1. The block base is `SHA256(identity)[0..8]` as a big-endian
//!    integer modulo the block count — stable for the same identity
//!    forever.
//! 2. A block whose ports intersect another project's block, or an
//!    unavailable port, is skipped; probing continues forward with
//!    wraparound.
//! 3. An existing record keeps its base and every assignment it
//!    already published; only new services are added.
//! 4. A legacy published port (the port the project's own Compose
//!    file declares today) is reused when it is not reserved by
//!    another project or unavailable on the target.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::core::ForgeError;

/// Default lowest host port Forge will allocate.
pub const DEFAULT_RANGE_START: u32 = 10_000;
/// Default highest host port Forge will allocate.
pub const DEFAULT_RANGE_END: u32 = 19_999;
/// Ports reserved per project, matching the sibling allocator.
pub const DEFAULT_BLOCK_SIZE: u32 = 20;
/// Default host bind address for the rendered override.
pub const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";

/// Bounded size of a port-registry document read from a target.
pub const MAX_REGISTRY_BYTES: u64 = 1_048_576;

/// Validated port range plus the per-project block size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortRange {
    pub start: u32,
    pub end: u32,
    pub block_size: u32,
}

impl PortRange {
    /// Reject ranges the sibling allocator also rejected: the window
    /// must be a real user port window and must hold at least one
    /// whole block.
    pub fn new(start: u32, end: u32, block_size: u32) -> Result<Self, ForgeError> {
        if start < 1024 || end > 65535 || start > end {
            return Err(ForgeError::PublishInvalid {
                reason: format!("port range {start}-{end} must be between 1024 and 65535"),
            });
        }
        if block_size < 1 || (end - start + 1) < block_size {
            return Err(ForgeError::PublishInvalid {
                reason: format!(
                    "port block size {block_size} does not fit inside the configured range {start}-{end}"
                ),
            });
        }
        Ok(PortRange {
            start,
            end,
            block_size,
        })
    }

    pub fn block_count(&self) -> u32 {
        (self.end - self.start + 1) / self.block_size
    }

    pub fn block_ports(&self, base: u32) -> BTreeSet<u32> {
        (base..base.saturating_add(self.block_size)).collect()
    }
}

impl Default for PortRange {
    fn default() -> Self {
        PortRange {
            start: DEFAULT_RANGE_START,
            end: DEFAULT_RANGE_END,
            block_size: DEFAULT_BLOCK_SIZE,
        }
    }
}

/// One project's registry record: its stable block base and the
/// published port assigned to each `<service>:<target>/<protocol>` key.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortRecord {
    #[serde(default)]
    pub base: u32,
    #[serde(default)]
    pub services: BTreeMap<String, u32>,
}

/// The whole target-hosted registry document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortRegistry {
    #[serde(default)]
    pub projects: BTreeMap<String, PortRecord>,
}

impl PortRegistry {
    /// Parse a registry document. An empty document is an empty
    /// registry; a malformed one is refused by name so the operator
    /// repairs the target instead of silently losing every block.
    pub fn parse(bytes: &[u8]) -> Result<Self, ForgeError> {
        if bytes.iter().all(|b| b.is_ascii_whitespace()) {
            return Ok(PortRegistry::default());
        }
        if bytes.len() as u64 > MAX_REGISTRY_BYTES {
            return Err(ForgeError::PublishInvalid {
                reason: format!(
                    "port registry document is {} bytes, above the {MAX_REGISTRY_BYTES} byte bound",
                    bytes.len()
                ),
            });
        }
        serde_json::from_slice::<PortRegistry>(bytes).map_err(|err| {
            ForgeError::DeployTargetUnavailable {
                reason: format!(
                    "target port-registry.json is not a valid registry document: {err}"
                ),
            }
        })
    }

    /// Render the registry the way the sibling wrote it: sorted keys,
    /// two-space indent, trailing newline. Byte-stable for an
    /// unchanged allocation.
    pub fn render(&self) -> Result<String, ForgeError> {
        let mut text =
            serde_json::to_string_pretty(self).map_err(|err| ForgeError::PublishInvalid {
                reason: format!("cannot encode port registry: {err}"),
            })?;
        text.push('\n');
        Ok(text)
    }
}

/// One published host port discovered in a Compose document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedPort {
    pub key: String,
    pub service: String,
    pub published: u32,
    pub target: u32,
    pub protocol: String,
}

/// Pure allocator: holds no filesystem state, so it is trivially
/// testable against the Python oracle and reusable per project.
#[derive(Debug, Clone, Copy)]
pub struct PortAllocator {
    range: PortRange,
}

impl PortAllocator {
    pub fn new(range: PortRange) -> Self {
        PortAllocator { range }
    }

    pub fn range(&self) -> PortRange {
        self.range
    }

    /// The deterministic first-choice block for an identity.
    pub fn candidate_base(&self, identity: &str) -> u32 {
        let digest = Sha256::digest(identity.as_bytes());
        let mut head = [0u8; 8];
        head.copy_from_slice(&digest[..8]);
        let block_index = u64::from_be_bytes(head) % u64::from(self.range.block_count());
        self.range.start + (block_index as u32) * self.range.block_size
    }

    /// Allocate ports for `services`, mutating `registry` in place.
    ///
    /// `legacy` carries the host ports the project's own Compose file
    /// publishes today; they are reused when free. `unavailable`
    /// carries host ports the target already binds (discovered from
    /// the target's running containers) which must never be handed
    /// out.
    pub fn allocate(
        &self,
        registry: &mut PortRegistry,
        identity: &str,
        services: &[String],
        legacy: &BTreeMap<String, u32>,
        unavailable: &BTreeSet<u32>,
    ) -> Result<PortRecord, ForgeError> {
        let mut deduped: Vec<String> = Vec::with_capacity(services.len());
        for service in services {
            if !deduped.contains(service) {
                deduped.push(service.clone());
            }
        }

        let mut reserved: BTreeSet<u32> = unavailable.clone();
        for (other_identity, record) in &registry.projects {
            if other_identity == identity {
                continue;
            }
            reserved.extend(record.services.values().copied());
        }

        let base = match registry.projects.get(identity) {
            Some(existing) => existing.base,
            None => {
                let base = self.allocate_base(identity, registry, &reserved)?;
                registry.projects.insert(
                    identity.to_string(),
                    PortRecord {
                        base,
                        services: BTreeMap::new(),
                    },
                );
                base
            }
        };

        let record = registry
            .projects
            .get_mut(identity)
            .expect("record inserted above when absent");
        for service in &deduped {
            if record.services.contains_key(service) {
                continue;
            }
            if let Some(legacy_port) = legacy.get(service) {
                if !reserved.contains(legacy_port) {
                    record.services.insert(service.clone(), *legacy_port);
                    continue;
                }
            }
            let mut used: BTreeSet<u32> = record.services.values().copied().collect();
            used.extend(reserved.iter().copied());
            let mut assigned = None;
            for offset in 0..self.range.block_size {
                let port = base.saturating_add(offset);
                if port > self.range.end {
                    break;
                }
                if !used.contains(&port) {
                    assigned = Some(port);
                    break;
                }
            }
            let Some(port) = assigned else {
                return Err(ForgeError::PublishInvalid {
                    reason: format!(
                        "no free host port in the {}-{} block of `{identity}` for service `{service}`",
                        base,
                        base.saturating_add(self.range.block_size.saturating_sub(1))
                    ),
                });
            };
            record.services.insert(service.clone(), port);
        }

        Ok(record.clone())
    }

    fn allocate_base(
        &self,
        identity: &str,
        registry: &PortRegistry,
        reserved: &BTreeSet<u32>,
    ) -> Result<u32, ForgeError> {
        let block_count = self.range.block_count();
        let first_index =
            (self.candidate_base(identity) - self.range.start) / self.range.block_size;
        for offset in 0..block_count {
            let index = (first_index + offset) % block_count;
            let base = self.range.start + index * self.range.block_size;
            let ports = self.range.block_ports(base);
            if ports.iter().any(|port| reserved.contains(port)) {
                continue;
            }
            let clashes = registry.projects.iter().any(|(other_identity, record)| {
                other_identity != identity
                    && self
                        .range
                        .block_ports(record.base)
                        .iter()
                        .any(|port| ports.contains(port))
            });
            if !clashes {
                return Ok(base);
            }
        }
        Err(ForgeError::PublishInvalid {
            reason: format!(
                "no free {}-port block remains in {}-{}",
                self.range.block_size, self.range.start, self.range.end
            ),
        })
    }
}

/// Collect published host ports from a `docker compose config
/// --format json` document.
///
/// Both the long form (`{"published": "8000", "target": 8000}`) and
/// the short string form (`"${PORT:-5000}:8080/tcp"`) are accepted, and
/// `${VAR:-default}` / `${VAR-default}` expand to their Compose
/// default exactly as the sibling's regular expressions did.
pub fn extract_published_ports(compose: &Value) -> Vec<PublishedPort> {
    let mut result: Vec<PublishedPort> = Vec::new();
    let Some(services) = compose.get("services").and_then(Value::as_object) else {
        return result;
    };
    for (service, definition) in services {
        let Some(ports) = definition.get("ports").and_then(Value::as_array) else {
            continue;
        };
        for port in ports {
            let (published, target, protocol) = match port {
                Value::Object(_) => (
                    port.get("published").and_then(scalar_to_u32),
                    port.get("target").and_then(scalar_to_u32),
                    port.get("protocol")
                        .map(|value| match value.as_str() {
                            Some(text) => text.to_string(),
                            None => scalar_to_string(value),
                        })
                        .unwrap_or_else(|| "tcp".to_string()),
                ),
                other => parse_port_string(&scalar_to_string(other)),
            };
            let (Some(published), Some(target)) = (published, target) else {
                continue;
            };
            result.push(PublishedPort {
                key: format!("{service}:{target}/{protocol}"),
                service: service.clone(),
                published,
                target,
                protocol,
            });
        }
    }
    result
}

fn scalar_to_u32(value: &Value) -> Option<u32> {
    match value {
        Value::Number(number) => number.as_u64().map(|raw| raw as u32),
        Value::String(text) => text.trim().parse::<u32>().ok(),
        _ => None,
    }
}

fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Port-string parser mirroring the sibling's `_parse_port_string`.
fn parse_port_string(raw: &str) -> (Option<u32>, Option<u32>, String) {
    let expanded = expand_compose_defaults(raw);
    let trimmed = expanded.trim().trim_matches(|c| c == '"' || c == '\'');
    let (value, protocol) = match trimmed.rsplit_once('/') {
        Some((head, tail)) => (head, tail.to_string()),
        None => (trimmed, "tcp".to_string()),
    };
    let parts: Vec<&str> = value.split(':').collect();
    let (published, target) = match parts.len() {
        2 => (parts[0], parts[1]),
        3 => (parts[1], parts[2]),
        _ => return (None, None, protocol),
    };
    if !published.chars().all(|c| c.is_ascii_digit()) || !target.chars().all(|c| c.is_ascii_digit())
    {
        return (None, None, protocol);
    }
    match (published.parse::<u32>(), target.parse::<u32>()) {
        (Ok(published), Ok(target)) => (Some(published), Some(target), protocol),
        _ => (None, None, protocol),
    }
}

/// Replace `${VAR:-default}` and `${VAR-default}` with their default,
/// leaving every other `${...}` token untouched.
pub fn expand_compose_defaults(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            out.push_str("${");
            out.push_str(after);
            return out;
        };
        let inner = &after[..end];
        let replacement = if let Some(index) = inner.find(":-") {
            Some(&inner[index + 2..])
        } else {
            inner.rfind('-').map(|index| &inner[index + 1..])
        };
        match replacement {
            Some(default) => out.push_str(default),
            None => out.push_str(&rest[start..start + end + 3]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Render the `!override` Compose document that replaces the
/// project's declared host ports. Sorted by service so a repeated
/// run rewrites identical bytes.
pub fn render_override(
    ports: &[PublishedPort],
    assignments: &BTreeMap<String, u32>,
    bind_address: &str,
) -> Result<String, ForgeError> {
    let mut by_service: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for port in ports {
        let Some(host_port) = assignments.get(&port.key) else {
            return Err(ForgeError::PublishInvalid {
                reason: format!("no host port assigned for published port `{}`", port.key),
            });
        };
        by_service
            .entry(port.service.clone())
            .or_default()
            .push(format!(
                "{bind_address}:{host_port}:{}/{}",
                port.target, port.protocol
            ));
    }
    let mut lines: Vec<String> = vec!["services:".to_string()];
    for (service, mappings) in &by_service {
        lines.push(format!("  {service}:"));
        lines.push("    ports: !override".to_string());
        for mapping in mappings {
            lines.push(format!("      - \"{mapping}\""));
        }
    }
    Ok(format!("{}\n", lines.join("\n")))
}

/// The empty override written when a project publishes no host ports.
pub fn render_empty_override() -> String {
    "services: {}\n".to_string()
}

/// Host ports the target already binds, discovered from one
/// `docker ps --format` capture. Ports outside the allocated range are
/// ignored by the caller.
pub fn published_ports_from_docker_ps(capture: &str) -> BTreeSet<u32> {
    let mut ports = BTreeSet::new();
    for line in capture.lines() {
        for mapping in line.split(',') {
            let mapping = mapping.trim();
            if mapping.is_empty() {
                continue;
            }
            let Some(arrow) = mapping.split_once("->") else {
                continue;
            };
            let Some((_address, port_text)) = arrow.0.rsplit_once(':') else {
                continue;
            };
            if let Ok(port) = port_text.trim().parse::<u32>() {
                ports.insert(port);
            }
        }
    }
    ports
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn allocator(start: u32, end: u32, block: u32) -> PortAllocator {
        PortAllocator::new(PortRange::new(start, end, block).unwrap())
    }

    #[test]
    fn range_rejects_out_of_bounds_and_block_sizes() {
        assert!(PortRange::new(80, 9000, 20).is_err());
        assert!(PortRange::new(10000, 70000, 20).is_err());
        assert!(PortRange::new(20000, 10000, 20).is_err());
        assert!(PortRange::new(10000, 10005, 20).is_err());
        assert!(PortRange::new(10000, 10019, 20).is_ok());
    }

    #[test]
    fn default_range_matches_the_sibling_defaults() {
        let range = PortRange::default();
        assert_eq!(range.start, 10_000);
        assert_eq!(range.end, 19_999);
        assert_eq!(range.block_size, 20);
        assert_eq!(range.block_count(), 500);
    }

    #[test]
    fn candidate_base_matches_python_sha256_block_hashing() {
        let allocator = allocator(10_000, 10_999, 20);
        assert_eq!(allocator.candidate_base("github.com/example/app"), 10_900);
        assert_eq!(allocator.candidate_base("github.com/example/one"), 10_240);
        assert_eq!(allocator.candidate_base("local:alethefy"), 10_540);
    }

    #[test]
    fn candidate_base_matches_python_default_range() {
        let allocator = PortAllocator::new(PortRange::default());
        assert_eq!(allocator.candidate_base("local:alethefy"), 12_540);
    }

    #[test]
    fn allocation_is_stable_across_runs() {
        let mut first = PortRegistry::default();
        let mut second = PortRegistry::default();
        let allocator = allocator(10_000, 10_999, 20);
        let services = keys(&["web", "db"]);
        let a = allocator
            .allocate(
                &mut first,
                "github.com/example/app",
                &services,
                &BTreeMap::new(),
                &BTreeSet::new(),
            )
            .unwrap();
        let b = allocator
            .allocate(
                &mut second,
                "github.com/example/app",
                &services,
                &BTreeMap::new(),
                &BTreeSet::new(),
            )
            .unwrap();
        assert_eq!(a, b);
        assert_eq!(a.base, 10_900);
        assert_eq!(a.services.get("web"), Some(&10_900));
        assert_eq!(a.services.get("db"), Some(&10_901));
        assert_eq!(first.render().unwrap(), second.render().unwrap());
    }

    #[test]
    fn second_project_probes_to_the_next_free_block() {
        let mut registry = PortRegistry::default();
        let allocator = allocator(10_000, 10_199, 20);
        let services = keys(&["web"]);
        let one = allocator
            .allocate(
                &mut registry,
                "github.com/example/one",
                &services,
                &BTreeMap::new(),
                &BTreeSet::new(),
            )
            .unwrap();
        let two = allocator
            .allocate(
                &mut registry,
                "github.com/example/two",
                &services,
                &BTreeMap::new(),
                &BTreeSet::new(),
            )
            .unwrap();
        assert_eq!(one.base, 10_040);
        assert_eq!(two.base, 10_120);
    }

    #[test]
    fn existing_record_keeps_published_ports_when_a_service_is_added() {
        let mut registry = PortRegistry::default();
        let allocator = allocator(10_000, 10_999, 20);
        let first = allocator
            .allocate(
                &mut registry,
                "github.com/example/app",
                &keys(&["web"]),
                &BTreeMap::new(),
                &BTreeSet::new(),
            )
            .unwrap();
        let updated = allocator
            .allocate(
                &mut registry,
                "github.com/example/app",
                &keys(&["web", "api"]),
                &BTreeMap::new(),
                &BTreeSet::new(),
            )
            .unwrap();
        assert_eq!(first.services.get("web"), updated.services.get("web"));
        assert_eq!(updated.services.get("web"), Some(&10_900));
        assert_eq!(updated.services.get("api"), Some(&10_901));
    }

    #[test]
    fn available_legacy_ports_are_reused() {
        let mut registry = PortRegistry::default();
        let allocator = allocator(10_000, 10_999, 20);
        let legacy = BTreeMap::from([("web".to_string(), 18080), ("db".to_string(), 15432)]);
        let result = allocator
            .allocate(
                &mut registry,
                "github.com/example/app",
                &keys(&["web", "db"]),
                &legacy,
                &BTreeSet::new(),
            )
            .unwrap();
        assert_eq!(result.base, 10_900);
        assert_eq!(result.services.get("web"), Some(&18080));
        assert_eq!(result.services.get("db"), Some(&15432));
    }

    #[test]
    fn legacy_port_held_by_another_project_is_not_reused() {
        let mut registry = PortRegistry::default();
        let allocator = allocator(10_000, 10_999, 20);
        let legacy = BTreeMap::from([("redis".to_string(), 6379)]);
        let services = keys(&["redis"]);
        let one = allocator
            .allocate(
                &mut registry,
                "github.com/example/one",
                &services,
                &legacy,
                &BTreeSet::new(),
            )
            .unwrap();
        let two = allocator
            .allocate(
                &mut registry,
                "github.com/example/two",
                &services,
                &legacy,
                &BTreeSet::new(),
            )
            .unwrap();
        assert_eq!(one.services.get("redis"), Some(&6379));
        assert_eq!(two.services.get("redis"), Some(&10_920));
    }

    #[test]
    fn unavailable_ports_are_never_assigned() {
        let mut registry = PortRegistry::default();
        let allocator = PortAllocator::new(PortRange::default());
        let services = keys(&["alethefy:8000/tcp"]);
        let legacy = BTreeMap::from([("alethefy:8000/tcp".to_string(), 8000)]);
        let result = allocator
            .allocate(
                &mut registry,
                "local:alethefy",
                &services,
                &legacy,
                &BTreeSet::from([8000]),
            )
            .unwrap();
        assert_eq!(result.base, 12_540);
        assert_eq!(result.services.get("alethefy:8000/tcp"), Some(&12_540));
    }

    #[test]
    fn exhausted_range_refuses_instead_of_colliding() {
        let mut registry = PortRegistry::default();
        let allocator = allocator(10_000, 10_019, 20);
        allocator
            .allocate(
                &mut registry,
                "local:one",
                &keys(&["web"]),
                &BTreeMap::new(),
                &BTreeSet::new(),
            )
            .unwrap();
        let err = allocator
            .allocate(
                &mut registry,
                "local:two",
                &keys(&["web"]),
                &BTreeMap::new(),
                &BTreeSet::new(),
            )
            .unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn registry_round_trips_through_target_bytes() {
        let mut registry = PortRegistry::default();
        let allocator = PortAllocator::new(PortRange::default());
        allocator
            .allocate(
                &mut registry,
                "local:alethefy",
                &keys(&["alethefy:8000/tcp"]),
                &BTreeMap::new(),
                &BTreeSet::new(),
            )
            .unwrap();
        let rendered = registry.render().unwrap();
        assert_eq!(
            rendered,
            "{\n  \"projects\": {\n    \"local:alethefy\": {\n      \"base\": 12540,\n      \"services\": {\n        \"alethefy:8000/tcp\": 12540\n      }\n    }\n  }\n}\n"
        );
        assert_eq!(PortRegistry::parse(rendered.as_bytes()).unwrap(), registry);
    }

    #[test]
    fn empty_registry_document_parses_as_empty() {
        assert_eq!(
            PortRegistry::parse(b"   \n").unwrap(),
            PortRegistry::default()
        );
        assert_eq!(PortRegistry::parse(b"{}").unwrap(), PortRegistry::default());
    }

    #[test]
    fn malformed_registry_is_refused_as_target_unavailable() {
        let err = PortRegistry::parse(b"not json").unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
    }

    #[test]
    fn extracts_compose_ports_in_long_form() {
        let compose = serde_json::json!({
            "services": {
                "web": {"ports": [{"published": "8080", "target": 80, "protocol": "tcp"}]},
                "db": {"ports": [{"published": "5432", "target": 5432, "protocol": "tcp"}]},
                "internal": {"expose": ["9000"]}
            }
        });
        let ports = extract_published_ports(&compose);
        let names: Vec<&str> = ports.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(names, vec!["db:5432/tcp", "web:80/tcp"]);
    }

    #[test]
    fn extracts_compose_ports_with_variable_defaults() {
        let compose = serde_json::json!({
            "services": {
                "api": {"ports": ["${API_PORT:-5000}:8080"]},
                "db": {"ports": ["${DB_PORT:-5432}:5432/tcp"]}
            }
        });
        let ports = extract_published_ports(&compose);
        let api = ports
            .iter()
            .find(|port| port.service == "api")
            .expect("api port");
        let db = ports
            .iter()
            .find(|port| port.service == "db")
            .expect("db port");
        assert_eq!(api.published, 5000);
        assert_eq!(api.target, 8080);
        assert_eq!(db.published, 5432);
        assert_eq!(db.target, 5432);
    }

    #[test]
    fn expand_compose_defaults_matches_python_substitutions() {
        assert_eq!(
            expand_compose_defaults("${API_PORT:-5000}:8080"),
            "5000:8080"
        );
        assert_eq!(expand_compose_defaults("${DB_PORT-5432}/tcp"), "5432/tcp");
        assert_eq!(expand_compose_defaults("${UNSET}:80"), "${UNSET}:80");
        assert_eq!(
            expand_compose_defaults("127.0.0.1:8000:8000"),
            "127.0.0.1:8000:8000"
        );
    }

    #[test]
    fn unresolvable_port_strings_are_skipped() {
        let compose = serde_json::json!({
            "services": {"web": {"ports": ["${UNSET}:80"]}}
        });
        assert!(extract_published_ports(&compose).is_empty());
    }

    #[test]
    fn renders_override_bytes_identical_to_the_sibling() {
        let compose = serde_json::json!({
            "services": {
                "web": {"ports": [{"published": "8080", "target": 80, "protocol": "tcp"}]},
                "db": {"ports": [{"published": "5432", "target": 5432, "protocol": "tcp"}]}
            }
        });
        let ports = extract_published_ports(&compose);
        let assignments = BTreeMap::from([
            ("web:80/tcp".to_string(), 18080),
            ("db:5432/tcp".to_string(), 15432),
        ]);
        let rendered = render_override(&ports, &assignments, DEFAULT_BIND_ADDRESS).unwrap();
        assert_eq!(
            rendered,
            "services:\n  db:\n    ports: !override\n      - \"127.0.0.1:15432:5432/tcp\"\n  web:\n    ports: !override\n      - \"127.0.0.1:18080:80/tcp\"\n"
        );
    }

    #[test]
    fn override_without_assignment_is_refused() {
        let compose = serde_json::json!({
            "services": {"web": {"ports": [{"published": "8080", "target": 80, "protocol": "tcp"}]}}
        });
        let ports = extract_published_ports(&compose);
        let err = render_override(&ports, &BTreeMap::new(), DEFAULT_BIND_ADDRESS).unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn empty_override_is_valid_yaml() {
        let yaml = render_empty_override();
        assert_eq!(yaml, "services: {}\n");
        let parsed: Value = serde_yaml::from_str(&yaml).unwrap();
        assert!(parsed
            .get("services")
            .unwrap()
            .as_object()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn docker_ps_capture_yields_bound_host_ports() {
        let capture = "alethefy|127.0.0.1:8000->8000/tcp\nproduction-router|127.0.0.1:28080->80/tcp\nredis|6379/tcp\n";
        let ports = published_ports_from_docker_ps(capture);
        assert!(ports.contains(&8000));
        assert!(ports.contains(&28080));
        assert_eq!(ports.len(), 2);
    }
}
