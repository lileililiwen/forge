//! Forge-wide operator credentials and browser sessions.
//!
//! This store is deliberately separate from project OIDC identities: a
//! Forge operator session authorizes the Forge portal across the registry,
//! while project sessions continue to authorize project-owned APIs.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use chrono::Utc;
use rand::Rng;
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::Path;

const SESSION_TTL_SECONDS: i64 = 12 * 60 * 60;

fn connection(db_path: &Path) -> Result<Connection, String> {
    crate::registry::Registry::open(db_path).map_err(|err| err.to_string())?;
    let db = Connection::open(db_path).map_err(|err| err.to_string())?;
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS forge_admin (
            singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
            email TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS forge_admin_sessions (
            token_hash TEXT PRIMARY KEY,
            created_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            revoked INTEGER NOT NULL DEFAULT 0
        );",
    )
    .map_err(|err| err.to_string())?;
    Ok(db)
}

pub fn setup(db_path: &Path, email: &str, password: &str) -> Result<(), String> {
    let email = email.trim().to_ascii_lowercase();
    if !email.contains('@') || email.len() > 254 || email.chars().any(char::is_whitespace) {
        return Err("email address is invalid".into());
    }
    if password.chars().count() < 12 || password.len() > 1024 {
        return Err("password must contain 12 to 1024 characters".into());
    }
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| "password hashing failed".to_string())?
        .to_string();
    let db = connection(db_path)?;
    db.execute(
        "INSERT INTO forge_admin(singleton, email, password_hash, created_at) VALUES (1, ?1, ?2, ?3)",
        params![email, password_hash, Utc::now().timestamp()],
    )
    .map_err(|err| {
        if err.to_string().contains("UNIQUE") {
            "Forge administrator is already initialized".to_string()
        } else {
            err.to_string()
        }
    })?;
    Ok(())
}

pub fn is_configured(db_path: &Path) -> Result<bool, String> {
    let db = connection(db_path)?;
    db.query_row("SELECT 1 FROM forge_admin WHERE singleton = 1", [], |_| {
        Ok(())
    })
    .optional()
    .map(|value| value.is_some())
    .map_err(|err| err.to_string())
}

pub fn authenticate(db_path: &Path, email: &str, password: &str) -> Result<Option<String>, String> {
    let db = connection(db_path)?;
    let stored: Option<(String, String)> = db
        .query_row(
            "SELECT email, password_hash FROM forge_admin WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|err| err.to_string())?;
    let Some((stored_email, encoded)) = stored else {
        return Ok(None);
    };
    let parsed =
        PasswordHash::new(&encoded).map_err(|_| "stored password hash is invalid".to_string())?;
    let password_valid = Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok();
    if stored_email != email.trim().to_ascii_lowercase() || !password_valid {
        return Ok(None);
    }
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let token_hash = token_digest(&token);
    let now = Utc::now().timestamp();
    db.execute(
        "INSERT INTO forge_admin_sessions(token_hash, created_at, expires_at, revoked) VALUES (?1, ?2, ?3, 0)",
        params![token_hash, now, now + SESSION_TTL_SECONDS],
    )
    .map_err(|err| err.to_string())?;
    Ok(Some(token))
}

pub fn session_valid(db_path: &Path, token: &str) -> Result<bool, String> {
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Ok(false);
    }
    let db = connection(db_path)?;
    db.query_row(
        "SELECT 1 FROM forge_admin_sessions WHERE token_hash = ?1 AND revoked = 0 AND expires_at > ?2",
        params![token_digest(token), Utc::now().timestamp()],
        |_| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(|err| err.to_string())
}

/// The actor behind a valid session, for audit trails that must name who
/// authorized a mutation. The store holds exactly one Forge administrator,
/// so a valid session resolves to that account's address; an invalid or
/// expired token resolves to `None` — the actor is never taken from a
/// browser-supplied field. The password hash is never read here.
pub fn session_actor(db_path: &Path, token: &str) -> Result<Option<String>, String> {
    if !session_valid(db_path, token)? {
        return Ok(None);
    }
    let db = connection(db_path)?;
    db.query_row(
        "SELECT email FROM forge_admin WHERE singleton = 1",
        [],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|err| err.to_string())
}

pub fn revoke(db_path: &Path, token: &str) -> Result<(), String> {
    let db = connection(db_path)?;
    db.execute(
        "UPDATE forge_admin_sessions SET revoked = 1 WHERE token_hash = ?1",
        [token_digest(token)],
    )
    .map(|_| ())
    .map_err(|err| err.to_string())
}

/// Replace the single administrator password in place — the stored email is
/// untouched — and revoke every outstanding session so no cookie issued before
/// the rotation survives it. Validation matches [`setup`]; a registry with no
/// configured administrator is refused rather than silently initialized.
pub fn change_password(db_path: &Path, password: &str) -> Result<(), String> {
    if password.chars().count() < 12 || password.len() > 1024 {
        return Err("password must contain 12 to 1024 characters".into());
    }
    let db = connection(db_path)?;
    let configured: Option<i64> = db
        .query_row("SELECT 1 FROM forge_admin WHERE singleton = 1", [], |_| {
            Ok(1)
        })
        .optional()
        .map_err(|err| err.to_string())?;
    if configured.is_none() {
        return Err(
            "no Forge administrator is configured; run `forge identity setup` first".into(),
        );
    }
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| "password hashing failed".to_string())?
        .to_string();
    db.execute(
        "UPDATE forge_admin SET password_hash = ?1 WHERE singleton = 1",
        params![password_hash],
    )
    .map_err(|err| err.to_string())?;
    revoke_all_sessions_in(&db)?;
    Ok(())
}

pub fn revoke_all_sessions(db_path: &Path) -> Result<(), String> {
    let db = connection(db_path)?;
    revoke_all_sessions_in(&db)
}

fn revoke_all_sessions_in(db: &Connection) -> Result<(), String> {
    db.execute(
        "UPDATE forge_admin_sessions SET revoked = 1 WHERE revoked = 0",
        [],
    )
    .map(|_| ())
    .map_err(|err| err.to_string())
}

/// Generate one cryptographically strong password from operating-system
/// entropy. Registry-free and TTY-free: it never opens the database and never
/// logs the value, so the caller prints it once. The alphabet omits look-alike
/// characters (`0/O`, `1/l/I`) so a pasted password stays reliable, and the
/// default range always satisfies the [`setup`] minimum.
pub fn generate_password(length: usize) -> Result<String, String> {
    if length < 12 || length > 128 {
        return Err("password length must be between 12 and 128 characters".into());
    }
    const ALPHABET: &[u8] =
        b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789!@#$%^&*-_=+";
    let mut rng = rand::thread_rng();
    let password: String = (0..length)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect();
    Ok(password)
}

fn token_digest(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn forge_admin_password_and_session_lifecycle() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("registry.db");
        assert!(!is_configured(&db_path).unwrap());
        setup(&db_path, "Admin@Example.com", "a-long-password-for-review").unwrap();
        assert!(is_configured(&db_path).unwrap());
        assert!(setup(&db_path, "other@example.com", "a-long-password-for-review").is_err());
        assert!(
            authenticate(&db_path, "admin@example.com", "wrong-password")
                .unwrap()
                .is_none()
        );
        let token = authenticate(&db_path, "admin@example.com", "a-long-password-for-review")
            .unwrap()
            .unwrap();
        assert!(session_valid(&db_path, &token).unwrap());
        let db = Connection::open(&db_path).unwrap();
        let stored: String = db
            .query_row(
                "SELECT password_hash FROM forge_admin WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!stored.contains("a-long-password-for-review"));
        let digest = token_digest(&token);
        assert_ne!(digest, token);
        revoke(&db_path, &token).unwrap();
        assert!(!session_valid(&db_path, &token).unwrap());
    }

    #[test]
    fn setup_refuses_weak_passwords_and_bad_emails() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("registry.db");
        // Weak (under 12 characters) and malformed emails are refused and
        // leave no administrator behind.
        assert!(setup(&db_path, "operator@example.test", "short12").is_err());
        assert!(setup(&db_path, "not-an-email", "a-long-password-for-review").is_err());
        assert!(setup(
            &db_path,
            "spaced @example.test",
            "a-long-password-for-review"
        )
        .is_err());
        assert!(!is_configured(&db_path).unwrap());
    }

    #[test]
    fn expired_session_authorizes_nothing() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("registry.db");
        setup(
            &db_path,
            "operator@example.test",
            "a-long-password-for-review",
        )
        .unwrap();
        let token = authenticate(
            &db_path,
            "operator@example.test",
            "a-long-password-for-review",
        )
        .unwrap()
        .unwrap();
        assert!(session_valid(&db_path, &token).unwrap());
        // Force the stored session past its bound; a raw or digest token from an
        // expired session must authorize nothing.
        let db = Connection::open(&db_path).unwrap();
        db.execute(
            "UPDATE forge_admin_sessions SET expires_at = ?1",
            [Utc::now().timestamp() - 1],
        )
        .unwrap();
        drop(db);
        assert!(!session_valid(&db_path, &token).unwrap());
    }

    #[test]
    fn change_password_rotates_hash_and_revokes_sessions() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("registry.db");
        setup(
            &db_path,
            "operator@example.test",
            "a-long-password-for-review",
        )
        .unwrap();
        let token = authenticate(
            &db_path,
            "operator@example.test",
            "a-long-password-for-review",
        )
        .unwrap()
        .unwrap();
        assert!(session_valid(&db_path, &token).unwrap());
        let before = Connection::open(&db_path)
            .unwrap()
            .query_row::<String, _, _>(
                "SELECT email FROM forge_admin WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        change_password(&db_path, "a-different-long-password").unwrap();
        // Old password stops working; the new one authenticates.
        assert!(authenticate(
            &db_path,
            "operator@example.test",
            "a-long-password-for-review"
        )
        .unwrap()
        .is_none());
        assert!(authenticate(
            &db_path,
            "operator@example.test",
            "a-different-long-password"
        )
        .unwrap()
        .is_some());
        // Email is preserved and the pre-rotation session is revoked.
        let after = Connection::open(&db_path)
            .unwrap()
            .query_row::<String, _, _>(
                "SELECT email FROM forge_admin WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(before, after);
        assert!(!session_valid(&db_path, &token).unwrap());
    }

    #[test]
    fn change_password_refuses_weak_input_and_missing_administrator() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("registry.db");
        // No administrator yet: refused, and none is created.
        assert!(change_password(&db_path, "a-different-long-password").is_err());
        assert!(!is_configured(&db_path).unwrap());
        setup(
            &db_path,
            "operator@example.test",
            "a-long-password-for-review",
        )
        .unwrap();
        // Weak input is refused and leaves the current hash usable.
        assert!(change_password(&db_path, "short12").is_err());
        assert!(authenticate(
            &db_path,
            "operator@example.test",
            "a-long-password-for-review"
        )
        .unwrap()
        .is_some());
    }

    #[test]
    fn generate_password_respects_bounds_and_is_random() {
        assert!(generate_password(11).is_err());
        assert!(generate_password(129).is_err());
        let first = generate_password(20).unwrap();
        let second = generate_password(20).unwrap();
        assert_eq!(first.chars().count(), 20);
        assert_eq!(second.chars().count(), 20);
        assert_ne!(first, second);
        // Meets the administrator minimum without a registry present.
        assert!(generate_password(12).unwrap().chars().count() >= 12);
    }
}
