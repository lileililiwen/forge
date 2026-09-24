# Design: Workspace Governance adapter consumption

## Ownership and boundaries

Workspace Governance owns governance semantics: the portfolio registry, the
audit checker, the adapter script itself. Forge owns provider resolution,
bounded execution of the adapter, and observation normalization. The
existing v0.1.0 executable-adapter boundary (JSON request on stdin,
observation on stdout) does not change.

## Preset resolution

`forge governance use workspace-governance . [--workspace-root R]`:

1. If `--adapter` is given, it wins (current behavior, unchanged).
2. Else resolve `R` from `--workspace-root`, then `FORGE_WORKSPACE_ROOT`.
3. Candidate = `R/scripts/forge_governance_adapter.py`, must exist, be a
   regular file and be executable; otherwise the command refuses with a
   message naming the exact candidate path (operator adds `--adapter` or
   fixes the root).
4. No implicit upward directory search and no network lookup — discovery is
   a configuration input, not a guess.

The stored `.forge/providers.yaml` records the resolved absolute adapter
path plus provider id, so later checks do not depend on the environment
variable staying set, and switching providers still never rewrites
`forge.yaml` or registry identity.

## Audit-to-observation mapping (fixture contract)

Forge fixtures mirror the sibling adapter's mappings so both repos test the
same boundary: no ERROR findings → `pass`; ERROR finding for this project →
`fail`; adoption gap / registry gap → `blocked`; project unknown to the
registry → `unknown`. Evidence lines carry the audit codes
(e.g. `AUDIT-SECRET-WORD`, `DISCOVERED_UNREGISTERED`) truncated to the
existing bound and redacted.

## Failure isolation

Unresolved root, missing script, non-zero adapter exit, malformed JSON,
identity mismatch — all surface as today's `Unavailable`/`Incompatible`
observations with bounded detail. Local workflows keep running.

## Migration and compatibility

No schema change. Hosts that already configured the provider with an
explicit `--adapter` are untouched. `list`/`status`/`inspect` output shape
unchanged; only selection ergonomics are added.

## Verification

- Unit: resolution order, executable-bit and regular-file checks, refusal
  text naming the candidate path.
- Contract: four fixture adapters (python stub or shell stub) emitting real
  audit-shaped observations; assert normalized status, evidence and
  redaction across CLI, and byte-stable manifests.
- Docs: `docs/provider-evidence.md` gains the `workspace-governance` row
  with live/fixture rules; live run recorded only if the sibling repo is
  present and the adapter executes.
