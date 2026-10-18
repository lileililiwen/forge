#!/usr/bin/env python3
"""UI convention conformance checker owned by Workspace Governance.

Answers one question per repository: does this repository consume the shared
UI conventions, or reimplement them?

The shared layer is `dotnet-platform-libs/ui`. Its adoption count across the
portfolio was zero when this checker landed, and nothing in Workspace
Governance could see that, because `product_code_quality.py` reads the
`quality` block and the `readme-status-claim` rule reads `README.md` — neither
looks at UI. A repository could satisfy both while shipping its own login form
and its own colour literals.

Exit codes
----------
0  conformant, or every finding is covered by the supplied baseline
1  blocking finding: at least one ERROR outside the baseline
2  configuration error, or a REVIEW_REQUIRED finding outside the baseline

The checker is fail-closed. A missing scan scope, a missing adoption
declaration, a missing branding document, a missing contrast artefact, an
unreadable file, a symlink, and a token source that cannot be consulted are all
REVIEW_REQUIRED. None of them is ever turned into a guessed pass, because a
gate that reports OK without performing a comparison is worse than no gate.

Scope split, and why it exists
------------------------------
The auth rule runs on a repo-wide census: `UI_AUTH_HANDROLLED` matches on a
path across 2067 files, and it is exactly the rule that must not be narrowed by
a declaration. It opens only the files whose *name* matched, which is 113 files
across the whole fleet, each bounded by `MAX_FILE_BYTES`. The other content
rules run only inside the repository's declared `ui.scope.roots`, because they
have to open every file. A repository that declares no scope therefore has those
rules silent by *declaration* rather than by exemption, and `UI_SCOPE_UNDECLARED`
fires so the silence is visible. Any repository over `ui.scope.max_files`
(default 400) is reported with its exact count and the ceiling it exceeded.

Adoption evidence, and what it is not
-------------------------------------
A filename pattern cannot tell a *migrated* login page from a *hand-rolled*
one, so the auth rule reads the file it matched and asks whether the file
demonstrably renders the shared layer: a `platform-*` class name, or a
stylesheet reference that names the shared layer. That is evidence read out of
the file, not a declaration read out of `.project.json` — a repository that
declares adoption and then hand-writes a login page is still reported, which is
the combination this checker exists for. Every file that goes unreported is
named in the notes together with the signal that was read, so a silence is
visible rather than silent.

Token resolution is NOT implemented here
----------------------------------------
`UI_COLOR_LITERAL` asks whether a literal resolves to a token. That repo owns
the semantics and implements them in `ui/scripts/lib/token-model.mjs`
(`readTokens` / `resolveAliases` / `declaredLiterals`). This module spawns one
bounded `node` process, calls that module, and compares against its output. It
re-implements neither the DTCG walk, nor the alias chain, nor the canonical
name rule. When the module cannot be consulted the checker fails closed and
names the cause; a literal is never declared token-reachable by a comparison
that did not run.

Read-only: it writes nothing inside a scanned project.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Iterable


SCHEMA_VERSION = 1

# ---------------------------------------------------------------------------
# The finding vocabulary. Fixed by the portfolio owner.
# ---------------------------------------------------------------------------

UI_AUTH_HANDROLLED = "UI_AUTH_HANDROLLED"
UI_TOKEN_NOT_CONSUMED = "UI_TOKEN_NOT_CONSUMED"
UI_COLOR_LITERAL = "UI_COLOR_LITERAL"
UI_THEME_ABSENT = "UI_THEME_ABSENT"
UI_BRANDING_ABSENT = "UI_BRANDING_ABSENT"
UI_CONTRAST_UNMEASURED = "UI_CONTRAST_UNMEASURED"
UI_SCOPE_UNDECLARED = "UI_SCOPE_UNDECLARED"

ALL_CODES = (
    UI_AUTH_HANDROLLED,
    UI_TOKEN_NOT_CONSUMED,
    UI_COLOR_LITERAL,
    UI_THEME_ABSENT,
    UI_BRANDING_ABSENT,
    UI_CONTRAST_UNMEASURED,
    UI_SCOPE_UNDECLARED,
)

BASELINE_FILE = "ui-conformance-baseline.json"

# ---------------------------------------------------------------------------
# The UI surface
# ---------------------------------------------------------------------------

UI_EXTENSIONS = frozenset({
    ".tsx", ".jsx", ".vue", ".svelte", ".css",
    ".scss", ".razor", ".cshtml", ".html", ".dart",
})

# Pruned at any depth. Universally non-source: build output, vendored
# dependencies, VCS metadata, coverage, framework caches.
SKIP_DIRS = frozenset({
    "target", "node_modules", "obj", "bin", "dist", "build",
    ".git", "__pycache__", "coverage", ".next",
})

# Matched against the FILE NAME only, never the directory path, so
# `src/authors/Article.tsx` does not fire on the `authors` segment.
AUTH_FILENAME_RE = re.compile(
    r"(login|logout|register|signup|sign-in|signin"
    r"|forgot.?password|reset.?password|auth)",
    re.IGNORECASE,
)

# ---------------------------------------------------------------------------
# Shared-layer evidence. String shapes over file content, in the same class as
# THEME_MECHANISM_RE below — not token semantics. Nothing here walks a token
# tree, follows an alias, or resolves a colour; that stays in token-model.mjs.
# ---------------------------------------------------------------------------

# The shared layer's class vocabulary. A file that uses one is speaking the
# shared composition. This is evidence of the NAME, not of the stylesheet that
# defines it: an adopting repository proves that half in its own suite, where a
# test asserts every `platform-*` name its pages use is defined by a stylesheet
# the shared feed emitted.
SHARED_CLASS_RE = re.compile(r"\bplatform-[a-z0-9]+(?:-[a-z0-9]+)*\b", re.IGNORECASE)

# A path that names the shared layer: an `@platform/*` package specifier, the
# vendored feed directory, the served `platform/` directory, or the token layer.
# Coarse on purpose, and shared with the existing `platform-*` vocabulary.
SHARED_LAYER_PATH_RE = re.compile(
    r"@platform/"      # npm package specifier
    r"|platform-ui/"   # the vendored feed directory
    r"|/platform/"     # a served vendored copy, e.g. ~/css/platform/
    r"|design-tokens/",  # the token layer itself
    re.IGNORECASE,
)

# A quoted stylesheet path. Combined with SHARED_LAYER_PATH_RE this covers both
# the vendored token stylesheet and a vendored shared-layer stylesheet: a path
# that names the layer and ends in `.css` is one or the other.
CSS_REFERENCE_RE = re.compile(r"""["'(]([^"'()\s]*\.css)["')]""", re.IGNORECASE)

# `<` opening a tag, comment or doctype: this file renders markup of its own.
# `a < b` and `count <3` do not match, so a comparison in a Razor code block
# reads as markup — the direction that keeps a finding rather than dropping one.
MARKUP_RE = re.compile(r"<[a-zA-Z!/?]")

# Only these two suffixes can be a handler page that renders no screen at all.
RAZOR_PAGE_SUFFIXES = frozenset({".cshtml", ".razor"})

LAYOUT_DIRECTIVE_RE = re.compile(r"""@layout\s*=\s*["']([^"']+)["']""", re.IGNORECASE)
VIEWSTART_LAYOUT_RE = re.compile(r"""Layout\s*=\s*["']([^"']+)["']""")

VIEWSTART_FILE = "_ViewStart.cshtml"
SHARED_LAYOUT_DIRECTORY = "Shared"

THEME_MECHANISM_RE = re.compile(
    r"\[data-theme\s*="          # attribute selector
    r"|prefers-color-scheme"     # media query
    r"|ThemeProvider"            # React provider
    r"|useTheme"                 # React hook
    r"|theme_provider"           # Flutter
    r"|ThemeData\b",             # Flutter Material
    re.IGNORECASE,
)

COLOR_LITERAL_RE = re.compile(
    r"#[0-9a-fA-F]{3}(?:[0-9a-fA-F]{1,5})?\b"   # #rgb .. #rrggbbaa
    r"|\brgba?\([^()]*\)"                        # rgb() / rgba()
    r"|\bhsla?\([^()]*\)",                       # hsl() / hsla()
    re.IGNORECASE,
)

# A literal spelling normalisation, not a colour semantic: `#FFF` and
# `#ffffff` are the same literal written two ways. No colour-space conversion,
# no perceptual distance, no similarity threshold.
_LONG_HEX_RE = re.compile(r"^#([0-9a-fA-F]{6})[0-9a-fA-F]{2}$")

DEFAULT_BRANDING_FILES = ("branding.json", "branding.yaml", "branding.yml")
DEFAULT_CONTRAST_FILES = ("contrast-audit.json", "ui-contrast.json")

# A committed constant, never an environment variable: a checker whose
# behaviour depends on an env var being set fails differently on a CI runner
# that lacks it (AGENTS.md, "Shared infrastructure").
DEFAULT_SCOPE_CEILING = 400
DEFAULT_NODE_TIMEOUT = 30

# Bounded reads. A file past the ceiling is recorded as a coverage gap, never
# as a clean file.
MAX_FILE_BYTES = 512 * 1024
MAX_SCOPE_FILES = 20_000
COLOR_FINDINGS_PER_FILE = 5


class ConfigError(ValueError):
    """Raised when configuration is missing, invalid, or cannot be applied."""


def _governance_root() -> Path:
    return Path(__file__).resolve().parents[1]


# ---------------------------------------------------------------------------
# Report shapes
# ---------------------------------------------------------------------------


@dataclass
class Finding:
    rule_id: str
    level: str  # ERROR | REVIEW_REQUIRED
    severity: str  # FAIL | REVIEW_REQUIRED
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
    summary: dict[str, int] = field(default_factory=lambda: {
        "ui_files": 0,
        "files_in_scope": 0,
        "findings": 0,
        "errors": 0,
        "review_required": 0,
        "baselined": 0,
        "unreadable": 0,
        "truncated": 0,
        "coverage_review": 0,
    })
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

    def codes(self) -> set[str]:
        return {f.rule_id for f in self.findings}


def _finding(
    rule_id: str,
    level: str,
    file: str,
    line: int,
    marker: str,
    remediation: str,
) -> Finding:
    return Finding(
        rule_id=rule_id,
        level=level,
        severity="FAIL" if level == "ERROR" else "REVIEW_REQUIRED",
        file=file,
        line=line,
        marker=marker[:400],
        remediation=remediation,
    )


# ---------------------------------------------------------------------------
# Census
# ---------------------------------------------------------------------------


def census(root: Path, skip_dirs: frozenset[str] = SKIP_DIRS) -> list[str]:
    """Every UI file under `root`, as sorted project-relative posix paths.

    One `os.walk` with in-place pruning rather than one `rglob` per extension:
    a single pass is what keeps a 2067-file repository cheap to enumerate.
    Symlinked files are skipped and symlinked directories are not followed —
    the fail-closed direction, because an unreadable path is a coverage gap
    and never a silent pass.
    """
    found: list[str] = []
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        dirnames[:] = sorted(
            d for d in dirnames
            if d not in skip_dirs and not (Path(dirpath) / d).is_symlink()
        )
        for name in filenames:
            if Path(name).suffix.lower() not in UI_EXTENSIONS:
                continue
            absolute = Path(dirpath) / name
            try:
                if absolute.is_symlink() or not absolute.is_file():
                    continue
                found.append(absolute.relative_to(root).as_posix())
            except (OSError, ValueError):
                continue
    return sorted(found)


# ---------------------------------------------------------------------------
# Policy parsing
# ---------------------------------------------------------------------------


@dataclass
class UiPolicy:
    scope_roots: tuple[str, ...] = ()
    scope_exclude: tuple[str, ...] = ()
    max_files: int = DEFAULT_SCOPE_CEILING
    adopted: bool = False
    consume_paths: tuple[str, ...] = ()
    branding_document: str | None = None
    contrast_evidence: str | None = None
    token_source: str | None = None

    @property
    def scope_declared(self) -> bool:
        return bool(self.scope_roots)


def _project_relative(raw: Any, field: str) -> str:
    if not isinstance(raw, str) or not raw:
        raise ConfigError(f"ui.{field} entries must be non-empty strings")
    if raw.startswith("/") or ".." in Path(raw).parts:
        raise ConfigError(f"ui.{field} entry must be project-relative: {raw!r}")
    return raw


def parse_ui_policy(declaration: dict[str, Any]) -> UiPolicy:
    """Read the additive `ui` block. Absent block means every rule undeclared.

    Nothing here infers a value. A repository that declares nothing gets the
    fail-closed findings, which is the whole design: the owner declares, the
    checker never guesses.
    """
    ui = declaration.get("ui")
    if ui is None:
        return UiPolicy()
    if not isinstance(ui, dict):
        raise ConfigError("ui must be an object")

    roots: tuple[str, ...] = ()
    exclude: tuple[str, ...] = ()
    ceiling = DEFAULT_SCOPE_CEILING
    scope = ui.get("scope")
    if scope is not None:
        if not isinstance(scope, dict):
            raise ConfigError("ui.scope must be an object")
        raw_roots = scope.get("roots", [])
        if not isinstance(raw_roots, list):
            raise ConfigError("ui.scope.roots must be a list of strings")
        roots = tuple(_project_relative(r, "scope.roots") for r in raw_roots)
        raw_exclude = scope.get("exclude", [])
        if not isinstance(raw_exclude, list):
            raise ConfigError("ui.scope.exclude must be a list of strings")
        exclude = tuple(_project_relative(r, "scope.exclude") for r in raw_exclude)
        raw_ceiling = scope.get("max_files", DEFAULT_SCOPE_CEILING)
        if not isinstance(raw_ceiling, int) or isinstance(raw_ceiling, bool) or raw_ceiling < 1:
            raise ConfigError("ui.scope.max_files must be a positive integer")
        ceiling = raw_ceiling

    adopted = False
    consume: tuple[str, ...] = ()
    shared = ui.get("shared_layer")
    if shared is not None:
        if not isinstance(shared, dict):
            raise ConfigError("ui.shared_layer must be an object")
        raw_adopted = shared.get("adopted", False)
        if not isinstance(raw_adopted, bool):
            raise ConfigError("ui.shared_layer.adopted must be a boolean")
        adopted = raw_adopted
        raw_consume = shared.get("consume_paths", [])
        if not isinstance(raw_consume, list):
            raise ConfigError("ui.shared_layer.consume_paths must be a list of strings")
        consume = tuple(_project_relative(r, "shared_layer.consume_paths") for r in raw_consume)

    def _declared(name: str) -> str | None:
        value = ui.get(name)
        if value is None:
            return None
        return _project_relative(value, name)

    return UiPolicy(
        scope_roots=roots,
        scope_exclude=exclude,
        max_files=ceiling,
        adopted=adopted,
        consume_paths=consume,
        branding_document=_declared("branding_document"),
        contrast_evidence=_declared("contrast_evidence"),
        token_source=_declared("token_source"),
    )


# ---------------------------------------------------------------------------
# Token resolution — the owning module is called, never re-implemented
# ---------------------------------------------------------------------------

# Six lines of bridge. The token semantics live in token-model.mjs; this only
# reads its `declaredLiterals()` output. The module is loaded by dynamic
# import because the path is an argument, not a literal specifier.
TOKEN_BRIDGE = (
    "const { readTokens, resolveAliases, declaredLiterals } = "
    "await import(process.argv[2]);\n"
    "const { readFileSync } = await import('node:fs');\n"
    "const source = JSON.parse(readFileSync(process.argv[3], 'utf8'));\n"
    "const literals = declaredLiterals(resolveAliases(readTokens(source)));\n"
    "process.stdout.write(JSON.stringify([...literals].filter((v) => typeof v === 'string')));\n"
)

DEFAULT_TOKEN_MODULE = "../dotnet-platform-libs/ui/scripts/lib/token-model.mjs"
DEFAULT_TOKEN_SOURCE = "../dotnet-platform-libs/ui/tokens/tokens.json"


class TokenSourceError(RuntimeError):
    """The owning token resolver could not be consulted. Fail closed."""


def normalise_literal(literal: str) -> str:
    """Lowercase, and expand 3-digit hex. Spelling, not colour semantics."""
    value = literal.strip().lower()
    if re.fullmatch(r"#[0-9a-f]{3}", value):
        return "#" + "".join(ch * 2 for ch in value[1:])
    return value


def _looks_like_colour(literal: str) -> bool:
    value = literal.strip().lower()
    return bool(
        re.fullmatch(r"#[0-9a-f]{3,8}", value)
        or re.fullmatch(r"(?:rgb|rgba|hsl|hsla)\([^()]*\)", value)
    )


def resolve_token_literals(
    token_source: Path,
    module_path: Path | None = None,
    timeout: int = DEFAULT_NODE_TIMEOUT,
) -> frozenset[str]:
    """The colour literals the token source declares, via the owning module.

    One bounded subprocess per run. Any failure — no `node`, no module, an
    unreadable source, an unresolved alias (which `resolveAliases` throws on),
    or a timeout — raises `TokenSourceError`, and the caller fails closed.
    """
    module = Path(module_path) if module_path else (_governance_root() / DEFAULT_TOKEN_MODULE).resolve()
    if not module.is_file():
        raise TokenSourceError(f"token model module not found: {module}")
    if not token_source.is_file():
        raise TokenSourceError(f"token source not found: {token_source}")
    try:
        completed = subprocess.run(
            ["node", "--input-type=module", "-e", TOKEN_BRIDGE,
             "bridge", module.as_uri(), str(token_source)],
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
    except FileNotFoundError as exc:
        raise TokenSourceError("node is not on PATH; the token model cannot be consulted") from exc
    except subprocess.TimeoutExpired as exc:
        raise TokenSourceError(f"the token model did not answer within {timeout}s") from exc
    if completed.returncode != 0:
        detail = (completed.stderr or "").strip().splitlines()
        reason = detail[0][:200] if detail else "no diagnostic"
        raise TokenSourceError(f"the token model failed: {reason}")
    try:
        literals = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise TokenSourceError(f"the token model returned unparseable output: {exc}") from exc
    if not isinstance(literals, list):
        raise TokenSourceError("the token model returned an unexpected shape")
    return frozenset(normalise_literal(v) for v in literals if _looks_like_colour(v))


# ---------------------------------------------------------------------------
# Declaration evidence
# ---------------------------------------------------------------------------


def _resolve_inside(root: Path, relative: str) -> Path:
    candidate = (root / relative).resolve(strict=False)
    base = root.resolve()
    if base != candidate and base not in candidate.parents:
        raise ConfigError(f"declared path escapes the project root: {relative!r}")
    return candidate


def _declared_or_default(root: Path, declared: str | None, defaults: Iterable[str]) -> Path | None:
    if declared is not None:
        candidate = _resolve_inside(root, declared)
        return candidate if candidate.is_file() and not candidate.is_symlink() else None
    for name in defaults:
        candidate = root / name
        if candidate.is_file() and not candidate.is_symlink():
            return candidate
    return None


def _in_scope(rel: str, policy: UiPolicy) -> bool:
    for excluded in policy.scope_exclude:
        if rel == excluded or rel.startswith(excluded.rstrip("/") + "/"):
            return False
    for root in policy.scope_roots:
        if root == "." or rel == root or rel.startswith(root.rstrip("/") + "/"):
            return True
    return False


def _read_text(path: Path) -> tuple[str | None, bool]:
    """Return `(text, truncated)`, or `(None, False)` when unreadable."""
    try:
        with path.open("rb") as handle:
            raw = handle.read(MAX_FILE_BYTES + 1)
    except OSError:
        return None, False
    truncated = len(raw) > MAX_FILE_BYTES
    if truncated:
        raw = raw[:MAX_FILE_BYTES]
    try:
        return raw.decode("utf-8", errors="replace"), truncated
    except (UnicodeDecodeError, MemoryError):
        return None, False


# ---------------------------------------------------------------------------
# Shared-layer evidence, read from the file the pattern matched
# ---------------------------------------------------------------------------


def _shows_shared_layer(text: str) -> str | None:
    """The shared-layer signal this file shows, or `None`.

    Returns the signal in words so the note can name what was actually read
    rather than asserting that a check passed.
    """
    if SHARED_CLASS_RE.search(text):
        return "a platform-* class name"
    for path in CSS_REFERENCE_RE.findall(text):
        if SHARED_LAYER_PATH_RE.search(path):
            return (
                "the vendored token stylesheet"
                if "design-tokens" in path.lower()
                else "a vendored shared-layer stylesheet"
            )
    return None


def _layout_file(name: str) -> str:
    value = name.strip().replace("\\", "/")
    return value if value.endswith(".cshtml") else f"{value}.cshtml"


def _first_existing(project_dir: Path, candidates: Iterable[Path]) -> str | None:
    """The first candidate that resolves to a real file inside the project.

    Every candidate goes through `_resolve_inside`, so a declared layout name
    can never reach outside the repository.
    """
    for candidate in candidates:
        relative = candidate.as_posix()
        if relative in (".", ""):
            continue
        try:
            resolved = _resolve_inside(project_dir, relative)
        except ConfigError:
            continue
        if resolved.is_file() and not resolved.is_symlink():
            return relative
    return None


def _nearest_viewstart(project_dir: Path, rel: str) -> str | None:
    """The Razor Pages root holding `rel`: the nearest ancestor with a
    `_ViewStart.cshtml`. `None` when the page is outside a Pages root."""
    directory = Path(rel).parent
    while True:
        root = directory.as_posix()
        if (project_dir / directory / VIEWSTART_FILE).is_file():
            return root
        if directory == Path("."):
            return None
        directory = directory.parent


def _composed_layout(project_dir: Path, rel: str, text: str) -> str | None:
    """The project-relative surface `rel` renders inside, or `None`.

    Razor's own order: an explicit `@layout` wins, otherwise the nearest
    `_ViewStart.cshtml` assigns one, resolved first at the Pages root and then
    under its `Shared/` directory. Nothing is inferred from the file name.
    """
    page = Path(rel).parent
    directive = LAYOUT_DIRECTIVE_RE.search(text)
    if directive:
        return _first_existing(project_dir, [page / _layout_file(directive.group(1))])

    root = _nearest_viewstart(project_dir, rel)
    if root is None:
        return None
    start_text, _ = _read_text(project_dir / root / VIEWSTART_FILE)
    if start_text is None:
        return None
    assigned = VIEWSTART_LAYOUT_RE.search(start_text)
    if assigned is None:
        return None
    layout = _layout_file(assigned.group(1))
    root_path = Path(root)
    return _first_existing(project_dir, [
        root_path / layout,
        root_path / SHARED_LAYOUT_DIRECTORY / layout,
    ])


def _rendered_by_shared_surface(
    project_dir: Path, rel: str, text: str, policy: UiPolicy
) -> str | None:
    """The shared-layer surface `rel` renders inside, or `None`.

    Three gates, all required: the repository declares adoption, the file is a
    Razor page, and the file renders no markup of its own. A page that renders a
    screen is never exempt here — a declared adopter that hand-writes a login
    page is exactly the finding this rule exists to raise.
    """
    if not policy.adopted:
        return None
    if Path(rel).suffix.lower() not in RAZOR_PAGE_SUFFIXES:
        return None
    if MARKUP_RE.search(text):
        return None
    layout = _composed_layout(project_dir, rel, text)
    if layout is None:
        return None
    layout_text, _ = _read_text(project_dir / layout)
    if layout_text is None:
        return None
    return layout if _shows_shared_layer(layout_text) is not None else None


# ---------------------------------------------------------------------------
# Evaluation
# ---------------------------------------------------------------------------


def evaluate(
    project_dir: Path,
    policy: UiPolicy,
    *,
    token_source: Path | None = None,
    module_path: Path | None = None,
    node_timeout: int = DEFAULT_NODE_TIMEOUT,
) -> Report:
    report = Report(
        schema_version=SCHEMA_VERSION,
        status="PASS",
        project=project_dir.name,
    )

    files = census(project_dir)
    report.summary["ui_files"] = len(files)
    if not files:
        report.notes.append("no UI surface: this repository is out of scope for ui-convention-check")
        return report

    # --- scope ------------------------------------------------------------
    over_ceiling = len(files) > policy.max_files
    if not policy.scope_declared:
        report.findings.append(_finding(
            UI_SCOPE_UNDECLARED, "REVIEW_REQUIRED", ".project.json", 0,
            f"no ui.scope.roots declared for a repository shipping {len(files)} UI file(s)",
            "declare ui.scope.roots in .project.json so the content rules have a scope to read",
        ))
    if over_ceiling:
        report.notes.append(
            f"UI surface is {len(files)} files, over the {policy.max_files}-file ceiling; "
            "filename rules still run over the whole census and content rules stay inside the declared scope"
        )

    # --- hand-rolled auth (filename pattern over the whole census) ---------
    # The pattern opens the files it matched, and no others. Adoption is read
    # from the declaration only as a gate on the composition test below; the
    # shared-layer test itself is read out of the file.
    for rel in files:
        if not AUTH_FILENAME_RE.search(Path(rel).name):
            continue
        if any(rel == p or rel.startswith(p.rstrip("/") + "/") for p in policy.consume_paths):
            continue
        signal: str | None = None
        text, _truncated = _read_text(project_dir / rel)
        if text is not None:
            signal = _shows_shared_layer(text)
            if signal is None:
                layout = _rendered_by_shared_surface(project_dir, rel, text, policy)
                if layout is not None:
                    signal = f"the shared surface it renders inside, {layout}"
        if signal is not None:
            report.notes.append(
                f"{rel} is not reported as hand-rolled: it shows the shared layer through {signal}"
            )
            continue
        report.findings.append(_finding(
            UI_AUTH_HANDROLLED, "ERROR", rel, 0,
            Path(rel).name,
            "delete this screen and consume the shared ui layer's auth package",
        ))

    # --- adoption ---------------------------------------------------------
    if not policy.adopted:
        report.findings.append(_finding(
            UI_TOKEN_NOT_CONSUMED, "ERROR", ".project.json", 0,
            "ui.shared_layer.adopted is not declared true",
            "declare ui.shared_layer.adopted with a consume_paths list, or record why the layer is declined",
        ))

    # --- branding and contrast (declaration-level) ------------------------
    branding = _declared_or_default(project_dir, policy.branding_document, DEFAULT_BRANDING_FILES)
    if not policy.adopted and branding is None:
        report.findings.append(_finding(
            UI_BRANDING_ABSENT, "ERROR", ".project.json", 0,
            "no branding document resolvable",
            "ship branding.json / branding.yaml or declare ui.branding_document",
        ))
    contrast = _declared_or_default(project_dir, policy.contrast_evidence, DEFAULT_CONTRAST_FILES)
    if not policy.adopted and contrast is None:
        report.findings.append(_finding(
            UI_CONTRAST_UNMEASURED, "ERROR", ".project.json", 0,
            "no measured-contrast artefact resolvable",
            "record a measured contrast run and declare ui.contrast_evidence",
        ))

    # --- content-level rules (declared scope only) ------------------------
    if policy.scope_declared:
        scoped = [rel for rel in files if _in_scope(rel, policy)]
        report.summary["files_in_scope"] = len(scoped)
        if len(scoped) > MAX_SCOPE_FILES:
            report.notes.append(
                f"declared scope holds {len(scoped)} UI files, over the {MAX_SCOPE_FILES}-file read ceiling; "
                "content rules read the first 5000 in sorted order"
            )
            scoped = scoped[:MAX_SCOPE_FILES]
        report.notes.append(
            f"content rules read {len(scoped)} of {len(files)} UI file(s): "
            "only files inside the declared ui.scope.roots are opened"
        )
        theme_seen = False
        token_error: str | None = None
        literals: frozenset[str] = frozenset()

        if token_source is not None:
            try:
                literals = resolve_token_literals(token_source, module_path, node_timeout)
            except TokenSourceError as exc:
                # Fail closed: every literal found is reported, and the run is
                # REVIEW_REQUIRED because the comparison did not run.
                token_error = str(exc)
        else:
            token_error = "no token source resolved"

        for rel in scoped:
            text, truncated = _read_text(project_dir / rel)
            if text is None:
                report.summary["unreadable"] += 1
                continue
            if truncated:
                report.summary["truncated"] += 1
            if not theme_seen and THEME_MECHANISM_RE.search(text):
                theme_seen = True

            hits = []
            for index, line in enumerate(text.splitlines(), start=1):
                for match in COLOR_LITERAL_RE.finditer(line):
                    hits.append((index, match.group(0)))
                if len(hits) >= COLOR_FINDINGS_PER_FILE * 20:
                    break
            unreached = [
                (index, literal)
                for index, literal in hits
                if normalise_literal(literal) not in literals
            ]
            for index, literal in unreached[:COLOR_FINDINGS_PER_FILE]:
                report.findings.append(_finding(
                    UI_COLOR_LITERAL, "ERROR", rel, index,
                    literal,
                    "replace the literal with a shared design-token reference",
                ))
            if len(unreached) > COLOR_FINDINGS_PER_FILE:
                report.notes.append(
                    f"{rel}: {len(unreached)} colour literal(s) not reachable from the token source; "
                    f"the first {COLOR_FINDINGS_PER_FILE} are reported"
                )

        if token_error is not None:
            report.summary["coverage_review"] += 1
            report.notes.append(
                f"UI_COLOR_LITERAL could not be compared against the token source: {token_error}; "
                "every colour literal in scope is reported as unreached (fail-closed)"
            )
        elif literals:
            report.notes.append(
                f"token source declares {len(literals)} colour literal(s); the comparison runs through "
                "dotnet-platform-libs/ui/scripts/lib/token-model.mjs, not a Python copy"
            )
        if not theme_seen and not policy.adopted:
            report.findings.append(_finding(
                UI_THEME_ABSENT, "ERROR", ".project.json", 0,
                f"no theme mechanism in {len(scoped)} scanned UI file(s)",
                "adopt the shared layer's ThemeProvider or declare a [data-theme] / prefers-color-scheme mechanism",
            ))
    else:
        report.notes.append(
            "content rules did not run: no ui.scope.roots declared, so no file was opened. "
            "This is a declared silence, not a pass — see UI_SCOPE_UNDECLARED"
        )

    report.summary["errors"] = sum(1 for f in report.findings if f.level == "ERROR")
    report.summary["review_required"] = sum(1 for f in report.findings if f.level == "REVIEW_REQUIRED")
    report.summary["findings"] = len(report.findings)
    if report.summary["errors"]:
        # A blocking finding outranks a coverage gap: both are non-zero, and
        # the gap is still named in the notes.
        report.status = "FAIL"
    elif report.summary["review_required"] or report.summary["coverage_review"] or report.summary["unreadable"]:
        report.status = "REVIEW_REQUIRED"
    return report


# ---------------------------------------------------------------------------
# Baseline
# ---------------------------------------------------------------------------


def load_baseline(path: Path) -> dict[str, frozenset[str]]:
    """`project -> finding codes covered at snapshot time`.

    A missing baseline file is an empty baseline, not an error: the default
    run must fail closed on every finding, and "no baseline supplied" is the
    fail-closed case.
    """
    if not path.is_file():
        return {}
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ConfigError(f"cannot read baseline {path}: {exc}") from exc
    if not isinstance(document, dict):
        raise ConfigError(f"baseline {path} must contain a JSON object")
    entries = document.get("entries", [])
    if not isinstance(entries, list):
        raise ConfigError(f"baseline {path}: entries must be a list")
    covered: dict[str, frozenset[str]] = {}
    for entry in entries:
        if not isinstance(entry, dict):
            raise ConfigError(f"baseline {path}: each entry must be an object")
        project = entry.get("project")
        codes = entry.get("codes")
        if not isinstance(project, str) or not isinstance(codes, list):
            raise ConfigError(f"baseline {path}: each entry needs a project and a codes list")
        covered[project] = frozenset(c for c in codes if isinstance(c, str))
    return covered


def apply_baseline(report: Report, covered: Iterable[str]) -> list[Finding]:
    """Remove covered findings, count them, name them. Everything else stays.

    The baseline downgrades a recorded finding; it never deletes the evidence.
    A finding whose code is not covered fails normally, which is what makes
    this a ratchet rather than a switch.
    """
    allowed = frozenset(covered)
    kept: list[Finding] = []
    suppressed: list[Finding] = []
    for finding in report.findings:
        (suppressed if finding.rule_id in allowed else kept).append(finding)
    report.findings = kept
    report.summary["baselined"] = len(suppressed)
    if suppressed:
        counts: dict[str, int] = {}
        for finding in suppressed:
            counts[finding.rule_id] = counts.get(finding.rule_id, 0) + 1
        detail = ", ".join(f"{code} x{counts[code]}" for code in sorted(counts))
        report.notes.append(
            f"baseline covers {len(suppressed)} pre-existing finding(s) here ({detail}); "
            "the entry is a recorded decision to look later, not a pass"
        )
    report.summary["findings"] = len(report.findings)
    report.summary["errors"] = sum(1 for f in report.findings if f.level == "ERROR")
    report.summary["review_required"] = sum(1 for f in report.findings if f.level == "REVIEW_REQUIRED")
    # Recomputed from what survived, so the baseline can only ever downgrade
    # toward PASS and never silently repair a FAIL into something softer than
    # the surviving findings justify.
    if report.summary["errors"]:
        report.status = "FAIL"
    elif report.summary["review_required"] or report.summary["coverage_review"] or report.summary["unreadable"]:
        report.status = "REVIEW_REQUIRED"
    else:
        report.status = "PASS"
    return suppressed


def exit_code(report: Report) -> int:
    if report.status == "FAIL":
        return 1
    if report.status == "REVIEW_REQUIRED":
        return 2
    return 0


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def _load_declaration(path: Path) -> dict[str, Any]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ConfigError(f"cannot read declaration: {path} ({exc})") from exc
    try:
        value = json.loads(text)
    except json.JSONDecodeError as exc:
        raise ConfigError(f"declaration is not valid JSON: {exc}") from exc
    if not isinstance(value, dict):
        raise ConfigError("declaration must be a JSON object")
    return value


def _print_report(report: Report, as_json: bool) -> None:
    if as_json:
        print(json.dumps(report.to_json(), indent=2, sort_keys=True))
        return
    for note in report.notes:
        print(f"  note {report.project}: {note}")
    for finding in report.findings:
        location = f"{finding.file}:{finding.line}" if finding.line else finding.file
        print(f"  {finding.level} {finding.rule_id} {location} {finding.marker}")
    print(
        f"{report.status}: {report.project} ui_files={report.summary['ui_files']} "
        f"errors={report.summary['errors']} review_required={report.summary['review_required']} "
        f"baselined={report.summary['baselined']}"
    )


def _fleet_roots(governance_root: Path, override: str | None = None) -> tuple[Path, list[dict[str, Any]]]:
    registry_path = governance_root / "projects.json"
    if not registry_path.is_file():
        raise ConfigError(f"registry not found: {registry_path}")
    document = _load_declaration(registry_path)
    projects = document.get("projects")
    if not isinstance(projects, list):
        raise ConfigError("projects.json: projects must be a list")
    if override:
        return Path(override).expanduser().resolve(), projects
    workspace_root = os.environ.get("WORKSPACE_ROOT") or document.get("workspace_root") or Path.cwd()
    return Path(str(workspace_root)).expanduser().resolve(), projects


def audit(
    governance_root: Path,
    baseline_path: Path,
    *,
    as_json: bool,
    token_source: Path | None,
    module_path: Path | None,
    node_timeout: int,
    root_override: str | None = None,
) -> int:
    """Run every registered project under the workspace root."""
    workspace_root, projects = _fleet_roots(governance_root, root_override)
    covered = load_baseline(baseline_path)
    reports: list[Report] = []
    worst = 0
    for entry in projects:
        if not isinstance(entry, dict) or not isinstance(entry.get("path"), str):
            continue
        project_dir = (workspace_root / entry["path"]).resolve()
        if not project_dir.is_dir():
            continue
        try:
            declaration = _load_declaration(project_dir / ".project.json")
            policy = parse_ui_policy(declaration)
        except ConfigError as exc:
            report = Report(schema_version=SCHEMA_VERSION, status="REVIEW_REQUIRED", project=entry["path"])
            report.notes.append(f"configuration: {exc}")
            reports.append(report)
            worst = max(worst, 2)
            continue
        if not census(project_dir):
            continue
        report = evaluate(
            project_dir, policy,
            token_source=token_source, module_path=module_path, node_timeout=node_timeout,
        )
        report.project = entry["path"]
        apply_baseline(report, covered.get(entry["path"], frozenset()))
        reports.append(report)
        worst = max(worst, exit_code(report))

    failing = [r for r in reports if r.status == "FAIL"]
    review = [r for r in reports if r.status == "REVIEW_REQUIRED"]
    if as_json:
        print(json.dumps({
            "schema_version": SCHEMA_VERSION,
            "workspace_root": str(workspace_root),
            "baseline": str(baseline_path),
            "projects_checked": len(reports),
            "failing": len(failing),
            "review_required": len(review),
            "projects": [r.to_json() for r in reports],
        }, indent=2, sort_keys=True))
    else:
        for report in reports:
            if report.status == "PASS" and not report.summary["baselined"]:
                continue
            _print_report(report, False)
        print(
            f"AUDIT: projects={len(reports)} fail={len(failing)} review_required={len(review)} "
            f"baseline={baseline_path.name}"
        )
    return worst


def main(argv: list[str] | None = None) -> int:
    governance_root = _governance_root()
    parser = argparse.ArgumentParser(
        description="UI convention conformance checker",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("project", nargs="?", type=Path, help="project directory; omit with --audit")
    parser.add_argument("--json", action="store_true", help="emit a versioned JSON report on stdout")
    parser.add_argument("--baseline", type=Path, default=governance_root / BASELINE_FILE,
                        help="baseline file covering pre-existing findings (default: %s)" % BASELINE_FILE)
    parser.add_argument("--audit", action="store_true", help="check every registered project under the workspace root")
    parser.add_argument("--root", default=None, help="workspace root for --audit; defaults to WORKSPACE_ROOT or the registry's workspace_root")
    parser.add_argument("--declaration", type=Path, default=None, help="explicit declaration file")
    parser.add_argument("--tokens", type=Path, default=None, help="token source; defaults to the platform layer's tokens.json")
    parser.add_argument("--token-module", type=Path, default=None, help="token-model.mjs path")
    parser.add_argument("--node-timeout", type=int, default=DEFAULT_NODE_TIMEOUT, help="seconds allowed for the token resolver")
    args = parser.parse_args(argv)

    default_source = (governance_root / DEFAULT_TOKEN_SOURCE).resolve()
    token_source = args.tokens.resolve() if args.tokens else default_source

    try:
        if args.audit:
            return audit(
                governance_root, args.baseline, as_json=args.json,
                token_source=token_source, module_path=args.token_module,
                node_timeout=args.node_timeout, root_override=args.root,
            )
        if args.project is None:
            print("a project directory is required unless --audit is given", file=sys.stderr)
            return 2
        project_dir = args.project.resolve()
        if not project_dir.is_dir():
            print(f"project directory not found: {project_dir}", file=sys.stderr)
            return 2
        declaration_path = args.declaration or (project_dir / ".project.json")
        declaration = _load_declaration(declaration_path)
        policy = parse_ui_policy(declaration)
        if policy.token_source:
            token_source = _resolve_inside(project_dir, policy.token_source)
        report = evaluate(
            project_dir, policy,
            token_source=token_source, module_path=args.token_module, node_timeout=args.node_timeout,
        )
        if args.baseline and args.baseline.is_file():
            apply_baseline(report, load_baseline(args.baseline).get(project_dir.name, frozenset()))
        _print_report(report, args.json)
        return exit_code(report)
    except ConfigError as exc:
        print(f"REVIEW_REQUIRED: configuration: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())