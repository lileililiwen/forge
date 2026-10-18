"""Release, deployment, product quality, and README claim validation."""

from __future__ import annotations

import workspace_check as _facade
globals().update({key: value for key, value in vars(_facade).items() if not key.startswith("__")})

def validate_release_evidence(release_evidence: Any, project_path: Path, findings: list[Finding], project_id: str) -> None:
    """Audit a project's optional release evidence record.

    Release evidence is a producer-neutral record of source revision, version,
    toolchain, artifacts, digests, SBOM, provenance, checks, and publication.
    """
    if not isinstance(release_evidence, dict):
        finding(findings, "ERROR", "RELEASE_EVIDENCE_SHAPE", "release_evidence must be an object", project_id)
        return
    seen: set[str] = set()
    for field, entry in release_evidence.items():
        if not isinstance(field, str):
            finding(findings, "ERROR", "RELEASE_EVIDENCE_FIELD", f"release evidence field must be a string, got {type(field).__name__}", project_id)
            continue
        if field in seen:
            finding(findings, "WARNING", "RELEASE_EVIDENCE_DUPLICATE", f"release evidence field repeated: {field}", project_id)
        seen.add(field)
        if field not in RELEASE_EVIDENCE_FIELDS:
            finding(findings, "WARNING", "RELEASE_EVIDENCE_FIELD", f"unknown release evidence field: {field}", project_id)
        if not isinstance(entry, dict):
            finding(findings, "ERROR", "RELEASE_EVIDENCE_SHAPE", f"release evidence {field!r} must be an object", project_id)
            continue
        state = entry.get("state")
        if state not in RELEASE_EVIDENCE_STATES:
            finding(findings, "ERROR", "RELEASE_EVIDENCE_STATE", f"release evidence {field!r} has invalid state: {state!r}", project_id)
            continue
        ref = entry.get("evidence_ref")
        if ref is not None and not isinstance(ref, str):
            finding(findings, "ERROR", "RELEASE_EVIDENCE_REF", f"release evidence {field!r} evidence_ref must be a string when present", project_id)
            ref = None
        if state in {"configured", "verified"} and isinstance(ref, str) and ref and not (project_path / ref).is_file():
            level = "ERROR" if state == "verified" else "WARNING"
            code = "RELEASE_EVIDENCE_STALE" if state == "verified" else "RELEASE_EVIDENCE_PENDING"
            finding(findings, level, code, f"release evidence {field!r} evidence_ref missing: {ref}", project_id)


def validate_deployment_evidence(deployment: Any, declaration: dict[str, Any], project_path: Path, findings: list[Finding], project_id: str) -> None:
    """For projects that opt in to deployment, require complete release evidence.

    A deployable project must carry a release_evidence block whose required
    fields (revision, artifacts, digests, provenance, checks, publication) are
    either verified or configured with a present evidence_ref. Anything else
    is an ERROR that blocks the deployable gate; metadata alone is never
    sufficient to flip deployment.deployable to true.
    """
    if not isinstance(deployment, dict) or not deployment.get("deployable"):
        return
    declaration_release = declaration.get("release_evidence")
    if not isinstance(declaration_release, dict):
        finding(findings, "ERROR", "RELEASE_EVIDENCE_REQUIRED", "deployable project must declare release_evidence", project_id)
        return
    for field in DEPLOYABLE_RELEASE_REQUIRED_FIELDS:
        entry = declaration_release.get(field)
        if not isinstance(entry, dict):
            finding(findings, "ERROR", "RELEASE_EVIDENCE_INSUFFICIENT", f"deployable project must declare release_evidence.{field}", project_id)
            continue
        state = entry.get("state")
        if state == "verified":
            continue
        if state == "configured":
            ref = entry.get("evidence_ref")
            if isinstance(ref, str) and ref and (project_path / ref).is_file():
                continue
            finding(findings, "ERROR", "RELEASE_EVIDENCE_INSUFFICIENT", f"deployable project release_evidence.{field} configured but evidence_ref missing: {ref!r}", project_id)
            continue
        finding(findings, "ERROR", "RELEASE_EVIDENCE_INSUFFICIENT", f"deployable project release_evidence.{field} state is {state!r}; verified or configured+ref is required", project_id)


def load_promotion_ledger(governance_root: Path, findings: list[Finding]) -> dict[str, dict[str, Any]]:
    """Load the deployment-promo ledger if present, with tamper detection.

    Returns a map of project_id -> latest promote entry. Missing ledger
    returns an empty map (no error). Tampered entries produce a single
    `PROMOTIONS_LEDGER_TAMPERED` ERROR on the findings list; the function
    still returns the on-disk entries so the audit can keep reporting.
    """
    try:
        import promote_project  # local import to keep this module standalone
    except ImportError:
        return {}
    path = promote_project.ledger_path(governance_root)
    sidecar = promote_project.ledger_sidecar_path(governance_root)
    if not path.is_file():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
        entries = payload.get("entries", [])
    except (OSError, json.JSONDecodeError) as exc:
        finding(findings, "ERROR", "PROMOTIONS_LEDGER_TAMPERED", f"cannot parse ledger: {exc}")
        return {}
    if sidecar.is_file():
        try:
            expected = sidecar.read_text(encoding="utf-8").strip()
            actual = promote_project.compute_ledger_hash(entries)
        except OSError:
            expected = ""
            actual = ""
        if expected and expected != actual:
            finding(findings, "ERROR", "PROMOTIONS_LEDGER_TAMPERED", f"ledger entries do not match recorded sidecar at {sidecar}")
    latest: dict[str, dict[str, Any]] = {}
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        project_id = entry.get("project_id")
        if not isinstance(project_id, str):
            continue
        if entry.get("kind") == "promote":
            latest[project_id] = entry
    return latest


def validate_deployment_promotion(deployment: Any, project_path: Path, project_id: str, governance_root: Path | None, findings: list[Finding]) -> None:
    """For projects with `deployable: true`, the audit must find a matching
    promotion ledger entry; a stale entry is a WARNING so the project still
    ships while the operator is asked to re-run promotion.
    """
    if not isinstance(deployment, dict) or not deployment.get("deployable"):
        return
    if governance_root is None:
        return
    ledger = load_promotion_ledger(governance_root, findings)
    entry = ledger.get(project_id)
    if entry is None:
        finding(findings, "ERROR", "DEPLOYMENT_PROMOTION", "deployable project has no matching entry in deployment/promotions.json", project_id)
        return
    recorded_revision = entry.get("revision")
    head_file = project_path / ".git" / "HEAD"
    if not head_file.is_file():
        return
    head = head_file.read_text(encoding="utf-8").strip()
    if head.startswith("ref:"):
        ref_name = head.split(":", 1)[1].strip()
        ref_path = project_path / ".git" / ref_name
        head = ref_path.read_text(encoding="utf-8").strip() if ref_path.is_file() else head
    if isinstance(recorded_revision, str) and head and recorded_revision != head and recorded_revision != "unknown":
        finding(findings, "WARNING", "DEPLOYMENT_PROMOTION_STALE", f"promoted revision {recorded_revision!r} differs from project HEAD {head!r}; re-promote at HEAD with scripts/promote_project.py {project_id} --evidence-ref <ref>", project_id)


def validate_product_quality(declaration: dict[str, Any], project_path: Path, findings: list[Finding], project_id: str) -> None:
    """Run the product-code quality checker on a project that opts in.

    The checker is imported in-process. The audit translates its findings into
    the existing Finding vocabulary: FAIL becomes ERROR, REVIEW_REQUIRED
    becomes WARNING, PASS produces no finding.
    """
    import product_code_quality  # local import to keep workspace_check importable without the checker
    import vocabulary as vocabulary_module  # local import for the same reason
    try:
        policy = product_code_quality.load_policy(declaration)
    except (product_code_quality.ConfigError, vocabulary_module.VocabularyError) as exc:
        finding(findings, "WARNING", "QUALITY_POLICY", f"product quality policy invalid: {exc}", project_id)
        return
    try:
        report = product_code_quality.evaluate(policy.with_root(project_path))
    except product_code_quality.ConfigError as exc:
        finding(findings, "WARNING", "QUALITY_POLICY", f"product quality check cannot run: {exc}", project_id)
        return
    for note in report.notes:
        if report.status == "REVIEW_REQUIRED":
            finding(findings, "WARNING", "QUALITY_REVIEW_REQUIRED", note, project_id)
        elif report.status == "FAIL":
            finding(findings, "ERROR", "QUALITY_FAILURE", note, project_id)
    for item in report.findings:
        level = "ERROR" if item.level == "ERROR" or report.status == "FAIL" else "WARNING"
        code = "QUALITY_TEST_CODE" if item.rule_id == "product-test-code" else "QUALITY_PLACEHOLDER"
        message = f"{item.file}:{item.line} {item.marker}".strip()
        finding(findings, level, code, message, project_id)


def promoted_project_ids(governance_root: Path | None = None) -> set[str]:
    """Project ids that hold a `promote` entry in the promotion ledger.

    A finding-free read on purpose: `load_promotion_ledger` owns the ledger's
    integrity findings and `validate_deployment_promotion` owns the staleness
    warning, so this reader exists only to answer the `production-evidence`
    surface question without adding a second stream of the same codes. An
    unreadable ledger yields an empty set, which reads as "no promotion
    evidence" rather than as a verified claim.
    """
    root = Path(governance_root) if governance_root is not None else _default_governance_root()
    path = root / "deployment" / "promotions.json"
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return set()
    entries = document.get("entries") if isinstance(document, dict) else None
    if not isinstance(entries, list):
        return set()
    return {
        str(entry.get("project_id"))
        for entry in entries
        if isinstance(entry, dict)
        and entry.get("kind") == "promote"
        and isinstance(entry.get("project_id"), str)
    }


def validate_readme_claims(
    declaration: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    governance_root: Path | None = None,
    coverage_gaps: list[tuple[str, str, int, str, str]] | None = None,
    promoted_projects: set[str] | None = None,
) -> None:
    """Verify this repository's README status claims against its file tree.

    Invoked for every registered project, triggered by claim presence and by
    nothing else. There is no `readme_claims` block, no `mode`, no opt-out and
    no per-repository exemption: a check that can be switched off is not a
    check (`design.md` §10, ruling 10). Only 2 of 81 declarations carry a
    `quality` block, so gating on that block would leave 79 repositories
    unexamined — which is why this is a sibling of `validate_product_quality`
    and not a branch inside it.

    Two output streams, by design:

    * a contradicted surface is a per-repository ERROR, one finding per
      surface, naming the README line, the surface, and the evidence path;
    * an unscoped statement is NOT a per-repository finding. It is appended to
      `coverage_gaps`, which the caller emits once for the whole run as the
      aggregate `QUALITY_README_CLAIM_UNSCOPED` CHECKER-COVERAGE metric.
    """
    import product_code_quality  # local import: keeps workspace_check importable without the checker

    if README_CLAIMS_UNSUPPORTED_FIELD in declaration:
        # Ruling 10: canonical behaviour applies anyway, and the unsupported
        # field is reported as a WARNING naming it. This is the whole refusal:
        # there is no branch here that reads the value.
        finding(
            findings,
            "WARNING",
            "QUALITY_REVIEW_REQUIRED",
            f"declaration carries an unsupported {README_CLAIMS_UNSUPPORTED_FIELD!r} field; "
            "README claims are verified for every project, honour no mode and cannot be switched off",
            project_id,
        )
    try:
        report = product_code_quality.evaluate_claims(
            project_path,
            declaration,
            governance_root=governance_root,
            manifest_read=read_manifest_dependencies,
            promoted_projects=promoted_projects,
        )
    except (product_code_quality.ConfigError, product_code_quality.VocabularyError) as exc:
        finding(
            findings,
            "WARNING",
            "QUALITY_REVIEW_REQUIRED",
            f"README claim check cannot run for {project_id!r}: {exc}",
            project_id,
        )
        return
    for item in report.contradicted:
        finding(
            findings,
            "ERROR",
            QUALITY_README_CLAIM_CONTRADICTED,
            item.remediation,
            project_id,
        )
    if coverage_gaps is not None:
        for gap in report.gaps:
            coverage_gaps.append((gap.project, gap.file, gap.line, gap.reason, gap.statement))


# Patterns used by the bounded static scan in `validate_standalone`.
# The scan only inspects the listed files, never globs, and never
# executes the project. Detected siblings are recorded as a
# STANDALONE_DETECTED_SIBLING warning but never override the declared
# status — the declaration is the source of truth.
_SIBLING_PATH_PATTERN = re.compile(r"((?:\.\./)+)([A-Za-z0-9_.-]+)")
