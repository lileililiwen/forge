# Design: ui-convention-guidance

## Implementation boundary

Workspace Governance owns the agent route and the repository-local Gate binder.
The change modifies only:

- `skills/workspace-governance/SKILL.md` and
  `skills/workspace-governance/references/ui-convention.md`;
- `scripts/bind_ui_gate.py` and `tests/test_bind_ui_gate.py`;
- `docs/ui-convention.md`;
- this OpenSpec change and its promoted canonical spec.

The binder accepts exactly one target repository path. It reads and may update
only that repository's `.ai-gate/gate.yaml`. It does not edit the shared checker,
the frozen baseline, UI feed assets, or any application source.

## Language, runtime, and commands

The binder uses Python 3 and the Python standard library. It preserves the
target YAML as text because the Gate file may contain comments and hand-authored
ordering. The command runs from a repository checkout in the workspace, with
the Workspace Governance checkout at the repository-relative path
`../workspace-governance`.

Supported commands:

```text
python3 scripts/bind_ui_gate.py <repository> [--check | --apply] [--json]
```

No mode writes by default. `--check` reports whether changes are needed and
returns non-zero when the manifest is unbound. `--apply` performs the write.
`--json` emits the same outcome as a machine-readable object. The existing
`scripts/ui_conformance.py` remains the checker and is invoked by the target
repository's own Gate after binding.

## Ownership and shared architecture

Workspace Governance owns policy, the checker vocabulary, and Gate integration.
`dotnet-platform-libs` owns the token/component packages and feed digests.
Forge owns project generation. Product repositories own their UI scope,
branding, contrast evidence, and adoption declaration. This change extends the
governance owner with an agent route and a per-repository binder; it does not
copy packages or checker semantics into another owner.

The generated Gate command is the established workspace-relative form:

```yaml
checks:
  ui-conformance: true
commands:
  ui-conformance: 'python3 ../workspace-governance/scripts/ui_conformance.py .'
```

This command requires the Workspace Governance checkout at the documented
sibling location. It uses no environment variable, absolute path, shell
expansion, or external service. Standalone consumers that do not retain this
workspace layout must vendor or otherwise provision the checker in a separate
change; the binder does not silently invent that distribution boundary.

## Behavioral model

| Input state | `--check` or default | `--apply` |
|---|---|---|
| Gate file missing | `NO_MANIFEST`, non-zero, no write | `NO_MANIFEST`, non-zero, no write |
| Manifest is not a recognized `workspace-governance@…` rule pack | `REFUSED` with the missing/mismatched top-level key, non-zero | Same refusal, no write |
| `ui-conformance` entries absent and manifest shape valid | `BOUND` plan reported and nothing written; default exits zero as a preview, `--check` exits non-zero because a change is needed | Write only the two required entries and report `BOUND` |
| Both entries already match | `ALREADY_BOUND`, zero | `ALREADY_BOUND`, zero; bytes unchanged |
| Existing selection or command differs | `CONFLICT`, non-zero | `CONFLICT`, non-zero; preserve existing values |

The operation is idempotent. It preserves unrelated comments, keys and order.
It never creates a missing Gate file, resolves a second repository, or walks a
workspace. A failed or conflicting operation leaves the target bytes unchanged.

## Contract and compatibility

The stable check ID is `ui-conformance`. The binder adds
`checks.ui-conformance: true` and the fixed command shown above. The target Gate
must contain top-level `version`, `runtime`, and `rule_pack` keys, and the
`rule_pack` value must begin with `workspace-governance@`. Existing non-matching
values are operator intent and are reported rather than overwritten.

The CLI outcome vocabulary is `BOUND`, `ALREADY_BOUND`, `CONFLICT`, `REFUSED`,
and `NO_MANIFEST`. Human output names the repository and reason; JSON includes
the status, detail, whether a write occurred, and keys added. A default dry run
exits zero as a preview of what it would write; `--check` exits non-zero on a
repository needing a change and zero only when already bound; refusal and
conflict are non-zero in every mode.

## User experience and accessibility

The human workflow starts from the skill routing row, continues in the
UI-convention reference, and ends at the read-only checker or the explicit
binder command. Dry-run output precedes a write. The operator can inspect one
repository and decline to apply without side effects. Conflicts and malformed
manifests name the reason and leave files untouched. JSON is available for
automation without changing the human-readable default.

This change has no browser or mobile screen. The CLI has conventional terminal
text, a non-zero exit on refusal, and machine-readable JSON. Screen-reader and
keyboard interaction are provided by the invoking terminal; browser viewport,
color contrast, and reduced-motion checks are not applicable.

## Verification oracle

The package is complete when:

- `tests/test_bind_ui_gate.py` proves unbound dry-run, apply, byte-identical
  second apply, check exit behavior, missing/unknown manifest refusal,
  conflicting-entry preservation, repository isolation, and JSON output;
- `python3 scripts/check_skill_refs.py` proves the routing row resolves;
- `python3 scripts/bind_ui_gate.py --check` on a scratch repository proves the
  dry-run does not write, and apply followed by apply proves idempotence;
- strict change and canonical-spec validation pass;
- `git diff --check` is clean.

The UI checker itself remains separately owned and is not re-tested by the
binder's unit suite. Consumer conformance is not claimed by this change.

## Decision ledger

- The feed remains vendored per the existing D1 ruling.
- The existing seven checker codes and the frozen baseline remain authoritative;
  this package documents them without introducing another vocabulary.
- Flutter/Dart remains the existing D3 exemption until a Dart artifact exists.
- `branding.json` remains consumer-owned per D4.
- Migration, project generation, and fleet binding are separate follow-up
  changes; no product repository is silently modified here.
