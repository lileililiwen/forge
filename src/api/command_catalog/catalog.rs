//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde_json::json;
use std::collections::BTreeSet;
use std::sync::OnceLock;

use super::builder::CatalogBuilder;
use super::model::CommandRow;
use super::routes::{ALL_CATEGORIES, CONTRACT_VERSION, IMPLEMENTED_WEB_ROUTES};

/// The complete catalog, built once from the static rows above.
pub fn rows() -> &'static [CommandRow] {
    static CATALOG: OnceLock<Vec<CommandRow>> = OnceLock::new();
    CATALOG.get_or_init(|| CatalogBuilder::new().build())
}

/// Catalog integrity problems; empty means the catalog satisfies every
/// invariant design.md requires (unique ids, valid parents, reason/state
/// rules, resolvable web routes, complete category coverage).
pub fn problems() -> Vec<String> {
    let mut issues = Vec::new();
    let mut seen = BTreeSet::new();
    let ids: BTreeSet<&str> = rows().iter().map(|row| row.id.as_str()).collect();
    for row in rows() {
        if !seen.insert(row.id.as_str()) {
            issues.push(format!("duplicate catalog id `{}`", row.id));
        }
        if let Some(parent) = &row.parent_id {
            if !ids.contains(parent.as_str()) {
                issues.push(format!(
                    "row `{}` references unknown parent `{}`",
                    row.id, parent
                ));
            }
        } else if row.id.contains('.') {
            issues.push(format!("row `{}` has a dotted id but no parent_id", row.id));
        }
        if row.summary.is_empty() {
            issues.push(format!("row `{}` has an empty summary", row.id));
        }
        match row.availability {
            "web" => {
                if row.reason.is_some() {
                    issues.push(format!("web row `{}` must not carry a reason", row.id));
                }
                match row.route {
                    Some(route) if IMPLEMENTED_WEB_ROUTES.contains(&route) => {}
                    Some(route) => issues.push(format!(
                        "web row `{}` names unimplemented route `{}`",
                        row.id, route
                    )),
                    None => issues.push(format!("web row `{}` has no route", row.id)),
                }
            }
            "disabled"
            | "cli_only"
            | "provider_required"
            | "project_capability_required"
            | "not_yet_web" => {
                if row.reason.is_none_or(|reason| reason.is_empty()) {
                    issues.push(format!(
                        "non-web row `{}` (state {}) must carry a plain-language reason",
                        row.id, row.availability
                    ));
                }
                if row.route.is_some() {
                    issues.push(format!("non-web row `{}` must not name a route", row.id));
                }
            }
            other => issues.push(format!(
                "row `{}` has unknown availability `{}`",
                row.id, other
            )),
        }
        match row.risk {
            "read" | "local_write" | "remote_write" | "session_admin" => {}
            other => issues.push(format!("row `{}` has unknown risk `{}`", row.id, other)),
        }
        match row.scope {
            "workspace" | "project" | "forge" | "profile" | "provider" => {}
            other => issues.push(format!("row `{}` has unknown scope `{}`", row.id, other)),
        }
        if !ALL_CATEGORIES.iter().any(|cat| cat.id() == row.category) {
            issues.push(format!("row `{}` has unknown category", row.id));
        }
        if let Some(prefix) = row.cli_invocation.strip_prefix("forge ") {
            if prefix.replace(' ', ".") != row.id {
                issues.push(format!(
                    "row `{}` has an inconsistent cli_invocation",
                    row.id
                ));
            }
        } else {
            issues.push(format!("row `{}` has a malformed cli_invocation", row.id));
        }
    }
    let used: BTreeSet<&str> = rows().iter().map(|row| row.category).collect();
    for cat in ALL_CATEGORIES {
        if !used.contains(cat.id()) {
            issues.push(format!("category `{}` is never used", cat.id()));
        }
    }
    issues
}

/// The serialized `categories` list for the API envelope.
fn categories_json() -> serde_json::Value {
    let rows = rows();
    let value: Vec<_> = ALL_CATEGORIES
        .iter()
        .map(|cat| {
            json!({
                "id": cat.id(),
                "label": cat.label(),
                "count": rows.iter().filter(|row| row.category == cat.id()).count(),
            })
        })
        .collect();
    json!(value)
}

/// The `GET /v1/admin/commands` body: `{commands, categories, contract}`.
pub fn envelope() -> serde_json::Value {
    json!({
        "contract": CONTRACT_VERSION,
        "categories": categories_json(),
        "commands": rows(),
    })
}

#[cfg(test)]
pub(super) mod tests {
    use std::collections::BTreeMap;

    use super::super::routes::{
        WEB_ROUTE_ADMIN_AGENT_LIST, WEB_ROUTE_ADMIN_AGENT_STATUS,
        WEB_ROUTE_ADMIN_ANALYTICS_METRICS, WEB_ROUTE_ADMIN_CATALOG, WEB_ROUTE_ADMIN_CATALOG_GAPS,
        WEB_ROUTE_ADMIN_CATALOG_INSPECT, WEB_ROUTE_ADMIN_CATALOG_LANGUAGES,
        WEB_ROUTE_ADMIN_CATALOG_TAGS, WEB_ROUTE_ADMIN_CLASSIFY_LIST, WEB_ROUTE_ADMIN_CLASSIFY_SHOW,
        WEB_ROUTE_ADMIN_CONTRACTS, WEB_ROUTE_ADMIN_CONTRACT_EMIT, WEB_ROUTE_ADMIN_CONTRACT_INSPECT,
        WEB_ROUTE_ADMIN_CREATION_COMPONENT, WEB_ROUTE_ADMIN_CREATION_COMPONENTS,
        WEB_ROUTE_ADMIN_CREATION_COMPONENT_RESOLVE, WEB_ROUTE_ADMIN_CREATION_FEATURE,
        WEB_ROUTE_ADMIN_CREATION_FEATURES, WEB_ROUTE_ADMIN_CREATION_FEATURE_RESOLVE,
        WEB_ROUTE_ADMIN_CREATION_INTENT_PLANS, WEB_ROUTE_ADMIN_CREATION_INTENT_VALIDATE,
        WEB_ROUTE_ADMIN_CREATION_PROCEDURE, WEB_ROUTE_ADMIN_CREATION_PROCEDURES,
        WEB_ROUTE_ADMIN_CREATION_PROFILE, WEB_ROUTE_ADMIN_CREATION_PROFILES,
        WEB_ROUTE_ADMIN_CREATION_PROFILE_RESOLVE, WEB_ROUTE_ADMIN_CREATION_STANDARD,
        WEB_ROUTE_ADMIN_CREATION_STANDARDS, WEB_ROUTE_ADMIN_CREATION_STANDARD_CHECK,
        WEB_ROUTE_ADMIN_CREATION_STANDARD_DIFF, WEB_ROUTE_ADMIN_CREATION_UI_PATTERN,
        WEB_ROUTE_ADMIN_CREATION_UI_PATTERNS, WEB_ROUTE_ADMIN_CREATION_UI_PATTERN_RESOLVE,
        WEB_ROUTE_ADMIN_DELIVERY_HERMORA_RETRY, WEB_ROUTE_ADMIN_DELIVERY_PREFLIGHT,
        WEB_ROUTE_ADMIN_DELIVERY_PROMOTE, WEB_ROUTE_ADMIN_DELIVERY_STAGE,
        WEB_ROUTE_ADMIN_DELIVERY_STATUS, WEB_ROUTE_ADMIN_DEPLOY, WEB_ROUTE_ADMIN_DEPLOY_HISTORY,
        WEB_ROUTE_ADMIN_DEPLOY_INSPECT, WEB_ROUTE_ADMIN_DEPLOY_PLAN, WEB_ROUTE_ADMIN_DEPLOY_STATUS,
        WEB_ROUTE_ADMIN_DESCRIBE_LIST, WEB_ROUTE_ADMIN_DESCRIBE_SHOW,
        WEB_ROUTE_ADMIN_EVIDENCE_MATRIX, WEB_ROUTE_ADMIN_EVIDENCE_PROVIDER_INSPECT,
        WEB_ROUTE_ADMIN_FEATURE, WEB_ROUTE_ADMIN_FEATURE_REMOVE, WEB_ROUTE_ADMIN_FEATURE_UPGRADE,
        WEB_ROUTE_ADMIN_FLEET_INSPECT, WEB_ROUTE_ADMIN_GOVERNANCE_INSPECT,
        WEB_ROUTE_ADMIN_GOVERNANCE_LIST, WEB_ROUTE_ADMIN_GOVERNANCE_STATUS,
        WEB_ROUTE_ADMIN_GRADUATION_IMPORT, WEB_ROUTE_ADMIN_GRADUATION_PREVIEW,
        WEB_ROUTE_ADMIN_IDENTITY_CONFIG, WEB_ROUTE_ADMIN_IDENTITY_SESSIONS,
        WEB_ROUTE_ADMIN_IDENTITY_SESSION_INSPECT, WEB_ROUTE_ADMIN_INTENT_APPLY,
        WEB_ROUTE_ADMIN_INTENT_RESOLVE, WEB_ROUTE_ADMIN_PORTFOLIO_EVIDENCE_IMPORT,
        WEB_ROUTE_ADMIN_PORTFOLIO_GOALS, WEB_ROUTE_ADMIN_PORTFOLIO_PROJECT,
        WEB_ROUTE_ADMIN_PORTFOLIO_READ_EVIDENCE, WEB_ROUTE_ADMIN_PORTFOLIO_READ_GOALS,
        WEB_ROUTE_ADMIN_PORTFOLIO_READ_RELATIONS, WEB_ROUTE_ADMIN_PORTFOLIO_READ_REVIEWS,
        WEB_ROUTE_ADMIN_PORTFOLIO_READ_TAGS, WEB_ROUTE_ADMIN_PORTFOLIO_RELATIONS,
        WEB_ROUTE_ADMIN_PORTFOLIO_RELATION_REMOVE, WEB_ROUTE_ADMIN_PORTFOLIO_REVIEWS,
        WEB_ROUTE_ADMIN_PORTFOLIO_TAGS, WEB_ROUTE_ADMIN_PORTFOLIO_TAG_REMOVE,
        WEB_ROUTE_ADMIN_PROJECT_IMPORT, WEB_ROUTE_ADMIN_PROJECT_NEW,
        WEB_ROUTE_ADMIN_PROJECT_REGISTER, WEB_ROUTE_ADMIN_PUBLISH, WEB_ROUTE_ADMIN_PUBLISH_PLAN,
        WEB_ROUTE_ADMIN_PUBLISH_PROVIDERS, WEB_ROUTE_ADMIN_PUBLISH_PROVIDER_INSPECT,
        WEB_ROUTE_ADMIN_RELEASE, WEB_ROUTE_ADMIN_RELEASE_HISTORY, WEB_ROUTE_ADMIN_RELEASE_INSPECT,
        WEB_ROUTE_ADMIN_RELEASE_PLAN, WEB_ROUTE_ADMIN_REMEDIATE_APPLY,
        WEB_ROUTE_ADMIN_REMEDIATE_DIFF, WEB_ROUTE_ADMIN_REMEDIATE_PLAN,
        WEB_ROUTE_ADMIN_REMEDIATE_SCAN, WEB_ROUTE_ADMIN_SHIPPING_PLUGINS, WEB_ROUTE_ADMIN_SPEC,
        WEB_ROUTE_ADMIN_SPECS, WEB_ROUTE_ADMIN_SPEC_APPLY, WEB_ROUTE_ADMIN_SPEC_INSPECT,
        WEB_ROUTE_ADMIN_SPEC_ROUTE, WEB_ROUTE_ADMIN_STUDIO_PREVIEW, WEB_ROUTE_ADMIN_STUDIO_REFINE,
        WEB_ROUTE_ADMIN_STUDIO_SPEC_SAVE, WEB_ROUTE_CLASSIFY_APPLY, WEB_ROUTE_CLASSIFY_APPROVE,
        WEB_ROUTE_CLASSIFY_REJECT, WEB_ROUTE_DELIVERY_ALLOWLIST,
        WEB_ROUTE_DELIVERY_ALLOWLIST_REMOVE, WEB_ROUTE_DELIVERY_APPROVE,
        WEB_ROUTE_DELIVERY_OVERVIEW, WEB_ROUTE_DELIVERY_PREVIEW, WEB_ROUTE_DELIVERY_PUBLISH,
        WEB_ROUTE_DELIVERY_RECONCILE, WEB_ROUTE_FLEET_STATUS, WEB_ROUTE_PROJECTS,
        WEB_ROUTE_PROJECT_DETAIL, WEB_ROUTE_PROJECT_PLAN, WEB_ROUTE_PROJECT_STATUS,
    };
    use super::*;

    #[test]
    fn catalog_has_no_integrity_problems() {
        let issues = problems();
        assert!(issues.is_empty(), "catalog problems: {issues:?}");
    }

    #[test]
    fn catalog_covers_every_clap_path() {
        // The row count equals the 233 Clap paths (probe-verified from
        // `Cli::command()`, including the `identity change-password` and
        // `identity generate-password` leaf commands, the `workspace` /
        // `workspace.sync` bulk-convergence paths and the `plugins` /
        // `plugins.list` registry paths, plus the `classify.derive` and
        // `classify.apply` paths, plus the new `identity.status` path) plus
        // the explicit top-level `help` row.
        assert_eq!(rows().len(), 234);
        assert!(rows().iter().any(|row| row.id == "help"));
    }

    #[test]
    fn web_rows_only_point_at_implemented_routes() {
        for row in rows().iter().filter(|row| row.availability == "web") {
            let route = row.route.expect("web row must name a route");
            assert!(
                IMPLEMENTED_WEB_ROUTES.contains(&route),
                "row {} points at unimplemented route {route}",
                row.id
            );
        }
        let web: Vec<(&str, &str)> = rows()
            .iter()
            .filter(|row| row.availability == "web")
            .map(|row| (row.id.as_str(), row.route.expect("web row route")))
            .collect();
        assert_eq!(
            web,
            vec![
                ("list", WEB_ROUTE_PROJECTS),
                ("inspect", WEB_ROUTE_PROJECT_DETAIL),
                ("register", WEB_ROUTE_ADMIN_PROJECT_REGISTER),
                ("import", WEB_ROUTE_ADMIN_PROJECT_IMPORT),
                ("graduation.preview", WEB_ROUTE_ADMIN_GRADUATION_PREVIEW),
                ("graduation.import", WEB_ROUTE_ADMIN_GRADUATION_IMPORT),
                ("new", WEB_ROUTE_ADMIN_PROJECT_NEW),
                ("profile.list", WEB_ROUTE_ADMIN_CREATION_PROFILES),
                ("profile.inspect", WEB_ROUTE_ADMIN_CREATION_PROFILE),
                ("profile.resolve", WEB_ROUTE_ADMIN_CREATION_PROFILE_RESOLVE),
                ("feature.list", WEB_ROUTE_ADMIN_CREATION_FEATURES),
                ("feature.inspect", WEB_ROUTE_ADMIN_CREATION_FEATURE),
                ("feature.resolve", WEB_ROUTE_ADMIN_CREATION_FEATURE_RESOLVE),
                ("feature.add", WEB_ROUTE_ADMIN_FEATURE),
                ("feature.remove", WEB_ROUTE_ADMIN_FEATURE_REMOVE),
                ("feature.upgrade", WEB_ROUTE_ADMIN_FEATURE_UPGRADE),
                ("upgrade", WEB_ROUTE_PROJECT_PLAN),
                ("component.list", WEB_ROUTE_ADMIN_CREATION_COMPONENTS),
                ("component.inspect", WEB_ROUTE_ADMIN_CREATION_COMPONENT),
                (
                    "component.resolve",
                    WEB_ROUTE_ADMIN_CREATION_COMPONENT_RESOLVE
                ),
                ("ui-pattern.list", WEB_ROUTE_ADMIN_CREATION_UI_PATTERNS),
                ("ui-pattern.inspect", WEB_ROUTE_ADMIN_CREATION_UI_PATTERN),
                (
                    "ui-pattern.resolve",
                    WEB_ROUTE_ADMIN_CREATION_UI_PATTERN_RESOLVE
                ),
                ("intent.validate", WEB_ROUTE_ADMIN_CREATION_INTENT_VALIDATE),
                ("intent.resolve", WEB_ROUTE_ADMIN_INTENT_RESOLVE),
                ("intent.apply", WEB_ROUTE_ADMIN_INTENT_APPLY),
                ("intent.list", WEB_ROUTE_ADMIN_CREATION_INTENT_PLANS),
                ("procedure.list", WEB_ROUTE_ADMIN_CREATION_PROCEDURES),
                ("procedure.inspect", WEB_ROUTE_ADMIN_CREATION_PROCEDURE),
                ("standard.list", WEB_ROUTE_ADMIN_CREATION_STANDARDS),
                ("standard.inspect", WEB_ROUTE_ADMIN_CREATION_STANDARD),
                ("standard.check", WEB_ROUTE_ADMIN_CREATION_STANDARD_CHECK),
                ("standard.diff", WEB_ROUTE_ADMIN_CREATION_STANDARD_DIFF),
                ("doctor", WEB_ROUTE_PROJECT_DETAIL),
                ("check", WEB_ROUTE_PROJECT_STATUS),
                ("spec.generate", WEB_ROUTE_ADMIN_SPEC),
                ("spec.list", WEB_ROUTE_ADMIN_SPECS),
                ("spec.inspect", WEB_ROUTE_ADMIN_SPEC_INSPECT),
                ("spec.route", WEB_ROUTE_ADMIN_SPEC_ROUTE),
                ("spec.apply", WEB_ROUTE_ADMIN_SPEC_APPLY),
                ("remediate.scan", WEB_ROUTE_ADMIN_REMEDIATE_SCAN),
                ("remediate.plan", WEB_ROUTE_ADMIN_REMEDIATE_PLAN),
                ("remediate.diff", WEB_ROUTE_ADMIN_REMEDIATE_DIFF),
                ("remediate.apply", WEB_ROUTE_ADMIN_REMEDIATE_APPLY),
                ("describe.list", WEB_ROUTE_ADMIN_DESCRIBE_LIST),
                ("describe.show", WEB_ROUTE_ADMIN_DESCRIBE_SHOW),
                ("classify.apply", WEB_ROUTE_CLASSIFY_APPLY),
                ("classify.list", WEB_ROUTE_ADMIN_CLASSIFY_LIST),
                ("classify.show", WEB_ROUTE_ADMIN_CLASSIFY_SHOW),
                ("classify.approve", WEB_ROUTE_CLASSIFY_APPROVE),
                ("classify.reject", WEB_ROUTE_CLASSIFY_REJECT),
                ("contract.list", WEB_ROUTE_ADMIN_CONTRACTS),
                ("contract.inspect", WEB_ROUTE_ADMIN_CONTRACT_INSPECT),
                ("contract.emit", WEB_ROUTE_ADMIN_CONTRACT_EMIT),
                ("governance.list", WEB_ROUTE_ADMIN_GOVERNANCE_LIST),
                ("governance.status", WEB_ROUTE_ADMIN_GOVERNANCE_STATUS),
                ("governance.inspect", WEB_ROUTE_ADMIN_GOVERNANCE_INSPECT),
                ("release.prepare", WEB_ROUTE_ADMIN_RELEASE_PLAN),
                ("release.apply", WEB_ROUTE_ADMIN_RELEASE),
                ("release.list", WEB_ROUTE_ADMIN_RELEASE_HISTORY),
                ("release.inspect", WEB_ROUTE_ADMIN_RELEASE_INSPECT),
                ("deploy.plan", WEB_ROUTE_ADMIN_DEPLOY_PLAN),
                ("deploy.apply", WEB_ROUTE_ADMIN_DEPLOY),
                ("deploy.list", WEB_ROUTE_ADMIN_DEPLOY_HISTORY),
                ("deploy.inspect", WEB_ROUTE_ADMIN_DEPLOY_INSPECT),
                ("deploy.status", WEB_ROUTE_ADMIN_DEPLOY_STATUS),
                ("publish", WEB_ROUTE_ADMIN_PUBLISH),
                ("publish.provider.list", WEB_ROUTE_ADMIN_PUBLISH_PROVIDERS),
                (
                    "publish.provider.inspect",
                    WEB_ROUTE_ADMIN_PUBLISH_PROVIDER_INSPECT
                ),
                ("provider.matrix", WEB_ROUTE_ADMIN_EVIDENCE_MATRIX),
                (
                    "provider.inspect",
                    WEB_ROUTE_ADMIN_EVIDENCE_PROVIDER_INSPECT
                ),
                ("plugins.list", WEB_ROUTE_ADMIN_SHIPPING_PLUGINS),
                ("fleet.list", WEB_ROUTE_PROJECTS),
                ("fleet.status", WEB_ROUTE_FLEET_STATUS),
                ("fleet.inspect", WEB_ROUTE_ADMIN_FLEET_INSPECT),
                ("project.list", WEB_ROUTE_ADMIN_CATALOG),
                ("project.inspect", WEB_ROUTE_ADMIN_CATALOG_INSPECT),
                ("project.tags", WEB_ROUTE_ADMIN_CATALOG_TAGS),
                ("project.languages", WEB_ROUTE_ADMIN_CATALOG_LANGUAGES),
                ("project.gaps", WEB_ROUTE_ADMIN_CATALOG_GAPS),
                ("inventory.show", WEB_ROUTE_PROJECTS),
                ("portfolio.tag.add", WEB_ROUTE_ADMIN_PORTFOLIO_TAGS),
                ("portfolio.tag.remove", WEB_ROUTE_ADMIN_PORTFOLIO_TAG_REMOVE),
                ("portfolio.tag.list", WEB_ROUTE_ADMIN_PORTFOLIO_READ_TAGS),
                (
                    "portfolio.relation.add",
                    WEB_ROUTE_ADMIN_PORTFOLIO_RELATIONS
                ),
                (
                    "portfolio.relation.remove",
                    WEB_ROUTE_ADMIN_PORTFOLIO_RELATION_REMOVE
                ),
                (
                    "portfolio.relation.list",
                    WEB_ROUTE_ADMIN_PORTFOLIO_READ_RELATIONS
                ),
                ("portfolio.review.set", WEB_ROUTE_ADMIN_PORTFOLIO_REVIEWS),
                (
                    "portfolio.review.list",
                    WEB_ROUTE_ADMIN_PORTFOLIO_READ_REVIEWS
                ),
                ("portfolio.goal.add", WEB_ROUTE_ADMIN_PORTFOLIO_GOALS),
                ("portfolio.goal.link", WEB_ROUTE_ADMIN_PORTFOLIO_GOALS),
                ("portfolio.goal.list", WEB_ROUTE_ADMIN_PORTFOLIO_READ_GOALS),
                (
                    "portfolio.evidence.import",
                    WEB_ROUTE_ADMIN_PORTFOLIO_EVIDENCE_IMPORT
                ),
                (
                    "portfolio.evidence.list",
                    WEB_ROUTE_ADMIN_PORTFOLIO_READ_EVIDENCE
                ),
                ("portfolio.show", WEB_ROUTE_ADMIN_PORTFOLIO_PROJECT),
                ("portfolio.share.set", WEB_ROUTE_DELIVERY_ALLOWLIST),
                (
                    "portfolio.share.remove",
                    WEB_ROUTE_DELIVERY_ALLOWLIST_REMOVE
                ),
                ("portfolio.share.show", WEB_ROUTE_DELIVERY_OVERVIEW),
                ("portfolio.share.list", WEB_ROUTE_DELIVERY_OVERVIEW),
                ("portfolio.share.preview", WEB_ROUTE_DELIVERY_PREVIEW),
                ("portfolio.share.approve", WEB_ROUTE_DELIVERY_APPROVE),
                ("portfolio.share.publish", WEB_ROUTE_DELIVERY_PUBLISH),
                ("portfolio.share.reconcile", WEB_ROUTE_DELIVERY_RECONCILE),
                ("portfolio.share.audit", WEB_ROUTE_DELIVERY_OVERVIEW),
                ("analytics.metrics", WEB_ROUTE_ADMIN_ANALYTICS_METRICS),
                ("delivery.status", WEB_ROUTE_ADMIN_DELIVERY_STATUS),
                ("delivery.preflight", WEB_ROUTE_ADMIN_DELIVERY_PREFLIGHT),
                ("delivery.stage", WEB_ROUTE_ADMIN_DELIVERY_STAGE),
                ("delivery.promote", WEB_ROUTE_ADMIN_DELIVERY_PROMOTE),
                (
                    "delivery.hermora-retry",
                    WEB_ROUTE_ADMIN_DELIVERY_HERMORA_RETRY
                ),
                ("studio.spec", WEB_ROUTE_ADMIN_STUDIO_SPEC_SAVE),
                ("studio.preview", WEB_ROUTE_ADMIN_STUDIO_PREVIEW),
                ("studio.refine", WEB_ROUTE_ADMIN_STUDIO_REFINE),
                ("identity.validate-config", WEB_ROUTE_ADMIN_IDENTITY_CONFIG),
                ("identity.session-list", WEB_ROUTE_ADMIN_IDENTITY_SESSIONS),
                (
                    "identity.session-inspect",
                    WEB_ROUTE_ADMIN_IDENTITY_SESSION_INSPECT
                ),
                ("agent.status", WEB_ROUTE_ADMIN_AGENT_STATUS),
                ("agent.list", WEB_ROUTE_ADMIN_AGENT_LIST),
            ]
        );
    }

    #[test]
    fn publish_routes_are_implemented_web_routes() {
        // Both publish routes are registered by the router and listed as
        // implemented web routes, so the executable `publish` row (which points
        // at the apply route) can never name a route that does not exist and
        // the read-only plan route stays a first-class implemented endpoint.
        assert!(
            IMPLEMENTED_WEB_ROUTES.contains(&WEB_ROUTE_ADMIN_PUBLISH_PLAN),
            "publish plan route must be implemented"
        );
        assert!(
            IMPLEMENTED_WEB_ROUTES.contains(&WEB_ROUTE_ADMIN_PUBLISH),
            "publish apply route must be implemented"
        );
    }

    #[test]
    fn delivery_routes_are_implemented_web_routes() {
        // Every staged delivery route is registered by the router and listed
        // as an implemented web route, so the recatalogued delivery rows can
        // never name a route that does not exist.
        for route in [
            WEB_ROUTE_ADMIN_DELIVERY_STATUS,
            WEB_ROUTE_ADMIN_DELIVERY_PREFLIGHT,
            WEB_ROUTE_ADMIN_DELIVERY_STAGE,
            WEB_ROUTE_ADMIN_DELIVERY_PROMOTE,
            WEB_ROUTE_ADMIN_DELIVERY_HERMORA_RETRY,
        ] {
            assert!(
                IMPLEMENTED_WEB_ROUTES.contains(&route),
                "delivery route {route} must be implemented"
            );
        }
    }

    #[test]
    fn web_execution_rows_are_well_formed_and_point_at_implemented_routes() {
        // Layer C: the catalog's `execution` block is the browser's runnable
        // contract. The project creation/registration rows plus the project
        // feature/spec/deploy lifecycle write rows carry one; every one must be a
        // `web` row resolving to a real implemented route with a well-formed typed
        // parameter list, and no non-`web` row may ever carry one.
        let executable_ids: Vec<&str> = rows()
            .iter()
            .filter(|row| row.execution.is_some())
            .map(|row| row.id.as_str())
            .collect();
        assert_eq!(
            executable_ids,
            vec![
                "register",
                "import",
                "graduation.preview",
                "graduation.import",
                "new",
                "feature.add",
                "feature.remove",
                "feature.upgrade",
                "intent.resolve",
                "intent.apply",
                "spec.generate",
                "spec.apply",
                "remediate.plan",
                "remediate.apply",
                "classify.apply",
                "classify.approve",
                "classify.reject",
                "release.apply",
                "deploy.apply",
                "publish",
                "delivery.preflight",
                "delivery.stage",
                "delivery.promote",
                "delivery.hermora-retry",
                "studio.spec",
                "studio.refine",
            ]
        );
        // Each executable row's typed parameter list must match the mandatory
        // fields that route's `authoring_descriptor` gate accepts, in the exact
        // order, so the browser's generated controls are the route's real input
        // surface and nothing more.
        let expected: BTreeMap<&str, Vec<(&str, &str, bool)>> = BTreeMap::from([
            ("register", vec![("project", "string", true)]),
            (
                "import",
                vec![
                    ("project", "string", true),
                    ("profile", "string", false),
                    ("id", "string", false),
                ],
            ),
            (
                "new",
                vec![
                    ("project", "string", true),
                    ("profile", "string", true),
                    ("name", "string", false),
                    ("features", "string_array", false),
                ],
            ),
            (
                "graduation.preview",
                vec![
                    ("artifact_json", "string", true),
                    ("profile", "string", true),
                    ("id", "string", false),
                ],
            ),
            (
                "graduation.import",
                vec![
                    ("artifact_json", "string", true),
                    ("profile", "string", true),
                    ("id", "string", false),
                ],
            ),
            (
                "intent.resolve",
                vec![
                    ("action", "string", true),
                    ("profile", "string", false),
                    ("required", "string_array", false),
                    ("forbidden", "string_array", false),
                    ("constraints", "string_array", false),
                ],
            ),
            (
                "intent.apply",
                vec![
                    ("action", "string", true),
                    ("profile", "string", false),
                    ("required", "string_array", false),
                    ("forbidden", "string_array", false),
                    ("constraints", "string_array", false),
                ],
            ),
            (
                "feature.add",
                vec![("feature", "string", true), ("version", "string", false)],
            ),
            ("feature.remove", vec![("feature", "string", true)]),
            (
                "feature.upgrade",
                vec![("feature", "string", true), ("version", "string", false)],
            ),
            (
                "spec.generate",
                vec![
                    ("findings", "string_array", true),
                    ("reason", "string", false),
                ],
            ),
            (
                "spec.apply",
                vec![
                    ("findings", "string_array", true),
                    ("reason", "string", false),
                ],
            ),
            (
                "remediate.plan",
                vec![("finding", "string", true), ("pack", "string", false)],
            ),
            (
                "remediate.apply",
                vec![("finding", "string", true), ("pack", "string", false)],
            ),
            // The classify apply resolves the approved set server-side
            // from the project's recorded proposals, so it has no
            // browser-supplied typed parameters; approve/reject each
            // carry only the proposal id (`<kind>-<hash>`).
            ("classify.apply", vec![]),
            ("classify.approve", vec![("proposal", "string", true)]),
            ("classify.reject", vec![("proposal", "string", true)]),
            ("release.apply", vec![("version", "string", true)]),
            ("deploy.apply", vec![("target", "string", false)]),
            // The bare publish resolves the provider, project and revision
            // server-side, so it has no browser-supplied typed parameters.
            ("publish", vec![]),
            // Delivery preflight likewise resolves every input server-side.
            ("delivery.preflight", vec![]),
            (
                "delivery.stage",
                vec![("confirm_operation_id", "string", true)],
            ),
            (
                "delivery.promote",
                vec![("confirm_revision", "string", true)],
            ),
            (
                "delivery.hermora-retry",
                vec![
                    ("deployment_url", "string", true),
                    ("secret_ref", "string", true),
                ],
            ),
            (
                "studio.spec",
                vec![
                    ("spec", "string", true),
                    ("expected_revision", "string", false),
                ],
            ),
            (
                "studio.refine",
                vec![
                    ("request", "string", true),
                    ("expected_revision", "string", true),
                    ("selected_files", "string_array", false),
                ],
            ),
        ]);
        for row in rows() {
            match &row.execution {
                Some(execution) => {
                    assert_eq!(
                        row.availability, "web",
                        "execution row {} must be web",
                        row.id
                    );
                    assert!(
                        IMPLEMENTED_WEB_ROUTES.contains(&execution.route),
                        "execution row {} names unimplemented route {}",
                        row.id,
                        execution.route
                    );
                    assert_eq!(
                        execution.route,
                        row.route.expect("web row route"),
                        "execution row {} route disagrees with row route",
                        row.id
                    );
                    assert_eq!(execution.method, "POST", "row {}", row.id);
                    assert!(execution.confirm_required, "row {}", row.id);
                    assert!(execution.digest_bound, "row {}", row.id);
                    assert!(
                        !execution.parameters.is_empty()
                            || matches!(
                                row.id.as_str(),
                                "publish" | "delivery.preflight" | "classify.apply"
                            ),
                        "row {} must carry typed parameters unless every input is server-resolved",
                        row.id
                    );
                    // Pin the exact typed parameter list against the route.
                    let expected_params = expected
                        .get(row.id.as_str())
                        .unwrap_or_else(|| panic!("unexpected execution row {}", row.id));
                    let actual: Vec<(&str, &str, bool)> = execution
                        .parameters
                        .iter()
                        .map(|p| (p.name.as_str(), p.kind, p.required))
                        .collect();
                    assert_eq!(
                        &actual, expected_params,
                        "row {} parameters disagree with the route's typed fields",
                        row.id
                    );
                    for param in &execution.parameters {
                        assert!(!param.name.is_empty(), "row {} empty param", row.id);
                        assert!(
                            matches!(param.kind, "string" | "string_array" | "boolean"),
                            "row {} parameter {} has unknown kind {}",
                            row.id,
                            param.name,
                            param.kind
                        );
                    }
                }
                None => {
                    assert!(
                        row.availability == "web" || row.execution.is_none(),
                        "non-web row {} must not carry execution",
                        row.id
                    );
                }
            }
        }
        // The execution block is purely additive: it must not introduce any
        // catalog integrity problem.
        let issues = problems();
        assert!(issues.is_empty(), "catalog problems: {issues:?}");
    }

    #[test]
    fn ids_are_unique_and_parented_rows_follow_dot_paths() {
        let mut seen = BTreeSet::new();
        for row in rows() {
            assert!(seen.insert(row.id.as_str()), "duplicate {}", row.id);
            match &row.parent_id {
                Some(parent) => assert!(
                    row.id.starts_with(&format!("{parent}.")),
                    "row {} is not a child of {}",
                    row.id,
                    parent
                ),
                None => assert!(!row.id.contains('.'), "top row {} is dotted", row.id),
            }
        }
    }
}
