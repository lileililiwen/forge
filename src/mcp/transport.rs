//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::methods::dispatch;
use super::model::{rpc_code, McpProtocolError, McpRequest, McpResponse, McpRpcError};
use serde_json::Value;
use std::io::{self, BufRead, Write};
use std::path::Path;
/// Run the stdio server loop. Reads JSON-RPC 2.0 requests one
/// per line from stdin, dispatches them through Core, and
/// writes JSON-RPC 2.0 responses one per line to stdout.
/// Diagnostic messages go to stderr; the stdout channel
/// contains only the response envelope so a model can consume
/// the stream without parsing diagnostic noise.
///
/// `reader` and `writer` are split so tests can drive the
/// loop without spawning a child process.
pub fn run_session<R: BufRead, W: Write, E: Write>(
    db_path: Option<&Path>,
    mut reader: R,
    mut writer: W,
    mut diagnostics: E,
) -> io::Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Ok(());
        }
        if line.trim().is_empty() {
            continue;
        }
        let request = match McpRequest::parse_line(&line) {
            Ok(r) => r,
            Err(McpProtocolError::Empty) => continue,
            Err(McpProtocolError::Parse(reason)) => {
                let response = McpResponse::failure(
                    Value::Null,
                    McpRpcError::new(rpc_code::PARSE_ERROR, reason),
                );
                writeln!(writer, "{}", response.to_line())?;
                writer.flush()?;
                continue;
            }
        };
        let id = request.id.clone().unwrap_or(Value::Null);
        match dispatch(db_path, &request) {
            Ok(value) => {
                let response = McpResponse::success(id, value);
                writeln!(writer, "{}", response.to_line())?;
                writer.flush()?;
            }
            Err(err) => {
                // The JSON-RPC response carries the full
                // error so the model can interpret it; the
                // diagnostic stream only sees a redacted
                // summary keyed on the tool name so a
                // credential embedded in the failed Core
                // call does not leak through stderr.
                let response = McpResponse::failure(id.clone(), err);
                let redacted = redact_for_diagnostics(&response, &request.method);
                let _ = writeln!(
                    diagnostics,
                    "mcp error method={} id={} code={} message={}",
                    request.method, id, redacted.code, redacted.message
                );
                writeln!(writer, "{}", response.to_line())?;
                writer.flush()?;
            }
        }
    }
}

/// Build a redaction-safe summary of a JSON-RPC error for
/// the diagnostic stream. The full error is preserved on
/// the JSON-RPC response so the caller can correlate the
/// failure; the diagnostic stream only ever sees the tool
/// name and the structured code.
fn redact_for_diagnostics(response: &McpResponse, method: &str) -> McpRpcError {
    let code = response
        .error
        .as_ref()
        .map(|e| e.code)
        .unwrap_or(rpc_code::INTERNAL_ERROR);
    McpRpcError {
        code,
        message: format!("tool `{method}` failed with code {code}"),
        data: None,
    }
}

/// Convenience: run the server against the host stdin/stdout
/// and stderr.
pub fn serve_stdio(db_path: Option<&Path>) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let reader = stdin.lock();
    let writer = stdout.lock();
    let diagnostics = stderr.lock();
    run_session(db_path, reader, writer, diagnostics)
}
