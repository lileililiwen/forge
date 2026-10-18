# Design: Forge Gate conformance for Workspace Governance

## Implementation boundary

Repository: `/home/paul/code/forge`. Add `modules/workspace-governance/gate.toml` with an explicit source-size include for its maintained Python scripts; do not change Forge-wide scanner defaults or scanner code. Split only the four oversized maintained Python scripts in `modules/workspace-governance/scripts/`: `workspace_check.py`, `bind_gate_commands.py`, `product_code_quality.py`, and `ui_conformance.py`. Keep each existing script path as an executable/import-compatible facade. Include the product-quality helper modules in the existing Forge archive and installer because installed `forge concern check` runs the checker as a standalone script. Document manifest precedence and synchronization of the selected commands. Add focused tests for the project-owned Gate manifest, Python syntax, packaged assets and installed-resource execution. Do not modify external sibling repositories, Gate evidence schemas, module `.project.json`, CI provider workflows, or the source-size threshold.

## Language and runtime

- Governance module: Python 3.11, standard library plus repository-declared development dependencies. Oracles: `python3 -m unittest discover -s tests -v` from the module root and `python3 scripts/workspace_check.py --self-check`.
- Distribution: `scripts/package.sh` builds the release archive; `scripts/install.sh` validates and installs its governance resources. The split helper modules must travel beside the quality-checker facade.
- Gate: use the current Forge binary, rehearse `./target/debug/forge gate --dry-run modules/workspace-governance`, then run full Gate with a bounded timeout. The native module manifest takes precedence over `.ai-gate/gate.yaml`, so it must preserve all six enabled checks and their existing commands.

## Ownership and shared code

`crates/driftwatchdog/src/gate/source_size.rs` remains unchanged as the single scanner. The module's `gate.toml` sets `[source_size].include = ["scripts/**/*.py"]`, using the existing per-project include mechanism. This is deliberately narrower than a global conventional-root change: only Workspace Governance opts its scripts into the baseline, and other Forge consumers retain their current candidate sets.

Python code remains owned by `modules/workspace-governance`. Extract cohesive helpers into adjacent modules and re-export them from the original script files so existing `python3 scripts/<name>.py ...`, `import scripts.<name>`, and `from scripts.<name> import symbol` callers retain behavior. Keep constants, finding codes, function signatures, argument parsing, output ordering, and exit codes unchanged. New module names must describe their responsibility and stay below the same 1,000-line limit. Tests remain in the existing test tree; test files are excluded from product source-size scanning.

## User experience and interface

No new screen, route, or CLI syntax. Maintainers continue to run `forge gate <module path>`. Gate `FAIL` must identify each oversized source path and measured line count; `PASS` reports nonzero evaluated candidate evidence for the module. Plain text and JSON remain the existing supported output shapes. Output must not require color, a terminal width, mouse, or visual-only cues. No browser/device verification applies because the changed user workflow is the existing CLI.

## Behavioral model and invariants

1. Candidate selection uses the existing repository-owned file list and exclusions.
2. This module explicitly includes `scripts/**/*.py`; the Forge-wide conventional-root defaults remain unchanged.
3. Explicit include/exclude precedence, symlink/binary handling, raw newline counting, 1,000-line maximum, result bounding, and review-required boundary behavior are unchanged.
4. Before refactor, the four scripts exceed 1,000 raw newline characters. After refactor, each is at or below 1,000, without excluding any product source from the scan.
5. Existing Python public imports and script invocations return equivalent data and process outcomes.

## Contract and compatibility

There is no wire-contract or persistence migration. The existing stable Gate concern ID `source-file-size`, evidence contract, `GateResult`, finding rule, and CLI command remain unchanged. Existing Python file paths are compatibility façades; module extraction is internal. Do not delete or rename the script entry points.

## Failure and boundary policy

- Missing script root: no candidates from that root; normal repository discovery applies.
- Empty source root: measured zero candidates remains a pass, as before.
- Git boundary or source read uncertainty: existing `REVIEW_REQUIRED` path remains authoritative.
- Any included file over 1,000 lines: existing bounded `FAIL` finding, with no baseline exemption.
- Python import or runtime errors after extraction: module unittest suite/self-check fails; no partial results or silent fallback is introduced.
- Missing split product-quality helpers in a release archive: `scripts/install.sh` refuses the archive before writing to the installation prefix.

## Verification oracle

- A focused manifest contract test pins the module's Python include, 1,000-line default, no exclusions, and all six existing check commands.
- Existing scanner tests pass, including explicit-include, exclusion, boundary and line-count behavior.
- The package and installer source contracts list both product-quality helpers, and a Forge binary executing from an installed prefix passes the product-code concern check without the source checkout.
- Run Forge Gate dry-run and full Gate on the module. The full result must show `source-file-size: PASS`, a nonzero evaluated source-candidate count, and no file-size findings; all other selected module checks must pass or be documented as a pre-existing unrelated failure with exact remediation.
- Run the complete Python unittest suite and governance self-check. CLI entry points and representative output remain byte/shape compatible where existing tests assert them.
- Run `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive`, and `git diff --check` before archive.
- No production Rust scanner/runtime source changes; the installed-resource contract test is updated to assemble the actual split Python resource set and remains formatted.

## Decision ledger

- Preserve the established 1,000-line policy; the observed gap is module-specific coverage and file structure, not policy threshold.
- Keep script paths as compatibility facades rather than update every in-tree or external import site.
- Treat installed Forge governance resources as a caller: every helper imported by the product-quality facade must ship beside it in package archives and installations.
- No external sibling edits are in scope; their governance-consumer migrations remain separate owner changes.
- No unresolved implementation decision remains.
