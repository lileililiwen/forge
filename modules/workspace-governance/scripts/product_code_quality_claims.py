"""Repository README status-claim analysis for product-code quality."""

from __future__ import annotations

import json
import os
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

import product_code_quality as _core

globals().update({name: value for name, value in vars(_core).items() if not name.startswith("__")})

def _synonym_patterns() -> "dict[str, tuple[re.Pattern[str], ...]]":
    """Compile the claim synonyms once. Word-bounded, case-insensitive."""
    global _SYNONYM_PATTERNS
    if not _SYNONYM_PATTERNS:
        _SYNONYM_PATTERNS = {
            surface: tuple(
                re.compile(r"(?<![a-z0-9])" + re.escape(synonym) + r"(?![a-z0-9])", re.IGNORECASE)
                for synonym in sorted(synonyms, key=len, reverse=True)
            )
            for surface, synonyms in CLAIM_SURFACE_SYNONYMS.items()
        }
    return _SYNONYM_PATTERNS


_FUTURE_PATTERNS: "tuple[re.Pattern[str], ...]" = ()


def _future_patterns() -> "tuple[re.Pattern[str], ...]":
    """Compile the condition-3 scoping markers once, word-bounded."""
    global _FUTURE_PATTERNS
    if not _FUTURE_PATTERNS:
        _FUTURE_PATTERNS = tuple(
            re.compile(r"(?<![a-z])" + re.escape(marker).replace(r"\ ", r"\s+"), re.IGNORECASE)
            for marker in README_FUTURE_MARKERS + README_CONDITIONAL_MARKERS
        )
    return _FUTURE_PATTERNS


class ClaimReadError(Exception):
    """Raised when `README.md` exists but cannot be read safely."""


@dataclass
class ClaimGap:
    """One statement the checker could not scope to a decidable surface.

    A gap is information about the checker's own coverage, never about a defect
    in the repository (`design.md` §10, ruling 7). It is never a pass and never
    a per-repository finding.
    """

    project: str
    file: str
    line: int
    statement: str
    reason: str

    def describe(self) -> str:
        return f"{self.project}: {self.file}:{self.line} {self.reason} [{self.statement}]"


@dataclass
class ClaimReport:
    """The result of the `readme-status-claim` rule for one repository."""

    project: str
    findings: list[Finding] = field(default_factory=list)
    gaps: list[ClaimGap] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    census_complete: bool = True

    @property
    def contradicted(self) -> list[Finding]:
        return [item for item in self.findings if item.level == "ERROR"]


def claim_vocabulary(governance_root: Path | None = None) -> tuple[tuple[str, ...], tuple[str, ...]]:
    """Resolve `(surfaces, absence_phrases)` fail-closed from `vocabulary.json`.

    A missing, empty, or unusable pair raises `VocabularyError`. It is never
    defaulted: an empty surface list would make every claim vacuously true,
    which is a silent pass on exactly the thing this rule exists to catch.
    """
    import vocabulary

    root = Path(governance_root) if governance_root is not None else _governance_root()
    loaded = vocabulary.load_vocabulary_for_governance_root(root)
    try:
        surfaces = tuple(loaded["readme_claim_surfaces"])
        phrases = tuple(loaded["readme_absence_phrases"])
    except KeyError as exc:
        raise VocabularyError(
            f"vocabulary is missing {exc.args[0]!r}; the readme-claim-verification rule is fail-closed"
        ) from exc
    if not surfaces or not phrases:
        raise VocabularyError("readme_claim_surfaces and readme_absence_phrases must both be non-empty")
    unknown = sorted(set(surfaces) - set(CLAIM_SURFACE_SYNONYMS))
    if unknown:
        raise VocabularyError(f"readme_claim_surfaces has no evidence rule for: {', '.join(unknown)}")
    return surfaces, phrases


def _absence_patterns(phrases: Iterable[str]) -> tuple["re.Pattern[str]", ...]:
    """Compile the absence vocabulary.

    An entry containing `...` is a bounded wildcard — that is how the design's
    `no … exists` form is expressed in a machine-readable list, and it is what
    lets a claim name a comma-separated list of surfaces in one sentence. Every
    other entry is a literal phrase.
    """
    compiled: list[re.Pattern[str]] = []
    for phrase in phrases:
        body = r".{1,200}?".join(
            re.escape(segment).replace(r"\ ", r"\s+") for segment in phrase.split("...")
        )
        compiled.append(re.compile(r"(?<![a-z0-9])" + body + r"(?![a-z0-9])", re.IGNORECASE))
    return tuple(compiled)


def _prose_sentences(text: str) -> list[tuple[int, str]]:
    """Split README prose into `(line_number, sentence)` pairs.

    A claim is a *sentence*, not a line and not a paragraph: a status line that
    states what exists and then what does not must not let the positive half
    contribute surfaces to the negative half. Fenced code blocks, HTML comments
    and block quotes are dropped wholesale, and markdown structure that starts
    a new block is a sentence boundary.
    """
    sentences: list[tuple[int, str]] = []
    buffer: list[str] = []
    start_line = 1
    in_fence = False
    in_comment = False
    for index, raw in enumerate(text.splitlines(), start=1):
        stripped = raw.strip()
        if in_comment:
            if "-->" in stripped:
                in_comment = False
            continue
        if in_fence:
            if stripped.startswith("```") or stripped.startswith("~~~"):
                in_fence = False
            continue
        if stripped.startswith("```") or stripped.startswith("~~~"):
            if buffer:
                sentences.extend(_flush(buffer, start_line))
                buffer = []
            in_fence = True
            continue
        if stripped.startswith("<!--"):
            if "-->" not in stripped:
                in_comment = True
            continue
        if not stripped:
            if buffer:
                sentences.extend(_flush(buffer, start_line))
                buffer = []
            continue
        if stripped.startswith(">") or stripped.startswith("#") or re.match(r"^\s*([-*+]|\d+\.)\s", raw):
            if buffer:
                sentences.extend(_flush(buffer, start_line))
                buffer = []
            if stripped.startswith("#"):
                # A heading is structure, never prose to evaluate.
                continue
            if stripped.startswith(">"):
                # A block quote is someone else's statement, so it is not this
                # repository's status claim (`design.md` §4.1, condition 2).
                continue
        if not buffer:
            start_line = index
        buffer.append(stripped)
    if buffer:
        sentences.extend(_flush(buffer, start_line))
    return sentences


def _flush(buffer: list[str], start_line: int) -> list[tuple[int, str]]:
    """Split one wrapped prose block into sentences, keeping the start line."""
    text = " ".join(buffer)
    parts = re.split(r"(?<=[.!?])\s+(?=[A-Z0-9`(\"'\[])", text)
    result: list[tuple[int, str]] = []
    for part in parts:
        candidate = part.strip()
        if candidate:
            result.append((start_line, candidate))
    return result


def read_readme(root_dir: Path) -> tuple[str, bool]:
    """Read one bounded `README.md`. Returns `(text, truncated)`.

    A missing README raises `FileNotFoundError` — the caller turns that into "no
    claim, no finding", because absence of documentation is not a false claim
    and the profile's `REQUIRED_FILE` requirement owns the missing-file case.
    An unreadable or non-UTF-8 file raises `ClaimReadError` and is counted as a
    coverage gap, never read as a verified claim.
    """
    path = root_dir / CLAIM_README
    if not path.is_file():
        raise FileNotFoundError(str(path))
    if path.is_symlink():
        raise ClaimReadError(f"{CLAIM_README} is a symlink and was not read")
    try:
        with path.open("rb") as handle:
            raw = handle.read(CLAIM_READ_LIMIT_BYTES + 1)
    except OSError as exc:
        raise ClaimReadError(f"{CLAIM_README} cannot be read: {exc}") from exc
    truncated = len(raw) > CLAIM_READ_LIMIT_BYTES
    if truncated:
        raw = raw[:CLAIM_READ_LIMIT_BYTES]
    try:
        return raw.decode("utf-8"), truncated
    except UnicodeDecodeError as exc:
        raise ClaimReadError(f"{CLAIM_README} is not valid UTF-8: {exc}") from exc


def claim_surfaces(sentence: str, surfaces: Iterable[str]) -> tuple[str, ...]:
    """Resolve the surfaces a claim names, in canonical vocabulary order.

    Only the surfaces the claim names are ever returned: that is the scope
    requirement, and it is why a claim is never evaluated against the rest of
    the repository.
    """
    resolved: list[str] = []
    patterns = _synonym_patterns()
    for surface in surfaces:
        if surface in patterns and any(pattern.search(sentence) for pattern in patterns[surface]):
            resolved.append(surface)
    return tuple(resolved)


@dataclass
class TreeCensus:
    """One bounded, read-only, symlink-refusing census of a project tree."""

    files: list[str] = field(default_factory=list)
    dirs: set[str] = field(default_factory=set)
    complete: bool = True
    truncated_reason: str = ""


def census_tree(root_dir: Path) -> TreeCensus:
    """Walk a project tree once, bounded, never following a symlink.

    The census is path-name only: no file contents are read, no project command
    is executed, and nothing is written. Symlinked directories are pruned
    rather than followed, matching `discover_projects`.
    """
    result = TreeCensus()
    entries = 0
    for directory, subdirs, files in os.walk(root_dir, followlinks=False):
        current = Path(directory)
        try:
            depth = len(current.relative_to(root_dir).parts)
        except ValueError:  # pragma: no cover - defensive
            continue
        if depth >= CLAIM_SCAN_MAX_DEPTH:
            subdirs[:] = []
            continue
        pruned: list[str] = []
        for name in sorted(subdirs):
            # The skip set is explicit rather than a blanket dot-directory rule:
            # `.github/workflows/` is the CI surface's evidence, so pruning every
            # dot-directory would hide it.
            if name in CLAIM_SCAN_SKIP_DIRS:
                continue
            candidate = current / name
            if candidate.is_symlink():
                continue
            pruned.append(name)
        subdirs[:] = pruned
        for name in sorted(subdirs):
            entries += 1
            if entries > CLAIM_SCAN_MAX_ENTRIES:
                result.complete = False
                result.truncated_reason = (
                    f"directory census stopped at {CLAIM_SCAN_MAX_ENTRIES} entries; "
                    "surfaces it could not reach are reported as untested, not absent"
                )
                return result
            result.dirs.add(((current / name).relative_to(root_dir)).as_posix())
        for name in sorted(files):
            entries += 1
            if entries > CLAIM_SCAN_MAX_ENTRIES:
                result.complete = False
                result.truncated_reason = (
                    f"directory census stopped at {CLAIM_SCAN_MAX_ENTRIES} entries; "
                    "surfaces it could not reach are reported as untested, not absent"
                )
                return result
            candidate = current / name
            if candidate.is_symlink():
                continue
            relative = candidate.relative_to(root_dir).as_posix()
            if not _is_path_safe(Path(relative)):
                continue
            result.files.append(relative)
    result.files.sort()
    return result


def _language_for(suffix: str) -> str | None:
    for language, values in LANGUAGE_EXTENSIONS.items():
        if suffix in values:
            return language
    return None


def _source_extension_census(census: TreeCensus, *, rooted: bool) -> str | None:
    """Return one evidence path for a language source file, or None.

    `rooted=True` restricts the search to a source root or the repository root
    (`application-source`); `rooted=False` searches everywhere the census
    reached (`repository-content`, which must fire on a tree that has no source
    root at all). Both reuse `DEFAULT_EXCLUDED_DIRS` and `_is_excluded`, so test
    code, fixtures and examples never stand in as product source.
    """
    extensions = {ext for values in LANGUAGE_EXTENSIONS.values() for ext in values}
    for relative in census.files:
        name = relative.rsplit("/", 1)[-1]
        suffix = Path(name).suffix.lower()
        if suffix not in extensions:
            continue
        parts = relative.split("/")
        if rooted and len(parts) > 1 and parts[0] not in CLAIM_SOURCE_ROOTS:
            continue
        if _is_excluded(Path(relative), (), _language_for(suffix) or "generic"):
            continue
        return relative
    return None


def _first(census: TreeCensus, predicate) -> str | None:
    for relative in census.files:
        if predicate(relative):
            return relative
    return None


def _any_root(census: TreeCensus, names: Iterable[str]) -> str | None:
    for name in sorted(names):
        if name in census.dirs:
            return name
    return None


def _manifest_evidence(
    project_path: Path,
    manifest_read: Any,
    predicate,
) -> str | None:
    """Search the project's own dependency manifests for one evidence path.

    Reuses `workspace_check.read_manifest_dependencies` so a pinned platform
    package is read by the same bounded parser every other check uses.
    """
    reader = manifest_read
    if reader is None:
        try:
            import workspace_check
        except ImportError:  # pragma: no cover - governance scripts are siblings
            return None
        reader = workspace_check.read_manifest_dependencies
    try:
        manifest = reader(project_path)
    except (OSError, ValueError):  # pragma: no cover - reader is fail-closed
        return None
    for package in sorted(manifest.pinned):
        if predicate(package):
            # `pinned[package]` holds "<version> (<manifest path>)" entries, so
            # the finding names the exact manifest that carries the pin.
            evidence = manifest.pinned[package][0] if manifest.pinned[package] else package
            return f"{package} pinned in {evidence}"
    return None


def evaluate_claims(
    project_path: Path,
    declaration: dict[str, Any] | None = None,
    *,
    governance_root: Path | None = None,
    surfaces: Iterable[str] | None = None,
    absence_phrases: Iterable[str] | None = None,
    manifest_read: Any = None,
    promoted_projects: Iterable[str] | None = None,
) -> ClaimReport:
    """Run the `readme-status-claim` rule for one repository.

    Triggered by claim presence, not by any declaration field: there is no
    `readme_claims` block, no `mode`, and no exemption
    (`design.md` §10, ruling 10). A repository with no qualifying claim
    produces no finding and no gap, so the other repositories in the fleet are
    byte-identical to their pre-change finding set.

    Raises `VocabularyError` when the surface vocabulary cannot be resolved,
    and `ConfigError` when `project_path` is not a project root. Both are
    fail-closed configuration errors, never a guessed pass.
    """
    project_path = Path(project_path)
    if not project_path.is_dir():
        raise ConfigError(f"project directory not found: {project_path}")
    if surfaces is None or absence_phrases is None:
        resolved_surfaces, resolved_phrases = claim_vocabulary(governance_root)
        surfaces = resolved_surfaces if surfaces is None else surfaces
        absence_phrases = resolved_phrases if absence_phrases is None else absence_phrases
    surfaces = tuple(surfaces)
    report = ClaimReport(project=project_path.name)
    try:
        text, truncated = read_readme(project_path)
    except FileNotFoundError:
        return report
    except ClaimReadError as exc:
        report.gaps.append(ClaimGap(report.project, CLAIM_README, 1, str(exc), "README could not be read"))
        return report
    if truncated:
        report.gaps.append(
            ClaimGap(
                report.project,
                CLAIM_README,
                1,
                f"read to the {CLAIM_READ_LIMIT_BYTES}-byte bound and truncated",
                "README truncated at the bounded read size; a claim beyond the bound is untested",
            )
        )
    patterns = _absence_patterns(absence_phrases)
    future = _future_patterns()
    census: TreeCensus | None = None
    promoted = set(promoted_projects or ())
    # One finding per contradicted surface for the whole README, not per
    # sentence: when two claims name the same surface the surface is reported
    # once, against the first claim, naming every line that names it.
    seen: dict[str, list[int]] = {}
    contradicted: list[tuple[str, int, list[int], str, str]] = []
    for line, sentence in _prose_sentences(text):
        if not any(pattern.search(sentence) for pattern in patterns):
            continue
        if any(pattern.search(sentence) for pattern in future):
            # Condition 3: scoped to a future, hypothetical, or conditional
            # state, so it is not a claim about the current tree and not a
            # coverage gap either.
            continue
        named = claim_surfaces(sentence, surfaces)
        if not named:
            report.gaps.append(
                ClaimGap(
                    report.project,
                    CLAIM_README,
                    line,
                    _excerpt(sentence),
                    "negative status statement names no surface the checker can resolve",
                )
            )
            continue
        if census is None:
            census = census_tree(project_path)
            if not census.complete:
                report.census_complete = False
                report.gaps.append(
                    ClaimGap(report.project, CLAIM_README, line, census.truncated_reason, "bounded census incomplete")
                )
                continue
        for surface in named:
            if surface in seen:
                if line not in seen[surface]:
                    seen[surface].append(line)
                continue
            seen[surface] = [line]
            present, evidence = surface_evidence(
                surface,
                census,
                project_path,
                declaration,
                manifest_read=manifest_read,
                promoted_projects=promoted,
            )
            if present:
                contradicted.append((surface, evidence, _excerpt(sentence)))
    for surface, evidence, excerpt in contradicted:
        lines = seen[surface]
        report.findings.append(
            Finding(
                rule_id=CLAIM_RULE_ID,
                level="ERROR",
                severity="FAIL",
                file=CLAIM_README,
                line=lines[0],
                marker=excerpt,
                remediation=(
                    f"{CLAIM_README} line{'s' if len(lines) > 1 else ''} "
                    f"{', '.join(str(item) for item in lines)} "
                    f"{'deny' if len(lines) > 1 else 'denies'} the {surface!r} surface but the tree holds "
                    f"{evidence}; correct the claim or the tree — the repository owner decides which side is wrong"
                ),
            )
        )
    if report.findings:
        report.notes.append(
            f"{len(report.findings)} contradicted README claim surface(s) in {report.project}"
        )
    return report


def _excerpt(sentence: str, limit: int = 160) -> str:
    return sentence if len(sentence) <= limit else sentence[: limit - 3] + "..."


def surface_evidence(
    surface: str,
    census: TreeCensus,
    project_path: Path,
    declaration: dict[str, Any] | None = None,
    *,
    manifest_read: Any = None,
    promoted_projects: set[str] | None = None,
) -> tuple[bool, str]:
    """Decide one surface from bounded, read-only file-tree evidence.

    Returns `(present, evidence_path)`. A surface with no evidence rule raises
    `ConfigError`: a surface that cannot be decided is not a check, and the
    vocabulary is never widened to admit one (`design.md` §10, ruling 8).
    """
    if surface == "application-source":
        evidence = _source_extension_census(census, rooted=True)
        return (evidence is not None, evidence or "")
    if surface == "repository-content":
        evidence = _source_extension_census(census, rooted=False)
        return (evidence is not None, evidence or "")
    if surface == "build":
        evidence = _first(
            census,
            lambda rel: rel.rsplit("/", 1)[-1] in CLAIM_BUILD_FILES
            or rel.rsplit("/", 1)[-1].endswith(CLAIM_BUILD_SUFFIXES),
        )
        return (evidence is not None, evidence or "")
    if surface == "test":
        root = _any_root(census, CLAIM_TEST_ROOTS)
        if root is not None:
            return (True, root)
        evidence = _first(
            census,
            lambda rel: ".Tests/" in f"{rel}/" and rel.endswith(".csproj")
            or Path(rel).name.startswith("test_")
            or any(Path(rel).name.endswith(suffix) for suffix in (".test.ts", ".spec.ts", ".test.js", ".spec.js", "_test.go")),
        )
        return (evidence is not None, evidence or "")
    if surface == "ci":
        evidence = _first(census, lambda rel: rel.startswith(".github/workflows/"))
        if evidence is None:
            evidence = _first(census, lambda rel: rel.rsplit("/", 1)[-1] in CLAIM_CI_FILES or rel.startswith(".circleci/"))
        return (evidence is not None, evidence or "")
    if surface == "runtime":
        evidence = _first(census, lambda rel: rel.rsplit("/", 1)[-1] in CLAIM_RUNTIME_ENTRY_NAMES)
        if evidence is None:
            evidence = _any_root(census, ("cmd", "bin"))
        if evidence is None:
            evidence = _first(
                census,
                lambda rel: Path(rel).name.lower().startswith("dockerfile")
                and re.search(r"^\s*(ENTRYPOINT|CMD)\b", _read_bounded(project_path / rel), re.IGNORECASE | re.MULTILINE),
            )
        return (evidence is not None, evidence or "")
    if surface == "api":
        evidence = _first(census, lambda rel: Path(rel).name.endswith("Controller.cs"))
        if evidence is None:
            evidence = _any_root(census, CLAIM_API_ROOTS)
        if evidence is None:
            evidence = _first(census, lambda rel: re.match(r"^(openapi|swagger).*\.ya?ml$", Path(rel).name, re.IGNORECASE) is not None)
        return (evidence is not None, evidence or "")
    if surface == "web-client":
        evidence = _any_root(census, CLAIM_WEB_CLIENT_ROOTS)
        if evidence is not None:
            return (True, evidence)
        evidence = _first(census, lambda rel: rel == "package.json" and _declares_ui_framework(project_path / rel))
        return (evidence is not None, evidence or "")
    if surface == "mobile-client":
        evidence = _first(
            census,
            lambda rel: Path(rel).name == "pubspec.yaml" and _declares_flutter(project_path / rel),
        )
        if evidence is None:
            evidence = _first(census, lambda rel: Path(rel).name.endswith((".xcodeproj", ".xcworkspace")))
        if evidence is None:
            evidence = _first(census, lambda rel: Path(rel).suffix == ".dart" and "/packages/" not in f"/{rel}")
        if evidence is None and "android" in census.dirs and "ios" in census.dirs:
            return (True, "android+ios")
        return (evidence is not None, evidence or "")
    if surface == "migrations":
        evidence = _any_root(census, CLAIM_MIGRATION_ROOTS)
        if evidence is not None:
            return (True, evidence)
        evidence = _first(
            census,
            lambda rel: Path(rel).suffix == ".sql" and any(part in CLAIM_MIGRATION_ROOTS for part in rel.split("/")[:-1]),
        )
        return (evidence is not None, evidence or "")
    if surface == "deployment":
        # The declaration is the authority, and it comes FIRST. A repository
        # whose `deployment.deployable` is false has no deployment surface no
        # matter what files it carries: a root `compose.yaml` describes a LOCAL
        # run, and `compose_file` records the local runtime, not a provisioned
        # environment. Reading the tree first made every local compose file look
        # like deployment evidence, which produced a false positive on 3 of the
        # 4 repositories the rule first saw.
        if isinstance(declaration, dict):
            deployment = declaration.get("deployment")
            if isinstance(deployment, dict):
                deployable = deployment.get("deployable")
                if deployable is False:
                    return (False, "declaration deployment.deployable is false")
                if deployable is True:
                    compose = deployment.get("compose_file")
                    if isinstance(compose, str) and compose and _resolves_inside(project_path, compose):
                        return (True, f"declaration deployable with {compose}")
        evidence = _first(census, lambda rel: re.match(r"^compose.*\.ya?ml$", Path(rel).name, re.IGNORECASE) is not None)
        if evidence is None:
            root = census.dirs.intersection({"deployment", "deploy"})
            if root:
                evidence = sorted(root)[0]
        if evidence is None and isinstance(declaration, dict):
            deployment = declaration.get("deployment")
            compose = deployment.get("compose_file") if isinstance(deployment, dict) else None
            if isinstance(compose, str) and compose and _resolves_inside(project_path, compose):
                evidence = compose
        return (evidence is not None, evidence or "")
    if surface == "provider-integration":
        consumes = False
        if isinstance(declaration, dict):
            capabilities = declaration.get("capabilities")
            if isinstance(capabilities, list):
                consumes = any(
                    isinstance(item, dict) and isinstance(item.get("consumes"), list) and item["consumes"]
                    for item in capabilities
                )
        if consumes:
            return (True, "declaration capabilities[].consumes[]")
        evidence = _manifest_evidence(project_path, manifest_read, lambda name: name.startswith("platform"))
        return (evidence is not None, evidence or "")
    if surface == "production-evidence":
        if isinstance(declaration, dict) and isinstance(declaration.get("release_evidence"), dict):
            return (True, "declaration release_evidence")
        if promoted_projects and project_path.name in promoted_projects:
            return (True, "promotion ledger")
        return (False, "")
    raise ConfigError(f"no evidence rule for README claim surface: {surface!r}")
