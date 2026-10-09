//! Identity commands (`identity`).
//!
//! Typed CLI handlers for per-project OIDC admin sessions.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::core::ForgeError;
use forge::identity::{
    build_challenge, delete_challenge_file as delete_identity_challenge,
    list_sessions as list_identity_sessions, load_challenge, load_session, mint_session,
    redact_identity_evidence, render_challenge_human,
    render_outcome_human as render_identity_outcome_human,
    render_session_human as render_identity_session_human, save_challenge, save_session,
    terminate_session, validate_callback, validate_claims, validate_session, AuthCallback,
    IdentityConfig, IdentityOutcome, ProviderClaims, IDENTITY_CONTRACT_VERSION,
};
use std::path::Path;

use super::commands_services::IdentityCommands;
use super::functions_13::resolve_identity_target;
use super::functions_9::{delete_session_file_at, parse_rfc3339, read_password_stdin};
use super::projects::{as_output, open_registry};
use crate::{Format, Output};

pub(crate) fn cmd_identity(
    db_path: &Path,
    command: &IdentityCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        IdentityCommands::Setup {
            email,
            password_stdin,
        } => {
            let password = if *password_stdin {
                read_password_stdin()?
            } else {
                let password = read_secret("New Forge password: ")?;
                let confirmation = read_secret("Confirm password: ")?;
                if password != confirmation {
                    return Err(ForgeError::IdentityInvalid {
                        reason: "password confirmation does not match".to_string(),
                    });
                }
                password
            };
            forge::identity::global::setup(db_path, email, &password)
                .map_err(|reason| ForgeError::IdentityInvalid { reason })?;
            Ok(as_output(
                format,
                format!(
                    "Forge administrator `{}` initialized",
                    email.trim().to_ascii_lowercase()
                ),
                serde_json::json!({ "contract": "forge-admin-login/1.0.0", "email": email.trim().to_ascii_lowercase(), "initialized": true }),
            ))
        }
        IdentityCommands::ChangePassword { password_stdin } => {
            let password = if *password_stdin {
                read_password_stdin()?
            } else {
                let password = read_secret("New Forge password: ")?;
                let confirmation = read_secret("Confirm password: ")?;
                if password != confirmation {
                    return Err(ForgeError::IdentityInvalid {
                        reason: "password confirmation does not match".to_string(),
                    });
                }
                password
            };
            forge::identity::global::change_password(db_path, &password)
                .map_err(|reason| ForgeError::IdentityInvalid { reason })?;
            Ok(as_output(
                format,
                "Forge administrator password updated; every existing browser session was revoked"
                    .to_string(),
                serde_json::json!({ "contract": "forge-admin-login/1.0.0", "changed": true }),
            ))
        }
        IdentityCommands::GeneratePassword { length } => {
            let password = forge::identity::global::generate_password(*length)
                .map_err(|reason| ForgeError::IdentityInvalid { reason })?;
            Ok(as_output(
                format,
                password.clone(),
                serde_json::json!({ "contract": "forge-admin-login/1.0.0", "length": *length, "password": password }),
            ))
        }
        IdentityCommands::Status => {
            let configured = forge::identity::global::is_configured(db_path)
                .map_err(|reason| ForgeError::IdentityInvalid { reason })?;
            let email = if configured {
                forge::identity::global::email(db_path)
                    .map_err(|reason| ForgeError::IdentityInvalid { reason })?
            } else {
                None
            };
            let human = match &email {
                Some(address) => format!("configured: true\nemail: {address}"),
                None => "configured: false".to_string(),
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": "forge-admin-login/1.0.0",
                    "configured": configured,
                    "email": email,
                }),
            ))
        }
        IdentityCommands::ValidateConfig { target } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
            match IdentityConfig::from_manifest_opt(&project_id, &manifest)? {
                Some(cfg) => {
                    let detail = format!(
                        "validate-config: project={} provider={} issuer={} client_id={} \
                         audience={} redirect_uri={} scopes={} admin_claim={} admin_values={} \
                         state_ttl={} session_ttl={}",
                        project_id,
                        cfg.provider,
                        redact_identity_evidence(&cfg.issuer),
                        cfg.client_id,
                        cfg.audience,
                        cfg.redirect_uri,
                        cfg.scopes.len(),
                        cfg.admin_claim,
                        cfg.admin_values.len(),
                        cfg.state_ttl_seconds,
                        cfg.session_ttl_seconds,
                    );
                    if let Ok(registry) = open_registry(db_path) {
                        let _ = registry.record_operation("identity", &project_id, "done", &detail);
                    }
                    let json = serde_json::json!({
                        "contract": IDENTITY_CONTRACT_VERSION,
                        "project_id": project_id,
                        "config": cfg,
                    });
                    Ok(as_output(
                        format,
                        format!(
                            "identity config validated for project `{}`: provider={} \
                             issuer={} client_id={} scopes={} admin_claim={} \
                             state_ttl={}s session_ttl={}s",
                            project_id,
                            cfg.provider,
                            redact_identity_evidence(&cfg.issuer),
                            cfg.client_id,
                            cfg.scopes.join(","),
                            cfg.admin_claim,
                            cfg.state_ttl_seconds,
                            cfg.session_ttl_seconds,
                        ),
                        json,
                    ))
                }
                None => {
                    let detail = "validate-config: project has no identity block".to_string();
                    if let Ok(registry) = open_registry(db_path) {
                        let _ =
                            registry.record_operation("identity", &project_id, "rejected", &detail);
                    }
                    Err(ForgeError::IdentityInvalid {
                        reason: format!(
                            "project `{project_id}` has no `identity:` block; declare one in \
                             forge.yaml to enable OIDC admin federation"
                        ),
                    })
                }
            }
        }
        IdentityCommands::BuildChallenge { target } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
            let cfg =
                IdentityConfig::from_manifest_opt(&project_id, &manifest)?.ok_or_else(|| {
                    ForgeError::IdentityInvalid {
                        reason: format!(
                            "project `{project_id}` has no `identity:` block; declare one before \
                         building an auth challenge"
                        ),
                    }
                })?;
            let now = chrono::Utc::now();
            let challenge = build_challenge(&project_id, &cfg, now)?;
            save_challenge(&dir, &project_id, &challenge)?;
            let detail = format!(
                "build-challenge: project={} state={} nonce={} code_challenge={}",
                project_id, challenge.state, challenge.nonce, challenge.code_challenge
            );
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation("identity", &project_id, "done", &detail);
            }
            let json = serde_json::json!({
                "contract": IDENTITY_CONTRACT_VERSION,
                "project_id": project_id,
                "challenge": challenge,
            });
            Ok(as_output(format, render_challenge_human(&challenge), json))
        }
        IdentityCommands::CompleteAuth {
            target,
            state,
            code,
            error,
            error_description,
            subject,
            issuer,
            audience,
            nonce,
            issued_at,
            expires_at,
            scope,
            admin_claim_value,
        } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
            let cfg =
                IdentityConfig::from_manifest_opt(&project_id, &manifest)?.ok_or_else(|| {
                    ForgeError::IdentityInvalid {
                        reason: format!(
                            "project `{project_id}` has no `identity:` block; declare one before \
                         completing the OIDC round trip"
                        ),
                    }
                })?;
            let callback = AuthCallback {
                project_id: project_id.clone(),
                state: state.clone(),
                code: code.clone(),
                error: error.clone(),
                error_description: error_description.clone(),
            };
            let now = chrono::Utc::now();
            let challenge = load_challenge(&dir, &project_id, state)?.ok_or_else(|| {
                let mut reason = format!(
                    "no pending OIDC challenge found for project `{project_id}` with state \
                         `{state}`; call `forge identity build-challenge` first and complete the \
                         round trip before the challenge expires"
                );
                if let Some(err) = error.as_deref() {
                    reason.push_str(&format!(
                        "; provider error: `{}`",
                        redact_identity_evidence(err)
                    ));
                }
                if let Some(desc) = error_description.as_deref() {
                    reason.push_str(&format!(
                        "; description: `{}`",
                        redact_identity_evidence(desc)
                    ));
                }
                ForgeError::IdentityInvalid { reason }
            })?;
            validate_callback(&callback, &challenge, now)?;
            let iat = match issued_at.as_deref() {
                Some(raw) => Some(parse_rfc3339(raw, "issued_at")?),
                None => None,
            };
            let exp = match expires_at.as_deref() {
                Some(raw) => Some(parse_rfc3339(raw, "expires_at")?),
                None => None,
            };
            let claims_iat = iat.unwrap_or(now);
            let claims_exp = exp.unwrap_or(claims_iat + chrono::Duration::seconds(60));
            let scopes = scope
                .as_deref()
                .map(|s| s.split_whitespace().map(str::to_string).collect::<Vec<_>>())
                .unwrap_or_else(|| cfg.scopes.clone());
            let mut claim_map = std::collections::BTreeMap::new();
            claim_map.insert(cfg.admin_claim.clone(), admin_claim_value.clone());
            let claims = ProviderClaims {
                issuer: issuer.clone().unwrap_or_else(|| cfg.issuer.clone()),
                audience: audience.clone().unwrap_or_else(|| cfg.audience.clone()),
                subject: subject.clone(),
                issued_at: claims_iat,
                expires_at: claims_exp,
                nonce: nonce.clone(),
                scopes,
                claims: claim_map,
            };
            validate_claims(&claims, &challenge, &cfg, now)?;
            let session = match mint_session(&cfg, &claims, &challenge, now) {
                Ok(session) => session,
                Err(err) => {
                    if let Ok(registry) = open_registry(db_path) {
                        let _ = registry.record_operation(
                            "identity",
                            &project_id,
                            "rejected",
                            &format!("complete-auth: {}", err),
                        );
                    }
                    return Err(err);
                }
            };
            save_session(&dir, &project_id, &session)?;
            let _ = delete_identity_challenge(&dir, &project_id, state);
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "identity",
                    &project_id,
                    "done",
                    &format!(
                        "complete-auth: session {} minted for subject {}",
                        session.session_id, session.subject
                    ),
                );
            }
            let outcome = IdentityOutcome::Session(session);
            let json = serde_json::json!({
                "contract": IDENTITY_CONTRACT_VERSION,
                "project_id": project_id,
                "outcome": outcome,
            });
            Ok(as_output(
                format,
                render_identity_outcome_human(&outcome),
                json,
            ))
        }
        IdentityCommands::SessionList { target } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let sessions = list_identity_sessions(&dir, &project_id)?;
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "identity",
                    &project_id,
                    "done",
                    &format!("session-list: {} session(s)", sessions.len()),
                );
            }
            let human = if sessions.is_empty() {
                format!("no admin sessions for project `{project_id}`")
            } else {
                let mut text = format!("admin sessions for project `{project_id}`:");
                for s in &sessions {
                    text.push_str(&format!("\n  - {}", render_identity_session_human(s)));
                }
                text
            };
            let json = serde_json::json!({
                "contract": IDENTITY_CONTRACT_VERSION,
                "project_id": project_id,
                "sessions": sessions,
            });
            Ok(as_output(format, human, json))
        }
        IdentityCommands::SessionInspect { target, session } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let loaded = match load_session(&dir, &project_id, session)? {
                Some(session) => session,
                None => {
                    let owner = forge::identity::lookup_session_in_sibling_projects(&dir, session)?;
                    match owner {
                        Some((_, owner_id, _)) => {
                            return Err(ForgeError::IdentitySessionCrossProject {
                                reason: format!(
                                    "session `{session}` was minted for project `{owner_id}`; \
                                     inspecting it through project `{project_id}` is refused"
                                ),
                            });
                        }
                        None => {
                            return Err(ForgeError::IdentitySessionNotFound {
                                reason: format!(
                                    "session `{session}` was not found under \
                                     `.forge/identity/{project_id}/` and no other project owns it"
                                ),
                            });
                        }
                    }
                }
            };
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "identity",
                    &project_id,
                    "done",
                    &format!("session-inspect: session {session}"),
                );
            }
            let json = serde_json::json!({
                "contract": IDENTITY_CONTRACT_VERSION,
                "project_id": project_id,
                "session": loaded,
            });
            Ok(as_output(
                format,
                render_identity_session_human(&loaded),
                json,
            ))
        }
        IdentityCommands::SessionValidate {
            target,
            session,
            permission,
        } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let loaded = match load_session(&dir, &project_id, session)? {
                Some(session) => session,
                None => {
                    let registry = open_registry(db_path).ok();
                    let mut owner: Option<(String, std::path::PathBuf)> = None;
                    if let Some(registry) = registry {
                        let all = registry.list().unwrap_or_default();
                        let projects: Vec<(String, std::path::PathBuf)> = all
                            .into_iter()
                            .map(|p| (p.id, std::path::PathBuf::from(p.path)))
                            .collect();
                        if let Some((_, owner_id, owner_dir)) =
                            forge::identity::lookup_session_across_projects(session, projects)?
                        {
                            owner = Some((owner_id, owner_dir));
                        }
                    }
                    if owner.is_none() {
                        if let Some((_, owner_id, owner_dir)) =
                            forge::identity::lookup_session_in_sibling_projects(&dir, session)?
                        {
                            owner = Some((owner_id, owner_dir));
                        }
                    }
                    match owner {
                        Some((owner_id, _)) => {
                            return Err(ForgeError::IdentitySessionCrossProject {
                                reason: format!(
                                    "session `{session}` was minted for project `{owner_id}`; \
                                     presenting it to project `{project_id}` is refused; \
                                     sessions are project-scoped and may not be shared across \
                                     unrelated applications"
                                ),
                            });
                        }
                        None => {
                            return Err(ForgeError::IdentitySessionNotFound {
                                reason: format!(
                                    "session `{session}` was not found under \
                                     `.forge/identity/{project_id}/` and no other registered \
                                     project owns it"
                                ),
                            });
                        }
                    }
                }
            };
            let now = chrono::Utc::now();
            match validate_session(&loaded, &project_id, permission, now) {
                Ok(()) => {
                    if let Ok(registry) = open_registry(db_path) {
                        let _ = registry.record_operation(
                            "identity",
                            &project_id,
                            "done",
                            &format!("session-validate: session {session} granted {permission}"),
                        );
                    }
                    let json = serde_json::json!({
                        "contract": IDENTITY_CONTRACT_VERSION,
                        "project_id": project_id,
                        "session": loaded,
                        "granted": true,
                        "permission": permission,
                    });
                    Ok(as_output(
                        format,
                        format!(
                            "session `{}` granted `{permission}` for project `{project_id}`",
                            session
                        ),
                        json,
                    ))
                }
                Err(err) => {
                    if let Ok(registry) = open_registry(db_path) {
                        let _ = registry.record_operation(
                            "identity",
                            &project_id,
                            "rejected",
                            &format!("session-validate: {} ({})", err, err.code()),
                        );
                    }
                    Err(err)
                }
            }
        }
        IdentityCommands::SessionTerminate { target, session } => {
            let (dir, project_id) = resolve_identity_target(db_path, target)?;
            let mut loaded = match load_session(&dir, &project_id, session)? {
                Some(session) => session,
                None => {
                    let owner = forge::identity::lookup_session_in_sibling_projects(&dir, session)?;
                    match owner {
                        Some((_, owner_id, _)) => {
                            return Err(ForgeError::IdentitySessionCrossProject {
                                reason: format!(
                                    "session `{session}` was minted for project `{owner_id}`; \
                                     terminating it through project `{project_id}` is refused"
                                ),
                            });
                        }
                        None => {
                            return Err(ForgeError::IdentitySessionNotFound {
                                reason: format!(
                                    "session `{session}` was not found under \
                                     `.forge/identity/{project_id}/` and no other project owns it"
                                ),
                            });
                        }
                    }
                }
            };
            terminate_session(&mut loaded);
            save_session(&dir, &project_id, &loaded)?;
            delete_session_file_at(&dir, &project_id, session)?;
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "identity",
                    &project_id,
                    "done",
                    &format!("session-terminate: session {session}"),
                );
            }
            let json = serde_json::json!({
                "contract": IDENTITY_CONTRACT_VERSION,
                "project_id": project_id,
                "session": loaded,
                "terminated": true,
            });
            Ok(as_output(
                format,
                format!(
                    "session `{session}` terminated for project `{project_id}`; state=`revoked`"
                ),
                json,
            ))
        }
    }
}

/// Read a secret from an interactive Unix terminal with echo disabled.
fn read_secret(prompt: &str) -> Result<String, ForgeError> {
    use std::io::{IsTerminal, Write};
    if !std::io::stdin().is_terminal() {
        return Err(ForgeError::IdentityInvalid {
            reason: "administrator setup requires an interactive terminal".to_string(),
        });
    }
    #[cfg(unix)]
    unsafe {
        let fd = libc::STDIN_FILENO;
        let mut original: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(fd, &mut original) != 0 {
            return Err(ForgeError::IdentityInvalid {
                reason: "cannot read terminal settings".to_string(),
            });
        }
        let mut hidden = original;
        hidden.c_lflag &= !libc::ECHO;
        if libc::tcsetattr(fd, libc::TCSANOW, &hidden) != 0 {
            return Err(ForgeError::IdentityInvalid {
                reason: "cannot disable terminal echo".to_string(),
            });
        }
        print!("{prompt}");
        let _ = std::io::stdout().flush();
        let mut value = String::new();
        let read = std::io::stdin().read_line(&mut value);
        let _ = libc::tcsetattr(fd, libc::TCSANOW, &original);
        println!();
        read.map_err(|_| ForgeError::IdentityInvalid {
            reason: "cannot read password".to_string(),
        })?;
        Ok(value.trim_end_matches(['\r', '\n']).to_string())
    }
    #[cfg(not(unix))]
    {
        let _ = prompt;
        Err(ForgeError::IdentityInvalid {
            reason: "hidden password setup is unsupported on this platform".to_string(),
        })
    }
}
