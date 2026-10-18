#!/usr/bin/env python3
"""Portable product-code quality checker owned by Workspace Governance.

Detects test code in product source roots and counts placeholder markers.
Designed to be invoked both in-process (by the audit) and as a standalone
command (by Driftwatchdog Gate bindings and CI).

Exit codes
----------
0  no blocking finding, no review-required gap
1  quality failure: test code in product scope, or threshold exceeded
2  configuration error: missing or invalid policy, unreadable product file,
   or any other condition that must be reviewed manually

The checker is fail-closed: a missing product_roots block, an unsupported
language, an exclusion that escapes the project root, or an unreadable
product file is REVIEW_REQUIRED, never a guessed pass.

The `readme-status-claim` rule is the one rule that is *not* gated by the
`quality` block. It is triggered by the presence of a present-tense absence
claim in `README.md`, and it runs for every registered project
(`design.md` §10, ruling 10). It is strictly read-only: it reads one bounded
`README.md` plus a bounded, symlink-refusing directory census, spawns nothing,
and writes nothing.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from dataclasses import dataclass, field, asdict
from functools import lru_cache
from pathlib import Path
from typing import Any, Iterable

from vocabulary import VocabularyError


SCHEMA_VERSION = 1
LANGUAGES = ("rust", "python", "typescript", "javascript", "csharp", "go", "generic")

# The default placeholder markers and the secret redaction pattern live in
# vocabulary.json (see scripts/vocabulary.py). This module keeps no local
# duplicate list: both resolve lazily from the vocabulary, so a missing or
# unusable vocabulary raises VocabularyError instead of silently scanning
# with an empty list.


def _governance_root() -> Path:
    return Path(__file__).resolve().parents[1]


def default_placeholder_markers() -> tuple[str, ...]:
    import vocabulary

    return tuple(vocabulary.load_vocabulary_for_governance_root(_governance_root())["placeholder_markers"])


_SECRET_PATTERN: "re.Pattern[str] | None" = None


def _secret_pattern() -> "re.Pattern[str]":
    global _SECRET_PATTERN
    if _SECRET_PATTERN is None:
        import vocabulary

        _SECRET_PATTERN = vocabulary.secret_pattern(
            vocabulary.load_vocabulary_for_governance_root(_governance_root())["secret_field_substrings"]
        )
    return _SECRET_PATTERN


def __getattr__(name: str) -> Any:
    if name == "DEFAULT_PLACEHOLDER_MARKERS":
        return default_placeholder_markers()
    if name == "SECRET_WORDS":
        return _secret_pattern()
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")

# Default test-scope and out-of-product directories. These are matched
# against path segments, not full paths, so they apply inside any root.
# The list is restricted to universally non-product paths (test, fixture,
# generated, vendor, build) so projects can opt-in to scanning their own
# `scripts`, `docs`, or `deployment` trees by naming them in product_roots.
DEFAULT_EXCLUDED_DIRS = frozenset({
    "tests",
    "test",
    "__tests__",
    "spec",
    "specs",
    "node_modules",
    "vendor",
    "target",
    "build",
    "dist",
    "out",
    ".git",
    "fixtures",
    "fixture",
    "example",
    "examples",
    "generated",
    ".github",
    ".gitlab",
})

# Per-language file-name patterns that always count as out of product scope.
LANGUAGE_EXCLUDED_GLOBS: dict[str, tuple[str, ...]] = {
    "python": ("test_*.py", "*_test.py"),
    "javascript": ("*.test.js", "*.spec.js", "*.test.jsx", "*.spec.jsx"),
    "typescript": ("*.test.ts", "*.spec.ts", "*.test.tsx", "*.spec.tsx"),
    "go": ("*_test.go"),
}

# Per-language filename extension filter. Generic uses the full set.
LANGUAGE_EXTENSIONS: dict[str, tuple[str, ...]] = {
    "rust": (".rs",),
    "python": (".py",),
    "typescript": (".ts", ".tsx"),
    "javascript": (".js", ".jsx", ".mjs", ".cjs"),
    "csharp": (".cs",),
    "go": (".go",),
    "generic": (),  # generic matches any text file
}

# Secret-like vocabulary that must never appear in a finding message.
# Resolved from vocabulary.json at use time; see _secret_pattern.


@dataclass
class Policy:
    product_roots: tuple[str, ...]
    languages: tuple[str, ...]
    exclude: tuple[str, ...] = ()
    placeholder_threshold: int = 0
    placeholder_markers: tuple[str, ...] = field(default_factory=default_placeholder_markers)
    root_dir: Path | None = None

    def with_root(self, root: Path) -> "Policy":
        return Policy(
            product_roots=self.product_roots,
            languages=self.languages,
            exclude=self.exclude,
            placeholder_threshold=self.placeholder_threshold,
            placeholder_markers=self.placeholder_markers,
            root_dir=root,
        )


@dataclass
class Finding:
    rule_id: str
    level: str  # ERROR | WARNING | INFO
    severity: str  # human-facing: FAIL | REVIEW_REQUIRED | PASS
    file: str
    line: int
    marker: str
    remediation: str

    def to_json(self) -> dict[str, Any]:
        return asdict(self)


@dataclass
class Report:
    schema_version: int
    status: str  # PASS | FAIL | REVIEW_REQUIRED
    project: str
    summary: dict[str, int] = field(default_factory=lambda: {"test_code": 0, "placeholder": 0, "errors": 0, "warnings": 0, "files_scanned": 0, "files_unreadable": 0, "excluded_paths": 0, "readme_claim_contradicted": 0, "readme_claim_unscoped": 0})
    findings: list[Finding] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)

    def to_json(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "status": self.status,
            "project": self.project,
            "summary": self.summary,
            "findings": [f.to_json() for f in self.findings],
            "notes": list(self.notes),
        }


class ConfigError(ValueError):
    """Raised when policy is missing, invalid, or cannot be applied."""


# ---------------------------------------------------------------------------
# Policy parsing
# ---------------------------------------------------------------------------


def parse_policy(raw: dict[str, Any]) -> Policy:
    if not isinstance(raw, dict):
        raise ConfigError("quality must be an object")
    product_roots = raw.get("product_roots")
    if not isinstance(product_roots, list) or not product_roots:
        raise ConfigError("quality.product_roots must be a non-empty list of strings")
    roots: list[str] = []
    for entry in product_roots:
        if not isinstance(entry, str) or not entry:
            raise ConfigError("each product_roots entry must be a non-empty string")
        if entry.startswith("/") or ".." in Path(entry).parts:
            raise ConfigError(f"product_roots entry must be project-relative: {entry!r}")
        roots.append(entry)
    languages_raw = raw.get("languages")
    if not isinstance(languages_raw, list) or not languages_raw:
        raise ConfigError("quality.languages must be a non-empty list")
    languages: list[str] = []
    for entry in languages_raw:
        if not isinstance(entry, str) or entry not in LANGUAGES:
            raise ConfigError(f"unsupported language: {entry!r}; supported: {sorted(LANGUAGES)}")
        languages.append(entry)
    if len(set(languages)) != len(languages):
        raise ConfigError("quality.languages entries must be unique")
    exclude_raw = raw.get("exclude", [])
    if not isinstance(exclude_raw, list):
        raise ConfigError("quality.exclude must be a list of strings")
    exclude: list[str] = []
    for entry in exclude_raw:
        if not isinstance(entry, str) or not entry:
            raise ConfigError("each quality.exclude entry must be a non-empty string")
        if entry.startswith("/") or ".." in Path(entry).parts:
            raise ConfigError(f"quality.exclude entry must be project-relative: {entry!r}")
        exclude.append(entry)
    threshold = raw.get("placeholder_threshold", 0)
    if not isinstance(threshold, int) or isinstance(threshold, bool) or threshold < 0:
        raise ConfigError("quality.placeholder_threshold must be a non-negative integer")
    markers_raw = raw.get("placeholder_markers", list(default_placeholder_markers()))
    if not isinstance(markers_raw, list) or not markers_raw:
        raise ConfigError("quality.placeholder_markers must be a non-empty list")
    markers: list[str] = []
    for entry in markers_raw:
        if not isinstance(entry, str) or not entry:
            raise ConfigError("each quality.placeholder_markers entry must be a non-empty string")
        markers.append(entry)
    return Policy(
        product_roots=tuple(roots),
        languages=tuple(languages),
        exclude=tuple(exclude),
        placeholder_threshold=threshold,
        placeholder_markers=tuple(markers),
    )


def load_policy(declaration: dict[str, Any]) -> Policy:
    if "quality" not in declaration:
        raise ConfigError("project declaration is missing the 'quality' block")
    return parse_policy(declaration["quality"])


# ---------------------------------------------------------------------------
# Path classification
# ---------------------------------------------------------------------------


def _is_path_safe(rel: Path) -> bool:
    if rel.is_absolute():
        return False
    parts = rel.parts
    if not parts or parts[0] == "..":
        return False
    return True


def _is_excluded(rel: Path, exclude: Iterable[str], language: str) -> bool:
    parts = rel.parts
    for part in parts:
        if part in DEFAULT_EXCLUDED_DIRS:
            return True
    rel_posix = rel.as_posix()
    for raw in exclude:
        candidate = Path(raw)
        try:
            if rel == candidate or rel.is_relative_to(candidate):
                return True
        except ValueError:
            pass
        if candidate.as_posix() in rel_posix:
            return True
    for pattern in LANGUAGE_EXCLUDED_GLOBS.get(language, ()):
        if rel.match(pattern):
            return True
    return False


def _resolve_root(root_dir: Path, raw: str) -> Path:
    if not _is_path_safe(Path(raw)):
        raise ConfigError(f"product_roots entry must be project-relative: {raw!r}")
    candidate = (root_dir / raw).resolve(strict=False)
    base = root_dir.resolve()
    if not str(candidate).startswith(str(base)):
        raise ConfigError(f"product_roots entry escapes project root: {raw!r}")
    return candidate


def _resolve_exclude(root_dir: Path, raw: str) -> Path:
    if not _is_path_safe(Path(raw)):
        raise ConfigError(f"exclude entry must be project-relative: {raw!r}")
    candidate = (root_dir / raw).resolve(strict=False)
    base = root_dir.resolve()
    if not str(candidate).startswith(str(base)):
        raise ConfigError(f"exclude entry escapes project root: {raw!r}")
    return candidate


# Keep the historical checker module as the compatibility facade.
if __name__ == "__main__":
    sys.modules.setdefault("product_code_quality", sys.modules[__name__])
import product_code_quality_scope as _scope
globals().update({name: value for name, value in vars(_scope).items() if not name.startswith("__")})


def _marker_pattern(markers: Iterable[str]) -> re.Pattern[str]:
    escaped = [re.escape(m) for m in markers]
    return re.compile(r"\b(" + "|".join(escaped) + r")\b", re.IGNORECASE)


def scan_placeholders(path: Path, lines: list[str], markers: Iterable[str]) -> list[Finding]:
    pattern = _marker_pattern(markers)
    findings: list[Finding] = []
    for index, line in enumerate(lines, start=1):
        for match in pattern.finditer(line):
            token = match.group(0)
            if _secret_pattern().search(token):
                continue
            findings.append(Finding(
                rule_id="placeholder-marker",
                level="ERROR" if token.lower() in {"hack", "unimplemented!", "todo!"} else "WARNING",
                severity="FAIL",
                file=path.name,
                line=index,
                marker=token,
                remediation="replace the marker with the missing implementation or remove it once resolved",
            ))
    return findings


# ---------------------------------------------------------------------------
# Secret redaction
# ---------------------------------------------------------------------------


def redact(text: str) -> str:
    return _secret_pattern().sub("[REDACTED]", text)


# ---------------------------------------------------------------------------
# README status-claim verification (rule `readme-status-claim`)
# ---------------------------------------------------------------------------
#
# A README status line is a factual assertion, and this rule checks it against
# the file tree. The scope rule is the whole point of the capability: a claim is
# verified ONLY against the surfaces it names, and nothing else. A repository
# whose status line scopes its absence to surfaces that are genuinely absent is
# clean no matter how much source code, test suite or CI it ships
# (`design.md` §5.1).

CLAIM_RULE_ID = "readme-status-claim"
CLAIM_README = "README.md"

# Bounded read of one file. A README past this ceiling is read to the ceiling
# and the truncation is reported as a coverage gap, never as a verified claim.
CLAIM_READ_LIMIT_BYTES = 64 * 1024

# Bounded, read-only directory census, pruned to generated, vendored and
# dependency roots only.
#
# This is deliberately NOT `DEFAULT_EXCLUDED_DIRS`. That set answers "is this
# product source?", and it contains `tests`, `spec` and `.github` — the very
# evidence the `test` and `ci` surfaces exist to find. Reusing it here would
# have made a `tests/` root and a CI workflow invisible and silently reported
# the surface as absent, which is the one failure mode this rule must not have.
# `DEFAULT_EXCLUDED_DIRS` and `_is_excluded` are reused where they belong, in
# the source census below.
CLAIM_SCAN_SKIP_DIRS = frozenset(
    {
        ".git",
        "node_modules",
        "vendor",
        "target",
        "build",
        "dist",
        "out",
        "generated",
        "coverage",
        "fixtures",
        "fixture",
        "example",
        "examples",
        "__pycache__",
        ".venv",
        "venv",
        ".next",
        ".turbo",
        ".dart_tool",
        ".idea",
        "obj",
    }
)
CLAIM_SCAN_MAX_DEPTH = 12
# An exhausted census is not a clean bill of health: it is recorded as a
# coverage gap so a bounded walk can never be read as "the surface is absent".
CLAIM_SCAN_MAX_ENTRIES = 200_000

# Source roots for the `application-source` surface. `repository-content` uses
# the same extension census without this restriction, because a document-only
# tree has no source root at all.
CLAIM_SOURCE_ROOTS = frozenset(
    {"src", "lib", "app", "apps", "packages", "tools", "cmd", "internal", "server", "client"}
)

CLAIM_BUILD_FILES = (
    "Cargo.toml",
    "CMakeLists.txt",
    "go.mod",
    "package.json",
    "pyproject.toml",
    "pubspec.yaml",
)
CLAIM_BUILD_SUFFIXES = (".sln", ".slnx", ".csproj")
CLAIM_TEST_ROOTS = frozenset({"tests", "test", "__tests__", "spec"})
CLAIM_CI_FILES = ("Jenkinsfile", ".gitlab-ci.yml", "azure-pipelines.yml", "circleci.yml")
CLAIM_RUNTIME_ENTRY_NAMES = frozenset(
    {"Program.cs", "main.py", "main.go", "main.rs", "index.ts", "index.js", "main.dart"}
)
CLAIM_API_SUFFIXES = ("*Controller.cs",)
CLAIM_API_ROOTS = frozenset({"routes", "api"})
CLAIM_WEB_CLIENT_ROOTS = frozenset({"client", "web", "frontend", "wwwroot", "ui"})
CLAIM_MIGRATION_ROOTS = frozenset({"migrations", "Migrations", "alembic"})
CLAIM_UI_FRAMEWORKS = (
    "react",
    "vue",
    "angular",
    "@angular/core",
    "svelte",
    "solid-js",
    "preact",
    "next",
    "nuxt",
)

# Condition 3 of `design.md` §4.1: a statement scoped to a future,
# hypothetical, or unbuilt state is not a claim about the repository's current
# state, and is not a coverage gap either — it is not a status claim at all.
# These markers are the bounded implementation of that prose. The conditional
# forms matter: "when no run exists, the export refuses" describes a
# conditional antecedent, not what this repository currently is, and counting
# it as an unscoped status claim would dilute the one metric that is supposed
# to mean something.
README_FUTURE_MARKERS = (
    "will be",
    "will contain",
    "we will",
    "is intended to",
    "are intended to",
    "intends to",
    "plan to",
    "plans to",
    "once the",
    "first release",
    "next release",
    "initial release",
    "target architecture",
    "planning decision",
    "once implemented",
    "after implementation",
    "to be built",
    "remains to be built",
)
README_CONDITIONAL_MARKERS = ("when no", "when a", "when the", "if no", "if a", "if the", "unless", "until")

# The claim synonyms per surface, exactly as tabulated in `design.md` §4.2. The
# surface *keys* are the vocabulary; the synonyms and the evidence rules live
# here, beside the other checker-level tables, because they describe how the
# tree is read rather than which words a repository may declare.
CLAIM_SURFACE_SYNONYMS: dict[str, tuple[str, ...]] = {
    # Bare "source" and bare "implementation" were here and had to go. They are
    # the words an integration uses — "live source adapters", "source
    # provenance", "source-first" — so any sentence about deferring a provider
    # integration resolved to the application-source surface and fired. A surface
    # name has to be specific enough that a sentence about something else cannot
    # land on it.
    #
    # The denial forms are kept, because there the phrase names the surface
    # rather than merely containing a common word: "no source" is a claim about
    # application source, while "source adapters" is a claim about a provider
    # integration. Matching the denial keeps that detection without reintroducing
    # the false positive.
    "application-source": (
        "no application source",
        "no extension application",
        "application source",
        "extension application",
        "no source",
    ),
    "repository-content": (
        "planning artifacts only",
        "specifications and documentation only",
        "documentation only",
        "nothing is implemented",
    ),
    "build": ("build",),
    "test": ("tests", "test suite", "test project"),
    "ci": ("ci configuration", "ci workflow", "pipeline", "ci"),
    "runtime": ("product runtime", "runtime integration", "running application", "runtime"),
    "api": ("http api", "endpoints", "api"),
    "web-client": ("web client", "web ui", "frontend", "spa"),
    "mobile-client": ("mobile client", "mobile app", "flutter client"),
    "migrations": ("database migrations", "migrations"),
    "deployment": ("deployment evidence", "deployment"),
    "provider-integration": ("sibling platform integration", "provider integration", "provider"),
    "production-evidence": (
        "production evidence",
        "production verification",
        "release evidence",
        "published",
        "released",
    ),
}

_SYNONYM_PATTERNS: "dict[str, tuple[re.Pattern[str], ...]]" = {}


# Keep the historical checker module as the compatibility facade.
if __name__ == "__main__":
    sys.modules.setdefault("product_code_quality", sys.modules[__name__])
import product_code_quality_claims as _claims
globals().update({name: value for name, value in vars(_claims).items() if not name.startswith("__")})


def _resolves_inside(project_path: Path, relative: str) -> bool:
    """Resolve a declaration-supplied relative path, refusing escapes.

    An absolute path or a `..` segment is refused without being read, and is
    never normalized into the project root.
    """
    candidate = Path(relative)
    if candidate.is_absolute() or ".." in candidate.parts:
        return False
    return (project_path / candidate).is_file()


def _read_bounded(path: Path, limit: int = 8192) -> str:
    try:
        with path.open("rb") as handle:
            return handle.read(limit).decode("utf-8", errors="replace")
    except OSError:
        return ""


def _declares_flutter(path: Path) -> bool:
    return "flutter" in _read_bounded(path).lower()


def _declares_ui_framework(path: Path) -> bool:
    text = _read_bounded(path).lower()
    return any(f'"{name}' in text for name in CLAIM_UI_FRAMEWORKS)


# ---------------------------------------------------------------------------
# Evaluation
# ---------------------------------------------------------------------------


def evaluate(policy: Policy, declaration: dict[str, Any] | None = None, *, governance_root: Path | None = None) -> Report:
    if policy.root_dir is None:
        raise ConfigError("policy is missing root_dir; call policy.with_root(...)")
    root_dir = policy.root_dir
    report = Report(schema_version=SCHEMA_VERSION, status="PASS", project=root_dir.name)

    # Resolve roots and exclusions once.
    try:
        resolved_roots = [_resolve_root(root_dir, raw) for raw in policy.product_roots]
    except ConfigError as exc:
        report.status = "REVIEW_REQUIRED"
        report.notes.append(f"configuration: {exc}")
        return report
    try:
        resolved_excludes = [_resolve_exclude(root_dir, raw) for raw in policy.exclude]
    except ConfigError as exc:
        report.status = "REVIEW_REQUIRED"
        report.notes.append(f"configuration: {exc}")
        return report

    for raw in policy.product_roots:
        candidate = root_dir / raw
        if not candidate.is_dir():
            report.status = "REVIEW_REQUIRED"
            report.notes.append(f"product_roots entry is not a directory: {raw}")
            return report

    files_scanned = 0
    files_unreadable = 0
    test_findings: list[Finding] = []
    placeholder_findings: list[Finding] = []

    for language in policy.languages:
        for root in resolved_roots:
            try:
                candidates = list(_candidate_files(root, language))
            except OSError as exc:
                report.status = "REVIEW_REQUIRED"
                report.notes.append(f"cannot enumerate {root}: {exc}")
                return report
            for path in candidates:
                try:
                    rel = path.relative_to(root_dir)
                except ValueError:
                    continue
                if _is_excluded(rel, list(policy.exclude), language):
                    report.summary["excluded_paths"] += 1
                    continue
                try:
                    text = path.read_text(encoding="utf-8")
                except (OSError, UnicodeDecodeError) as exc:
                    files_unreadable += 1
                    report.status = "REVIEW_REQUIRED"
                    report.notes.append(f"unreadable product file: {rel} ({exc})")
                    return report
                lines = text.splitlines()
                files_scanned += 1
                scanner = LANGUAGE_TEST_SCANNERS.get(language)
                if scanner is not None:
                    for finding in scanner(path, lines):
                        finding.file = rel.as_posix()
                        finding.marker = redact(finding.marker)
                        finding.remediation = redact(finding.remediation)
                        test_findings.append(finding)
                for finding in scan_placeholders(path, lines, policy.placeholder_markers):
                    finding.file = rel.as_posix()
                    finding.marker = redact(finding.marker)
                    finding.remediation = redact(finding.remediation)
                    placeholder_findings.append(finding)

    report.summary["files_scanned"] = files_scanned
    report.summary["files_unreadable"] = files_unreadable
    report.summary["test_code"] = len(test_findings)
    report.summary["placeholder"] = len(placeholder_findings)

    # Placeholder threshold: only counting WARNING+ERROR (i.e. all markers).
    if report.summary["placeholder"] > policy.placeholder_threshold:
        for finding in placeholder_findings:
            report.findings.append(finding)
        report.summary["errors"] += sum(1 for f in placeholder_findings if f.level == "ERROR")
        report.summary["warnings"] += sum(1 for f in placeholder_findings if f.level == "WARNING")
        report.status = "FAIL"
        report.notes.append(
            f"placeholder marker count {report.summary['placeholder']} exceeds threshold {policy.placeholder_threshold}"
        )
    else:
        # Below threshold: still surface warnings, but do not fail.
        for finding in placeholder_findings:
            if finding.level == "WARNING":
                report.findings.append(finding)
        report.summary["warnings"] += sum(1 for f in placeholder_findings if f.level == "WARNING")
        report.notes.append(
            f"placeholder marker count {report.summary['placeholder']} at or below threshold {policy.placeholder_threshold}"
        )

    if test_findings:
        for finding in test_findings:
            report.findings.append(finding)
        report.summary["errors"] += len(test_findings)
        report.status = "FAIL"
        report.notes.append(f"{len(test_findings)} test-code findings in product scope")

    # The README status-claim rule participates in the same report
    # (spec: "Emit bounded machine-readable findings"). It is claim-triggered and
    # configuration-free, so it runs here for the standalone CLI exactly as it
    # runs for the fleet audit. A contradicted claim is FAIL; a claim the
    # checker cannot scope is REVIEW_REQUIRED and never a pass.
    try:
        claim_report = evaluate_claims(root_dir, declaration, governance_root=governance_root)
    except (ConfigError, VocabularyError) as exc:
        report.status = "REVIEW_REQUIRED"
        report.notes.append(f"readme-status-claim cannot run: {exc}")
    else:
        for item in claim_report.findings:
            item.marker = redact(item.marker)
            item.remediation = redact(item.remediation)
            report.findings.append(item)
        report.summary["readme_claim_contradicted"] = len(claim_report.contradicted)
        report.summary["readme_claim_unscoped"] = len(claim_report.gaps)
        if claim_report.contradicted:
            report.summary["errors"] += len(claim_report.contradicted)
            report.status = "FAIL"
            report.notes.extend(claim_report.notes)
        for gap in claim_report.gaps:
            report.status = "REVIEW_REQUIRED"
            report.notes.append(f"readme-status-claim (REVIEW_REQUIRED, checker coverage): {gap.describe()}")

    if report.status == "PASS":
        report.notes.append("product-code quality check passed")

    return report


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def _load_declaration(path: Path) -> dict[str, Any]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ConfigError(f"cannot read declaration: {path} ({exc})")
    try:
        value = json.loads(text)
    except json.JSONDecodeError as exc:
        raise ConfigError(f"declaration is not valid JSON: {exc}")
    if not isinstance(value, dict):
        raise ConfigError("declaration must be a JSON object")
    return value


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("project", type=Path, help="project directory containing .project.json")
    parser.add_argument("--declaration", type=Path, default=None, help="explicit declaration file (defaults to <project>/.project.json)")
    parser.add_argument("--policy", type=Path, default=None, help="explicit policy JSON file (overrides declaration.quality)")
    parser.add_argument("--json", action="store_true", help="emit a versioned JSON report on stdout")
    args = parser.parse_args(argv)

    project_dir = args.project.resolve()
    if not project_dir.is_dir():
        print(f"project directory not found: {project_dir}", file=sys.stderr)
        return 2
    try:
        if args.policy is not None:
            policy_raw = json.loads(args.policy.read_text(encoding="utf-8"))
            if not isinstance(policy_raw, dict):
                raise ConfigError("policy file must contain a JSON object")
            policy = parse_policy(policy_raw)
            declaration_for_claims: dict[str, Any] | None = None
        else:
            declaration_path = args.declaration if args.declaration else project_dir / ".project.json"
            declaration_for_claims = _load_declaration(declaration_path)
            policy = load_policy(declaration_for_claims)
    except (ConfigError, VocabularyError) as exc:
        report = Report(
            schema_version=SCHEMA_VERSION,
            status="REVIEW_REQUIRED",
            project=project_dir.name,
        )
        report.notes.append(f"configuration: {exc}")
        if args.json:
            print(json.dumps(report.to_json(), indent=2, sort_keys=True))
        else:
            print(f"REVIEW_REQUIRED: {exc}")
        return 2

    policy = policy.with_root(project_dir)
    report = evaluate(policy, declaration_for_claims)
    if args.json:
        print(json.dumps(report.to_json(), indent=2, sort_keys=True))
    else:
        for note in report.notes:
            print(f"{report.status}: {note}")
        for finding in report.findings:
            print(f"  {finding.level} {finding.rule_id} {finding.file}:{finding.line} {finding.marker}")
    if report.status == "FAIL":
        return 1
    if report.status == "REVIEW_REQUIRED":
        return 2
    return 0


for _part in (_scope, _claims):
    _part.__dict__.update({name: value for name, value in globals().items() if not name.startswith("__")})


if __name__ == "__main__":
    sys.exit(main())
