"""Manifest reading and command derivation for gate bindings."""

import bind_gate_commands as _facade
globals().update({key: value for key, value in vars(_facade).items() if not key.startswith("__")})

def block_entries(lines: list[str], key: str) -> list[tuple[str, str]]:
    """`(entry, value)` for every `  <entry>: <value>` line in a top-level block.

    The value keeps its own quoting and is stripped of surrounding
    whitespace only, so `commands:` and `checks:` are read the same way
    whichever style wrote them. Comment lines, list items and anything not
    at two-space indentation are skipped.
    """
    bounds = top_level_block_bounds(lines, key)
    if bounds is None:
        return []
    start, end = bounds
    entries = []
    for index in range(start + 1, end):
        line = lines[index]
        if not line.startswith("  ") or line.strip().startswith(("#", "-")):
            continue
        if ":" not in line:
            continue
        entry, value = line.strip().split(":", 1)
        entries.append((entry.strip(), value.strip()))
    return entries


def top_level_scalar(lines: list[str], key: str) -> str:
    """The value of a top-level `key: value` line, unquoted, or `""`."""
    prefix = f"{key}:"
    for line in lines:
        if not line.startswith(prefix):
            continue
        return unquote_scalar(line[len(prefix):].strip())
    return ""


def read_manifest(text: str) -> tuple[list[str], dict[str, str], str]:
    """Return `(selected, commands, problem)` for one manifest.

    `selected` is the concern ids the manifest turned on, in the order the
    manifest lists them, so a binding lands in the order the repository
    declared rather than in the order this tool happened to derive them.
    `problem` is non-empty when the manifest cannot be read as a gate
    manifest, in which case nothing may be written.

    Only a `true` value is a required selection. `false` and `"optional"`
    are read by [`read_gate_policy`] instead: both leave the concern out of
    the gate's unbound set, and `"optional"` is a scheduling decision the
    gate does not fail on.
    """
    lines = text.splitlines(keepends=True)
    if top_level_block_bounds(lines, "checks") is None:
        return [], {}, "no `checks:` block, so the manifest selects nothing to bind"
    selected = [entry for entry, value in block_entries(lines, "checks") if value == "true"]
    bound = dict(block_entries(lines, "commands"))
    return selected, bound, ""


@dataclass
class GatePolicy:
    """The three manifest keys that decide which checks the gate fails on
    without the manifest listing them.

    `profile` names the built-in profile whose default selection applies;
    `disabled` and `exempt` are concerns the manifest took out of that
    selection again, by `checks: <id>: false` and by a `not_applicable:`
    exemption with a reason.
    """

    profile: str
    disabled: frozenset[str] = frozenset()
    exempt: frozenset[str] = frozenset()


def read_gate_policy(text: str) -> GatePolicy:
    """Read the selection the gate resolves from keys other than `checks:`.

    The gate's own `.ai-gate` reader (`driftwatchdog/src/gate/aigate.rs`)
    turns `checks: <id>: false` into a disabled check, which *removes* the
    concern even when a profile selected it, and a `not_applicable`
    exemption removes it from the unbound set. Both are read here for the
    same reason: a check the gate does not fail on is not debt, and naming
    it would over-report in the other direction.
    """
    lines = text.splitlines()
    disabled = {
        entry
        for entry, value in block_entries(lines, "checks")
        if value == "false"
    }
    exempt = {entry for entry, _reason in block_entries(lines, "not_applicable")}
    return GatePolicy(
        profile=top_level_scalar(lines, "profile"),
        disabled=frozenset(disabled),
        exempt=frozenset(exempt),
    )


def gate_required(selected: list[str], policy: GatePolicy) -> list[str]:
    """Every concern the gate would report as an unbound required check.

    The manifest's own `checks:` in the order it declares them, then the
    built-in profile's defaults it did not declare, minus everything the
    gate excuses: a check the manifest disabled, a check exempted under
    `not_applicable`, and the one check the gate executes in-process. What
    comes back is what the debt record must name, so this function and the
    gate's `unbound_required_checks` are required to agree.
    """
    declared = set(selected)
    concerns = list(selected)
    concerns.extend(
        concern
        for concern in PROFILE_DEFAULTS.get(policy.profile, ())
        if concern not in declared and concern not in policy.disabled
    )
    return [
        concern
        for concern in concerns
        if concern not in policy.exempt and concern not in IN_PROCESS_CONCERNS
    ]


def profile_only_reason(concern: str, profile: str) -> str:
    """Why a profile-scheduled check is debt this tool will not write away.

    The gate's `.ai-gate` reader refuses a manifest whose `commands:` names
    a check the manifest does not select, so a command for a check that
    only `profile:` selected would not close the gap — it would make the
    manifest unparseable, which fails the gate harder than the gap does.
    The fix is in the manifest, and it is one line.
    """
    return (
        f"the gate schedules {concern} through `profile: {profile}` and this manifest "
        f"does not select it under `checks:`, so the gate refuses a `commands.{concern}` "
        f"entry for it; select it under `checks:` and add scripts/check-{concern}.* to "
        f"bind it, or exempt it under `not_applicable:` with a reason"
    )



# --- detecting the shape from disk -----------------------------


@dataclass
class Shape:
    """What is actually on disk, and where.

    Every field is a path relative to the repository root. Detection never
    consults `.project.json`.
    """

    solutions: list[str] = field(default_factory=list)
    projects: list[str] = field(default_factory=list)
    crates: list[str] = field(default_factory=list)
    workspaces: list[str] = field(default_factory=list)
    node_manifests: list[str] = field(default_factory=list)
    script_keys: dict[str, list[str]] = field(default_factory=dict)
    pubspecs: list[str] = field(default_factory=list)
    python_dirs: list[str] = field(default_factory=list)
    python_manifests: list[str] = field(default_factory=list)
    pytest_configs: list[str] = field(default_factory=list)
    security_tools: list[str] = field(default_factory=list)

    def label(self) -> str:
        """A short shape name for the report and the debt record."""
        if self.solutions or self.projects:
            return "dotnet"
        if self.crates:
            return "rust"
        if self.node_manifests:
            return "node"
        if self.pubspecs:
            return "flutter"
        if self.python_dirs or self.python_manifests or self.pytest_configs:
            return "python"
        return "unknown"


def walk_files(root: Path, max_depth: int = MAX_DEPTH):
    """Yield `(relative_path, absolute_path)` for every file not under a
    skipped directory.

    `os.walk` is used with in-place pruning so a vendored tree is never
    descended into at all, rather than filtered after the fact.
    """
    root_depth = len(root.parts)
    for dirpath, dirnames, filenames in os.walk(root):
        current = Path(dirpath)
        if len(current.parts) - root_depth > max_depth:
            dirnames[:] = []
            continue
        dirnames[:] = sorted(d for d in dirnames if d not in SKIP_DIRS)
        for filename in sorted(filenames):
            absolute = current / filename
            yield str(absolute.relative_to(root)), absolute


def read_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError, UnicodeDecodeError):
        return None


def is_test_project(path: str) -> bool:
    """True when a `.csproj` path is a test project by .NET convention.

    Used only where it narrows a choice to exactly one candidate; a
    repository with several test projects and no solution stays unresolved
    rather than picking the first in sorted order.
    """
    parts = Path(path).parts
    name = parts[-1]
    if "tests" in {p.lower() for p in parts} or "test" in {p.lower() for p in parts}:
        return True
    stem = name[: -len(PROJECT_SUFFIX)]
    return stem.endswith((".Tests", ".Test", "Tests", "Test"))


def detect_shape(repo: Path) -> Shape:
    """Walk the repository and return what is on disk. Never raises."""
    shape = Shape()
    dependency_text: list[str] = []

    for relative, absolute in walk_files(repo):
        name = Path(relative).name
        depth = len(Path(relative).parts) - 1

        if name.endswith(SOLUTION_SUFFIXES):
            shape.solutions.append(relative)
            continue
        if name.endswith(PROJECT_SUFFIX):
            shape.projects.append(relative)
            continue
        if name == "Cargo.toml":
            shape.crates.append(relative)
            try:
                if "[workspace]" in absolute.read_text(encoding="utf-8", errors="replace"):
                    shape.workspaces.append(relative)
            except OSError:
                pass
            continue
        if name == "package.json":
            shape.node_manifests.append(relative)
            data = read_json(absolute)
            scripts = data.get("scripts") if isinstance(data, dict) else None
            if isinstance(scripts, dict):
                shape.script_keys[relative] = sorted(str(k) for k in scripts)
            continue
        if name == "pubspec.yaml":
            shape.pubspecs.append(relative)
            continue
        if name in DEPENDENCY_FILES and depth == 0:
            try:
                dependency_text.append(absolute.read_text(encoding="utf-8", errors="replace"))
            except OSError:
                pass
            continue
        if name == "pyproject.toml":
            shape.python_manifests.append(relative)
            try:
                dependency_text.append(absolute.read_text(encoding="utf-8", errors="replace"))
            except OSError:
                pass
            continue
        if name in ("pytest.ini", "conftest.py", "tox.ini") and depth <= 1:
            shape.pytest_configs.append(relative)
            continue
        if name == "setup.cfg" and depth == 0:
            try:
                if "[tool:pytest]" in absolute.read_text(encoding="utf-8", errors="replace"):
                    shape.pytest_configs.append(relative)
            except OSError:
                pass
            continue
        if re.fullmatch(r"test_[^/]*\.py|test\.py", name) and Path(relative).parts[0] in (
            "tests",
            "test",
        ):
            if "tests" not in shape.python_dirs and "test" not in shape.python_dirs:
                shape.python_dirs.append(Path(relative).parts[0])
            continue

    for text in dependency_text:
        for tool in PYTHON_SECURITY_TOOLS:
            if tool in text and tool not in shape.security_tools:
                shape.security_tools.append(tool)

    return shape


def python_runner(repo: Path, shape: Shape) -> tuple[str, str]:
    """Return `(command, evidence)` for a Python repository's test runner.

    The two runners are not interchangeable, and picking the wrong one is
    worse than refusing: `unittest` discovery finds no `TestCase` in a
    bare-function file and reports success having run nothing. So the test
    files are read.
    """
    config_evidence = ""
    if shape.pytest_configs:
        for config in shape.pytest_configs:
            try:
                text = (repo / config).read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            if "[tool.pytest.ini_options]" in text or config.endswith("pytest.ini"):
                config_evidence = config
                break
            if config.endswith("conftest.py") and config.count("/") == 0:
                config_evidence = config
                break
            if config.endswith("tox.ini") and "[pytest]" in text:
                config_evidence = config
                break
            if config.endswith("setup.cfg") and "[tool:pytest]" in text:
                config_evidence = config
                break
    if config_evidence:
        return "pytest", config_evidence

    directories = [d for d in ("tests", "test") if (repo / d).is_dir()]
    if not directories:
        return "", "no tests/ or test/ directory, and no pytest configuration"
    needs_pytest = False
    defining = 0
    unittest_files = 0
    for directory in directories:
        for path in sorted((repo / directory).rglob("test_*.py")):
            try:
                text = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            if not ANY_TEST_DEF.search(text):
                # A file that defines no test is collected by nobody, so it
                # says nothing about which runner the repository needs.
                continue
            defining += 1
            if BARE_TEST_DEF.search(text) or IMPORT_PYTEST.search(text):
                needs_pytest = True
            if IMPORT_UNITTEST.search(text) or "TestCase" in text:
                unittest_files += 1
    if not defining:
        return (
            "",
            f"no test_*.py under {directories[0]}/ defines a test, so no runner is detectable",
        )
    if needs_pytest:
        return "pytest", f"{directories[0]}/ holds a test file pytest must collect"
    if unittest_files == defining:
        return (
            f"python3 -m unittest discover -s {directories[0]} -v",
            f"{unittest_files} test file(s) in {directories[0]}/ are unittest.TestCase only",
        )
    return (
        "",
        f"{defining} test file(s) in {directories[0]}/ are neither unittest.TestCase nor a "
        "bare def test_, so no runner is shown to collect them",
    )


# --- deriving a command per concern ---------------------------


@dataclass
class Candidate:
    """One entry point that could answer a concern, and its evidence.

    `stack` is the toolchain the candidate came from, declared by the
    generator that produced it and never parsed back out of `command`. It is
    what makes composability decidable: two candidates that cover different
    stacks are two halves of one repository, and two that cover the same stack
    are one product described twice.
    """

    command: str
    evidence: str
    root_level: bool
    stack: str = ""


def dotnet_entry(shape: Shape, *, for_tests: bool) -> tuple[str, str]:
    """Return `(entry, evidence)` for `dotnet build` / `dotnet test`.

    A solution is the entry point when one exists. Failing that, exactly
    one `.csproj` may be chosen; several with no solution is a finding, not
    a search to guess through.
    """
    if shape.solutions:
        root = [s for s in shape.solutions if "/" not in s]
        if len(root) == 1:
            return root[0], f"root solution {root[0]}"
        if root:
            return "", f"{len(root)} root solutions ({', '.join(root)}); no single entry point"
        if len(shape.solutions) == 1:
            return shape.solutions[0], f"the only solution, {shape.solutions[0]}"
        return "", f"{len(shape.solutions)} solutions and none at the root"
    if not shape.projects:
        return "", "no .sln, .slnx or .csproj file"
    if for_tests:
        tests = [p for p in shape.projects if is_test_project(p)]
        if len(tests) == 1:
            return tests[0], f"the only test project, {tests[0]}"
        if tests:
            return "", f"{len(tests)} test projects and no solution ({', '.join(tests[:4])})"
    if len(shape.projects) == 1:
        return shape.projects[0], f"the only project, {shape.projects[0]}"
    return "", f"{len(shape.projects)} .csproj files and no solution"


# --- comparing an existing command -----------------------------


def unquote_scalar(raw: str) -> str:
    """Return the string a `commands:` entry holds, whichever style wrote it.

    The comparison against an existing binding has to be semantic, not
    byte-exact. The seven already-wired repositories in this fleet write
    their commands three different ways — `cargo build --workspace`,
    `"git diff --check"`, `'npm test'` — and a byte comparison would call
    all three a conflict with a command this tool derived identically.
    """
    value = raw.strip()
    if len(value) >= 2 and value[0] == "'" and value[-1] == "'":
        return value[1:-1].replace("''", "'")
    if len(value) >= 2 and value[0] == '"' and value[-1] == '"':
        try:
            decoded = json.loads(value)
        except json.JSONDecodeError:
            return value
        return decoded if isinstance(decoded, str) else value
    # A plain scalar ends at an unquoted ` #`, which YAML treats as a comment.
    marker = value.find(" #")
    return value[:marker].rstrip() if marker != -1 else value


def rust_commands(shape: Shape, concern: str) -> list[Candidate]:
    """One candidate per crate manifest, each carrying its own directory.

    A `Cargo.toml` declaring `[workspace]` manages the crates beneath it, so
    those members are not separate entry points: `alltools-platform` has one
    workspace root and 34 members under it, and offering all 35 would be
    noise rather than ambiguity. A nested manifest needs its directory in
    the command, because the Gate runs each command from the repository root.
    """
    workspace_dirs = {str(Path(m).parent) for m in shape.workspaces}

    def under_a_workspace(directory: str) -> bool:
        # A workspace at the repository root owns every crate beneath it.
        if "." in workspace_dirs:
            return True
        return any(directory.startswith(f"{root}/") for root in workspace_dirs)

    candidates = []
    for manifest in shape.crates:
        directory = str(Path(manifest).parent)
        if manifest not in shape.workspaces and under_a_workspace(directory):
            continue
        is_workspace = manifest in shape.workspaces
        prefix = "" if directory == "." else f"cd {directory} && "
        root_level = directory == "."
        note = "declares [workspace]" if is_workspace else "declares no [workspace]"
        if concern == "build":
            command = f"{prefix}cargo build --workspace" if is_workspace else f"{prefix}cargo build"
        elif concern == "tests":
            command = f"{prefix}cargo test --workspace" if is_workspace else f"{prefix}cargo test"
        elif concern == "security":
            command, note = f"{prefix}cargo audit", "cargo-audit is the Rust dependency audit"
        elif concern == "format":
            command, note = f"{prefix}cargo fmt --all --check", "rustfmt is the Rust formatter"
        else:
            command = (
                f"{prefix}cargo clippy --workspace --all-targets -- -D warnings"
                if is_workspace
                else f"{prefix}cargo clippy --all-targets -- -D warnings"
            )
            note = "clippy is the Rust linter"
        candidates.append(Candidate(command, f"{manifest} ({note})", root_level, "rust"))
    return candidates


def node_commands(shape: Shape, concern: str) -> list[Candidate]:
    """One candidate per `package.json`, only where the script key exists.

    A manifest without the key cannot answer the concern, so it is not a
    candidate at all — that exclusion happens before any ambiguity is
    reported, or a repository whose root Node tooling has no `test` script
    would be reported ambiguous when its Rust workspace can answer.
    """
    key = {
        "build": "build",
        "tests": "test",
        "lint": "lint",
        "format": "format:check",
    }.get(concern)
    candidates = []
    for manifest in shape.node_manifests:
        scripts = set(shape.script_keys.get(manifest, []))
        directory = str(Path(manifest).parent)
        prefix = "" if directory == "." else f"cd {directory} && "
        root_level = directory == "."
        if concern == "security":
            script = next((s for s in ("security", "audit") if s in scripts), None)
            if script:
                candidates.append(
                    Candidate(
                        f"{prefix}npm run {script}",
                        f"{manifest} declares a {script!r} script",
                        root_level,
                        "node",
                    )
                )
            else:
                candidates.append(
                    Candidate(
                        f"{prefix}npm audit --audit-level=high",
                        f"{manifest} has no security script, so this is the npm dependency audit",
                        root_level,
                        "node",
                    )
                )
            continue
        if key is None or key not in scripts:
            continue
        command = f"{prefix}npm test" if key == "test" else f"{prefix}npm run {key}"
        candidates.append(Candidate(command, f"{manifest} declares scripts.{key}", root_level, "node"))
    return candidates


def python_candidates(repo: Path, shape: Shape, concern: str) -> tuple[list[Candidate], str]:
    """Return `(candidates, refusal_reason)` for a Python repository.

    Only one runner is offered, and it is the one the files on disk show can
    collect them. Offering both would make every Python repository ambiguous.
    The refusal reason is returned rather than discarded, so a repository whose
    test files no runner can be shown to collect says exactly that.
    """
    candidates = []
    if concern == "tests":
        command, evidence = python_runner(repo, shape)
        if command:
            candidates.append(Candidate(command, evidence, True, "python"))
        else:
            return [], evidence
    elif concern == "security":
        # At most one tool is offered. A repository's dependency file can
        # mention several of these in prose, and offering one candidate per
        # mention would report a false ambiguity for a repository that
        # simply declares one of them.
        for tool in PYTHON_SECURITY_TOOLS:
            if tool in shape.security_tools:
                return [
                    Candidate(
                        PYTHON_SECURITY_TOOLS[tool],
                        f"the repository declares the {tool} dependency",
                        True,
                        "python",
                    )
                ], ""
        if not shape.security_tools:
            return [], (
                "no Python security tool is declared in this repository's pyproject.toml or "
                "requirements*.txt, and a Python dependency audit is not in the standard library"
            )
    return candidates, ""


def python_build_available(repo: Path) -> tuple[bool, str]:
    """Return whether Python packaging metadata and the build frontend exist.

    A backend alone is not an executable command: `python -m build` is
    provided by the separate `build` frontend. Require both declarations so
    the binder does not turn an unbound concern into a predictable missing
    module failure.
    """
    pyproject = repo / "pyproject.toml"
    try:
        metadata = pyproject.read_text(encoding="utf-8")
    except OSError:
        return False, "no readable pyproject.toml declares a Python build backend"
    try:
        document = tomllib.loads(metadata)
    except tomllib.TOMLDecodeError as exc:
        return False, f"pyproject.toml is invalid TOML: {exc}"
    if not isinstance(document.get("project"), dict):
        return False, "pyproject.toml does not declare [project] distribution metadata"
    if not isinstance(document.get("build-system"), dict):
        return False, "pyproject.toml does not declare a [build-system] table"

    declared: list[str] = []
    project = document.get("project")
    if isinstance(project, dict):
        if isinstance(project.get("dependencies"), list):
            declared.extend(project["dependencies"])
        optional = project.get("optional-dependencies")
        if isinstance(optional, dict):
            for dependencies in optional.values():
                if isinstance(dependencies, list):
                    declared.extend(dependencies)
    groups = document.get("dependency-groups")
    if isinstance(groups, dict):
        for dependencies in groups.values():
            if isinstance(dependencies, list):
                declared.extend(dependencies)
    if any(
        isinstance(value, str)
        and re.match(r"(?i)^\s*build(?:\[[^]]+\])?\s*(?:$|[<=>!~;])", value)
        for value in declared
    ):
        return True, "pyproject.toml declares a build backend and the `build` frontend"
    for name in DEPENDENCY_FILES:
        path = repo / name
        if not path.is_file():
            continue
        try:
            body = path.read_text(encoding="utf-8")
        except OSError:
            continue
        for line in body.splitlines():
            requirement = line.split("#", 1)[0].strip()
            if re.match(r"(?i)^build(?:\[[^]]+\])?\s*(?:$|[<=>!~;])", requirement):
                return True, "pyproject.toml declares a build backend and a requirements file declares `build`"
    return False, "the build-system exists but the `build` frontend is not declared"


def flutter_candidates(shape: Shape, concern: str) -> list[Candidate]:
    candidates = []
    for manifest in shape.pubspecs:
        directory = str(Path(manifest).parent)
        prefix = "" if directory == "." else f"cd {directory} && "
        if concern == "tests":
            candidates.append(
                Candidate(f"{prefix}flutter test", f"{manifest} is a Flutter package", directory == ".", "flutter")
            )
    return candidates


def resolve(candidates: list[Candidate]) -> tuple[Candidate | None, str]:
    """Apply dominance, then composability, and return `(winner, reason_if_refused)`.

    Three answers, in this order.

    1. **One.** A single candidate is bound unchanged.
    2. **A dominant one.** A root-level entry point beats a nested one.
    3. **A composite.** Candidates spanning two or more stacks are joined
       with ` && `, because neither implies the other and the gate's job is
       to cover the whole repository, not half of it.

    Only then is it a refusal. Two candidates of the **same** stack still
    refuse: two Flutter packages or two `package.json` files are one product
    described twice, and choosing between them would assert a shape the
    repository does not have.

    The order matters and is not cosmetic. Dominance runs first, so a
    repository with a root `package.json` and a nested one still resolves to
    the root without a composite. Composability runs before the refusal, so a
    polyglot repository is joined rather than refused — the case that would
    otherwise leave `crossify`'s frontend unbuilt behind a green `build`.
    """
    if not candidates:
        return None, ""
    if len(candidates) == 1:
        return candidates[0], ""
    roots = [c for c in candidates if c.root_level]
    if len(roots) == 1:
        return roots[0], ""
    remaining = roots or candidates
    composite = compose(remaining)
    if composite is not None:
        return composite, ""
    described = "; ".join(f"{c.command} ({c.evidence})" for c in remaining)
    return None, f"{len(remaining)} equally plausible entry points: {described}"


def compose(candidates: list[Candidate]) -> Candidate | None:
    """Join candidates that cover disjoint product surfaces, or return None.

    A composite is offered only when the candidates span **two or more
    distinct stacks**. That is the test for "both must run", and it is
    available from declared information rather than from the shape label or
    by parsing the command: each generator tags its own candidates with the
    toolchain it speaks for.

    A candidate with no declared stack cannot be reasoned about, so it is not
    composable. An empty stack is a gap in a generator, not a repository
    shape, and guessing one would reintroduce the guessing this tool exists to
    avoid.

    Each part is wrapped in a subshell. This is load-bearing, not decoration:
    a candidate may `cd` into its own directory, and `cd` mutates the shell's
    working directory for everything after it. Joining
    `cd rust && cargo build --workspace` and `cd django_archived && npm run
    build` with a bare ` && ` would run the second `cd` from inside `rust/`,
    where `django_archived` does not exist. The gate would then report a
    composite that can only fail, and the repository would go red for a
    command the tool wrote. Wrapping every part in `( ... )` confines each
    `cd` to its own subshell, and it is applied uniformly so the shape of the
    command does not depend on whether a particular part happens to `cd`.

    Order is the generators' emission order, which is already deterministic
    because each iterates a sorted manifest list. That is load-bearing: a
    composite whose order varied between runs would report every polyglot
    repository `DIRTY` on every re-run, which would be a false positive
    introduced by this function.

    `root_level` is False on the result because a composite is never a single
    entry point, and no later rule should treat it as one.
    """
    if len({c.stack for c in candidates}) < 2:
        return None
    if any(not c.stack for c in candidates):
        return None
    return Candidate(
        command=" && ".join(f"({c.command})" for c in candidates),
        evidence="; ".join(f"{c.command} ({c.evidence})" for c in candidates),
        root_level=False,
        stack="+".join(dict.fromkeys(c.stack for c in candidates)),
    )


@dataclass
class ConcernOutcome:
    """What happened to one concern of one repository."""

    repo: str
    concern: str
    status: str
    command: str = ""
    evidence: str = ""
    reason: str = ""
    # True when this concern is a `BOUND` that the profile scheduled rather
    # than the manifest, so the write must add a `checks: <id>: true` entry
    # beside the `commands:` entry. The gate refuses a `commands:` entry for
    # an unselected check, so the two are written together or not at all.
    selects: bool = False


# --- concerns this workspace's own checkers answer -------------

# The two checkers in *this* repository that answer a gate concern. Both are
# reached from a sibling repository by a relative path, which is why the
# reachability test below is a file check rather than an assumption: the gate
# runs the bound command with the repository root as the working directory
# (`gate.rs` `AdapterInput { working_dir: proj.root }`), so `../<repo>/…`
# resolves for every repository that is a sibling of this one, and resolves
# to nothing for one that is not.
#
# `workspace_check.py --root .. --project <this repository>` is passed the
# workspace root *and* this repository's own name, both explicitly, and both
# omissions are defects rather than defaults:
#
# * without `--root`, the checker's default is the directory holding
#   `projects.json` — this repository — and run from a sibling that default
#   makes all 81 registered projects report `PROJECT_MISSING` against the
#   sibling's own subdirectories. 81 false findings is not a pass.
# * without `--project`, the checker audits the **whole workspace**, and the
#   gate's concern filters findings by *code*, not by project
#   (`governance/mod.rs`: `if !scope.contains(finding.code.as_str())`), so
#   every repository's gate would be failed by every *other* repository's
#   findings. Measured on this fleet: the workspace audit reports 12
#   in-scope findings and not one of them belongs to `bargora` — 4 come from
#   `chronicleaf`, 3 from `metiscope`, 3 from `openlearning`, 1 from
#   `dotnet-platform-libs`. Scoping to one project is what makes the verdict
#   this repository's own rather than the workspace's.
#
# `--project` matches a registry `id` **or** a registry `path`, and the
# `path` of all 81 registered projects is its directory name, so a
# repository's own directory name resolves for every repository here.
GOVERNANCE_CHECKER = "scripts/workspace_check.py"
GOVERNANCE_COMMAND = (
    "python3 ../workspace-governance/scripts/workspace_check.py "
    "--root .. --project {project} --json"
)
GOVERNANCE_EVIDENCE = "the portfolio checker at ../workspace-governance/scripts/workspace_check.py"

PRODUCT_QUALITY_CHECKER = "scripts/product_code_quality.py"
PRODUCT_QUALITY_COMMAND = "python3 ../workspace-governance/scripts/product_code_quality.py ."
PRODUCT_QUALITY_EVIDENCE = "the product-quality checker at ../workspace-governance/scripts/product_code_quality.py"

# Concern -> the checker that answers it, in this workspace.
#
# `placeholder-threshold` and `product-code-boundary` are answered by the
# same script; they are two gates over one scan, and the scan is a single
# policy read.
GOVERNANCE_CHECKERS: dict[str, tuple[str, str, str]] = {
    "governance-quality": (GOVERNANCE_CHECKER, GOVERNANCE_COMMAND, GOVERNANCE_EVIDENCE),
    "placeholder-threshold": (
        PRODUCT_QUALITY_CHECKER,
        PRODUCT_QUALITY_COMMAND,
        PRODUCT_QUALITY_EVIDENCE,
    ),
    "product-code-boundary": (
        PRODUCT_QUALITY_CHECKER,
        PRODUCT_QUALITY_COMMAND,
        PRODUCT_QUALITY_EVIDENCE,
    ),
}

# The concerns whose checker reads a `quality` block out of the repository's
# own `.project.json` and is fail-closed without it. `product_code_quality.py`
# raises `ConfigError("project declaration is missing the 'quality' block")`
# and exits 2 — a configuration error — rather than scanning, so a repository
# without one has no policy to run and the checker cannot answer the concern
# at all.
QUALITY_POLICY_FILE = ".project.json"
QUALITY_POLICY_KEY = "quality"


# --- declaring the sibling edge the command walks ----------------

# Ruling: a `mechanism: sibling` edge — a relative-path invocation of another
# checkout — is lawful only when the consuming project also declares the
# provider `required: true` and pins the edge with a digest. So the command
# this tool writes and the declaration that accounts for it are one
# operation, in the same shape as the `quality` block and the command that
# reads it: either both land or neither does, because a bound command with no
# declaration is exactly the undeclared edge the declaration exists to expose.
#
# The declaration is never fabricated. A command that reaches no other
# checkout is bound without one, and a command this tool cannot describe as an
# edge (a `../` path it cannot resolve to a provider and an entry) leaves the
# concern UNBOUND and recorded in the debt record, rather than writing a
# declaration that would not be true.
SIBLING_LAWFUL_ALTERNATIVES = (
    "declare the copy as `mechanism: consume` with its contract and a sha256 "
    "digest, or publish the artifact and consume it by version"
)

# `../<provider>/<relative>` — the one shape a sibling invocation has here.
# The provider is a directory name, which is the registry `path` of all 81
# registered projects, and the entry is the file it names.
SIBLING_PROVIDER = "workspace-governance"
SIBLING_INVOCATION = re.compile(
    r"\.\./(?P<provider>[a-z][a-z0-9]*(?:-[a-z0-9]+)*)/(?P<relative>[A-Za-z0-9][A-Za-z0-9._/-]*)"
)
SIBLING_REACH = re.compile(r"(?:^|[\s'\"(])\.\./")
SIBLING_SUFFIXES = (".py", ".mjs", ".js", ".sh", ".cjs")
