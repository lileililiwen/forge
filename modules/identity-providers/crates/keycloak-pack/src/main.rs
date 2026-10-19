//! The local Keycloak provider pack.
//!
//! The pack owns one Keycloak realm per project on a local container runtime. It
//! speaks the `forge-identity-provider/1.0` contract on standard input and standard
//! output. It never returns a secret value: the client secret it generates is written
//! to a file with owner-only permissions and returned as a reference.

mod keycloak;
mod runtime;

use forge_identity_provider::{
    plan_digest, validate_request, Population, ProposedChange, ProviderOperation, ProviderRequest,
    ProviderResponse, ProviderStatus, RecoveryAction, CONTRACT_ID,
};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let response = handle_line(&line);
        let _ = writeln!(
            stdout,
            "{}",
            serde_json::to_string(&response).unwrap_or_default()
        );
        let _ = stdout.flush();
    }
}

fn handle_line(line: &str) -> ProviderResponse {
    let request: ProviderRequest = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(error) => {
            return failure_response(
                "forge-identity-provider",
                "unknown",
                "unknown",
                "unknown",
                ProviderOperation::Probe,
                format!("the request line is not valid JSON: {error}"),
            )
        }
    };

    if let Err(error) = validate_request(&request) {
        return failure_response(
            &request.provider_id,
            &request.operation_id,
            &request.project_id,
            &request.project_revision,
            request.operation,
            error.to_string(),
        );
    }

    let input = request.input.clone();
    let state_dir = input
        .get("state_dir")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let port = input.get("port").and_then(Value::as_u64).unwrap_or(0);
    let callback_path = input
        .get("callback_path")
        .and_then(Value::as_str)
        .unwrap_or("/signin-oidc")
        .to_string();
    let scopes: Vec<String> = input
        .get("scopes")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_else(|| vec!["openid".to_string(), "profile".to_string()]);

    match request.operation {
        ProviderOperation::Probe => probe(&request, &state_dir, port),
        ProviderOperation::Plan => plan(&request, &state_dir, port, &callback_path, &scopes),
        ProviderOperation::Apply => apply(&request, &state_dir, port, &callback_path, &scopes),
        ProviderOperation::Status => status(&request, &state_dir, port),
        ProviderOperation::Stop => stop(&request, &state_dir),
    }
}

fn probe(request: &ProviderRequest, state_dir: &str, port: u64) -> ProviderResponse {
    let mut recovery = Vec::new();
    if state_dir.is_empty() {
        recovery.push(RecoveryAction {
            reason: "state_dir_missing".to_string(),
            instruction: "Pass a writable state directory for the local provider runtime."
                .to_string(),
        });
    }
    if !runtime::container_runtime_available() {
        recovery.push(RecoveryAction {
            reason: "container_runtime_unavailable".to_string(),
            instruction: "Install a container runtime so the local Keycloak pack can start."
                .to_string(),
        });
    }
    if port == 0 {
        recovery.push(RecoveryAction {
            reason: "port_missing".to_string(),
            instruction: "Pass the local port the provider runtime should listen on.".to_string(),
        });
    }

    if recovery.is_empty() {
        ProviderResponse {
            contract: CONTRACT_ID.to_string(),
            provider_id: request.provider_id.clone(),
            operation_id: request.operation_id.clone(),
            project_id: request.project_id.clone(),
            project_revision: request.project_revision.clone(),
            operation: request.operation,
            status: ProviderStatus::Ready,
            summary: "The local Keycloak pack can serve this project.".to_string(),
            plan_digest: None,
            proposed_changes: Vec::new(),
            population: None,
            secret_references: Vec::new(),
            recovery: Vec::new(),
            details: json!({ "mode": "local-loopback-http" }),
        }
    } else {
        ProviderResponse {
            contract: CONTRACT_ID.to_string(),
            provider_id: request.provider_id.clone(),
            operation_id: request.operation_id.clone(),
            project_id: request.project_id.clone(),
            project_revision: request.project_revision.clone(),
            operation: request.operation,
            status: ProviderStatus::Blocked,
            summary: "The local Keycloak pack cannot serve this project yet.".to_string(),
            plan_digest: None,
            proposed_changes: Vec::new(),
            population: None,
            secret_references: Vec::new(),
            recovery,
            details: json!({ "mode": "local-loopback-http" }),
        }
    }
}

fn plan(
    request: &ProviderRequest,
    state_dir: &str,
    port: u64,
    callback_path: &str,
    scopes: &[String],
) -> ProviderResponse {
    let realm = keycloak::realm_name(&request.project_id);
    let client_id = keycloak::client_id(&request.project_id);
    let changes = vec![
        ProposedChange {
            kind: "provider.compose".to_string(),
            target: format!("{state_dir}/{realm}/compose.yaml"),
            summary: format!("Write the local Keycloak compose file for realm '{realm}'."),
        },
        ProposedChange {
            kind: "provider.realm".to_string(),
            target: format!("realm:{realm}"),
            summary: format!("Create the isolated Keycloak realm '{realm}'."),
        },
        ProposedChange {
            kind: "provider.client".to_string(),
            target: format!("client:{client_id}"),
            summary: format!(
                "Create the OIDC client '{client_id}' with the callback path '{callback_path}'."
            ),
        },
        ProposedChange {
            kind: "manifest.auth".to_string(),
            target: "forge.yaml".to_string(),
            summary:
                "Record the provider, issuer, client id and callback path in the project manifest."
                    .to_string(),
        },
    ];

    let digest = plan_digest(
        &request.project_id,
        &request.provider_id,
        request.operation.as_str(),
        &changes,
    )
    .unwrap_or_default();

    ProviderResponse {
        contract: CONTRACT_ID.to_string(),
        provider_id: request.provider_id.clone(),
        operation_id: request.operation_id.clone(),
        project_id: request.project_id.clone(),
        project_revision: request.project_revision.clone(),
        operation: request.operation,
        status: ProviderStatus::Ready,
        summary: format!(
            "The plan creates one isolated Keycloak realm '{realm}' for this project."
        ),
        plan_digest: Some(digest),
        proposed_changes: changes,
        population: None,
        secret_references: Vec::new(),
        recovery: Vec::new(),
        details: json!({
            "mode": "local-loopback-http",
            "realm": realm,
            "client_id": client_id,
            "scopes": scopes,
            "port": port,
        }),
    }
}

fn apply(
    request: &ProviderRequest,
    state_dir: &str,
    port: u64,
    callback_path: &str,
    scopes: &[String],
) -> ProviderResponse {
    let realm = keycloak::realm_name(&request.project_id);
    let client_id = keycloak::client_id(&request.project_id);
    let project_dir = format!("{state_dir}/{realm}");

    let changes = vec![
        ProposedChange {
            kind: "provider.compose".to_string(),
            target: format!("{project_dir}/compose.yaml"),
            summary: format!("Write the local Keycloak compose file for realm '{realm}'."),
        },
        ProposedChange {
            kind: "provider.realm".to_string(),
            target: format!("realm:{realm}"),
            summary: format!("Create the isolated Keycloak realm '{realm}'."),
        },
        ProposedChange {
            kind: "provider.client".to_string(),
            target: format!("client:{client_id}"),
            summary: format!(
                "Create the OIDC client '{client_id}' with the callback path '{callback_path}'."
            ),
        },
        ProposedChange {
            kind: "manifest.auth".to_string(),
            target: "forge.yaml".to_string(),
            summary:
                "Record the provider, issuer, client id and callback path in the project manifest."
                    .to_string(),
        },
    ];
    let digest = plan_digest(&request.project_id, &request.provider_id, "plan", &changes)
        .unwrap_or_default();

    let expected = request
        .input
        .get("plan_digest")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !expected.is_empty() && expected != digest {
        return blocked_response(
            request,
            "plan_digest_mismatch",
            "The plan changed since it was confirmed. Run the preview again.",
        );
    }

    // The bootstrap admin password is generated before the runtime starts, so the
    // container and the admin login use the same credential.
    let admin = match keycloak::load_or_create_admin(&project_dir) {
        Ok(admin) => admin,
        Err(error) => return failed_response(request, "admin_credentials_failed", &error),
    };
    if let Err(error) = runtime::write_compose(&project_dir, &realm, port) {
        return failed_response(request, "compose_write_failed", &error);
    }
    if let Err(error) = runtime::start_container(&project_dir, &realm, &admin.password) {
        return failed_response(request, "container_start_failed", &error);
    }
    if let Err(error) = runtime::wait_for_health(&realm, port) {
        return failed_response(request, "provider_health_timeout", &error);
    }
    // The admin login is the credential check. A refused login means the
    // running runtime was bootstrapped with another credential, so it cannot
    // serve this project. The pack refuses it and never destroys it on its own:
    // the reset removes the local realm and its users, so the owner decides.
    let token = match keycloak::admin_token(&admin, port) {
        Ok(token) => token,
        Err(error) => {
            let origin = if admin.fresh {
                "The state directory held no credential for this runtime, and the running provider was started with another one."
            } else {
                "The running provider was started with another credential than the one this state directory holds."
            };
            return blocked_response(
                request,
                "provider_credential_mismatch",
                &format!(
                    "{error}. {origin} Reset the local provider and run the setup again; the reset removes this project's local realm and its users: {}",
                    runtime::reset_command(&project_dir, &realm)
                ),
            );
        }
    };
    if let Err(error) = keycloak::ensure_realm(&realm, &token, port) {
        return failed_response(request, "realm_creation_failed", &error);
    }
    if let Err(error) =
        keycloak::ensure_client(&realm, &client_id, callback_path, scopes, &token, port)
    {
        return failed_response(request, "client_creation_failed", &error);
    }

    let issuer = format!("http://127.0.0.1:{port}/realms/{realm}");
    let population = Population {
        population_id: format!("keycloak-local/{realm}"),
        issuer: issuer.clone(),
        client_id: client_id.clone(),
        callback_path: callback_path.to_string(),
        scopes: scopes.to_vec(),
        registration_url: Some(format!("{issuer}/protocol/openid-connect/registrations?client_id={client_id}&response_type=code&scope=openid&redirect_uri={callback_path}")),
    };

    ProviderResponse {
        contract: CONTRACT_ID.to_string(),
        provider_id: request.provider_id.clone(),
        operation_id: request.operation_id.clone(),
        project_id: request.project_id.clone(),
        project_revision: request.project_revision.clone(),
        operation: request.operation,
        status: ProviderStatus::Ready,
        summary: format!("The isolated Keycloak realm '{realm}' is ready."),
        plan_digest: Some(digest),
        proposed_changes: changes,
        population: Some(population),
        secret_references: vec![format!("file://{project_dir}/admin-password")],
        recovery: Vec::new(),
        details: json!({
            "mode": "local-loopback-http",
            "realm": realm,
            "port": port,
            "state_dir": project_dir,
        }),
    }
}

fn status(request: &ProviderRequest, state_dir: &str, port: u64) -> ProviderResponse {
    let realm = keycloak::realm_name(&request.project_id);
    let project_dir = format!("{state_dir}/{realm}");
    let running = runtime::container_running(&realm);
    let mut recovery = Vec::new();
    if !running {
        recovery.push(RecoveryAction {
            reason: "provider_not_running".to_string(),
            instruction: "Start the local provider runtime before checking the population again."
                .to_string(),
        });
    }
    ProviderResponse {
        contract: CONTRACT_ID.to_string(),
        provider_id: request.provider_id.clone(),
        operation_id: request.operation_id.clone(),
        project_id: request.project_id.clone(),
        project_revision: request.project_revision.clone(),
        operation: request.operation,
        status: if running {
            ProviderStatus::Ready
        } else {
            ProviderStatus::Blocked
        },
        summary: if running {
            "The local provider runtime is running.".to_string()
        } else {
            "The local provider runtime is not running.".to_string()
        },
        plan_digest: None,
        proposed_changes: Vec::new(),
        population: None,
        secret_references: Vec::new(),
        recovery,
        details: json!({ "realm": realm, "port": port, "state_dir": project_dir }),
    }
}

fn stop(request: &ProviderRequest, state_dir: &str) -> ProviderResponse {
    let realm = keycloak::realm_name(&request.project_id);
    let project_dir = format!("{state_dir}/{realm}");
    if let Err(error) = runtime::stop_container(&project_dir, &realm) {
        return failed_response(request, "container_stop_failed", &error);
    }
    ProviderResponse {
        contract: CONTRACT_ID.to_string(),
        provider_id: request.provider_id.clone(),
        operation_id: request.operation_id.clone(),
        project_id: request.project_id.clone(),
        project_revision: request.project_revision.clone(),
        operation: request.operation,
        status: ProviderStatus::Ready,
        summary: "The local provider runtime is stopped.".to_string(),
        plan_digest: None,
        proposed_changes: Vec::new(),
        population: None,
        secret_references: Vec::new(),
        recovery: Vec::new(),
        details: json!({ "realm": realm, "state_dir": project_dir }),
    }
}

fn blocked_response(
    request: &ProviderRequest,
    reason: &str,
    instruction: &str,
) -> ProviderResponse {
    ProviderResponse {
        contract: CONTRACT_ID.to_string(),
        provider_id: request.provider_id.clone(),
        operation_id: request.operation_id.clone(),
        project_id: request.project_id.clone(),
        project_revision: request.project_revision.clone(),
        operation: request.operation,
        status: ProviderStatus::Blocked,
        summary: instruction.to_string(),
        plan_digest: None,
        proposed_changes: Vec::new(),
        population: None,
        secret_references: Vec::new(),
        recovery: vec![RecoveryAction {
            reason: reason.to_string(),
            instruction: instruction.to_string(),
        }],
        details: json!({}),
    }
}

fn failed_response(request: &ProviderRequest, reason: &str, detail: &str) -> ProviderResponse {
    ProviderResponse {
        contract: CONTRACT_ID.to_string(),
        provider_id: request.provider_id.clone(),
        operation_id: request.operation_id.clone(),
        project_id: request.project_id.clone(),
        project_revision: request.project_revision.clone(),
        operation: request.operation,
        status: ProviderStatus::Failed,
        summary: "The provider operation failed.".to_string(),
        plan_digest: None,
        proposed_changes: Vec::new(),
        population: None,
        secret_references: Vec::new(),
        recovery: vec![RecoveryAction {
            reason: reason.to_string(),
            instruction: format!("Recover the provider and run the setup again. {detail}"),
        }],
        details: json!({ "reason": reason }),
    }
}

fn failure_response(
    provider_id: &str,
    operation_id: &str,
    project_id: &str,
    project_revision: &str,
    operation: ProviderOperation,
    summary: String,
) -> ProviderResponse {
    ProviderResponse {
        contract: CONTRACT_ID.to_string(),
        provider_id: provider_id.to_string(),
        operation_id: operation_id.to_string(),
        project_id: project_id.to_string(),
        project_revision: project_revision.to_string(),
        operation,
        status: ProviderStatus::Failed,
        summary,
        plan_digest: None,
        proposed_changes: Vec::new(),
        population: None,
        secret_references: Vec::new(),
        recovery: Vec::new(),
        details: json!({}),
    }
}
