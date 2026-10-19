//! The local container runtime the Keycloak pack drives.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// The pinned Keycloak image the local pack runs.
pub const KEYCLOAK_IMAGE: &str = "quay.io/keycloak/keycloak:26.0.5";

/// The container runtime binary the pack uses.
const CONTAINER_RUNTIME: &str = "docker";

/// True when a container runtime is on the path.
pub fn container_runtime_available() -> bool {
    Command::new(CONTAINER_RUNTIME)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Writes the compose file for one project.
pub fn write_compose(project_dir: &str, realm: &str, port: u64) -> Result<(), String> {
    fs::create_dir_all(project_dir)
        .map_err(|error| format!("cannot create the state directory: {error}"))?;
    let compose = format!(
        "services:\n  keycloak:\n    image: {KEYCLOAK_IMAGE}\n    container_name: forge-identity-{realm}\n    command: [\"start-dev\"]\n    environment:\n      KC_BOOTSTRAP_ADMIN_USERNAME: \"${{KEYCLOAK_ADMIN}}\"\n      KC_BOOTSTRAP_ADMIN_PASSWORD: \"${{KEYCLOAK_ADMIN_PASSWORD}}\"\n      KC_HEALTH_ENABLED: \"true\"\n      KC_HTTP_ENABLED: \"true\"\n    ports:\n      - \"{port}:8080\"\n    volumes:\n      - keycloak-data:/opt/keycloak/data\nvolumes:\n  keycloak-data:\n"
    );
    let path = Path::new(project_dir).join("compose.yaml");
    fs::write(&path, compose).map_err(|error| format!("cannot write the compose file: {error}"))?;
    Ok(())
}

/// The exact command that resets one project's local provider runtime.
///
/// The pack never runs this on its own: the reset removes the local realm and
/// its users, so the owner decides when that happens.
pub fn reset_command(project_dir: &str, realm: &str) -> String {
    format!(
        "docker compose --project-name forge-identity-{realm} -f {project_dir}/compose.yaml down --volumes"
    )
}

/// Starts the container for one project.
pub fn start_container(project_dir: &str, realm: &str, password: &str) -> Result<(), String> {
    let status = Command::new(CONTAINER_RUNTIME)
        .args([
            "compose",
            "--project-name",
            &format!("forge-identity-{realm}"),
            "-f",
            &format!("{project_dir}/compose.yaml"),
            "up",
            "-d",
        ])
        .env("KEYCLOAK_ADMIN", "forge-admin")
        .env("KEYCLOAK_ADMIN_PASSWORD", password)
        .output()
        .map_err(|error| format!("cannot start the container runtime: {error}"))?;
    if !status.status.success() {
        return Err(format!(
            "the container runtime refused to start the provider: {}",
            String::from_utf8_lossy(&status.stderr).trim()
        ));
    }
    Ok(())
}

/// True when the container for one project is running.
pub fn container_running(realm: &str) -> bool {
    Command::new(CONTAINER_RUNTIME)
        .args([
            "ps",
            "--filter",
            &format!("name=forge-identity-{realm}"),
            "--format",
            "{{.Names}}",
        ])
        .output()
        .map(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .any(|line| line.contains(&format!("forge-identity-{realm}")))
        })
        .unwrap_or(false)
}

/// Stops the container for one project.
pub fn stop_container(project_dir: &str, realm: &str) -> Result<(), String> {
    let status = Command::new(CONTAINER_RUNTIME)
        .args([
            "compose",
            "--project-name",
            &format!("forge-identity-{realm}"),
            "-f",
            &format!("{project_dir}/compose.yaml"),
            "down",
        ])
        .output()
        .map_err(|error| format!("cannot stop the container runtime: {error}"))?;
    if !status.status.success() {
        return Err(format!(
            "the container runtime refused to stop the provider: {}",
            String::from_utf8_lossy(&status.stderr).trim()
        ));
    }
    Ok(())
}

/// Waits until the provider answers its health endpoint.
pub fn wait_for_health(_realm: &str, port: u64) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(180);
    // The master realm exists from the first start, so its discovery document
    // proves the provider is serving before the project realm is created.
    let url = format!("http://127.0.0.1:{port}/realms/master/.well-known/openid-configuration");
    while Instant::now() < deadline {
        if let Ok(output) = Command::new("curl")
            .args(["--silent", "--fail", "--max-time", "5", &url])
            .output()
        {
            if output.status.success() {
                return Ok(());
            }
        }
        sleep(Duration::from_secs(2));
    }
    Err(format!(
        "the provider did not become healthy on port {port}"
    ))
}

/// The path of the file that holds the generated admin password.
pub fn admin_password_path(project_dir: &str) -> String {
    format!("{project_dir}/admin-password")
}

/// Reads a secret file.
pub fn read_secret(path: &str) -> Option<String> {
    let value = fs::read_to_string(path).ok()?;
    let value = value.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Writes a secret file with owner-only permissions.
pub fn write_secret(path: &str, value: &str) -> Result<(), String> {
    fs::write(path, value).map_err(|error| format!("cannot write the secret file: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}
