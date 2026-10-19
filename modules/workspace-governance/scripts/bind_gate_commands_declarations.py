"""Interface and quality declarations derived for gate bindings."""

import bind_gate_commands as _facade
globals().update({key: value for key, value in vars(_facade).items() if not key.startswith("__")})

def interface_id_for(relative: str) -> str:
    """The kebab-case interface id a checker path declares: `scripts/foo_bar.py` -> `foo-bar`."""
    stem = Path(relative).name
    for suffix in SIBLING_SUFFIXES:
        if stem.endswith(suffix):
            stem = stem[: -len(suffix)]
            break
    return re.sub(r"[^a-z0-9]+", "-", stem.lower()).strip("-")


def governance_provider(relative: str) -> str | None:
    """The provider id when the reached path descends into a Forge module.

    A module inside Forge's monorepo is reached as
    `forge/modules/<module>/…`, so the first path segment of the invocation is
    the monorepo and not the provider. The provider is the module's own
    project id — the one the registry records for exactly that path — so the
    declared edge names the same project the registry does, and a consumer's
    interface declaration keeps meaning what it meant before the module
    moved into Forge.
    """
    parts = relative.split("/")
    if "modules" in parts:
        index = parts.index("modules")
        if index + 1 < len(parts):
            return parts[index + 1]
    return None


def sibling_interface(command: str) -> tuple[str, dict] | None:
    """`(provider, interfaces[] entry)` declaring `command`, or None.

    None means "this command reaches no checkout", and it is never a licence
    to invent one: the caller either binds a plain local command or refuses
    the concern by name.
    """
    match = SIBLING_INVOCATION.search(command or "")
    if not match:
        return None
    relative = match.group("relative")
    interface_id = interface_id_for(relative)
    if not interface_id:
        return None
    provider = governance_provider(relative) or match.group("provider")
    return provider, {
        "id": interface_id,
        "consumes": True,
        "provider": provider,
        "mechanism": "sibling",
        "entry": command,
        "digest": "sha256:" + hashlib.sha256(command.encode("utf-8")).hexdigest(),
        "required": True,
        "evidence_ref": GATE_YAML,
    }


def declared_interface_ids(repo: Path) -> set[str]:
    """The interface ids this repository's own declaration already carries."""
    data = read_json(repo / PROJECT_JSON)
    if not isinstance(data, dict):
        return set()
    interfaces = data.get("interfaces")
    if not isinstance(interfaces, list):
        return set()
    return {
        item["id"]
        for item in interfaces
        if isinstance(item, dict) and isinstance(item.get("id"), str)
    }


def has_declaration(repo: Path) -> bool:
    """True when the repository carries a `.project.json` this tool may write into.

    The declaration is the registry's document, and this tool never creates
    one: a repository without it has nowhere to declare a sibling edge, so the
    edge cannot be bound lawfully and the concern stays debt.
    """
    return isinstance(read_json(repo / PROJECT_JSON), dict)


def render_interfaces(text: str, entries: list[dict]) -> str:
    """Insert or extend the `interfaces` array of a `.project.json` document.

    An entry whose id is already declared is left exactly as it is: a
    hand-written declaration is the operator's, and this tool never overwrites
    one. Rendering is `json.dumps(..., indent=2)`, the same round trip the
    `quality` block uses, so the diff a run produces is the added entries and
    nothing else.
    """
    data = json.loads(text)
    ordered = dict(data)
    existing = ordered.get("interfaces")
    if not isinstance(existing, list):
        existing = []
    declared = {
        item.get("id") for item in existing if isinstance(item, dict)
    }
    merged = list(existing) + [item for item in entries if item["id"] not in declared]
    ordered["interfaces"] = merged
    return json.dumps(ordered, indent=2) + "\n"


def write_interfaces(repo: Path, entries: list[dict]) -> str:
    """Write `entries` into the repository's `.project.json`; return the new text.

    A repository with no `.project.json` is refused rather than created. The
    caller has already established that the concern can be described, so the
    only failure left is an unreadable document, which is refused by name.
    """
    target = repo / PROJECT_JSON
    if not target.is_file():
        raise ValueError(
            f"no {PROJECT_JSON} to declare the sibling edge in; the declaration "
            f"is the registry's document and is not created here"
        )
    text = target.read_text(encoding="utf-8")
    updated = render_interfaces(text, entries)
    target.write_text(updated, encoding="utf-8")
    return updated

# The two concerns the derived `quality` block answers, by name. They are two
# gates over one scan.
QUALITY_CONCERNS = ("placeholder-threshold", "product-code-boundary")

# The deliberately strict default. Not a derivation and not a neutral value:
# `0` is the strictest threshold the checker accepts, and it is written as a
# default a project may relax on purpose. The workspace principle is that a
# check which runs and complains is worth more than a check which does not
# run, and a threshold above `0` would let a repository pass a gate that
# scanned its product code and found unfinished work. Both projects that
# declare this block by hand chose `0`.
QUALITY_DEFAULT_THRESHOLD = 0

# Path segments that mark test scope. The checker already excludes these
# (`DEFAULT_EXCLUDED_DIRS` in `product_code_quality.py`), so a root under one
# of them would scan nothing and report PASS. A root that can only ever scan
# nothing is a false pass, so it is never derived.
QUALITY_TEST_SEGMENTS = frozenset({"tests", "test", "__tests__", "spec", "specs"})

# Manifest name -> the language the checker scans for it, the file suffixes
# that make a directory source rather than a manifest directory, and whether
# the source sits in a `src` subdirectory beside the manifest.
#
# The `src` convention is **per manifest kind**, not one rule for all of them,
# and getting it wrong refuses a whole toolchain: a Rust crate keeps its
# sources in `src/` beside `Cargo.toml`, a Node package in `src/` beside
# `package.json`, and a .NET project **beside the `.csproj` itself**, because
# that file already names the project and the sources are its siblings. Naming
# `src/Foo/src` for a project at `src/Foo/Foo.csproj` finds nothing, and a
# directory with no source in it is a false pass waiting to happen.
QUALITY_MANIFEST_LANGUAGES: tuple[tuple[str, str, tuple[str, ...], bool], ...] = (
    ("Cargo.toml", "rust", (".rs",), True),
    ("package.json", "typescript", (".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"), True),
    (".csproj", "csharp", (".cs",), False),
    (".py", "python", (".py",), False),
)


def has_quality_policy(repo: Path) -> bool:
    """True when the repository's own `.project.json` declares `quality`.

    Read-only and fail-closed in the same direction as the checker: a
    `.project.json` that is absent, unparseable, or holds no `quality` block
    all answer `False`, because all three make the checker exit 2.
    """
    data = read_json(repo / QUALITY_POLICY_FILE)
    return isinstance(data, dict) and isinstance(data.get(QUALITY_POLICY_KEY), dict)


def _is_test_scope(relative: str) -> bool:
    """True when a repository-relative path is inside test scope.

    Checked on the whole path, not on the file name, because a test project
    is identified by where it sits: `tests/Api.Tests/Api.Tests.csproj` is a
    test project by its directory, whatever its file is called.
    """
    return any(part.lower() in QUALITY_TEST_SEGMENTS for part in Path(relative).parts[:-1])


def _language_files_present(root: Path, extensions: tuple[str, ...], limit: int = 200) -> bool:
    """True when `root` holds at least one file of the scanned language.

    Bounded at `limit` matches: the question is only "is this directory
    source, or is it a manifest directory?", and a repository that has not
    produced a source file in its first 200 candidates is not one this tool
    should name a product root for. Symlinks are refused, on the same rule
    the checker applies.
    """
    seen = 0
    for path in root.rglob("*"):
        if seen >= limit:
            break
        if path.suffix in extensions:
            try:
                if path.is_file() and not path.is_symlink():
                    return True
            except OSError:
                continue
            seen += 1
    return False


def derive_quality_block(repo: Path, shape: Shape) -> tuple[dict | None, str, str]:
    """Derive a `quality` block from disk, or refuse by name.

    Returns `(block, evidence, reason)`. `block` is `None` when no product
    root could be derived, and `reason` then says what was examined.

    Two of the three fields are **derived**, from the same detection the
    build commands use and never from the `stack` label, because a product
    root is a fact about where the code is:

    * `product_roots` — the `src` directory beside each manifest the shape
      found, when that directory holds a file of the manifest's language.
    * `languages` — the union of those languages.

    The third is a **judgement**, and is written as a deliberately strict
    default rather than a neutral one: `placeholder_threshold: 0`. See
    [`QUALITY_DEFAULT_THRESHOLD`].

    `exclude` and `placeholder_markers` are deliberately absent. Both are
    judgements about what is not product and about which words mean
    "unfinished"; a guessed exclusion hides a real finding instead of
    reporting one, and the marker vocabulary is the checker's own
    (`vocabulary.json`), so a copy in every `.project.json` would be a
    second source of truth that drifts from the first.
    """
    candidates: list[tuple[str, str, str]] = []  # (root, language, manifest)
    examined: list[str] = []

    for manifest, language, extensions, uses_src in QUALITY_MANIFEST_LANGUAGES:
        if manifest == "Cargo.toml":
            found = shape.crates
        elif manifest == "package.json":
            found = shape.node_manifests
        elif manifest == ".csproj":
            found = shape.projects
        else:
            found = shape.python_dirs
        for relative in found:
            if manifest == ".csproj" and is_test_project(relative):
                continue
            directory = Path(relative).parent
            if uses_src:
                # `src/` beside the manifest. A manifest at the repository
                # root is the common case, and its `src` is `src` like any
                # other.
                source = repo / directory / "src"
                root_relative = (
                    f"{directory.as_posix()}/src" if str(directory) != "." else "src"
                )
            else:
                # The manifest's own directory: a `.csproj` already names its
                # project, so its siblings are the product. The repository
                # root is never used as a root — the checker rejects an empty
                # entry, and a root that scans everything is not a derivation.
                source = repo / directory
                root_relative = "" if str(directory) == "." else directory.as_posix()
                if not root_relative:
                    continue
            if _is_test_scope(root_relative):
                continue
            examined.append(relative)
            if not source.is_dir():
                continue
            if not _language_files_present(source, extensions):
                continue
            candidates.append((root_relative, language, relative))

    if not candidates:
        looked = ", ".join(sorted(set(examined))) or "no manifest on disk"
        return (
            None,
            "",
            f"no product root is derivable: none of the source directories beside "
            f"the manifests on disk ({looked}) holds a file of that manifest's "
            f"language, so there is no directory the checker can be told to scan. "
            f"Which directories are product is an owner decision this tool does "
            f"not guess; add a `{QUALITY_POLICY_KEY}` block to "
            f"{QUALITY_POLICY_FILE} naming the product roots, or exempt the "
            f"concern under `not_applicable:` with a reason",
        )

    roots = sorted({root for root, _language, _manifest in candidates})
    languages = sorted({language for _root, language, _manifest in candidates})
    evidence = "; ".join(
        f"{manifest} -> {root}/"
        for root, _language, manifest in sorted(candidates, key=lambda c: (c[0], c[2]))
    )
    return (
        {
            "product_roots": roots,
            "languages": languages,
            "placeholder_threshold": QUALITY_DEFAULT_THRESHOLD,
        },
        evidence,
        "",
    )


def render_quality_block(text: str, block: dict) -> str:
    """Insert `block` as the `quality` key of a `.project.json` document.

    The round trip is `json.dumps(..., indent=2)`, which is byte-identical to
    all 81 declarations in this workspace as they stand, so the diff a run
    produces is the block and nothing else. Key order is preserved: the block
    is appended last, which is where both hand-written examples put it.
    """
    data = json.loads(text)
    ordered = dict(data)
    ordered[QUALITY_POLICY_KEY] = block
    return json.dumps(ordered, indent=2) + "\n"


def write_quality_block(repo: Path, block: dict) -> str:
    """Write `block` into the repository's `.project.json`; return the new text.

    A repository with no `.project.json` is refused rather than created: the
    declaration is the registry's document, and writing a one-key file to
    stand in for it would be inventing the repository's identity.
    """
    target = repo / QUALITY_POLICY_FILE
    if not target.is_file():
        raise ValueError(
            f"no {QUALITY_POLICY_FILE} to add a `{QUALITY_POLICY_KEY}` block to; the "
            f"declaration is the registry's document and is not created here"
        )
    text = target.read_text(encoding="utf-8")
    updated = render_quality_block(text, block)
    target.write_text(updated, encoding="utf-8")
    return updated



def interface_ids_for(repo: Path, shape: Shape, required: list[str]) -> list[str]:
    """The interface ids the required concerns would declare, deduplicated in order.

    Derived from the same facts the command derivation itself uses — the
    concern's checker, `../<provider>/<relative>` existing on disk, and, for a
    quality concern, the policy it is fail-closed without — so the declaration
    write and the command write are decided on one rule rather than two. An id
    the declaration already carries is still returned; the caller subtracts
    what is already declared.
    """
    ids: list[str] = []
    for concern in required:
        checker = GOVERNANCE_CHECKERS.get(concern)
        if checker is None:
            continue
        relative = checker[0]
        if not (repo / governance_prefix(repo) / relative).is_file():
            continue
        if concern in QUALITY_CONCERNS and not (
            has_quality_policy(repo) or derive_quality_block(repo, shape)[0] is not None
        ):
            continue
        interface_id = interface_id_for(relative)
        if interface_id and interface_id not in ids:
            ids.append(interface_id)
    return ids


def governance_concern(repo: Path, concern: str, shape: Shape) -> tuple[str, str, str]:
    """Derive one of this workspace's own concerns, or refuse by name.

    A repository's **own** `scripts/check-<concern>.*` wins over the
    workspace default, for the same reason it wins everywhere else here: it
    is more specific than the fallback, and a repository that wrote a script
    for this check meant that script. The workspace checker is consulted only
    when the repository wrote none.

    The gate protocol decides what is possible here. `gate.rs:206-208` runs a
    bound command as `sh -c <cmd>` with the repository root as the working
    directory and **no stdin** (`runner.rs:103`, `Stdio::null()`), so a bound
    command has to work from its argument vector and the inherited
    environment and report through stdout and its exit code. Both commands
    below satisfy that: they take a path, print a report on stdout, and exit
    0/1 (and 2 for a configuration error).

    `forge_governance_adapter.py` does **not** satisfy it and is deliberately
    not bound: it reads `sys.stdin.read()` and emits an observation
    (`status`/`detail`/`evidence`) rather than a findings report, so with no
    stdin it sees an empty input and the gate's report parser would not
    recognise the output. It is a one-input adapter, not a checker.
    """
    entry = GOVERNANCE_CHECKERS.get(concern)
    if entry is None:
        return "", "", f"{concern} is not answered by a checker in this workspace"

    own_command, own_evidence, own_reason = script_concern(repo, concern)
    if own_command:
        return own_command, own_evidence, own_reason

    relative, command, evidence = entry
    prefix = governance_prefix(repo)

    if not (repo / prefix / relative).is_file():
        return (
            "",
            "",
            f"this repository does not sit next to the Forge-owned governance "
            f"module, so {prefix}/{relative} does not exist and the {concern} "
            f"checker cannot be reached from it; {own_reason}",
        )

    if concern in QUALITY_CONCERNS and not has_quality_policy(repo):
        # The checker is reachable but has no policy to run. Two fields of the
        # policy are derivable from disk and one is a judgement, so the tool
        # derives the block rather than refusing outright — but the concern
        # binds only once the block is actually on disk, which `bind_repo`
        # writes in the same operation. Until then it is debt: a `commands:`
        # entry whose checker still exits 2 for want of the policy it reads is
        # a bound check that runs nothing, which is the same defect as a
        # `checks:` entry with no command behind it.
        block, _block_evidence, refusal = derive_quality_block(repo, shape)
        if block is None:
            return "", "", refusal
        # The reason deliberately does NOT name the derived roots. Every
        # repository's roots differ, so a reason carrying them would split
        # this one cause into 39 near-identical groups in the debt record and
        # destroy the grouping that lets a reader see the cause first. What
        # derived a block is a function return value (`derive_quality_block`),
        # and the roots it returns are in the diff the run writes.
        return (
            "",
            "",
            f"{relative} reads a `{QUALITY_POLICY_KEY}` block out of this "
            f"repository's {QUALITY_POLICY_FILE} and is fail-closed without one: "
            f"it exits 2 with `project declaration is missing the "
            f"'{QUALITY_POLICY_KEY}' block` and never scans. A block IS derivable "
            f"from this repository's files — its product_roots and languages from "
            f"the manifests on disk, and placeholder_threshold "
            f"{block['placeholder_threshold']}, the deliberately strict default a "
            f"project may relax in its own {QUALITY_POLICY_FILE} — so a run with "
            f"--apply writes the block and binds this concern in the same "
            f"operation. Without --apply this run is a dry run, writes nothing, "
            f"and the concern stays in the debt record. To opt out, exempt the "
            f"concern under `not_applicable:` with a reason",
        )

    # The command reaches another checkout, so binding it writes a
    # `mechanism: sibling` interface edge as well as the command. A repository
    # with no declaration has nowhere for that edge to be declared, and an
    # undeclared edge is exactly what this rule exists to stop, so the concern
    # stays unbound and is recorded in the debt record by name.
    if not has_declaration(repo):
        return (
            "",
            "",
            f"the {concern} checker is reachable at "
            f"{prefix}/{relative}, but a command that reaches another "
            f"checkout is lawful only when the edge is declared and this repository has "
            f"no {PROJECT_JSON} to declare it in; the declaration is the registry's "
            f"document and this tool never creates one, so the concern stays unbound "
            f"rather than becoming an undeclared sibling edge",
        )

    # `{project}` is this repository's own directory name, which is also its
    # `path` in the registry for all 81 registered projects. It is read from
    # the path, not from `.project.json`, on the same rule as everything else
    # here: the label is not an input.
    #
    # `{governance}` is the path from this repository to the Forge-owned
    # governance module, derived from where this repository is.
    return (
        command.format(governance=prefix, project=repo.name),
        evidence.format(governance=prefix),
        "",
    )


def derive(concern: str, shape: Shape, repo: Path) -> tuple[str, str, str]:
    """Return `(command, evidence, reason)` for one concern.

    `command` empty means the concern is unresolved and `reason` says why.
    Every rule is gated on a file existing.
    """
    if concern in REPOSITORY_CONCERNS:
        return "git diff --check", "a git checkout, verified by repository discovery", ""

    if concern in OPENSPEC_NAME_CONCERNS:
        if (repo / OPENSPEC_NAMES_SCRIPT).is_file():
            return f"node {OPENSPEC_NAMES_SCRIPT}", OPENSPEC_NAMES_SCRIPT, ""
        return "", "", f"no {OPENSPEC_NAMES_SCRIPT} in this repository"

    if concern in SCRIPT_CONCERNS:
        command, relative = SCRIPT_CONCERNS[concern]
        if (repo / relative).is_file():
            return command, relative, ""
        return "", "", f"no {relative} in this repository"

    if concern in GOVERNANCE_CHECKERS:
        return governance_concern(repo, concern, shape)

    if concern not in SHAPE_CONCERNS:
        return script_concern(repo, concern)

    has_dotnet = bool(shape.solutions or shape.projects)
    has_python = bool(shape.python_dirs or shape.pytest_configs or (repo / "pyproject.toml").is_file())
    only_python = has_python and not (
        has_dotnet or shape.crates or shape.node_manifests or shape.pubspecs
    )

    if concern == "build" and only_python:
        available, evidence = python_build_available(repo)
        if available:
            return "python -m build", evidence, ""
        return "", "", evidence
    if concern == "build" and shape.pubspecs and not (has_dotnet or shape.crates or shape.node_manifests):
        return (
            "",
            "",
            "flutter build needs a target platform this repository does not name",
        )
    if concern == "build" and has_dotnet and not shape.solutions and not shape.projects:
        return "", "", "no .NET project on disk to build"

    candidates: list[Candidate] = []
    if has_dotnet:
        if concern == "lint":
            return (
                "",
                "",
                "no single .NET linter command covers a solution and its projects; "
                "declare scripts/check-lint.* if the repository wants one",
            )
        entry, evidence = dotnet_entry(shape, for_tests=concern == "tests")
        if not entry:
            return "", "", evidence
        candidates.append(
            Candidate(DOTNET_COMMANDS[concern](entry), evidence, "/" not in entry, "dotnet")
        )
    if shape.crates:
        candidates.extend(rust_commands(shape, concern))
    if shape.node_manifests:
        candidates.extend(node_commands(shape, concern))
    refusals: list[str] = []
    if has_python:
        python_candidates_found, python_refusal = python_candidates(repo, shape, concern)
        candidates.extend(python_candidates_found)
        if python_refusal:
            refusals.append(python_refusal)
    if shape.pubspecs:
        candidates.extend(flutter_candidates(shape, concern))

    if not candidates:
        if shape.node_manifests:
            key = {"build": "build", "tests": "test", "lint": "lint", "format": "format:check"}.get(concern)
            if key:
                return (
                    "",
                    "",
                    f"no package.json in this repository declares a {key!r} script, so "
                    f"there is no {concern} command to derive",
                )
        if refusals:
            return "", "", refusals[0]
        if shape.pubspecs and concern not in ("build",):
            return "", "", f"no {concern} rule exists for a Flutter package"
        if concern == "build":
            return "", "", "no recognised project shape on disk"
        if concern == "tests":
            return "", "", "no test runner is detectable on disk"
        return "", "", f"no rule derives a {concern} command from the shape on disk"
    winner, reason = resolve(candidates)
    if winner is None:
        return "", "", reason
    return winner.command, winner.evidence, ""


def script_concern(repo: Path, concern: str) -> tuple[str, str, str]:
    """A concern a repository implements as its own script, or nothing.

    `careBridge` and `openpanel` bind their concerns this way, so the
    convention is read off the disk rather than assumed: a repository that
    has not written the script does not get a command.
    """
    found = [
        suffix
        for suffix in SCRIPT_SUFFIXES
        if (repo / "scripts" / f"check-{concern}{suffix}").is_file()
    ]
    if len(found) == 1:
        relative = f"scripts/check-{concern}{found[0]}"
        return relative, relative, ""
    if len(found) > 1:
        names = ", ".join(f"scripts/check-{concern}{s}" for s in found)
        return "", "", f"{concern} has more than one candidate script: {names}"
    return (
        "",
        "",
        f"no rule derives a {concern} command and this repository has no "
        f"scripts/check-{concern}.* to run",
    )


# --- the outcome types -----------------------------------------
