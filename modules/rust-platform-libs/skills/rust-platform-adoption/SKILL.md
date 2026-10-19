---
name: rust-platform-adoption
description: Use when a Rust product considers shared platform crates, consumes platform-contracts schemas, or evaluates the Rust platform consumer baseline and crate publication status.
---

# Rust platform adoption

The Rust platform owns small, opt-in crates and its consumer conformance
recipe. Products own their data, users, migrations, and runtime. Read
`README.md`, `HANDOFF.md`, `docs/consumer-baseline.md`,
`docs/publication.md`, and the relevant crate manifests before choosing an
adoption path.

## Decision path

1. Name the capability and compare it with the crate matrix and actual
   consumer evidence. Keep provider, HTTP, SQLx, OIDC, and other optional
   adapters out of foundation crates unless a specific contract justifies
   them. Inspect feature defaults and transitive dependencies.
2. Check publication state. The publication pipeline and a released crate are
   different: the README records the first crates.io release as blocked on
   owner approval. Do not claim a registry version is available from a
   completed pipeline or archived spec.
3. For an external consumer, prefer a released pinned crate when verified.
   Before release, inspect the owner's supported git-pin or path route and
   Workspace Governance's prohibition on sibling source coupling. Do not
   introduce a `../` dependency into a governed product merely because an
   in-workspace fixture uses one.
4. Run the consumer baseline, native build/tests, and platform-contracts
   parity check against a pinned schema revision. A crate test or fixture
   alone does not establish product runtime behavior.
5. For changes here, follow `AGENTS.md` and the selected OpenSpec package;
   keep publication, consumer adoption, and runtime proof as separate stages.

If the compatible artifact is unavailable under the consumer's governance
rules, record the blocked adoption and its exact prerequisite instead of
copying crate source or silently coupling checkouts.
