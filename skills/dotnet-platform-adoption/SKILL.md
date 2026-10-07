---
name: dotnet-platform-adoption
description: Use when a .NET product or scaffold adopts Platform packages, configures a local package source, migrates a legacy consumer, or verifies portable restore behavior.
---

# .NET platform adoption

The platform owns reusable packages and its anonymous static NuGet v3 feed.
Each product owns its package references, composition and deployment. Read the
package matrix, consumer guide, `HANDOFF.md` and relevant OpenSpec package.

## Required path

1. Use exact `Platform.*` package versions through the consumer's single
   defaulted `PlatformPackageVersion` property.
2. Use the canonical repository `NuGet.config`: `Platform.*` maps to the public
   platform feed and everything else maps to nuget.org.
3. For a legacy consumer, run
   `scripts/migrate-consumer-nuget.py <consumer>` first, then rerun with
   `--apply` only when its version report is unambiguous.
4. For unpublished development packages, pack a unique prerelease version and
   configure the current user's source with
   `scripts/configure-local-source.py --feed <directory>`. Pass the version as
   `-p:PlatformPackageVersion=<version>`; do not edit tracked consumer files.
5. Before claiming portability, restore/build with an isolated global package
   folder and without the user-level local source. Verify Docker or CI
   separately when the consumer ships those paths.

Never commit a local feed path, credential, GitHub Packages source, sibling
`ProjectReference`, absolute checkout path or `../` dependency. Never reuse an
ID/version for different package bytes. A local build is development evidence,
not proof that GitHub Actions, Docker or another computer can restore.
