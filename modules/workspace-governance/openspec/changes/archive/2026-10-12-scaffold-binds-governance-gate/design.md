# Design: Bind governance checks when scaffolding a project

## Runtime and boundaries

- Runtime: Python 3.
- Profile vocabulary: `scripts/bind_gate_commands.py::PROFILE_DEFAULTS`, the binder's mirror of Driftwatchdog built-in profiles. Add the ten workspace profiles supported by the gate.
- Scaffold entry point: `scripts/init_project.py::main` validates the profile before `target.mkdir`, writes only absent template files, then invokes the binder for the new repository.
- Generated manifest: canonical `templates/project/.ai-gate/gate.yaml`; binder remains the sole owner of derived command values.

## Flow and invariants

1. Resolve project ID, validate profile, and load templates before creating files.
2. Write the declaration and missing templates using existing preservation rules.
3. Call the binder's scoped apply API for only the target repository. Explicit target selection may inspect a new directory that has not been initialized as a Git repository. The clean-tree guard is bypassed only when both `.project.json` and `.ai-gate/gate.yaml` were created by this initializer in this invocation.
4. Return a nonzero scaffold result if binder invocation fails; do not report a successful initialized project.

The profile registry is exact and case-sensitive. Unknown names fail with the supported choices and leave a nonexistent target nonexistent. Binder runs only after the manifest exists. It may bind only what current repository evidence supports; empty scaffolds keep unresolved commands absent and remain blocked by the gate. A completed binder operation can report unresolved checks; these are printed as debt, while a refused or dirty-manifest result fails initialization. Existing files are preserved, and no command is fabricated.

## Failure behavior

- Unknown profile: exit 2 before writes.
- Binder nonzero or launch error: report the repository and binder failure, return nonzero, and preserve generated files for a safe retry.
- Binder clean-tree or manifest refusal: surfaced as failure; no fallback binding is written by the scaffold.

## Verification oracle

Temporary workspace integration tests invoke the real initializer and binder script. They verify valid-profile acceptance, typo refusal before target creation, binder invocation after manifest creation with a repository-specific selector, and successful generated command binding where fixture evidence supports it. A fixture without derivable commands confirms the scaffold does not invent one.
