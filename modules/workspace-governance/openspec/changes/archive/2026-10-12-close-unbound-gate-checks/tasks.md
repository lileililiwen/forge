# Tasks: close-unbound-gate-checks

**This is prerequisite work, not business work.** Nothing below is a product
task. Each entry is a decision owed on `workspace-governance`'s gate, and each must be closed
before product work proceeds, so that later gate results are read against a
baseline that is actually standing.

## 0. Preconditions

- [x] 0.1 The debt is read from the committed record
  `workspace-governance/gate-debt.json` — not from gate output parsed by hand,
  and not from a fresh binder run.
- [x] 0.2 Every reason in §1 is quoted from that record rather than restated,
  so this change and the record cannot disagree.
- [x] 0.3 This repository's `.ai-gate/gate.yaml` is the authorized write
  target; no `.project.json` or sibling manifest is changed.
- [x] 0.4 The owner is `lileililiwen`, as named in this repository's
  `.project.json` publication metadata. The owner delegated completion and
  archive of the active changes in the current request.

## 1. Close each unbound check

Each item names the file, the key, or the command. There is nothing to
re-derive.

### 1. `security` — UNBOUND

- **Original recorded reason:** "no Python security tool is declared in this
  repository's pyproject.toml or requirements*.txt, and a Python dependency
  audit is not in the standard library"
- **Current state:** `requirements-dev.txt` declares `pip-audit`; the Gate
  command scans that requirements file rather than the host environment.

**Close it — declare the tool, then bind it.**

- [x] 1.1 Confirm the existing `requirements-dev.txt` declares
  `jsonschema`, `PyYAML`, and `pip-audit==2.10.1`; no dependency file needed
  to be replaced or broadened.
- [x] 1.2 Bind `security` to `pip-audit -r requirements-dev.txt` in this
  repository's `.ai-gate/gate.yaml`, so it audits declared dependencies.
- [x] 1.3 Run `python3 scripts/bind_gate_commands.py --root
  /home/paul/code --repo workspace-governance --verbose`; the generator
  classifies the owner-scoped command as `CONFLICT` and leaves it in place;
  there are 0 unresolved checks.
- [x] 1.4 Run the real `driftwatchdog gate`; `security` passes with
  `No known vulnerabilities found`, and the complete Gate passes.

## 4. Definition of done

- [x] 2.1 `gate-debt.json` has no unresolved entry for `workspace-governance`;
  the legacy debt driver reports 0 entries.
- [x] 2.2 The Gate plan lists `security` as a required command-bound check;
  the real Gate run reports no `UNBOUND` check.
- [x] 2.3 Every selected concern has a written command in
  `.ai-gate/gate.yaml`; no exemption or disabled check was used.
- [x] 2.4 The real Gate ran and passed all six required checks; exact verdict
  and the security result are recorded in `HANDOFF.md`.

## 5. Completion

- [x] The owner's decision is recorded by the scoped development dependency
  and command. The gate passes; unrelated host Python advisories are not
  charged to this repository.
- [x] Archive with the owner's authorization recorded above.
