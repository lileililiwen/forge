//! The Supabase provider pack.
//!
//! The pack accepts only an isolated Supabase project. It never provisions a Supabase
//! project, and it reports the provider as unavailable when the project reference is
//! missing or the project is not isolated, so Forge never falls back to another
//! provider.

use forge_identity_provider::{
    validate_request, ProviderOperation, ProviderRequest, ProviderResponse, ProviderStatus,
    RecoveryAction, CONTRACT_ID,
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
            return response(
                "supabase",
                "unknown",
                "unknown",
                "unknown",
                ProviderOperation::Probe,
                ProviderStatus::Failed,
                format!("the request line is not valid JSON: {error}"),
                Vec::new(),
            )
        }
    };

    if let Err(error) = validate_request(&request) {
        return response(
            &request.provider_id,
            &request.operation_id,
            &request.project_id,
            &request.project_revision,
            request.operation,
            ProviderStatus::Failed,
            error.to_string(),
            Vec::new(),
        );
    }

    let project_ref = request
        .input
        .get("supabase_project_ref")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let isolated = request
        .input
        .get("isolated_project")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if project_ref.is_empty() {
        return blocked(
            &request,
            "supabase_project_ref_missing",
            "Name the isolated Supabase project this application uses.",
        );
    }
    if !isolated {
        return blocked(
            &request,
            "supabase_project_not_isolated",
            "The Supabase project must be dedicated to this application. Shared projects are refused.",
        );
    }
    if !matches!(
        request.operation,
        ProviderOperation::Probe | ProviderOperation::Plan
    ) {
        return blocked(
            &request,
            "supabase_apply_not_supported",
            "This pack prepares an isolated Supabase project; provisioning it is an operator task.",
        );
    }
    // The pack never advertises a provider as ready without a verified OIDC and
    // registration capability check for the isolated project.
    let capability_verified = request
        .input
        .get("oidc_capability_verified")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !capability_verified {
        return blocked(
            &request,
            "supabase_oidc_capability_unverified",
            "The isolated Supabase project has not passed the OIDC and registration capability check, so this pack cannot be advertised as ready.",
        );
    }

    response(
        &request.provider_id,
        &request.operation_id,
        &request.project_id,
        &request.project_revision,
        request.operation,
        ProviderStatus::Ready,
        format!("The isolated Supabase project '{project_ref}' can serve this application."),
        Vec::new(),
    )
}

fn blocked(request: &ProviderRequest, reason: &str, instruction: &str) -> ProviderResponse {
    response(
        &request.provider_id,
        &request.operation_id,
        &request.project_id,
        &request.project_revision,
        request.operation,
        ProviderStatus::Blocked,
        instruction.to_string(),
        vec![RecoveryAction {
            reason: reason.to_string(),
            instruction: instruction.to_string(),
        }],
    )
}

struct Answer {
    provider_id: String,
    operation_id: String,
    project_id: String,
    project_revision: String,
    operation: ProviderOperation,
    status: ProviderStatus,
    summary: String,
    recovery: Vec<RecoveryAction>,
}

#[allow(clippy::too_many_arguments)]
fn response(
    provider_id: &str,
    operation_id: &str,
    project_id: &str,
    project_revision: &str,
    operation: ProviderOperation,
    status: ProviderStatus,
    summary: String,
    recovery: Vec<RecoveryAction>,
) -> ProviderResponse {
    let answer = Answer {
        provider_id: provider_id.to_string(),
        operation_id: operation_id.to_string(),
        project_id: project_id.to_string(),
        project_revision: project_revision.to_string(),
        operation,
        status,
        summary,
        recovery,
    };
    ProviderResponse {
        contract: CONTRACT_ID.to_string(),
        provider_id: answer.provider_id,
        operation_id: answer.operation_id,
        project_id: answer.project_id,
        project_revision: answer.project_revision,
        operation: answer.operation,
        status: answer.status,
        summary: answer.summary,
        plan_digest: None,
        proposed_changes: Vec::new(),
        population: None,
        secret_references: Vec::new(),
        recovery: answer.recovery,
        details: json!({}),
    }
}
