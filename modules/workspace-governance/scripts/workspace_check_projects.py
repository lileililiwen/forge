"""Project registry discovery and taxonomy validation."""

from __future__ import annotations

import workspace_check as _facade
globals().update({key: value for key, value in vars(_facade).items() if not key.startswith("__")})

def discover_projects(workspace_root: Path, registry: dict[str, Any]) -> list[dict[str, str]]:
    """Discover immediate child projects without persisting a machine path."""
    discovery = registry.get("discovery") or {}
    mode = discovery.get("mode", "explicit")
    if mode not in {"immediate-directories", "explicit-plus-immediate-directories"}:
        return []
    excluded = {str(item) for item in discovery.get("exclude", []) if isinstance(item, str)}
    projects: list[dict[str, str]] = []
    try:
        children = sorted(workspace_root.iterdir(), key=lambda item: item.name)
    except OSError:
        return projects
    for child in children:
        if child.name.startswith(".") or child.name in excluded or not child.is_dir() or child.is_symlink():
            continue
        if not any((child / marker).exists() for marker in PROJECT_MARKERS):
            continue
        projects.append({"id": child.name, "path": child.relative_to(workspace_root).as_posix()})
    return projects


def load_json(path: Path, findings: list[Finding], code: str) -> dict[str, Any] | None:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        finding(findings, "ERROR", code, f"missing JSON file: {path}")
        return None
    except (OSError, json.JSONDecodeError) as exc:
        finding(findings, "ERROR", code, f"invalid JSON {path}: {exc}")
        return None
    if not isinstance(value, dict):
        finding(findings, "ERROR", code, f"JSON object required: {path}")
        return None
    return value


def load_decisions_register(
    governance_root: Path,
    registry: dict[str, Any],
    findings: list[Finding],
    *,
    self_check: bool,
) -> "list[object] | None":
    """Load `decisions.json` from the governance root and surface findings.

    The register is governance-owned and is required for the governance
    self-check (ERROR `DECISIONS_REGISTER_MISSING`). A portfolio audit on
    sibling repos tolerates the absence of the register (single WARNING
    `DECISIONS_REGISTER_ABSENT`); broken shapes and unknown registry
    references are always errors. Returns the parsed decisions on success,
    `None` on any failure path (the caller does not need the value).

    The `WORKSPACE_DECISIONS_PATH` env var overrides the default location
    so tests can stage a per-workspace register without touching the
    governance root file. Production callers never set it.
    """
    import decisions as decisions_module

    override = os.environ.get("WORKSPACE_DECISIONS_PATH")
    if override:
        register_path = Path(override)
    else:
        register_path = decisions_module.decisions_path(governance_root)
    if not register_path.is_file():
        if self_check:
            finding(findings, "ERROR", DECISIONS_REGISTER_MISSING,
                    f"decisions register missing at {register_path}")
        else:
            finding(findings, "WARNING", DECISIONS_REGISTER_ABSENT,
                    f"decisions register absent at {register_path}; "
                    "self-check mode requires it")
        return None
    try:
        loaded = decisions_module.load_decisions(register_path)
    except decisions_module.DecisionError as exc:
        finding(findings, "ERROR", DECISIONS_REGISTER_INVALID, str(exc))
        return None
    registry_ids = {
        entry.get("id")
        for entry in (registry.get("projects") or [])
        if isinstance(entry, dict) and isinstance(entry.get("id"), str)
    }
    offenders = decisions_module.validate_against_registry(loaded, registry_ids)
    if offenders:
        finding(
            findings,
            "ERROR",
            DECISIONS_REGISTRY_REFERENCE,
            f"decisions register references unknown registry ids: {offenders}",
        )
        return None
    return loaded


def validate_registry(registry: dict[str, Any], workspace_root: Path, findings: list[Finding], check_paths: bool = True, *, canonical_profiles: set[str] | frozenset[str] | None = None, governance_root: Path | None = None, taxonomy_enabled: bool | None = None) -> list[dict[str, Any]]:
    if registry.get("schema_version") != 1:
        finding(findings, "ERROR", "REGISTRY_SCHEMA", "registry schema_version must be 1")
    projects = registry.get("projects")
    if not isinstance(projects, list):
        finding(findings, "ERROR", "REGISTRY_PROJECTS", "registry projects must be an array")
        return []
    seen: set[str] = set()
    seen_paths: set[str] = set()
    for entry in projects:
        if not isinstance(entry, dict):
            finding(findings, "ERROR", "REGISTRY_ENTRY", "each registry project must be an object")
            continue
        project_id = entry.get("id")
        path_value = entry.get("path")
        if not isinstance(project_id, str) or not PROJECT_ID.fullmatch(project_id):
            finding(findings, "ERROR", "PROJECT_ID", f"invalid project id: {project_id!r}")
            continue
        if project_id in seen:
            finding(findings, "ERROR", "PROJECT_DUPLICATE", f"duplicate project id: {project_id}", project_id)
        seen.add(project_id)
        if not isinstance(path_value, str) or Path(path_value).is_absolute() or ".." in Path(path_value).parts:
            finding(findings, "ERROR", "PROJECT_PATH", f"project path must be repository-relative: {path_value!r}", project_id)
        else:
            if path_value in seen_paths:
                finding(findings, "ERROR", "PROJECT_PATH_DUPLICATE", f"duplicate project path: {path_value}", project_id)
            seen_paths.add(path_value)
            if check_paths:
                project_path = workspace_root / path_value
                if not project_path.is_dir():
                    finding(findings, "ERROR", "PROJECT_MISSING", f"registered project directory missing: {project_path}", project_id)
        profile = entry.get("profile")
        if not isinstance(profile, str) or not profile:
            finding(findings, "ERROR", "PROFILE_MISSING", "project profile is required", project_id)
        else:
            profiles = canonical_profiles if canonical_profiles is not None else canonical_profiles_for(governance_root)
            if profile not in profiles:
                finding(findings, "ERROR", "PROFILE_UNKNOWN", f"unknown profile: {profile}; canonical profiles: {sorted(profiles)}", project_id)
        lifecycle = entry.get("lifecycle")
        if lifecycle not in LIFECYCLES:
            finding(findings, "ERROR", "LIFECYCLE", f"invalid lifecycle: {lifecycle!r}", project_id)
        if taxonomy_audit_enabled(taxonomy_enabled):
            # Loaded here rather than up front: while the gate is closed the
            # audit must not depend on the axis sets at all.
            validate_registry_taxonomy(entry, findings, project_id, governance_root=governance_root)
    return projects


def profile_requirements(profile: str) -> tuple[list[str], bool, bool, bool]:
    if profile == "inventory":
        return [".project.json"], False, False, False
    if profile == "tooling":
        return ["README.md", ".project.json"], False, False, True
    if profile == "deployment":
        return ["README.md", ".project.json"], False, False, True
    if profile == "dotnet-template":
        return ["README.md", "AGENTS.md", ".project.json"], True, False, True
    if profile == "governance":
        return ["README.md", "ROADMAP.md", "HANDOFF.md", "AGENTS.md", ".ai-rules/workflow.md", ".ai-rules/completion.md", ".project.json"], True, False, True
    return ["README.md", "ROADMAP.md", "HANDOFF.md", "AGENTS.md", ".ai-rules/workflow.md", ".ai-rules/completion.md", ".project.json"], True, True, True


def load_profile_spec(governance_root: Path, profile: str) -> dict[str, Any] | None:
    """Load the profile JSON next to the governance checker, if present.

    Profiles are documentation by default. The checker only reads them to
    surface a soft warning when a project declares a capability its profile
    does not list as supported.
    """
    path = governance_root / "profiles" / f"{profile}.json"
    if not path.is_file():
        return None
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if not isinstance(value, dict):
        return None
    return value


def validate_profile_docs(governance_root: Path, findings: list[Finding], *, canonical_profiles: set[str] | frozenset[str] | None = None) -> None:
    """A `profiles/<name>.json` document naming a non-canonical profile is ERROR `PROFILE_DOC_UNKNOWN`."""
    profiles = canonical_profiles if canonical_profiles is not None else canonical_profiles_for(governance_root)
    profiles_dir = Path(governance_root) / "profiles"
    if not profiles_dir.is_dir():
        return
    for child in sorted(profiles_dir.glob("*.json")):
        try:
            value = json.loads(child.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            finding(findings, "ERROR", "PROFILE_DOC_UNKNOWN", f"cannot parse profile document {child.name}: {exc}")
            continue
        if not isinstance(value, dict):
            finding(findings, "ERROR", "PROFILE_DOC_UNKNOWN", f"profile document must contain a JSON object: {child.name}")
            continue
        documented = value.get("id", child.stem)
        if child.stem not in profiles or documented not in profiles:
            finding(findings, "ERROR", "PROFILE_DOC_UNKNOWN", f"profile document {child.name} names a non-canonical profile: {documented!r}; canonical profiles: {sorted(profiles)}")


def _is_self_consistent_derived_kind(
    declaration: dict[str, Any],
    kind: str,
    *,
    canonical_roles: set[str] | frozenset[str],
    canonical_stacks: set[str] | frozenset[str],
) -> bool:
    """Is `kind` the canonical form derived from this declaration's own axes?

    True only when both axes are present, both are canonical, and `kind`
    equals `derive_kind(role, stack)`. `kind` is a *derived, deprecated*
    composite, so its canonical form is whatever the two axes derive — and the
    derivation is a product of two enumerated sets, so it is not enumerable as
    a hand-maintained list. Absent or non-canonical axes return False: nothing
    has been derived yet, so the legacy form is not measurable.
    """
    role = declaration.get("role")
    stack = declaration.get("stack")
    if not isinstance(role, str) or not isinstance(stack, str):
        return False
    if role not in canonical_roles or stack not in canonical_stacks:
        return False
    return kind == derive_kind(role, stack)


def validate_kind(declaration: dict[str, Any], findings: list[Finding], project_id: str, *, canonical_kinds: set[str] | frozenset[str] | None = None, governance_root: Path | None = None, canonical_roles: set[str] | frozenset[str] | None = None, canonical_stacks: set[str] | frozenset[str] | None = None) -> None:
    """A declaration whose `kind` is neither canonical nor self-consistent is WARNING `KIND_UNKNOWN`.

    A `kind` outside the legacy `kinds` vocabulary is known when it is the
    value its own two axes derive, because the field is derived and deprecated
    and the axes are its authority. `kinds` is the frozen legacy vocabulary, not
    the authority on derived values, so a canonical derivation is not held to
    it; everything else — a hand-authored legacy value that disagrees with its
    own derivation, a one-off value in neither form, and any value whose axes
    are absent or non-canonical — still reports.

    Kind is descriptive and never drives profile requirements.
    """
    kinds = canonical_kinds if canonical_kinds is not None else canonical_kinds_for(governance_root)
    kind = declaration.get("kind")
    if isinstance(kind, str) and kind in kinds:
        return
    if isinstance(kind, str):
        # Only the value that would otherwise be reported consults the axes, so
        # a canonical `kind` keeps resolving with no vocabulary read at all.
        roles = canonical_roles if canonical_roles is not None else canonical_roles_for(governance_root)
        stacks = canonical_stacks if canonical_stacks is not None else canonical_stacks_for(governance_root)
        if _is_self_consistent_derived_kind(declaration, kind, canonical_roles=roles, canonical_stacks=stacks):
            return
    finding(findings, "WARNING", "KIND_UNKNOWN", f"unknown kind: {kind!r}; canonical kinds: {sorted(kinds)}", project_id)


def _resolve_axes(
    surface: dict[str, Any],
    findings: list[Finding],
    project_id: str,
    *,
    canonical_roles: set[str] | frozenset[str],
    canonical_stacks: set[str] | frozenset[str],
    label: str,
) -> tuple[str | None, str | None, bool]:
    """Read one authority surface's two axes.

    Returns `(role, stack, both_canonical)`. An absent or non-canonical value
    is WARNING `TAXONOMY_AXIS_UNKNOWN` naming the offending value and both
    canonical sets; the two axes are independent, so one bad value does not
    suppress the report on the other.
    """
    axes: dict[str, str | None] = {}
    canonical = True
    for axis, values in (("role", canonical_roles), ("stack", canonical_stacks)):
        value = surface.get(axis)
        if value is None:
            finding(findings, "WARNING", TAXONOMY_AXIS_UNKNOWN,
                    f"{label} does not declare a canonical {axis}; canonical {axis} values: {sorted(values)}", project_id)
            axes[axis] = None
            canonical = False
            continue
        if not isinstance(value, str) or value not in values:
            finding(findings, "WARNING", TAXONOMY_AXIS_UNKNOWN,
                    f"{label} {axis} is not canonical: {value!r}; canonical roles: {sorted(canonical_roles)}; "
                    f"canonical stacks: {sorted(canonical_stacks)}", project_id)
            axes[axis] = None
            canonical = False
            continue
        axes[axis] = value
    return axes["role"], axes["stack"], canonical


def validate_registry_taxonomy(
    entry: dict[str, Any],
    findings: list[Finding],
    project_id: str,
    *,
    canonical_roles: set[str] | frozenset[str] | None = None,
    canonical_stacks: set[str] | frozenset[str] | None = None,
    governance_root: Path | None = None,
) -> None:
    """A registry entry's `role`/`stack` must be canonical when present.

    A registry entry may legitimately omit the axes while the fleet migration
    is in flight, so an absent value is still reported as WARNING
    `TAXONOMY_AXIS_UNKNOWN` on the surface that omits it. Registry
    `id`/`path` handling is untouched: an `id` that differs in spelling from
    its `path` is resolved through `path` and is never a finding.
    """
    roles = canonical_roles if canonical_roles is not None else canonical_roles_for(governance_root)
    stacks = canonical_stacks if canonical_stacks is not None else canonical_stacks_for(governance_root)
    _resolve_axes(entry, findings, project_id, canonical_roles=roles, canonical_stacks=stacks, label="registry entry")


def validate_taxonomy(
    entry: dict[str, Any],
    declaration: dict[str, Any],
    findings: list[Finding],
    project_id: str,
    *,
    canonical_roles: set[str] | frozenset[str] | None = None,
    canonical_stacks: set[str] | frozenset[str] | None = None,
    governance_root: Path | None = None,
) -> None:
    """Audit the two classification axes across both authority surfaces.

    - an absent or non-canonical value on either surface is WARNING
      `TAXONOMY_AXIS_UNKNOWN` naming the value and both canonical sets;
    - a disagreement between the two surfaces is ERROR `TAXONOMY_MISMATCH`
      (level fixed by ruling 2) naming both values and both keys;
    - an axis present on only one surface is an absent value, not a
      contradiction, so it reports `TAXONOMY_AXIS_UNKNOWN` and no mismatch;
    - a `kind` that disagrees with the value derived from its own axes, and a
      stack-qualified composite still present while both axes are declared, are
      WARNING `TAXONOMY_LEGACY_VALUE`.

    The axes are descriptive: nothing here reads a required file, a CI or
    verification requirement, or an evidence, deployment, or promotion state.
    """
    roles = canonical_roles if canonical_roles is not None else canonical_roles_for(governance_root)
    stacks = canonical_stacks if canonical_stacks is not None else canonical_stacks_for(governance_root)
    declaration_role, declaration_stack, declaration_canonical = _resolve_axes(
        declaration, findings, project_id, canonical_roles=roles, canonical_stacks=stacks, label="declaration")
    registry_role, registry_stack, _ = _resolve_axes(
        entry, findings, project_id, canonical_roles=roles, canonical_stacks=stacks, label="registry entry")

    if declaration_canonical and registry_role is not None and declaration_role != registry_role:
        finding(findings, TAXONOMY_MISMATCH_LEVEL, TAXONOMY_MISMATCH,
                f"role disagreement: registry entry {registry_role!r} against declaration {declaration_role!r}", project_id)
    if declaration_canonical and registry_stack is not None and declaration_stack != registry_stack:
        finding(findings, TAXONOMY_MISMATCH_LEVEL, TAXONOMY_MISMATCH,
                f"stack disagreement: registry entry {registry_stack!r} against declaration {declaration_stack!r}", project_id)

    if not declaration_canonical:
        # Nothing has been derived yet, so the legacy composite is not yet
        # measurable: an absent axis must not also report TAXONOMY_LEGACY_VALUE.
        return
    derived = derive_kind(declaration_role or "", declaration_stack or "")
    kind = declaration.get("kind")
    if isinstance(kind, str) and kind and kind != derived:
        finding(findings, "WARNING", TAXONOMY_LEGACY_VALUE,
                f"kind {kind!r} is a hand-authored composite that disagrees with the derived value {derived!r} "
                f"(derived as stack-role, or the bare role when stack is 'none')", project_id)


def validate_capabilities(capabilities: Any, project_path: Path, findings: list[Finding], project_id: str, supported: set[str] | None = None) -> None:
    """Audit a project's optional capability declarations.

    Declarations describe intent, not implementation. Governance records the
    declared state and the presence/absence of native evidence; it never
    promotes a capability to verified on its own.

    When `supported` is provided, capabilities not listed there are surfaced
    as a WARNING so profile owners can keep the supported set intentional.
    """
    if not isinstance(capabilities, dict):
        finding(findings, "ERROR", "CAPABILITIES_SHAPE", "capabilities must be an object", project_id)
        return
    for name, entry in capabilities.items():
        if not isinstance(name, str) or not PROJECT_ID.fullmatch(name):
            finding(findings, "ERROR", "CAPABILITY_NAME", f"invalid capability name: {name!r}", project_id)
            continue
        if supported is not None and name not in supported:
            finding(findings, "WARNING", "CAPABILITY_UNSUPPORTED", f"capability {name!r} is not declared as supported by the project profile", project_id)
        if not isinstance(entry, dict):
            finding(findings, "ERROR", "CAPABILITY_SHAPE", f"capability {name!r} must be an object", project_id)
            continue
        owner = entry.get("owner")
        if not isinstance(owner, str) or not owner:
            finding(findings, "ERROR", "CAPABILITY_OWNER", f"capability {name!r} requires a non-empty owner", project_id)
        elif owner not in CAPABILITY_OWNERS and not PROJECT_ID.fullmatch(owner):
            finding(findings, "WARNING", "CAPABILITY_OWNER", f"capability {name!r} owner is not a known platform, product, or shared id: {owner!r}", project_id)
        state = entry.get("evidence_state")
        if state not in CAPABILITY_EVIDENCE_STATES:
            finding(findings, "ERROR", "CAPABILITY_STATE", f"capability {name!r} has invalid evidence_state: {state!r}", project_id)
            continue
        ref = entry.get("evidence_ref")
        if ref is not None and not isinstance(ref, str):
            finding(findings, "ERROR", "CAPABILITY_REF", f"capability {name!r} evidence_ref must be a string when present", project_id)
            ref = None
        if state in {"configured", "verified"}:
            if not isinstance(ref, str) or not ref:
                finding(findings, "WARNING", "CAPABILITY_PENDING", f"capability {name!r} is {state} but has no evidence_ref", project_id)
            elif (project_path / ref).is_file():
                continue
            else:
                level = "ERROR" if state == "verified" else "WARNING"
                code = "CAPABILITY_STALE" if state == "verified" else "CAPABILITY_PENDING"
                finding(findings, level, code, f"capability {name!r} evidence_ref missing: {ref}", project_id)
        elif state == "unverified":
            finding(findings, "WARNING", "CAPABILITY_PENDING", f"capability {name!r} is unverified; native evidence required", project_id)
        elif state == "blocked":
            finding(findings, "WARNING", "CAPABILITY_BLOCKED", f"capability {name!r} is declared blocked; verify the non-goal is recorded", project_id)


# ---------------------------------------------------------------------------
# Capability adoption gate
#
# Measurement is a three-way join and all three sides must agree:
#   1. the registry (`capabilities.json`) names exactly one provider,
#   2. the consumer's own manifests carry a pinned coordinate for the package,
#   3. the consumer's `.project.json` records the pairing in `consumes[]`.
# Everything below is read-only. No project command is executed, no package
# manager is invoked, no lockfile or transitive graph is resolved, and no
# network request is made. Nothing here promotes a capability to `verified`.
# ---------------------------------------------------------------------------
