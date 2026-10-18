# Design: Declared interfaces and boundary findings

## Runtime and boundaries

- Runtime: Python 3.11; no new dependency. The declaration is validated with the
  JSON Schema already shipped at `schemas/project.schema.json` and read by
  `scripts/workspace_check.py`.
- Schema owner: this repository (`schemas/project.schema.json`). The interface
  *entry schema* is deliberately not split out to `platform-contracts`: decision
  D3 splits the **capability** entry schema (a `platform.*` wire family) from the
  capability **instances**, and an `interfaces[]` block inside the project
  declaration is an instance. When an interface descriptor must travel between
  systems, `platform-contracts` owns its envelope — out of scope here.
- Registry: `capabilities.json` is unchanged. Its `posture` vocabulary gains no
  value; this change only adds a reporter for exemptions whose provider has not
  declared verified evidence.
- Generators: `scripts/bind_gate_commands.py`, `bind_ui_gate.py` and
  `bind_verification_command.py` remain the only writers of a sibling-path
  command, and become the only writers of the matching `mechanism: sibling`
  declaration.

## The declaration

`interfaces` is an optional array. An absent array means "nothing declared" and
produces no finding — the 82 existing declarations stay valid and silent.

```jsonc
"interfaces": [
  {
    "id": "gate-result",              // kebab-case, unique within the project
    "provides": true,                 // exactly one of provides/consumes per entry
    "mechanism": "invoke",            // invoke | consume | observe | sibling
    "entry": "driftwatchdog gate --format json",
    "contract": "platform.gate-result/1.0.0",
    "digest": "sha256:…",             // required for mechanism consume/observe
    "required": true                  // consumer side; also the sibling-lawful flag
  },
  {
    "id": "platform-ui-tokens",
    "consumes": true,
    "provider": "dotnet-platform-libs",  // must declare a matching provides entry
    "mechanism": "consume",
    "entry": "@platform/design-tokens",
    "contract": "@platform/design-tokens/0.1.0",
    "digest": "sha256:…",
    "required": false
  }
]
```

Rules the schema enforces, and the audit re-checks against reality:

1. `id` matches `^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$` and is unique per project.
2. Exactly one of `provides` / `consumes` is true.
3. `mechanism` is one of four **closed** values. `invoke` = bounded argv plus one
   versioned JSON document on stdin/stdout; `consume` = a published artifact
   pinned by version and digest; `observe` = a read-only document at a declared
   path, digest-pinned; `sibling` = a repository-relative invocation of another
   checkout, lawful only under rule 6.
4. `contract` is `"<name>/<major>.<minor>.<patch>"` with a kebab-or-dotted name;
   a bare name without a version is invalid. `contract` is required for every
   mechanism except `sibling`.
5. `digest` is `"sha256:<64 hex>"` and is required for `consume` and `observe`.
6. `mechanism: sibling` requires the consumer to also declare the same provider
   `required: true` **and** to carry a `digest`; otherwise the edge is unlawful
   and the finding names both lawful alternatives.
7. `entry` uses the vocabulary of its mechanism (argv string, package name, URL,
   or a repository-relative path that satisfies the existing path rules — no
   absolute path, no `..` escape, both already implemented as
   `STANDALONE_ABSOLUTE_PATH` / `STANDALONE_PATH_ESCAPE`).

## Findings

Codes are stable strings, declared in `capability-adoption-gate`'s vocabulary so
no reviewer has to guess a code's meaning.

| Code | Severity | Trigger |
|---|---|---|
| `INTERFACE_DECLARATION_INVALID` | ERROR | schema violation (id, exclusive flags, missing contract, bad digest form) |
| `INTERFACE_MECHANISM_UNKNOWN` | ERROR | `mechanism` outside the closed set |
| `INTERFACE_CONTRACT_UNVERSIONED` | ERROR | a non-`sibling` edge naming a contract without a version |
| `INTERFACE_PROVIDER_UNDECLARED` | WARNING | a `consumes` entry whose provider declares no matching `provides` entry (provider may be outside the registry) |
| `INTERFACE_CONSUMED_UNUSED` | WARNING | a declared consumption edge with no `evidence_ref` pointing at the consumer's own use |
| `INTERFACE_SIBLING_PATH_UNPINNED` | WARNING, ramps to FAIL on a recorded date | a `sibling` edge without a digest or without `required: true` |
| `INTERFACE_VENDORED_WITHOUT_DIGEST` | WARNING, ramps to FAIL on a recorded date | a byte copy of another project's artifact not declared as `consume` with a digest |
| `CAPABILITY_PROVIDER_UNVERIFIED` | WARNING | a capability registry entry still `exempt / provider-unavailable` because its provider declares no verified entry |

The ramp follows the existing pattern in `capability-adoption-gate`
("Escalate on a recorded WARN to FAIL ramp"): the date is recorded in
`decisions.json` as a new decision, so the escalation is an owner decision that
is visible and revocable, not a silent tightening. Nothing becomes blocking
before the fleet has had a release cycle to declare.

## Flow and invariants

1. Load the declaration with the existing loader; validate it against the extended
   schema. A declaration that fails validation yields `INTERFACE_DECLARATION_INVALID`
   and no interface assertions at all (never a partial read).
2. Build the interface index across the registry: `provides` entries indexed by
   `(project id, interface id)`; `consumes` entries resolved against it.
3. Assert the rules above per entry; emit findings with the code, the interface
   id, and a remediation naming the smallest lawful change.
4. Cross-check the pairing: an edge is *declared* only when both sides declare it;
   a one-sided edge is `INTERFACE_PROVIDER_UNDECLARED`.
5. Report `CAPABILITY_PROVIDER_UNVERIFIED` for each registry entry whose posture
   is `exempt` and whose provider declares no `verified` capability record, so the
   nine current exemptions have a named owner and a next action.

Invariants: the audit stays read-only; no declaration is written by the audit;
no finding is emitted for a project that declares no interfaces; a declaration is
never partially honoured; and no mechanism becomes lawful merely because a tool
accepts it.

## Generators

`bind_gate_commands.py --apply` writes two files into each sibling today
(`.project.json`, `.ai-gate/gate.yaml`). It gains one responsibility: when it
binds a command that reaches into another checkout, it also writes the matching
`interfaces[]` entry (`mechanism: sibling`, `provider`, `required: true`) and a
digest of the *command string* it wrote, so the 67 existing bindings become
declared edges instead of unlawful ones. A binding it cannot describe this way
stays absent and keeps the concern `UNBOUND` — the existing debt path, not a
fabricated declaration.

## Failure behavior

- Schema violation: `INTERFACE_DECLARATION_INVALID`, ERROR, no partial assertions.
- Unknown mechanism: ERROR, and the entry is otherwise ignored.
- Missing provider for a declared consumption: WARNING naming the provider id.
- A `sibling` edge without a digest: WARNING naming both lawful alternatives
  (declare `consume` with a digest, or publish an artifact).
- Absent `interfaces`: no findings, exit unchanged.
- Any classification error is a finding, never an exception that aborts the audit.

## Rollback

The change is additive and reversible in one step per surface:

- Schema: remove the `interfaces` property and its `$defs`; the 82 declarations
  are unaffected either way because the block is optional and no other file
  reads it yet.
- Audit: remove the interface pass and the new codes from the vocabulary; no
  persisted state depends on them (the audit stores no interface data).
- Generators: stop writing the declaration alongside the command; the command
  itself is preserved so no repository loses a binding.

No fleet-wide file rewrite is part of this change, so there is no rollback
manifest to keep. The generator change is scoped to the repositories it targets
and is verified against a temporary scaffold before any sibling is touched.

## Verification oracle

- Unit: schema validation accepts each mechanism and rejects an unknown one, a
  version-less contract, a `sibling` edge without a digest, and a non-exclusive
  entry; findings carry the exact codes above.
- Fixture: one project declaring `provides`/`consumes` pairs of each mechanism,
  including a deliberately unlawful `sibling` edge, asserted to produce exactly
  one `INTERFACE_SIBLING_PATH_UNPINNED`.
- Registry: the nine current exemptions each produce
  `CAPABILITY_PROVIDER_UNVERIFIED` until their provider declares verified
  evidence.
- Non-regression: the full Python suite (948 tests at the time of writing), the
  workspace self-check, the auto-gate, and strict OpenSpec validation all stay
  green with zero new findings on the 82 existing declarations.
- Fleet dry run: bind a temporary scaffold and assert exactly one declared
  `sibling` edge is written next to the command, with the same digest the audit
  recomputes.
