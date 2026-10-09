//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::CANONICAL_MANIFEST;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs;
use std::path::Path;

use super::constants::MAX_EVIDENCE_CHARS;
use super::model::{
    AgentProvider, AgentSession, RuntimeBacking, SessionState, SessionTransition, TransitionRecord,
};
use super::sessions::{classify_sisyphusfy_output, read_session, run_spec, write_session};
use super::transitions::{
    apply_transition, bounded_evidence, first_json_document, map_ariadex_status, new_session,
    parse_ariadex_version, resolve_binary_path, sanitize_handle, validate_session_id,
};

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_manifest(dir: &Path, id: &str) {
        let body = format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
        );
        fs::write(dir.join(CANONICAL_MANIFEST), body).unwrap();
    }

    #[test]
    fn session_id_validation_matches_project_rules() {
        for id in ["a", "agent-1", "session-001"] {
            assert!(validate_session_id(id).is_ok(), "{id}");
        }
        for id in ["", "Agent-1", "1abc", "-abc", "abc-", "a--b", "a_b", "a b"] {
            assert!(validate_session_id(id).is_err(), "{id}");
        }
    }

    #[test]
    fn start_records_state_for_present_or_missing_provider() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "demo");
        let session = new_session(
            tmp.path(),
            "sess-1",
            AgentProvider::Codex,
            "",
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        // The test environment may or may not include a `codex`
        // binary on PATH; the contract guarantees that the
        // transition either succeeds with `active` evidence or
        // fails with `agent-unavailable`, never with a simulated
        // pause/takeover state.
        match apply_transition(
            session,
            SessionTransition::Start,
            DateTime::<Utc>::from_timestamp(1, 0).unwrap(),
        ) {
            Ok(outcome) => {
                assert_eq!(outcome.state, SessionState::Active);
                assert!(outcome
                    .evidence
                    .iter()
                    .any(|e| e.contains("provider: codex")));
            }
            Err(err) => {
                assert_eq!(err.code(), "agent-unavailable");
            }
        }
    }

    #[test]
    fn pause_and_takeover_report_unsupported_state() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "pause-app");
        // Build a synthetic session in the `Active` state without
        // requiring a real provider binary on PATH.
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let mut session =
            new_session(tmp.path(), "sess-pause", AgentProvider::Opencode, "", now).unwrap();
        session.state = SessionState::Active;
        let outcome = apply_transition(
            session,
            SessionTransition::Pause,
            DateTime::<Utc>::from_timestamp(1, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(outcome.state, SessionState::Unsupported);
        assert!(outcome
            .evidence
            .iter()
            .any(|e| e.contains("pause primitive")));
        assert!(outcome.next_step.is_some());
        let outcome = apply_transition(
            outcome.session,
            SessionTransition::Takeover,
            DateTime::<Utc>::from_timestamp(2, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(outcome.state, SessionState::Unsupported);
    }

    #[test]
    fn restart_preserves_prior_state_and_records_active() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "restart-app");
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let mut session = new_session(
            tmp.path(),
            "sess-restart",
            AgentProvider::Codex,
            "spec-restart-abcdef",
            now,
        )
        .unwrap();
        session.state = SessionState::Active;
        let outcome = apply_transition(
            session,
            SessionTransition::Restart,
            DateTime::<Utc>::from_timestamp(1, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(outcome.state, SessionState::Active);
        assert_eq!(outcome.session.transitions.len(), 1);
        assert_eq!(outcome.session.spec_id, "spec-restart-abcdef");
    }

    #[test]
    fn run_spec_refuses_when_bound_spec_missing() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "run-app");
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let mut session = new_session(
            tmp.path(),
            "sess-run",
            AgentProvider::Codex,
            "spec-missing-123456",
            now,
        )
        .unwrap();
        session.state = SessionState::Active;
        write_session(tmp.path(), &session).unwrap();
        let err = run_spec(
            tmp.path(),
            "sess-run",
            None,
            DateTime::<Utc>::from_timestamp(1, 0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err.code(), "spec-invalid");
        // Session file is preserved with the original spec id so
        // the boundary scenario (session that ended without
        // verification) remains observable.
        let reread = read_session(tmp.path(), "sess-run").unwrap().unwrap();
        assert_eq!(reread.spec_id, "spec-missing-123456");
        assert_eq!(reread.state, SessionState::Active);
    }

    #[test]
    fn write_and_read_session_roundtrips_transitions() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "roundtrip");
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let mut session =
            new_session(tmp.path(), "sess-rt", AgentProvider::Codex, "", now).unwrap();
        session.state = SessionState::Active;
        session.transitions.push(TransitionRecord {
            kind: SessionTransition::Start,
            state: SessionState::Active,
            at: now,
            evidence: vec!["provider: codex".to_string()],
            note: "start".to_string(),
        });
        let files = write_session(tmp.path(), &session).unwrap();
        assert_eq!(files.len(), 2);
        let read_back = read_session(tmp.path(), "sess-rt").unwrap().unwrap();
        assert_eq!(read_back.transitions.len(), 1);
        assert_eq!(read_back.transitions[0].kind, SessionTransition::Start);
    }

    #[test]
    fn legacy_session_files_without_backing_deserialize_unchanged() {
        // A session.json written before `supervised-agent-adapters`
        // carries no `backing` key at all; the serde default must
        // keep it loadable (and the cross-surface suites keep
        // writing exactly this shape).
        let legacy = r#"{"contract":"0.1.0","session_id":"sess-old","project_id":"old","project_path":"/tmp/old","provider":"codex","spec_id":"","state":"active","started_at":"1970-01-01T00:00:00+00:00","last_transition_at":"1970-01-01T00:00:00+00:00","transitions":[]}"#;
        let session: AgentSession = serde_json::from_str(legacy).expect("legacy session loads");
        assert_eq!(session.session_id, "sess-old");
        assert!(session.backing.is_none());
    }

    #[test]
    fn backing_roundtrips_through_session_json() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "backing-app");
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let mut session =
            new_session(tmp.path(), "sess-b", AgentProvider::Ariadex, "", now).unwrap();
        session.state = SessionState::Active;
        session.backing = Some(RuntimeBacking {
            runtime: "ariadex".to_string(),
            handle: "cafe1234abcd".to_string(),
            adapter_version: "0.1.0".to_string(),
        });
        write_session(tmp.path(), &session).unwrap();
        let read_back = read_session(tmp.path(), "sess-b").unwrap().unwrap();
        let backing = read_back.backing.expect("backing persisted");
        assert_eq!(backing.runtime, "ariadex");
        assert_eq!(backing.handle, "cafe1234abcd");
        assert_eq!(backing.adapter_version, "0.1.0");
        assert_eq!(read_back.provider, AgentProvider::Ariadex);
    }

    #[test]
    fn provider_enum_supervised_vocabulary() {
        assert_eq!(AgentProvider::Ariadex.label(), "ariadex");
        assert_eq!(AgentProvider::Ariadex.binary(), "ariadex");
        assert!(AgentProvider::Ariadex.is_supervised());
        assert!(!AgentProvider::Opencode.is_supervised());
        assert!(!AgentProvider::Codex.is_supervised());
        let serialized = serde_json::to_string(&AgentProvider::Ariadex).unwrap();
        assert_eq!(serialized, "\"ariadex\"");
    }

    #[test]
    fn ordered_binary_resolution_prefers_env_then_path() {
        let tmp = TempDir::new().unwrap();
        let dir_a = tmp.path().join("a");
        let dir_b = tmp.path().join("b");
        fs::create_dir_all(&dir_a).unwrap();
        fs::create_dir_all(&dir_b).unwrap();
        let in_a = dir_a.join("ariadex");
        fs::write(&in_a, b"#!/bin/sh\n").unwrap();

        // PATH search finds the first executable-shaped candidate.
        let (resolved, attempts) = resolve_binary_path(
            "FORGE_ARIADEX_BIN",
            None,
            &[dir_a.clone(), dir_b.clone()],
            "ariadex",
        )
        .expect("resolves via PATH");
        assert_eq!(resolved, in_a);
        assert!(
            attempts.iter().any(|a| a.contains("PATH")),
            "attempts={attempts:?}"
        );

        // The env override wins over PATH when it names a real file,
        // and is recorded as the attempt.
        let pinned = tmp.path().join("pinned-ariadex");
        fs::write(&pinned, b"#!/bin/sh\n").unwrap();
        let env_value = std::ffi::OsString::from(&pinned);
        let (resolved, attempts) = resolve_binary_path(
            "FORGE_ARIADEX_BIN",
            Some(&env_value),
            std::slice::from_ref(&dir_a),
            "ariadex",
        )
        .expect("env override resolves");
        assert_eq!(resolved, pinned);
        assert!(
            attempts[0].contains("FORGE_ARIADEX_BIN"),
            "attempts={attempts:?}"
        );

        // A dead env path records the miss in the attempt list and
        // falls through (it never silently runs a nonexistent pin).
        let dead = std::ffi::OsString::from("/nonexistent/ariadex");
        let err = resolve_binary_path(
            "FORGE_ARIADEX_BIN",
            Some(&dead),
            std::slice::from_ref(&dir_b),
            "ariadex",
        )
        .unwrap_err();
        assert!(
            err.iter()
                .any(|a| a.contains("/nonexistent/ariadex") && a.contains("not found")),
            "attempts={err:?}"
        );

        // An empty env value is ignored, not trusted.
        let empty = std::ffi::OsString::from("  ");
        let (resolved, _attempts) = resolve_binary_path(
            "FORGE_ARIADEX_BIN",
            Some(&empty),
            std::slice::from_ref(&dir_a),
            "ariadex",
        )
        .expect("falls through to PATH");
        assert_eq!(resolved, in_a);

        // Nothing found → the failure carries the attempt list.
        let err = resolve_binary_path("FORGE_ARIADEX_BIN", None, &[dir_b], "ariadex").unwrap_err();
        assert!(
            err.iter().any(|a| a.contains("not found")),
            "attempts={err:?}"
        );
    }

    #[test]
    fn first_json_document_survives_trailing_text_and_double_docs() {
        // `status --json` may append `blocker ...` lines after the document.
        let text = "{\n  \"mode\": \"AUTO\",\n  \"session\": \"abc123\"\n}\nblocker u-1: stuck\n";
        let doc = first_json_document(text).expect("first doc");
        assert_eq!(doc["mode"], "AUTO");
        // Duplicate-owner `start --json` prints two documents back-to-back.
        let double =
            "{\"duplicate\": true, \"started\": false}\n{\"ok\": true, \"reused\": true}\n";
        let doc = first_json_document(double).expect("first of two");
        assert_eq!(doc["duplicate"], true);
        // Compact refusals parse; plain text yields None.
        assert!(first_json_document("{\"ok\": false, \"error\": \"no daemon\"}").is_some());
        assert!(first_json_document("managed runtime started").is_none());
        assert!(first_json_document("").is_none());
    }

    #[test]
    fn version_probe_parses_documented_surface_only() {
        assert_eq!(
            parse_ariadex_version("ariadex 0.1.0\n").as_deref(),
            Some("0.1.0")
        );
        assert_eq!(
            parse_ariadex_version("ariadex 0.1.0+g1a2b3c4-dirty\n").as_deref(),
            Some("0.1.0+g1a2b3c4-dirty")
        );
        assert_eq!(parse_ariadex_version("ariadex dev-build\n"), None);
        assert_eq!(parse_ariadex_version("0.1.0\n"), None);
        assert_eq!(parse_ariadex_version("ariadex\n"), None);
        assert_eq!(parse_ariadex_version(""), None);
    }

    #[test]
    fn status_mapping_never_claims_active_without_a_live_daemon() {
        let alive_auto: Value =
            serde_json::json!({"daemon": {"alive": true, "mode": "AUTO", "session": "cafe12"}});
        let (state, mode, handle, daemon) = map_ariadex_status(&alive_auto);
        assert_eq!(state, SessionState::Active);
        assert_eq!(mode, "AUTO");
        assert_eq!(handle.as_deref(), Some("cafe12"));
        assert_eq!(daemon, "alive");

        let alive_pause: Value =
            serde_json::json!({"daemon": {"alive": true, "mode": "PAUSE", "session": "cafe12"}});
        assert_eq!(map_ariadex_status(&alive_pause).0, SessionState::Paused);

        // MANUAL is a real mode but not an automated-running state.
        let alive_manual: Value = serde_json::json!({"daemon": {"alive": true, "mode": "MANUAL"}});
        let (state, _, _, _) = map_ariadex_status(&alive_manual);
        assert_eq!(state, SessionState::Disconnected);

        let stale: Value =
            serde_json::json!({"daemon": {"alive": false, "mode": "AUTO", "session": "cafe12"}});
        let (state, _, _, daemon) = map_ariadex_status(&stale);
        assert_eq!(state, SessionState::Disconnected);
        assert_eq!(daemon, "stale");

        // Local (no daemon) view: the manager is gone → disconnected
        // even though durable mode reads AUTO.
        let local: Value =
            serde_json::json!({"mode": "AUTO", "session": "cafe12", "agent": "opencode"});
        let (state, mode, _, daemon) = map_ariadex_status(&local);
        assert_eq!(state, SessionState::Disconnected);
        assert_eq!(mode, "AUTO");
        assert_eq!(daemon, "absent");

        // Unknown shapes never map to active.
        let junk: Value = serde_json::json!({"unexpected": true});
        assert_eq!(map_ariadex_status(&junk).0, SessionState::Disconnected);
    }

    #[test]
    fn sisyphusfy_verdict_requires_independent_verification() {
        // clean exit + complete + passing verification → done
        let done = serde_json::json!({
            "stop_reason": "complete", "iterations": 3,
            "verification": {"status": "success", "source": "configured"}
        });
        let (verdict, evidence, reason) =
            classify_sisyphusfy_output(true, &serde_json::to_string(&done).unwrap());
        assert_eq!(verdict, "done");
        assert!(evidence.iter().any(|e| e == "stop_reason: complete"));
        assert!(reason.contains("verification passed"), "reason={reason}");

        // complete without a passing verification → unverified
        let unverified = serde_json::json!({
            "stop_reason": "complete", "iterations": 1,
            "verification": {"status": "skipped", "source": "unavailable"}
        });
        let (verdict, _, _) =
            classify_sisyphusfy_output(true, &serde_json::to_string(&unverified).unwrap());
        assert_eq!(verdict, "unverified");

        // blocked iteration → partial naming the supervisor's reason
        let blocked = serde_json::json!({
            "stop_reason": "blocked", "iterations": 2,
            "verification": {"status": "skipped", "source": "unavailable"},
            "blocked_reason": ["NEED_PERMISSION"]
        });
        let (verdict, evidence, reason) =
            classify_sisyphusfy_output(false, &serde_json::to_string(&blocked).unwrap());
        assert_eq!(verdict, "partial");
        assert!(reason.contains("blocked"), "reason={reason}");
        assert!(reason.contains("NEED_PERMISSION"), "reason={reason}");
        assert!(evidence
            .iter()
            .any(|e| e == "supervisor-reason: NEED_PERMISSION"));

        // agent_failed → partial; verification_failed → unverified
        let failed = serde_json::json!({
            "stop_reason": "agent_failed", "iterations": 1,
            "verification": {"status": "skipped", "source": "unavailable"},
            "agent_error": {"message": "exited with code 1"}
        });
        let (verdict, _, reason) =
            classify_sisyphusfy_output(false, &serde_json::to_string(&failed).unwrap());
        assert_eq!(verdict, "partial");
        assert!(reason.contains("exited with code 1"), "reason={reason}");
        let verify_failed = serde_json::json!({
            "stop_reason": "verification_failed", "iterations": 1,
            "verification": {"status": "failure", "source": "configured"}
        });
        let (verdict, _, _) =
            classify_sisyphusfy_output(false, &serde_json::to_string(&verify_failed).unwrap());
        assert_eq!(verdict, "unverified");

        // unparsable outcome → unverified, never done, raw failure bounded
        let (verdict, evidence, _) = classify_sisyphusfy_output(false, "not a json document\n");
        assert_eq!(verdict, "unverified");
        assert!(evidence
            .iter()
            .any(|e| e.starts_with("raw-outcome: not a json document")));

        // exit/document disagreement → unverified (a stray exit 0 with a
        // non-complete stop reason never upgrades)
        let disagree = serde_json::json!({"stop_reason": "complete", "iterations": 9});
        let (verdict, _, reason) =
            classify_sisyphusfy_output(false, &serde_json::to_string(&disagree).unwrap());
        assert_eq!(verdict, "unverified");
        assert!(reason.contains("disagree"), "reason={reason}");
    }

    #[test]
    fn evidence_is_redacted_and_char_bounded() {
        let secret = "daemon token AKIA1234567890ABCDEF restarted";
        let bounded = bounded_evidence(secret);
        assert!(
            !bounded.contains("AKIA1234567890ABCDEF"),
            "bounded={bounded}"
        );
        let long = "x".repeat(MAX_EVIDENCE_CHARS + 200);
        let bounded = bounded_evidence(&long);
        assert!(bounded.chars().count() <= MAX_EVIDENCE_CHARS + 1);
        assert!(bounded.ends_with('…'));
        assert_eq!(sanitize_handle("  abc\u{7}def  "), "abcdef");
    }
}
