//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::policy::redact_credentials;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use super::contract::RELEASE_STATE_DIR;
use super::model::{
    CapturedChangelog, ReleaseConfig, ReleaseIdentity, ReleaseReport, ReleaseState,
};

pub(super) fn parse_semver_component(raw: &str, kind: &str, full: &str) -> Result<u64, ForgeError> {
    if raw.is_empty() {
        return Err(ForgeError::ReleaseInvalid {
            reason: format!("version `{full}` is missing the {kind} component"),
        });
    }
    if !raw.chars().all(|c| c.is_ascii_digit()) {
        return Err(ForgeError::ReleaseInvalid {
            reason: format!("version `{full}` {kind} component is not numeric"),
        });
    }
    // No leading zeros except literal `0`.
    if raw.len() > 1 && raw.starts_with('0') {
        return Err(ForgeError::ReleaseInvalid {
            reason: format!("version `{full}` {kind} component has a leading zero"),
        });
    }
    raw.parse::<u64>().map_err(|_| ForgeError::ReleaseInvalid {
        reason: format!("version `{full}` {kind} component overflows u64"),
    })
}

/// Hex SHA-256 over bytes. Used for release identity hashing
/// and adapter contract signatures.
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Build a [`ReleaseConfig`] from a manifest's release section.
/// Convenience for callers that already have the manifest.
pub fn release_config_from_manifest(manifest: &Manifest) -> Result<ReleaseConfig, ForgeError> {
    match manifest.release.as_ref() {
        Some(meta) => ReleaseConfig::from_manifest_meta(meta),
        None => Err(ForgeError::ReleaseInvalid {
            reason: format!(
                "project `{}` has no `release` section; declare one with at least `versioning: semver`",
                manifest.project.id
            ),
        }),
    }
}

/// Compute the on-disk state path for one release. The path
/// is lexically scoped to the project directory, the project
/// id and the release identity so two projects (or two
/// release attempts on the same project) can never share a
/// state file.
pub fn state_path_for(
    project_dir: &Path,
    project_id: &str,
    identity: &ReleaseIdentity,
) -> Result<PathBuf, ForgeError> {
    if project_id.trim().is_empty() {
        return Err(ForgeError::ReleaseInvalid {
            reason: "project id is required to resolve the release state path".to_string(),
        });
    }
    if identity.id.trim().is_empty() {
        return Err(ForgeError::ReleaseInvalid {
            reason: "release id is required to resolve the release state path".to_string(),
        });
    }
    Ok(project_dir
        .join(RELEASE_STATE_DIR)
        .join(project_id)
        .join(&identity.id)
        .join("state.json"))
}

pub fn load_release_state(path: &Path) -> Result<ReleaseState, ForgeError> {
    if !path.exists() {
        return Ok(ReleaseState::default());
    }
    let bytes = fs::read(path).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!("cannot read release state {}: {err}", path.display()),
    })?;
    if bytes.is_empty() {
        return Ok(ReleaseState::default());
    }
    serde_json::from_slice(&bytes).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!(
            "release state at {} is not valid JSON: {err}",
            path.display()
        ),
    })
}

pub fn save_release_state(path: &Path, state: &ReleaseState) -> Result<(), ForgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::ReleaseInvalid {
            reason: format!(
                "cannot create release state directory {}: {err}",
                parent.display()
            ),
        })?;
    }
    let bytes = serde_json::to_vec_pretty(state).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!("cannot serialize release state: {err}"),
    })?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!("cannot write release state tmp {}: {err}", tmp.display()),
    })?;
    fs::rename(&tmp, path).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!("cannot rename release state {}: {err}", path.display()),
    })?;
    Ok(())
}

/// Redact credential-like substrings from a piece of evidence.
/// The redaction is delegated to
/// [`crate::policy::redact_credentials`] so the release,
/// policy and distribution contracts share one definition
/// of "secret".
pub fn redact_release_evidence(text: &str) -> String {
    redact_credentials(text)
}

/// Load a file from `project_dir` and return its content
/// hash plus a bounded excerpt. The full content is not
/// embedded in the report so the report stays small even
/// for large changelogs.
pub fn load_changelog(project_dir: &Path, relative: &str) -> Result<CapturedChangelog, ForgeError> {
    if !lexically_inside(".", relative) {
        return Err(ForgeError::ReleaseInvalid {
            reason: format!("changelog `{relative}` resolves outside the project"),
        });
    }
    let path = project_dir.join(relative);
    if !path.is_file() {
        return Err(ForgeError::ReleaseInvalid {
            reason: format!(
                "changelog file `{}` does not exist; release requires a present changelog",
                path.display()
            ),
        });
    }
    let bytes = fs::read(&path).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!("cannot read changelog {}: {err}", path.display()),
    })?;
    let content_hash = hash_bytes(&bytes);
    let text = String::from_utf8_lossy(&bytes);
    let excerpt = excerpt_first_lines(&text, 40);
    Ok(CapturedChangelog {
        path: relative.to_string(),
        content_hash,
        excerpt,
    })
}

fn excerpt_first_lines(text: &str, max_lines: usize) -> String {
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        if i >= max_lines {
            out.push_str("…\n");
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Capture the current working-tree revision. A non-git
/// project is refused before any release side effect runs.
pub fn capture_source_revision(dir: &Path) -> Result<String, ForgeError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .map_err(|err| ForgeError::ReleaseInvalid {
            reason: format!("git rev-parse failed: {err}"),
        })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(ForgeError::ReleaseInvalid {
            reason: format!(
                "directory `{}` is not a git working tree with a HEAD commit: {}",
                dir.display(),
                stderr.trim()
            ),
        });
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sha.is_empty() {
        return Err(ForgeError::ReleaseInvalid {
            reason: "git rev-parse returned an empty SHA".to_string(),
        });
    }
    Ok(sha)
}

/// Render a [`ReleaseReport`] for human output. The transport
/// renders the same data the JSON envelope carries so a
/// partial run is observable on stdout.
pub fn render_report_human(report: &ReleaseReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("project: {}", report.project_id));
    lines.push(format!("release_id: {}", report.identity.id));
    lines.push(format!("version: {}", report.identity.version));
    lines.push(format!(
        "source_revision: {}",
        report.identity.source_revision
    ));
    if let Some(changelog) = &report.changelog {
        lines.push(format!("changelog: {}", changelog.path));
        lines.push(format!("changelog_hash: {}", changelog.content_hash));
    }
    if !report.docs_locales.is_empty() {
        lines.push(format!("docs_locales: {}", report.docs_locales.join(", ")));
    }
    lines.push(format!("stages: {}", report.stages.join(", ")));
    lines.push(format!("mode: {}", release_mode_label(report)));
    lines.push(format!("state: {}", report.state_path));
    if !report.checks.is_empty() {
        lines.push("checks:".to_string());
        for check in &report.checks {
            lines.push(format!(
                "  - {kind} {status} applicable={applicable} revision={revision}: {detail}",
                kind = check.kind,
                status = check.status,
                applicable = check.applicable,
                revision = check.source_revision,
                detail = check.detail
            ));
        }
    }
    if !report.stage_outcomes.is_empty() {
        lines.push("stage_outcomes:".to_string());
        for outcome in &report.stage_outcomes {
            lines.push(format!(
                "  - {stage} target=`{target}` {status}: {note}",
                stage = outcome.stage,
                target = outcome.target,
                status = outcome.status,
                note = outcome.note
            ));
            for line in &outcome.evidence {
                lines.push(format!("      evidence: {line}"));
            }
            for line in &outcome.recovery {
                lines.push(format!("      recovery: {line}"));
            }
        }
    }
    lines.push(format!("summary: {}", report.note));
    lines.join("\n")
}

fn release_mode_label(report: &ReleaseReport) -> String {
    if report.dry_run {
        "dry-run".to_string()
    } else if report.retry {
        "retry".to_string()
    } else {
        "apply".to_string()
    }
}

/// Lexically confine `rel` to `project_dir` without touching
/// the filesystem. Absolute paths must already sit inside
/// the project, and `..` segments must never escape the
/// root. Symlinks are rechecked canonically at write time.
pub(super) fn lexically_inside(_project_dir: &str, rel: &str) -> bool {
    let candidate = Path::new(rel);
    if candidate.is_absolute() {
        return false;
    }
    let mut depth = 0i32;
    for component in candidate.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => return false,
            Component::CurDir => {}
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            Component::Normal(_) => {
                depth += 1;
            }
        }
    }
    true
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::core::manifest::ReleaseMeta;
    use crate::release::contract::DEFAULT_STAGES;
    use crate::release::model::Semver;
    use std::fs;
    use tempfile::TempDir;

    fn meta_with(text: &str) -> ReleaseMeta {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            format!(
                "schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\nrelease:\n{text}"
            )
            .as_bytes(),
        )
        .expect("manifest");
        manifest.release.expect("release section")
    }

    #[test]
    fn semver_parses_full_triple() {
        let v = Semver::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.label(), "1.2.3");
    }

    #[test]
    fn semver_parses_prerelease() {
        let v = Semver::parse("1.2.3-rc.1").unwrap();
        assert_eq!(v.label(), "1.2.3-rc.1");
    }

    #[test]
    fn semver_rejects_non_triple() {
        let err = Semver::parse("1.2").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
        let err = Semver::parse("1.2.3.4").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
        let err = Semver::parse("").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn semver_rejects_non_numeric_components() {
        let err = Semver::parse("1.x.3").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
        let err = Semver::parse("01.0.0").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn semver_rejects_unsupported_versioning() {
        let meta = meta_with("  versioning: calver\n");
        let err = ReleaseConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn release_config_defaults_to_eight_stages() {
        let meta = meta_with("  versioning: semver\n");
        let cfg = ReleaseConfig::from_manifest_meta(&meta).unwrap();
        let expected: Vec<String> = DEFAULT_STAGES.iter().map(|s| s.to_string()).collect();
        assert_eq!(cfg.stages, expected);
    }

    #[test]
    fn release_config_rejects_unknown_check_kind() {
        let meta = meta_with("  versioning: semver\n  checks:\n    - kind: lint\n");
        let err = ReleaseConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn release_config_accepts_known_checks() {
        let meta = meta_with(
            "  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: test\n    - kind: driftwatch\n",
        );
        let cfg = ReleaseConfig::from_manifest_meta(&meta).unwrap();
        let kinds: Vec<&str> = cfg.checks.iter().map(|c| c.kind.as_str()).collect();
        assert_eq!(kinds, vec!["doctor", "test", "driftwatch"]);
    }

    #[test]
    fn release_config_rejects_package_path_outside_project() {
        let meta =
            meta_with("  versioning: semver\n  packages:\n    - name: x\n      path: ../evil\n");
        let err = ReleaseConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn release_config_rejects_duplicate_check_kind() {
        let meta =
            meta_with("  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: doctor\n");
        let err = ReleaseConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn release_identity_is_stable_for_same_inputs() {
        let v = Semver::parse("1.2.3").unwrap();
        let a = ReleaseIdentity::derive("app", &v, "deadbeefcafe");
        let b = ReleaseIdentity::derive("app", &v, "deadbeefcafe");
        assert_eq!(a, b);
        let c = ReleaseIdentity::derive("app", &v, "different");
        assert_ne!(a, c);
    }

    #[test]
    fn load_changelog_reads_present_file() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("CHANGELOG.md"),
            "## 1.0.0\n- initial release\n",
        )
        .unwrap();
        let captured = load_changelog(tmp.path(), "CHANGELOG.md").unwrap();
        assert_eq!(captured.path, "CHANGELOG.md");
        assert!(captured.excerpt.contains("1.0.0"));
    }

    #[test]
    fn load_changelog_refuses_missing_file() {
        let tmp = TempDir::new().unwrap();
        let err = load_changelog(tmp.path(), "CHANGELOG.md").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn load_changelog_refuses_outside_project_path() {
        let err = load_changelog(Path::new("."), "../etc/passwd").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }
}
