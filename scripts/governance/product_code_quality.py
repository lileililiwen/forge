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


def _candidate_files(root: Path, language: str) -> Iterable[Path]:
    extensions = LANGUAGE_EXTENSIONS.get(language, ())
    if not extensions:
        # generic: every regular file
        for path in sorted(root.rglob("*")):
            try:
                if path.is_file() and not path.is_symlink():
                    yield path
            except OSError:
                continue
        return
    for ext in extensions:
        for path in sorted(root.rglob(f"*{ext}")):
            try:
                if path.is_file() and not path.is_symlink():
                    yield path
            except OSError:
                continue


# ---------------------------------------------------------------------------
# Test-code detection
# ---------------------------------------------------------------------------


# Rust module-tree awareness.
#
# The lexical pass below answers one question: "is this item lexically under a
# `#[cfg(test)]` attribute?". Rust answers a second, larger one: "is this file
# itself only ever compiled for tests?", and the answer lives in the module
# tree, not in the file. A file `x.rs` is reached only if some `mod.rs`,
# `lib.rs` or `main.rs` declares `mod x;`, so a declaration carrying
# `#[cfg(test)]` means the crate never compiles the file in a product build.
#
# Reading that declaration is the whole fix. A file-name list would be the
# alternative, and it is the wrong one for two reasons: `tests.rs` in a product
# root is the convention, so any name-based exemption silently excuses real
# test code the moment somebody uses the conventional name, and an `exclude`
# hides a finding instead of reporting one.
RUST_DECLARING_FILES = ("mod.rs", "lib.rs", "main.rs")

# `pub mod x;`, `pub(crate) mod x;`, `pub(super) mod x;`, `mod x;`
_RUST_MOD_DECL_RE = re.compile(
    r"^\s*(?:pub\s*(?:\([^)]*\)\s*)?)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;"
)
# `#[cfg(test)]` exactly. `cfg(any(test, ...))` and `cfg(all(test, ...))` do
# not match, which is the point: an ambiguous guard is not a test guard.
_RUST_CFG_TEST_RE = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")
_RUST_PATH_ATTR_RE = re.compile(r'#\s*\[\s*path\s*=\s*"([^"]+)"\s*\]')
# Attribute and comment lines. A `#[cfg(test)]` attaches to the next *item*, so
# these lines carry a pending guard instead of ending it. `#[cfg(test)]` alone
# on a line is not a block and opens no scope of its own.
_RUST_ATTR_OR_COMMENT_RE = re.compile(r"^\s*(#\[|//|/\*|\*)")


def _rust_item_is_test_scoped(lines: list[str]) -> set[int]:
    """Return the 1-based line numbers a `#[cfg(test)]` guard covers.

    The guard is tracked per *item*, not per line, because that is what Rust
    means. `#[cfg(test)]`, then `#[path = "tests.rs"]`, then `mod tests;` is one
    guarded item: the `#[path]` line sits between the attribute and the item it
    applies to, and treating it as the end of the guard is what made a correctly
    declared test module look like product test code.
    """
    scoped: set[int] = set()
    guarded_depths: list[int] = []  # brace depths of open cfg(test) blocks
    pending = False  # a cfg(test) attribute is waiting for its item
    brace_depth = 0
    for index, line in enumerate(lines, start=1):
        stripped = line.strip()
        if not stripped:
            continue
        in_guarded_block = bool(guarded_depths) and brace_depth > guarded_depths[-1]
        if in_guarded_block:
            scoped.add(index)
        if _RUST_ATTR_OR_COMMENT_RE.match(stripped):
            if _RUST_CFG_TEST_RE.search(stripped):
                pending = True
            continue
        if pending or in_guarded_block:
            scoped.add(index)
        if pending and stripped.endswith("{"):
            guarded_depths.append(brace_depth)
        pending = False
        brace_depth += stripped.count("{") - stripped.count("}")
        while guarded_depths and brace_depth <= guarded_depths[-1]:
            guarded_depths.pop()
    return scoped


def _rust_declared_test_modules(lines: list[str]) -> dict[str, bool]:
    """Map each module name in `lines` to whether it is declared under cfg(test).

    Also records the module under the file name a `#[path = "..."]` attribute
    points at, so a declaration whose path differs from its name — the shape
    `rust-ecommerce/src/shipment/mod.rs` uses — is still found.
    """
    declared: dict[str, bool] = {}
    pending_path: str | None = None
    pending_test = False
    for line in lines:
        stripped = line.strip()
        if _RUST_ATTR_OR_COMMENT_RE.match(stripped):
            match = _RUST_PATH_ATTR_RE.search(stripped)
            if match:
                pending_path = match.group(1).replace("\\", "/")
            if _RUST_CFG_TEST_RE.search(stripped):
                pending_test = True
            continue
        decl = _RUST_MOD_DECL_RE.match(stripped)
        if decl is not None:
            declared.setdefault(decl.group(1), pending_test)
        if pending_path is not None:
            declared.setdefault(pending_path, pending_test)
        pending_path = None
        pending_test = False
    return declared


def _rust_module_keys(path: Path) -> tuple[str, ...]:
    """The names by which a declaring file may reach `path`."""
    directory = path.parent
    if path.name == "mod.rs":
        # A directory module is declared from the parent directory.
        return (directory.name,)
    return (path.stem,)


def _rust_declaration_dirs(path: Path) -> tuple[Path, ...]:
    """The directories a `mod <name>;` declaration reaching `path` may sit in."""
    dirs: list[Path] = [path.parent]
    if path.name == "mod.rs":
        dirs.append(path.parent.parent)
    return tuple(d for d in dirs if d.is_dir())


@lru_cache(maxsize=512)
def _rust_declaring_text(path_str: str, _mtime_ns: int, _size: int) -> str:
    """Read a declaring file once per unchanged revision.

    The cache key carries the mtime and size, so a file that changes is read
    again and the cache cannot serve a stale module tree. It exists because one
    `mod.rs` is consulted by every sibling `.rs` file, and a fleet scan reads the
    same handful of declaring files thousands of times.
    """
    return Path(path_str).read_text(encoding="utf-8")


def _rust_declared_under_cfg_test(path: Path) -> bool:
    """Whether the crate declares this file's module under `#[cfg(test)]`.

    Reads only the declaring files that Rust itself would consult: a `mod.rs`,
    `lib.rs` or `main.rs` beside the file, and for a directory module the
    parent directory's `mod.rs`. A missing, unreadable or symlinked declaring
    file authorizes nothing, so the file stays in product scope and its findings
    stand — the fail-closed direction.
    """
    keys = _rust_module_keys(path)
    for directory in _rust_declaration_dirs(path):
        for declaring_name in RUST_DECLARING_FILES:
            declaring = directory / declaring_name
            if declaring == path:
                continue
            try:
                if declaring.is_symlink() or not declaring.is_file():
                    continue
                stat = declaring.stat()
                text = _rust_declaring_text(str(declaring), stat.st_mtime_ns, stat.st_size)
            except (OSError, UnicodeDecodeError):
                continue
            declared = _rust_declared_test_modules(text.splitlines())
            if any(declared.get(key) for key in keys):
                return True
    return False


def _scan_rust_test_code(path: Path, lines: list[str]) -> list[Finding]:
    """Detect `#[test]` or `mod tests` outside `#[cfg(test)]` test scope.

    Two scopes are honoured, and both come from the crate rather than from a
    file-name guess: the lexical one, where a `#[cfg(test)]` attribute covers
    the item it is written above, and the module-tree one, where the crate
    declares this file's module under `#[cfg(test)]` and so never compiles it
    outside a test build.
    """
    if _rust_declared_under_cfg_test(path):
        return []
    findings: list[Finding] = []
    scoped = _rust_item_is_test_scoped(lines)
    for index, line in enumerate(lines, start=1):
        if index in scoped:
            continue
        stripped = line.strip()
        if re.search(r"#\s*\[\s*test\s*\]", stripped):
            findings.append(Finding(
                rule_id="product-test-code",
                level="ERROR",
                severity="FAIL",
                file=path.name,
                line=index,
                marker=stripped,
                remediation="move test code into a `#[cfg(test)]` module or into the project tests/ root",
            ))
        if re.match(r"mod\s+tests\b", stripped):
            findings.append(Finding(
                rule_id="product-test-code",
                level="ERROR",
                severity="FAIL",
                file=path.name,
                line=index,
                marker=stripped,
                remediation="place `mod tests` under a `#[cfg(test)]` attribute or move it to the project tests/ root",
            ))
    return findings


def _scan_python_test_code(path: Path, lines: list[str]) -> list[Finding]:
    findings: list[Finding] = []
    for index, line in enumerate(lines, start=1):
        stripped = line.strip()
        if stripped.startswith("import unittest") or stripped.startswith("from unittest"):
            findings.append(Finding(
                rule_id="product-test-code",
                level="ERROR",
                severity="FAIL",
                file=path.name,
                line=index,
                marker=stripped,
                remediation="move unittest imports into a test_* file under the project tests/ root",
            ))
        if re.search(r"\bimport\s+pytest\b", stripped) or re.search(r"\bfrom\s+pytest\b", stripped):
            findings.append(Finding(
                rule_id="product-test-code",
                level="ERROR",
                severity="FAIL",
                file=path.name,
                line=index,
                marker=stripped,
                remediation="move pytest imports into a test_* file under the project tests/ root",
            ))
        if re.match(r"def\s+test_\w+", stripped) and not path.name.startswith("test_") and not path.name.endswith("_test.py"):
            findings.append(Finding(
                rule_id="product-test-code",
                level="ERROR",
                severity="FAIL",
                file=path.name,
                line=index,
                marker=stripped,
                remediation="rename the file to test_*.py or move the function into a test file under tests/",
            ))
    return findings


# A `describe`/`it`/`test` **call**, not a method of that name. The bare
# function is a Mocha/Jest/Vitest global; `.test(` is overwhelmingly a regular
# expression or a string being matched, which is the single largest source of
# false positives this scanner had:
#
#   if (/attestation/i.test(err.message))            <- a regex match
#   return /iphone|android/.test(ua);                 <- a regex match
#   } else if (QUOTA_PATTERN.test(body)) {            <- a compiled regex
#   return "...".test(value);                         <- a string method
#
# None of those is a test, and all four were reported as test code in product
# scope. The qualifier is what separates them: a call on the bare identifier is
# the framework global, and a call on any expression is a method on that
# expression. A method named `test` is a method, whatever it is called on.
_TS_TEST_CALL = re.compile(
    r"(?<![\w.$])(describe|it|test)\s*\(",
)
# `foo.test(` and `foo?.test(` — a member call. Excluded so the lookbehind
# above does not have to reason about the optional-chaining operator.
_TS_MEMBER_CALL = re.compile(r"[\w$?)\]]\s*(?:\?\.)?\s*\.\s*(describe|it|test)\s*\(")


def _scan_typescript_test_code(path: Path, lines: list[str]) -> list[Finding]:
    """Detect a bare `describe`/`it`/`test` call outside test scope.

    The same false-positive class the Rust scanner had, in a different
    language: a two-token lexical match cannot tell a framework global from a
    method that happens to share its name. So the match is anchored on the
    qualifier, which is the only thing that distinguishes them.

    A comment mentioning a `*.test.ts` file is not a call either, and is not
    reported — it is prose, and the remediation this rule exists to give
    ("move it into a test file") makes no sense for a doc comment.
    """
    findings: list[Finding] = []
    for index, line in enumerate(lines, start=1):
        stripped = line.strip()
        if "import" in stripped:
            continue
        # Prose about tests, not a call. A `//` or `*` or `/*` prefix means the
        # line is a comment, and a comment cannot call anything.
        if re.match(r"(//|/\*|\*)", stripped):
            continue
        if not _TS_TEST_CALL.search(stripped):
            continue
        # Strip member calls and re-test: what remains is a call on a bare
        # identifier, which is the framework global.
        without_members = _TS_MEMBER_CALL.sub("", stripped)
        if not _TS_TEST_CALL.search(without_members):
            continue
        findings.append(Finding(
            rule_id="product-test-code",
            level="ERROR",
            severity="FAIL",
            file=path.name,
            line=index,
            marker=stripped,
            remediation="move describe/it/test calls into a *.test.ts or *.spec.ts file under the project tests/ root",
        ))
    return findings


def _scan_csharp_test_code(path: Path, lines: list[str]) -> list[Finding]:
    findings: list[Finding] = []
    for index, line in enumerate(lines, start=1):
        stripped = line.strip()
        for attr in ("[Test]", "[TestMethod]", "[Fact]", "[Theory]"):
            if attr in stripped:
                findings.append(Finding(
                    rule_id="product-test-code",
                    level="ERROR",
                    severity="FAIL",
                    file=path.name,
                    line=index,
                    marker=stripped,
                    remediation="move the test method into a separate test project",
                ))
                break
    return findings


def _scan_generic_test_code(path: Path, lines: list[str]) -> list[Finding]:
    findings: list[Finding] = []
    lower_name = path.name.lower()
    if "test" in lower_name and (lower_name.startswith("test_") or lower_name.endswith("_test") or ".test." in lower_name or ".spec." in lower_name):
        findings.append(Finding(
            rule_id="product-test-code",
            level="ERROR",
            severity="FAIL",
            file=path.name,
            line=1,
            marker=path.name,
            remediation="rename or move the file out of the product root",
        ))
    return findings


LANGUAGE_TEST_SCANNERS = {
    "rust": _scan_rust_test_code,
    "python": _scan_python_test_code,
    "typescript": _scan_typescript_test_code,
    "javascript": _scan_typescript_test_code,
    "csharp": _scan_csharp_test_code,
    "go": _scan_generic_test_code,
    "generic": _scan_generic_test_code,
}


# ---------------------------------------------------------------------------
# Placeholder scanning
# ---------------------------------------------------------------------------


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


if __name__ == "__main__":
    sys.exit(main())
