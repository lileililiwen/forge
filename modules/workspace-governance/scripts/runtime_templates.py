#!/usr/bin/env python3
"""Render versioned Docker/Compose runtime templates safely."""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


GOVERNANCE_ROOT = Path(__file__).resolve().parents[1]
TEMPLATE_ROOT = GOVERNANCE_ROOT / "templates" / "runtime"
PROJECT_ID = re.compile(r"^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$")
TOKEN_PATTERN = re.compile(r"\{\{([A-Za-z_][A-Za-z0-9_]*)\}\}")
SECRET_NAME = re.compile(r"(?:password|passwd|secret|token|apikey|api_key|api-key|credential|private[_-]?key|bearer|session[_-]?key)", re.I)
RELATIVE_VALUE = re.compile(r"^(?!/)(?![A-Za-z]:[\\/])[^\x00]+$")
EXTERNAL_SUFFIX = "-postgres-external"
DEFAULT_DB_PORT = "5432"


class TemplateError(Exception):
    def __init__(self, code: str, message: str):
        super().__init__(message)
        self.code = code


def _json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise TemplateError("TEMPLATE_INVALID", f"invalid JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise TemplateError("TEMPLATE_INVALID", f"JSON object required: {path}")
    return value


def load_registry() -> dict[str, dict[str, Any]]:
    registry = _json(TEMPLATE_ROOT / "registry.json")
    if registry.get("schema_version") != 1:
        raise TemplateError("TEMPLATE_INVALID", "runtime template registry schema_version must be 1")
    entries = registry.get("templates")
    if not isinstance(entries, list):
        raise TemplateError("TEMPLATE_INVALID", "runtime template registry must contain templates[]")
    result: dict[str, dict[str, Any]] = {}
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get("id"), str):
            raise TemplateError("TEMPLATE_INVALID", "every runtime template entry needs an id")
        template_id = entry["id"]
        if template_id in result:
            raise TemplateError("TEMPLATE_INVALID", f"duplicate runtime template: {template_id}")
        if entry.get("version") != 1 or not isinstance(entry.get("required_variables"), list):
            raise TemplateError("TEMPLATE_INVALID", f"invalid metadata for template {template_id}")
        files = entry.get("files")
        if not isinstance(files, list) or not files:
            raise TemplateError("TEMPLATE_INVALID", f"template {template_id} has no files")
        directory = TEMPLATE_ROOT / template_id
        metadata_path = directory / "template.json"
        metadata = _json(metadata_path)
        if metadata.get("id") != template_id or metadata.get("version") != entry["version"] or metadata.get("required_variables") != entry["required_variables"] or metadata.get("files") != entry["files"]:
            raise TemplateError("TEMPLATE_INVALID", f"template metadata does not agree with registry: {template_id}")
        for relative in files:
            if not isinstance(relative, str) or Path(relative).is_absolute() or ".." in Path(relative).parts:
                raise TemplateError("TEMPLATE_INVALID", f"invalid generated path for {template_id}: {relative!r}")
            source = directory / relative
            if not source.is_file():
                raise TemplateError("TEMPLATE_INVALID", f"template {template_id} references missing file: {source}")
        result[template_id] = entry
    return result


def _relative_path(value: str, label: str) -> None:
    if not RELATIVE_VALUE.fullmatch(value) or ".." in Path(value).parts:
        raise TemplateError("RENDER_REJECTED", f"{label} must be repository-relative")


def _validate_variables(entry: dict[str, Any], variables: dict[str, str]) -> None:
    for key in variables:
        if SECRET_NAME.search(key):
            raise TemplateError("RENDER_REJECTED", f"secret variable is not accepted: {key}")
    required = set(entry["required_variables"])
    optional = set(entry.get("optional_variables", []))
    unknown = sorted(set(variables) - required - optional)
    if unknown:
        raise TemplateError("TEMPLATE_INVALID", f"unknown variables: {', '.join(unknown)}")
    missing = sorted(required - variables.keys())
    if missing:
        raise TemplateError("INSUFFICIENT_EVIDENCE", f"missing variables: {', '.join(missing)}")
    for key, value in variables.items():
        if not isinstance(value, str) or not value.strip():
            raise TemplateError("RENDER_REJECTED", f"variable {key} must be non-empty text")
        if key in {"app_project", "output_dir"}:
            _relative_path(value, key)
        if key == "project_name" and not PROJECT_ID.fullmatch(value):
            raise TemplateError("RENDER_REJECTED", "project_name must be lowercase kebab-case")
        if key == "port" and (not value.isdigit() or not 1 <= int(value) <= 65535):
            raise TemplateError("RENDER_REJECTED", "port must be between 1 and 65535")
        if key == "db_port" and (not value.isdigit() or not 1 <= int(value) <= 65535):
            raise TemplateError("RENDER_REJECTED", "db_port must be between 1 and 65535")
        if key == "db_host" and ("\n" in value or "\r" in value):
            raise TemplateError("RENDER_REJECTED", "db_host must be a single line")


def _render_text(text: str, variables: dict[str, str]) -> str:
    rendered = TOKEN_PATTERN.sub(lambda match: variables.get(match.group(1), match.group(0)), text)
    unresolved = sorted(set(match.group(1) for match in TOKEN_PATTERN.finditer(rendered)))
    if unresolved:
        raise TemplateError("RENDER_REJECTED", f"unresolved template variables: {', '.join(unresolved)}")
    if SECRET_NAME.search(rendered) and "${" not in rendered:
        # This catches accidental literal secret-looking assignments in the
        # generated template while allowing Compose environment references.
        for line in rendered.splitlines():
            if SECRET_NAME.search(line) and "${" not in line and ":" in line:
                raise TemplateError("RENDER_REJECTED", "generated template contains a literal secret field")
    return rendered


def _load_declaration(root: Path) -> dict[str, Any]:
    path = root / ".project.json"
    if not path.is_file():
        raise TemplateError("DECLARATION_INVALID", f"missing project declaration: {path}")
    declaration = _json(path)
    if not isinstance(declaration.get("id"), str) or not PROJECT_ID.fullmatch(declaration["id"]):
        raise TemplateError("DECLARATION_INVALID", "project declaration has an invalid id")
    if not isinstance(declaration.get("deployment"), dict):
        raise TemplateError("DECLARATION_INVALID", "project declaration has no deployment object")
    return declaration


def _is_external_template(template_id: str) -> bool:
    return template_id.endswith(EXTERNAL_SUFFIX)


def _check_env_ignore_conflict(path: Path) -> None:
    """Refuse before any write if `.gitignore` would not actually ignore `.env`."""
    if not path.exists():
        return
    if path.is_dir():
        raise TemplateError("SECRET_IGNORE_CONFLICT", f".gitignore is a directory: {path}")
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        raise TemplateError("SECRET_IGNORE_CONFLICT", f".gitignore is unreadable: {exc}") from exc
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("!") and ".env" in stripped:
            raise TemplateError("SECRET_IGNORE_CONFLICT", f".gitignore contains an unignore rule for .env: {path}")


def _gitignore_already_ignores_env(text: str) -> bool:
    for line in text.splitlines():
        stripped = line.strip()
        if stripped in {".env", "/.env", "**/.env", ".env/*"}:
            return True
    return False


def _env_ignore_output(text: str | None) -> str:
    """Return the .gitignore content with `.env` ignored, idempotently."""
    if text is None:
        return ".env\n"
    if _gitignore_already_ignores_env(text):
        return text
    suffix = "" if text.endswith("\n") else "\n"
    return text + suffix + ".env\n"


def _apply_optional_defaults(entry: dict[str, Any], variables: dict[str, str]) -> dict[str, str]:
    """Copy variables and apply optional defaults (e.g. db_port -> 5432)."""
    result = dict(variables)
    for key in entry.get("optional_variables", []):
        if key not in result:
            if key == "db_port":
                result[key] = DEFAULT_DB_PORT
    return result


def inspect_target(root: Path) -> dict[str, Any]:
    root = root.resolve()
    evidence: list[str] = []
    suggestions: list[str] = []
    csproj = sorted(root.rglob("*.csproj"))
    solutions = sorted(root.rglob("*.sln")) + sorted(root.rglob("*.slnx"))
    if csproj or solutions:
        suggestions.append("dotnet-web-postgres")
        evidence.extend(path.relative_to(root).as_posix() for path in csproj)
        evidence.extend(path.relative_to(root).as_posix() for path in solutions)
    if (root / "Cargo.toml").is_file():
        suggestions.append("rust-web-postgres")
        evidence.append("Cargo.toml")
    if (root / "pyproject.toml").is_file() or (root / "requirements.txt").is_file():
        suggestions.append("python-web-postgres")
        evidence.extend(name for name in ("pyproject.toml", "requirements.txt") if (root / name).is_file())
    if (root / "package.json").is_file():
        suggestions.append("node-web")
        evidence.append("package.json")
    if (root / "index.html").is_file() and not suggestions:
        suggestions.append("static-site")
        evidence.append("index.html")
    suggestions = sorted(set(suggestions))
    if len(suggestions) != 1:
        detail = "no supported runtime evidence" if not suggestions else f"ambiguous runtime choices: {', '.join(suggestions)}"
        raise TemplateError("TYPE_REQUIRED", f"{detail}; select --type explicitly")
    return {"suggestions": suggestions, "evidence": sorted(set(evidence))}


def render_target(root: Path, template_type: str, variables: dict[str, str], replace: set[str] | None = None) -> dict[str, Any]:
    root = root.resolve()
    registry = load_registry()
    entry = registry.get(template_type)
    if entry is None:
        raise TemplateError("TEMPLATE_INVALID", f"unknown runtime template: {template_type}")
    is_external = _is_external_template(template_type)
    effective_variables = _apply_optional_defaults(entry, variables)
    _validate_variables(entry, effective_variables)
    declaration = _load_declaration(root)
    if is_external:
        # Refuse BEFORE writing any file: a conflicting .gitignore would leave
        # the local `.env` exposed in version control. The actual edit happens
        # atomically with the template file writes below.
        _check_env_ignore_conflict(root / ".gitignore")
    replace = replace or set()
    outputs: dict[Path, str] = {}
    directory = TEMPLATE_ROOT / template_type
    for relative in entry["files"]:
        output = root / relative
        if output.exists() and relative not in replace:
            raise TemplateError("TARGET_EXISTS", f"target exists; name it with --replace: {relative}")
        outputs[output] = _render_text((directory / relative).read_text(encoding="utf-8"), effective_variables)
    if is_external:
        gitignore_path = root / ".gitignore"
        existing = gitignore_path.read_text(encoding="utf-8") if gitignore_path.is_file() else None
        outputs[gitignore_path] = _env_ignore_output(existing)

    updated = json.loads(json.dumps(declaration))
    deployment = updated["deployment"]
    deployment["compose_file"] = "compose.yaml"
    deployment["template_type"] = template_type
    deployment["template_version"] = entry["version"]
    deployment["rendered_files"] = entry["files"]
    deployment["template_state"] = "configured"
    if is_external:
        deployment["database_source"] = "external"
    declaration_text = json.dumps(updated, indent=2) + "\n"
    declaration_path = root / ".project.json"
    if declaration_path.exists() and ".project.json" not in replace:
        # Metadata is an expected update, but a malformed declaration was
        # already rejected above; write it atomically with the generated set.
        pass

    temporary: list[tuple[Path, Path]] = []
    try:
        for output, content in {**outputs, declaration_path: declaration_text}.items():
            output.parent.mkdir(parents=True, exist_ok=True)
            handle = tempfile.NamedTemporaryFile("w", encoding="utf-8", dir=output.parent, delete=False)
            with handle:
                handle.write(content)
            temporary.append((Path(handle.name), output))
        for temporary_path, output in temporary:
            os.replace(temporary_path, output)
    except OSError as exc:
        for temporary_path, _ in temporary:
            temporary_path.unlink(missing_ok=True)
        raise TemplateError("RENDER_REJECTED", f"atomic render failed: {exc}") from exc
    return {"state": "configured", "template_type": template_type, "rendered_files": entry["files"]}


def check_target(root: Path) -> dict[str, Any]:
    root = root.resolve()
    declaration = _load_declaration(root)
    deployment = declaration["deployment"]
    template_type = deployment.get("template_type")
    if not isinstance(template_type, str):
        raise TemplateError("INSUFFICIENT_EVIDENCE", "deployment.template_type is not configured")
    registry = load_registry()
    entry = registry.get(template_type)
    if entry is None:
        raise TemplateError("TEMPLATE_INVALID", f"unknown configured template: {template_type}")
    files = deployment.get("rendered_files", [])
    if files != entry["files"]:
        raise TemplateError("INSUFFICIENT_EVIDENCE", "deployment.rendered_files does not match the selected template registry")
    for relative in files:
        if not isinstance(relative, str) or Path(relative).is_absolute() or ".." in Path(relative).parts:
            raise TemplateError("INSUFFICIENT_EVIDENCE", f"rendered file is not repository-relative: {relative!r}")
        path = root / relative
        if not path.is_file():
            raise TemplateError("INSUFFICIENT_EVIDENCE", f"rendered file is missing: {relative}")
        _render_text(path.read_text(encoding="utf-8"), {})
    compose_value = deployment.get("compose_file", "compose.yaml")
    if not isinstance(compose_value, str) or Path(compose_value).is_absolute() or ".." in Path(compose_value).parts:
        raise TemplateError("INSUFFICIENT_EVIDENCE", "deployment.compose_file must be repository-relative")
    compose = root / compose_value
    docker = shutil.which("docker")
    if docker and compose.is_file():
        check_environment = os.environ.copy()
        check_environment.setdefault("POSTGRES_PASSWORD", "runtime-template-check-only")
        result = subprocess.run([docker, "compose", "-f", str(compose), "config", "--quiet"], cwd=root, env=check_environment, capture_output=True, text=True, timeout=30)
        if result.returncode:
            raise TemplateError("COMPOSE_INVALID", (result.stderr or result.stdout).strip()[:2000])
        docker_state = "verified"
    else:
        docker_state = "TOOL_UNAVAILABLE"
    return {"state": "configured", "compose_validation": docker_state, "template_type": template_type}


def _variables(values: list[str]) -> dict[str, str]:
    result: dict[str, str] = {}
    for item in values:
        if "=" not in item:
            raise TemplateError("RENDER_REJECTED", f"variable must use key=value: {item!r}")
        key, value = item.split("=", 1)
        result[key] = value
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="runtime_templates.py")
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("list")
    inspect = sub.add_parser("inspect")
    inspect.add_argument("--root", type=Path, required=True)
    check = sub.add_parser("check")
    check.add_argument("--root", type=Path, required=True)
    render = sub.add_parser("render")
    render.add_argument("--root", type=Path, required=True)
    render.add_argument("--type", required=True, dest="template_type")
    render.add_argument("--var", action="append", default=[])
    render.add_argument("--replace", action="append", default=[])
    args = parser.parse_args(argv)
    try:
        if args.command == "list":
            print(json.dumps(load_registry(), indent=2))
        elif args.command == "inspect":
            print(json.dumps(inspect_target(args.root), indent=2))
        elif args.command == "check":
            print(json.dumps(check_target(args.root), indent=2))
        else:
            print(json.dumps(render_target(args.root, args.template_type, _variables(args.var), set(args.replace)), indent=2))
        return 0
    except TemplateError as exc:
        print(f"{exc.code}: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
