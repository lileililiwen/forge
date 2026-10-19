## Context

The platform already has a package manifest and consumer conformance checks, but those are repository-internal. Consumers need a safe entry point to inspect a sibling project before adopting packages. The target is a diagnostic tool, not a project generator or upgrade bot.

## Goals / Non-Goals

**Goals:** explicit target directory, deterministic checks, dry-run package suggestions, JSON output, and exit codes suitable for CI.

**Non-Goals:** modifying files, network-dependent remediation, understanding product business rules, or enforcing a single application architecture.

## Decisions

- Start with a .NET global tool or standalone script backed by a provider-neutral diagnostic core; keep CLI presentation separate from checks.
- Require `--project-dir` and resolve all files relative to it; never infer a sibling target from the current working directory.
- Checks include SDK/global.json, central package management, exact Platform package versions, production references to test packages, solution/project discoverability, nullable/warnings-as-errors signals, and optional Docker/feed availability.
- Classify results as pass, warning, failure, or environment-blocked. Environment checks never become source failures without explicit opt-in.
- Make package alignment preview-only in the first version; output proposed edits without writing them.
- Emit stable JSON schema and human-readable output with secret-free diagnostics.

Alternatives considered: a direct fork of the starter CLI would assume its template and infrastructure; an automatic converter would be unsafe across heterogeneous applications; repository-only scripts are hard to consume from sibling projects.

## Risks / Trade-offs

- [Risk] Heuristics produce false positives → report evidence and classification, and keep checks narrowly defined.
- [Risk] Feed/Docker checks are unavailable locally → classify environment blockers separately and preserve rerun instructions.
- [Risk] Tool version drifts from package manifest → test the tool against pinned fixtures and publish compatibility metadata.

## Migration Plan

Consumers run the tool read-only before adoption, then add package references manually or through a reviewed patch. Rollback is removal of package references; the tool never edits them.

## Open Questions

None; automatic edits are explicitly deferred.
