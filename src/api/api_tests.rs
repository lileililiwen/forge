//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use std::net::{IpAddr, Ipv4Addr};

use super::contract::{API_SYNTHETIC_PROJECT, MAX_BODY_BYTES};
use super::model::{ApiConfig, ApiError, Route};
use super::router::{err_status, is_mutating, required_permission, route_request};
use super::server::{parse_request, request_hash};

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    #[test]
    fn routing_matches_healthz_and_project_routes() {
        assert!(matches!(
            route_request("GET", "/healthz"),
            Some(Route::Healthz)
        ));
        assert!(matches!(
            route_request("GET", "/v1/projects"),
            Some(Route::ListProjects)
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects"),
            Some(Route::CreateProject)
        ));
        assert!(matches!(
            route_request("GET", "/v1/projects/rust-web"),
            Some(Route::InspectProject { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects/rust-web/doctor"),
            Some(Route::Doctor { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("GET", "/v1/projects/rust-web/governance"),
            Some(Route::Governance { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects/rust-web/features"),
            Some(Route::AddFeature { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects/rust-web/upgrade"),
            Some(Route::UpgradeProject { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects/rust-web/specs"),
            Some(Route::GenerateSpec { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects/rust-web/agents"),
            Some(Route::AgentTransition { ref id }) if id == "rust-web"
        ));
        assert!(matches!(
            route_request("POST", "/v1/projects/rust-web/deployments"),
            Some(Route::ApplyDeployment { ref id }) if id == "rust-web"
        ));
        assert_eq!(
            route_request("POST", "/v1/publish/github"),
            Some(Route::GitHubPush)
        );
        assert!(matches!(
            route_request("GET", "/v1/operations/42"),
            Some(Route::GetOperation { op_id: 42 })
        ));
    }

    #[test]
    fn routing_rejects_unknown_paths() {
        assert!(route_request("GET", "/v2/projects").is_none());
        assert!(route_request("GET", "/nope").is_none());
        assert!(route_request("GET", "/v1/operations/abc").is_none());
    }

    #[test]
    fn parse_request_extracts_method_path_and_headers() {
        let raw = b"GET /v1/projects HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer deadbeef\r\nIdempotency-Key: abc-123\r\nContent-Length: 0\r\n\r\n";
        let req = parse_request(raw, None, MAX_BODY_BYTES).unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/v1/projects");
        assert_eq!(req.bearer_token.as_deref(), Some("deadbeef"));
        assert_eq!(req.idempotency_key.as_deref(), Some("abc-123"));
    }

    #[test]
    fn parse_request_caps_body_at_max_bytes() {
        let raw = b"POST /v1/projects HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello";
        let err = parse_request(raw, None, 4).unwrap_err();
        assert!(matches!(err, ApiError::BodyTooLarge { .. }));
    }

    #[test]
    fn err_status_maps_unauthorized_and_conflict_codes() {
        assert_eq!(
            err_status(&ForgeError::ApiUnauthorized { reason: "x".into() }),
            401
        );
        assert_eq!(
            err_status(&ForgeError::ApiProjectMismatch { reason: "x".into() }),
            403
        );
        assert_eq!(
            err_status(&ForgeError::IdempotencyKeyConflict { reason: "x".into() }),
            409
        );
        assert_eq!(err_status(&ForgeError::PushConfirmRequired), 409);
        assert_eq!(
            err_status(&ForgeError::UnknownProject { query: "x".into() }),
            400
        );
    }

    #[test]
    fn healthz_route_does_not_require_authorization() {
        // `GET /healthz` should render the contract version
        // even when no bearer token is supplied.
        let raw = b"GET /healthz HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let request = parse_request(raw, None, MAX_BODY_BYTES).unwrap();
        assert!(request.bearer_token.is_none());
        assert_eq!(
            route_request(&request.method, &request.path),
            Some(Route::Healthz)
        );
    }

    #[test]
    fn required_permission_is_admin_for_mutating_routes() {
        assert_eq!(required_permission(&Route::Healthz), None);
        assert_eq!(required_permission(&Route::ListProjects), None);
        assert_eq!(
            required_permission(&Route::InspectProject { id: "a".into() }),
            None
        );
        assert_eq!(
            required_permission(&Route::CreateProject),
            Some("admin:access")
        );
        assert_eq!(
            required_permission(&Route::AddFeature { id: "a".into() }),
            Some("admin:access")
        );
        assert_eq!(
            required_permission(&Route::ApplyDeployment { id: "a".into() }),
            Some("admin:access")
        );
    }

    #[test]
    fn is_mutating_classifies_routes() {
        assert!(!is_mutating(&Route::Healthz));
        assert!(!is_mutating(&Route::ListProjects));
        assert!(!is_mutating(&Route::GetOperation { op_id: 1 }));
        assert!(is_mutating(&Route::CreateProject));
        assert!(is_mutating(&Route::AddFeature { id: "a".into() }));
        assert!(is_mutating(&Route::ApplyDeployment { id: "a".into() }));
    }

    #[test]
    fn api_config_from_env_overrides_bind_and_port() {
        // Use a unique environment override and ensure
        // the parser picks it up.
        std::env::set_var("FORGE_API_BIND", "127.0.0.1");
        std::env::set_var("FORGE_API_PORT", "9999");
        let cfg = ApiConfig::from_env();
        assert_eq!(cfg.bind, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(cfg.port, 9999);
        std::env::remove_var("FORGE_API_BIND");
        std::env::remove_var("FORGE_API_PORT");
    }

    #[test]
    fn synthetic_project_constant_is_stable() {
        assert_eq!(API_SYNTHETIC_PROJECT, "__api__");
    }

    #[test]
    fn request_hash_is_deterministic_per_request() {
        let raw = b"POST /v1/projects/rust-web/features HTTP/1.1\r\nContent-Length: 17\r\n\r\n{\"feature\":\"x\"}";
        let request = parse_request(raw, None, MAX_BODY_BYTES).unwrap();
        let first = request_hash(&request);
        let second = request_hash(&request);
        assert_eq!(first, second);
        let mut other = request.clone();
        other.method = "GET".to_string();
        assert_ne!(first, request_hash(&other));
    }
}
