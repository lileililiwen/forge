//! API serve helper (`api`).
//!
//! `forge api serve` transport handler.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::api::{
    serve as api_serve, ApiConfig, ShutdownSignal, API_CONTRACT_VERSION, API_SYNTHETIC_PROJECT,
};
use forge::core::ForgeError;
use std::path::Path;

use crate::{Format, Output};

pub(super) fn cmd_api_serve(
    db_path: &Path,
    bind: Option<std::net::IpAddr>,
    port: Option<u16>,
    max_body_bytes: Option<usize>,
    format: Format,
) -> Result<Output, ForgeError> {
    let mut config = ApiConfig::from_env();
    if let Some(value) = bind {
        config.bind = value;
    }
    if let Some(value) = port {
        config.port = value;
    }
    if let Some(value) = max_body_bytes {
        config.max_body_bytes = value;
    }
    let shutdown = ShutdownSignal::new();
    let human = format!(
        "forge api serving on http://{addr} (contract {version}, synthetic project id `{synthetic}`); press Ctrl-C to stop",
        addr = config.socket_addr(),
        version = API_CONTRACT_VERSION,
        synthetic = API_SYNTHETIC_PROJECT,
    );
    let json = serde_json::json!({
        "contract": API_CONTRACT_VERSION,
        "bind": config.bind.to_string(),
        "port": config.port,
        "max_body_bytes": config.max_body_bytes,
        "synthetic_project": API_SYNTHETIC_PROJECT,
        "registry": db_path.display().to_string(),
    });
    if matches!(format, Format::Json | Format::Ndjson) {
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("{human}");
    }
    let accepted = api_serve(&config, db_path, shutdown)?;
    let summary = format!("forge api stopped after {accepted} accepted connection(s)");
    let summary_json = serde_json::json!({
        "stopped": true,
        "accepted": accepted,
    });
    match format {
        Format::Human | Format::Table => Ok(Output::Human(summary)),
        Format::Json | Format::Ndjson => Ok(Output::Json(summary_json)),
    }
}
