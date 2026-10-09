//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use forge::core::ForgeError;

use super::commands_ops::InventoryCommands;
use super::fleet_exec::{legacy_inventory_snapshot, load_inventory_snapshot};
use super::projects::as_output;
use crate::{Format, Output};

pub(crate) fn cmd_inventory(
    command: &InventoryCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        InventoryCommands::Show { source, domain } => {
            use forge::publish::inventory::{classify, resolve_source};
            let path = source
                .clone()
                .or_else(|| resolve_source(None))
                .ok_or_else(|| ForgeError::PublishInvalid {
                    reason: "inventory source is not configured: pass a positional path or set \
                         $FORGE_INVENTORY_SOURCE"
                        .to_string(),
                })?;
            let snapshot = load_inventory_snapshot(&path)?;
            let report = classify(&snapshot, domain);
            let mut human = format!(
                "inventory {contract}: {provider} at {source}, declared={declared}, \
                 compose_ready={ready}, compose_missing={missing}, \
                 source_unavailable={unavailable}, invalid={invalid}\n",
                contract = report.contract,
                provider = report.provider,
                source = source_label(&snapshot),
                declared = report.entries.len(),
                ready = report
                    .entries
                    .iter()
                    .filter(|e| matches!(
                        e.classification,
                        forge::publish::inventory::InventoryClassification::ComposeReady
                    ))
                    .count(),
                missing = report
                    .entries
                    .iter()
                    .filter(|e| matches!(
                        e.classification,
                        forge::publish::inventory::InventoryClassification::ComposeMissing
                    ))
                    .count(),
                unavailable = report
                    .entries
                    .iter()
                    .filter(|e| matches!(
                        e.classification,
                        forge::publish::inventory::InventoryClassification::SourceUnavailable
                    ))
                    .count(),
                invalid = report
                    .entries
                    .iter()
                    .filter(|e| matches!(
                        e.classification,
                        forge::publish::inventory::InventoryClassification::Invalid
                    ))
                    .count(),
            );
            for entry in &report.entries {
                human.push_str(&format!(
                    "  {id:<20} {runtime:<8} {classification:<18} subdomain={subdomain}\n",
                    id = entry.id,
                    runtime = entry.runtime.as_str(),
                    classification = entry.classification.as_str(),
                    subdomain = entry.subdomain.as_deref().unwrap_or("-"),
                ));
            }
            let json = serde_json::to_value(&report).map_err(|err| ForgeError::PublishInvalid {
                reason: format!("cannot encode inventory report: {err}"),
            })?;
            Ok(as_output(format, human, json))
        }
    }
}

fn source_label(snapshot: &forge::publish::inventory::InventorySnapshot) -> String {
    snapshot
        .source
        .clone()
        .unwrap_or_else(|| "<unspecified>".to_string())
}

/// Render the `forge fleet online` plan. Every value the operator
/// would see at runtime is rendered as plain text, including the
/// SSH argv and the curl argv. `--dry-run` is the rehearsal: nothing
/// is contacted and nothing is journaled; the plan preview is the
/// entire output. The per-host list deliberately comes from the
/// served Caddyfile, which is only readable at runtime — a dry-run
/// that printed a synthetic host list would mislead the operator
/// when the served file disagrees with the inventory.
fn render_fleet_online_plan(
    inventory_source: &str,
    target: &str,
    domain: &str,
    timeout_secs: u64,
    _entries: &[forge::fleet::online::LivenessEntry],
    caddyfile_path: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "fleet online: dry-run, inventory={inventory_source}, target={target}, domain={domain}, \
         timeout={timeout_secs}s\n"
    ));
    out.push_str(&format!(
        "  probe plan: 1) ssh {target} cat {caddyfile_path}\n"
    ));
    out.push_str(&format!(
        "  probe plan: 2) ssh {target} env PATH=<docker-desktop> /usr/local/bin/docker ps --no-trunc --format ...\n"
    ));
    out.push_str(&format!(
        "  probe plan: 3) per compose-ready entry with a served route, one bounded HTTPS GET (curl -s -m {timeout_secs} --max-filesize 65536 https://<host>); per-host list is resolved at runtime from the served Caddyfile, not printed here\n"
    ));
    out
}

/// Probe the served router rules and target containers to verdict
/// whether every routed host is online. Mirrors `cmd_publish_fleet`'s
/// inventory resolution exactly: explicit `--inventory` first, then
/// the legacy workspace-governance registry compatibility adapter,
/// then the inventory classification. Only `compose_ready` entries
/// are probed; the others are reported with their classification,
/// never silently omitted. The whole surface is read-only: no
/// journal rows, no registry writes, no target writes, and every
/// captured string passes through `policy::redact_credentials`.
#[allow(clippy::too_many_arguments)]
pub(super) fn cmd_fleet_online(
    inventory: Option<&std::path::Path>,
    fleet_registry: Option<&std::path::Path>,
    workspace_root: Option<&std::path::Path>,
    domain: String,
    timeout_secs: u64,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::fleet::online::{
        caddyfile_path_from_env, cat_caddyfile_command, classify, docker_ps_command,
        match_container, parse_caddyfile_hosts, probe_http, route_for_project, ssh_target_from_env,
        ProbeFacts, DEFAULT_HTTP_TIMEOUT_SECS, LIVENESS_CONTRACT_VERSION, MAX_HTTP_TIMEOUT_SECS,
        SSH_TARGET_ENV,
    };
    use forge::publish::fleet::{default_registry_path, default_workspace_root};
    use forge::publish::inventory::{
        classify as classify_inventory, resolve_source, DEFAULT_DOMAIN,
    };

    if !(1..=MAX_HTTP_TIMEOUT_SECS).contains(&timeout_secs) {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "timeout {timeout_secs}s is out of bounds; expected 1..={MAX_HTTP_TIMEOUT_SECS}"
            ),
        });
    }
    let _ = DEFAULT_HTTP_TIMEOUT_SECS;
    let _ = SSH_TARGET_ENV;

    // Resolve the inventory exactly the way `cmd_publish_fleet` does:
    // explicit `--inventory` first, then the legacy registry
    // compatibility adapter. The seven-project handoff keeps working
    // without a sibling checkout during migration.
    let lifecycle = "active".to_string();
    let (snapshot, source_label) = if let Some(path) = inventory
        .map(|p| p.to_path_buf())
        .or_else(|| resolve_source(None))
        .or_else(|| fleet_registry.map(|p| p.to_path_buf()))
    {
        let snapshot = load_inventory_snapshot(&path)?;
        (snapshot, format!("inventory:{}", path.display()))
    } else {
        let workspace_root = workspace_root
            .map(|p| p.to_path_buf())
            .unwrap_or_else(default_workspace_root);
        let registry_path = fleet_registry
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| default_registry_path(Some(&workspace_root)));
        let snapshot = legacy_inventory_snapshot(&registry_path, &workspace_root, &lifecycle)?;
        (snapshot, format!("registry:{}", registry_path.display()))
    };

    let fleet_report = classify_inventory(&snapshot, DEFAULT_DOMAIN);
    let target = ssh_target_from_env();
    let caddyfile_path = caddyfile_path_from_env();

    // Dry-run: render the plan, return success, journal nothing.
    if dry_run {
        let entries: Vec<forge::fleet::online::LivenessEntry> = fleet_report
            .entries
            .iter()
            .map(|entry| forge::fleet::online::LivenessEntry {
                id: entry.id.clone(),
                classification: entry.classification.as_str().to_string(),
                container: None,
                route: None,
                http_status: None,
                verdict: forge::fleet::online::Verdict::Unavailable,
                detail: None,
            })
            .collect();
        let human = render_fleet_online_plan(
            &source_label,
            &target,
            &domain,
            timeout_secs,
            &entries,
            &caddyfile_path,
        );
        let json = serde_json::json!({
            "contract": LIVENESS_CONTRACT_VERSION,
            "dry_run": true,
            "inventory_source": source_label,
            "target": target,
            "domain": domain,
            "timeout_secs": timeout_secs,
            "caddyfile_path": caddyfile_path,
        });
        return Ok(as_output(format, human, json));
    }

    // Real probe: read the served Caddyfile once via SSH (or a
    // local test stub named in `FORGE_PUBLISH_SSH_TARGET`).
    let caddyfile_argv = cat_caddyfile_command(&target, &caddyfile_path);
    let caddyfile_argv_str: Vec<String> = caddyfile_argv
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let mut ssh_command = std::process::Command::new(&caddyfile_argv[0]);
    for arg in caddyfile_argv.iter().skip(1) {
        ssh_command.arg(arg);
    }
    ssh_command.env("LC_ALL", "C");
    let caddyfile_output = match ssh_command.output() {
        Ok(out) => out,
        Err(err) => {
            return Err(ForgeError::PublishInvalid {
                reason: format!(
                    "cannot spawn `{}` to read served Caddyfile: {err}; \
                     the binary must be present on the controller",
                    caddyfile_argv_str
                        .first()
                        .map(|s| s.as_str())
                        .unwrap_or("<ssh>")
                ),
            });
        }
    };
    let caddyfile_text = String::from_utf8_lossy(&caddyfile_output.stdout).into_owned();
    let caddyfile_stderr = String::from_utf8_lossy(&caddyfile_output.stderr).into_owned();
    if !caddyfile_output.status.success() && caddyfile_text.trim().is_empty() {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "probe `{}` failed: {}",
                caddyfile_argv_str.join(" "),
                forge::policy::redact_credentials(caddyfile_stderr.trim())
            ),
        });
    }
    let nav_host = std::env::var("FORGE_PUBLISH_NAV_HOST")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "apps".to_string());
    let routes = parse_caddyfile_hosts(&caddyfile_text, &domain, &nav_host).map_err(|err| {
        ForgeError::PublishInvalid {
            reason: format!("served Caddyfile `{caddyfile_path}` is malformed: {err}"),
        }
    })?;

    // Read container state from the target.
    let docker_argv = docker_ps_command(&target);
    let docker_argv_str: Vec<String> = docker_argv
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let mut docker_command = std::process::Command::new(&docker_argv[0]);
    for arg in docker_argv.iter().skip(1) {
        docker_command.arg(arg);
    }
    docker_command.env("LC_ALL", "C");
    let docker_output = match docker_command.output() {
        Ok(out) => out,
        Err(err) => {
            return Err(ForgeError::PublishInvalid {
                reason: format!(
                    "cannot spawn `{}` to read target `docker ps`: {err}; \
                     the binary must be present on the controller",
                    docker_argv_str
                        .first()
                        .map(|s| s.as_str())
                        .unwrap_or("<ssh>")
                ),
            });
        }
    };
    let docker_stdout = String::from_utf8_lossy(&docker_output.stdout).into_owned();
    let docker_stderr = String::from_utf8_lossy(&docker_output.stderr).into_owned();
    if !docker_output.status.success() && docker_stdout.trim().is_empty() {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "probe `{}` failed: {}",
                docker_argv_str.join(" "),
                forge::policy::redact_credentials(docker_stderr.trim())
            ),
        });
    }

    let mut assembled: Vec<(String, String, ProbeFacts, Option<String>)> =
        Vec::with_capacity(fleet_report.entries.len());
    for entry in &fleet_report.entries {
        let id = entry.id.clone();
        let classification = entry.classification.as_str().to_string();
        if classification != "compose_ready" {
            // Non-ready entries are reported with their classification
            // and never probed. No container / route / http probe.
            assembled.push((
                id,
                classification,
                ProbeFacts {
                    container: None,
                    route: None,
                    http_status: None,
                    body: None,
                    probe_error: None,
                },
                None,
            ));
            continue;
        }
        // Compose-ready: build the public URL the served router
        // claims, then verify the container is up.
        let route = route_for_project(&routes, &id, &domain);
        let container = match_container(&docker_stdout, &id);
        match route {
            None => assembled.push((
                id,
                classification,
                ProbeFacts {
                    container,
                    route: None,
                    http_status: None,
                    body: None,
                    probe_error: None,
                },
                None,
            )),
            Some(host) => {
                let url = format!("https://{host}");
                let probe = probe_http(&url, timeout_secs);
                match probe {
                    Ok((status, body)) => assembled.push((
                        id,
                        classification,
                        ProbeFacts {
                            container,
                            route: Some(host.clone()),
                            http_status: Some(status),
                            body: Some(body),
                            probe_error: None,
                        },
                        Some(url),
                    )),
                    Err(err) => assembled.push((
                        id,
                        classification,
                        ProbeFacts {
                            container,
                            route: Some(host.clone()),
                            http_status: None,
                            body: None,
                            probe_error: Some(err.to_string()),
                        },
                        Some(url),
                    )),
                }
            }
        }
    }

    // Run every entry through the classifier (one place so the
    // verdict matrix has exactly one source of truth).
    let entries: Vec<(String, String, ProbeFacts)> = assembled
        .into_iter()
        .map(|(id, classification, facts, _url)| (id, classification, facts))
        .collect();
    let probe_results: Vec<(String, String, ProbeFacts)> = entries.clone();
    let mut entries_for_report: Vec<(String, String, ProbeFacts)> =
        Vec::with_capacity(entries.len());
    let mut probe_summary: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (id, classification, facts) in probe_results {
        let (verdict, _status, _detail) = classify(facts.clone());
        *probe_summary.entry(verdict.as_str()).or_insert(0) += 1;
        entries_for_report.push((id, classification, facts));
    }
    let _ = probe_summary;

    let now = forge::publish::inventory::now_rfc3339();
    let report = forge::fleet::online::build_report(
        entries_for_report,
        source_label.clone(),
        target.clone(),
        domain.clone(),
        now.clone(),
    );

    let mut human = String::new();
    human.push_str(&format!(
        "fleet online ({contract}): {source}, target={target}, domain={domain}\n",
        contract = report.contract,
        source = source_label,
    ));
    human.push_str(&format!(
        "  summary: online={online} down={down} no_route={no_route} \
         not_deployed={not_deployed} unavailable={unavailable} skipped={skipped}\n",
        online = report.summary.online,
        down = report.summary.down,
        no_route = report.summary.no_route,
        not_deployed = report.summary.not_deployed,
        unavailable = report.summary.unavailable,
        skipped = report.summary.skipped,
    ));
    for entry in &report.entries {
        let container = entry.container.as_deref().unwrap_or("-");
        let route = entry.route.as_deref().unwrap_or("-");
        let status = entry
            .http_status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "-".to_string());
        let detail = entry.detail.as_deref().unwrap_or("");
        human.push_str(&format!(
            "  {id:<24} {verdict:<13} classification={classification:<14} \
             container={container:<24} route={route:<32} http={status:<4} {detail}\n",
            id = entry.id,
            verdict = entry.verdict.as_str(),
            classification = entry.classification,
            container = container,
            route = route,
            status = status,
            detail = if detail.is_empty() { "" } else { detail },
        ));
    }
    let json = serde_json::to_value(&report).map_err(|err| ForgeError::PublishInvalid {
        reason: format!("cannot encode fleet liveness report: {err}"),
    })?;
    let output = as_output(format, human, json);

    if !report.summary.all_probed_online() {
        // Echo the report so operators get the same content in
        // success and failure paths.
        match &output {
            Output::Human(text) => println!("{text}"),
            Output::Json(value) => {
                println!("{}", serde_json::to_string_pretty(value).unwrap());
            }
            Output::Raw(text) => print!("{text}"),
        }
        return Err(ForgeError::PublishDeployFailed {
            reason: format!(
                "fleet online: {} of {} probed host(s) are not ONLINE ({} down, {} no_route, {} not_deployed, {} unavailable)",
                report.summary.down
                    + report.summary.no_route
                    + report.summary.not_deployed
                    + report.summary.unavailable,
                report.summary.online
                    + report.summary.down
                    + report.summary.no_route
                    + report.summary.not_deployed
                    + report.summary.unavailable,
                report.summary.down,
                report.summary.no_route,
                report.summary.not_deployed,
                report.summary.unavailable,
            ),
        });
    }
    Ok(output)
}

#[cfg(test)]
pub(super) mod command_catalog_tests {
    use crate::Cli;
    use clap::CommandFactory;
    use std::collections::BTreeSet;

    fn walk(cmd: &clap::Command, prefix: &str, out: &mut BTreeSet<String>) {
        for sub in cmd.get_subcommands() {
            let name = sub.get_name();
            let path = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{prefix}.{name}")
            };
            out.insert(path.clone());
            walk(sub, &path, out);
        }
    }

    #[test]
    fn catalog_has_exactly_one_row_per_clap_path() {
        let root = <Cli as CommandFactory>::command();
        let mut clap_paths = BTreeSet::new();
        walk(&root, "", &mut clap_paths);
        // Clap adds `help` only at build time, so the catalog carries it
        // explicitly.
        clap_paths.insert("help".to_string());

        let catalog_ids: BTreeSet<String> = forge::api::command_catalog::rows()
            .iter()
            .map(|row| row.id.clone())
            .collect();
        let missing: Vec<&str> = clap_paths
            .difference(&catalog_ids)
            .map(String::as_str)
            .collect();
        let extra: Vec<&str> = catalog_ids
            .difference(&clap_paths)
            .map(String::as_str)
            .collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "catalog drifted from the Clap tree: missing={missing:?} extra={extra:?}",
        );
    }

    #[test]
    fn catalog_integrity_is_clean() {
        let problems = forge::api::command_catalog::problems();
        assert!(
            problems.is_empty(),
            "catalog integrity problems: {problems:?}"
        );
    }
}
