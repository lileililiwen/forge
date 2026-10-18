"""Adoption grants and versioned interface boundary validation."""

from __future__ import annotations

import workspace_check as _facade
globals().update({key: value for key, value in vars(_facade).items() if not key.startswith("__")})

ADOPTION_GRANT_SHAPE = "ADOPTION_GRANT_SHAPE"
ADOPTION_GRANT_SCOPE_VIOLATION = "ADOPTION_GRANT_SCOPE_VIOLATION"
ADOPTION_GRANT_UNSATISFIED = "ADOPTION_GRANT_UNSATISFIED"
ADOPTION_GRANT_STALE = "ADOPTION_GRANT_STALE"
ADOPTION_GRANT_PROVIDER_UNVERIFIED = "ADOPTION_GRANT_PROVIDER_UNVERIFIED"
ADOPTION_GRANT_FIELDS = ("capability", "provider", "package", "actor", "issued_at", "expires_at", "write_set")
ADOPTION_RECORD_FIELDS = ("consumer", "capability", "provider", "package", "version", "replaced", "actor", "grant_expires_at", "rollback_ref", "recorded_at")
ADOPTION_LEDGER_FILE = "adoptions.json"
# Sources a source-level coupling would arrive through. The grant permits
# consuming a published artifact; it never permits a checkout dependency.
_COUPLING_SOURCES = ("Cargo.toml", "package.json", "pyproject.toml", "go.mod", "NuGet.config", "Directory.Build.props")
_PROMOTION_LEDGER_PATTERN = re.compile(r"(?:^|/)deployment/promotions(?:\.sha256|\.json)?$")
_MSBUILD_PROJECT_REFERENCE = re.compile(r"<ProjectReference\b[^>]*>", re.IGNORECASE)
_MSBUILD_PATH_ATTRIBUTE = re.compile(r'\b(?:Include|Path)\s*=\s*"([^"]+)"', re.IGNORECASE)


def adoption_ledger_path(governance_root: Path | None = None) -> Path:
    """Return the append-only adoption ledger path at the governance root."""
    root = Path(governance_root) if governance_root is not None else _default_governance_root()
    return root / "adoption" / ADOPTION_LEDGER_FILE


def _canonical_record_hash(record: dict[str, Any]) -> str:
    """Hash a record body, excluding the `content_hash` field itself."""
    body = {key: value for key, value in record.items() if key != "content_hash"}
    return hashlib.sha256(json.dumps(body, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest()


def load_adoption_ledger(governance_root: Path | None, findings: list[Finding]) -> list[dict[str, Any]]:
    """Load `adoption/adoptions.json` with tamper detection.

    A missing ledger returns an empty list and no finding, matching
    `load_promotion_ledger`: a repository with no grant has nothing to record.
    A record whose `content_hash` does not match its body, or whose
    `prev_hash` does not match the row before it, is refused with
    `ADOPTION_GRANT_UNSATISFIED` — the same posture as a hand-edited
    `deployable: true` (`DEPLOYMENT_PROMOTION`). The rows are still returned so
    the audit can keep reporting against them.
    """
    path = adoption_ledger_path(governance_root)
    if not path.is_file():
        return []
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError, UnicodeDecodeError) as exc:
        finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"adoption ledger cannot be parsed at {path}: {exc}")
        return []
    if not isinstance(payload, dict) or not isinstance(payload.get("records"), list):
        finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"adoption ledger must be an object with a records array: {path}")
        return []
    records = payload["records"]
    previous = ""
    for index, record in enumerate(records):
        if not isinstance(record, dict):
            finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"adoption ledger row {index} must be an object")
            continue
        recorded = record.get("content_hash")
        if not isinstance(recorded, str) or recorded != _canonical_record_hash(record):
            finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"adoption ledger row {index} does not match its content_hash; a hand-edited record is refused")
        if record.get("prev_hash", "") != previous:
            finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"adoption ledger row {index} breaks the append chain; a hand-inserted row is refused")
        previous = recorded if isinstance(recorded, str) else _canonical_record_hash(record)
    return records


def _parse_grant_time(value: Any) -> datetime | None:
    """Parse an ISO 8601 timestamp, tolerating a trailing `Z`."""
    if not isinstance(value, str) or not value.strip():
        return None
    text = value.strip()
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    try:
        parsed = datetime.fromisoformat(text)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        return parsed.replace(tzinfo=timezone.utc)
    return parsed


def _write_set_escapes(project_path: Path, pattern: str) -> bool:
    """Return True when a write_set pattern resolves outside the consumer root.

    `..` and absolute patterns are already rejected as shape errors. This
    catches the remaining escape route: a literal path segment that is a
    symlink pointing outside the repository. The path is never read.
    """
    literal = re.split(r"[*?\[]", pattern, maxsplit=1)[0]
    literal = literal.rsplit("/", 1)[0] if "/" in literal else ""
    if not literal:
        return False
    try:
        root = project_path.resolve(strict=True)
        resolved = (project_path / literal).resolve(strict=False)
    except (OSError, RuntimeError):
        return False
    return not resolved.is_relative_to(root)


def _scan_source_coupling(project_path: Path, provider_path: Path | None = None) -> list[tuple[str, str]]:
    """Return source-level couplings found in the consumer's own manifests.

    A `ProjectReference`, relative path, or `../` dependency on a sibling
    checkout is forbidden under any grant, so this reuses the existing bounded
    sibling scan and adds the MSBuild `ProjectReference` case, which is how a
    .NET consumer would couple to a provider source tree. Nothing outside the
    consumer root is ever read.
    """
    found: list[tuple[str, str]] = list(_bounded_static_scan_for_siblings(project_path))
    candidates = list(_COUPLING_SOURCES)
    candidates.extend(sorted(item.name for item in project_path.glob("*.csproj")))
    for name in candidates:
        path = project_path / name
        if not path.is_file():
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for element in _MSBUILD_PROJECT_REFERENCE.findall(text):
            for attribute in _MSBUILD_PATH_ATTRIBUTE.findall(element):
                if _is_absolute_path(attribute) or _has_path_escape(attribute):
                    found.append((name, attribute))
    if provider_path is not None:
        provider_key = provider_path.name.lower()
        for relative, target in found:
            if provider_key in target.lower() or provider_key in relative.lower():
                return [(relative, target)]
    return found


def _declared_consumes(declaration: dict[str, Any], capability_id: str) -> list[Any]:
    capabilities = declaration.get("capabilities")
    if not isinstance(capabilities, dict):
        return []
    entry = capabilities.get(capability_id)
    if not isinstance(entry, dict) or not isinstance(entry.get("consumes"), list):
        return []
    return entry["consumes"]


def validate_adoption_grant(
    declaration: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    capability_registry: CapabilityRegistry | None = None,
    provider_paths: dict[str, Path] | None = None,
    ledger: list[dict[str, Any]] | None = None,
    provider_verified: Any = None,
    now: datetime | None = None,
) -> None:
    """Audit a project's optional `adoption_grant` declaration.

    The grant is the only permission in this repository that lets an agent
    touch a capability owned by a sibling. It is deliberately narrow: one
    consumer, one capability, one provider, one package, one actor, one named
    write set, one mandatory expiry, single use. The provider repository is
    read-only at all times, and the check is read-only like every other audit
    check — it proves an authorization existed, was in scope, and produced a
    recorded, unmodified outcome. It never runs a build or a test.
    """
    grant = declaration.get("adoption_grant")
    if grant is None:
        return
    if not isinstance(grant, dict):
        finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, "adoption_grant must be an object", project_id)
        return
    for field in ADOPTION_GRANT_FIELDS:
        if field not in grant:
            finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, f"adoption_grant.{field} is required", project_id)
    if any(field not in grant for field in ADOPTION_GRANT_FIELDS):
        return
    write_set = grant["write_set"]
    if not isinstance(write_set, list) or not write_set:
        finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, "adoption_grant.write_set must be a non-empty array", project_id)
        return
    for index, pattern in enumerate(write_set):
        if not isinstance(pattern, str) or not pattern:
            finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, f"adoption_grant.write_set[{index}] must be a non-empty string", project_id)
            return
        if _is_absolute_path(pattern) or _has_path_escape(pattern):
            finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, f"adoption_grant.write_set[{index}] is absolute or escapes the project root: {pattern}", project_id)
            return
    status = grant.get("status", "active")
    if status not in {"active", "revoked"}:
        finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, f"adoption_grant.status is invalid: {status!r}", project_id)
        return

    capability_id = str(grant["capability"])
    provider_id = str(grant["provider"])
    package = str(grant["package"])
    paths = provider_paths or {}

    # 1. Identity. A grant may not invent a provider, and it may not be the
    #    thing that creates the capability binding it consumes.
    if provider_id not in paths:
        finding(findings, "WARNING", ADOPTION_GRANT_PROVIDER_UNVERIFIED, f"adoption_grant names provider {provider_id!r}, which is not a registered project id; the grant is refused and no write under it is validated", project_id)
        return
    if capability_registry is None or capability_id not in capability_registry.capabilities:
        finding(findings, "WARNING", ADOPTION_GRANT_PROVIDER_UNVERIFIED, f"adoption_grant names capability {capability_id!r}, which binds to no provider in the capability registry; the grant is refused and no write under it is validated", project_id)
        return
    bound_provider = _resolved_provider(capability_registry.capabilities[capability_id])
    if bound_provider != provider_id:
        finding(findings, "WARNING", ADOPTION_GRANT_PROVIDER_UNVERIFIED, f"adoption_grant names capability {capability_id!r} with provider {provider_id!r}, but the registry binds it to {bound_provider!r}; the grant is refused and no write under it is validated", project_id)
        return
    provider_path = paths.get(provider_id)

    # 2. Window. Expiry is mandatory precisely so a grant cannot become
    #    standing authorization, and a repoint is not retroactively legal.
    moment = now or datetime.now(timezone.utc)
    expires_at = _parse_grant_time(grant["expires_at"])
    issued_at = _parse_grant_time(grant["issued_at"])
    if expires_at is None or issued_at is None:
        finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, "adoption_grant.issued_at and adoption_grant.expires_at must be ISO 8601 timestamps", project_id)
        return
    if expires_at <= moment:
        finding(findings, "WARNING", ADOPTION_GRANT_STALE, f"adoption_grant expired at {grant['expires_at']}; the repoint it authorized is not retroactively lawful", project_id)
    if issued_at > moment:
        finding(findings, "WARNING", ADOPTION_GRANT_STALE, f"adoption_grant is not active before {grant['issued_at']}", project_id)
    if status == "revoked":
        finding(findings, "WARNING", ADOPTION_GRANT_STALE, "adoption_grant is revoked; it authorizes no repoint", project_id)

    # 3. Provider. There is nothing lawful to adopt from an unverified
    #    provider, and a grant never substitutes for the provider's evidence.
    if not (callable(provider_verified) and provider_verified(provider_id, capability_id)):
        finding(findings, "WARNING", ADOPTION_GRANT_PROVIDER_UNVERIFIED, f"provider {provider_id!r} does not declare capability {capability_id!r} as verified with a resolving evidence_ref; the grant has nothing lawful to adopt from", project_id)

    # 4. Write set. Every permitted write resolves inside the consumer root.
    for index, pattern in enumerate(write_set):
        normalised = pattern.replace("\\", "/")
        if _PROMOTION_LEDGER_PATTERN.search(normalised):
            finding(findings, "ERROR", ADOPTION_GRANT_SCOPE_VIOLATION, f"write_set[{index}] names the promotion ledger ({pattern}); adoption is not promotion and is never a path to deployable", project_id)
        if provider_path is not None and provider_path.name.lower() in normalised.lower():
            finding(findings, "ERROR", ADOPTION_GRANT_SCOPE_VIOLATION, f"write_set[{index}] names the provider repository ({pattern}); the provider root is read-only under every grant", project_id)
        if _write_set_escapes(project_path, pattern):
            finding(findings, "ERROR", ADOPTION_GRANT_SCOPE_VIOLATION, f"write_set[{index}] resolves outside the consumer root and was not read: {pattern}", project_id)

    # 5. Forbidden content in the consumer's own files. These are refused
    #    regardless of the resulting code state, because the violation is the
    #    finding.
    for source, target in _scan_source_coupling(project_path, provider_path):
        finding(findings, "ERROR", ADOPTION_GRANT_SCOPE_VIOLATION, f"{source} carries a source-level coupling to a sibling checkout ({target}); a ProjectReference, relative path, or ../ dependency remains forbidden regardless of any grant", project_id)
    deployment = declaration.get("deployment")
    if isinstance(deployment, dict) and deployment.get("deployable"):
        finding(findings, "ERROR", ADOPTION_GRANT_SCOPE_VIOLATION, "adoption_grant is active but deployment.deployable is true; adoption is not promotion and is never a path to deployable", project_id)
    granted_version = grant.get("version")
    if isinstance(granted_version, str) and granted_version:
        pinned = read_manifest_dependencies(project_path).pinned.get(_normalise_package_name(package), [])
        for edge in pinned:
            actual = edge.split(" ", 1)[0]
            if actual != granted_version:
                finding(findings, "ERROR", ADOPTION_GRANT_SCOPE_VIOLATION, f"consumer pins {package} {actual} but the grant authorizes {granted_version}; a version change to provider {provider_id!r} is a provider change and needs that change's own authorization", project_id)

    # 6. The record. A granted repoint must leave an immutable adoption row,
    #    and one permission is sized for one change.
    records = ledger or []
    mine = [
        item
        for item in records
        if isinstance(item, dict)
        and item.get("consumer") == project_id
        and item.get("capability") == capability_id
        and item.get("provider") == provider_id
    ]
    adoptions = [item for item in mine if item.get("kind", "adoption") == "adoption"]
    for item in mine:
        missing = [field for field in ADOPTION_RECORD_FIELDS if field not in item]
        if missing:
            finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"adoption record for {project_id}/{capability_id} is missing {', '.join(missing)}; the adoption is not treated as authorized", project_id)
    if len(adoptions) > 1:
        finding(findings, "WARNING", ADOPTION_GRANT_STALE, f"{len(adoptions)} adoption records exist for {project_id}/{capability_id} under one grant; the permission was sized for one change", project_id)
    consumed = _declared_consumes(declaration, capability_id)
    if consumed and not mine:
        finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"{project_id} declares capabilities[{capability_id!r}].consumes[] but no adoption record exists in adoption/{ADOPTION_LEDGER_FILE}; the adoption is not treated as authorized", project_id)


# ---------------------------------------------------------------------------
# Declared interfaces and boundary findings
#
# A project MAY declare the boundaries it provides and consumes in an
# optional `interfaces[]` array. Everything below is read-only: the audit
# never writes a declaration, never executes a project command, and never
# reads a byte outside the workspace root it was given. Three properties are
# deliberate and each one is a rule, not an implementation detail:
#
# 1. A declaration is validated as a whole before anything is asserted about
#    it, so a malformed declaration never yields a partial read.
# 2. A project that declares no interfaces reports nothing. Governance
#    charges a boundary a project claimed; it never invents one, so a
#    duplicate that nobody declared stays invisible rather than guessed at.
# 3. An edge counts as declared only when both sides declare it, so a
#    consumption the provider does not provide is a finding rather than
#    silent adoption.
#
# Two of the declaration rules are asserted as findings rather than as
# malformation, because the canonical specification names a distinct code for
# each and a malformed declaration yields no findings at all: a contract
# without a version (`INTERFACE_CONTRACT_UNVERSIONED`) and a `sibling` edge
# that is not pinned (`INTERFACE_SIBLING_PATH_UNPINNED`). Every other rule in
# `schemas/project.schema.json` is in the malformed set below.
# ---------------------------------------------------------------------------


def interface_escalation_dates(governance_root: Path | None = None) -> dict[str, str]:
    """`{finding code: ISO effective date}` recorded in the decisions register.

    A ramp is an owner decision, not a property of the checker. The register
    entry lives in `decisions.json` and carries an `escalates` object naming
    the finding codes and the date the WARNING becomes blocking; the register
    loader tolerates additive entry-level keys, which is what lets the ramp be
    machine-readable without a second list to keep in sync.

    Unreadable or malformed input yields an empty mapping, so the finding
    stays a WARNING: nothing may become blocking on the strength of a record
    that could not be read. Read-only and never raises.
    """
    override = os.environ.get("WORKSPACE_DECISIONS_PATH")
    root = Path(governance_root) if governance_root is not None else _default_governance_root()
    path = Path(override) if override else root / "decisions.json"
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return {}
    if not isinstance(document, dict):
        return {}
    entries = document.get("decisions")
    if not isinstance(entries, list):
        return {}
    dates: dict[str, str] = {}
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        ramp = entry.get(INTERFACE_ESCALATION_FIELD)
        if not isinstance(ramp, dict):
            continue
        codes = ramp.get("finding_codes")
        effective = ramp.get("effective")
        if not isinstance(codes, list) or not isinstance(effective, str):
            continue
        try:
            date.fromisoformat(effective)
        except ValueError:
            continue
        for code in codes:
            if isinstance(code, str) and code:
                dates[code] = effective
    return dates


def interface_finding_level(
    code: str,
    escalation: dict[str, str] | None = None,
    today: date | None = None,
) -> str:
    """The level an interface finding is reported at: WARNING, or blocking after the ramp.

    `INTERFACE_SIBLING_PATH_UNPINNED` and `INTERFACE_VENDORED_WITHOUT_DIGEST`
    begin as WARNINGs and become blocking once the date recorded in
    `decisions.json` has passed. The escalation is never triggered by an
    environment variable, by a project-controlled switch, or by elapsed time
    alone: it is an owner decision, it is dated, and it stays visible and
    revocable in the register. `FAIL` in the finding vocabulary is the
    blocking level `ERROR`, which is what the gate turns into a failed run.
    """
    effective = (escalation or {}).get(code)
    if not effective:
        return "WARNING"
    try:
        boundary = date.fromisoformat(effective)
    except ValueError:
        return "WARNING"
    return "ERROR" if (today or date.today()) >= boundary else "WARNING"


def build_interface_index(
    workspace_root: Path, projects: list[dict[str, Any]]
) -> dict[tuple[str, str], dict[str, Any]]:
    """Index every declared `provides` interface by `(project id, interface id)`.

    One bounded read per declaration, read-only, and no directory walk: the
    provider side of an edge is a declaration, so the index is the set of
    declarations the fleet actually makes. A declaration that cannot be read,
    or an `interfaces` value that is not an array, contributes nothing — and
    nothing is exactly what a consumer's missing provision has to look like
    for the one-sided edge to be reported rather than assumed.
    """
    index: dict[tuple[str, str], dict[str, Any]] = {}
    for entry in projects:
        if not isinstance(entry, dict):
            continue
        project_id = entry.get("id")
        path = entry.get("path")
        if not isinstance(project_id, str) or not isinstance(path, str):
            continue
        try:
            declaration = json.loads(
                (workspace_root / path / ".project.json").read_text(encoding="utf-8")
            )
        except (OSError, UnicodeDecodeError, json.JSONDecodeError):
            continue
        if not isinstance(declaration, dict):
            continue
        interfaces = declaration.get("interfaces")
        if not isinstance(interfaces, list):
            continue
        for item in interfaces:
            if not isinstance(item, dict):
                continue
            interface_id = item.get("id")
            if item.get("provides") is not True or not isinstance(interface_id, str):
                continue
            if item.get("mechanism") not in INTERFACE_MECHANISMS:
                continue
            index.setdefault((project_id, interface_id), item)
    return index


def _interface_shape_problems(interfaces: Any) -> list[str]:
    """Every declaration-shape problem in an `interfaces` value, in order.

    This is the malformed set: the required keys, the id grammar and
    uniqueness, the exclusive side flags, the provider a consumption names,
    the entry and digest and evidence types, the `observe` path rules, the
    digest a `consume`/`observe` edge must carry, and the contract every
    mechanism except `sibling` must name. An unknown mechanism is
    deliberately absent from this set: it has its own finding and the entry is
    otherwise ignored, so a mechanism this checker does not know is not
    reported as a malformed document.
    """
    problems: list[str] = []
    if not isinstance(interfaces, list):
        return ["`interfaces` must be an array of interface declarations"]
    seen: dict[str, int] = {}
    for index, item in enumerate(interfaces):
        where = f"interfaces[{index}]"
        if not isinstance(item, dict):
            problems.append(f"{where} must be an object")
            continue
        interface_id = item.get("id")
        if isinstance(interface_id, str) and PROJECT_ID.fullmatch(interface_id):
            where = f"interface {interface_id!r}"
            if interface_id in seen:
                problems.append(
                    f"{where} repeats the id already declared at interfaces[{seen[interface_id]}]; "
                    "interface ids are unique within a project"
                )
            else:
                seen[interface_id] = index
        else:
            problems.append(f"{where}.id must be a kebab-case identifier")
        unknown = sorted(key for key in item if key not in INTERFACE_ENTRY_KEYS)
        if unknown:
            problems.append(f"{where} has unknown key(s): {', '.join(unknown)}")
        provides = item.get("provides")
        consumes = item.get("consumes")
        for key, value in (("provides", provides), ("consumes", consumes)):
            if value is not None and not isinstance(value, bool):
                problems.append(f"{where}.{key} must be a boolean")
        if (provides is True) == (consumes is True):
            problems.append(f"{where} must declare exactly one of provides/consumes as true")
        mechanism = item.get("mechanism")
        if not isinstance(mechanism, str) or not mechanism:
            problems.append(f"{where}.mechanism is required")
        entry = item.get("entry")
        if not isinstance(entry, str) or not entry:
            problems.append(f"{where}.entry is required")
        elif _is_absolute_path(entry):
            problems.append(f"{where}.entry must not be an absolute path")
        if consumes is True:
            provider = item.get("provider")
            if not isinstance(provider, str) or not PROJECT_ID.fullmatch(provider):
                problems.append(
                    f"{where}.provider must be the kebab-case id of the project providing it"
                )
        contract = item.get("contract")
        if contract is not None and (
            not isinstance(contract, str) or not INTERFACE_CONTRACT_NAME.fullmatch(contract)
        ):
            problems.append(f"{where}.contract must be a \"<name>/<version>\" identifier")
        digest = item.get("digest")
        if digest is not None and (
            not isinstance(digest, str) or not INTERFACE_DIGEST.fullmatch(digest)
        ):
            problems.append(f"{where}.digest must be a sha256 digest of the form sha256:<64 hex>")
        evidence_ref = item.get("evidence_ref")
        if evidence_ref is not None and (
            not isinstance(evidence_ref, str)
            or not evidence_ref
            or _is_absolute_path(evidence_ref)
            or _has_path_escape(evidence_ref)
        ):
            problems.append(f"{where}.evidence_ref must be a repository-relative path")
        required = item.get("required")
        if required is not None and not isinstance(required, bool):
            problems.append(f"{where}.required must be a boolean")
        if isinstance(mechanism, str) and mechanism in INTERFACE_MECHANISMS:
            if mechanism != "sibling" and contract is None:
                problems.append(f"{where}.contract is required for mechanism {mechanism!r}")
            if mechanism in ("consume", "observe") and digest is None:
                problems.append(f"{where}.digest is required for mechanism {mechanism!r}")
            if mechanism == "observe" and isinstance(entry, str) and entry and _has_path_escape(entry):
                problems.append(
                    f"{where}.entry must be a repository-relative document path; "
                    "'..' is refused and never normalized"
                )
    return problems


def _retained_copy_digest(project_path: Path, entry: str) -> str | None:
    """The sha256 of the bytes an `entry` names inside the project, or None.

    `entry` is a retained copy only when it names a file the project actually
    holds. A published coordinate (`@platform/design-tokens`) names no file
    here, and the audit fetches nothing: no network, no package manager, no
    lockfile. `None` therefore means "not a retained copy the audit can
    digest", never "a copy that matched".
    """
    if _is_absolute_path(entry) or _has_path_escape(entry):
        return None
    try:
        resolved = (project_path / entry).resolve(strict=True)
        resolved.relative_to(project_path.resolve(strict=True))
    except (OSError, RuntimeError, ValueError):
        return None
    if not resolved.is_file():
        return None
    try:
        return "sha256:" + hashlib.sha256(resolved.read_bytes()).hexdigest()
    except OSError:
        return None


def _sibling_edge_pinning_problem(item: dict[str, Any]) -> str:
    """Why a `sibling` edge is not pinned, or `""` when it is.

    Both halves are required: the consuming project declares the provider
    `required: true`, and the edge carries a digest of the invocation it
    wrote. The digest is over the `entry` string, which is the invocation —
    the thing a consumer can actually pin, and the same bytes the generator
    hashes when it writes the entry.
    """
    missing: list[str] = []
    if item.get("required") is not True:
        missing.append("`required: true` naming the provider as part of the baseline")
    digest = item.get("digest")
    entry = item.get("entry")
    if not isinstance(digest, str):
        missing.append("a `digest`")
    elif not isinstance(entry, str) or digest != "sha256:" + hashlib.sha256(
        entry.encode("utf-8")
    ).hexdigest():
        missing.append("a `digest` that matches the declared invocation")
    if not missing:
        return ""
    return "is missing " + " and ".join(missing)


def validate_interfaces(
    declaration: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    index: dict[tuple[str, str], dict[str, Any]] | None = None,
    escalation: dict[str, str] | None = None,
) -> None:
    """Validate this project's `interfaces[]` and charge its unpinned edges.

    Read-only. An absent `interfaces` key is valid and silent, and a
    declaration that fails the shape rules above yields
    `INTERFACE_DECLARATION_INVALID` and no interface assertions at all — never
    a partial read. The declared rules are then asserted per entry: the
    closed mechanism set, the versioned contract, the pinning rule for
    `sibling`, the cross-project pairing of a `consumes` edge against the
    provider's own `provides` entry, the consumer's own evidence for a
    consumption, and the digest of a retained byte copy.
    """
    if "interfaces" not in declaration:
        return
    interfaces = declaration.get("interfaces")
    problems = _interface_shape_problems(interfaces)
    if problems:
        for problem in problems:
            finding(
                findings,
                "ERROR",
                INTERFACE_DECLARATION_INVALID,
                f"invalid interface declaration: {problem}",
                project_id,
            )
        return

    for item in interfaces:
        interface_id = item.get("id")
        mechanism = item.get("mechanism")
        if mechanism not in INTERFACE_MECHANISMS:
            finding(
                findings,
                "ERROR",
                INTERFACE_MECHANISM_UNKNOWN,
                f"interface {interface_id!r} declares mechanism {mechanism!r}; "
                f"accepted mechanisms: {', '.join(INTERFACE_MECHANISMS)}",
                project_id,
            )
            continue
        if mechanism == "sibling":
            unpinned = _sibling_edge_pinning_problem(item)
            if unpinned:
                finding(
                    findings,
                    interface_finding_level(INTERFACE_SIBLING_PATH_UNPINNED, escalation),
                    INTERFACE_SIBLING_PATH_UNPINNED,
                    f"interface {interface_id!r} reaches provider {item.get('provider')!r} by a "
                    f"repository-relative path and {unpinned}; a sibling edge is lawful only when "
                    f"the provider is declared `required: true` and the edge is digest-pinned. "
                    f"The two lawful alternatives: {SIBLING_LAWFUL_ALTERNATIVES}",
                    project_id,
                )
        else:
            contract = item.get("contract")
            if not isinstance(contract, str) or not INTERFACE_CONTRACT.fullmatch(contract):
                finding(
                    findings,
                    "ERROR",
                    INTERFACE_CONTRACT_UNVERSIONED,
                    f"interface {interface_id!r} names contract {contract!r} without a version; "
                    "write \"<name>/<major>.<minor>.<patch>\"",
                    project_id,
                )
                continue
        if item.get("consumes") is not True:
            continue
        provider = item.get("provider")
        if (index or {}).get((provider, interface_id)) is None:
            finding(
                findings,
                "WARNING",
                INTERFACE_PROVIDER_UNDECLARED,
                f"interface {interface_id!r} consumes from provider {provider!r}, which declares "
                "no matching `provides` entry for it; an edge counts as declared only when both "
                "sides declare it",
                project_id,
            )
        evidence_ref = item.get("evidence_ref")
        if not isinstance(evidence_ref, str) or not _standalone_evidence_resolves(
            project_path, evidence_ref
        ):
            finding(
                findings,
                "WARNING",
                INTERFACE_CONSUMED_UNUSED,
                f"interface {interface_id!r} declares a consumption with no `evidence_ref` "
                "resolving to this project's own use of the edge; record the repository-relative "
                "path where the project uses it",
                project_id,
            )
        entry = item.get("entry")
        if mechanism != "consume" or not isinstance(entry, str):
            continue
        retained = _retained_copy_digest(project_path, entry)
        if retained is not None and retained != item.get("digest"):
            finding(
                findings,
                interface_finding_level(INTERFACE_VENDORED_WITHOUT_DIGEST, escalation),
                INTERFACE_VENDORED_WITHOUT_DIGEST,
                f"interface {interface_id!r} keeps a byte copy at {entry!r} of provider "
                f"{provider!r} whose declared digest does not pin those bytes; declare the "
                f"sha256 of the retained copy, which is {retained}, or consume the published "
                "artifact by version",
                project_id,
            )


def report_unverified_exempt_capabilities(
    project_id: str,
    findings: list[Finding],
    registry: "CapabilityRegistry | None",
    provider_verified: Any = None,
) -> None:
    """WARNING `CAPABILITY_PROVIDER_UNVERIFIED` on the provider that owns an exemption.

    A registry entry is exempt because its provider declares no verified
    capability record, so without a reporter the exemption silently absorbs
    every consumer and nobody owns the next action. The finding is attributed
    to the provider, which is the one repository that can end it by recording
    verified evidence, and the resolved provider is named in the message.

    Read-only in all three directions the specification names: no posture is
    changed, no consumer is charged for an unfrozen capability, and the
    existing exemption semantics are untouched. A capability whose provider
    declares verified evidence stops being reported here, and a capability
    that binds to no provider is left to `CAPABILITY_ADOPTION_UNRESOLVED`.
    """
    if registry is None:
        return
    for capability_id in sorted(registry.capabilities):
        entry = registry.capabilities[capability_id]
        if not isinstance(entry, dict) or entry.get("posture") != "exempt":
            continue
        provider = _resolved_provider(entry)
        if provider is None or provider != project_id:
            continue
        if callable(provider_verified) and provider_verified(provider, capability_id):
            continue
        reason = str(entry.get("reason") or "no reason recorded")
        finding(
            findings,
            "WARNING",
            CAPABILITY_PROVIDER_UNVERIFIED,
            f"capability {capability_id!r} is exempt ({reason}) and provider {provider!r} "
            "declares no `verified` capability record for it; the exemption lifts when the "
            "provider records verified evidence with a resolving evidence_ref",
            project_id,
        )
