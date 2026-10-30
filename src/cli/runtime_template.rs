use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn valid_bundle(root: &Path) -> bool {
    root.join("runtime_templates.py").is_file()
        && root.join("templates/runtime/registry.json").is_file()
        && root.join("manifest.json").is_file()
}

fn bundle_root() -> Result<PathBuf, String> {
    if let Some(configured) = std::env::var_os("FORGE_RUNTIME_TEMPLATES_HOME") {
        let root = PathBuf::from(configured);
        return if valid_bundle(&root) {
            Ok(root)
        } else {
            Err(format!(
                "FORGE_RUNTIME_TEMPLATES_HOME does not contain a runtime bundle, registry, and manifest: {}",
                root.display()
            ))
        };
    }

    let executable = std::env::current_exe().map_err(|error| {
        format!("cannot resolve Forge executable path for bundled templates: {error}")
    })?;
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = source_root.join("scripts/governance/runtime-templates");
    // A build cache may relocate the executable outside the checkout. Prefer
    // the bundle the checkout carries; a portable binary on another host has
    // no source bundle and falls through to its installed prefix.
    if valid_bundle(&source) {
        return Ok(source);
    }
    let prefix = executable
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "cannot resolve Forge installation prefix".to_string())?;
    let installed = prefix.join("share/forge/runtime-templates");
    if valid_bundle(&installed) {
        return Ok(installed);
    }
    Err(format!(
        "runtime template bundle is missing at {}",
        installed.display()
    ))
}

pub fn cmd_runtime_template(args: &[String]) -> ExitCode {
    let root = match bundle_root() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("forge runtime-template: {error}");
            return ExitCode::FAILURE;
        }
    };
    let script = root.join("runtime_templates.py");
    let mut command = Command::new("python3");
    command
        .arg(script)
        .args(args)
        .current_dir(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    match command.status() {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1).clamp(0, 255) as u8),
        Err(error) => {
            eprintln!("forge runtime-template: cannot run python3: {error}");
            ExitCode::FAILURE
        }
    }
}
