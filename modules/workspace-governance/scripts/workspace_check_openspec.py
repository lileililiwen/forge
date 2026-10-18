"""OpenSpec purpose, seriality, handoff, and gate-debt validation."""

from __future__ import annotations

import workspace_check as _facade
globals().update({key: value for key, value in vars(_facade).items() if not key.startswith("__")})

def validate_spec_purposes(project_path: Path, findings: list[Finding], project_id: str) -> None:
    """Report a spec whose `## Purpose` still holds the archiver's TBD marker.

    Archiving a change writes `TBD - created by archiving change <name>.` into the
    `## Purpose` of every spec file that change produced, so a spec whose purpose
    is literally the string `TBD` reads as a finished spec. The level is fixed at
    ERROR by ruling 1: the marker is an unfinished statement presented as a
    settled one, and a warning nobody must clear is how the 1,234 of them
    accumulated in the first place.

    The finding names the spec file relative to its repository and the change the
    marker records, so the remediation is "resolve this spec from this change"
    rather than "look at a list of them". The change name is whatever the marker
    says, in either spelling: the archiver has emitted both the bare change name
    and a date-prefixed one, and this rule does not resolve the archive —
    `scripts/spec_purpose_backfill.py` does that, and guessing here would report a
    change the spec did not come from.

    A repository with no `openspec/specs` tree is skipped, and the rule is
    deliberately not gated on `profile_requirements`: a descriptive rule must
    not become a requirements driver.
    """
    specs_root = project_path / SPEC_PURPOSE_SPECS_DIR
    if not specs_root.is_dir():
        return
    for spec in sorted(specs_root.rglob("spec.md")):
        if not spec.is_file() or spec.is_symlink():
            continue
        try:
            text = spec.read_text(encoding="utf-8")
        except OSError as exc:
            finding(findings, "WARNING", "SPEC_UNREADABLE",
                    f"cannot read spec file: {exc}", project_id)
            continue
        match = SPEC_PURPOSE_PLACEHOLDER_RE.search(text)
        if match is None:
            continue
        marker = match.group(1).strip()
        change = SPEC_PURPOSE_CHANGE_RE.match(marker)
        origin = f" archived from change {change.group('name')!r}" if change else ""
        finding(
            findings,
            SPEC_PURPOSE_PLACEHOLDER_LEVEL,
            SPEC_PURPOSE_PLACEHOLDER,
            f"{spec.relative_to(project_path).as_posix()} holds the archived-change Purpose "
            f"marker {marker!r}{origin}; a spec's purpose is real prose, not the archiver's TBD",
            project_id,
        )


def validate_declared_verification_is_bound(
    project_path: Path, findings: list[Finding], project_id: str
) -> None:
    """Report a declared `verification.command` that no Gate actually runs.

    The existing `VERIFICATION_COMMAND` rule asks the opposite question — is a
    command *declared*? — and all 81 repositories satisfy it, so it catches none
    of the 70 whose `.ai-gate/gate.yaml` binds no command and whose gate
    therefore executes nothing. Ruling D18 calls that documentation: a verifier
    that no gate runs does not gate.

    Both halves must be present for a binding to be real, and this rule checks
    both, because the manifest parser refuses one without the other:

    * `checks:` must select `declared-verification` as `true` — a selection of
      `false` binds nothing, and the generator refuses to override it;
    * `commands:` must bind that concern.

    A repository with no `.ai-gate/gate.yaml` is reported too, and distinctly:
    there is no manifest to bind into, which is a different defect from a
    manifest that declines to bind. The message says which, so the remediation
    is not "add a command" for a repository that has nowhere to put one.

    The reading is line-based on purpose. The concern id is a fixed constant
    written by `scripts/bind_verification_command.py`, and the rule reads the
    same constant, so the two cannot disagree about what "bound" means. A full
    YAML parse is not used because a manifest carrying an unknown top-level
    field fails to parse at all — 11 of the 76 in the fleet do — and a rule
    that crashed on those would be blind exactly where the state is worst.
    """
    manifest = project_path / GATE_MANIFEST_RELATIVE
    command = _declared_verification_command(project_path)
    if command is None:
        return

    if not manifest.is_file():
        native = project_path / "gate.toml"
        where = (
            f"uses a native {native.name}, which this rule does not read; its "
            f"declared command must be bound there"
            if native.is_file()
            else f"has no {GATE_MANIFEST_RELATIVE} to bind it into"
        )
        finding(
            findings,
            QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL,
            QUALITY_VERIFICATION_COMMAND_UNBOUND,
            f"declares verification.command {command!r} but {where}; "
            "the command is documentation, not a gate",
            project_id,
        )
        return

    try:
        lines = manifest.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        finding(
            findings,
            QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL,
            QUALITY_VERIFICATION_COMMAND_UNBOUND,
            f"declares verification.command but {GATE_MANIFEST_RELATIVE} cannot be read: {exc}",
            project_id,
        )
        return

    checks = _yaml_block(lines, "checks")
    commands = _yaml_block(lines, "commands")
    if checks is None:
        # No selection block at all: nothing could be bound, whatever
        # `commands:` says.
        finding(
            findings,
            QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL,
            QUALITY_VERIFICATION_COMMAND_UNBOUND,
            f"declares verification.command {command!r} but {GATE_MANIFEST_RELATIVE} "
            "selects no `checks:` block, so a bound command could never run",
            project_id,
        )
        return

    selected = _yaml_entry(checks, DECLARED_VERIFICATION_CONCERN)
    if selected != "true":
        detail = "does not select it" if selected is None else f"selects it as {selected!r}, not `true`"
        finding(
            findings,
            QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL,
            QUALITY_VERIFICATION_COMMAND_UNBOUND,
            f"declares verification.command {command!r} but {GATE_MANIFEST_RELATIVE} "
            f"{detail} under `checks:`",
            project_id,
        )
        return

    if _yaml_entry(commands or [], DECLARED_VERIFICATION_CONCERN) is None:
        finding(
            findings,
            QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL,
            QUALITY_VERIFICATION_COMMAND_UNBOUND,
            f"declares verification.command {command!r} but {GATE_MANIFEST_RELATIVE} "
            f"binds no `commands.{DECLARED_VERIFICATION_CONCERN}`; the gate runs nothing",
            project_id,
        )


def _declared_verification_command(project_path: Path) -> str | None:
    """The repository's own declared command, or None when it declares none.

    Reads the same field the generator reads, with the same refusals: an
    unparseable or absent declaration yields None, and this rule then has
    nothing to report. A rule that reported "declares a command it does not
    declare" would be the rule's own bug, not the repository's.
    """
    declaration_path = project_path / ".project.json"
    if not declaration_path.is_file():
        return None
    try:
        data = json.loads(declaration_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if not isinstance(data, dict):
        return None
    verification = data.get("verification")
    if not isinstance(verification, dict):
        return None
    command = verification.get("command")
    if not isinstance(command, str) or not command.strip():
        return None
    return command.strip()


def _yaml_block(lines: list[str], key: str) -> list[str] | None:
    """The indented lines of a top-level `key:` block, or None if absent."""
    try:
        start = next(i for i, line in enumerate(lines) if line.startswith(f"{key}:"))
    except StopIteration:
        return None
    end = len(lines)
    for index in range(start + 1, len(lines)):
        line = lines[index]
        if line.strip() and not line[0].isspace():
            end = index
            break
    return lines[start + 1 : end]


def _yaml_entry(block: list[str], key: str) -> str | None:
    """One `  <key>: <value>` entry's value inside a block, or None."""
    needle = f"  {key}:"
    for line in block:
        if line.startswith(needle):
            return line[len(needle) :].strip()
    return None


def unarchived_changes(project_path: Path) -> list[str]:
    """The repository's active OpenSpec change names, sorted.

    One walk, two rules: `validate_change_names` names the directories here
    that are not valid change ids, and `validate_new_change_with_gate_debt`
    counts the same set as new work in progress. A trigger the naming rule
    ignores would be a trigger nobody can fix by renaming a directory.

    `archive/` is excluded because an archived change is finished work, and a
    dot-prefixed directory because OpenSpec keeps its own state there.
    """
    changes = project_path / "openspec" / "changes"
    if not changes.is_dir():
        return []
    return [
        child.name
        for child in sorted(changes.iterdir())
        if child.is_dir() and child.name != "archive" and not child.name.startswith(".")
    ]


def validate_change_names(project_path: Path, findings: list[Finding], project_id: str) -> None:
    for name in unarchived_changes(project_path):
        if not CHANGE_ID.fullmatch(name):
            finding(findings, "ERROR", "OPENSPEC_NAME", f"invalid active OpenSpec change name: {name}", project_id)


def validate_openspec_seriality(
    project_path: Path, findings: list[Finding], project_id: str
) -> None:
    """Require one active OpenSpec change and a matching local handoff pointer."""
    if project_id != "workspace-governance":
        return

    changes = unarchived_changes(project_path)
    if len(changes) > 1:
        finding(
            findings,
            "ERROR",
            OPENSPEC_CHANGE_SERIALITY,
            "multiple active OpenSpec changes are not allowed; finish and archive "
            f"the current change before starting another: {', '.join(changes)}",
            project_id,
        )
        return

    handoff_path = project_path / "HANDOFF.md"
    if not handoff_path.exists():
        if changes:
            finding(
                findings,
                "ERROR",
                OPENSPEC_CHANGE_SERIALITY,
                f"HANDOFF.md is missing; current_spec must name the active change {changes[0]!r}",
                project_id,
            )
        return
    try:
        handoff = handoff_path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as exc:
        expected = (
            f"active change {changes[0]!r}"
            if changes
            else "stale current_spec state"
        )
        finding(
            findings,
            "ERROR",
            OPENSPEC_CHANGE_SERIALITY,
            f"HANDOFF.md cannot be read to verify {expected}: {exc}",
            project_id,
        )
        return

    pointers = re.findall(
        r"^[ \t]*current_spec[ \t]*:[ \t]*(.*?)[ \t]*$",
        handoff,
        flags=re.MULTILINE,
    )
    normalized = [value.strip().strip("\"'").strip() for value in pointers]
    if len(changes) == 1:
        expected = changes[0]
        if not normalized:
            problem = "current_spec is missing; it must appear exactly once"
        elif len(normalized) > 1:
            problem = "current_spec appears more than once; it must appear exactly once"
        elif normalized[0] != expected:
            problem = f"current_spec is {normalized[0]!r}; it must name the active change {expected!r}"
        else:
            return
    else:
        if len(normalized) > 1:
            problem = "current_spec appears more than once with no active change"
        elif normalized and normalized[0].lower() not in {"", "none"}:
            problem = f"current_spec {normalized[0]!r} is stale because no change is active"
        else:
            return

    finding(
        findings,
        "ERROR",
        OPENSPEC_CHANGE_SERIALITY,
        f"{problem}; active changes: {', '.join(changes) if changes else 'none'}",
        project_id,
    )


def live_unbound_gate_checks(project_path: Path) -> tuple[tuple[str, ...] | None, str | None]:
    """Read current unbound concerns with the binder's required-check rules.

    This reads manifest declarations only: it does not derive or run commands,
    write the manifest, or update the workspace debt snapshot.
    """
    manifest_path = project_path / ".ai-gate" / "gate.yaml"
    try:
        text = manifest_path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        return None, f"cannot read {manifest_path.relative_to(project_path)}: {exc}"

    try:
        from bind_gate_commands import (
            PROFILE_DEFAULTS,
            block_entries,
            gate_required,
            read_gate_policy,
            read_manifest,
            top_level_block_bounds,
            unquote_scalar,
        )

        selected, commands, problem = read_manifest(text)
        if problem:
            return None, problem
        policy = read_gate_policy(text)
        if policy.profile not in PROFILE_DEFAULTS:
            return None, f"unsupported gate profile {policy.profile!r}"
        lines = text.splitlines()
        for top_level_key in ("profile", "checks", "commands", "not_applicable"):
            if sum(line.startswith(f"{top_level_key}:") for line in lines) > 1:
                return None, f"duplicate top-level `{top_level_key}:` declaration"
        for block_name in ("checks", "commands", "not_applicable"):
            bounds = top_level_block_bounds(lines, block_name)
            if bounds is None:
                continue
            start, end = bounds
            for line in lines[start + 1:end]:
                stripped = line.strip()
                if not stripped or stripped.startswith("#"):
                    continue
                if (
                    not line.startswith("  ")
                    or line.startswith("   ")
                    or stripped.startswith("-")
                    or ":" not in stripped
                ):
                    return None, f"malformed `{block_name}:` mapping entry: {stripped!r}"
                key, value = stripped.split(":", 1)
                if not key or key != key.strip() or not value.strip():
                    return None, f"malformed `{block_name}:` mapping entry: {stripped!r}"
            entries = block_entries(lines, block_name)
            names = [name for name, _value in entries]
            if len(names) != len(set(names)):
                return None, f"duplicate entry in `{block_name}:` mapping"
            if block_name == "checks" and any(
                value not in {"true", "false", "optional"}
                for _name, value in entries
            ):
                return None, "invalid value in `checks:` mapping"
            if block_name == "not_applicable" and any(
                not unquote_scalar(value).strip() or value.lstrip().startswith("#")
                for _name, value in entries
            ):
                return None, "empty reason in `not_applicable:` mapping"
        if top_level_block_bounds(lines, "checks") is None:
            # Inline empty checks are valid; every other absent checks block
            # is rejected by read_manifest above.
            if not any(line.strip() == "checks: {}" for line in lines):
                return None, "missing `checks:` mapping"
        required = gate_required(selected, policy)

        def has_command(concern: str) -> bool:
            raw = commands.get(concern, "").strip()
            value = unquote_scalar(raw).strip()
            if not raw or raw.startswith("#") or not value:
                return False
            if raw[0] not in {"'", '"'} and value.lower() in {"null", "~"}:
                return False
            return True

        unbound = tuple(sorted(concern for concern in required if not has_command(concern)))
        return unbound, None
    except Exception as exc:  # noqa: BLE001 - unresolved policy must fail closed
        return None, f"cannot resolve the gate manifest: {exc}"


def validate_new_change_with_gate_debt(
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    debt: dict[str, tuple[str, ...]] | None,
    baseline: frozenset[tuple[str, str]] | None,
    exceptions: dict[tuple[str, str], str] | None,
) -> None:
    """Block new work on a repository that still carries gate debt.

    22 repositories record at least one required check the gate reports as
    `UNBOUND`. The record says so; nothing acted on it, so the debt sits still
    while the next feature starts on top of it. This rule is the acting.

    The record remains the source for already-known debt. For a repository
    with active work, the current manifest is also read through the binder's
    required-check parser; an unbound concern missing from the snapshot is a
    live increase and blocks the new change immediately.

    Three ways to *not* be new work, and the second is what keeps the rule a
    ratchet rather than a deadlock:

    - a change named in `GATE_DEBT_CLOSING_CHANGES` is not new work at all: it
      *is* the work that clears the debt, and a rule that blocked it would make
      the debt permanent;
    - a `(project, change)` pair in `GATE_DEBT_BASELINE_FILE` was already open
      when the rule landed. Failing it fails the legacy fleet for the state the
      rule exists to stop growing, and cannot be discharged by any action
      available to the repository. The baseline is a committed list, so this is
      a reviewable one-time grandfathering and not a switch;
    - a pair in `GATE_DEBT_EXCEPTIONS_FILE` is a deliberate decision about new
      work, reported at `WARNING` with its reason quoted so the decision stays
      visible in the run.

    `debt=None` means the debt record could not be read, and `baseline=None`
    that the ratchet could not. The caller has already reported each once as a
    run-level `ERROR`, which is what fails the run; `baseline=None` grandfathers
    nothing, because a ratchet that could not be read grandfathers nothing.
    """
    changes = unarchived_changes(project_path)
    if not changes or debt is None:
        return

    recorded = debt.get(project_id, ())
    current, current_error = live_unbound_gate_checks(project_path)
    increased = (
        tuple(sorted(set(current or ()) - set(recorded)))
        if current_error is None
        else ()
    )
    concerns = tuple(dict.fromkeys((*recorded, *increased)))
    if not concerns and current_error is None:
        return

    closing = ", ".join(sorted(GATE_DEBT_CLOSING_CHANGES))
    unbound = ", ".join(concerns)
    for change in changes:
        if change in GATE_DEBT_CLOSING_CHANGES:
            continue
        if baseline is not None and (project_id, change) in baseline:
            continue
        reason = (exceptions or {}).get((project_id, change))
        if reason is not None:
            finding(
                findings,
                QUALITY_NEW_CHANGE_WITH_GATE_DEBT_SATISFIED_LEVEL,
                QUALITY_NEW_CHANGE_WITH_GATE_DEBT,
                f"opens change {change!r} while required check(s) {unbound} are unbound; "
                f"satisfied by a recorded exception in {GATE_DEBT_EXCEPTIONS_FILE}: {reason}",
                project_id,
            )
            continue
        if current_error is not None:
            finding(
                findings,
                QUALITY_NEW_CHANGE_WITH_GATE_DEBT_LEVEL,
                QUALITY_NEW_CHANGE_WITH_GATE_DEBT,
                f"opens change {change!r} but the checker could not establish live gate state: {current_error}; "
                f"resolve the manifest before starting new work",
                project_id,
            )
            continue
        if increased:
            finding(
                findings,
                QUALITY_NEW_CHANGE_WITH_GATE_DEBT_LEVEL,
                QUALITY_NEW_CHANGE_WITH_GATE_DEBT,
                f"opens change {change!r} while live required check(s) {', '.join(increased)} are unbound "
                f"and absent from gate-debt.json; bind them and regenerate the debt snapshot before starting new work",
                project_id,
            )
            continue
        finding(
            findings,
            QUALITY_NEW_CHANGE_WITH_GATE_DEBT_LEVEL,
            QUALITY_NEW_CHANGE_WITH_GATE_DEBT,
            f"opens change {change!r} while required check(s) {unbound} are unbound; "
            f"bind them under {closing} first, or record a deliberate exception "
            f"in {GATE_DEBT_EXCEPTIONS_FILE} before starting new work",
            project_id,
        )


def validate_handoff_quality(
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    rules_drift_baseline: frozenset[tuple[str, str, str]] | None = None,
    observed_drift_keys: set[tuple[str, str, str]] | None = None,
    scanned_projects: set[str] | None = None,
) -> None:
    """Surface a scaffold `HANDOFF.md` and gate on rules drift that is not baselined.

    Two distinct surfaces meet here, deliberately:

    - `scripts/rules_drift.py --check` is the **detailed report**. It names every
      clause a repository does not visibly carry, and it never rewrites a project.
    - this function is the **blocking delta**. It reports only a finding whose
      `(project, file, label)` key is absent from `rules-drift-baseline.json`, so a
      repository that documents the shared rules in its own words stays silent and a
      newly introduced gap blocks. A key that is present in the baseline is known
      debt, not a new failure.

    A repository's legitimate escape is the per-file
    `<!-- workspace-governance:rules-customized -->` marker, which
    `rules_drift.OPT_OUT` honours before any clause is evaluated. There is no
    environment variable, no skip flag, and no exemption registry: a check that can
    be switched off is not a check.
    """
    import rules_drift  # local import to keep this module standalone

    if rules_drift.scaffold_handoff(project_path):
        finding(findings, "WARNING", "HANDOFF_SCAFFOLD", "HANDOFF.md still carries the initializer scaffold", project_id)

    if scanned_projects is not None:
        scanned_projects.add(project_id)

    if rules_drift_baseline is None:
        return

    for item in rules_drift.drift(project_path):
        if item["code"] != "RULES_DRIFT":
            # RULES_UNREADABLE is a read failure, not a clause key. It is reported
            # by the detailed report and must never be ratcheted into the baseline.
            continue
        key = (project_id, item["file"], item["label"])
        if observed_drift_keys is not None:
            observed_drift_keys.add(key)
        if key not in rules_drift_baseline:
            finding(
                findings,
                "ERROR",
                "RULES_DRIFT_NEW",
                f"{item['file']} does not carry the {item['label']}, and this finding is not in rules-drift-baseline.json",
                project_id,
            )
