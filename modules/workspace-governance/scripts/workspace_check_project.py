"""Per-project declaration validation orchestration."""

from __future__ import annotations

import workspace_check as _facade
globals().update({key: value for key, value in vars(_facade).items() if not key.startswith("__")})

def validate_project(
    entry: dict[str, Any],
    workspace_root: Path,
    findings: list[Finding],
    strict: bool = True,
    governance_root: Path | None = None,
    *,
    check_publication: bool = True,
    capability_registry: CapabilityRegistry | None = None,
    provider_paths: dict[str, Path] | None = None,
    adoption_ledger: list[dict[str, Any]] | None = None,
    rules_drift_baseline: frozenset[tuple[str, str, str]] | None = None,
    observed_drift_keys: set[tuple[str, str, str]] | None = None,
    scanned_projects: set[str] | None = None,
    taxonomy_enabled: bool | None = None,
    spec_purpose_enabled: bool | None = None,
    verification_binding_enabled: bool | None = None,
    readme_claim_coverage: list[tuple[str, str, int, str, str]] | None = None,
    promoted_projects: set[str] | None = None,
    gate_debt: dict[str, tuple[str, ...]] | None = None,
    gate_debt_baseline: frozenset[tuple[str, str]] | None = None,
    gate_debt_exceptions: dict[tuple[str, str], str] | None = None,
    interface_index: dict[tuple[str, str], dict[str, Any]] | None = None,
    interface_escalation: dict[str, str] | None = None,
) -> None:
    project_id = str(entry.get("id"))
    project_path = workspace_root / str(entry.get("path"))
    declaration_path = project_path / ".project.json"
    if not declaration_path.exists():
        if strict:
            finding(findings, "ERROR", "DECLARATION_MISSING", "adopted project must contain .project.json", project_id)
        return
    declaration = load_json(declaration_path, findings, "PROJECT_DECLARATION")
    if declaration is None:
        return
    if declaration.get("schema_version") != 1:
        finding(findings, "ERROR", "DECLARATION_SCHEMA", "project declaration schema_version must be 1", project_id)
    if declaration.get("id") != project_id:
        finding(findings, "ERROR", "DECLARATION_ID", f"declaration id must be {project_id!r}", project_id)
    validate_kind(declaration, findings, project_id, governance_root=governance_root)
    if taxonomy_audit_enabled(taxonomy_enabled):
        validate_taxonomy(entry, declaration, findings, project_id, governance_root=governance_root)
    if spec_purpose_audit_enabled(spec_purpose_enabled):
        validate_spec_purposes(project_path, findings, project_id)
    profile = str(entry.get("profile"))
    required, requires_ci, requires_openspec, requires_verification = profile_requirements(profile)
    for relative in required:
        if not (project_path / relative).exists():
            finding(findings, "ERROR", "REQUIRED_FILE", f"missing required file: {relative}", project_id)
    verification = declaration.get("verification")
    if requires_verification and (not isinstance(verification, dict) or not str(verification.get("command", "")).strip()):
        finding(findings, "ERROR", "VERIFICATION_COMMAND", "a real verification command is required", project_id)
    if verification_binding_audit_enabled(verification_binding_enabled):
        validate_declared_verification_is_bound(project_path, findings, project_id)
    if profile != "inventory":
        from repository_metadata import validate_discoverability, validate_publication

        if not (project_path / "LICENSE").is_file():
            finding(findings, "ERROR", "LICENSE_MISSING", "standard project must contain a root LICENSE", project_id)
        validate_discoverability(declaration, project_path, findings, project_id)
        if check_publication:
            validate_publication(declaration, findings, project_id)
    if requires_ci and not any((project_path / candidate).exists() for candidate in ("Jenkinsfile", ".gitlab-ci.yml")) and not (project_path / ".github" / "workflows").is_dir():
        finding(findings, "ERROR", "CI_MISSING", "a CI workflow is required by this profile", project_id)
    if requires_openspec and not (project_path / "openspec").is_dir():
        finding(findings, "ERROR", "OPENSPEC_MISSING", "OpenSpec directory is required by this profile", project_id)
    if requires_openspec:
        validate_change_names(project_path, findings, project_id)
    validate_openspec_seriality(project_path, findings, project_id)
    # Unconditional, unlike the rules above: a repository on any profile that
    # carries gate debt may not open a new change, and `inventory` is the
    # profile most of the debted repositories are on. `None` means the caller
    # did not load the records, so read them from `governance_root` here.
    if gate_debt is None or gate_debt_baseline is None or gate_debt_exceptions is None:
        try:
            gate_debt = load_gate_debt_record(governance_root)
        except GateDebtRecordError:
            gate_debt = None
        try:
            gate_debt_baseline = load_gate_debt_baseline(governance_root)
        except GateDebtBaselineError:
            gate_debt_baseline = None
        try:
            gate_debt_exceptions = load_gate_debt_exceptions(governance_root)
        except GateDebtExceptionsError:
            gate_debt_exceptions = None
    validate_new_change_with_gate_debt(
        project_path,
        findings,
        project_id,
        debt=gate_debt,
        baseline=gate_debt_baseline,
        exceptions=gate_debt_exceptions,
    )
    deployment = declaration.get("deployment")
    if not isinstance(deployment, dict) or not isinstance(deployment.get("deployable"), bool):
        finding(findings, "ERROR", "DEPLOYMENT_DECLARATION", "deployment.deployable must be boolean", project_id)
    else:
        compose = deployment.get("compose_file")
        if compose is not None and (not isinstance(compose, str) or not compose or Path(compose).is_absolute() or ".." in Path(compose).parts):
            finding(findings, "ERROR", "DEPLOYMENT_COMPOSE", "deployment.compose_file must be repository-relative when present", project_id)
        template_type = deployment.get("template_type")
        template_version = deployment.get("template_version")
        rendered_files = deployment.get("rendered_files")
        template_state = deployment.get("template_state")
        if template_type is not None and (not isinstance(template_type, str) or not re.fullmatch(r"[a-z][a-z0-9]*(?:-[a-z0-9]+)*", template_type)):
            finding(findings, "ERROR", "DEPLOYMENT_TEMPLATE", "deployment.template_type must be a kebab-case identifier", project_id)
        if template_version is not None and (not isinstance(template_version, int) or isinstance(template_version, bool) or template_version < 1):
            finding(findings, "ERROR", "DEPLOYMENT_TEMPLATE", "deployment.template_version must be a positive integer", project_id)
        if rendered_files is not None:
            if not isinstance(rendered_files, list) or len(set(rendered_files)) != len(rendered_files) or any(not isinstance(item, str) or Path(item).is_absolute() or ".." in Path(item).parts for item in rendered_files):
                finding(findings, "ERROR", "DEPLOYMENT_TEMPLATE", "deployment.rendered_files must contain unique repository-relative paths", project_id)
        if template_state is not None and template_state not in {"configured", "verified", "blocked"}:
            finding(findings, "ERROR", "DEPLOYMENT_TEMPLATE", "deployment.template_state is invalid", project_id)
    if isinstance(deployment, dict) and deployment.get("deployable"):
        compose = deployment.get("compose_file")
        job = deployment.get("jenkins_job")
        if not isinstance(compose, str) or not compose:
            finding(findings, "ERROR", "DEPLOYMENT_COMPOSE", "deployable project must declare compose_file", project_id)
        elif not (project_path / compose).is_file():
            finding(findings, "ERROR", "DEPLOYMENT_COMPOSE", f"declared Compose file is missing: {compose}", project_id)
        if not isinstance(job, str) or not job:
            finding(findings, "ERROR", "DEPLOYMENT_JOB", "deployable project must declare jenkins_job", project_id)
    capabilities = declaration.get("capabilities")
    supported: set[str] | None = None
    if capabilities is not None and governance_root is not None:
        spec = load_profile_spec(governance_root, profile)
        if spec is not None:
            raw = spec.get("supported_capabilities")
            if isinstance(raw, list):
                supported = {item for item in raw if isinstance(item, str)}
    if capabilities is not None:
        validate_capabilities(capabilities, project_path, findings, project_id, supported=supported)
    if capability_registry is not None:
        paths = provider_paths or {}
        resolved = lambda provider, capability: provider_capability_verified(workspace_root, paths, provider, capability)  # noqa: E731
        measure_capability_adoption(
            declaration,
            project_path,
            findings,
            project_id,
            capability_registry,
            provider_verified=resolved,
        )
        validate_adoption_grant(
            declaration,
            project_path,
            findings,
            project_id,
            capability_registry=capability_registry,
            provider_paths=paths,
            ledger=adoption_ledger or [],
            provider_verified=resolved,
        )
        # The exemption's owner, and the next action that ends it. Read-only,
        # and emitted once per exempt capability rather than once per project.
        report_unverified_exempt_capabilities(
            project_id,
            findings,
            capability_registry,
            resolved,
        )
    validate_interfaces(
        declaration,
        project_path,
        findings,
        project_id,
        index=interface_index,
        escalation=interface_escalation,
    )
    release_evidence = declaration.get("release_evidence")
    if release_evidence is not None:
        validate_release_evidence(release_evidence, project_path, findings, project_id)
    if isinstance(deployment, dict) and deployment.get("deployable"):
        validate_deployment_evidence(deployment, declaration, project_path, findings, project_id)
        validate_deployment_promotion(deployment, project_path, project_id, governance_root, findings)
    if isinstance(declaration.get("quality"), dict):
        validate_product_quality(declaration, project_path, findings, project_id)
    # Claim-triggered, not gated: runs for every registered project, and only
    # a present-tense absence claim in the README makes it produce anything.
    validate_readme_claims(
        declaration,
        project_path,
        findings,
        project_id,
        governance_root=governance_root,
        coverage_gaps=readme_claim_coverage,
        promoted_projects=promoted_projects,
    )
    if isinstance(declaration.get("standalone"), dict) or isinstance(declaration.get("integrations"), list):
        _facade.validate_standalone(
            declaration, project_path, findings, project_id, governance_root=governance_root
        )
    if entry.get("adoption") == "adopted":
        validate_handoff_quality(
            project_path,
            findings,
            project_id,
            rules_drift_baseline=rules_drift_baseline,
            observed_drift_keys=observed_drift_keys,
            scanned_projects=scanned_projects,
        )


def scan_for_secrets(value: Any, findings: list[Finding], location: str = "registry", *, secret_words: "re.Pattern[str] | None" = None, governance_root: Path | None = None) -> None:
    pattern = secret_words if secret_words is not None else secret_words_for(governance_root)
    if isinstance(value, dict):
        for key, child in value.items():
            if pattern.search(str(key)) and child not in (None, "", False):
                finding(findings, "ERROR", "SECRET_VALUE", f"secret-like field contains a value: {location}.{key}")
            scan_for_secrets(child, findings, f"{location}.{key}", secret_words=pattern)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            scan_for_secrets(child, findings, f"{location}[{index}]", secret_words=pattern)
