# Bootstrap fixture consumers

Each subdirectory is a standalone .NET project that imports
`build/Platform.Consumer.props` from the platform repo and exercises one
behavioural branch of the consumer bootstrap contract.

| Fixture | Scenario |
|---|---|
| `SourceMode/` | `PlatformAsSource=true` and the checkout exists → `ProjectReference` to `Platform.Core`. |
| `PackageMode/` | `PlatformAsSource` unset and `PlatformPackageVersion=0.1.0` → `PackageReference` to `Platform.Core`; the local feed is wired in `nuget.config`. |
| `OptOut/` | Bootstrap opted in, then `PlatformConsumerOptOut=true` → no reference, no defaults, no diagnostic. |
| `ConsumerDefaults/` | Bootstrap opted in, no overrides → defaults applied. |
| `ConsumerOverrides/` | Bootstrap opted in, every defaulted property overridden → consumer values win. |
| `NotOptedIn/` | `PlatformConsumerBootstrap` not set → bootstrap is a no-op. |
| `UnsupportedTarget/` | `TargetFramework=net8.0` → named diagnostic. |
| `MissingCheckout/` | `PlatformAsSource=true` with a non-existent `PlatformConsumerSourceRoot` → named diagnostic. |
| `MissingVersion/` | `PlatformAsSource` unset and `PlatformPackageVersion` unset → named diagnostic. |

The fixture projects intentionally live inside the platform source tree so
they can import `build/Platform.Consumer.props` directly. They are not
referenced by the platform solution and are not built by the regular
solution build; they are evaluated and exercised by
`tests/Platform.ConsumerConformance/Tests/BootstrapConformanceTests.cs`.
