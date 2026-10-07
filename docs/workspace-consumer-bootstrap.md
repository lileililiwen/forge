# Legacy workspace consumer bootstrap

`build/Platform.Consumer.props` is retained only so existing consumers do not
break before migration. Do not import it in a new project. Its
`PlatformAsSource` branch depends on a sibling checkout and is therefore not a
portable Docker, GitHub Actions or clean-machine contract.

The supported replacement is the package-only contract in
[`platform-consumer-adoption.md`](platform-consumer-adoption.md):

- keep package IDs as `Platform.*`;
- map `Platform.*` to the anonymous static platform feed;
- map all other packages to NuGet.org;
- pin one exact `PlatformPackageVersion`;
- configure any unpublished local feed in the developer's user-level NuGet
  configuration, never in the repository.

## Migrate a legacy repository

Check without writing:

```bash
python3 /path/to/dotnet-platform-libs/scripts/migrate-consumer-nuget.py /path/to/consumer
```

Apply after reviewing the detected version:

```bash
python3 /path/to/dotnet-platform-libs/scripts/migrate-consumer-nuget.py /path/to/consumer --apply
```

The migration is atomic with respect to version ambiguity: multiple, floating
or unresolved `Platform.*` versions stop before any write. Apply mode:

1. writes the canonical `NuGet.config` source mapping;
2. creates or normalizes one defaulted `PlatformPackageVersion` property;
3. changes literal `Platform.*` versions to that property;
4. removes the recognized `PlatformAsSource` project-reference branch and
   makes the existing package-reference branch unconditional;
5. removes `PlatformAsSource` and `PlatformConsumerSourceRoot` from repository
   `Directory.*.props` files.

Mixed custom content inside a legacy source-only item group is refused rather
than guessed. Move that content to its own item group, then rerun migration.

## Compatibility boundary

Until a consumer is migrated, the legacy properties and bootstrap retain their
old behavior. That compatibility is not an endorsement for new use. Migration
must be performed and verified in each consumer repository; this platform
repository does not rewrite consumers automatically.

After migration, verify restore, build and tests first with any local source
disabled and an isolated NuGet cache. That is the portability proof. A local
prerelease build is only a development overlay and must use a unique version.
