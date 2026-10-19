//! Thin local runtime entry points over Docker Compose and GitRunByKaru.

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::RunCommands;

const COMPOSE_FILES: [&str; 4] = [
    "compose.yaml",
    "compose.yml",
    "docker-compose.yaml",
    "docker-compose.yml",
];

fn canonical_directory(path: &Path, description: &str) -> Result<PathBuf, ExitCode> {
    let canonical = path.canonicalize().map_err(|error| {
        eprintln!(
            "forge run: cannot resolve {description} `{}`: {error}",
            path.display()
        );
        ExitCode::from(2)
    })?;
    if !canonical.is_dir() {
        eprintln!(
            "forge run: {description} `{}` is not a directory",
            path.display()
        );
        return Err(ExitCode::from(2));
    }
    Ok(canonical)
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

fn child_exit(program: &str, command: &mut Command) -> ExitCode {
    match command.status() {
        Ok(status) => match status.code() {
            Some(code) => ExitCode::from(code.clamp(0, 255) as u8),
            None => {
                eprintln!("forge run: `{program}` terminated without an exit code");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("forge run: failed to start `{program}`: {error}");
            ExitCode::from(127)
        }
    }
}

fn run_compose(
    project_root: &Path,
    file: Option<&Path>,
    profiles: &[String],
    args: &[String],
) -> ExitCode {
    let root = match canonical_directory(project_root, "project root") {
        Ok(root) => root,
        Err(code) => return code,
    };
    let compose_file = match file {
        Some(file) if file.is_absolute() => file.to_path_buf(),
        Some(file) => root.join(file),
        None => match COMPOSE_FILES
            .iter()
            .map(|name| root.join(name))
            .find(|candidate| candidate.is_file())
        {
            Some(file) => file,
            None => {
                eprintln!(
                    "forge run compose: no Compose file found in `{}` (looked for {})",
                    root.display(),
                    COMPOSE_FILES.join(", ")
                );
                return ExitCode::from(2);
            }
        },
    };
    let compose_file = match compose_file.canonicalize() {
        Ok(file) if file.is_file() => file,
        Ok(_) => {
            eprintln!(
                "forge run compose: `{}` is not a file",
                compose_file.display()
            );
            return ExitCode::from(2);
        }
        Err(error) => {
            eprintln!(
                "forge run compose: cannot read `{}`: {error}",
                compose_file.display()
            );
            return ExitCode::from(2);
        }
    };
    let file_arg = compose_file
        .strip_prefix(&root)
        .map(Path::to_path_buf)
        .unwrap_or(compose_file);

    let Some(docker) = find_on_path("docker") else {
        eprintln!(
            "forge run compose: Docker Compose is unavailable; install Docker with Compose support"
        );
        return ExitCode::from(127);
    };

    let mut command = Command::new(docker);
    command
        .current_dir(&root)
        .arg("compose")
        .arg("--project-directory")
        .arg(&root)
        .arg("-f")
        .arg(file_arg);
    for profile in profiles {
        command.arg("--profile").arg(profile);
    }
    if args.is_empty() || args.first().is_some_and(|first| first.starts_with('-')) {
        command.arg("up");
    }
    command.args(args);
    child_exit("docker compose", &mut command)
}

fn run_local(
    path: &Path,
    port: Option<u16>,
    no_open: bool,
    keep: bool,
    docker: bool,
    json: bool,
) -> ExitCode {
    let root = match canonical_directory(path, "application path") {
        Ok(root) => root,
        Err(code) => return code,
    };
    if port == Some(0) {
        eprintln!("forge run local: port must be between 1 and 65535");
        return ExitCode::from(2);
    }
    let Some(runtime) = find_on_path("gitrunbykaru") else {
        eprintln!(
            "forge run local: `gitrunbykaru` is not installed on PATH; install it with `npm install --global gitrunbykaru`"
        );
        return ExitCode::from(127);
    };

    let mut command = Command::new(runtime);
    command.current_dir(&root).arg(&root);
    if let Some(port) = port {
        command.arg("--port").arg(port.to_string());
    }
    if no_open {
        command.arg("--no-open");
    }
    if keep {
        command.arg("--keep");
    }
    if docker {
        command.arg("--docker");
    }
    if json {
        command.arg("--json");
    }
    child_exit("gitrunbykaru", &mut command)
}

pub(crate) fn cmd_run(command: &RunCommands) -> ExitCode {
    match command {
        RunCommands::Compose {
            project_root,
            file,
            profiles,
            args,
        } => run_compose(project_root, file.as_deref(), profiles, args),
        RunCommands::Local {
            path,
            port,
            no_open,
            keep,
            docker,
            json,
        } => run_local(path, *port, *no_open, *keep, *docker, *json),
    }
}
