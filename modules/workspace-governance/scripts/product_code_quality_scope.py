"""Product source-root discovery and test-code classification."""

from __future__ import annotations

import ast
import json
import re
from pathlib import Path
from typing import Iterable

import product_code_quality as _core

globals().update({name: value for name, value in vars(_core).items() if not name.startswith("__")})

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
