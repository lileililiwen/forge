## Context

Canonical specs were promoted mechanically and retain a `TBD` purpose. The
repository status changed from planning-only to implemented, while generated
OpenSpec context and ADR history were not fully refreshed. The documentation
must preserve the distinction between code-level verification, fixture
evidence, unavailable providers and release acceptance.

## Goals / Non-Goals

**Goals:** remove placeholders, make status and dependency pointers truthful,
add cheap automated checks, and retain historical evidence wording.

**Non-Goals:** changing requirement meaning, rewriting archived history, or
marking any pending audit package complete.

## Decisions

Each canonical spec receives a one-sentence purpose derived from its capability
requirements. Repository status uses explicit labels for implemented locally,
contract-verified, native-verified, provider-verified and release-ready. The
OpenSpec context describes the current implementation baseline; the ADR keeps
its historical planning context but labels the decision as superseded-by-
implementation where needed. A script checks canonical specs and active
changes for `TBD`, stale `current_spec`, broken local links and contradictory
status markers.

## Requirement and scenario coverage

- **R1 — Canonical purpose and status truthfulness:** every promoted spec and
  status document is readable and evidence-qualified.
- **R2 — Governance traceability checks:** stale placeholders and pointers fail
  a deterministic repository check.

## Failure and compatibility

Historical archived change artifacts remain unchanged except where the local
workflow explicitly permits promotion metadata updates. The checker reports
actionable file/line failures and does not infer runtime completion.

## Migration Plan

Documentation-only migration. Update canonical specs and governance docs in one
related change, then validate all links, names and strict OpenSpec structure.

## Verification Strategy

Run the new traceability checker, strict OpenSpec validation, link/whitespace
checks, and review README/ROADMAP/HANDOFF against current source and evidence.

## Open Questions

The exact status vocabulary can be finalized during baseline mapping, but it
must retain separate native, provider and release evidence states.
