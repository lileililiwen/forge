//! Deterministic, receipt-protected project metadata generation and search.

use crate::core::{validate_project_id, ForgeError};
use crate::registry::{ProjectRecord, Registry};
use crate::standard::sha256_hex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path};

mod validation;
use validation::validate_metadata_sources;

pub const METADATA_CONTRACT: &str = "forge-project-metadata/1.0.0";
const GENERATOR_ID: &str = "forge-project-metadata";
const METADATA_PATH: &str = ".forge/project-metadata.json";
const RECEIPT_PATH: &str = ".forge/project-metadata.receipt.json";
const MAX_SOURCE_BYTES: u64 = 1_048_576;
const MAX_SPEC_FILES: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProjectMetadata {
    pub contract: String,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audience: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub use_cases: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub search_phrases: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<Capability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<Interface>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub integration_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documentation: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub specifications: Vec<String>,
    pub source_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Interface {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provides: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mechanism: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildOutcome {
    pub project_id: String,
    pub state: &'static str,
    pub path: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetadataMatch {
    pub project_id: String,
    pub score: usize,
    pub matched_fields: Vec<String>,
    pub metadata: ProjectMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetadataIssue {
    pub project_id: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchOutcome {
    pub contract: &'static str,
    pub query: String,
    pub matches: Vec<MetadataMatch>,
    pub issues: Vec<MetadataIssue>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnershipReceipt {
    schema_version: u32,
    generator: String,
    project_id: String,
    artifact_path: String,
    artifact_sha256: String,
    source_revision: String,
}

#[derive(Debug)]
struct Sources {
    declaration_bytes: Vec<u8>,
    declaration: Value,
    manifest_bytes: Option<Vec<u8>>,
    manifest: Option<Value>,
    readme_bytes: Option<Vec<u8>>,
    spec_paths: Vec<String>,
}

pub fn build_project_metadata(root: &Path) -> Result<BuildOutcome, ForgeError> {
    let metadata = make_metadata(root)?;
    let root = root
        .canonicalize()
        .map_err(|err| invalid(format!("cannot resolve project root: {err}")))?;
    let forge_dir = root.join(".forge");
    if forge_dir.exists() {
        ensure_within(&forge_dir, &root)?;
        if !forge_dir.is_dir() {
            return Err(conflict(".forge is not a directory"));
        }
    }
    let metadata_path = root.join(METADATA_PATH);
    let receipt_path = root.join(RECEIPT_PATH);
    let metadata_exists = path_present(&metadata_path)?;
    let receipt_exists = path_present(&receipt_path)?;
    let bytes = serde_json::to_vec_pretty(&metadata).map_err(|err| invalid(err.to_string()))?;
    let digest = sha256_hex(&bytes);

    let state = match (metadata_exists, receipt_exists) {
        (false, false) => "created",
        (true, true) => {
            let current = read_bounded(&metadata_path, &root)?;
            let receipt_bytes = read_bounded(&receipt_path, &root)?;
            let receipt: OwnershipReceipt = serde_json::from_slice(&receipt_bytes)
                .map_err(|_| invalid("project metadata receipt is malformed"))?;
            let current_metadata: ProjectMetadata = serde_json::from_slice(&current)
                .map_err(|_| conflict("project metadata artifact is malformed"))?;
            if receipt.schema_version != 1
                || receipt.generator != GENERATOR_ID
                || receipt.project_id != metadata.project_id
                || receipt.artifact_path != METADATA_PATH
                || receipt.artifact_sha256 != sha256_hex(&current)
                || current_metadata.contract != METADATA_CONTRACT
                || current_metadata.project_id != receipt.project_id
                || receipt.source_revision != current_metadata.source_revision
            {
                return Err(conflict(
                    "project metadata ownership receipt does not match its artifact",
                ));
            }
            if current == bytes && receipt.source_revision == metadata.source_revision {
                return Ok(BuildOutcome {
                    project_id: metadata.project_id,
                    state: "unchanged",
                    path: METADATA_PATH,
                });
            }
            "updated"
        }
        _ => {
            return Err(conflict(
                "project metadata artifact and ownership receipt must both be absent or valid",
            ))
        }
    };

    fs::create_dir_all(&forge_dir)
        .map_err(|err| invalid(format!("cannot create .forge directory: {err}")))?;
    let receipt = OwnershipReceipt {
        schema_version: 1,
        generator: GENERATOR_ID.into(),
        project_id: metadata.project_id.clone(),
        artifact_path: METADATA_PATH.into(),
        artifact_sha256: digest,
        source_revision: metadata.source_revision.clone(),
    };
    let receipt_bytes =
        serde_json::to_vec_pretty(&receipt).map_err(|err| invalid(err.to_string()))?;
    replace_pair(
        &metadata_path,
        &bytes,
        &receipt_path,
        &receipt_bytes,
        metadata_exists,
    )?;
    Ok(BuildOutcome {
        project_id: metadata.project_id,
        state,
        path: METADATA_PATH,
    })
}

pub fn search_registered_projects(
    registry_path: &Path,
    query: &str,
    project_id: Option<&str>,
    limit: usize,
) -> Result<SearchOutcome, ForgeError> {
    let terms = validate_query(query, project_id, limit)?;
    let registry = Registry::open_read_only(registry_path)?;
    let mut matches = Vec::new();
    let mut issues = Vec::new();
    for record in registry.list()? {
        if project_id.is_some_and(|wanted| record.id != wanted) {
            continue;
        }
        match read_registered_metadata(&record) {
            Ok(metadata) => {
                if metadata.contract != METADATA_CONTRACT {
                    issues.push(MetadataIssue {
                        project_id: record.id,
                        code: "metadata-contract-unsupported".into(),
                    });
                    continue;
                }
                let current = match make_metadata(Path::new(&record.path)) {
                    Ok(current) => current,
                    Err(_) => {
                        issues.push(MetadataIssue {
                            project_id: record.id,
                            code: "metadata-source-unavailable".into(),
                        });
                        continue;
                    }
                };
                if current.source_revision != metadata.source_revision {
                    issues.push(MetadataIssue {
                        project_id: record.id,
                        code: "metadata-stale".into(),
                    });
                    continue;
                }
                if let Some((score, matched_fields)) = match_score(&metadata, &terms) {
                    matches.push(MetadataMatch {
                        project_id: record.id,
                        score,
                        matched_fields,
                        metadata,
                    });
                }
            }
            Err(code) => issues.push(MetadataIssue {
                project_id: record.id,
                code,
            }),
        }
    }
    matches.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.project_id.cmp(&b.project_id))
    });
    matches.truncate(limit);
    issues.sort_by(|a, b| {
        a.project_id
            .cmp(&b.project_id)
            .then_with(|| a.code.cmp(&b.code))
    });
    Ok(SearchOutcome {
        contract: METADATA_CONTRACT,
        query: query.to_owned(),
        matches,
        issues,
    })
}

pub fn validate_query(
    query: &str,
    project_id: Option<&str>,
    limit: usize,
) -> Result<Vec<String>, ForgeError> {
    if query.chars().count() > 128 || !(1..=50).contains(&limit) {
        return Err(ForgeError::InvalidArgument {
            reason: "query must be 1..=128 characters and limit must be 1..=50".into(),
        });
    }
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if terms.is_empty() || terms.len() > 8 {
        return Err(ForgeError::InvalidArgument {
            reason: "query must contain 1..=8 non-empty terms".into(),
        });
    }
    if let Some(id) = project_id {
        validate_project_id(id).map_err(|reason| ForgeError::InvalidArgument {
            reason: format!("invalid project id: {reason}"),
        })?;
    }
    Ok(terms)
}

fn make_metadata(root: &Path) -> Result<ProjectMetadata, ForgeError> {
    let sources = collect_sources(root)?;
    validate_metadata_sources(&sources.declaration, sources.manifest.as_ref())?;
    let id = sources
        .declaration
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(".project.json.id is required"))?;
    validate_project_id(id).map_err(|reason| invalid(format!(".project.json.id: {reason}")))?;
    let discoverability = sources
        .declaration
        .get("discoverability")
        .and_then(Value::as_object);
    let string = |key: &str| {
        discoverability
            .and_then(|d| d.get(key))
            .and_then(Value::as_str)
            .map(redact)
    };
    let strings = |key: &str| -> Vec<String> {
        discoverability
            .and_then(|d| d.get(key))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(redact)
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };
    let capabilities = sources
        .declaration
        .get("capabilities")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .map(|(id, value)| Capability {
                    id: id.clone(),
                    owner: value.get("owner").and_then(Value::as_str).map(redact),
                    evidence_state: value
                        .get("evidence_state")
                        .and_then(Value::as_str)
                        .map(redact),
                    evidence_ref: value
                        .get("evidence_ref")
                        .and_then(Value::as_str)
                        .map(redact),
                })
                .collect()
        })
        .unwrap_or_default();
    let interfaces = sources
        .declaration
        .get("interfaces")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|value| {
                    Some(Interface {
                        id: value.get("id")?.as_str()?.to_owned(),
                        provides: value.get("provides").and_then(Value::as_bool),
                        consumes: value.get("consumes").and_then(Value::as_bool),
                        provider: value.get("provider").and_then(Value::as_str).map(redact),
                        mechanism: value.get("mechanism").and_then(Value::as_str).map(redact),
                        contract: value.get("contract").and_then(Value::as_str).map(redact),
                        digest: value.get("digest").and_then(Value::as_str).map(redact),
                        entry: value.get("entry").and_then(Value::as_str).map(redact),
                        required: value.get("required").and_then(Value::as_bool),
                        evidence_ref: value
                            .get("evidence_ref")
                            .and_then(Value::as_str)
                            .map(redact),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let integration_ids = sources
        .declaration
        .get("integrations")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.get("id").and_then(Value::as_str).map(redact))
                .collect()
        })
        .unwrap_or_default();
    let mut source_payload = Map::new();
    source_payload.insert(
        "declaration".into(),
        Value::String(sha256_hex(&sources.declaration_bytes)),
    );
    source_payload.insert(
        "manifest".into(),
        Value::String(
            sources
                .manifest_bytes
                .as_deref()
                .map(sha256_hex)
                .unwrap_or_default(),
        ),
    );
    source_payload.insert(
        "readme".into(),
        Value::String(
            sources
                .readme_bytes
                .as_deref()
                .map(sha256_hex)
                .unwrap_or_default(),
        ),
    );
    source_payload.insert(
        "specifications".into(),
        serde_json::to_value(&sources.spec_paths).unwrap(),
    );
    let source_revision = sha256_hex(&serde_json::to_vec(&Value::Object(source_payload)).unwrap());
    let name = sources
        .manifest
        .as_ref()
        .and_then(|value| value.get("project"))
        .and_then(|p| p.get("name"))
        .and_then(Value::as_str)
        .map(redact)
        .or_else(|| Some(id.to_owned()));
    let mut keywords = strings("keywords");
    let mut topics = strings("topics");
    let mut audience = strings("audience");
    let mut use_cases = strings("use_cases");
    let mut search_phrases = strings("search_phrases");
    sort_dedup(&mut keywords);
    sort_dedup(&mut topics);
    sort_dedup(&mut audience);
    sort_dedup(&mut use_cases);
    sort_dedup(&mut search_phrases);
    let mut specifications = sources.spec_paths;
    sort_dedup(&mut specifications);
    let mut documentation = sources
        .readme_bytes
        .map(|_| vec!["README.md".into()])
        .unwrap_or_default();
    if let Some(homepage) = string("homepage") {
        documentation.push(homepage);
    }
    sort_dedup(&mut documentation);
    let profile = declaration_string(&sources.declaration, "profile").or_else(|| {
        sources
            .manifest
            .as_ref()
            .and_then(|value| value.get("project"))
            .and_then(|project| project.get("profile"))
            .and_then(Value::as_str)
            .map(redact)
    });
    let mut metadata = ProjectMetadata {
        contract: METADATA_CONTRACT.into(),
        project_id: id.into(),
        name,
        description: string("description"),
        keywords,
        topics,
        audience,
        use_cases,
        search_phrases,
        role: declaration_string(&sources.declaration, "role"),
        stack: declaration_string(&sources.declaration, "stack"),
        profile,
        lifecycle: declaration_string(&sources.declaration, "lifecycle"),
        capabilities,
        interfaces,
        integration_ids,
        documentation,
        specifications,
        source_revision,
    };
    metadata.capabilities.sort_by(|a, b| a.id.cmp(&b.id));
    metadata.interfaces.sort_by(|a, b| a.id.cmp(&b.id));
    metadata.integration_ids.sort();
    metadata.integration_ids.dedup();
    Ok(metadata)
}

fn collect_sources(root: &Path) -> Result<Sources, ForgeError> {
    let root = root
        .canonicalize()
        .map_err(|err| invalid(format!("cannot resolve project root: {err}")))?;
    if !root.is_dir() {
        return Err(invalid("project root is not a directory"));
    }
    let declaration_path = root.join(".project.json");
    let declaration_bytes = read_bounded(&declaration_path, &root)
        .map_err(|e| invalid(format!(".project.json unavailable: {e}")))?;
    let declaration: Value = serde_json::from_slice(&declaration_bytes)
        .map_err(|err| invalid(format!(".project.json is invalid JSON: {err}")))?;
    if !declaration.is_object() {
        return Err(invalid(".project.json must be an object"));
    }
    let manifest_path = root.join("forge.yaml");
    let manifest_bytes = optional_bounded(&manifest_path, &root)?;
    let manifest = manifest_bytes
        .as_ref()
        .map(|bytes| {
            serde_yaml::from_slice::<Value>(bytes)
                .map_err(|err| invalid(format!("forge.yaml is invalid: {err}")))
        })
        .transpose()?;
    let readme_bytes = optional_bounded(&root.join("README.md"), &root)?;
    let specs_root = root.join("openspec/specs");
    let mut spec_paths = Vec::new();
    if path_present(&specs_root)? {
        ensure_within(&specs_root, &root)?;
        let dirs = fs::read_dir(&specs_root)
            .map_err(|err| invalid(format!("cannot read openspec/specs: {err}")))?;
        let mut entries_seen = 0usize;
        for entry in dirs {
            let entry =
                entry.map_err(|err| invalid(format!("cannot read specification entry: {err}")))?;
            entries_seen += 1;
            if entries_seen > MAX_SPEC_FILES {
                return Err(invalid("openspec/specs contains too many entries"));
            }
            if entry
                .file_type()
                .map_err(|err| invalid(err.to_string()))?
                .is_dir()
            {
                let spec = entry.path().join("spec.md");
                if path_present(&spec)? {
                    ensure_within(&spec, &root)?;
                    if !fs::metadata(&spec)
                        .map_err(|err| {
                            invalid(format!("cannot inspect specification file: {err}"))
                        })?
                        .is_file()
                    {
                        return Err(invalid("specification path is not a regular file"));
                    }
                    let spec_id = entry.file_name().to_string_lossy().into_owned();
                    if spec_paths.len() == MAX_SPEC_FILES {
                        return Err(invalid("openspec/specs contains too many entries"));
                    }
                    spec_paths.push(spec_id);
                }
            }
        }
    }
    spec_paths.sort();
    Ok(Sources {
        declaration_bytes,
        declaration,
        manifest_bytes,
        manifest,
        readme_bytes,
        spec_paths,
    })
}

fn read_registered_metadata(record: &ProjectRecord) -> Result<ProjectMetadata, String> {
    let root = Path::new(&record.path)
        .canonicalize()
        .map_err(|_| "metadata-project-unavailable".to_owned())?;
    let metadata_path = root.join(METADATA_PATH);
    let receipt_path = root.join(RECEIPT_PATH);
    let bytes = read_bounded(&metadata_path, &root).map_err(|_| "metadata-missing".to_owned())?;
    let receipt_bytes =
        read_bounded(&receipt_path, &root).map_err(|_| "metadata-receipt-invalid".to_owned())?;
    let receipt: OwnershipReceipt = serde_json::from_slice(&receipt_bytes)
        .map_err(|_| "metadata-receipt-invalid".to_owned())?;
    if receipt.schema_version != 1
        || receipt.generator != GENERATOR_ID
        || receipt.project_id != record.id
        || receipt.artifact_path != METADATA_PATH
        || receipt.artifact_sha256 != sha256_hex(&bytes)
    {
        return Err("metadata-receipt-invalid".into());
    }
    let metadata: ProjectMetadata =
        serde_json::from_slice(&bytes).map_err(|_| "metadata-invalid".to_owned())?;
    if metadata.project_id != record.id || receipt.source_revision != metadata.source_revision {
        return Err("metadata-receipt-invalid".into());
    }
    Ok(metadata)
}

fn match_score(metadata: &ProjectMetadata, terms: &[String]) -> Option<(usize, Vec<String>)> {
    let fields: Vec<(&str, &str)> = vec![("project_id", metadata.project_id.as_str())]
        .into_iter()
        .chain(metadata.name.as_deref().map(|v| ("name", v)))
        .chain(metadata.description.as_deref().map(|v| ("description", v)))
        .chain(metadata.keywords.iter().map(|v| ("keywords", v.as_str())))
        .chain(metadata.topics.iter().map(|v| ("topics", v.as_str())))
        .chain(metadata.audience.iter().map(|v| ("audience", v.as_str())))
        .chain(metadata.use_cases.iter().map(|v| ("use_cases", v.as_str())))
        .chain(
            metadata
                .search_phrases
                .iter()
                .map(|v| ("search_phrases", v.as_str())),
        )
        .chain(
            metadata
                .capabilities
                .iter()
                .map(|v| ("capabilities", v.id.as_str())),
        )
        .chain(
            metadata
                .interfaces
                .iter()
                .map(|v| ("interfaces", v.id.as_str())),
        )
        .chain(
            metadata
                .integration_ids
                .iter()
                .map(|v| ("integrations", v.as_str())),
        )
        .chain(
            metadata
                .specifications
                .iter()
                .map(|v| ("specifications", v.as_str())),
        )
        .collect();
    let mut matched = BTreeSet::new();
    for term in terms {
        let current: BTreeSet<&str> = fields
            .iter()
            .filter_map(|(name, field)| field.to_lowercase().contains(term).then_some(*name))
            .collect();
        if current.is_empty() {
            return None;
        }
        matched.extend(current);
    }
    let fields = matched.into_iter().map(str::to_owned).collect::<Vec<_>>();
    Some((fields.len(), fields))
}

fn replace_pair(
    first: &Path,
    first_bytes: &[u8],
    second: &Path,
    second_bytes: &[u8],
    replacing: bool,
) -> Result<(), ForgeError> {
    let nonce = format!(
        "{}.{}.tmp",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let first_tmp = first.with_extension(format!("json.{nonce}"));
    let second_tmp = second.with_extension(format!("json.{nonce}"));
    write_staged(&first_tmp, first_bytes)
        .map_err(|err| invalid(format!("cannot stage metadata: {err}")))?;
    if let Err(err) = write_staged(&second_tmp, second_bytes) {
        let _ = fs::remove_file(&first_tmp);
        return Err(invalid(format!("cannot stage metadata receipt: {err}")));
    }
    let first_backup = first.with_extension(format!("json.{nonce}.bak"));
    let second_backup = second.with_extension(format!("json.{nonce}.bak"));
    if replacing {
        if let Err(err) = fs::rename(first, &first_backup) {
            let _ = fs::remove_file(&first_tmp);
            let _ = fs::remove_file(&second_tmp);
            return Err(invalid(format!("cannot preserve owned metadata: {err}")));
        }
        if let Err(err) = fs::rename(second, &second_backup) {
            let restore =
                fs::hard_link(&first_backup, first).and_then(|_| fs::remove_file(&first_backup));
            let _ = fs::remove_file(&first_tmp);
            let _ = fs::remove_file(&second_tmp);
            return Err(invalid(match restore {
                Ok(()) => format!("cannot preserve metadata receipt: {err}"),
                Err(rollback) => {
                    format!("cannot preserve metadata receipt: {err}; rollback failed: {rollback}")
                }
            }));
        }
    }
    if let Err(err) = fs::hard_link(&first_tmp, first) {
        let rollback = rollback_pair(
            first,
            second,
            &first_backup,
            &second_backup,
            replacing,
            false,
            false,
        );
        let _ = fs::remove_file(&first_tmp);
        let _ = fs::remove_file(&second_tmp);
        return Err(invalid(rollback.map_or_else(
            |rollback| format!("cannot promote metadata: {err}; rollback failed: {rollback}"),
            |_| format!("cannot promote metadata: {err}"),
        )));
    }
    if let Err(err) = fs::remove_file(&first_tmp) {
        let rollback = rollback_pair(
            first,
            second,
            &first_backup,
            &second_backup,
            replacing,
            true,
            false,
        );
        let _ = fs::remove_file(&second_tmp);
        return Err(invalid(rollback.map_or_else(
            |rollback| format!("cannot finalize metadata: {err}; rollback failed: {rollback}"),
            |_| format!("cannot finalize metadata: {err}"),
        )));
    }
    if let Err(err) = fs::hard_link(&second_tmp, second) {
        let rollback = rollback_pair(
            first,
            second,
            &first_backup,
            &second_backup,
            replacing,
            true,
            false,
        );
        let _ = fs::remove_file(&second_tmp);
        return Err(invalid(rollback.map_or_else(
            |rollback| {
                format!("cannot promote metadata receipt: {err}; rollback failed: {rollback}")
            },
            |_| format!("cannot promote metadata receipt: {err}"),
        )));
    }
    if let Err(err) = fs::remove_file(&second_tmp) {
        let rollback = rollback_pair(
            first,
            second,
            &first_backup,
            &second_backup,
            replacing,
            true,
            true,
        );
        return Err(invalid(rollback.map_or_else(
            |rollback| {
                format!("cannot finalize metadata receipt: {err}; rollback failed: {rollback}")
            },
            |_| format!("cannot finalize metadata receipt: {err}"),
        )));
    }
    if replacing {
        let _ = fs::remove_file(first_backup);
        let _ = fs::remove_file(second_backup);
    }
    Ok(())
}

fn write_staged(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    if let Err(err) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(path);
        return Err(err);
    }
    Ok(())
}

fn rollback_pair(
    first: &Path,
    second: &Path,
    first_backup: &Path,
    second_backup: &Path,
    replacing: bool,
    first_promoted: bool,
    second_promoted: bool,
) -> Result<(), std::io::Error> {
    if first_promoted {
        remove_if_present(first)?;
    }
    if second_promoted {
        remove_if_present(second)?;
    }
    if replacing {
        fs::hard_link(first_backup, first)?;
        fs::remove_file(first_backup)?;
        fs::hard_link(second_backup, second)?;
        fs::remove_file(second_backup)?;
    }
    Ok(())
}

fn remove_if_present(path: &Path) -> Result<(), std::io::Error> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn read_bounded(path: &Path, root: &Path) -> Result<Vec<u8>, ForgeError> {
    ensure_within(path, root)?;
    let mut file =
        fs::File::open(path).map_err(|err| invalid(format!("cannot read source file: {err}")))?;
    let opened = file
        .metadata()
        .map_err(|err| invalid(format!("cannot inspect open source file: {err}")))?;
    let current =
        fs::metadata(path).map_err(|err| invalid(format!("cannot inspect source path: {err}")))?;
    if !opened.is_file() || opened.len() > MAX_SOURCE_BYTES || !same_file(&opened, &current) {
        return Err(invalid(
            "source file is not regular, exceeds 1 MiB, or changed while opening",
        ));
    }
    ensure_within(path, root)?;
    let mut bytes = Vec::with_capacity(opened.len() as usize);
    Read::take(&mut file, MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| invalid(format!("cannot read source file: {err}")))?;
    if bytes.len() as u64 > MAX_SOURCE_BYTES {
        return Err(invalid("source file exceeds 1 MiB"));
    }
    Ok(bytes)
}

#[cfg(unix)]
fn same_file(first: &fs::Metadata, second: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    first.dev() == second.dev() && first.ino() == second.ino()
}

#[cfg(not(unix))]
fn same_file(first: &fs::Metadata, second: &fs::Metadata) -> bool {
    first.len() == second.len() && first.modified().ok() == second.modified().ok()
}

fn optional_bounded(path: &Path, root: &Path) -> Result<Option<Vec<u8>>, ForgeError> {
    if !path_present(path)? {
        return Ok(None);
    }
    read_bounded(path, root).map(Some)
}

fn ensure_within(path: &Path, root: &Path) -> Result<(), ForgeError> {
    let resolved = path
        .canonicalize()
        .map_err(|err| invalid(format!("cannot resolve source path: {err}")))?;
    if !resolved.starts_with(root)
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(invalid("source path escapes project root"));
    }
    Ok(())
}
fn path_present(path: &Path) -> Result<bool, ForgeError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(invalid(format!("cannot inspect path: {err}"))),
    }
}

fn declaration_string(declaration: &Value, key: &str) -> Option<String> {
    declaration.get(key).and_then(Value::as_str).map(redact)
}

fn redact(value: &str) -> String {
    crate::policy::redact_credentials(value)
        .chars()
        .take(2048)
        .collect()
}

fn sort_dedup(values: &mut Vec<String>) {
    values.sort_by(|a, b| {
        a.to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b))
    });
    values.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
}

fn invalid(reason: impl Into<String>) -> ForgeError {
    ForgeError::ProjectMetadataInvalid {
        reason: reason.into(),
    }
}

fn conflict(reason: impl Into<String>) -> ForgeError {
    ForgeError::ProjectMetadataConflict {
        reason: reason.into(),
    }
}
