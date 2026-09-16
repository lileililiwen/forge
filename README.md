# Forge

Forge is a language-agnostic developer control plane and software assembly platform. It coordinates heterogeneous projects, resolves reusable software parts, generates ordinary source deterministically and uses AI for interpretation and unresolved project-specific work.

## Status

Planning only. The repository began with [Requirements & Product Design v0.3](requirement.md); it has no application source, package manifest, build pipeline or runtime tests. The requirements document version is not a delivered Forge release. All 24 OpenSpec changes are proposed and unchecked. Generated project behavior, integrations and stack support have not been implemented or runtime-verified.

## MVP and delivery

v0.1 is deliberately limited to `forge.yaml`, project/profile registries, and `forge import`, `forge list`, `forge inspect`, `forge new`, `forge doctor`. Its profiles are `aspnet-web`, `rust-web`, `nextjs-web`, `flutter-app`, and `python-service`. These command names describe planned interfaces, not commands available from this checkout.

v0.2 adds features and upgrades; v0.3 integrates DriftWatch, specs and existing agent infrastructure; v0.4 exposes mature MCP operations; v0.5 adds repository distribution, translations, releases and deployment. Advanced components, UI patterns, AI planning, identity, analytics, API and portal have separate later changes.

See the [dependency-ordered roadmap](ROADMAP.md), [complete section coverage](docs/requirements-coverage.md), [architecture](docs/architecture.md), and [current handoff](HANDOFF.md).

## Documentation quickstart

With Node.js and the OpenSpec CLI available (validated here with OpenSpec 1.6.0):

```sh
node scripts/check-openspec-change-names.mjs
openspec list
openspec status --change core-manifest-registry
openspec validate --all --strict --no-interactive
```

Foundation toolchain (established by `core-manifest-registry`, see
[ADR 0001](docs/adr/0001-foundation-toolchain.md)): Rust stable with
`rusqlite` bundled (no system SQLite required).

```sh
cargo fmt --check
cargo build        # produces ./target/debug/forge
cargo test
cargo clippy --all-targets -- -D warnings
```

There is no Forge installation packaging yet.

## Product boundaries

- Deterministic templates, packages, codemods and migrations precede AI generation.
- Generated projects must build and operate through their native tools without Forge.
- The product model stays independent of language, framework, AI vendor, IDE and host.
- Forge coordinates DriftWatch, the existing PTY agent manager, content and analytics tools; it does not replace them.
- Project maturity is evidence-based and optional to advance; L0 experimentation is valid.
- Forge is not a programming language, low-code runtime, IDE, CMS, Git host, AI model, universal runtime, Kubernetes replacement or generic CI/CD replacement.

The full authoritative brief remains in [requirement.md](requirement.md). [AGENTS.md](AGENTS.md) defines the project contribution entry point.
