//! Minimal Rust static web server for Forge-owned browser assets.
//!
//! This listener serves only files from the standalone `frontend/` directory.
//! It has no API routes and does not access the registry or authentication DB.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_REQUEST_LINE: usize = 8192;

/// Serve the standalone frontend until the process receives Ctrl-C.
pub fn serve(bind: IpAddr, port: u16, root: &Path) -> Result<usize, String> {
    let root = root
        .canonicalize()
        .map_err(|err| format!("frontend directory is unavailable: {err}"))?;
    let address = SocketAddr::new(bind, port);
    let listener = TcpListener::bind(address)
        .map_err(|err| format!("cannot bind web listener on {address}: {err}"))?;
    let mut accepted = 0usize;
    for incoming in listener.incoming() {
        match incoming {
            Ok(mut stream) => {
                accepted += 1;
                let _ = stream.set_read_timeout(Some(REQUEST_TIMEOUT));
                let _ = stream.set_write_timeout(Some(REQUEST_TIMEOUT));
                let _ = serve_one(&mut stream, &root);
            }
            Err(err) => eprintln!("forge web: accept error: {err}"),
        }
    }
    Ok(accepted)
}

fn serve_one(stream: &mut TcpStream, root: &Path) -> std::io::Result<()> {
    let reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader
        .take(MAX_REQUEST_LINE as u64)
        .read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    let path = target.split('?').next().unwrap_or("");

    if method != "GET" && method != "HEAD" {
        return write_response(
            stream,
            405,
            "text/plain; charset=utf-8",
            b"Method Not Allowed\n",
            method == "HEAD",
        );
    }
    let Some(file_name) = asset_name(path) else {
        return write_response(
            stream,
            404,
            "text/plain; charset=utf-8",
            b"Not Found\n",
            method == "HEAD",
        );
    };
    let file_path: PathBuf = root.join(file_name);
    let bytes = match fs::read(file_path) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return write_response(
                stream,
                404,
                "text/plain; charset=utf-8",
                b"Not Found\n",
                method == "HEAD",
            );
        }
        Err(err) => return Err(err),
    };
    write_response(
        stream,
        200,
        content_type(file_name),
        &bytes,
        method == "HEAD",
    )
}

fn asset_name(path: &str) -> Option<&'static str> {
    match path {
        "/" | "/login.html" => Some("login.html"),
        "/index.html" => Some("index.html"),
        "/styles.css" => Some("styles.css"),
        "/config.js" => Some("config.js"),
        "/app.js" => Some("app.js"),
        _ => None,
    }
}

fn content_type(file_name: &str) -> &'static str {
    match file_name.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\n\r\n",
        body.len()
    )?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_server_has_an_allowlist_and_serves_login_at_root() {
        assert_eq!(asset_name("/"), Some("login.html"));
        assert_eq!(asset_name("/login.html"), Some("login.html"));
        assert_eq!(asset_name("/app.js"), Some("app.js"));
        assert_eq!(asset_name("/../src/main.rs"), None);
        assert_eq!(asset_name("/v1/admin/session"), None);
        assert_eq!(content_type("styles.css"), "text/css; charset=utf-8");
    }
}
