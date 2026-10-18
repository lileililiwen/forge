# Design: Forge governance adapter shim

## Ownership and boundaries

The audit checker (`workspace_check.py`) owns finding semantics. The
adapter owns one translation: audit facts about exactly one project into
one Forge observation document. The adapter imports nothing from Forge and
Forge imports nothing from this repo; the boundary is the published v0.1.0
JSON contract.

## Request handling

stdin JSON must contain `contract == "0.1.0"`, `action == "check"`,
non-empty `project_id`, and `project_path`. Anything else → stderr note,
exit 2, no stdout. The request is the only input; the adapter never trusts
`project_path` for locating the registry — the registry root is the
adapter's own repository (resolved from `__file__`), while `project_path`
is only carried into evidence if the audit itself references it.

## Evaluation

1. Locate the project entry in `projects.json` (id match). Not found →
   `status: unknown`, detail "not registered in workspace registry".
2. Run the in-process audit for that project (reuse
   `workspace_check.load_projects`/check functions or invoke the CLI with
   `--project <id> --json` if that surface exists — pick one and pin it in
   tests) and collect findings whose project equals the requested id.
3. Status rule: any ERROR-level finding → `fail`; otherwise
   `adoption != adopted` or DISCOVERED_UNREGISTERED → `blocked`; otherwise
   → `pass`. WARN findings never downgrade `pass` but are included in
   evidence (so Forge can display them without this repo inventing a new
   status).
4. `source_revision`: `git -C <workspace_root>/<path> rev-parse HEAD` when
   the directory is a git repo; omitted otherwise (Forge falls back).

## Observation output

```json
{"provider": "workspace-governance",
 "protocol_version": "0.1.0",
 "project_id": "<echoed>",
 "status": "pass|fail|blocked|unknown",
 "source_revision": "...",
 "evidence": ["AUDIT-…: message (≤200 chars, secret-scrubbed)", "..."],
 "detail": "n errors, m warnings, adoption=<state>",
 "metadata": {"registry_schema_version": 1, "observed_via": "workspace_check"}}
```

- Evidence capped (≤16 lines) after ERROR-first ordering; every line
  scrubbed by the existing SECRET_WORDS pattern → `[redacted]`.
- Exactly one JSON document on stdout, nothing else; UTF-8, trailing
  newline.
- Governance failures (status fail/blocked) still exit 0 — the contract
  reserves non-zero exits for adapter-unavailability so Forge can
  distinguish "the portfolio says no" from "the portfolio could not be
  asked".

## Read-only guarantee

The script performs no file writes and no registry mutation; smoke test
hashes the repo tree before/after a sample invocation.

## Testing and rollout

- `tests/test_forge_governance_adapter.py`: synthetic registry + findings
  fixtures for each status; malformed stdin; unknown project; secret
  scrubbing; exit-code rule.
- README: one paragraph under Optional consumers with the exact
  `forge governance use workspace-governance . --adapter
  <repo>/scripts/forge_governance_adapter.py` line.
- `workspace_check.py --self-check` and the unittest discovery pick the new
  test up automatically (standard naming).

## Failure classification summary

| Condition | stdout | exit |
| --- | --- | --- |
| valid request, any governance status | observation | 0 |
| malformed/unknown contract request | — | 2 |
| audit internal error | — | 3 (Forge → unavailable) |
