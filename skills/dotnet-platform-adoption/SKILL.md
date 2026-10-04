---
name: dotnet-platform-adoption
description: Use when a .NET product or Forge scaffold adopts shared platform packages, a local feed or kit, an identity starter, or a cross-project capability owned by dotnet-platform-libs.
---

# .NET platform adoption

The platform owns reusable opt-in libraries; each product owns its source,
users, migrations, configuration, and deployment. Read `README.md` package
matrix, `docs/packages.md`, the consumer guide, `HANDOFF.md`, and the relevant
OpenSpec package before selecting a dependency.

## Adoption check

1. Name the repeated capability and compare the package contract with the
   product's actual need. Inspect transitive dependencies and consumer evidence;
   do not impose an ORM, provider, or version floor on consumers that do not
   need it.
2. Choose a published, pinned package or a versioned source/template when a
   compiled package would force incompatible dependency versions. A Forge kit
   is a deterministic scaffold input; verify the generated project restores
   and builds from its committed, project-relative feed without the sibling
   checkout or Forge runtime.
3. Record the adoption in the consumer's own manifest and composition root.
   Check Workspace Governance's capability registry and adoption grant for
   measured consumption; a declaration without a real manifest edge is not
   adoption.
4. Run the consumer's native restore/build/tests and relevant contract parity
   checks. Verify runtime behavior separately where the capability needs it.
   `PlatformConsumerOptOut` is an explicit consumer choice, not evidence that a
   capability was adopted.
5. Keep end-user identity separate from operator SSO. An identity starter is
   opt-in and site-local unless a versioned contract and product design say
   otherwise; it does not make this repository a central user database.

Follow `AGENTS.md` for changes to the platform itself. Do not add a
`ProjectReference`, absolute workspace path, or `../` sibling dependency to a
consumer. Do not claim publication or support from an OpenSpec plan alone.
