# Shared library adoption (.NET / Rust)

Adoption lives in the owning repositories; this reference only routes there.

## Read first

- `docs/package-backlog.md` — the package index. It names the owning
  repository and the OpenSpec change for each adoption package, including the
  dotnet-platform-libs consumer bootstrap, the rust-platform-libs consumer
  baseline, and the platform-contracts consumer parity follow-up.
- `docs/capabilities.md` — the canonical `capabilities` vocabulary, the
  `consumes[]` edge, the posture ramp, the decline vocabulary, and the
  `CAPABILITY_ADOPTION_*` findings. `capabilities.json` binds each capability
  id to one provider.
- `docs/adoption-grant.md` — the bounded permission to repoint a consumer onto
  a registered provider, and the `adoption/adoptions.json` ledger.
- `scripts/init_project.py` — adopts a project with `--profile` and `--kind`
  without copying shared gate runtimes, installers, caches, or credentials.

## Rules of the road

- One repository per change; never implement a cross-repo package from
  another repository.
- Consume the governance vocabulary from `vocabulary.json`; never re-declare
  it in an adopting project.
- Shared runtimes stay centralized; product repositories keep only their
  declarations and exceptions.
- Adoption is measured, not asserted. Record the edge in the consumer's
  `capabilities[].consumes[]` and pin the coordinate in the consumer's own
  manifest; a record without a resolvable manifest edge is a finding.
- Consuming a published, pinned artifact is a consumer-side change. Editing a
  sibling's *source* still needs scope authorization, and a source-level
  coupling (`ProjectReference`, relative path, or `../`) is forbidden outright.
