//! Verification and normalization for GitHub push deliveries.

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubPushEvent {
    pub delivery_id: String,
    pub repository: String,
    pub git_ref: String,
    pub after: String,
    pub signature: String,
    pub body: Vec<u8>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitHubPushError {
    #[error("GitHub delivery id is required")]
    MissingDelivery,
    #[error("GitHub repository is not allowed")]
    RepositoryMismatch,
    #[error("GitHub ref is not allowed")]
    RefMismatch,
    #[error("GitHub push must contain a full 40-character commit SHA")]
    InvalidRevision,
    #[error("GitHub signature is invalid")]
    InvalidSignature,
}

pub fn verify_push(
    event: &GitHubPushEvent,
    secret: &[u8],
    allowed_repository: &str,
    allowed_ref: &str,
) -> Result<(), GitHubPushError> {
    if event.delivery_id.trim().is_empty() {
        return Err(GitHubPushError::MissingDelivery);
    }
    if event.repository != allowed_repository {
        return Err(GitHubPushError::RepositoryMismatch);
    }
    if event.git_ref != allowed_ref {
        return Err(GitHubPushError::RefMismatch);
    }
    if event.after.len() != 40 || !event.after.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GitHubPushError::InvalidRevision);
    }
    let Some(signature) = event.signature.strip_prefix("sha256=") else {
        return Err(GitHubPushError::InvalidSignature);
    };
    let expected = hmac_sha256_hex(secret, &event.body);
    if !constant_time_equal(signature.as_bytes(), expected.as_bytes()) {
        return Err(GitHubPushError::InvalidSignature);
    }
    Ok(())
}

fn hmac_sha256_hex(secret: &[u8], body: &[u8]) -> String {
    let mut key = [0u8; 64];
    if secret.len() > 64 {
        let mut hash = Sha256::new();
        hash.update(secret);
        key[..32].copy_from_slice(&hash.finalize());
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let mut inner = [0x36u8; 64];
    let mut outer = [0x5cu8; 64];
    for index in 0..64 {
        inner[index] ^= key[index];
        outer[index] ^= key[index];
    }
    let mut inner_hash = Sha256::new();
    inner_hash.update(inner);
    inner_hash.update(body);
    let inner_digest = inner_hash.finalize();
    let mut outer_hash = Sha256::new();
    outer_hash.update(outer);
    outer_hash.update(inner_digest);
    format!("{:x}", outer_hash.finalize())
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(signature: &str) -> GitHubPushEvent {
        GitHubPushEvent {
            delivery_id: "delivery-1".to_string(),
            repository: "acme/example".to_string(),
            git_ref: "refs/heads/main".to_string(),
            after: "0123456789abcdef0123456789abcdef01234567".to_string(),
            signature: signature.to_string(),
            body: br#"{"ref":"refs/heads/main"}"#.to_vec(),
        }
    }

    #[test]
    fn accepts_valid_signature_and_boundaries() {
        let body = br#"{"ref":"refs/heads/main"}"#;
        let signature = hmac_sha256_hex(b"secret", body);
        let mut push = event(&format!("sha256={signature}"));
        push.body = body.to_vec();
        assert!(verify_push(&push, b"secret", "acme/example", "refs/heads/main").is_ok());
    }

    #[test]
    fn rejects_invalid_signature_repository_ref_and_short_sha() {
        let mut push = event("sha256=bad");
        assert_eq!(
            verify_push(&push, b"secret", "acme/example", "refs/heads/main"),
            Err(GitHubPushError::InvalidSignature)
        );
        let signature = hmac_sha256_hex(b"secret", &push.body);
        push.signature = format!("sha256={signature}");
        push.repository = "other/repo".to_string();
        assert_eq!(
            verify_push(&push, b"secret", "acme/example", "refs/heads/main"),
            Err(GitHubPushError::RepositoryMismatch)
        );
        push.repository = "acme/example".to_string();
        push.after = "abc".to_string();
        assert_eq!(
            verify_push(&push, b"secret", "acme/example", "refs/heads/main"),
            Err(GitHubPushError::InvalidRevision)
        );
    }
}
