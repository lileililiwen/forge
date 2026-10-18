"""Repository binding, report aggregation, and gate command execution."""

import bind_gate_commands as _facade
globals().update({key: value for key, value in vars(_facade).items() if not key.startswith("__")})

@dataclass
class Outcome:
    """What happened to one repository.

    The first six fields are the sibling's, unchanged, so a caller that
    reads `repo`/`status`/`detail`/`wrote`/`before`/`after` reads the same
    thing here. `concerns` is new because a repository's unit of work in
    this tool is a concern.
    """

    repo: str
    status: str
    detail: str = ""
    wrote: bool = False
    before: str = ""
    after: str = ""
    concerns: list[ConcernOutcome] = field(default_factory=list)
    shape: str = ""


def debt_concerns(outcome: Outcome) -> list[ConcernOutcome]:
    """Return actual debt, including writes intentionally blocked by dirty state.

    A `BOUND` concern in a clean dry-run is a proposed command, but a `BOUND`
    concern in a DIRTY repository remains unbound on disk: the write guard
    refused it. Record that live gate state so the snapshot cannot silently
    drop a project merely because its manifest has unrelated work in flight.
    """
    dirty_bindings = [
        concern
        for concern in outcome.concerns
        if outcome.status == DIRTY and concern.status == BOUND
    ]
    for concern in dirty_bindings:
        concern.reason = f"gate manifest is dirty; binding remains unbound: {concern.command}"
    return [
        concern
        for concern in outcome.concerns
        if concern.status in NOT_RESOLVED
        or concern in dirty_bindings
    ]


@dataclass
class Report:
    """The whole run, in a form a caller can assert on and a human can read."""

    outcomes: list[Outcome] = field(default_factory=list)

    def add(self, outcome: Outcome) -> None:
        self.outcomes.append(outcome)

    def count(self, status: str) -> int:
        return sum(1 for o in self.outcomes if o.status == status)

    def names(self, status: str) -> list[str]:
        return [o.repo for o in self.outcomes if o.status == status]

    def unresolved(self) -> list[ConcernOutcome]:
        return [c for outcome in self.outcomes for c in debt_concerns(outcome)]

    def debt_total(self) -> int:
        return len(self.unresolved())

    def conflicts(self) -> list[ConcernOutcome]:
        return [c for o in self.outcomes for c in o.concerns if c.status == CONFLICT]


# --- writing, reusing the sibling's comment-preserving editor ---


def render_checks(text: str, selections: list[str]) -> str:
    """Add `checks: <concern>: true` for each name in `selections`.

    The gate's `.ai-gate` reader refuses a document whose `commands:` names a
    check the manifest does not select (`aigate.rs:213`), so a command for a
    profile-scheduled concern needs this entry to exist. It is written with
    the value `true` and never over an existing key: a repository that
    already lists the concern — as `true`, `false`, or `"optional"` — is
    left exactly as it is, because every one of those is a decision the
    repository made about its own selection.
    """
    if not selections:
        return text
    lines = text.splitlines(keepends=True)
    present = {entry for entry, _value in block_entries(lines, "checks")}
    additions = [name for name in selections if name not in present]
    if not additions:
        return text
    block = top_level_block_bounds(lines, "checks")
    if block is None:
        # `read_manifest` refuses a manifest with no `checks:` block, so this
        # is unreachable from `bind_repo`. It is refused rather than guessed
        # at, because inventing the block would invent the whole selection.
        raise ValueError("manifest has no `checks:` block to add a selection to")
    _start, end = block
    for offset, name in enumerate(additions):
        lines.insert(end + offset, f"  {name}: true\n")
    return "".join(lines)


def render_manifest(text: str, bindings: list[tuple[str, str]]) -> str:
    """Insert `bindings` into the manifest's `commands:` block.

    `bindings` is `(concern, command)` in the order the manifest declares
    its `checks:`, so the result reads in the repository's own order. The
    generated comment is written once, guarded by its marker, so a partial
    binding does not accumulate a second copy.
    """
    if not bindings:
        return text
    lines = text.splitlines(keepends=True)
    block = top_level_block_bounds(lines, "commands")
    if block is None:
        body = [line for line in lines if line.strip()]
        trailing = len(lines) - len(body)
        out = list(lines[: len(lines) - trailing])
        if out and not out[-1].endswith("\n"):
            out[-1] += "\n"
        if out and out[-1].strip():
            out.append("\n")
        out.append("commands:\n")
        out.append(GENERATED_COMMENT)
        for concern, command in bindings:
            out.append(f"  {concern}: {yaml_scalar(command)}\n")
        out.extend(lines[len(lines) - trailing :])
        return "".join(out)

    _start, end = block
    has_comment = any(GENERATED_MARKER in line for line in lines[:end])
    insert_at = end
    if not has_comment:
        lines.insert(insert_at, GENERATED_COMMENT)
        insert_at += 1
    for offset, (concern, command) in enumerate(bindings):
        lines.insert(insert_at + offset, f"  {concern}: {yaml_scalar(command)}\n")
    return "".join(lines)


def classify_repo(repo_path: Path, selected: list[str], bound: dict[str, str],
                  shape: Shape, profile_only: frozenset[str] = frozenset(),
                  profile: str = "") -> list[ConcernOutcome]:
    """Decide each selected concern, without writing anything.

    `selected` is the gate's required set, which is a superset of the
    manifest's own `checks:` — see [`gate_required`]. `profile_only` names
    the ones only the profile scheduled.

    A profile-only concern is **derived first** rather than refused outright.
    The gate's manifest reader rejects a `commands:` entry for a check the
    manifest does not select, so the old answer was to refuse and record the
    gap. But a repository that names `profile: product` has already committed
    to the three concerns that profile schedules, and the gate already fails
    it for not binding them — so where a checker is genuinely reachable, the
    correct write is **both** a `checks:` entry and a `commands:` entry, and
    [`bind_repo`] performs them as one operation.

    Where no checker is reachable, the concern is recorded and neither entry
    is written. That is the case that matters for honesty: a `checks:` entry
    with no command behind it is a worse state than the gap, because it
    makes a declared check one the gate now fails on while still running
    nothing.
    """
    outcomes = []

    def proposal(concern: str) -> tuple[str, str, str]:
        """`derive(...)`, plus the rule that a sibling edge must be describable.

        Every command this tool binds either reaches no other checkout — a
        local runner, a repository's own script — or reaches one, and then it
        is an interface edge that has to be declared (`mechanism: sibling`,
        provider, entry, digest). `derive` knows the command; only here is the
        edge visible. A command that walks into another checkout in a shape
        this tool cannot describe is refused by name, so the concern stays
        debt instead of becoming an undeclared edge, and no declaration is
        ever fabricated for it.
        """
        command, evidence, reason = _facade.derive(concern, shape, repo_path)
        if command and SIBLING_REACH.search(command) and sibling_interface(command) is None:
            return (
                "",
                "",
                f"the derived command `{command}` reaches another checkout, and this "
                f"tool cannot describe that edge as a declared interface (a provider and "
                f"the entry it invokes, with a digest); the concern stays unbound rather "
                f"than becoming an undeclared sibling edge. The two lawful alternatives "
                f"are {SIBLING_LAWFUL_ALTERNATIVES}",
            )
        return command, evidence, reason

    for concern in selected:
        if concern in profile_only:
            command, evidence, reason = proposal(concern)
            if command:
                # Reachable: bound, and `bind_repo` writes the matching
                # `checks:` entry with it.
                outcomes.append(
                    ConcernOutcome(
                        repo_path.name,
                        concern,
                        BOUND,
                        command,
                        evidence,
                        selects=True,
                    )
                )
            else:
                outcomes.append(
                    ConcernOutcome(
                        repo_path.name,
                        concern,
                        UNRESOLVED,
                        reason=reason or profile_only_reason(concern, profile),
                    )
                )
            continue
        if concern in bound:
            command, evidence, _reason = proposal(concern)
            if not command:
                # Bound, and by an authority other than this tool: the
                # `declared-verification` command the sibling generator
                # writes, or a check this tool has no rule for. The gate will
                # run it, so it is already bound and is left alone.
                outcomes.append(
                    ConcernOutcome(
                        repo_path.name,
                        concern,
                        ALREADY_BOUND,
                        unquote_scalar(bound[concern]),
                        "bound by another authority; this tool derives nothing for it",
                    )
                )
            elif unquote_scalar(bound[concern]) == command:
                outcomes.append(
                    ConcernOutcome(repo_path.name, concern, ALREADY_BOUND, command, evidence)
                )
            else:
                outcomes.append(
                    ConcernOutcome(
                        repo_path.name,
                        concern,
                        CONFLICT,
                        bound[concern],
                        evidence,
                        "an existing command differs from the derived one; left untouched",
                    )
                )
            continue
        command, evidence, reason = proposal(concern)
        if command:
            outcomes.append(ConcernOutcome(repo_path.name, concern, BOUND, command, evidence))
        else:
            outcomes.append(ConcernOutcome(repo_path.name, concern, UNRESOLVED, reason=reason))
    return outcomes


def roll_up(concerns: list[ConcernOutcome], total_selected: int) -> str:
    outstanding = [c for c in concerns if c.status in NOT_RESOLVED]
    if not outstanding:
        return RESOLVED
    if len(outstanding) == total_selected:
        return UNRESOLVED
    return PARTIAL


# --- the debt record -------------------------------------------


def build_debt(report: Report, root: Path) -> dict:
    """Assemble the debt record.

    No timestamp: a `generated_at` field makes every run a diff and trains
    reviewers to ignore the file. Sorted everywhere, so re-running on an
    unchanged fleet is byte-identical and a change in it is a real change
    in the debt.
    """
    repositories = []
    by_reason: dict[str, list[str]] = {}
    for outcome in sorted(report.outcomes, key=lambda o: o.repo):
        gaps = debt_concerns(outcome)
        if not gaps:
            continue
        repositories.append(
            {
                "repo": outcome.repo,
                "shape": outcome.shape,
                "unresolved": [
                    {"concern": c.concern, "status": c.status, "reason": c.reason or c.command}
                    for c in sorted(gaps, key=lambda c: c.concern)
                ],
            }
        )
        for gap in gaps:
            by_reason.setdefault(gap.reason or gap.command, []).append(f"{outcome.repo}:{gap.concern}")

    groups = [
        {
            "reason": reason,
            "checks": len(sorted(set(entries))),
            "repositories": sorted({e.split(":")[0] for e in entries}),
            "entries": sorted(set(entries)),
        }
        for reason, entries in sorted(by_reason.items(), key=lambda kv: (-len(kv[1]), kv[0]))
    ]
    return {
        "generator": "scripts/bind_gate_commands.py",
        "note": (
            "Every check listed here is one the gate would report as UNBOUND: a "
            "check this repository's .ai-gate/gate.yaml selects under `checks:`, or "
            "one its `profile:` schedules by default, with no command bound under "
            "`commands:`. The gate reports UNBOUND for these and fails the project. "
            "A wrong command is worse than a listed gap: resolve these by hand, "
            "then re-run the generator."
        ),
        "totals": {
            "repositories_with_debt": len(repositories),
            "unresolved_checks": sum(g["checks"] for g in groups),
        },
        "by_reason": groups,
        "repositories": repositories,
    }


def write_debt_record(debt: dict, path: Path) -> None:
    path.write_text(json.dumps(debt, indent=2, sort_keys=False) + "\n", encoding="utf-8")


# --- the per-repository driver ---------------------------------


def declaration_dirty(repo_path: Path) -> bool:
    """True when `.project.json` has uncommitted work.

    The same rule the sibling's `is_dirty` applies to the manifest, applied to
    the one other file this tool now writes. A run adds a `quality` block to
    that file, and adding a key to a declaration somebody is mid-edit on
    would write into their work. Untracked counts, for the reason
    `is_dirty` gives: an uncommitted `.project.json` is still uncommitted.
    """
    import subprocess

    try:
        completed = subprocess.run(
            ["git", "status", "--porcelain", "--", PROJECT_JSON],
            cwd=repo_path,
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        # If git cannot answer, do not guess that the tree is clean.
        return True
    return completed.stdout.strip() != ""


def bind_repo(repo_path: Path, apply: bool = False, check_clean: bool = True) -> Outcome:
    """Bind one repository's gate-required concerns. Never raises."""
    name = repo_path.name
    manifest = repo_path / GATE_YAML
    if not manifest.is_file():
        return Outcome(
            name, NO_MANIFEST, f"no {GATE_YAML}; nothing was written"
        )
    try:
        before = manifest.read_text(encoding="utf-8")
    except OSError as exc:
        return Outcome(name, REFUSED, f"cannot read {GATE_YAML}: {exc}")

    selected, bound, problem = read_manifest(before)
    if problem:
        return Outcome(name, REFUSED, problem)

    policy = read_gate_policy(before)
    required = gate_required(selected, policy)
    profile_only = frozenset(c for c in required if c not in set(selected))

    try:
        shape = detect_shape(repo_path)
    except Exception as exc:  # noqa: BLE001 - a bad repository must not stop the fleet
        return Outcome(name, REFUSED, f"could not inspect the repository: {exc}")

    # The dirty check refuses the *write*, not the reading. The gate
    # resolves a dirty repository exactly as it resolves a clean one and
    # fails on the same unbound checks, so leaving them out of the record
    # would hide real debt the moment somebody had work in progress — and
    # an `--apply` run immediately after a previous one would write a
    # record that silently under-reports.
    manifest_dirty = check_clean and is_dirty(repo_path)
    # The `quality` block and any `interfaces[]` entry go into
    # `.project.json`, so that file's own uncommitted work refuses the write
    # for the same reason. A declaration write is needed when a `quality`
    # block must be written, or when a required concern binds a command that
    # reaches another checkout and the declaration of that edge is not on
    # disk yet.
    declaration_needed = (
        not has_quality_policy(repo_path) and any(c in QUALITY_CONCERNS for c in required)
    ) or bool(
        [
            interface_id
            for interface_id in interface_ids_for(repo_path, shape, required)
            if interface_id not in declared_interface_ids(repo_path)
        ]
    )
    declaration_dirty_now = check_clean and declaration_needed and declaration_dirty(repo_path)
    dirty = manifest_dirty or declaration_dirty_now

    # The two product-quality concerns need a `quality` block in the
    # repository's own `.project.json` before the bound command can run
    # anything, and the checker is fail-closed without it (exit 2). So the
    # block is derived from disk and written HERE, before classification and
    # under the same `--apply` and dirty guards as the manifest write — which
    # is what makes the two halves one operation. No manifest on disk ever
    # carries a `commands:` entry whose checker still exits 2 for want of the
    # policy it reads, because the policy is written first or not at all.
    quality_written = ""
    if (
        apply
        and not dirty
        and not has_quality_policy(repo_path)
        and any(c in QUALITY_CONCERNS for c in required)
    ):
        block, block_evidence, block_reason = derive_quality_block(repo_path, shape)
        if block is not None:
            try:
                write_quality_block(repo_path, block)
            except (OSError, ValueError) as exc:
                return Outcome(
                    name, REFUSED,
                    f"cannot write the `{QUALITY_POLICY_KEY}` block into "
                    f"{QUALITY_POLICY_FILE}: {exc}",
                    shape=shape.label(),
                )
            quality_written = f"{QUALITY_POLICY_KEY} -> {block_evidence}"
        else:
            # Not derivable. The concern is refused by name below, and the
            # reason is the one this function produced.
            quality_written = ""

    try:
        concerns = classify_repo(
            repo_path,
            required,
            bound,
            shape,
            profile_only=profile_only,
            profile=policy.profile,
        )
    except Exception as exc:  # noqa: BLE001
        return Outcome(name, REFUSED, f"{GATE_YAML} could not be evaluated: {exc}")

    status = roll_up(concerns, len(required))
    label = f"{len(required)} required check(s)"

    if manifest_dirty:
        return Outcome(
            name,
            DIRTY,
            f"{GATE_YAML} has uncommitted work; {label} classified and recorded, nothing written",
            before=before,
            after=before,
            concerns=concerns,
            shape=shape.label(),
        )

    if declaration_dirty_now:
        return Outcome(
            name,
            DIRTY,
            f"{PROJECT_JSON} has uncommitted work; {label} classified and recorded, nothing written",
            before=before,
            after=before,
            concerns=concerns,
            shape=shape.label(),
        )

    # The sibling edges this repository's commands make. A command this tool
    # wrote and a command it found already bound identically are the same
    # edge, and both are declared here, which is what turns the fleet's
    # existing bindings from undeclared into declared ones on a re-run. A
    # `CONFLICT` command is deliberately not described: the tool leaves it
    # exactly as it is, and a declaration it cannot reconcile with its own
    # derivation is not one it may write. A repository with no declaration to
    # write into keeps the old behaviour for the same reason.
    declarations: dict[str, dict] = {}
    if has_declaration(repo_path):
        for item in concerns:
            if item.status not in (BOUND, ALREADY_BOUND) or not item.command:
                continue
            described = sibling_interface(item.command)
            if described is None:
                continue
            declarations.setdefault(described[1]["id"], described[1])
    already_declared = declared_interface_ids(repo_path)
    missing = [item for key, item in sorted(declarations.items()) if key not in already_declared]
    missing_ids = ", ".join(item["id"] for item in missing)

    declared_here = ""
    if apply and not dirty and missing:
        try:
            write_interfaces(repo_path, missing)
        except (OSError, ValueError, json.JSONDecodeError) as exc:
            return Outcome(
                name,
                REFUSED,
                f"cannot declare the sibling edge in {PROJECT_JSON}: {exc}",
                concerns=concerns,
                shape=shape.label(),
            )
        declared_here = f"interfaces -> {missing_ids}"

    pending = [c for c in concerns if c.status == BOUND]
    if not pending:
        if apply:
            note = f"; wrote {declared_here} in {PROJECT_JSON}" if declared_here else ""
        else:
            note = (
                f"; would declare interfaces -> {missing_ids} in {PROJECT_JSON}"
                if missing
                else ""
            )
        return Outcome(
            name,
            status,
            f"{label}, all already bound or refused{note}",
            before=before,
            after=before,
            concerns=concerns,
            shape=shape.label(),
        )

    try:
        after = render_manifest(before, [(c.concern, c.command) for c in pending])
        # The `checks:` entry goes in **after** the `commands:` entry and
        # before the single write, so the file on disk never carries a
        # selection with no command behind it at any point. Both land in
        # `after`, which is the only text this function writes.
        selects = [c.concern for c in pending if c.selects]
        if selects:
            after = render_checks(after, selects)
    except Exception as exc:  # noqa: BLE001
        return Outcome(
            name, REFUSED, f"{GATE_YAML} could not be updated: {exc}", concerns=concerns,
            shape=shape.label(),
        )

    if not apply:
        written = [part for part in (quality_written, f"interfaces -> {missing_ids}" if missing else "") if part]
        suffix = f"; would write {', '.join(written)} in {QUALITY_POLICY_FILE}" if written else ""
        return Outcome(
            name,
            status,
            f"would bind {len(pending)} concern(s): "
            f"{', '.join(c.concern for c in pending)}{suffix}",
            before=before,
            after=after,
            concerns=concerns,
            shape=shape.label(),
        )
    try:
        manifest.write_text(after, encoding="utf-8")
    except OSError as exc:
        return Outcome(
            name, REFUSED, f"cannot write {GATE_YAML}: {exc}", concerns=concerns,
            shape=shape.label(),
        )
    detail = f"bound {len(pending)} concern(s): {', '.join(c.concern for c in pending)}"
    if declared_here:
        detail += f"; declared {declared_here} in {PROJECT_JSON}"
    return Outcome(
        name,
        status,
        detail,
        wrote=True,
        before=before,
        after=after,
        concerns=concerns,
        shape=shape.label(),
    )


def run(
    root: Path,
    apply: bool = False,
    only: list[str] | None = None,
    check_clean: bool = True,
) -> Report:
    """Bind across the workspace. Never raises."""
    report = Report()
    repos = {path.name: path for path in discover_repos(root)}
    if only:
        # A freshly scaffolded repository may not have a Git directory yet.
        # Explicit targeting is sufficient to inspect that one directory;
        # workspace-wide discovery remains restricted to sibling Git repos.
        for name in only:
            candidate = root / name
            if candidate.is_dir():
                repos[name] = candidate
    for repo_path in (repos[name] for name in sorted(repos) if not only or name in only):
        report.add(bind_repo(repo_path, apply=apply, check_clean=check_clean))
    return report


# --- output ----------------------------------------------------


def format_detail(outcome: Outcome, verbose: bool) -> str:
    """The per-repository line: what was bound, what was left, what is open."""
    parts = []
    if verbose:
        for concern in outcome.concerns:
            if concern.status == BOUND:
                parts.append(f"{concern.concern}={concern.command} <- {concern.evidence}")
            elif concern.status == ALREADY_BOUND:
                parts.append(f"{concern.concern}=already-bound")
            elif concern.status == CONFLICT:
                parts.append(f"{concern.concern}=CONFLICT")
            else:
                parts.append(f"{concern.concern}=UNRESOLVED: {concern.reason}")
    return " | ".join(parts)
