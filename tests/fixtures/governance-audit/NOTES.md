# Workspace Governance audit-shaped fixtures

Fixture seeds captured live from the real sibling adapter on this host
(`workspace-governance` at HEAD `204d140`, `scripts/forge_governance_adapter.py`
running the real `workspace_check` audit over the real `projects.json`
registry, `WORKSPACE_ROOT=/home/paul/code`), 2026-09-24. Each stub re-emits
the captured observation document verbatim; only the project id is echoed
from the request stdin (the real adapter echoes the requested id, and Forge
refuses a mismatched echo as `incompatible` before status mapping).

Commands used for the captures:

```sh
printf '{"contract":"0.1.0","action":"check","project_id":"<id>","project_path":"/home/paul/code/<id>"}' \
  | WORKSPACE_ROOT=/home/paul/code python3 <sibling>/scripts/forge_governance_adapter.py
```

| Stub | Captured project | Status | Verbatim evidence / detail |
| --- | --- | --- | --- |
| `adopted-clean.sh` | `forge` | `pass` | no evidence; `0 errors, 0 warnings, adoption=adopted`; `source_revision` echoed from the git repo |
| `error-findings.sh` | `crossalheart` | `fail` | `DECLARATION_MISSING: adopted project must contain .project.json`; `1 errors, 0 warnings, adoption=unknown` |
| `adoption-gap.sh` | `argoset` (scratch registry copy, `adoption` field removed) | `blocked` | `0 errors, 0 warnings, adoption=unknown` |
| `unregistered.sh` | invented id | `unknown` | `PROJECT_UNKNOWN: project not found in registry: <id>`; `not registered in workspace registry`; **no** `source_revision` key |

## Findings that corrected the design's assumptions

- **Packaged candidate layout.** The design guessed the adapter candidate at
  `<workspace-root>/scripts/forge_governance_adapter.py`. The real layout
  nests the checkout *inside* the portfolio it governs (`--root ..`), so the
  packaged candidate is
  `<workspace-root>/workspace-governance/scripts/forge_governance_adapter.py`
  and the root itself is the same value the adapter's own `WORKSPACE_ROOT`
  environment input takes. The preset resolves the real candidate against the
  real portfolio and hands that root to the adapter at run time, so later
  checks do not depend on the environment variable staying set.
- **`DISCOVERED_UNREGISTERED` vs `PROJECT_UNKNOWN`.** The design's example
  evidence code `DISCOVERED_UNREGISTERED` appears only for a folder discovered
  on disk that is absent from the registry; an invented id with no folder
  yields the checker's `PROJECT_UNKNOWN` code instead. Fixtures carry the real
  codes.
- **`blocked` needs an adoption gap without an ERROR.** Every real registry
  entry that is *not* adopted also carries an audit ERROR, and the adapter's
  `status_for` orders ERROR first, so a genuine `blocked` capture required a
  scratch registry copy removing `adoption` from a clean project
  (`argoset`) rather than an observed production entry.
- **The sibling ships the adapter as git mode `100644` (no execute bit).**
  Its sibling scripts (`workspace_check.py`, `auto_gate.py`) are `100755`.
  Forge's v0.1.0 boundary spawns the adapter directly (`Command::new(adapter)`),
  which a `0644` file cannot satisfy (Linux returns `EACCES`, exit 126, on a
  `0644` shebang file). The preset therefore honestly refuses a non-executable
  candidate naming the exact path. Live evidence and the exact next action are
  recorded in `docs/provider-evidence.md` and `HANDOFF.md`.

## Secret boundary

The real adapter scrubs secret-shaped substrings to `[redacted]` before
emitting evidence. Forge re-applies `policy::redact_credentials` (marker
`[REDACTED]`) and the `MAX_EVIDENCE_CHARS` bound on every evidence line and
the detail, so a non-conforming or hostile adapter cannot push credentials
into observations — proven by `tests/governance_contract.rs`.

## Live round trip (workspace-governance-adapter-consumption, 2026-09-24)

Against a scratch portfolio whose `workspace-governance/scripts/` holds a
byte-identical copy of the sibling adapter (only the execute bit added, since
the committed file is `0644`) and whose project folders symlink to the real
`/home/paul/code/*` checkouts, `forge governance use workspace-governance
<project> --workspace-root <scratch-portfolio>` resolved the packaged
candidate and `forge governance status <project>` consumed the real audit end
to end:

- `forge` → `status: pass`, `source_revision: 888d8058e7a2615e0c6525a533463e5029d56487`
- `crossalheart` → `status: fail`, evidence `DECLARATION_MISSING: adopted
  project must contain .project.json`, `source_revision: 916f794204be9a26e20680dbd99527cde4afe14c`
- `argoset` (scratch `adoption` removed) → `status: blocked`, `source_revision:
  1c4177c5fdecaac837cea22f9a56adb31774c0cf`
- an invented id → `status: unknown`, evidence `PROJECT_UNKNOWN: project not
  found in registry: <id>`, no `source_revision`

With the same selection pointed at an adapter file that was then removed,
`forge governance status` reported `status: unavailable` with the bounded
detail naming the cause and local `governance list` continued unchanged. The
real sibling repository tree stayed byte-identical throughout.
