//! How an approved manifest reaches a target.
//!
//! A publisher receives the manifest and nothing else — no query, no
//! registry path, no credential minted by Forge — so an adapter can
//! never turn a publication into a read of private state. The default
//! local export keeps Forge useful with no external access at all.

use serde::{Deserialize, Serialize};

use crate::core::ForgeError;
use crate::portfolio::share::validation::looks_like_secret;
use crate::portfolio::share::{
    PublicationStatus, MAX_MANIFEST_BYTES, PUBLISH_TIMEOUT_SECS, SHARE_ADAPTER_CONTRACT,
};

// --- publisher ----------------------------------------------------------

/// Everything a publisher is told about one publication. A publisher
/// receives the manifest and this context; it never receives a query,
/// a path into the registry, or a credential minted by Forge.
#[derive(Debug, Clone)]
pub struct PublishContext {
    pub operation_key: String,
    pub manifest_revision: i64,
    pub manifest_sha256: String,
    pub target: String,
}

/// What a publisher reports back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishOutcome {
    pub status: PublicationStatus,
    pub published_revision: Option<String>,
    pub error_code: Option<String>,
    /// True when the target already held exactly these bytes, so the
    /// publication was reconciled rather than repeated.
    pub already_present: bool,
}

impl PublishOutcome {
    pub fn published(published_revision: Option<String>) -> Self {
        Self {
            status: PublicationStatus::Published,
            published_revision,
            error_code: None,
            already_present: false,
        }
    }

    pub fn failed(error_code: impl Into<String>) -> Self {
        Self {
            status: PublicationStatus::Failed,
            published_revision: None,
            error_code: Some(error_code.into()),
            already_present: false,
        }
    }
}

/// One publication target. Implementations must be safe to call twice
/// for the same manifest: the caller guarantees a single audit row
/// per operation key, and the implementation must not produce a
/// second public artifact for a retry.
pub trait SharePublisher {
    /// Stable identifier recorded in the audit trail.
    fn label(&self) -> String;
    fn publish(
        &self,
        document: &str,
        context: &PublishContext,
    ) -> Result<PublishOutcome, crate::core::ForgeError>;
}

/// The default publisher: writes the approved document to a local
/// path. Forge stays useful with no GitHub access at all, and the
/// local export is the same bytes a consumer would receive.
#[derive(Debug, Clone)]
pub struct LocalFilePublisher {
    path: std::path::PathBuf,
    max_bytes: usize,
}

impl LocalFilePublisher {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self {
            path,
            max_bytes: MAX_MANIFEST_BYTES,
        }
    }

    /// The artifact path this publisher writes.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// True when the target already holds exactly these bytes. This is
    /// what makes a retry after a timed-out publication reconcile
    /// instead of writing a second time.
    pub fn already_published(&self, document: &str) -> bool {
        std::fs::read_to_string(&self.path).is_ok_and(|current| current == document)
    }
}

impl SharePublisher for LocalFilePublisher {
    fn label(&self) -> String {
        format!("local-file:{}", self.path.display())
    }

    fn publish(
        &self,
        document: &str,
        context: &PublishContext,
    ) -> Result<PublishOutcome, ForgeError> {
        if document.len() > self.max_bytes {
            return Err(ForgeError::PortfolioShareInvalid {
                reason: format!(
                    "manifest is larger than {} bytes; refusing to write a public artifact \
                     that exceeds the export bound",
                    self.max_bytes
                ),
            });
        }
        if self.already_published(document) {
            return Ok(PublishOutcome {
                status: PublicationStatus::Published,
                published_revision: None,
                error_code: None,
                already_present: true,
            });
        }
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|err| {
                    ForgeError::PortfolioShareInvalid {
                        reason: format!(
                            "publication target directory {} could not be created: {err}",
                            parent.display()
                        ),
                    }
                })?;
            }
        }
        // Write-then-rename: a reader never observes a half-written
        // public manifest, and a crash leaves the previous artifact
        // intact.
        let staging = self.path.with_extension(format!(
            "publishing-{}",
            sanitize_path_fragment(&context.operation_key)
        ));
        std::fs::write(&staging, document.as_bytes()).map_err(|err| {
            let _ = std::fs::remove_file(&staging);
            ForgeError::PortfolioShareInvalid {
                reason: format!(
                    "manifest could not be staged at {}: {err}",
                    staging.display()
                ),
            }
        })?;
        std::fs::rename(&staging, &self.path).map_err(|err| {
            let _ = std::fs::remove_file(&staging);
            ForgeError::PortfolioShareInvalid {
                reason: format!(
                    "manifest could not be moved into place at {}: {err}",
                    self.path.display()
                ),
            }
        })?;
        Ok(PublishOutcome::published(Some(
            context.manifest_sha256.clone(),
        )))
    }
}

/// The optional external adapter: a credential-injected executable
/// that receives the manifest on stdin and answers with one bounded
/// JSON response. Credentials belong to the deployment, are injected
/// as environment variables by the caller, and never appear in the
/// manifest, the request or the audit trail.
#[derive(Debug, Clone)]
pub struct SubprocessPublisher {
    command: std::path::PathBuf,
    env: Vec<(String, String)>,
    timeout: std::time::Duration,
}

/// Request envelope written to the adapter's stdin.
#[derive(Debug, Clone, Serialize)]
pub struct AdapterRequest {
    pub contract: String,
    pub operation_key: String,
    pub manifest_revision: i64,
    pub manifest_sha256: String,
    pub target: String,
    pub document: String,
}

/// Response envelope read from the adapter's stdout.
#[derive(Debug, Clone, Deserialize)]
pub struct AdapterResponse {
    pub contract: String,
    pub status: String,
    #[serde(default)]
    pub published_revision: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
}

/// Bound one adapter attempt. `FORGE_PORTFOLIO_SHARE_TIMEOUT_SECS`
/// overrides the default within `1..=3600` so a hostile or wedged
/// adapter cannot pin the CLI.
pub fn publish_timeout() -> std::time::Duration {
    let secs = std::env::var("FORGE_PORTFOLIO_SHARE_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(PUBLISH_TIMEOUT_SECS)
        .clamp(1, 3600);
    std::time::Duration::from_secs(secs)
}

impl SubprocessPublisher {
    pub fn new(command: std::path::PathBuf) -> Self {
        Self {
            command,
            env: Vec::new(),
            timeout: publish_timeout(),
        }
    }

    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_string(), value.to_string()));
        self
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl SharePublisher for SubprocessPublisher {
    fn label(&self) -> String {
        format!("adapter:{}", self.command.display())
    }

    fn publish(
        &self,
        document: &str,
        context: &PublishContext,
    ) -> Result<PublishOutcome, ForgeError> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let request = AdapterRequest {
            contract: SHARE_ADAPTER_CONTRACT.to_string(),
            operation_key: context.operation_key.clone(),
            manifest_revision: context.manifest_revision,
            manifest_sha256: context.manifest_sha256.clone(),
            target: context.target.clone(),
            document: document.to_string(),
        };
        let payload =
            serde_json::to_string(&request).map_err(|err| ForgeError::PortfolioShareInvalid {
                reason: format!("manifest could not be serialized for the adapter: {err}"),
            })?;
        let mut child = Command::new(&self.command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| ForgeError::PortfolioShareInvalid {
                reason: format!(
                    "publication adapter {} could not be started: {err}",
                    self.command.display()
                ),
            })?;
        {
            // `take` closes the pipe once the request is written: an
            // adapter reading to EOF must not wait on a handle Forge
            // still holds.
            let mut stdin =
                child
                    .stdin
                    .take()
                    .ok_or_else(|| ForgeError::PortfolioShareInvalid {
                        reason: "publication adapter has no stdin".to_string(),
                    })?;
            // A broken pipe here means the adapter exited early; its
            // own status is reported below rather than guessed at.
            let _ = stdin.write_all(payload.as_bytes());
        }
        let deadline = std::time::Instant::now() + self.timeout;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {
                    if std::time::Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(ForgeError::PortfolioShareInvalid {
                            reason: format!(
                                "publication adapter {} exceeded its {}s budget; \
                                 the approved manifest is retained and the attempt is retryable \
                                 under the same operation key",
                                self.command.display(),
                                self.timeout.as_secs()
                            ),
                        });
                    }
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
                Err(err) => {
                    let _ = child.kill();
                    return Err(ForgeError::PortfolioShareInvalid {
                        reason: format!(
                            "publication adapter {} could not be waited on: {err}",
                            self.command.display()
                        ),
                    });
                }
            }
        }
        let output = child
            .wait_with_output()
            .map_err(|err| ForgeError::PortfolioShareInvalid {
                reason: format!(
                    "publication adapter {} produced no readable response: {err}",
                    self.command.display()
                ),
            })?;
        let raw = String::from_utf8_lossy(&output.stdout).to_string();
        if looks_like_secret(&raw) {
            return Err(ForgeError::PortfolioShareInvalid {
                reason: "publication adapter response carried a credential-shaped value; \
                         it was discarded without being recorded"
                    .to_string(),
            });
        }
        if raw.len() > 64 * 1024 {
            return Err(ForgeError::PortfolioShareInvalid {
                reason: "publication adapter response is larger than 65536 bytes".to_string(),
            });
        }
        let response: AdapterResponse =
            serde_json::from_str(&raw).map_err(|err| ForgeError::PortfolioShareInvalid {
                reason: format!(
                    "publication adapter {} did not answer the {} contract: {err}",
                    self.command.display(),
                    SHARE_ADAPTER_CONTRACT
                ),
            })?;
        if response.contract != SHARE_ADAPTER_CONTRACT {
            return Err(ForgeError::PortfolioShareInvalid {
                reason: format!(
                    "publication adapter answered contract `{}`; expected `{SHARE_ADAPTER_CONTRACT}`",
                    response.contract
                ),
            });
        }
        let status = PublicationStatus::parse(&response.status).map_err(|err| {
            ForgeError::PortfolioShareInvalid {
                reason: format!("publication adapter reported an unusable status: {err}"),
            }
        })?;
        Ok(PublishOutcome {
            status,
            published_revision: response.published_revision,
            error_code: response.error_code,
            already_present: false,
        })
    }
}

/// Reduce an operation key to something safe inside a file name.
fn sanitize_path_fragment(raw: &str) -> String {
    let out: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '-'
            }
        })
        .take(64)
        .collect();
    if out.is_empty() {
        "publication".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_local_publisher_is_idempotent_for_identical_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested/portfolio-manifest.json");
        let publisher = LocalFilePublisher::new(path.clone());
        let context = PublishContext {
            operation_key: "op-1".to_string(),
            manifest_revision: 1,
            manifest_sha256: "abc".to_string(),
            target: "github-pages".to_string(),
        };
        let first = publisher.publish("{\"a\":1}\n", &context).expect("first");
        assert_eq!(first.status, PublicationStatus::Published);
        assert!(!first.already_present);
        assert!(path.exists());
        assert!(!path.with_extension("publishing-op-1").exists());
        let second = publisher.publish("{\"a\":1}\n", &context).expect("retry");
        assert!(second.already_present);
        assert_eq!(second.status, PublicationStatus::Published);
        assert!(publisher.already_published("{\"a\":1}\n"));
    }

    #[test]
    fn the_local_publisher_refuses_an_oversized_manifest() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("big.json");
        let mut publisher = LocalFilePublisher::new(path.clone());
        publisher.max_bytes = 16;
        let context = PublishContext {
            operation_key: "op-1".to_string(),
            manifest_revision: 1,
            manifest_sha256: "abc".to_string(),
            target: "t".to_string(),
        };
        let err = publisher
            .publish(&"x".repeat(64), &context)
            .expect_err("oversized");
        assert_eq!(err.code(), "portfolio-share-invalid");
        assert!(!path.exists());
    }

    /// A wedged adapter must not be able to pin Forge: it is killed at
    /// the budget and the attempt stays retryable.
    #[test]
    #[cfg(unix)]
    fn publish_timeout_is_bounded() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("forge-share-timeout-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tempdir");
        let script = dir.join("wedged.sh");
        std::fs::write(&script, "#!/bin/sh\nsleep 30\n").expect("write stub");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let context = PublishContext {
            operation_key: "op-1".to_string(),
            manifest_revision: 1,
            manifest_sha256: "abc".to_string(),
            target: "t".to_string(),
        };
        let publisher = SubprocessPublisher::new(script.clone())
            .with_timeout(std::time::Duration::from_millis(200));
        let err = publisher
            .publish("{}", &context)
            .expect_err("a wedged adapter must not be trusted");
        assert_eq!(err.code(), "portfolio-share-invalid");
        assert!(err.to_string().contains("budget"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_adapter_command_is_a_typed_refusal() {
        let publisher = SubprocessPublisher::new("/nonexistent/forge-share-adapter".into());
        let context = PublishContext {
            operation_key: "op-1".to_string(),
            manifest_revision: 1,
            manifest_sha256: "abc".to_string(),
            target: "t".to_string(),
        };
        let err = publisher.publish("{}", &context).expect_err("missing");
        assert_eq!(err.code(), "portfolio-share-invalid");
    }
}
