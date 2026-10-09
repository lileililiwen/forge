//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::{
    Availability, Category, CommandExecution, CommandRow, ExecParameter, Risk, Scope,
};
use super::routes::{
    REASON_GROUP, REASON_LOCAL_FS, REASON_NOT_YET_WEB, REASON_PROVIDER, WEB_ROUTE_PROJECTS,
};

pub(super) struct CatalogBuilder {
    pub(super) rows: Vec<CommandRow>,
}
impl CatalogBuilder {
    pub(super) fn new() -> Self {
        Self { rows: Vec::new() }
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn push(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        risk: Risk,
        availability: Availability,
        route: Option<&'static str>,
        reason: Option<&'static str>,
        capabilities: &'static [&'static str],
    ) {
        let id = match parent {
            Some(prefix) => format!("{prefix}.{name}"),
            None => name.to_string(),
        };
        let cli_invocation = format!("forge {}", id.replace('.', " "));
        self.rows.push(CommandRow {
            label: name.to_string(),
            cli_invocation,
            id,
            parent_id: parent.map(str::to_string),
            summary,
            category: category.id(),
            scope: scope.id(),
            risk: risk.id(),
            availability: availability.id(),
            route,
            reason,
            capabilities,
            execution: None,
        });
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn leaf(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        risk: Risk,
        availability: Availability,
        capabilities: &'static [&'static str],
    ) {
        let reason = match availability {
            Availability::Web => None,
            Availability::NotYetWeb => Some(REASON_NOT_YET_WEB),
            _ => Some(REASON_LOCAL_FS),
        };
        self.push(
            parent,
            name,
            summary,
            category,
            scope,
            risk,
            availability,
            None,
            reason,
            capabilities,
        );
    }
    pub(super) fn web(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        capabilities: &'static [&'static str],
    ) {
        self.push(
            parent,
            name,
            summary,
            category,
            scope,
            Risk::Read,
            Availability::Web,
            Some(WEB_ROUTE_PROJECTS),
            None,
            capabilities,
        );
    }
    /// A `web` row that resolves to an explicit typed route with its own
    /// risk. Used by the workbench commands, whose endpoints carry a project
    /// id and (for `upgrade`) a genuine local-write risk that the read-only
    /// [`web`](Self::web) helper must not understate.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn web_at(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        risk: Risk,
        route: &'static str,
        capabilities: &'static [&'static str],
    ) {
        self.push(
            parent,
            name,
            summary,
            category,
            scope,
            risk,
            Availability::Web,
            Some(route),
            None,
            capabilities,
        );
    }
    /// A `web` row that is executable from the portal: it resolves to an
    /// explicit typed `POST` admin route and carries a self-describing
    /// `execution` block (method, the row's own risk, the mandatory confirm
    /// and digest-binding, and the ordered typed parameters) so the frontend
    /// renders a runnable preview→confirm→apply control from the catalog with
    /// no bespoke wiring. `parameters` is `[(name, kind, required)]` with
    /// `kind` one of `string`, `string_array` or `boolean` — never a path or
    /// argv. Used by the project feature/spec lifecycle write commands.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn web_exec(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        risk: Risk,
        route: &'static str,
        method: &'static str,
        parameters: &[(&'static str, &'static str, bool)],
        capabilities: &'static [&'static str],
    ) {
        let id = match parent {
            Some(prefix) => format!("{prefix}.{name}"),
            None => name.to_string(),
        };
        let cli_invocation = format!("forge {}", id.replace('.', " "));
        let execution = CommandExecution {
            route,
            method,
            risk: risk.id(),
            confirm_required: true,
            digest_bound: true,
            parameters: parameters
                .iter()
                .map(|(pname, kind, required)| ExecParameter {
                    name: (*pname).to_string(),
                    kind,
                    required: *required,
                })
                .collect(),
        };
        self.rows.push(CommandRow {
            label: name.to_string(),
            cli_invocation,
            id,
            parent_id: parent.map(str::to_string),
            summary,
            category: category.id(),
            scope: scope.id(),
            risk: risk.id(),
            availability: Availability::Web.id(),
            route: Some(route),
            reason: None,
            capabilities,
            execution: Some(execution),
        });
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn cli_only(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        risk: Risk,
        reason: &'static str,
        capabilities: &'static [&'static str],
    ) {
        self.push(
            parent,
            name,
            summary,
            category,
            scope,
            risk,
            Availability::CliOnly,
            None,
            Some(reason),
            capabilities,
        );
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn provider_required(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
        risk: Risk,
        capabilities: &'static [&'static str],
    ) {
        self.push(
            parent,
            name,
            summary,
            category,
            scope,
            risk,
            Availability::ProviderRequired,
            None,
            Some(REASON_PROVIDER),
            capabilities,
        );
    }
    pub(super) fn group(
        &mut self,
        parent: Option<&str>,
        name: &str,
        summary: &'static str,
        category: Category,
        scope: Scope,
    ) {
        self.cli_only(
            parent,
            name,
            summary,
            category,
            scope,
            Risk::Read,
            REASON_GROUP,
            &[],
        );
    }
    pub(super) fn build(mut self) -> Vec<CommandRow> {
        self.enumerate();
        self.rows
    }
    /// Rows are emitted in the exact order the Clap tree declares, so the
    /// parity oracle and the rendered catalog stay visually in sync.
    pub(super) fn enumerate(&mut self) {
        self.project_rows();
        self.assurance_rows();
        self.shipping_rows();
        self.fleet_rows();
        self.platform_rows();
    }
}
