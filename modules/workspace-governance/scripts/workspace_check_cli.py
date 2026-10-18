"""Workspace audit orchestration and CLI presentation."""

from __future__ import annotations

import workspace_check as _facade
globals().update({key: value for key, value in vars(_facade).items() if not key.startswith("__")})

def run(args: argparse.Namespace) -> tuple[list[Finding], dict[str, int]]:
    findings: list[Finding] = []
    governance_root = Path(__file__).resolve().parents[1]
    try:
        vocabulary = _vocabulary_for(governance_root)
    except Exception as exc:
        finding(findings, "ERROR", "VOCABULARY", str(exc))
        return findings, {"errors": len(findings), "warnings": 0}
    canonical_profiles = vocabulary["profiles"]
    canonical_kinds = vocabulary["kinds"]
    import vocabulary as vocabulary_module

    secret_words = vocabulary_module.secret_pattern(vocabulary["secret_field_substrings"])
    registry_path = Path(args.registry)
    if not registry_path.is_absolute():
        registry_path = governance_root / registry_path
    registry = load_json(registry_path, findings, "REGISTRY_FILE")
    if registry is None:
        return findings, {"errors": len(findings), "warnings": 0}
    configured_root = getattr(args, "root", None) or os.environ.get("WORKSPACE_ROOT") or registry.get("workspace_root")
    if args.self_check:
        workspace_root = governance_root
    elif configured_root:
        workspace_root = Path(configured_root).expanduser().resolve()
    else:
        workspace_root = Path.cwd().resolve()
    load_decisions_register(governance_root, registry, findings, self_check=bool(args.self_check))
    capability_registry = load_capability_registry(governance_root, findings)
    adoption_ledger = load_adoption_ledger(governance_root, findings)
    # Ratchet baseline for rules drift. Loaded once, fail-closed. A load failure
    # emits one ERROR and disables the per-project delta rather than guessing: an
    # empty baseline would report every existing gap as new and fail the fleet for
    # the wrong reason. The ERROR already blocks, so the per-project pass is not
    # needed to make the run fail.
    try:
        rules_drift_baseline: frozenset[tuple[str, str, str]] | None = load_rules_drift_baseline(governance_root)
    except RulesDriftBaselineError as exc:
        finding(findings, "ERROR", "RULES_DRIFT_BASELINE", str(exc))
        rules_drift_baseline = None
    observed_drift_keys: set[tuple[str, str, str]] = set()
    scanned_projects: set[str] = set()
    # The gate-debt precondition, read fail-closed in the same shape as the
    # ratchet above. Both records are loaded once for the whole run. A load
    # failure emits one ERROR and closes the rule: no repository is reported as
    # debt-free on the strength of a record that was never read, and the ERROR
    # already fails the run.
    try:
        gate_debt: dict[str, tuple[str, ...]] | None = load_gate_debt_record(governance_root)
    except GateDebtRecordError as exc:
        finding(findings, "ERROR", QUALITY_NEW_CHANGE_WITH_GATE_DEBT, str(exc))
        gate_debt = None
    try:
        gate_debt_baseline: frozenset[tuple[str, str]] | None = load_gate_debt_baseline(governance_root)
    except GateDebtBaselineError as exc:
        finding(findings, "ERROR", QUALITY_NEW_CHANGE_WITH_GATE_DEBT, str(exc))
        gate_debt_baseline = None
    try:
        gate_debt_exceptions: dict[tuple[str, str], str] | None = load_gate_debt_exceptions(governance_root)
    except GateDebtExceptionsError as exc:
        finding(findings, "ERROR", QUALITY_NEW_CHANGE_WITH_GATE_DEBT, str(exc))
        gate_debt_exceptions = None
    # CHECKER-COVERAGE accumulator for the README claim rule. The unscoped
    # metric is emitted ONCE for the whole run, not per repository, because the
    # information it carries is about this checker's coverage and not about a
    # repository's defect (ruling 7).
    readme_claim_coverage: list[tuple[str, str, int, str, str]] = []
    # The interface ramp, read once per run from the decisions register: a
    # WARNING today, blocking after the recorded date. Unreadable input leaves
    # every interface finding a WARNING, because nothing becomes blocking on a
    # record that could not be read.
    interface_escalation = interface_escalation_dates(governance_root)
    # Provider identity is read from the registry's own entries, never from a
    # directory name, so the eight ids that differ from their folder name
    # resolve exactly like the rest.
    provider_paths = {
        str(item.get("id")): workspace_root / str(item.get("path"))
        for item in (registry.get("projects") or [])
        if isinstance(item, dict) and isinstance(item.get("id"), str) and isinstance(item.get("path"), str)
    }
    projects = validate_registry(registry, workspace_root, findings, check_paths=not args.self_check, canonical_profiles=canonical_profiles, governance_root=governance_root)
    validate_profile_docs(governance_root, findings, canonical_profiles=canonical_profiles)
    scan_for_secrets(registry, findings, secret_words=secret_words)
    effective_projects = list(projects)
    explicit_paths = {str(entry.get("path")) for entry in projects}
    discovery = registry.get("discovery") or {}
    if discovery.get("mode") in {"immediate-directories", "explicit-plus-immediate-directories"} and not args.self_check:
        policy = discovery.get("unregistered_policy", "report")
        for discovered in discover_projects(workspace_root, registry):
            if discovered["path"] in explicit_paths:
                continue
            level = "ERROR" if policy == "fail" else "WARNING"
            finding(findings, level, "DISCOVERED_UNREGISTERED", "project folder is discovered but not registered; local metadata is the source of truth", discovered["id"])
            declaration_path = workspace_root / discovered["path"] / ".project.json"
            if declaration_path.is_file():
                try:
                    declaration = json.loads(declaration_path.read_text(encoding="utf-8"))
                except (OSError, json.JSONDecodeError):
                    declaration = {}
                if isinstance(declaration, dict):
                    effective_projects.append({
                        "id": discovered["id"],
                        "path": discovered["path"],
                        "profile": declaration.get("profile", "inventory"),
                        "lifecycle": declaration.get("lifecycle", "inventory"),
                        "adoption": "pending",
                    })
    if args.self_check:
        selected = [{**entry, "path": "."} for entry in projects if entry.get("id") == "workspace-governance"]
        # The interface index is built from the declarations this run audits,
        # so a self-check resolves a `consumes` edge against the governance
        # repository's own declaration and a fleet audit against all of them.
        interface_index = build_interface_index(workspace_root, selected)
        for entry in selected:
            validate_project(
                entry,
                workspace_root,
                findings,
                governance_root=governance_root,
                check_publication=False,
                capability_registry=capability_registry,
                provider_paths=provider_paths,
                adoption_ledger=adoption_ledger,
                rules_drift_baseline=rules_drift_baseline,
                observed_drift_keys=observed_drift_keys,
                scanned_projects=scanned_projects,
                readme_claim_coverage=readme_claim_coverage,
                promoted_projects=promoted_project_ids(governance_root),
                gate_debt=gate_debt,
                gate_debt_baseline=gate_debt_baseline,
                gate_debt_exceptions=gate_debt_exceptions,
                interface_index=interface_index,
                interface_escalation=interface_escalation,
            )
    else:
        selected = effective_projects
        if args.project:
            selected = [entry for entry in effective_projects if entry.get("id") == args.project or entry.get("path") == args.project]
            if not selected:
                finding(findings, "ERROR", "PROJECT_UNKNOWN", f"project not found in registry: {args.project}")
        interface_index = build_interface_index(workspace_root, effective_projects)
        for entry in selected:
            adopted = entry.get("adoption", "pending") == "adopted"
            discovered_only = entry.get("path") not in explicit_paths
            if args.audit or args.project or adopted:
                validate_project(
                    entry,
                    workspace_root,
                    findings,
                    strict=not discovered_only,
                    governance_root=governance_root,
                    capability_registry=capability_registry,
                    provider_paths=provider_paths,
                    adoption_ledger=adoption_ledger,
                    rules_drift_baseline=rules_drift_baseline,
                    observed_drift_keys=observed_drift_keys,
                    scanned_projects=scanned_projects,
                    readme_claim_coverage=readme_claim_coverage,
                    promoted_projects=promoted_project_ids(governance_root),
                    gate_debt=gate_debt,
                    gate_debt_baseline=gate_debt_baseline,
                    gate_debt_exceptions=gate_debt_exceptions,
                    interface_index=interface_index,
                    interface_escalation=interface_escalation,
                )
            else:
                finding(findings, "WARNING", "ADOPTION_PENDING", "project is registered but not yet adopted", str(entry.get("id")))
    # One aggregate CHECKER-COVERAGE metric for the whole run, naming every
    # unscoped statement and the repository it came from. Level WARNING and
    # `REVIEW_REQUIRED` in the checker's own vocabulary: never a pass, and
    # never attributed to a repository.
    if readme_claim_coverage:
        names = "; ".join(
            f"{project} {file}:{line} ({reason}) {statement}"
            for project, file, line, reason, statement in readme_claim_coverage
        )
        finding(
            findings,
            "WARNING",
            QUALITY_README_CLAIM_UNSCOPED,
            f"checker coverage: {len(readme_claim_coverage)} README status statement(s) name no surface this "
            f"check can resolve, so they are untested rather than verified: {names}",
        )
    # Decay signal, not a failure: repositories that have fixed their drift make
    # baseline entries redundant, and the baseline should be tightened to match.
    # This is a WARNING and never touches the exit code.
    #
    # It is evaluated ONLY on a full audit. `--self-check` scans one repository
    # and `--project` scans one entry, so comparing either against a fleet-wide
    # snapshot would report the unscanned remainder as "redundant" and cry wolf.
    scanned_whole_fleet = (
        not args.self_check
        and not args.project
        and {key[0] for key in rules_drift_baseline}.issubset(scanned_projects)
    ) if rules_drift_baseline is not None else False
    if scanned_whole_fleet:
        redundant = rules_drift_baseline - observed_drift_keys
        if redundant:
            finding(
                findings,
                "WARNING",
                "RULES_DRIFT_BASELINE_STALE",
                f"{RULES_DRIFT_BASELINE_FILE} has {len(redundant)} entry(ies) no longer reported; tighten the baseline",
            )
    summary = {"errors": sum(item.level == "ERROR" for item in findings), "warnings": sum(item.level == "WARNING" for item in findings)}
    return findings, summary


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", help="workspace root; defaults to registry workspace_root")
    parser.add_argument("--registry", default="projects.json")
    parser.add_argument("--project", help="validate one registered project")
    parser.add_argument("--audit", action="store_true", help="validate every registered project and fail on adoption gaps")
    parser.add_argument("--self-check", action="store_true", help="validate the governance repository only")
    parser.add_argument("--json", action="store_true", dest="as_json")
    parser.add_argument(
        "--decisions",
        action="store_true",
        help="print a summary of the toolchain decision register and exit (loader errors are non-zero)",
    )
    args = parser.parse_args(argv)
    if args.decisions:
        return _print_decisions_summary(args)
    findings, summary = run(args)
    if args.as_json:
        print(json.dumps({"summary": summary, "findings": [asdict(item) for item in findings]}, indent=2, sort_keys=True))
    else:
        for item in findings:
            prefix = f"[{item.project}] " if item.project else ""
            print(f"{item.level} {item.code}: {prefix}{item.message}")
        print(f"Summary: errors={summary['errors']} warnings={summary['warnings']}")
    if any(item.code == "VOCABULARY" for item in findings):
        return 2
    return 1 if summary["errors"] else 0


def _print_decisions_summary(args: argparse.Namespace) -> int:
    """Print the toolchain decision register summary and exit.

    Loader failures (missing file, malformed JSON, bad shape, unknown registry
    references) produce a non-zero exit and a short message on stderr. The
    register is owned by the governance root; this summary reads from there
    regardless of `--root` so the surface is stable for downstream tooling.
    """
    import decisions as decisions_module

    governance_root = _default_governance_root()
    register_path = decisions_module.decisions_path(governance_root)
    try:
        loaded = decisions_module.load_decisions(register_path)
    except decisions_module.DecisionError as exc:
        print(f"ERROR decisions register: {exc}", file=sys.stderr)
        return 1
    registry_path = governance_root / args.registry
    if not registry_path.is_file():
        print(
            f"ERROR decisions register: registry file not found at {registry_path}",
            file=sys.stderr,
        )
        return 1
    try:
        registry = json.loads(registry_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"ERROR decisions register: cannot read registry {registry_path}: {exc}", file=sys.stderr)
        return 1
    if not isinstance(registry, dict):
        print(f"ERROR decisions register: registry must be a JSON object: {registry_path}", file=sys.stderr)
        return 1
    registry_ids = {
        entry.get("id")
        for entry in (registry.get("projects") or [])
        if isinstance(entry, dict) and isinstance(entry.get("id"), str)
    }
    offenders = decisions_module.validate_against_registry(loaded, registry_ids)
    if offenders:
        joined = ", ".join(sorted(offenders))
        print(
            f"ERROR decisions register: unknown registry references: {joined}",
            file=sys.stderr,
        )
        return 1
    summary = [
        {"id": entry.id, "title": entry.title, "status": entry.status, "date": entry.date}
        for entry in loaded
    ]
    if args.as_json:
        print(json.dumps({"decisions": summary, "total": len(summary)}, indent=2, sort_keys=True))
        return 0
    for entry in summary:
        print(f"{entry['id']}\t{entry['status']}\t{entry['date']}\t{entry['title']}")
    print(f"Total decisions: {len(summary)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
