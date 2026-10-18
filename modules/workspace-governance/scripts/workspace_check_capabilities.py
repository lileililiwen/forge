"""Capability declarations, adoption measurement, and posture rules."""

from __future__ import annotations

import workspace_check as _facade
globals().update({key: value for key, value in vars(_facade).items() if not key.startswith("__")})

@dataclass
class CapabilityRegistry:
    """The loaded capability registry. Every capability id binds one provider."""

    root: Path
    capabilities: dict[str, Any]
    graduations: list[dict[str, Any]]
    graduation_audits: int


def capability_registry_path(governance_root: Path | None = None) -> Path:
    """Return the capability registry path.

    `WORKSPACE_CAPABILITIES_PATH` mirrors the `WORKSPACE_DECISIONS_PATH` hook
    so tests can isolate the audit from the governance root's real registry.
    """
    override = os.environ.get("WORKSPACE_CAPABILITIES_PATH")
    if override:
        return Path(override)
    root = Path(governance_root) if governance_root is not None else _default_governance_root()
    return root / "capabilities.json"


def load_capability_registry(governance_root: Path | None, findings: list[Finding]) -> CapabilityRegistry | None:
    """Load `capabilities.json`, failing closed on a missing or empty registry.

    There is deliberately no empty-registry fallback: a check that degrades to
    "nothing to enforce" would silently restore the invisibility this change
    exists to remove. A missing, unparseable, or empty registry is one
    governance ERROR against the registry itself, not one error per project.
    """
    path = capability_registry_path(governance_root)
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, f"capability registry cannot be read at {path}: {exc}")
        return None
    except json.JSONDecodeError as exc:
        finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, f"capability registry is not valid JSON: {exc}")
        return None
    if not isinstance(payload, dict):
        finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, "capability registry must be a JSON object")
        return None
    capabilities = payload.get("capabilities")
    if not isinstance(capabilities, dict) or not capabilities:
        finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, "capability registry declares no capabilities; refusing to enforce nothing")
        return None
    graduations = payload.get("graduations")
    if not isinstance(graduations, list):
        graduations = []
    audits = payload.get("graduation_audits", DEFAULT_GRADUATION_AUDITS)
    if not isinstance(audits, int) or isinstance(audits, bool) or audits < 1:
        audits = DEFAULT_GRADUATION_AUDITS
    for name, entry in capabilities.items():
        if not isinstance(name, str) or not PROJECT_ID.fullmatch(name):
            finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, f"invalid capability id in registry: {name!r}")
            return None
        if not isinstance(entry, dict):
            finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, f"registry entry for {name!r} must be an object")
            return None
        posture = entry.get("posture")
        if posture not in CAPABILITY_POSTURES:
            finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, f"capability {name!r} has invalid posture: {posture!r}")
            return None
        if posture == "exempt" and not str(entry.get("reason") or "").strip():
            finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, f"capability {name!r} is exempt and requires a recorded reason")
            return None
    return CapabilityRegistry(path.parent, dict(capabilities), graduations, audits)


def _resolved_provider(entry: dict[str, Any]) -> str | None:
    """Return the single provider a capability binds to, or None when ambiguous.

    A capability id that is bound to more than one provider, or to none, is not
    a consumer's fault to fix: it is reported as
    `CAPABILITY_ADOPTION_UNRESOLVED` and never escalates.
    """
    provider = entry.get("provider")
    if isinstance(provider, str) and provider.strip():
        return provider.strip()
    if isinstance(provider, list):
        unique = {item.strip() for item in provider if isinstance(item, str) and item.strip()}
        if len(unique) == 1:
            return unique.pop()
    return None


def _graduation_for(capability_id: str, graduations: list[dict[str, Any]]) -> dict[str, Any] | None:
    """Return the latest appended graduation entry for a capability, if any."""
    matches = [
        item
        for item in graduations
        if isinstance(item, dict) and item.get("capability") == capability_id and item.get("to") == "required"
    ]
    return matches[-1] if matches else None


def effective_capability_posture(
    capability_id: str,
    entry: dict[str, Any],
    registry: CapabilityRegistry,
    provider_verified: bool,
) -> tuple[str, str]:
    """Resolve the posture an adoption finding is reported at.

    Returns `(posture, reason)`. `posture` is one of `exempt`, `expected`, or
    `required`.

    A capability may only be *reported* as `required` when every one of the
    three graduation conditions holds:

    1. the provider's capability is `verified` with a resolving evidence ref;
    2. a recorded consecutive-audit count has reached `graduation_audits`;
    3. a graduation entry was appended to the registry ledger.

    Wall-clock time is never a substitute. A capability whose posture says
    `required` without a valid graduation entry is *demoted back* to
    `expected` and no error is emitted for the entry itself, so lowering a
    counter or editing a date cannot manufacture a fleet error.
    """
    declared = entry.get("posture")
    if declared == "exempt":
        return "exempt", str(entry.get("reason") or "exempt")
    if declared == "expected":
        return "expected", "admitted as a warning"
    if not provider_verified:
        return "expected", "posture required but the provider capability is not verified"
    graduation = _graduation_for(capability_id, registry.graduations)
    if graduation is None:
        return "expected", "posture required but no graduation entry is recorded"
    if graduation.get("from") != "expected" or graduation.get("to") != "required":
        return "expected", "graduation entry does not record an expected -> required transition"
    recorded = graduation.get("audit_count")
    if not isinstance(recorded, int) or isinstance(recorded, bool) or recorded < registry.graduation_audits:
        return "expected", f"graduation entry records {recorded!r} consecutive audits, below graduation_audits={registry.graduation_audits}"
    evidence = graduation.get("provider_evidence")
    if not isinstance(evidence, dict) or not str(evidence.get("evidence_ref") or "").strip():
        return "expected", "graduation entry records no provider evidence"
    return "required", "graduated on a recorded transition"


# --- bounded, read-only manifest reading -----------------------------------
#
# Only the dependency stanza of a bounded list of manifest names is parsed.
# Lockfiles, transitive graphs, and registry queries are never read: resolving
# them needs a network or a package manager, both of which the audit is
# forbidden to use.

_MANIFEST_SCAN_DEPTH = 3
_MANIFEST_SCAN_FILE_BUDGET = 200
_MANIFEST_SKIP_DIRS = frozenset(
    {".git", "bin", "obj", "node_modules", "target", "vendor", "dist", "build", ".venv", "venv", "packages"}
)
_MANIFEST_EXACT_NAMES = ("Directory.Packages.props", "package.json", "Cargo.toml", "pyproject.toml")
_MANIFEST_PREFIXES = ("requirements",)

_MSBuild_PACKAGE_ELEMENT = re.compile(r"<Package(?:Version|Reference)\b[^>]*>", re.IGNORECASE)
_MSBuild_INCLUDE = re.compile(r'\bInclude\s*=\s*"([^"]+)"', re.IGNORECASE)
_MSBuild_VERSION = re.compile(r'\bVersion\s*=\s*"([^"]+)"', re.IGNORECASE)
_TOML_SECTION = re.compile(r"^\s*\[([^\]]+)\]\s*$")
_TOML_DEP_LINE = re.compile(r'^([A-Za-z0-9_.\-]+)\s*=\s*(.+?)\s*$')
_CARGO_DEP_STANZAS = frozenset({"dependencies", "dev-dependencies", "build-dependencies"})
_PYPROJECT_DEP_STANZAS = frozenset({"project.dependencies", "tool.poetry.dependencies", "tool.poetry.dev-dependencies", "project.optional-dependencies"})
_PYPROJECT_ARRAY = re.compile(r"^\s*dependencies\s*=\s*\[(.*)\]\s*$", re.IGNORECASE)
_PYPROJECT_STRING = re.compile(r"^([A-Za-z0-9_.\-]+)\s*(.*)$")


def _split_requirement(item: str) -> tuple[str, str | None]:
    """Split a PEP 508 requirement into a name and an exact version.

    A name with no version, or with a range operator, is not a pinned edge.
    """
    match = _PYPROJECT_STRING.match(item.strip())
    if not match:
        return "", None
    name, specifier = match.group(1), (match.group(2) or "").strip()
    exact = re.match(r"^==\s*([^\s,;]+)$", specifier)
    return name, (exact.group(1) if exact else None)
_REQUIREMENT_LINE = re.compile(r"^([A-Za-z0-9_.\-]+)\s*(\[[^\]]*\])?\s*(==|>=|<=|~=|!=|>|<)?\s*([^\s;#]+)?")


@dataclass
class ManifestRead:
    """The bounded result of reading a project's own dependency manifests."""

    pinned: dict[str, list[str]]
    sources: list[str]
    unreadable: list[str]


def _normalise_package_name(name: str) -> str:
    return name.strip().lower()


def _is_pinned_version(version: str | None) -> bool:
    """A coordinate counts as pinned when it names an exact version.

    A floating range (`*`, `latest`, a `^`/`~`/`>=` constraint, or a missing
    version) is not proof of a restored artifact, so it is not an edge.
    """
    if not version:
        return False
    value = version.strip().strip('"').strip("'")
    if not value or value in {"*", "latest"}:
        return False
    if value[0] in {"^", "~", ">", "<", "="}:
        return False
    return True


def _parse_msbuild_packages(text: str) -> list[tuple[str, str | None]]:
    found: list[tuple[str, str | None]] = []
    for element in _MSBuild_PACKAGE_ELEMENT.findall(text):
        include = _MSBuild_INCLUDE.search(element)
        if not include:
            continue
        version = _MSBuild_VERSION.search(element)
        found.append((include.group(1), version.group(1) if version else None))
    return found


def _parse_package_json_packages(text: str) -> list[tuple[str, str | None]]:
    try:
        payload = json.loads(text)
    except json.JSONDecodeError as exc:
        raise ValueError(f"package.json is not valid JSON: {exc}") from exc
    found: list[tuple[str, str | None]] = []
    if not isinstance(payload, dict):
        return found
    for stanza in ("dependencies", "devDependencies", "peerDependencies", "optionalDependencies"):
        entries = payload.get(stanza)
        if not isinstance(entries, dict):
            continue
        for name, version in entries.items():
            if isinstance(name, str) and isinstance(version, str):
                found.append((name, version))
    return found


def _strip_toml_comment(line: str) -> str:
    quoted = False
    for index, char in enumerate(line):
        if char == '"':
            quoted = not quoted
        elif char == "#" and not quoted:
            return line[:index]
    return line


def _parse_toml_packages(text: str, stanzas: frozenset[str]) -> list[tuple[str, str | None]]:
    """Parse `[dependencies]`-style stanzas only.

    Nested tables (`tool.poetry.dependencies`) are matched by their dotted
    path so the reader never has to understand TOML beyond table headers and
    simple key/value pairs. A multi-line PEP 621 array is consumed one line at
    a time and yields the same `(name, version)` pairs.
    """
    found: list[tuple[str, str | None]] = []
    current: str | None = None
    in_array = False
    for raw in text.splitlines():
        line = _strip_toml_comment(raw).strip()
        if not line:
            continue
        section = _TOML_SECTION.match(line)
        if section:
            current = section.group(1).strip().strip('"')
            in_array = False
            continue
        if current is None or current not in stanzas:
            continue
        if in_array:
            in_array = "]" not in line
            for item in re.findall(r"[\"']([^\"']+)[\"']", line):
                name, version = _split_requirement(item)
                if name:
                    found.append((name, version))
            continue
        if "=" not in line:
            continue
        key, _, value = line.partition("=")
        key = key.strip().strip('"')
        value = value.strip()
        if value.startswith("["):
            in_array = "]" not in value
            for item in re.findall(r"[\"']([^\"']+)[\"']", value):
                name, version = _split_requirement(item)
                if name:
                    found.append((name, version))
            continue
        found.append((key, _toml_version(value)))
    return found


def _toml_version(value: str) -> str | None:
    """Return the exact version a TOML value pins, or None when it floats."""
    match = re.match(r'^\{\s*version\s*=\s*"([^"]+)"', value)
    if match:
        return match.group(1)
    plain = value.strip().strip('"').strip("'")
    if not plain or plain.startswith("{") or plain.startswith("["):
        return None
    return plain


def _parse_pyproject_packages(text: str) -> list[tuple[str, str | None]]:
    found = _parse_toml_packages(text, _PYPROJECT_DEP_STANZAS)
    project = _toml_tables(text).get("project")
    if isinstance(project, dict):
        entries = project.get("dependencies")
        if isinstance(entries, list):
            for item in entries:
                if not isinstance(item, str):
                    continue
                name, version = _split_requirement(item)
                if name:
                    found.append((name, version))
    return found


def _toml_tables(text: str) -> dict[str, Any]:
    """Read tables and simple values out of a TOML manifest.

    This is a deliberately small literal reader, not a TOML implementation: it
    understands table headers, string/number/bool/array literals, and inline
    comments, which is the whole `pyproject.toml` surface the audit reads. A
    construct it does not understand is skipped rather than guessed at.
    """
    root: dict[str, Any] = {}
    current = root
    pending_array: list[str] | None = None
    for raw in text.splitlines():
        line = _strip_toml_comment(raw).strip()
        if not line:
            continue
        if pending_array is not None:
            pending_array.append(line)
            if "]" not in line:
                continue
            joined = " ".join(pending_array)
            pending_array = None
            for key, value in list(current.items())[::-1]:
                if value is None:
                    current[key] = _toml_array(joined)
                    break
            continue
        section = _TOML_SECTION.match(line)
        if section:
            parts = [part.strip().strip('"') for part in section.group(1).split(".")]
            current = root
            for part in parts:
                nxt = current.get(part)
                if not isinstance(nxt, dict):
                    nxt = {}
                    current[part] = nxt
                current = nxt
            continue
        if current is root and "=" not in line:
            continue
        if "=" not in line:
            continue
        key, _, value = line.partition("=")
        key = key.strip().strip('"')
        value = value.strip()
        if value.startswith("[") and "]" not in value:
            # A multi-line array: park the key and fill it once the
            # closing bracket arrives on a later line.
            current[key] = None
            pending_array = [value]
            continue
        if value.startswith("["):
            current[key] = _toml_array(value)
            continue
        current[key] = _toml_scalar(value)
    return root


def _toml_array(value: str) -> list[str]:
    inner = value.strip()[1:]
    if inner.endswith("]"):
        inner = inner[:-1]
    return [item.strip().strip('"').strip("'") for item in re.findall(r"[\"']([^\"']*)[\"']", inner)]


def _toml_scalar(value: str) -> Any:
    if value.startswith("{") and value.endswith("}"):
        return {}
    stripped = value.strip().strip('"').strip("'")
    if stripped in {"true", "false"}:
        return stripped == "true"
    try:
        return int(stripped)
    except ValueError:
        return stripped


def _parse_requirements_packages(text: str) -> list[tuple[str, str | None]]:
    found: list[tuple[str, str | None]] = []
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line or line.startswith("-"):
            continue
        match = _REQUIREMENT_LINE.match(line)
        if not match:
            continue
        name, _, operator, version = match.group(1), match.group(2), match.group(3), match.group(4)
        found.append((name, version if operator == "==" else None))
    return found


def _read_manifest_dependencies(relative: str, text: str) -> list[tuple[str, str | None]]:
    if relative == "Directory.Packages.props" or relative.endswith(".csproj"):
        return _parse_msbuild_packages(text)
    if relative == "package.json":
        return _parse_package_json_packages(text)
    if relative == "Cargo.toml":
        return _parse_toml_packages(text, _CARGO_DEP_STANZAS)
    if relative == "pyproject.toml":
        return _parse_pyproject_packages(text)
    return _parse_requirements_packages(text)


def read_manifest_dependencies(project_path: Path) -> ManifestRead:
    """Read the consumer's own dependency manifests, bounded and read-only.

    The walk is capped at `_MANIFEST_SCAN_DEPTH` directories below the project
    root and `_MANIFEST_SCAN_FILE_BUDGET` files, skips build and vendor
    directories, and only parses the dependency stanza of the six manifest
    kinds named in the design. A manifest that exists but cannot be parsed is
    reported in `unreadable` rather than treated as "no dependencies", so an
    unreadable file can never be read as proof of either adoption or absence.
    """
    pinned: dict[str, list[str]] = {}
    sources: list[str] = []
    unreadable: list[str] = []
    budget = _MANIFEST_SCAN_FILE_BUDGET
    for directory, subdirs, files in os.walk(project_path):
        current = Path(directory)
        depth = len(current.relative_to(project_path).parts)
        if depth >= _MANIFEST_SCAN_DEPTH:
            subdirs[:] = []
            continue
        subdirs[:] = sorted(name for name in subdirs if name not in _MANIFEST_SKIP_DIRS and not name.startswith("."))
        for name in sorted(files):
            if not _is_manifest_name(name):
                continue
            if budget <= 0:
                unreadable.append("... scan budget exhausted")
                return ManifestRead(pinned, sources, unreadable)
            budget -= 1
            relative = current.relative_to(project_path).as_posix()
            relative = name if relative == "." else f"{relative}/{name}"
            try:
                text = (current / name).read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):
                unreadable.append(relative)
                continue
            try:
                entries = _read_manifest_dependencies(relative, text)
            except ValueError:
                unreadable.append(relative)
                continue
            sources.append(relative)
            for package, version in entries:
                if _is_pinned_version(version):
                    pinned.setdefault(_normalise_package_name(package), []).append(f"{version} ({relative})")
    return ManifestRead(pinned, sorted(sources), sorted(unreadable))


def _is_manifest_name(name: str) -> bool:
    """Match only the six manifest kinds the design names, nothing else."""
    if name in _MANIFEST_EXACT_NAMES or name.endswith(".csproj"):
        return True
    return name.startswith(_MANIFEST_PREFIXES) and name.endswith(".txt")


def _validate_consumes_entry(
    entry: Any,
    capability_id: str,
    provider: str,
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    secret_words: "re.Pattern[str]",
) -> bool:
    """Validate one `consumes[]` record. Returns True when the record holds.

    A record that names the wrong provider is not a consumption edge for this
    capability; the caller reports the capability as not adopted.
    """
    if not isinstance(entry, dict):
        finding(findings, "WARNING", CAPABILITY_ADOPTION_UNEVIDENCED, f"capability {capability_id!r} consumes[] entry must be an object", project_id)
        return False
    declared_provider = entry.get("provider")
    if isinstance(declared_provider, str) and declared_provider.strip() and declared_provider.strip() != provider:
        finding(
            findings,
            "WARNING",
            CAPABILITY_ADOPTION_UNEVIDENCED,
            f"capability {capability_id!r} consumes[] names provider {declared_provider!r} but the registry binds it to {provider!r}",
            project_id,
        )
        return False
    ref = entry.get("evidence_ref")
    if not isinstance(ref, str) or not ref.strip():
        finding(findings, "WARNING", CAPABILITY_ADOPTION_UNEVIDENCED, f"capability {capability_id!r} consumes[] entry for {provider!r} has no evidence_ref", project_id)
        return False
    if _is_absolute_path(ref) or _has_path_escape(ref):
        finding(findings, "WARNING", CAPABILITY_ADOPTION_EVIDENCE_STALE, f"capability {capability_id!r} evidence_ref is absolute or escapes the project root and was not read: {_redact_secret_value(ref, secret_words)}", project_id)
        return False
    if not _standalone_evidence_resolves(project_path, ref):
        finding(findings, "WARNING", CAPABILITY_ADOPTION_EVIDENCE_STALE, f"capability {capability_id!r} evidence_ref no longer resolves in the project root: {_redact_secret_value(ref, secret_words)}", project_id)
        return False
    return True


def _local_implementation_note(entry: dict[str, Any] | None, project_path: Path, secret_words: "re.Pattern[str]") -> str:
    """Describe a declared local implementation without ever guessing one.

    Detection is bounded to declaration-supplied paths. An unregistered
    hand-rolled implementation is never inferred, because an inferred
    duplicate cannot be distinguished from a legitimate boundary decision.
    """
    if not isinstance(entry, dict):
        return ""
    raw = entry.get("local_implementation")
    paths = raw if isinstance(raw, list) else ([raw] if isinstance(raw, str) else [])
    notes: list[str] = []
    for item in paths:
        if not isinstance(item, str) or not item:
            continue
        if _is_absolute_path(item) or _has_path_escape(item):
            notes.append(f"declared local implementation {_redact_secret_value(item, secret_words)} escapes the project root and was not read")
            continue
        state = "present" if (project_path / item).exists() else "absent"
        notes.append(f"declared local implementation {_redact_secret_value(item, secret_words)} ({state})")
    return "; ".join(notes)


def _scan_prose_declines(project_path: Path, capability_id: str) -> list[tuple[str, str]]:
    """Find a project's own prose boundary argument for a capability.

    This names evidence the audit already has; it never *is* a decline. A hit
    asks the repository for a `declines[]` record via
    `CAPABILITY_ADOPTION_DECLINE_UNRECORDED`. The scan is bounded to four
    document names plus `.ai-rules/*.md` and reads them in full only when the
    capability id is mentioned at all.
    """
    documents = ["AGENTS.md", "README.md", "HANDOFF.md", "ROADMAP.md"]
    rules_dir = project_path / ".ai-rules"
    if rules_dir.is_dir():
        documents.extend(f".ai-rules/{item.name}" for item in sorted(rules_dir.glob("*.md")))
    mentions = {name.lower() for name in capability_id.replace("-", " ").split()}
    found: list[tuple[str, str]] = []
    for relative in documents:
        try:
            text = (project_path / relative).read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        lowered = text.lower()
        if not all(mention in lowered for mention in mentions):
            continue
        for rationale, triggers in _PROSE_DECLINE_TRIGGERS:
            for trigger in triggers:
                position = lowered.find(trigger)
                while position != -1:
                    window = lowered[max(0, position - 240) : position + 240]
                    if all(mention in window for mention in mentions):
                        found.append((relative, rationale))
                        break
                    position = lowered.find(trigger, position + 1)
                if found and found[-1][0] == relative and found[-1][1] == rationale:
                    break
    return found


def validate_declines(
    declines: Any,
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    registry: CapabilityRegistry,
) -> set[str]:
    """Validate a project's `declines[]` records and return the exempt ids.

    A decline is repo-local and capability-scoped: it exempts this project for
    this capability only, and never suppresses a finding in any other project
    or for any other capability. A decline with no resolvable evidence is
    itself reported, because an assertion is not a boundary argument.
    """
    exempt: set[str] = set()
    if declines is None:
        return exempt
    if not isinstance(declines, list):
        finding(findings, "WARNING", CAPABILITY_ADOPTION_DECLINE_UNRECORDED, "declines must be an array", project_id)
        return exempt
    for index, entry in enumerate(declines):
        if not isinstance(entry, dict):
            finding(findings, "WARNING", CAPABILITY_ADOPTION_DECLINE_UNRECORDED, f"declines[{index}] must be an object", project_id)
            continue
        capability_id = entry.get("capability")
        if not isinstance(capability_id, str) or not capability_id:
            finding(findings, "WARNING", CAPABILITY_ADOPTION_DECLINE_UNRECORDED, f"declines[{index}] requires a capability id", project_id)
            continue
        if capability_id not in registry.capabilities:
            finding(findings, "WARNING", CAPABILITY_ADOPTION_UNRESOLVED, f"declines[{index}] names capability {capability_id!r}, which binds to no provider in the capability registry", project_id)
            continue
        rationale = entry.get("rationale")
        if rationale not in DECLINE_RATIONALES:
            finding(findings, "WARNING", CAPABILITY_ADOPTION_DECLINE_UNRECORDED, f"decline for capability {capability_id!r} cites an unknown rationale: {rationale!r}; accepted values: {', '.join(DECLINE_RATIONALES)}", project_id)
            continue
        ref = entry.get("evidence_ref")
        if not isinstance(ref, str) or not ref.strip():
            finding(findings, "WARNING", CAPABILITY_ADOPTION_DECLINE_UNRECORDED, f"decline for capability {capability_id!r} requires an evidence_ref that resolves in the project root", project_id)
            continue
        if not _standalone_evidence_resolves(project_path, ref):
            finding(findings, "WARNING", CAPABILITY_ADOPTION_DECLINE_UNRECORDED, f"decline for capability {capability_id!r} cites an evidence_ref that does not resolve in the project root: {ref}", project_id)
            continue
        exempt.add(capability_id)
    return exempt


def provider_capability_verified(
    workspace_root: Path,
    provider_paths: dict[str, Path],
    provider_id: str,
    capability_id: str,
) -> bool:
    """Return True when the provider declares this capability `verified`.

    The provider's own declaration is the only accepted evidence. A grant or a
    consumer record never substitutes for it, and an unreadable or absent
    provider declaration is *not* verified — which is what keeps an
    unverifiable provider from graduating a capability to `required`.
    """
    provider_path = provider_paths.get(provider_id)
    if provider_path is None:
        return False
    try:
        declaration = json.loads((provider_path / ".project.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError, UnicodeDecodeError):
        return False
    if not isinstance(declaration, dict):
        return False
    capabilities = declaration.get("capabilities")
    if not isinstance(capabilities, dict):
        return False
    entry = capabilities.get(capability_id)
    if not isinstance(entry, dict) or entry.get("evidence_state") != "verified":
        return False
    ref = entry.get("evidence_ref")
    if not isinstance(ref, str) or not ref:
        return False
    del workspace_root  # the provider path already carries the root
    return _standalone_evidence_resolves(provider_path, ref)


def measure_capability_adoption(
    declaration: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    registry: CapabilityRegistry | None,
    provider_verified: Any = None,
) -> None:
    """Measure whether this project consumes each registered capability.

    Read-only and additive: a project that declares no `capabilities` block and
    no `declines[]` record produces no adoption finding, and a capability in
    `exempt` posture is never charged to anyone. The measurement never promotes
    a capability to `verified`; it only reports what the three sources agree
    on.
    """
    if registry is None:
        return
    secret_words = secret_words_for()
    capabilities = declaration.get("capabilities")
    consumes_index: dict[str, list[Any]] = {}
    if isinstance(capabilities, dict):
        for name, entry in capabilities.items():
            if isinstance(entry, dict) and isinstance(entry.get("consumes"), list):
                consumes_index[name] = entry["consumes"]
    declined = validate_declines(declaration.get("declines"), project_path, findings, project_id, registry)
    for name, entries in sorted(consumes_index.items()):
        if name not in registry.capabilities and name not in declined:
            finding(findings, "WARNING", CAPABILITY_ADOPTION_UNRESOLVED, f"capabilities[{name!r}].consumes[] names a capability that binds to no provider in the capability registry", project_id)
    manifests: ManifestRead | None = None
    prose_scanned: set[str] = set()
    for capability_id in sorted(registry.capabilities):
        entry = registry.capabilities[capability_id]
        provider = _resolved_provider(entry)
        if provider is None:
            finding(findings, "WARNING", CAPABILITY_ADOPTION_UNRESOLVED, f"capability {capability_id!r} does not bind to exactly one provider in the capability registry; no repository is charged", project_id)
            continue
        if provider == project_id:
            # The provider is exempt for its own capability: `consumes[]` is
            # empty and a self-adoption failure would be meaningless.
            continue
        if capability_id in declined:
            continue
        verified = bool(provider_verified(provider, capability_id)) if callable(provider_verified) else False
        posture, posture_reason = effective_capability_posture(capability_id, entry, registry, verified)
        if posture == "exempt":
            continue
        package = entry.get("package")
        if not isinstance(package, str) or not package.strip():
            finding(findings, "WARNING", CAPABILITY_ADOPTION_UNRESOLVED, f"capability {capability_id!r} binds to provider {provider!r} without naming a package; no repository is charged", project_id)
            continue
        if manifests is None:
            manifests = read_manifest_dependencies(project_path)
        edges = manifests.pinned.get(_normalise_package_name(package), [])
        consumes = consumes_index.get(capability_id, [])
        # The declaration record is validated on its own terms first: an
        # `evidence_ref` that escapes the root or no longer resolves is refused
        # whether or not the manifest edge happens to exist, and a capability
        # with no valid record is undocumented, never adopted.
        record_ok = bool(consumes) and any(
            _validate_consumes_entry(item, capability_id, provider, project_path, findings, project_id, secret_words)
            for item in consumes
        )
        if not edges:
            if manifests.unreadable:
                finding(
                    findings,
                    "WARNING",
                    CAPABILITY_ADOPTION_UNEVIDENCED,
                    f"capability {capability_id!r} consumption is unproven: {provider}/{package} is not pinned and a manifest could not be read ({', '.join(manifests.unreadable)}); adoption is never assumed",
                    project_id,
                )
                continue
            if not consumes and capability_id not in prose_scanned:
                prose_scanned.add(capability_id)
                for document, rationale in _scan_prose_declines(project_path, capability_id):
                    finding(
                        findings,
                        "WARNING",
                        CAPABILITY_ADOPTION_DECLINE_UNRECORDED,
                        f"{document} documents a non-adoption boundary for capability {capability_id!r} ({rationale}) but the project records no declines[] entry; prose is neither adoption nor a decline",
                        project_id,
                    )
                    break
            level = "ERROR" if posture == "required" else "WARNING"
            detail = _local_implementation_note(consumes[0] if consumes else None, project_path, secret_words)
            message = (
                f"capability {capability_id!r} is {posture} ({posture_reason}) but no manifest in the project root pins {package} from provider {provider!r}"
            )
            if consumes:
                message += f"; declared in capabilities[{capability_id!r}].consumes[] with no matching manifest edge"
            else:
                message += f"; capabilities[{capability_id!r}].consumes[] is absent"
            if detail:
                message += f"; {detail}"
            finding(findings, level, CAPABILITY_ADOPTION_MISSING, message, project_id)
            continue
        if not consumes:
            finding(
                findings,
                "WARNING",
                CAPABILITY_ADOPTION_UNEVIDENCED,
                f"manifest pins {package} from {', '.join(sorted(set(edges)))} but capabilities[{capability_id!r}].consumes[] records no evidence_ref for provider {provider!r}; the edge is real and the record is not",
                project_id,
            )
            continue
        del record_ok  # a valid record plus a proven edge is adoption; no finding
