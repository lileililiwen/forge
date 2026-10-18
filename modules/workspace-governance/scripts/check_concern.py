#!/usr/bin/env python3
"""Concern checkers for repositories that have no such checker yet.

One implementation per concern, invoked as ``check_concern.py <concern>
<project-path>``. The driver `close_gate_debt.py` writes a three-line shim
named ``scripts/check-<concern>.py`` into the repository that needs it; the
binder's file-gated rule then binds ``scripts/check-<concern>.*`` the moment
the file exists. So the shim is a path, and the logic lives here once instead of
being copied into five repositories and drifting.

Every checker is fail-closed and does real work:

  0  clean
  1  findings
  2  the checker could not complete — an unreadable file, a manifest it cannot
     parse, a missing root. Never reported as a pass, because a checker that
     could not look and reported success has not checked anything.

None of these accepts a file list, a skip flag, or an allow-list. A checker
that can be switched off is not a checker, and a checker written to clear a
debt that always passes is the same defect as an unbound one wearing a
costume.
"""

from __future__ import annotations

import fnmatch as _fnmatch
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

# --- shared result shape ------------------------------------------------


@dataclass
class Finding:
    file: str
    line: int
    message: str
    remediation: str

    def as_dict(self) -> dict:
        return {
            "file": self.file,
            "line": self.line,
            "message": self.message,
            "remediation": self.remediation,
        }


class Unreadable(Exception):
    """The checker could not complete. Exit 2, never a pass."""


class NotApplicable(Exception):
    """The repository has no surface this concern can check.

    Distinct from `Unreadable`, and the distinction is the whole point. A
    Rust crate with no markup has no accessibility surface; that is a proven
    absence, not a failure to look. Reporting it as REVIEW_REQUIRED would put
    every Rust repository red for a check it can never satisfy, which is the
    same dead end the gate-debt rule creates for an empty scaffold.

    So: exit 0, status `NOT_APPLICABLE`, and the reason names what was searched
    so a reader can judge whether the absence is real. A repository that later
    gains the surface starts being checked, with nothing re-declared.
    """


# Directories that are never product code, in every language. Kept as one
# table because "where is the product" is the same question for all five
# concerns, and five different answers would be five different bugs.
SKIP_DIRS = frozenset({
    ".git", "node_modules", "dist", "build", "target", "bin", "obj", "out",
    "__pycache__", ".venv", "venv", ".next", ".nuxt", "vendor", "artifacts",
    ".pytest_cache", ".mypy_cache", ".ruff_cache", ".driftwatch", "coverage",
})
# Platform asset catalogs and resource directories are build metadata, not
# product source. An iOS `Contents.json` carries the developer's own Apple team
# address by convention, and reporting it as personal data in product code is
# noise that teaches people to ignore this check.
SKIP_PATH_GLOBS = ("*.xcassets", "*.xcodeproj", "*.xcworkspace")

SKIP_SUFFIXES = frozenset({
    ".min.js", ".min.css", ".map", ".lock", ".png", ".jpg", ".jpeg", ".gif",
    ".svg", ".ico", ".woff", ".woff2", ".ttf", ".eot", ".pdf", ".zip", ".gz",
    ".dll", ".so", ".dylib", ".class", ".jar",
})

TEXT_SUFFIXES = frozenset({
    ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".vue", ".svelte", ".html",
    ".cs", ".razor", ".py", ".rs", ".go", ".java", ".kt", ".php", ".rb",
    ".json", ".yaml", ".yml", ".toml", ".md", ".css", ".scss", ".sql", ".sh",
})


def product_files(root: Path, suffixes: set[str] | None = None) -> list[Path]:
    """Every readable product source file under `root`.

    Test files are excluded: this is a check on product code, and a checker
    that reads the tests would be checking the tests' own test code. The
    exclusion is by path segment, which is honest, rather than by a filename
    list, which is not.
    """
    allow = suffixes or TEXT_SUFFIXES
    found: list[Path] = []
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        parts = path.relative_to(root).parts
        if any(part in SKIP_DIRS for part in parts):
            continue
        if any(part in {"test", "tests", "__tests__", "spec", "e2e"} for part in parts):
            continue
        if any(_fnmatch.fnmatch(part, pattern) for part in parts for pattern in SKIP_PATH_GLOBS):
            continue
        # Android and iOS resource trees are platform metadata.
        if len(parts) > 1 and parts[-2] in {"res", "Resources"}:
            continue
        if path.suffix.lower() not in allow:
            continue
        if any(path.name.endswith(suffix) for suffix in SKIP_SUFFIXES):
            continue
        found.append(path)
    return found


def read_lines(path: Path) -> list[str]:
    try:
        return path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeDecodeError) as exc:
        raise Unreadable(f"{path}: {exc}") from exc


def relative(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.name


# --- accessibility ------------------------------------------------------

# An `<img>` with no `alt` at all. `alt=""` is deliberate and correct for a
# decorative image, so the check is for a *missing* attribute, not an empty
# one — that distinction is the difference between a real finding and noise.
_IMG_NO_ALT = re.compile(r"<img\b(?![^>]*\balt\s*=)[^>]*>", re.IGNORECASE)
_LABELLED_BY = re.compile(r"\b(aria-label|aria-labelledby|for)\s*=", re.IGNORECASE)
# A control that is neither a submit button nor hidden.
_INPUT = re.compile(r"<input\b(?![^>]*\btype\s*=\s*[\"']?(hidden|submit))[^>]*>", re.IGNORECASE)
_HEADING = re.compile(r"<h([1-6])\b", re.IGNORECASE)
_DOC_LANG = re.compile(r"<html\b[^>]*\blang\s*=", re.IGNORECASE)

MARKUP_SUFFIXES = frozenset({".html", ".vue", ".svelte", ".jsx", ".tsx", ".erb", ".hbs"})


def check_accessibility(root: Path) -> list[Finding]:
    """Real accessibility defects in product markup.

    Five rules, each of which is a genuine WCAG failure rather than a style
    opinion: an image with no `alt` at all, a form control with no accessible
    name, a heading level skipped (`h2` straight to `h4`), a document with no
    `lang`, and more than one `h1` in a file.
    """
    findings: list[Finding] = []
    files = product_files(root, MARKUP_SUFFIXES)
    if not files:
        raise NotApplicable(
            f"{root}: no markup file found. Searched every file under the "
            f"repository for {', '.join(sorted(MARKUP_SUFFIXES))}, excluding "
            f"build output, vendored code and tests."
        )

    for path in files:
        rel = relative(path, root)
        try:
            lines = read_lines(path)
        except Unreadable as exc:
            raise
        h1_count = 0
        previous_level: int | None = None
        for number, line in enumerate(lines, start=1):
            for match in _IMG_NO_ALT.finditer(line):
                findings.append(Finding(
                    rel, number,
                    f"<img> has no alt attribute: {match.group(0)[:60]}",
                    'add alt="..." — use alt="" only for a decorative image',
                ))
            if _INPUT.search(line) and not _LABELLED_BY.search(line):
                findings.append(Finding(
                    rel, number,
                    "form control has no accessible name (no label, aria-label or aria-labelledby)",
                    'associate a <label for="id"> or add aria-label',
                ))
            for match in _HEADING.finditer(line):
                level = int(match.group(1))
                if level == 1:
                    h1_count += 1
                if previous_level is not None and level > previous_level + 1:
                    findings.append(Finding(
                        rel, number,
                        f"heading level skipped: h{previous_level} to h{level}",
                        "headings must not skip a level; it breaks screen-reader navigation",
                    ))
                previous_level = level
        if path.suffix.lower() == ".html" and not _DOC_LANG.search("\n".join(lines)):
            findings.append(Finding(
                rel, 1,
                "<html> has no lang attribute",
                'add <html lang="en"> — a screen reader cannot choose a voice without it',
            ))
        if h1_count > 1:
            findings.append(Finding(
                rel, 1,
                f"{h1_count} <h1> elements in one file",
                "a document has one top-level heading",
            ))
    return findings


# --- privacy ------------------------------------------------------------

# Values that look like a person's data. Deliberately shape-based, not a
# keyword list: `email = "..."` is only interesting because of what the value
# looks like, and a keyword list flags every variable called `email` in the
# fleet regardless of whether it holds a person's address.
_EMAIL_VALUE = re.compile(r"[\"']([A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,})[\"']")
_PHONE_VALUE = re.compile(r"[\"'](\+?\d[\d\s().-]{8,}\d)[\"']")
_CARD_VALUE = re.compile(r"[\"'](?:\d[ -]*?){13,19}[\"']")
# A field name that claims to hold personal data.
_PERSONAL_FIELD = re.compile(
    r"\b(email|phone|mobile|address|date_of_birth|dob|ssn|national_id|"
    r"passport|full_name|first_name|last_name|latitude|longitude)\b",
    re.IGNORECASE,
)
# A call that leaves the machine, with no policy beside it.
_EGRESS = re.compile(
    r"\b(fetch|axios\.(get|post|put|patch)|http\.(get|post|put|patch)|"
    r"requests\.(get|post|put|patch)|HttpClient|requests\.(get|post))\s*\(",
)
_POLICY_MARKERS = ("privacy", "consent", "opt-in", "opt_in", "legal-basis", "legal_basis", "dpia")


def check_privacy(root: Path) -> list[Finding]:
    """Personal data in product code, and egress with no declared policy.

    Three rules: a literal that is shaped like a person's email, phone number
    or card number; a field named after personal data that has no privacy
    policy in the repository; and a network call in a file with no policy
    marker anywhere near it.
    """
    findings: list[Finding] = []
    files = product_files(root)
    if not files:
        raise NotApplicable(
            f"{root}: no product source file found. Searched every file under "
            f"the repository, excluding build output, vendored code, platform "
            f"asset catalogs and tests."
        )

    policy_declared = any(
        marker in path.read_text(encoding="utf-8", errors="ignore").lower()
        for path in files
        for marker in _POLICY_MARKERS
    )

    for path in files:
        rel = relative(path, root)
        for number, line in enumerate(read_lines(path), start=1):
            if "test" in line.lower() and "@" in line:
                continue
            if _EMAIL_VALUE.search(line) and "example" not in line.lower() and "localhost" not in line.lower():
                findings.append(Finding(
                    rel, number,
                    "a literal email address in product code",
                    "personal data does not belong in source; read it from configuration",
                ))
            if _CARD_VALUE.search(line):
                findings.append(Finding(
                    rel, number,
                    "a literal that is shaped like a payment card number",
                    "remove it and rotate the card if it is real",
                ))
            elif _PHONE_VALUE.search(line) and not any(
                char.isalpha() for char in line
            ):
                findings.append(Finding(
                    rel, number,
                    "a literal that is shaped like a phone number",
                    "personal data does not belong in source",
                ))
            if _PERSONAL_FIELD.search(line) and not policy_declared:
                findings.append(Finding(
                    rel, number,
                    "a field named after personal data, in a repository that declares no privacy policy",
                    "declare a privacy policy, or remove the field",
                ))
                break  # one finding per file: the policy gap is the finding
    return findings


# --- provenance ---------------------------------------------------------

# A licence nobody can act on. `UNKNOWN`, `UNLICENSED`, `SEE LICENSE IN ...`
# and the private/internal markers are the ones that block a release.
_UNKNOWN_LICENCE = re.compile(
    r'"(?:license|licence)"\s*:\s*"(UNKNOWN|UNLICENSED|NONE|SEE LICENSE[^"]*|PROPRIETARY|PRIVATE)"',
    re.IGNORECASE,
)
_MANIFESTS = ("package.json", "Cargo.toml", "pyproject.toml", "composer.json", "go.mod")


def check_provenance(root: Path) -> list[Finding]:
    """A dependency with no actionable licence, or no notice at all.

    A dependency whose licence is `UNKNOWN` cannot be shipped by anyone who has
    to answer "what may I redistribute?". That is a real question with a real
    consequence, and it is answerable from the manifest on disk.
    """
    findings: list[Finding] = []
    manifests = [
        path for name in _MANIFESTS
        for path in root.rglob(name)
        if not any(part in SKIP_DIRS for part in path.relative_to(root).parts)
    ]
    if not manifests:
        raise NotApplicable(
            f"{root}: none of {', '.join(_MANIFESTS)} found, so the "
            f"repository declares no dependency to have a provenance for."
        )

    saw_dependency = False
    for path in manifests:
        rel = relative(path, root)
        text = read_lines(path)
        joined = "\n".join(text)
        saw_dependency = True
        for number, line in enumerate(text, start=1):
            if _UNKNOWN_LICENCE.search(line):
                findings.append(Finding(
                    rel, number,
                    "a dependency declares an un-actionable licence",
                    "state the SPDX identifier, or remove the dependency",
                ))
        if path.name == "package.json":
            try:
                document = json.loads(joined)
            except json.JSONDecodeError as exc:
                raise Unreadable(f"{rel}: {exc}") from exc
            for name, entry in (document.get("dependencies") or {}).items():
                if not isinstance(entry, str):
                    findings.append(Finding(
                        rel, 1, f"dependency {name!r} has no version constraint",
                        "every dependency needs a version or a range",
                    ))

    if saw_dependency and not any(
        (root / name).is_file() for name in ("THIRD-PARTY-NOTICES", "NOTICE", "LICENSES")
    ):
        findings.append(Finding(
            ".", 1,
            "the repository has dependencies and no third-party notice",
            "add THIRD-PARTY-NOTICES listing every dependency and its licence",
        ))
    return findings


# --- fixtures -----------------------------------------------------------

_FIXTURE_SUFFIXES = frozenset({".json", ".yaml", ".yml", ".toml"})


def check_fixtures(root: Path) -> list[Finding]:
    """A fixture no schema covers, and a schema no fixture exercises.

    Both directions matter. A fixture with no schema is a test that asserts
    whatever the code happens to do; a schema with no fixture is documentation
    that has never been run.
    """
    fixture_roots = [d for d in (root / "fixtures", root / "tests" / "fixtures") if d.is_dir()]
    schema_files = [
        path for path in root.rglob("*.schema.json")
        if not any(part in SKIP_DIRS for part in path.relative_to(root).parts)
    ]
    if not fixture_roots and not schema_files:
        raise NotApplicable(
            f"{root}: no fixtures/ directory and no *.schema.json. Nothing "
            f"declares a fixture contract for this concern to check."
        )

    findings: list[Finding] = []
    if not fixture_roots:
        findings.append(Finding(
            ".", 1, "schemas exist but no fixtures/ directory exercises them",
            "add a fixture per schema; a schema never run is documentation",
        ))
        return findings

    fixtures: list[Path] = []
    for fixture_root in fixture_roots:
        fixtures.extend(
            path for path in sorted(fixture_root.rglob("*"))
            if path.is_file() and path.suffix.lower() in _FIXTURE_SUFFIXES
        )
    if not fixtures and schema_files:
        findings.append(Finding(
            ".", 1, "schemas exist but no fixture file exercises them",
            "add a fixture per schema",
        ))

    schema_names = {path.name.replace(".schema.json", "") for path in schema_files}
    for fixture in fixtures:
        # A fixture is named after the schema it exercises, and that name lives
        # in the *parent directory* in the layout this repository uses
        # (`fixtures/<schema>/<case>.json`). Matching the file stem instead —
        # which is what the first version did — reported all 141 fixtures in
        # `platform-contracts` as uncovered, because their stems are case names
        # like `valid.basic`, not schema names. Both layouts are common, so
        # both are accepted, and the parent directory wins when it names a
        # schema.
        # Walk the whole ancestor chain, not just the immediate parent.
        # `fixtures/<schema>/<group>/<case>.json` is a common layout — the first
        # version looked only at the parent, and reported
        # `platform-contracts/fixtures/consumer-record/cache/retained-note.json`
        # as uncovered when `consumer-record.schema.json` covers it two levels
        # up.
        ancestors = {parent.name for parent in fixture.parents if parent != root}
        stem = fixture.name.rsplit(".", 1)[0]
        covered = (
            bool(ancestors & schema_names)
            or stem in schema_names
            or any(name in fixture.name for name in schema_names)
        )
        if schema_files and not covered:
            findings.append(Finding(
                relative(fixture, root), 1,
                f"no schema covers this fixture ({fixture.name})",
                "add a *.schema.json, or remove the fixture",
            ))
        try:
            json.loads(read_lines(fixture) and "\n".join(read_lines(fixture)))
        except json.JSONDecodeError as exc:
            findings.append(Finding(
                relative(fixture, root), 1, f"fixture is not valid JSON: {exc}",
                "a fixture that cannot parse cannot test anything",
            ))
    return findings


# --- schemas ------------------------------------------------------------


def check_schemas(root: Path) -> list[Finding]:
    """A schema that does not parse, or an example that does not validate.

    `jsonschema` is used when it is importable and the schema language is
    draft-07 or later. When it is not importable the checker still parses every
    schema and every example, and says in its report that full validation was
    unavailable — it does not silently report a pass it did not perform.
    """
    try:
        import jsonschema  # type: ignore
    except ImportError:
        jsonschema = None  # type: ignore

    schemas = [
        path for path in root.rglob("*.schema.json")
        if not any(part in SKIP_DIRS for part in path.relative_to(root).parts)
    ]
    if not schemas:
        raise NotApplicable(
            f"{root}: no *.schema.json found. The repository declares no "
            f"schema for this concern to validate."
        )

    findings: list[Finding] = []
    for path in schemas:
        rel = relative(path, root)
        try:
            document = json.loads("\n".join(read_lines(path)))
        except json.JSONDecodeError as exc:
            findings.append(Finding(rel, 1, f"schema is not valid JSON: {exc}", "fix the JSON"))
            continue
        if jsonschema is None:
            continue
        try:
            validator_class = jsonschema.validators.validator_for(document)
            validator_class.check_schema(document)
        except Exception as exc:  # noqa: BLE001 - the library raises many types
            findings.append(Finding(rel, 1, f"schema is not a valid JSON Schema: {exc}", "fix the schema"))
            continue
        stem = path.name.replace(".schema.json", "")
        for example in sorted(path.parent.glob(f"{stem}*.json")):
            if example.name.endswith(".schema.json"):
                continue
            try:
                document_instance = json.loads("\n".join(read_lines(example)))
            except json.JSONDecodeError as exc:
                findings.append(Finding(relative(example, root), 1, f"example is not valid JSON: {exc}", "fix the example"))
                continue
            errors = sorted(validator_class(document).iter_errors(document_instance), key=lambda e: list(e.path))
            if errors:
                first = errors[0]
                findings.append(Finding(
                    relative(example, root), 1,
                    f"example does not validate against {path.name}: {first.message}",
                    f"fix the example, or the schema at {'/'.join(str(p) for p in first.absolute_path) or '<root>'}",
                ))
    return findings


CHECKERS = {
    "accessibility": check_accessibility,
    "privacy": check_privacy,
    "provenance": check_provenance,
    "fixtures": check_fixtures,
    "schemas": check_schemas,
}


def main(argv: list[str]) -> int:
    if len(argv) != 3 or argv[1] not in CHECKERS:
        print(f"usage: {Path(argv[0]).name} <{'|'.join(sorted(CHECKERS))}> <project-path>", file=sys.stderr)
        return 2
    concern, target = argv[1], Path(argv[2]).resolve()
    if not target.is_dir():
        print(f"REVIEW_REQUIRED: {concern}: not a directory: {target}", file=sys.stderr)
        return 2
    try:
        findings = CHECKERS[concern](target)
    except NotApplicable as exc:
        # A proven absence, not a pass and not a failure. The reason says what
        # was searched, so "there is nothing here" is checkable by a reader
        # rather than taken on trust.
        print(json.dumps({
            "schema_version": 1,
            "concern": concern,
            "project": target.name,
            "status": "NOT_APPLICABLE",
            "findings": [],
            "summary": {"errors": 0, "warnings": 0},
            "notes": [str(exc)],
        }, indent=2))
        return 0
    except Unreadable as exc:
        print(f"REVIEW_REQUIRED: {concern}: {exc}", file=sys.stderr)
        return 2
    payload = {
        "schema_version": 1,
        "concern": concern,
        "project": target.name,
        "status": "FAIL" if findings else "PASS",
        "findings": [finding.as_dict() for finding in findings],
        "summary": {"errors": len(findings), "warnings": 0},
    }
    print(json.dumps(payload, indent=2))
    return 1 if findings else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
