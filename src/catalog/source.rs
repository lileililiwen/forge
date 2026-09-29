//! Read-only catalog source adapters
//! (`project-catalog-query-contract`).
//!
//! Every adapter wraps an existing reader rather than re-reading files:
//! the local SQLite registry ([`Registry`], opened **read-only**), the
//! external workspace registry ([`crate::fleet`]), the portable project
//! inventory ([`crate::publish::inventory`]) and explicitly declared Git
//! working trees. Only declared sources are read: Forge never scans a
//! parent directory or a sibling checkout implicitly.
//!
//! A source that cannot be read contributes an `unavailable`
//! [`SourceStatus`] with its reason — never zero rows presented as an
//! answer and never a placeholder record.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::{validate_project_id, ForgeError};
use crate::fleet::{self, FleetFreshness, FleetReport};
use crate::github::{self, GithubAdapter, GithubObservation, GithubState};
use crate::publish::inventory;
use crate::registry::Registry;

use super::record::{
    catalog_invalid, clean_field, normalize_timestamp, CatalogRecord, EvidenceState, Freshness,
    SourceKind,
};

/// Which sources a catalog query reads.
#[derive(Debug, Clone)]
pub struct CatalogSourceSelection {
    pub kinds: Vec<SourceKind>,
    /// Explicit Git working trees; only these are read.
    pub git_repositories: Vec<PathBuf>,
    /// Workspace registry document, if the workspace source is selected.
    pub workspace_registry: Option<PathBuf>,
    /// Portable inventory source, if the inventory source is selected.
    pub inventory: Option<PathBuf>,
    /// Explicit GitHub repositories (`owner/repo`) for the GitHub
    /// source. When empty, the GitHub source walks the local registry
    /// for projects whose `git_remote` looks like a GitHub URL.
    pub github_repositories: Vec<String>,
}

impl Default for CatalogSourceSelection {
    fn default() -> Self {
        CatalogSourceSelection {
            kinds: vec![SourceKind::Local],
            git_repositories: Vec::new(),
            workspace_registry: None,
            inventory: None,
            github_repositories: Vec::new(),
        }
    }
}

/// The state of one selected source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceStatus {
    pub source: String,
    pub source_kind: SourceKind,
    /// `available` or `unavailable`; never a silent zero.
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub records: usize,
}

impl SourceStatus {
    fn available(source: &str, kind: SourceKind, records: usize) -> Self {
        SourceStatus {
            source: source.to_string(),
            source_kind: kind,
            state: "available".to_string(),
            reason: None,
            records,
        }
    }

    fn unavailable(source: &str, kind: SourceKind, reason: impl Into<String>) -> Self {
        SourceStatus {
            source: source.to_string(),
            source_kind: kind,
            state: "unavailable".to_string(),
            reason: Some(clean_field(&reason.into())),
            records: 0,
        }
    }
}

/// Everything collected from the selected sources, with provenance.
#[derive(Debug, Clone)]
pub struct SourceBundle {
    pub records: Vec<CatalogRecord>,
    pub statuses: Vec<SourceStatus>,
    pub observed_at: String,
    pub max_age_seconds: i64,
}

/// One catalog read request. A struct, not positional arguments, so the
/// source list can grow without ambiguous call sites.
pub struct CatalogRequest<'a> {
    pub selection: &'a CatalogSourceSelection,
    pub registry_path: &'a Path,
    pub max_age_seconds: i64,
    pub now: DateTime<Utc>,
}

/// Read every selected source. The result always carries one status per
/// selected source, in selection order, so an unreadable source is named.
pub fn collect(request: &CatalogRequest) -> SourceBundle {
    let observed_at = request
        .now
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut bundle = SourceBundle {
        records: Vec::new(),
        statuses: Vec::new(),
        observed_at,
        max_age_seconds: request.max_age_seconds,
    };
    // The workspace join needs the local ids even when the local source
    // itself was not selected; read them without emitting local records.
    let local_ids = local_ids(request.registry_path);
    for kind in &request.selection.kinds {
        match kind {
            SourceKind::Local => collect_local(request, &mut bundle),
            SourceKind::Git => collect_git(request, &mut bundle),
            SourceKind::WorkspaceRegistry => collect_workspace(request, &local_ids, &mut bundle),
            SourceKind::Inventory => collect_inventory(request, &mut bundle),
            SourceKind::Github => collect_github(request, &mut bundle),
        }
    }
    bundle
}

fn collect_local(request: &CatalogRequest, bundle: &mut SourceBundle) {
    // No registry file means nothing is registered: an empty catalog is
    // not an error and Forge must not create the file to look.
    if !request.registry_path.is_file() {
        bundle
            .statuses
            .push(SourceStatus::available("local", SourceKind::Local, 0));
        return;
    }
    match Registry::open_read_only(request.registry_path) {
        Ok(registry) => match registry.list() {
            Ok(projects) => {
                let records: Vec<CatalogRecord> = projects
                    .iter()
                    .map(|project| local_record(project, request.max_age_seconds, request.now))
                    .collect();
                bundle.statuses.push(SourceStatus::available(
                    "local",
                    SourceKind::Local,
                    records.len(),
                ));
                bundle.records.extend(records);
            }
            Err(err) => bundle.statuses.push(SourceStatus::unavailable(
                "local",
                SourceKind::Local,
                format!("cannot read the local registry: {err}"),
            )),
        },
        Err(err) => bundle.statuses.push(SourceStatus::unavailable(
            "local",
            SourceKind::Local,
            format!("cannot open the local registry read-only: {err}"),
        )),
    }
}

fn collect_github(request: &CatalogRequest, bundle: &mut SourceBundle) {
    let adapter = GithubAdapter::from_env();
    if !adapter.binary_available() {
        let reason = if adapter.binary.is_none() {
            format!("{}: {}", github::GITHUB_BIN_ENV, adapter.source)
        } else {
            format!(
                "GitHub adapter binary `{}` is not executable; {}",
                adapter
                    .binary
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "<missing>".to_string()),
                adapter.source
            )
        };
        bundle.statuses.push(SourceStatus::unavailable(
            "github",
            SourceKind::Github,
            reason,
        ));
        return;
    }
    if adapter.token.is_none() {
        bundle.statuses.push(SourceStatus::unavailable(
            "github",
            SourceKind::Github,
            format!(
                "{} is not set; the GitHub source requires a token and is unavailable",
                github::GITHUB_TOKEN_ENV
            ),
        ));
        return;
    }
    let repositories = github_repositories_to_observe(request);
    if repositories.is_empty() {
        bundle.statuses.push(SourceStatus::unavailable(
            "github",
            SourceKind::Github,
            "no GitHub repositories were declared (pass --source github --github-repository \
             owner/repo, or set a --git-repository whose remote is on github.com, or register \
             a project whose git_remote points to a GitHub URL)"
                .to_string(),
        ));
        return;
    }
    let host = github::GITHUB_DEFAULT_HOST;
    let observation_request = github::GithubObservationRequest {
        host: host.to_string(),
        repositories: repositories.clone(),
    };
    match adapter.observe(&observation_request) {
        Ok(observations) => {
            let records: Vec<CatalogRecord> = observations
                .iter()
                .map(|observation| {
                    github::normalize_observation(observation, request.max_age_seconds, request.now)
                })
                .collect();
            let label = format!("github:{host}");
            let mut status = SourceStatus::available(&label, SourceKind::Github, records.len());
            let unavailable: Vec<&GithubObservation> = observations
                .iter()
                .filter(|observation| {
                    matches!(
                        observation.state,
                        GithubState::Unavailable { .. } | GithubState::Partial
                    )
                })
                .collect();
            if !unavailable.is_empty() {
                status.reason = Some(format!(
                    "{} of {} repository observation(s) reported an unavailable state",
                    unavailable.len(),
                    observations.len()
                ));
            }
            bundle.statuses.push(status);
            bundle.records.extend(records);
        }
        Err(err) => {
            bundle.statuses.push(SourceStatus::unavailable(
                "github",
                SourceKind::Github,
                err.to_string(),
            ));
        }
    }
}

/// Resolve the list of `owner/repo` identities the GitHub source
/// should observe. Explicit `--github-repository` flags win; when
/// none are given, walk the local registry for projects whose
/// `git_remote` looks like a GitHub URL.
fn github_repositories_to_observe(request: &CatalogRequest) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for raw in &request.selection.github_repositories {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(trimmed.to_string());
        }
    }
    if !out.is_empty() {
        return out;
    }
    if !request.registry_path.is_file() {
        return out;
    }
    let Ok(registry) = Registry::open_read_only(request.registry_path) else {
        return out;
    };
    let Ok(projects) = registry.list() else {
        return out;
    };
    for project in projects {
        let Some(remote) = project.git_remote.as_deref() else {
            continue;
        };
        if let Some(repository) = github_repository_from_remote(remote) {
            let key = repository.to_ascii_lowercase();
            if seen.insert(key) {
                out.push(repository);
            }
        }
    }
    out
}

/// Parse a `git_remote` URL and return the `owner/repo` it points to
/// when the host is github.com. SSH (`git@github.com:owner/repo.git`),
/// HTTPS (`https://github.com/owner/repo.git`) and git-protocol
/// (`git://github.com/owner/repo.git`) forms are accepted; everything
/// else returns `None` so a non-GitHub remote is silently skipped
/// rather than mis-mapped.
pub fn github_repository_from_remote(remote: &str) -> Option<String> {
    let trimmed = remote.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(stripped) = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
    {
        return owner_repo(stripped);
    }
    if let Some(stripped) = trimmed.strip_prefix("git://github.com/") {
        return owner_repo(stripped);
    }
    if let Some(stripped) = trimmed.strip_prefix("ssh://git@github.com/") {
        return owner_repo(stripped);
    }
    if let Some(stripped) = trimmed.strip_prefix("git@github.com:") {
        return owner_repo(stripped);
    }
    None
}

fn owner_repo(stripped: &str) -> Option<String> {
    let trimmed = stripped.trim_end_matches('/').trim_end_matches(".git");
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = trimmed.split('/');
    let owner = parts.next()?;
    let name = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if owner.is_empty() || name.is_empty() {
        return None;
    }
    if !owner
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return None;
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return None;
    }
    Some(format!("{owner}/{name}"))
}

fn collect_git(request: &CatalogRequest, bundle: &mut SourceBundle) {
    if request.selection.git_repositories.is_empty() {
        bundle.statuses.push(SourceStatus::unavailable(
            "git",
            SourceKind::Git,
            "the git source is selected but no --git-repository was declared",
        ));
        return;
    }
    for repository in &request.selection.git_repositories {
        match git_record(repository, request.now) {
            Ok(record) => {
                bundle
                    .statuses
                    .push(SourceStatus::available(&record.source, SourceKind::Git, 1));
                bundle.records.push(record);
            }
            Err(reason) => bundle.statuses.push(SourceStatus::unavailable(
                &format!("git:{}", absolute_display(repository)),
                SourceKind::Git,
                reason,
            )),
        }
    }
}

fn collect_workspace(
    request: &CatalogRequest,
    local_ids: &BTreeSet<String>,
    bundle: &mut SourceBundle,
) {
    let Some(path) = request.selection.workspace_registry.as_deref() else {
        bundle.statuses.push(SourceStatus::unavailable(
            "workspace-registry",
            SourceKind::WorkspaceRegistry,
            "the workspace-registry source is selected but no --workspace-registry was declared",
        ));
        return;
    };
    match fleet::observe(Some(path), request.max_age_seconds, local_ids) {
        Ok(report) => {
            let records: Vec<CatalogRecord> = report
                .entries
                .iter()
                .map(|entry| workspace_record(entry, &report, request.now))
                .collect();
            let label = format!("workspace-registry:{}", absolute_display(path));
            bundle.statuses.push(SourceStatus::available(
                &label,
                SourceKind::WorkspaceRegistry,
                records.len(),
            ));
            bundle.records.extend(records);
        }
        Err(err) => bundle.statuses.push(SourceStatus::unavailable(
            &format!("workspace-registry:{}", absolute_display(path)),
            SourceKind::WorkspaceRegistry,
            err.to_string(),
        )),
    }
}

fn collect_inventory(request: &CatalogRequest, bundle: &mut SourceBundle) {
    let Some(path) = request.selection.inventory.as_deref() else {
        bundle.statuses.push(SourceStatus::unavailable(
            "inventory",
            SourceKind::Inventory,
            "the inventory source is selected but no --inventory was declared",
        ));
        return;
    };
    match inventory::load_local(path) {
        Ok(snapshot) => {
            let label = format!("inventory:{}", absolute_display(path));
            let records: Vec<CatalogRecord> = snapshot
                .projects
                .iter()
                .map(|entry| {
                    inventory_record(entry, &snapshot, request.max_age_seconds, request.now)
                })
                .collect();
            let mut status = SourceStatus::available(&label, SourceKind::Inventory, records.len());
            if !snapshot.malformed.is_empty() {
                status.reason = Some(format!(
                    "{} inventory entr(ies) were refused by the inventory contract and are \
                     not projected",
                    snapshot.malformed.len()
                ));
            }
            bundle.statuses.push(status);
            bundle.records.extend(records);
        }
        Err(err) => bundle.statuses.push(SourceStatus::unavailable(
            &format!("inventory:{}", absolute_display(path)),
            SourceKind::Inventory,
            err.to_string(),
        )),
    }
}

fn local_ids(registry_path: &Path) -> BTreeSet<String> {
    if !registry_path.is_file() {
        return BTreeSet::new();
    }
    Registry::open_read_only(registry_path)
        .and_then(|registry| registry.list())
        .map(|projects| projects.into_iter().map(|project| project.id).collect())
        .unwrap_or_default()
}

fn local_record(
    project: &crate::registry::ProjectRecord,
    max_age_seconds: i64,
    now: DateTime<Utc>,
) -> CatalogRecord {
    let evidence = if project.available {
        EvidenceState::Present
    } else {
        EvidenceState::Unavailable
    };
    CatalogRecord {
        project_id: project.id.clone(),
        name: Some(clean_field(&project.name)),
        source: "local".to_string(),
        source_kind: SourceKind::Local,
        source_revision: project.last_commit.clone(),
        observed_at: normalize_timestamp(&project.observed_at),
        freshness: Freshness::Unknown,
        profile: non_empty(&project.profile),
        lifecycle: project.maturity.as_deref().and_then(non_empty_ref),
        repository: project.git_remote.as_deref().map(clean_field),
        tags: project
            .features
            .keys()
            .map(|key| clean_field(key))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        languages: project
            .stack
            .as_deref()
            .and_then(non_empty_ref)
            .map(|stack| vec![stack])
            .unwrap_or_default(),
        ci: project.quality_status.as_deref().and_then(non_empty_ref),
        compose: None,
        evidence,
    }
    .with_freshness(max_age_seconds, now)
}

/// A workspace entry is stamped with the moment the registry *document*
/// was last written, not the moment Forge read it, so two reads of an
/// unchanged registry produce identical bytes.
fn workspace_record(
    entry: &fleet::FleetEntry,
    report: &FleetReport,
    now: DateTime<Utc>,
) -> CatalogRecord {
    let observed_at = match report.age_seconds {
        Some(age) => (now - chrono::Duration::seconds(age.max(0)))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        None => normalize_timestamp(&report.observed_at),
    };
    CatalogRecord {
        project_id: entry.id.clone(),
        name: None,
        source: "workspace-registry".to_string(),
        source_kind: SourceKind::WorkspaceRegistry,
        source_revision: None,
        observed_at,
        freshness: fleet_freshness(report.freshness),
        profile: non_empty(&entry.profile),
        lifecycle: non_empty(&entry.lifecycle),
        repository: None,
        tags: Vec::new(),
        languages: Vec::new(),
        ci: None,
        compose: None,
        evidence: if entry.forge_yaml_present {
            EvidenceState::Present
        } else {
            EvidenceState::Absent
        },
    }
}

fn inventory_record(
    entry: &inventory::InventoryEntry,
    snapshot: &inventory::InventorySnapshot,
    max_age_seconds: i64,
    now: DateTime<Utc>,
) -> CatalogRecord {
    CatalogRecord {
        project_id: entry.id.clone(),
        name: None,
        source: "inventory".to_string(),
        source_kind: SourceKind::Inventory,
        source_revision: Some(entry.revision.clone()),
        observed_at: snapshot.generated_at.clone(),
        freshness: Freshness::Unknown,
        profile: non_empty(&entry.profile),
        lifecycle: None,
        repository: Some(clean_field(&entry.repository)),
        tags: Vec::new(),
        languages: Vec::new(),
        ci: None,
        compose: Some(inventory_compose_state(
            entry.compose_file.as_deref(),
            entry.source_path.as_deref(),
        )),
        evidence: EvidenceState::Present,
    }
    .with_freshness(max_age_seconds, now)
}

fn inventory_compose_state(compose_file: Option<&str>, source_path: Option<&str>) -> String {
    match (compose_file, source_path) {
        (Some(file), Some(root)) if Path::new(root).join(file).is_file() => {
            "compose_ready".to_string()
        }
        (Some(_), Some(root)) if !Path::new(root).is_dir() => "source_unavailable".to_string(),
        (None, _) => "compose_missing".to_string(),
        (Some(_), None) => "source_unavailable".to_string(),
        (Some(_), Some(_)) => "compose_missing".to_string(),
    }
}

fn git_record(path: &Path, now: DateTime<Utc>) -> Result<CatalogRecord, String> {
    let id = path
        .file_name()
        .map(|name| name.to_string_lossy().trim().to_string())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "git repository path has no usable directory name".to_string())?;
    validate_project_id(&id).map_err(|err| err.to_string())?;
    if !path.is_dir() {
        return Err(format!(
            "declared git repository `{}` is not a directory",
            path.display()
        ));
    }
    let revision = git_output(path, &["rev-parse", "HEAD"]).ok_or_else(|| {
        format!(
            "declared git repository `{}` is not a readable working tree",
            path.display()
        )
    })?;
    let remote =
        git_output(path, &["remote", "get-url", "origin"]).map(|value| clean_field(&value));
    Ok(CatalogRecord {
        project_id: id,
        name: None,
        source: format!("git:{}", absolute_display(path)),
        source_kind: SourceKind::Git,
        source_revision: Some(revision),
        observed_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        freshness: Freshness::Current,
        profile: None,
        lifecycle: None,
        repository: remote,
        tags: Vec::new(),
        languages: Vec::new(),
        ci: None,
        compose: None,
        evidence: EvidenceState::Present,
    })
}

fn git_output(dir: &Path, args: &[&str]) -> Option<String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(dir);
    for arg in args {
        command.arg(arg);
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn fleet_freshness(freshness: FleetFreshness) -> Freshness {
    match freshness {
        FleetFreshness::Fresh => Freshness::Current,
        FleetFreshness::Stale => Freshness::Stale,
        FleetFreshness::Unconfigured => Freshness::Unknown,
    }
}

fn non_empty(value: &str) -> Option<String> {
    non_empty_ref(value).map(|value| clean_field(&value))
}

fn non_empty_ref(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Echo a declared path back for display, made absolute when possible.
pub fn absolute_display(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(path))
                    .unwrap_or_else(|_| path.to_path_buf())
            }
        })
        .display()
        .to_string()
}

/// Validate the `--max-age` window (shared with the fleet vocabulary).
pub fn validate_max_age(max_age_seconds: i64) -> Result<(), ForgeError> {
    if !(super::record::MIN_MAX_AGE_SECONDS..=super::record::MAX_MAX_AGE_SECONDS)
        .contains(&max_age_seconds)
    {
        return Err(catalog_invalid(format!(
            "--max-age {max_age_seconds} is outside the bounded range {}..={} seconds",
            super::record::MIN_MAX_AGE_SECONDS,
            super::record::MAX_MAX_AGE_SECONDS
        )));
    }
    Ok(())
}
