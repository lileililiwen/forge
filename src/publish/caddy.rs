//! Host routing (Caddyfile + navigation index) rendering
//! (`decoupled-remote-publish`).
//!
//! Rust port of the target-hosted `generate-caddyfile.py`. After the
//! per-project deploys of a run, Forge renders the same two documents
//! the sibling produced — `platform/Caddyfile` and
//! `platform/site/index.html` — from the target-hosted
//! `port-registry.json` and ships them before asking the router to
//! reload.
//!
//! ## Route selection
//!
//! A project receives a public host only when one of its published
//! ports belongs to a preferred HTTP service (`web`, `frontend`,
//! `caddy`, `gateway`, `app`, `rust-app`, `api`) and the container
//! target port is not one of the infrastructure ports the router
//! itself binds. Projects without such a port are listed in a
//! trailing comment and rendered as "no public route" on the
//! navigation page, so an unrouted project is visible rather than
//! silently dropped.
//!
//! The documents contain host names and container ports only. No
//! secret ever reaches this renderer: the port registry carries
//! published host ports, not environment values.

use std::collections::BTreeMap;

use crate::core::ForgeError;
use crate::publish::port_allocator::PortRegistry;

/// Service preference order for public HTTP routing.
pub const PREFERRED_SERVICES: [&str; 7] = [
    "web", "frontend", "caddy", "gateway", "app", "rust-app", "api",
];

/// Container ports the router must never route a project through.
pub const IGNORED_TARGETS: [u32; 14] = [
    22, 25, 53, 80, 443, 5432, 6379, 9000, 9001, 9090, 9093, 9100, 9187, 9121,
];

/// The route a project is published at: the service name and its
/// allocated host port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub service: String,
    pub port: u32,
}

/// Choose the public route for one registry record, or `None` when
/// the project exposes no routable service.
///
/// Preferred HTTP service names (`web`, `frontend`, …) win first so
/// multi-port projects route their UI. Otherwise the lowest
/// non-infrastructure port wins, so every project that publishes a
/// plausible HTTP port receives its `<project>.<domain>` host and the
/// Caddy projection matches the adapter subdomain projection.
/// Infrastructure targets (ssh, postgres, redis, metrics, …) never
/// route.
pub fn choose_route(record: &crate::publish::port_allocator::PortRecord) -> Option<Route> {
    let mut parsed: Vec<(&str, u32, u32)> = Vec::new();
    for (key, port) in &record.services {
        let Some((service, target_protocol)) = key.split_once(':') else {
            continue;
        };
        let Some(target_text) = target_protocol.split('/').next() else {
            continue;
        };
        let Ok(target) = target_text.parse::<u32>() else {
            continue;
        };
        if IGNORED_TARGETS.contains(&target) {
            continue;
        }
        parsed.push((service, target, *port));
    }
    // Preferred HTTP service first (by preference order, then name,
    // then port for stability).
    let mut preferred: Vec<(usize, &str, u32)> = Vec::new();
    for (service, _, port) in &parsed {
        if let Some(index) = PREFERRED_SERVICES.iter().position(|name| *name == *service) {
            preferred.push((index, service, *port));
        }
    }
    if let Some((_, service, port)) = preferred.into_iter().min_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(b.1))
            .then_with(|| a.2.cmp(&b.2))
    }) {
        return Some(Route {
            service: service.to_string(),
            port,
        });
    }
    // Fallback: the lowest routable port, so single-service projects
    // named after the product (e.g. `alethefy:8000`) still route.
    parsed
        .into_iter()
        .min_by(|a, b| a.2.cmp(&b.2).then_with(|| a.0.cmp(b.0)))
        .map(|(service, _, port)| Route {
            service: service.to_string(),
            port,
        })
}

/// Render `platform/Caddyfile` for the whole registry.
pub fn render_caddyfile(
    registry: &PortRegistry,
    domain: &str,
    nav_host: &str,
) -> Result<String, ForgeError> {
    let projects = registry_projects(registry)?;
    let mut lines: Vec<String> = vec![
        "{".to_string(),
        "\tauto_https off".to_string(),
        "}".to_string(),
        String::new(),
        format!("http://{nav_host}.{domain} {{"),
        "\troot * /srv".to_string(),
        "\tfile_server".to_string(),
        "}".to_string(),
        String::new(),
    ];
    let mut skipped: Vec<String> = Vec::new();
    for (project, route) in &projects {
        match route {
            Some(route) => {
                lines.push(format!("http://{project}.{domain} {{"));
                lines.push(format!(
                    "\treverse_proxy host.docker.internal:{}",
                    route.port
                ));
                lines.push("}".to_string());
                lines.push(String::new());
            }
            None => skipped.push(project.clone()),
        }
    }
    if !skipped.is_empty() {
        lines.push(format!(
            "# No public HTTP service detected for: {}",
            skipped.join(", ")
        ));
        lines.push(String::new());
    }
    lines.push(":80 {".to_string());
    lines.push("\trespond \"Unknown application hostname: {http.request.host}\" 404".to_string());
    lines.push("}".to_string());
    lines.push(String::new());
    Ok(lines.join("\n"))
}

/// Render the navigation page served at the `nav_host` site root.
pub fn render_index(registry: &PortRegistry, domain: &str) -> Result<String, ForgeError> {
    let mut cards: Vec<String> = Vec::new();
    for (project, route) in registry_projects(registry)? {
        let card = match route {
            Some(_) => format!(
                "<article class=\"card\"><h2><a href=\"{}\">{}</a></h2><p class=\"status\">Public route available</p></article>",
                escape_html(&format!("https://{project}.{domain}")),
                escape_html(&project)
            ),
            None => format!(
                "<article class=\"card unavailable\"><h2>{}</h2><p class=\"status\">No public route</p></article>",
                escape_html(&project)
            ),
        };
        cards.push(card);
    }
    let body = if cards.is_empty() {
        "<p class=\"empty\">No projects are registered yet.</p>".to_string()
    } else {
        cards.join("\n")
    };
    Ok(INDEX_TEMPLATE.replace("{{BODY}}", &body))
}

/// Every registry project, sorted by project id, with its public route
/// when it has one. One sorted pass keeps the Caddyfile host order and
/// the navigation card order identical.
fn registry_projects(registry: &PortRegistry) -> Result<Vec<(String, Option<Route>)>, ForgeError> {
    let mut projects: Vec<(String, Option<Route>)> = Vec::with_capacity(registry.projects.len());
    for (identity, record) in &registry.projects {
        let project = project_id(identity);
        if project.is_empty() {
            return Err(ForgeError::PublishInvalid {
                reason: format!("port registry identity `{identity}` has no project id"),
            });
        }
        projects.push((project, choose_route(record)));
    }
    projects.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(projects)
}

/// Strip the `local:` identity prefix the port allocator stores.
pub fn project_id(identity: &str) -> String {
    identity
        .strip_prefix("local:")
        .unwrap_or(identity)
        .to_string()
}

/// HTML-escape text for the navigation page. Quotes are escaped too
/// because every escaped value lands inside an attribute.
fn escape_html(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            other => out.push(other),
        }
    }
    out
}

/// Published routes as a lookup keyed by project id, for callers that
/// need to answer "does this project have a public host?".
pub fn public_routes(registry: &PortRegistry) -> BTreeMap<String, Route> {
    registry
        .projects
        .iter()
        .filter_map(|(identity, record)| {
            choose_route(record).map(|route| (project_id(identity), route))
        })
        .collect()
}

const INDEX_TEMPLATE: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Production applications</title>
  <style>
    :root { color-scheme: light dark; font-family: system-ui, sans-serif; }
    body { margin: 0; background: #f4f6f8; color: #17202a; }
    main { max-width: 960px; margin: 0 auto; padding: 48px 20px; }
    h1 { margin: 0 0 8px; }
    .intro { color: #52606d; margin: 0 0 28px; }
    .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 16px; }
    .card { background: white; border: 1px solid #d9e2ec; border-radius: 12px; padding: 20px; box-shadow: 0 2px 8px #102a430d; }
    .card h2 { font-size: 1.1rem; margin: 0 0 12px; overflow-wrap: anywhere; }
    a { color: #1769aa; }
    .status { color: #486581; font-size: .9rem; margin: 0; }
    .unavailable { opacity: .7; }
    @media (prefers-color-scheme: dark) { body { background: #111827; color: #e5e7eb; } .intro, .status { color: #aab4c3; } .card { background: #1f2937; border-color: #374151; } a { color: #93c5fd; } }
  </style>
</head>
<body>
  <main>
    <h1>Production applications</h1>
    <p class="intro">Open a project below. This list is generated from the production route registry.</p>
    <section class="grid" aria-label="Production applications">
      {{BODY}}
    </section>
  </main>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publish::port_allocator::PortRecord;

    fn fixture() -> PortRegistry {
        let mut registry = PortRegistry::default();
        for (identity, service_port) in [
            ("local:alpha", ("web:3000/tcp", 17700u32)),
            ("local:database-only", ("postgres:5432/tcp", 17701)),
            ("local:unsafe<&", ("api:8080/tcp", 17702)),
        ] {
            registry.projects.insert(
                identity.to_string(),
                PortRecord {
                    base: 17700,
                    services: BTreeMap::from([(service_port.0.to_string(), service_port.1)]),
                },
            );
        }
        registry
    }

    #[test]
    fn caddyfile_matches_the_sibling_bytes() {
        let rendered = render_caddyfile(&fixture(), "tooosall.uk", "apps").unwrap();
        assert_eq!(
            rendered,
            "{\n\tauto_https off\n}\n\nhttp://apps.tooosall.uk {\n\troot * /srv\n\tfile_server\n}\n\nhttp://alpha.tooosall.uk {\n\treverse_proxy host.docker.internal:17700\n}\n\nhttp://unsafe<&.tooosall.uk {\n\treverse_proxy host.docker.internal:17702\n}\n\n# No public HTTP service detected for: database-only\n\n:80 {\n\trespond \"Unknown application hostname: {http.request.host}\" 404\n}\n"
        );
    }

    #[test]
    fn caddyfile_lists_only_unrouted_projects_in_the_comment() {
        let mut registry = PortRegistry::default();
        registry.projects.insert(
            "local:dbonly".to_string(),
            PortRecord {
                base: 1,
                services: BTreeMap::from([("postgres:5432/tcp".to_string(), 1)]),
            },
        );
        let rendered = render_caddyfile(&registry, "tooosall.uk", "apps").unwrap();
        assert!(rendered.contains("# No public HTTP service detected for: dbonly"));
        assert!(!rendered.contains("http://dbonly.tooosall.uk"));
    }

    #[test]
    fn index_matches_the_sibling_escape_behaviour() {
        let page = render_index(&fixture(), "tooosall.uk").unwrap();
        assert!(page.contains("href=\"https://alpha.tooosall.uk\""));
        assert!(page.contains("href=\"https://unsafe&lt;&amp;.tooosall.uk\""));
        assert!(page.contains("database-only"));
        assert!(page.contains("No public route"));
        assert!(!page.contains("17700"));
    }

    #[test]
    fn index_reports_an_empty_registry() {
        let page = render_index(&PortRegistry::default(), "tooosall.uk").unwrap();
        assert!(page.contains("No projects are registered yet."));
    }

    #[test]
    fn route_selection_prefers_web_over_api() {
        let record = PortRecord {
            base: 1,
            services: BTreeMap::from([
                ("api:3000/tcp".to_string(), 15001),
                ("web:3000/tcp".to_string(), 15002),
            ]),
        };
        let route = choose_route(&record).unwrap();
        assert_eq!(route.service, "web");
        assert_eq!(route.port, 15002);
    }

    #[test]
    fn route_selection_ignores_infrastructure_targets() {
        let record = PortRecord {
            base: 1,
            services: BTreeMap::from([("web:5432/tcp".to_string(), 15001)]),
        };
        assert!(choose_route(&record).is_none());
    }

    #[test]
    fn route_selection_falls_back_to_a_product_named_service() {
        let record = PortRecord {
            base: 12540,
            services: BTreeMap::from([("alethefy:8000/tcp".to_string(), 12540)]),
        };
        let route = choose_route(&record).unwrap();
        assert_eq!(route.service, "alethefy");
        assert_eq!(route.port, 12540);
    }

    #[test]
    fn route_selection_fallback_prefers_the_lowest_routable_port() {
        let record = PortRecord {
            base: 1,
            services: BTreeMap::from([
                ("worker:9000/tcp".to_string(), 15002),
                ("backend:8080/tcp".to_string(), 15001),
            ]),
        };
        // 9000 is an ignored infrastructure target, so backend wins.
        let route = choose_route(&record).unwrap();
        assert_eq!(route.service, "backend");
        assert_eq!(route.port, 15001);
    }

    #[test]
    fn route_selection_skips_malformed_registry_keys() {
        let record = PortRecord {
            base: 1,
            services: BTreeMap::from([("webport".to_string(), 15001)]),
        };
        assert!(choose_route(&record).is_none());
    }

    #[test]
    fn public_routes_answers_per_project_lookup() {
        let routes = public_routes(&fixture());
        assert_eq!(routes.get("alpha").map(|r| r.port), Some(17700));
        assert!(!routes.contains_key("database-only"));
    }

    #[test]
    fn project_id_strips_the_local_prefix_only() {
        assert_eq!(project_id("local:alethefy"), "alethefy");
        assert_eq!(
            project_id("github.com/example/app"),
            "github.com/example/app"
        );
    }
}
