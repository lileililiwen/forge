#!/usr/bin/env python3
"""Forge governance-provider adapter shim.

Reads one AdapterRequest JSON document on stdin, runs the existing audit
(`workspace_check`) for the named project, and writes exactly one
observation document to stdout. The adapter is read-only: it never
registers, adopts, mutates, or writes anything in this repository, and
it redacts secret-shaped substrings from every evidence line.

The contract is documented in
`openspec/changes/forge-governance-adapter/specs/forge-governance-adapter/spec.md`
and matches Forge's published `governance-provider-contract` v0.1.0.

Exit codes:

- 0: a trustworthy observation was printed (any governance status,
  including `unknown`).
- 2: the stdin request is malformed or the contract version is wrong;
  no observation is printed.
- 3: the underlying audit raised; Forge should classify this as
  `unavailable`, not as a governance `fail`.
"""

from __future__ import annotations

import json
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from types import SimpleNamespace
from typing import Any

from workspace_check import SECRET_WORDS, run as run_workspace_check
from workspace_check import STANDALONE_BLOCKED


PROVIDER = "workspace-governance"
PROTOCOL_VERSION = "0.1.0"
ACTION = "check"
EVIDENCE_CAP = 16
EVIDENCE_LINE_MAX = 200
REGISTRY_SCHEMA_VERSION = 1
EXIT_OK = 0
EXIT_BAD_REQUEST = 2
EXIT_AUDIT_FAILURE = 3
# Cap for the per-project standalone findings list embedded in the
# observation document. The list is sorted ERROR-first and then
# truncated so the observation never grows without bound. A small
# cap keeps the JSON shape stable for downstream consumers.
STANDALONE_FINDINGS_CAP = 16


# Same keyword set as the checker, but match the keyword plus any
# immediately-associated value (`=value`, `: value`, or ` value`) so a
# leaked secret value does not survive redaction.
SECRET_VALUE_PAIR = re.compile(
    rf"{SECRET_WORDS.pattern}\s*(?:[=:]\s*|\s+)([^\s,;]+|[A-Za-z0-9_\-./+=]+)",
    re.IGNORECASE,
)


class AdapterRequestError(Exception):
    """Raised when the stdin request fails validation (exit 2)."""


class AuditCrash(Exception):
    """Raised when the underlying audit raises (exit 3)."""


def load_request(stream: str) -> dict[str, str]:
    """Parse and validate the stdin request body.

    The body MUST be a JSON object with `contract == "0.1.0"`,
    `action == "check"`, non-empty `project_id`, and non-empty
    `project_path`. Anything else raises `AdapterRequestError`.
    """
    if stream is None or not str(stream).strip():
        raise AdapterRequestError("empty request body")
    try:
        request = json.loads(stream)
    except json.JSONDecodeError as exc:
        raise AdapterRequestError(f"invalid JSON: {exc}") from exc
    if not isinstance(request, dict):
        raise AdapterRequestError("request must be a JSON object")
    if request.get("contract") != PROTOCOL_VERSION:
        raise AdapterRequestError(f"contract must be {PROTOCOL_VERSION!r}")
    if request.get("action") != ACTION:
        raise AdapterRequestError(f"action must be {ACTION!r}")
    project_id = request.get("project_id")
    if not isinstance(project_id, str) or not project_id.strip():
        raise AdapterRequestError("project_id must be a non-empty string")
    project_path = request.get("project_path")
    if not isinstance(project_path, str) or not project_path.strip():
        raise AdapterRequestError("project_path must be a non-empty string")
    return {
        "contract": PROTOCOL_VERSION,
        "action": ACTION,
        "project_id": project_id,
        "project_path": project_path,
    }


def find_registry_entry(registry: dict, project_id: str) -> dict | None:
    for entry in registry.get("projects", []):
        if entry.get("id") == project_id:
            return entry
    return None


def detect_git_revision(project_path: Path) -> str | None:
    """Return the project's HEAD revision if `.git/HEAD` is present.

    Returns the SHA for a detached HEAD, follows a symbolic ref, or
    returns None so the caller can simply omit the field.
    """
    head = project_path / ".git" / "HEAD"
    if not head.is_file():
        return None
    raw = head.read_text(encoding="utf-8").strip()
    if not raw:
        return None
    if not raw.startswith("ref:"):
        return raw
    ref = raw.split(":", 1)[1].strip()
    ref_path = project_path / ".git" / ref
    if ref_path.is_file():
        return ref_path.read_text(encoding="utf-8").strip()
    return None


def run_audit(registry_path: Path, workspace_root: Path, project_id: str) -> list:
    """Run the in-process audit for one project, translating crashes.

    Returns the list of `Finding` objects the audit produced. If the
    audit raises for any reason, this function translates the exception
    into `AuditCrash` so the caller can map it to the `unavailable`
    classification on the Forge side.
    """
    args = SimpleNamespace(
        registry=str(registry_path),
        root=str(workspace_root),
        project=project_id,
        audit=False,
        self_check=False,
        as_json=False,
    )
    try:
        findings, _summary = run_workspace_check(args)
    except Exception as exc:  # noqa: BLE001 - translated below
        raise AuditCrash(f"audit failed for {project_id}: {exc}") from exc
    return findings


def project_findings(findings: list, project_id: str) -> list:
    """Filter findings to those attributable to the requested project.

    Project-attributed findings come first (in audit order). Registry-level
    findings (no `project`) are appended only when they are ERRORs, because
    a registry-level ERROR still blocks the project from passing.
    """
    own = [f for f in findings if f.project == project_id]
    registry_errors = [f for f in findings if f.project is None and f.level == "ERROR"]
    return own + registry_errors


def scrub_line(text: str) -> str:
    """Redact secret-shaped substrings from an evidence line.

    The pattern matches a SECRET_WORDS keyword together with any
    immediately-associated value (`token=ABC`, `api_key: hunter2`,
    `password hunter2`) so the value is not leaked in the observation.
    Any standalone keyword that survived the first pass is redacted in
    a second pass so e.g. "the token expired" still loses the word.
    """
    text = SECRET_VALUE_PAIR.sub("[redacted]", text)
    text = SECRET_WORDS.sub("[redacted]", text)
    return text


def render_evidence(findings: list) -> list[str]:
    """Render the evidence list, ERROR-first, scrubbed and capped.

    Each line is `<code>: <message>` truncated to 200 chars, with
    secret-shaped substrings replaced by `[redacted]`. At most
    `EVIDENCE_CAP` lines are returned.
    """
    errors = [f for f in findings if f.level == "ERROR"]
    warnings = [f for f in findings if f.level == "WARNING"]
    ordered = errors + warnings
    lines: list[str] = []
    for f in ordered:
        line = f"{f.code}: {f.message}"
        line = scrub_line(line)
        if len(line) > EVIDENCE_LINE_MAX:
            line = line[: EVIDENCE_LINE_MAX - 1] + "…"
        lines.append(line)
        if len(lines) >= EVIDENCE_CAP:
            break
    return lines


def status_for(findings: list, entry: dict | None) -> str:
    """Map audit findings and registry state to the contract status.

    Order matches the spec: any ERROR first, then the adoption gap, then
    the discovered-but-unregistered signal, then `pass`.
    """
    if any(f.level == "ERROR" for f in findings):
        return "fail"
    if entry is not None and entry.get("adoption") != "adopted":
        return "blocked"
    if any(f.code == "DISCOVERED_UNREGISTERED" for f in findings):
        return "blocked"
    return "pass"


def build_observation(
    *,
    project_id: str,
    status: str,
    findings: list,
    entry: dict | None,
    source_revision: str | None,
) -> dict[str, Any]:
    """Assemble the final observation document.

    Always echoes the requested `project_id` and the protocol version.
    `source_revision` is omitted entirely when no revision is known so
    the contract consumer can rely on the field's presence as a signal
    that the project is a git repo.
    """
    evidence = render_evidence(findings)
    errors = sum(1 for f in findings if f.level == "ERROR")
    warnings = sum(1 for f in findings if f.level == "WARNING")
    if entry is not None:
        adoption = entry.get("adoption", "unknown")
    else:
        adoption = "unregistered"

    if status == "unknown":
        detail = "not registered in workspace registry"
    elif status == "blocked" and entry is not None and entry.get("adoption") != "adopted":
        detail = f"0 errors, {warnings} warnings, adoption={adoption}"
    elif status == "blocked":
        detail = f"0 errors, {warnings} warnings, adoption=unregistered"
    else:
        detail = f"{errors} errors, {warnings} warnings, adoption={adoption}"

    observation: dict[str, Any] = {
        "provider": PROVIDER,
        "protocol_version": PROTOCOL_VERSION,
        "project_id": project_id,
        "status": status,
        "evidence": evidence,
        "detail": detail,
        "metadata": {
            "registry_schema_version": REGISTRY_SCHEMA_VERSION,
            "observed_via": "workspace_check",
        },
    }
    if source_revision is not None:
        observation["source_revision"] = source_revision
    return observation


def render_standalone_findings(raw: list[dict[str, Any]]) -> list[str]:
    """Render the standalone findings list, ERROR-first, scrubbed and capped.

    Each line is `<code>: <message>` truncated to 200 chars, with
    secret-shaped substrings replaced by `[redacted]`. At most
    `STANDALONE_FINDINGS_CAP` lines are returned so the observation
    remains bounded.
    """
    ordered = sorted(
        raw,
        key=lambda item: (0 if item.get("level") == "ERROR" else 1, 0 if item.get("code") else 1),
    )
    lines: list[str] = []
    for item in ordered:
        code = str(item.get("code", "STANDALONE"))
        message = str(item.get("message", ""))
        line = f"{code}: {message}"
        line = scrub_line(line)
        if len(line) > EVIDENCE_LINE_MAX:
            line = line[: EVIDENCE_LINE_MAX - 1] + "…"
        lines.append(line)
        if len(lines) >= STANDALONE_FINDINGS_CAP:
            break
    return lines


def build_standalone_observation(
    *,
    project_id: str,
    project_path: Path | None,
    declaration: dict[str, Any] | None,
    governance_root: Path,
) -> dict[str, Any]:
    """Build the additive standalone observation block for the adapter.

    Returns a dict with four keys:

    - `standalone_status`: one of the six canonical statuses
      (`standalone`, `standalone-with-optional-integrations`,
      `library-dependent`, `workspace-dependent`, `service-dependent`,
      `blocked`).
    - `standalone_observed_revision`: the git HEAD revision if the
      project is a git repo, otherwise the field is omitted entirely
      so the contract consumer can rely on the field's presence as a
      signal.
    - `standalone_integrations`: the declared `integrations` array
      normalized to a list of `{"id", "required", "evidence_ref"}`
      objects.
    - `standalone_findings`: an ERROR-first, secret-scrubbed,
      truncated list of audit findings for the standalone block.

    The function never raises. On audit failure or missing project
    path it returns a `blocked` status and a single STANDALONE_BLOCKED
    or STANDALONE_SHAPE finding.
    """
    from workspace_check import classify_standalone, Finding

    observation: dict[str, Any]
    if project_path is None:
        observation = {
            "standalone_status": "blocked",
            "standalone_integrations": [],
            "standalone_findings": [
                {
                    "level": "WARNING",
                    "code": STANDALONE_BLOCKED,
                    "message": "project is not on disk; cannot classify",
                }
            ],
        }
        observation["standalone_findings"] = render_standalone_findings(observation["standalone_findings"])
        return observation

    revision = detect_git_revision(project_path)
    if not isinstance(declaration, dict):
        observation = {
            "standalone_status": "blocked",
            "standalone_integrations": [],
            "standalone_findings": [
                {
                    "level": "WARNING",
                    "code": STANDALONE_BLOCKED,
                    "message": "no declaration available; cannot classify",
                }
            ],
        }
        if revision is not None:
            observation["standalone_observed_revision"] = revision
        observation["standalone_findings"] = render_standalone_findings(observation["standalone_findings"])
        return observation

    try:
        block = classify_standalone(
            declaration,
            project_path,
            [],
            project_id,
            governance_root=governance_root,
        )
    except Exception as exc:  # noqa: BLE001 - adapter must never raise
        block = {
            "standalone_status": "blocked",
            "standalone_integrations": [],
            "standalone_findings": [
                Finding("ERROR", STANDALONE_SHAPE, f"standalone audit crashed: {exc}", project_id)
            ],
        }

    observation = {
        "standalone_status": block.get("standalone_status", "blocked"),
        "standalone_integrations": block.get("standalone_integrations", []),
        "standalone_findings": render_standalone_findings(block.get("standalone_findings", [])),
    }
    if revision is not None:
        observation["standalone_observed_revision"] = revision
    return observation


def evaluate(
    request: dict[str, Any],
    *,
    governance_root: Path,
    workspace_root: Path,
) -> dict[str, Any]:
    """Validate, audit, and render the observation for one project.

    Public for tests; `main()` is the only stdin/stdout surface. The
    function never writes to disk and never mutates the registry.
    """
    registry_path = governance_root / "projects.json"
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    entry = find_registry_entry(registry, request["project_id"])

    findings = run_audit(registry_path, workspace_root, request["project_id"])
    project_only = project_findings(findings, request["project_id"])

    if entry is None:
        if any(f.code == "DISCOVERED_UNREGISTERED" for f in project_only):
            status = "blocked"
        else:
            status = "unknown"
    else:
        status = status_for(project_only, entry)

    # `source_revision` is only meaningful when the project is on disk;
    # we look at the registry path first, then fall back to the requested
    # id as a folder name in the workspace root.
    source_revision: str | None = None
    candidate: Path | None = None
    if entry is not None:
        candidate = workspace_root / str(entry.get("path", ""))
    elif (workspace_root / request["project_id"]).is_dir():
        candidate = workspace_root / request["project_id"]
    if candidate is not None and candidate.is_dir():
        source_revision = detect_git_revision(candidate)

    observation = build_observation(
        project_id=request["project_id"],
        status=status,
        findings=project_only,
        entry=entry,
        source_revision=source_revision,
    )

    # Additive standalone observation: always populated, even when the
    # project is not on disk, so consumers can rely on the field's
    # presence. The four new fields are added after the existing keys
    # so existing consumers that only read the first N keys see no
    # change.
    standalone = build_standalone_observation(
        project_id=request["project_id"],
        project_path=candidate if candidate is not None and candidate.is_dir() else None,
        declaration=_read_project_declaration(candidate) if candidate is not None and candidate.is_dir() else None,
        governance_root=governance_root,
    )
    observation.update(standalone)

    return observation


def _read_project_declaration(project_path: Path) -> dict[str, Any] | None:
    """Read and parse `.project.json` for a project, returning None on any failure.

    The function never raises: missing files, unreadable files, and
    non-object JSON all return None so the adapter observation falls
    back to a `blocked` standalone status without crashing.
    """
    declaration_path = project_path / ".project.json"
    if not declaration_path.is_file():
        return None
    try:
        payload = json.loads(declaration_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if not isinstance(payload, dict):
        return None
    return payload


def _resolve_workspace_root(governance_root: Path) -> Path:
    """Resolve the workspace root for the audit.

    Precedence matches the checker's rule: the registry's
    `workspace_root`, then `WORKSPACE_ROOT` in the environment, then the
    current working directory.
    """
    registry_path = governance_root / "projects.json"
    registry_root = None
    try:
        payload = json.loads(registry_path.read_text(encoding="utf-8"))
        if isinstance(payload, dict):
            registry_root = payload.get("workspace_root")
    except (OSError, json.JSONDecodeError):
        registry_root = None
    configured = os.environ.get("WORKSPACE_ROOT") or registry_root
    if configured:
        return Path(configured).expanduser().resolve()
    return Path.cwd().resolve()


def _resolve_roots() -> tuple[Path, Path]:
    """Resolve both roots the adapter needs.

    Exposed as a single hook so tests can patch the resolution and
    point the adapter at a synthetic governance/workspace pair without
    touching `__file__`.
    """
    governance_root = Path(__file__).resolve().parents[1]
    workspace_root = _resolve_workspace_root(governance_root)
    return governance_root, workspace_root


def main() -> int:
    """Read stdin, evaluate, write the observation to stdout."""
    stream = sys.stdin.read()
    try:
        request = load_request(stream)
    except AdapterRequestError as exc:
        print(f"adapter rejected request: {exc}", file=sys.stderr)
        return EXIT_BAD_REQUEST

    governance_root, workspace_root = _resolve_roots()

    try:
        observation = evaluate(
            request,
            governance_root=governance_root,
            workspace_root=workspace_root,
        )
    except AuditCrash as exc:
        print(f"adapter audit failure: {exc}", file=sys.stderr)
        return EXIT_AUDIT_FAILURE

    sys.stdout.write(json.dumps(observation) + "\n")
    return EXIT_OK


if __name__ == "__main__":
    raise SystemExit(main())
