//! Forge-owned concern-check command facade.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use super::commands::ConcernCommands;

const CHECKER_RELATIVE_PATH: &str = "modules/workspace-governance/scripts/check_concern.py";
const QUALITY_CHECKER_RELATIVE_PATH: &str =
    "modules/workspace-governance/scripts/product_code_quality.py";
const INSTALLED_CHECKER_RELATIVE_PATH: &str = "share/forge/governance/check_concern.py";
const INSTALLED_QUALITY_CHECKER_RELATIVE_PATH: &str =
    "share/forge/governance/product_code_quality.py";
const SUPPORTED_CONCERNS: &[&str] = &[
    "accessibility",
    "fixtures",
    "placeholder-threshold",
    "product-code-boundary",
    "privacy",
    "provenance",
    "schemas",
];

fn checker_path(
    executable: &Path,
    source_root: &Path,
    quality_check: bool,
) -> Result<PathBuf, String> {
    let (source_relative, installed_relative) = if quality_check {
        (
            QUALITY_CHECKER_RELATIVE_PATH,
            INSTALLED_QUALITY_CHECKER_RELATIVE_PATH,
        )
    } else {
        (CHECKER_RELATIVE_PATH, INSTALLED_CHECKER_RELATIVE_PATH)
    };
    let source = source_root.join(source_relative);
    // A build cache may relocate the executable outside the checkout, so the
    // prefix test below would miss the source resource. When the checkout that
    // built the binary still carries it, use it. A portable binary on another
    // host has no source resource and falls through to the installed path.
    if source.is_file() {
        return Ok(source);
    }

    let prefix = executable
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "cannot resolve Forge installation prefix".to_string())?;
    let installed = prefix.join(installed_relative);
    if installed.is_file() {
        return Ok(installed);
    }
    Err(format!(
        "Forge concern checker resource is missing at {}",
        installed.display()
    ))
}

pub(crate) fn cmd_concern(command: &ConcernCommands) -> ExitCode {
    match command {
        ConcernCommands::Check { id, project_root } => run_check(id, project_root),
    }
}

fn run_check(id: &str, project_root: &Path) -> ExitCode {
    if !SUPPORTED_CONCERNS.contains(&id) {
        eprintln!(
            "forge concern: unsupported concern `{id}` (expected one of: {})",
            SUPPORTED_CONCERNS.join(", ")
        );
        return ExitCode::from(2);
    }
    let project_root = match project_root.canonicalize() {
        Ok(path) if path.is_dir() => path,
        Ok(path) => {
            eprintln!(
                "forge concern: project root is not a directory: {}",
                path.display()
            );
            return ExitCode::from(2);
        }
        Err(error) => {
            eprintln!(
                "forge concern: cannot resolve project root {}: {error}",
                project_root.display()
            );
            return ExitCode::from(2);
        }
    };

    let executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("forge concern: cannot resolve executable path: {error}");
            return ExitCode::FAILURE;
        }
    };
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let quality_check = matches!(id, "placeholder-threshold" | "product-code-boundary");
    let checker = match checker_path(&executable, &source_root, quality_check) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("forge concern: {error}");
            return ExitCode::from(127);
        }
    };

    let mut command = Command::new("python3");
    command.arg(checker);
    if quality_check {
        command.arg(project_root).arg("--json");
    } else {
        command.arg(id).arg(project_root);
    }
    match command.status() {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1).clamp(0, 255) as u8),
        Err(error) => {
            eprintln!("forge concern: cannot start python3: {error}");
            ExitCode::from(127)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checker_path_uses_the_source_resource_for_source_binaries() {
        let source = tempfile::tempdir().unwrap();
        let script = source.path().join(CHECKER_RELATIVE_PATH);
        std::fs::create_dir_all(script.parent().unwrap()).unwrap();
        std::fs::write(&script, "# checker\n").unwrap();
        let executable = source.path().join("target/debug/forge");

        assert_eq!(
            checker_path(&executable, source.path(), false).unwrap(),
            script
        );
    }

    #[test]
    fn checker_path_uses_the_source_resource_when_the_executable_is_relocated() {
        let source = tempfile::tempdir().unwrap();
        let script = source.path().join(CHECKER_RELATIVE_PATH);
        std::fs::create_dir_all(script.parent().unwrap()).unwrap();
        std::fs::write(&script, "# checker\n").unwrap();
        let cache = tempfile::tempdir().unwrap();
        let executable = cache.path().join("debug/forge");

        assert_eq!(
            checker_path(&executable, source.path(), false).unwrap(),
            script
        );
    }

    #[test]
    fn checker_path_uses_installed_resource_for_binary_prefix() {
        let prefix = tempfile::tempdir().unwrap();
        let executable = prefix.path().join("bin/forge");
        let installed = prefix.path().join(INSTALLED_CHECKER_RELATIVE_PATH);
        std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
        std::fs::write(&installed, "# checker\n").unwrap();

        assert_eq!(
            checker_path(&executable, Path::new("/source"), false).unwrap(),
            installed
        );
    }
}
