//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::constants::{JSONRPC_VERSION, MCP_CONTRACT_VERSION};

/// Transport-level parse error. Never escapes the transport:
/// the transport renders it as a JSON-RPC `parse_error`
/// response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpProtocolError {
    Empty,
    Parse(String),
}
/// Classification of a single MCP tool. Drives the mutating
/// tool boundary, the authorization check, and the rendering
/// of side effects in the response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolKind {
    /// Read-only: never mutates project state.
    ReadOnly,
    /// Mutating: writes files inside the project scope.
    Mutating,
    /// ExternalWrite: issues a network side effect.
    ExternalWrite,
}
impl McpToolKind {
    pub fn label(&self) -> &'static str {
        match self {
            McpToolKind::ReadOnly => "read_only",
            McpToolKind::Mutating => "mutating",
            McpToolKind::ExternalWrite => "external_write",
        }
    }
}
/// JSON-RPC 2.0 error object. `code` is the standard
/// JSON-RPC integer code, `message` is the human-readable
/// summary and `data` carries the structured Core error
/// (with the stable `code()` from `ForgeError`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}
impl McpRpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}
/// JSON-RPC 2.0 request envelope. `id` is optional because
/// notification requests (without `id`) are valid JSON-RPC
/// but the surface returns an explicit error for them so the
/// caller does not silently lose work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}
impl McpRequest {
    /// Parse a single line of stdin. The line must contain a
    /// single JSON object; multiple objects on one line or
    /// partial JSON are refused with a structured parse error.
    pub fn parse_line(line: &str) -> Result<Self, McpProtocolError> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Err(McpProtocolError::Empty);
        }
        let value: Value = serde_json::from_str(trimmed)
            .map_err(|err| McpProtocolError::Parse(err.to_string()))?;
        if !value.is_object() {
            return Err(McpProtocolError::Parse(
                "request payload must be a JSON object".to_string(),
            ));
        }
        let request: McpRequest = serde_json::from_value(value)
            .map_err(|err| McpProtocolError::Parse(err.to_string()))?;
        if request.jsonrpc != JSONRPC_VERSION {
            return Err(McpProtocolError::Parse(format!(
                "jsonrpc must be `{JSONRPC_VERSION}`; got `{}`",
                request.jsonrpc
            )));
        }
        if request.method.is_empty() {
            return Err(McpProtocolError::Parse(
                "method must be a non-empty string".to_string(),
            ));
        }
        Ok(request)
    }
}
/// One declared MCP tool. The descriptor is the source of
/// truth for the tool name, the human description, the kind
/// (driving the mutating boundary), and the JSON Schema the
/// request payload must satisfy before Core is called.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpToolDescriptor {
    pub name: String,
    pub kind: McpToolKind,
    pub description: String,
    pub contract: String,
    pub input_schema: Value,
}
impl McpToolDescriptor {
    pub(super) fn read_only(name: &str, description: &str, input_schema: Value) -> Self {
        Self {
            name: name.to_string(),
            kind: McpToolKind::ReadOnly,
            description: description.to_string(),
            contract: MCP_CONTRACT_VERSION.to_string(),
            input_schema,
        }
    }
    pub(super) fn mutating(name: &str, description: &str, input_schema: Value) -> Self {
        Self {
            name: name.to_string(),
            kind: McpToolKind::Mutating,
            description: description.to_string(),
            contract: MCP_CONTRACT_VERSION.to_string(),
            input_schema,
        }
    }
    pub(super) fn external_write(name: &str, description: &str, input_schema: Value) -> Self {
        Self {
            name: name.to_string(),
            kind: McpToolKind::ExternalWrite,
            description: description.to_string(),
            contract: MCP_CONTRACT_VERSION.to_string(),
            input_schema,
        }
    }
}
/// JSON-RPC 2.0 response envelope. The transport always
/// emits the matching id (or `null` when the request did not
/// carry one) so the caller can correlate responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpRpcError>,
}
impl McpResponse {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }
    pub fn failure(id: Value, error: McpRpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
    /// Render the response as a single line of JSON for
    /// stdout. Pretty-printing is avoided to keep one
    /// request→one response→one line semantics.
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).expect("MCP response is always serializable")
    }
}

/// Standard JSON-RPC 2.0 error codes plus the Forge surface
/// codes for transport-level failures.
pub mod rpc_code {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const TOOL_UNAUTHORIZED: i32 = -32010;
    pub const TOOL_MISSING: i32 = -32011;
    pub const TOOL_REFUSED: i32 = -32012;
}
