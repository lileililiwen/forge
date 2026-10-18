"""Standalone project boundaries and sibling-coupling checks."""

from __future__ import annotations

import workspace_check as _facade
globals().update({key: value for key, value in vars(_facade).items() if not key.startswith("__")})

def _scan_cargo_toml_for_siblings(path: Path) -> list[str]:
    text: str
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return []
    found: list[str] = []
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if "path" not in stripped:
            continue
        match = _SIBLING_PATH_PATTERN.search(stripped)
        if match:
            found.append(match.group(2))
    return found


def _scan_package_json_for_siblings(path: Path) -> list[str]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return []
    found: list[str] = []
    workspaces = payload.get("workspaces") if isinstance(payload, dict) else None
    candidates: list[Any]
    if isinstance(workspaces, list):
        candidates = workspaces
    elif isinstance(workspaces, dict):
        packages = workspaces.get("packages")
        candidates = packages if isinstance(packages, list) else []
    else:
        candidates = []
    for entry in candidates:
        if not isinstance(entry, str):
            continue
        match = _SIBLING_PATH_PATTERN.search(entry)
        if match:
            found.append(match.group(2))
    return found


def _scan_pyproject_toml_for_siblings(path: Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return []
    found: list[str] = []
    in_target = False
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            in_target = line in ("[tool.uv.sources]", "[tool.poetry.sources]")
            continue
        if not in_target:
            continue
        match = _SIBLING_PATH_PATTERN.search(line)
        if match:
            found.append(match.group(2))
    return found


def _scan_go_mod_for_siblings(path: Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return []
    found: list[str] = []
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped.startswith("replace "):
            continue
        match = _SIBLING_PATH_PATTERN.search(stripped)
        if match:
            found.append(match.group(2))
    return found


_BOUNDED_STATIC_SCAN_TARGETS: tuple[tuple[str, Any], ...] = (
    ("Cargo.toml", _scan_cargo_toml_for_siblings),
    ("package.json", _scan_package_json_for_siblings),
    ("pyproject.toml", _scan_pyproject_toml_for_siblings),
    ("go.mod", _scan_go_mod_for_siblings),
)


def _bounded_static_scan_for_siblings(project_path: Path) -> list[tuple[str, str]]:
    """Bounded static scan for sibling source references.

    Returns a list of `(file, sibling_id)` pairs. The scan is bounded to
    a fixed set of manifest file names, never executes the project, and
    never follows paths. Detected siblings are advisory only.
    """
    found: list[tuple[str, str]] = []
    for relative, scanner in _BOUNDED_STATIC_SCAN_TARGETS:
        path = project_path / relative
        if not path.is_file():
            continue
        try:
            siblings = scanner(path)
        except Exception:  # noqa: BLE001 - scanner failures are advisory
            continue
        for sibling in siblings:
            found.append((relative, sibling))
    return found


def _is_absolute_path(value: str) -> bool:
    """Return True when `value` is an absolute machine-specific path.

    Absolute Unix paths, Windows paths with a drive letter, and
    Windows UNC paths are all rejected. Backslash is the only
    indicator we use for Windows because the schema pattern
    forbids forward slashes at the start.
    """
    if not isinstance(value, str) or not value:
        return False
    if value.startswith("/"):
        return True
    if re.match(r"^[A-Za-z]:[\\/]", value):
        return True
    if value.startswith("\\\\"):
        return True
    return False


def _has_path_escape(value: str) -> bool:
    """Return True when `value` contains a `..` segment that escapes the root."""
    if not isinstance(value, str) or not value:
        return False
    parts = value.replace("\\", "/").split("/")
    return any(part == ".." for part in parts)


def _redact_secret_value(value: str, secret_words: "re.Pattern[str]") -> str:
    """Redact a value when it contains a secret keyword followed by a value."""
    if not isinstance(value, str) or not secret_words.search(value):
        return value
    return secret_words.sub("[redacted]", value)


def _standalone_evidence_resolves(project_path: Path, evidence_ref: str) -> bool:
    """Return True when the repository-relative evidence ref resolves to a file under the project root."""
    if _is_absolute_path(evidence_ref) or _has_path_escape(evidence_ref):
        return False
    try:
        resolved = (project_path / evidence_ref).resolve(strict=True)
    except (OSError, RuntimeError):
        return False
    try:
        resolved.relative_to(project_path.resolve(strict=True))
    except ValueError:
        return False
    return True


def validate_standalone(
    declaration: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    governance_root: Path | None = None,
) -> None:
    """Audit a project's optional standalone and integrations declarations.

    The validator is the single source of truth for the
    standalone-project-boundary capability. It is invoked only when
    `declaration.standalone` or `declaration.integrations` is present,
    so existing declarations stay unchanged. The audit never executes
    project commands and never rewrites declared values.
    """
    import vocabulary as vocabulary_module

    try:
        canonical_statuses = frozenset(
            vocabulary_module.load_vocabulary_for_governance_root(governance_root or _default_governance_root()).get(
                "standalone_statuses",
                frozenset(),
            )
        )
    except vocabulary_module.VocabularyError as exc:
        finding(findings, "ERROR", STANDALONE_SHAPE, f"vocabulary unavailable for standalone audit: {exc}", project_id)
        return
    secret_words = secret_words_for(governance_root)

    standalone = declaration.get("standalone")
    if standalone is None:
        finding(findings, "WARNING", STANDALONE_BLOCKED, "standalone block absent; cannot classify", project_id)
    else:
        if not isinstance(standalone, dict):
            finding(findings, "ERROR", STANDALONE_SHAPE, "standalone must be an object", project_id)
        else:
            _validate_standalone_shape(standalone, canonical_statuses, findings, project_id, secret_words)
            _validate_standalone_paths(standalone, project_path, findings, project_id, secret_words)
            _validate_standalone_evidence(standalone, project_path, findings, project_id)

    integrations = declaration.get("integrations")
    if integrations is not None:
        _validate_integrations(integrations, project_path, findings, project_id, secret_words)

    detected = _bounded_static_scan_for_siblings(project_path)
    for file_name, sibling in detected:
        finding(
            findings,
            "WARNING",
            STANDALONE_DETECTED_SIBLING,
            f"static scan found '../{sibling}' in {file_name}; declaration is the source of truth",
            project_id,
        )


def _validate_standalone_shape(
    standalone: dict[str, Any],
    canonical_statuses: frozenset[str],
    findings: list[Finding],
    project_id: str,
    secret_words: "re.Pattern[str]",
) -> None:
    required = ("status", "local_defaults", "required_external_services", "required_sibling_projects")
    for key in required:
        if key not in standalone:
            finding(findings, "ERROR", STANDALONE_SHAPE, f"standalone.{key} is required", project_id)
    status = standalone.get("status")
    if status is not None and (not isinstance(status, str) or status not in canonical_statuses):
        finding(
            findings,
            "ERROR",
            STANDALONE_STATUS_INVALID,
            f"standalone.status {status!r} is not one of the canonical values: {sorted(canonical_statuses)}",
            project_id,
        )
    elif status == "blocked":
        # The operator explicitly declared the project cannot be classified
        # from available evidence; surface a warning so the audit does not
        # silently pass.
        finding(findings, "WARNING", STANDALONE_BLOCKED, "standalone.status is 'blocked'; cannot classify", project_id)
    for key in ("local_defaults", "required_external_services", "required_sibling_projects"):
        value = standalone.get(key)
        if value is None:
            continue
        if not isinstance(value, list) or any(not isinstance(item, str) or not item for item in value):
            finding(findings, "ERROR", STANDALONE_SHAPE, f"standalone.{key} must be an array of non-empty strings", project_id)
            continue
        for entry in value:
            redacted = _redact_secret_value(entry, secret_words)
            if redacted != entry:
                finding(findings, "ERROR", STANDALONE_SECRET_VALUE, f"standalone.{key} contains a secret-shaped value: [redacted]", project_id)


def _validate_standalone_paths(
    standalone: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    secret_words: "re.Pattern[str]",
) -> None:
    """Reject absolute paths and `..` escapes; record required siblings and services."""
    for key in ("local_defaults", "required_external_services", "required_sibling_projects"):
        value = standalone.get(key)
        if not isinstance(value, list):
            continue
        for entry in value:
            if not isinstance(entry, str) or not entry:
                continue
            if _is_absolute_path(entry):
                finding(findings, "ERROR", STANDALONE_ABSOLUTE_PATH, f"standalone.{key} contains an absolute path: [redacted]", project_id)
            elif _has_path_escape(entry):
                finding(findings, "ERROR", STANDALONE_PATH_ESCAPE, f"standalone.{key} contains a '..' segment: {entry!r}", project_id)
    siblings = standalone.get("required_sibling_projects")
    if isinstance(siblings, list):
        for entry in siblings:
            if isinstance(entry, str) and entry and not _is_absolute_path(entry) and not _has_path_escape(entry):
                finding(findings, "ERROR", STANDALONE_REQUIRED_SIBLING, f"required sibling project declared: {entry}", project_id)
    services = standalone.get("required_external_services")
    if isinstance(services, list):
        for entry in services:
            if isinstance(entry, str) and entry and not _is_absolute_path(entry) and not _has_path_escape(entry):
                finding(findings, "WARNING", STANDALONE_REQUIRED_SERVICE, f"required external service declared: {entry}", project_id)


def _validate_standalone_evidence(
    standalone: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
) -> None:
    evidence_ref = standalone.get("evidence_ref")
    if evidence_ref is not None and not isinstance(evidence_ref, str):
        finding(findings, "ERROR", STANDALONE_SHAPE, "standalone.evidence_ref must be a string when present", project_id)
        return
    if evidence_ref is None:
        state = standalone.get("evidence_state", "declared")
        if state in ("configured", "verified"):
            finding(findings, "ERROR", STANDALONE_SHAPE, f"standalone.evidence_state is {state!r} but evidence_ref is missing", project_id)
        return
    if _is_absolute_path(evidence_ref):
        finding(findings, "ERROR", STANDALONE_ABSOLUTE_PATH, f"standalone.evidence_ref is absolute: [redacted]", project_id)
        return
    if _has_path_escape(evidence_ref):
        finding(findings, "ERROR", STANDALONE_PATH_ESCAPE, f"standalone.evidence_ref escapes the project root: {evidence_ref!r}", project_id)
        return
    if not (project_path / evidence_ref).is_file():
        finding(findings, "WARNING", STANDALONE_EVIDENCE_MISSING, f"standalone.evidence_ref is missing on disk: {evidence_ref!r}", project_id)
        return
    state = standalone.get("evidence_state", "declared")
    if state not in {"declared", "configured", "verified", "unverified"}:
        finding(findings, "ERROR", STANDALONE_SHAPE, f"standalone.evidence_state is invalid: {state!r}", project_id)


def _validate_integrations(
    integrations: Any,
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    secret_words: "re.Pattern[str]",
) -> None:
    if not isinstance(integrations, list):
        finding(findings, "ERROR", STANDALONE_SHAPE, "integrations must be an array", project_id)
        return
    seen_ids: set[str] = set()
    for index, entry in enumerate(integrations):
        if not isinstance(entry, dict):
            finding(findings, "ERROR", STANDALONE_SHAPE, f"integrations[{index}] must be an object", project_id)
            continue
        entry_id = entry.get("id")
        if not isinstance(entry_id, str) or not PROJECT_ID.fullmatch(entry_id):
            finding(findings, "ERROR", STANDALONE_SHAPE, f"integrations[{index}].id must be a kebab-case identifier", project_id)
        elif entry_id in seen_ids:
            finding(findings, "ERROR", STANDALONE_SHAPE, f"integrations[{index}].id {entry_id!r} is duplicated", project_id)
        else:
            seen_ids.add(entry_id)
        required = entry.get("required")
        if not isinstance(required, bool):
            finding(findings, "ERROR", STANDALONE_SHAPE, f"integrations[{index}].required must be a boolean", project_id)
            required = False
        evidence_ref = entry.get("evidence_ref")
        if evidence_ref is not None and not isinstance(evidence_ref, str):
            finding(findings, "ERROR", STANDALONE_SHAPE, f"integrations[{index}].evidence_ref must be a string when present", project_id)
            evidence_ref = None
        if isinstance(evidence_ref, str):
            redacted = _redact_secret_value(evidence_ref, secret_words)
            if redacted != evidence_ref:
                finding(findings, "ERROR", STANDALONE_SECRET_VALUE, f"integrations[{index}].evidence_ref contains a secret-shaped value: [redacted]", project_id)
            if _is_absolute_path(evidence_ref):
                finding(findings, "ERROR", STANDALONE_ABSOLUTE_PATH, f"integrations[{index}].evidence_ref is absolute: [redacted]", project_id)
            elif _has_path_escape(evidence_ref):
                finding(findings, "ERROR", STANDALONE_PATH_ESCAPE, f"integrations[{index}].evidence_ref escapes the project root: {evidence_ref!r}", project_id)
        if not required:
            finding(findings, "WARNING", STANDALONE_OPTIONAL, f"optional integration declared: {entry_id!r}", project_id)
            continue
        if not evidence_ref:
            finding(findings, "WARNING", STANDALONE_INTEGRATION_PENDING, f"required integration missing evidence_ref: {entry_id!r}", project_id)
            continue
        if not _standalone_evidence_resolves(project_path, evidence_ref):
            finding(findings, "WARNING", STANDALONE_INTEGRATION_PENDING, f"required integration evidence_ref is missing or escapes the project root: {entry_id!r}", project_id)


def classify_standalone(
    declaration: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    governance_root: Path | None = None,
) -> dict[str, Any]:
    """Return a normalized standalone observation for a project.

    This is the public surface consumed by `forge_governance_adapter`.
    It runs the same shape/path/secret checks as `validate_standalone`
    so the adapter never has to shell out, and records the canonical
    status the audit derived plus the visible findings, integration
    list, and an evidence-ref resolution flag.

    The function never raises; it always returns a dict with at minimum
    `standalone_status` (one of the six canonical values) and a
    `standalone_findings` list. On audit failure the returned status
    is `blocked` and the findings carry the failure reason.
    """
    import vocabulary as vocabulary_module

    observed: dict[str, Any] = {
        "standalone_status": "blocked",
        "standalone_observed_revision": None,
        "standalone_integrations": [],
        "standalone_findings": [],
    }
    try:
        canonical_statuses = frozenset(
            vocabulary_module.load_vocabulary_for_governance_root(governance_root or _default_governance_root()).get(
                "standalone_statuses",
                frozenset(),
            )
        )
    except vocabulary_module.VocabularyError as exc:
        observed["standalone_status"] = "blocked"
        observed["standalone_findings"] = [
            {"level": "ERROR", "code": STANDALONE_SHAPE, "message": f"vocabulary unavailable: {exc}"},
        ]
        return observed

    internal: list[Finding] = []
    try:
        _facade.validate_standalone(
            declaration, project_path, internal, project_id, governance_root=governance_root
        )
    except Exception as exc:  # noqa: BLE001 - adapter must never raise
        internal.append(Finding("ERROR", STANDALONE_SHAPE, f"standalone audit crashed: {exc}", project_id))

    for item in internal:
        observed["standalone_findings"].append(
            {"level": item.level, "code": item.code, "message": item.message}
        )

    standalone = declaration.get("standalone")
    declared_status: str | None = None
    if isinstance(standalone, dict) and isinstance(standalone.get("status"), str):
        declared_status = standalone["status"]
        if declared_status in canonical_statuses:
            observed["standalone_status"] = declared_status

    siblings = standalone.get("required_sibling_projects") if isinstance(standalone, dict) else None
    services = standalone.get("required_external_services") if isinstance(standalone, dict) else None
    if isinstance(siblings, list) and siblings:
        observed["standalone_status"] = "workspace-dependent"
    elif isinstance(services, list) and services:
        observed["standalone_status"] = "service-dependent"

    integrations = declaration.get("integrations")
    if isinstance(integrations, list):
        for entry in integrations:
            if not isinstance(entry, dict):
                continue
            observed["standalone_integrations"].append(
                {
                    "id": entry.get("id"),
                    "required": entry.get("required", False),
                    "evidence_ref": entry.get("evidence_ref"),
                }
            )
    if observed["standalone_status"] == "standalone" and observed["standalone_integrations"]:
        observed["standalone_status"] = "standalone-with-optional-integrations"
    return observed
