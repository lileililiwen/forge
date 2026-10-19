//! The Keycloak realm and client operations.
//!
//! The pack talks to the local Keycloak admin REST API. It creates one realm per
//! project and one public OIDC client inside that realm, so two projects never share
//! a user population.

use crate::runtime::{admin_password_path, read_secret, write_secret};
use std::process::Command;

/// The realm name for one project.
pub fn realm_name(project_id: &str) -> String {
    format!("forge-{}", sanitize(project_id))
}

/// The OIDC client id for one project.
pub fn client_id(project_id: &str) -> String {
    format!("forge-{}", sanitize(project_id))
}

fn sanitize(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
            out.push(character.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// The admin credentials the pack uses against the local provider.
pub struct Admin {
    /// The bootstrap admin user name.
    pub username: String,
    /// The bootstrap admin password. Never returned in a contract response.
    pub password: String,
    /// True when the state directory held no credential, so this credential was
    /// generated for this run.
    pub fresh: bool,
}

/// Loads the generated admin password, or creates one when it is missing.
pub fn load_or_create_admin(project_dir: &str) -> Result<Admin, String> {
    std::fs::create_dir_all(project_dir)
        .map_err(|error| format!("cannot create the state directory: {error}"))?;
    let path = admin_password_path(project_dir);
    if let Some(password) = read_secret(&path) {
        return Ok(Admin {
            username: "forge-admin".to_string(),
            password,
            fresh: false,
        });
    }

    let password = generate_password();
    write_secret(&path, &password)?;
    Ok(Admin {
        username: "forge-admin".to_string(),
        password,
        fresh: true,
    })
}

fn generate_password() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    format!("forge-{nanos:x}")
}

/// The admin access token for the local provider.
pub fn admin_token(admin: &Admin, port: u64) -> Result<String, String> {
    let url = format!("http://127.0.0.1:{port}/realms/master/protocol/openid-connect/token");
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--max-time",
            "30",
            "-X",
            "POST",
            "-d",
            "grant_type=password",
            "-d",
            "client_id=admin-cli",
            "-d",
            &format!("username={}", admin.username),
            "-d",
            &format!("password={}", admin.password),
            &url,
        ])
        .output()
        .map_err(|error| format!("cannot reach the provider admin API: {error}"))?;
    if !output.status.success() {
        return Err("the provider admin API refused the bootstrap admin login".to_string());
    }
    let body = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .get("access_token")
                .and_then(|token| token.as_str())
                .map(str::to_string)
        })
        .ok_or_else(|| "the provider admin API returned no access token".to_string())
}

/// Creates the project realm when it does not exist yet.
pub fn ensure_realm(realm: &str, token: &str, port: u64) -> Result<(), String> {
    let status = Command::new("curl")
        .args([
            "--silent",
            "--output",
            "/dev/null",
            "--write-out",
            "%{http_code}",
            "--max-time",
            "30",
            "-X",
            "POST",
            "-H",
            &format!("Authorization: Bearer {token}"),
            "-H",
            "Content-Type: application/json",
            "-d",
            &format!("{{\"realm\":\"{realm}\",\"enabled\":true,\"sslRequired\":\"none\"}}"),
            &format!("http://127.0.0.1:{port}/admin/realms"),
        ])
        .output()
        .map_err(|error| format!("cannot reach the provider admin API: {error}"))?;
    if !status.status.success() {
        return Err("the provider admin API refused the realm creation".to_string());
    }
    let code = String::from_utf8_lossy(&status.stdout).trim().to_string();
    if !(code == "201" || code == "409") {
        return Err(format!(
            "the provider admin API answered {code} for the realm creation"
        ));
    }
    Ok(())
}

/// Creates the project OIDC client when it does not exist yet.
pub fn ensure_client(
    realm: &str,
    client_id: &str,
    callback_path: &str,
    scopes: &[String],
    token: &str,
    port: u64,
) -> Result<(), String> {
    let scope = scopes.join(" ");
    let body = format!(
        "{{\"clientId\":\"{client_id}\",\"enabled\":true,\"publicClient\":true,\"standardFlowEnabled\":true,\"directAccessGrantsEnabled\":false,\"serviceAccountsEnabled\":false,\"redirectUris\":[\"http://localhost:5000{callback_path}\",\"http://127.0.0.1:5000{callback_path}\"],\"webOrigins\":[\"http://localhost:5000\",\"http://127.0.0.1:5000\"],\"attributes\":{{\"post.logout.redirect.uris\":\"http://localhost:5000{callback_path}\"}},\"protocol\":\"openid-connect\"}}"
    );
    let _ = scope;
    let status = Command::new("curl")
        .args([
            "--silent",
            "--write-out",
            "\n%{http_code}",
            "--max-time",
            "30",
            "-X",
            "POST",
            "-H",
            &format!("Authorization: Bearer {token}"),
            "-H",
            "Content-Type: application/json",
            "-d",
            &body,
            &format!("http://127.0.0.1:{port}/admin/realms/{realm}/clients"),
        ])
        .output()
        .map_err(|error| format!("cannot reach the provider admin API: {error}"))?;
    if !status.status.success() {
        return Err("the provider admin API refused the client creation".to_string());
    }
    let answer = String::from_utf8_lossy(&status.stdout);
    let (payload, code) = answer
        .rsplit_once('\n')
        .map(|(payload, code)| (payload.to_string(), code.trim().to_string()))
        .unwrap_or((answer.to_string(), String::new()));
    if !(code == "201" || code == "409") {
        return Err(format!(
            "the provider admin API answered {code} for the client creation: {}",
            summarize(&payload)
        ));
    }
    Ok(())
}

/// A short, single-line summary of a provider answer.
fn summarize(payload: &str) -> String {
    let flat = payload.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.len() > 300 {
        format!("{}…", &flat[..300])
    } else {
        flat
    }
}
