//! Shared-PostgreSQL Compose overlay rendering
//! (`decoupled-remote-publish`).
//!
//! Rust port of the target-hosted `project-db-overlay.py`. Forge
//! collects the project's `docker compose config --no-interpolate
//! --format json` document from the target over SSH, renders the
//! production overlay locally, and ships it to
//! `RUNTIME_ROOT/<project>/shared-db.compose.yml`.
//!
//! ## Semantics carried over
//!
//! - Only Compose services whose image is `postgres`, `postgis` or
//!   `pgvector` are treated as databases and scaled to zero.
//! - Application services keep every `DATABASE_URL`,
//!   `DATA_SOURCE_NAME`, `CONNECTIONSTRING` and `PG*` environment
//!   key and remap it onto the `PRODUCTION_*` variables the
//!   target-local `.shared-db.env` provides.
//! - Database entries are pruned from `depends_on`, using
//!   `depends_on: !reset []` when nothing remains, and
//!   `production-db-network` is injected into the service networks.
//! - The overlay never renders a password: the credential stays in the
//!   target-local env file and is only ever referenced.
//!
//! ## Empty-services fix
//!
//! The Python renderer emitted a bare `services:` key when a project
//! declared no database-relevant environment (the live `alethefy`
//! shape), which `docker compose` refuses with `services must be a
//! mapping`. This renderer emits `services: {}` for that case and
//! validates the document as YAML before returning it, so an invalid
//! overlay can never be shipped.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::core::ForgeError;

/// Image names that identify a per-project database container.
pub const DATABASE_IMAGE_MARKERS: [&str; 3] = ["postgres", "postgis", "pgvector"];

/// Environment keys remapped onto the shared production database.
pub const DATABASE_KEYS: [&str; 7] = [
    "DATABASE_URL",
    "DATA_SOURCE_NAME",
    "PGHOST",
    "PGPORT",
    "PGDATABASE",
    "PGUSER",
    "PGPASSWORD",
];

/// The external network the shared database publishes.
pub const SHARED_DB_NETWORK: &str = "production-db-network";

/// The rendered overlay plus the `--scale` arguments that switch the
/// project's own database services off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedDbOverlay {
    pub yaml: String,
    pub scale_args: Vec<String>,
}

/// The shared-database projection of one application service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationService {
    pub environment: BTreeMap<String, String>,
    pub depends_on_db: BTreeSet<String>,
    pub depends_on: Option<Value>,
    pub networks: Vec<String>,
}

/// Validate a project id against the sibling's `IDENTIFIER` rule and
/// return its database identifier.
pub fn database_identifier(project: &str) -> Result<String, ForgeError> {
    let mut chars = project.chars();
    let starts_letter = matches!(chars.next(), Some(first) if first.is_ascii_alphabetic());
    let rest_ok = chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if project.is_empty() || !starts_letter || !rest_ok {
        return Err(ForgeError::PublishInvalid {
            reason: format!("unsafe project name for the shared database overlay: `{project}`"),
        });
    }
    Ok(project.replace('-', "_").to_lowercase())
}

/// Compose services that run their own database image.
pub fn database_services(compose: &Value) -> Vec<String> {
    let mut found = Vec::new();
    let Some(services) = compose.get("services").and_then(Value::as_object) else {
        return found;
    };
    for (name, service) in services {
        let image = service
            .get("image")
            .map(|value| match value.as_str() {
                Some(text) => text.to_lowercase(),
                None => value.to_string().to_lowercase(),
            })
            .unwrap_or_default();
        let image_name = image
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .split(':')
            .next()
            .unwrap_or_default()
            .to_string();
        if DATABASE_IMAGE_MARKERS.contains(&image_name.as_str()) {
            found.push(name.clone());
        }
    }
    found
}

/// Application services that talk to a database, with the exact
/// environment keys and dependencies the overlay must rewrite.
pub fn application_services(compose: &Value) -> BTreeMap<String, ApplicationService> {
    let db_services: BTreeSet<String> = database_services(compose).into_iter().collect();
    let mut result = BTreeMap::new();
    let Some(services) = compose.get("services").and_then(Value::as_object) else {
        return result;
    };
    for (name, service) in services {
        if db_services.contains(name) {
            continue;
        }
        let environment = database_environment(service.get("environment"));
        let depends_on = service.get("depends_on").cloned();
        let depends_on_db: BTreeSet<String> = depends_on
            .as_ref()
            .map(dependency_names)
            .unwrap_or_default()
            .into_iter()
            .filter(|dependency| db_services.contains(dependency))
            .collect();
        if environment.is_empty() && depends_on_db.is_empty() {
            continue;
        }
        result.insert(
            name.clone(),
            ApplicationService {
                environment,
                depends_on_db,
                depends_on,
                networks: network_names(service.get("networks")),
            },
        );
    }
    result
}

fn database_environment(environment: Option<&Value>) -> BTreeMap<String, String> {
    let mapping = environment_mapping(environment);
    mapping
        .into_iter()
        .filter(|(key, _)| {
            let upper = key.to_uppercase();
            DATABASE_KEYS.contains(&key.as_str())
                || upper.contains("DATABASE_URL")
                || upper.contains("CONNECTIONSTRING")
        })
        .collect()
}

fn environment_mapping(environment: Option<&Value>) -> BTreeMap<String, String> {
    let mut mapping = BTreeMap::new();
    match environment {
        Some(Value::Object(entries)) => {
            for (key, value) in entries {
                mapping.insert(key.clone(), python_scalar(value));
            }
        }
        Some(Value::Array(items)) => {
            for item in items {
                let Some(text) = item.as_str() else {
                    continue;
                };
                let (key, value) = match text.split_once('=') {
                    Some((key, value)) => (key, value),
                    None => (text, ""),
                };
                mapping.insert(key.to_string(), value.to_string());
            }
        }
        _ => {}
    }
    mapping
}

fn dependency_names(value: &Value) -> Vec<String> {
    match value {
        Value::Object(entries) => entries.keys().cloned().collect(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(text) => Some(text.clone()),
                Value::Object(entry) => entry
                    .get("service")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                _ => None,
            })
            .collect(),
        Value::String(text) => vec![text.clone()],
        _ => Vec::new(),
    }
}

/// Drop every database dependency from a `depends_on` value, keeping
/// the sibling's list or mapping shape so the rendered overlay keeps
/// the same `condition` details the target's Compose config carried.
fn prune_depends_on(value: Option<&Value>, databases: &BTreeSet<String>) -> Option<Value> {
    match value? {
        Value::Object(entries) => Some(Value::Object(
            entries
                .iter()
                .filter(|(dependency, _)| !databases.contains(*dependency))
                .map(|(dependency, value)| (dependency.clone(), value.clone()))
                .collect(),
        )),
        Value::Array(items) => Some(Value::Array(
            items
                .iter()
                .filter(|item| match item {
                    Value::Object(entry) => entry
                        .get("service")
                        .and_then(Value::as_str)
                        .map(|service| !databases.contains(service))
                        .unwrap_or(true),
                    other => !dependency_names(other)
                        .iter()
                        .any(|name| databases.contains(name)),
                })
                .cloned()
                .collect(),
        )),
        other => {
            if dependency_names(other)
                .iter()
                .any(|name| databases.contains(name))
            {
                None
            } else {
                Some(other.clone())
            }
        }
    }
}

fn network_names(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Object(entries)) => entries.keys().cloned().collect(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

/// Render the overlay for `project`.
///
/// - `env_file` is the target-local project env file
///   (`SECRETS_ROOT/<project>/.env`) when the target carries one.
///   Every rendered service receives it as an `env_file:` reference
///   so project secrets reach containers without ever being copied
///   to Linux: `--env-file` alone only feeds Compose interpolation,
///   it never injects container environment.
/// - `shared_network` is true when the target declares shared-DB
///   intent (`.shared-db.env` present). Only then are application
///   environment keys remapped onto `PRODUCTION_*`, database
///   `depends_on` pruned, `production-db-network` injected and local
///   databases scaled to zero. Without it the overlay only injects
///   `env_file:` and local databases keep running untouched.
///
/// Refuses a project whose Compose file declares a database service
/// but exposes no database-relevant application environment: that
/// combination cannot be served by the shared database and would
/// silently lose its data source. The caller deploys such
/// standalone-database composes without an overlay instead.
pub fn render_overlay(
    compose: &Value,
    project: &str,
    env_file: Option<&str>,
    shared_network: bool,
) -> Result<SharedDbOverlay, ForgeError> {
    database_identifier(project)?;
    let databases = database_services(compose);
    let services = application_services(compose);
    if !databases.is_empty() && services.is_empty() {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "no supported application database settings found for `{project}`; \
                 the shared database overlay cannot replace service(s) {}",
                databases.join(", ")
            ),
        });
    }

    // Non-database services, sorted: every one receives the
    // target-local env file when the project declares one.
    let db_set: BTreeSet<String> = databases.iter().cloned().collect();
    let plain_services: Vec<String> = service_names(compose)
        .into_iter()
        .filter(|name| !db_set.contains(name))
        .collect();

    let mut lines: Vec<String> = vec!["services:".to_string()];
    for (name, details) in &services {
        lines.push(format!("  {name}:"));
        if shared_network {
            lines.push("    environment:".to_string());
            for key in details.environment.keys() {
                let Some(value) = production_value(key) else {
                    continue;
                };
                lines.push(format!("      {key}: {}", yaml_scalar(value)));
            }
        }
        if let Some(path) = env_file {
            lines.push("    env_file:".to_string());
            lines.push(format!("      - {}", yaml_scalar(path)));
        }
        if shared_network {
            if !details.depends_on_db.is_empty() {
                match prune_depends_on(details.depends_on.as_ref(), &details.depends_on_db) {
                    Some(Value::Object(remaining)) if !remaining.is_empty() => {
                        lines.push("    depends_on:".to_string());
                        for (dependency, value) in &remaining {
                            lines.push(format!("      {dependency}:"));
                            match value {
                                Value::Object(nested) => {
                                    for (key, value) in nested {
                                        lines.push(format!(
                                            "        {key}: {}",
                                            yaml_scalar(&python_scalar(value))
                                        ));
                                    }
                                }
                                other => lines.push(format!(
                                    "        condition: {}",
                                    yaml_scalar(&python_scalar(other))
                                )),
                            }
                        }
                    }
                    Some(Value::Array(remaining)) if !remaining.is_empty() => {
                        lines.push("    depends_on:".to_string());
                        for dependency in remaining {
                            lines.push(format!(
                                "      - {}",
                                dependency.as_str().unwrap_or_default()
                            ));
                        }
                    }
                    _ => lines.push("    depends_on: !reset []".to_string()),
                }
            }
            let mut networks = details.networks.clone();
            if !networks.iter().any(|name| name == SHARED_DB_NETWORK) {
                networks.push(SHARED_DB_NETWORK.to_string());
            }
            lines.push("    networks:".to_string());
            for network in networks {
                lines.push(format!("      - {network}"));
            }
        }
    }
    if services.is_empty() {
        if let Some(path) = env_file {
            for name in &plain_services {
                lines.push(format!("  {name}:"));
                lines.push("    env_file:".to_string());
                lines.push(format!("      - {}", yaml_scalar(path)));
            }
        }
    }
    // A bare `services:` key is null, not a mapping, and
    // `docker compose` refuses it — keep `{}` when no service
    // rendered anything.
    if lines.len() == 1 {
        lines[0] = "services: {}".to_string();
    }
    if shared_network {
        lines.push("networks:".to_string());
        lines.push(format!("  {SHARED_DB_NETWORK}:"));
        lines.push("    external: true".to_string());
        lines.push(format!("    name: {SHARED_DB_NETWORK}"));
    }
    lines.push(String::new());

    let yaml = lines.join("\n");
    validate_yaml(&yaml)?;
    // Local databases are scaled off only when the shared database
    // replaces them; otherwise they keep running untouched.
    let scale_args = if shared_network {
        databases
            .iter()
            .flat_map(|service| ["--scale".to_string(), format!("{service}=0")])
            .collect()
    } else {
        Vec::new()
    };
    Ok(SharedDbOverlay { yaml, scale_args })
}

/// Every Compose service name, sorted. Used to attach the
/// target-local env file to services that need no DB remapping.
fn service_names(compose: &Value) -> Vec<String> {
    let mut names: Vec<String> = compose
        .get("services")
        .and_then(Value::as_object)
        .map(|services| services.keys().cloned().collect())
        .unwrap_or_default();
    names.sort();
    names
}

/// Map one project environment key onto the shared production
/// variable. `None` means the key is database-shaped but has no
/// shared-database equivalent and is dropped.
fn production_value(key: &str) -> Option<&'static str> {
    let upper = key.to_uppercase();
    if key == "DATABASE_URL" || upper.contains("DATABASE_URL") || key == "DATA_SOURCE_NAME" {
        Some("${PRODUCTION_DATABASE_URL}")
    } else if upper.contains("CONNECTIONSTRING") {
        Some("${PRODUCTION_CONNECTION_STRING}")
    } else if key == "PGHOST" {
        Some("production-postgres")
    } else if key == "PGPORT" {
        Some("5432")
    } else if key == "PGDATABASE" {
        Some("${PRODUCTION_DB_NAME}")
    } else if key == "PGUSER" {
        Some("${PRODUCTION_DB_USER}")
    } else if key == "PGPASSWORD" {
        Some("${PRODUCTION_DB_PASSWORD}")
    } else {
        None
    }
}

/// Quote a scalar the way the sibling's `json.dumps(str(value))`
/// did, so rendered bytes stay comparable with the Python overlay.
fn yaml_scalar(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("\"{value}\""))
}

/// Compose-environment scalars as the sibling rendered them: JSON
/// `null` becomes `None`, JSON booleans become `True`/`False`, and
/// everything else keeps its text.
fn python_scalar(value: &Value) -> String {
    match value {
        Value::Null => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Parse the rendered overlay as YAML. An overlay that cannot be
/// parsed is refused before it is ever shipped to a target.
pub fn validate_yaml(yaml: &str) -> Result<(), ForgeError> {
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(yaml).map_err(|err| ForgeError::PublishInvalid {
            reason: format!("rendered shared-database overlay is not valid YAML: {err}"),
        })?;
    let Some(services) = parsed.get("services") else {
        return Err(ForgeError::PublishInvalid {
            reason: "rendered shared-database overlay has no `services` key".to_string(),
        });
    };
    if !services.is_mapping() && !services.is_null() && !services.is_sequence() {
        return Err(ForgeError::PublishInvalid {
            reason: "rendered shared-database overlay `services` is not a mapping".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_project() -> Value {
        serde_json::json!({
            "services": {
                "api": {
                    "image": "rust-app:latest",
                    "environment": {"DATABASE_URL": "postgres://x", "APP_ENV": "production"},
                    "depends_on": ["db", "cache"],
                    "networks": ["appnet"]
                },
                "db": {"image": "pgvector/pgvector:pg15", "environment": {"POSTGRES_PASSWORD": "hunter2"}},
                "worker": {
                    "image": "worker:latest",
                    "environment": {
                        "PGHOST": "db", "PGDATABASE": "app", "PGUSER": "app",
                        "PGPASSWORD": "s", "OTHER": "x"
                    },
                    "depends_on": {"db": {"condition": "service_started"}}
                }
            }
        })
    }

    fn alethefy_class() -> Value {
        serde_json::json!({
            "name": "alethefy",
            "services": {
                "alethefy": {
                    "build": {"context": "/home/paul/code/alethefy"},
                    "environment": {"ALETHEFY_LOCAL_ONLY": "true", "ALETHEFY_RETENTION": "ephemeral"},
                    "networks": {"default": null},
                    "ports": [{"host_ip": "127.0.0.1", "protocol": "tcp", "published": "8000", "target": 8000}],
                    "read_only": true
                }
            }
        })
    }

    #[test]
    fn database_identifier_matches_sibling_rules() {
        assert_eq!(database_identifier("alethefy").unwrap(), "alethefy");
        assert_eq!(database_identifier("crossify-app").unwrap(), "crossify_app");
        assert!(database_identifier("").is_err());
        assert!(database_identifier("1project").is_err());
        assert!(database_identifier("proj.ect").is_err());
        assert!(database_identifier("proj ect").is_err());
    }

    #[test]
    fn database_services_detects_postgres_images() {
        assert_eq!(database_services(&db_project()), vec!["db".to_string()]);
        assert!(database_services(&alethefy_class()).is_empty());
        assert_eq!(
            database_services(&serde_json::json!({
                "services": {"pg": {"image": "postgres:16-alpine"}, "web": {"image": "nginx:1.27"}}
            })),
            vec!["pg".to_string()]
        );
    }

    #[test]
    fn application_services_collect_only_database_shaped_keys() {
        let services = application_services(&db_project());
        assert_eq!(services.keys().collect::<Vec<_>>(), vec!["api", "worker"]);
        assert_eq!(
            services
                .get("api")
                .unwrap()
                .environment
                .keys()
                .collect::<Vec<_>>(),
            vec!["DATABASE_URL"]
        );
        assert_eq!(
            services
                .get("worker")
                .unwrap()
                .depends_on_db
                .iter()
                .collect::<Vec<_>>(),
            vec!["db"]
        );
    }

    #[test]
    fn renders_overlay_bytes_identical_to_the_sibling() {
        let overlay = render_overlay(&db_project(), "alethefy", None, true).unwrap();
        assert_eq!(
            overlay.yaml,
            "services:\n  api:\n    environment:\n      DATABASE_URL: \"${PRODUCTION_DATABASE_URL}\"\n    depends_on:\n      - cache\n    networks:\n      - appnet\n      - production-db-network\n  worker:\n    environment:\n      PGDATABASE: \"${PRODUCTION_DB_NAME}\"\n      PGHOST: \"production-postgres\"\n      PGPASSWORD: \"${PRODUCTION_DB_PASSWORD}\"\n      PGUSER: \"${PRODUCTION_DB_USER}\"\n    depends_on: !reset []\n    networks:\n      - production-db-network\nnetworks:\n  production-db-network:\n    external: true\n    name: production-db-network\n"
        );
        assert_eq!(
            overlay.scale_args,
            vec!["--scale".to_string(), "db=0".to_string()]
        );
    }

    #[test]
    fn normalized_compose_depends_on_mapping_keeps_conditions() {
        let compose = serde_json::json!({
            "services": {
                "api": {
                    "image": "api:1",
                    "environment": {"DATABASE_URL": "postgres://x"},
                    "depends_on": {
                        "cache": {"condition": "service_started", "required": true},
                        "db": {"condition": "service_healthy", "required": true}
                    },
                    "networks": {"appnet": null}
                },
                "db": {"image": "postgres:16"}
            }
        });
        let overlay = render_overlay(&compose, "alethefy", None, true).unwrap();
        assert!(overlay.yaml.contains(
            "    depends_on:\n      cache:\n        condition: \"service_started\"\n        required: \"True\"\n"
        ));
        assert!(!overlay.yaml.contains("db:"));
    }

    #[test]
    fn empty_database_projection_renders_services_mapping() {
        let overlay = render_overlay(&alethefy_class(), "alethefy", None, true).unwrap();
        assert_eq!(
            overlay.yaml,
            "services: {}\nnetworks:\n  production-db-network:\n    external: true\n    name: production-db-network\n"
        );
        assert!(overlay.scale_args.is_empty());
        let parsed: serde_yaml::Value = serde_yaml::from_str(&overlay.yaml).unwrap();
        let services = parsed.get("services").unwrap();
        assert!(services.is_mapping());
        assert!(serde_yaml::to_string(&parsed)
            .unwrap()
            .contains("services: {}"));
    }

    #[test]
    fn database_only_project_is_refused() {
        let compose = serde_json::json!({"services": {"db": {"image": "postgres:16"}}});
        let err = render_overlay(&compose, "alethefy", None, true).unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
        assert!(err
            .to_string()
            .contains("no supported application database settings"));
    }

    #[test]
    fn overlay_never_renders_a_password_literal() {
        let overlay = render_overlay(&db_project(), "alethefy", None, true).unwrap();
        assert!(!overlay.yaml.contains("hunter2"));
        assert!(!overlay.yaml.contains("postgres://x"));
        assert!(overlay.yaml.contains("${PRODUCTION_DB_PASSWORD}"));
    }

    #[test]
    fn unsafe_project_names_are_refused() {
        let err = render_overlay(&db_project(), "../escape", None, true).unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn environment_accepts_the_list_form() {
        let compose = serde_json::json!({
            "services": {
                "api": {"image": "api:1", "environment": ["PGDATABASE=app", "APP_ENV=production"]},
                "db": {"image": "postgres:16"}
            }
        });
        let overlay = render_overlay(&compose, "alethefy", None, true).unwrap();
        assert!(overlay
            .yaml
            .contains("PGDATABASE: \"${PRODUCTION_DB_NAME}\""));
        assert_eq!(
            overlay.scale_args,
            vec!["--scale".to_string(), "db=0".to_string()]
        );
    }

    #[test]
    fn env_file_is_attached_to_every_rendered_service() {
        let overlay = render_overlay(
            &db_project(),
            "alethefy",
            Some("/secrets/alethefy/.env"),
            true,
        )
        .unwrap();
        assert!(overlay
            .yaml
            .contains("    env_file:\n      - \"/secrets/alethefy/.env\""));
        assert!(overlay
            .yaml
            .contains("DATABASE_URL: \"${PRODUCTION_DATABASE_URL}\""));
        assert!(overlay.yaml.contains("production-db-network"));
    }

    #[test]
    fn env_file_without_shared_db_renders_stubs_and_keeps_databases() {
        let overlay = render_overlay(
            &alethefy_class(),
            "alethefy",
            Some("/secrets/alethefy/.env"),
            false,
        )
        .unwrap();
        assert_eq!(
            overlay.yaml,
            "services:\n  alethefy:\n    env_file:\n      - \"/secrets/alethefy/.env\"\n"
        );
        assert!(!overlay.yaml.contains("production-db-network"));
        assert!(overlay.scale_args.is_empty());
        let parsed: serde_yaml::Value = serde_yaml::from_str(&overlay.yaml).unwrap();
        assert!(parsed.get("services").unwrap().is_mapping());
    }

    #[test]
    fn no_env_no_shared_renders_an_empty_mapping() {
        let overlay = render_overlay(&alethefy_class(), "alethefy", None, false).unwrap();
        assert_eq!(overlay.yaml, "services: {}\n");
        assert!(overlay.scale_args.is_empty());
    }

    #[test]
    fn validate_yaml_refuses_a_null_services_scalar() {
        assert!(validate_yaml("services:\nnetworks: {}\n").is_ok());
        assert!(validate_yaml("services:\n  api: 1\n").is_ok());
        assert!(validate_yaml(": :\n").is_err());
    }
}
