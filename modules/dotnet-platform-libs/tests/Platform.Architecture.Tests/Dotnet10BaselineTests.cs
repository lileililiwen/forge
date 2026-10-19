using System.Text.Json;
using System.Xml.Linq;

namespace Platform.Architecture.Tests;

/// <summary>
/// Verifies the .NET 10 platform baseline: SDK pin, target frameworks,
/// Microsoft package generation, derived artifacts, and documentation consistency.
/// Fails when source-controlled metadata retains SDK 8, net8.0, or Microsoft 8.x.
/// </summary>
public sealed class Dotnet10BaselineTests
{
    private static readonly string RepositoryRoot = LocateRepositoryRoot();

    private static readonly string[] MicrosoftPackagesRequiring10x =
    {
        "Microsoft.AspNetCore.Mvc.Testing",
        "Microsoft.Extensions.DependencyInjection.Abstractions",
        "Microsoft.Extensions.Logging.Abstractions",
        "Microsoft.Extensions.Options",
        "Microsoft.Extensions.Http",
        "Microsoft.Extensions.Hosting.Abstractions",
        "Microsoft.Extensions.Caching.Memory",
        "Microsoft.EntityFrameworkCore",
        "Microsoft.EntityFrameworkCore.Relational",
        "Microsoft.EntityFrameworkCore.InMemory",
        "Microsoft.EntityFrameworkCore.Sqlite",
        "Npgsql.EntityFrameworkCore.PostgreSQL",
        "Microsoft.Extensions.Diagnostics.HealthChecks.Abstractions",
        "Microsoft.Extensions.DependencyInjection",
        "Microsoft.Extensions.Logging",
    };

    [Fact]
    public void Global_json_pins_dotnet10_sdk()
    {
        var path = Path.Combine(RepositoryRoot, "global.json");
        Assert.True(File.Exists(path), "global.json must exist at " + path);
        using var document = JsonDocument.Parse(File.ReadAllText(path));
        var sdk = document.RootElement.GetProperty("sdk");
        Assert.Equal("10.0.400", sdk.GetProperty("version").GetString());
        Assert.Equal("latestPatch", sdk.GetProperty("rollForward").GetString());
        Assert.False(sdk.GetProperty("allowPrerelease").GetBoolean());
    }

    [Fact]
    public void All_platform_projects_target_net10()
    {
        var failures = new List<string>();
        foreach (var project in EnumeratePlatformProjects())
        {
            var document = XDocument.Load(project);
            var frameworks = document
                .Descendants()
                .Where(e => string.Equals(e.Name.LocalName, "TargetFramework", StringComparison.Ordinal)
                    || string.Equals(e.Name.LocalName, "TargetFrameworks", StringComparison.Ordinal))
                .Select(e => e.Value.Trim())
                .Where(v => v.Length > 0)
                .ToArray();
            foreach (var tfm in frameworks)
            {
                if (!string.Equals(tfm, "net10.0", StringComparison.Ordinal))
                {
                    failures.Add($"{Relative(project)} targets '{tfm}'");
                }
            }

            if (frameworks.Length == 0)
            {
                failures.Add($"{Relative(project)} declares no TargetFramework");
            }
        }

        Assert.True(
            failures.Count == 0,
            "Every platform project must target net10.0. Violations: " + string.Join("; ", failures));
    }

    [Fact]
    public void No_source_project_retains_net8_target()
    {
        var violations = EnumeratePlatformProjects()
            .Where(project => File.ReadAllText(project).Contains("net8.0", StringComparison.Ordinal))
            .Select(Relative)
            .ToArray();
        Assert.Empty(violations);
    }

    [Fact]
    public void Microsoft_framework_packages_resolve_to_10x()
    {
        var path = Path.Combine(RepositoryRoot, "Directory.Packages.props");
        var document = XDocument.Load(path);
        var pins = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "PackageVersion", StringComparison.Ordinal))
            .ToDictionary(
                e => e.Attribute("Include")?.Value ?? string.Empty,
                e => e.Attribute("Version")?.Value ?? string.Empty,
                StringComparer.OrdinalIgnoreCase);
        foreach (var id in MicrosoftPackagesRequiring10x)
        {
            Assert.True(pins.TryGetValue(id, out var version), $"Directory.Packages.props must pin {id}");
            Assert.StartsWith("10.", version, StringComparison.Ordinal);
        }
    }

    [Fact]
    public void Consumer_bootstrap_applies_to_net10()
    {
        var path = Path.Combine(RepositoryRoot, "build", "Platform.Consumer.props");
        Assert.True(File.Exists(path), "build/Platform.Consumer.props must exist at " + path);
        var content = File.ReadAllText(path);
        Assert.Contains("net10.0", content, StringComparison.Ordinal);
        Assert.DoesNotContain("net8.0", content, StringComparison.Ordinal);
    }

    [Fact]
    public void Sample_matrix_targets_net10()
    {
        var path = Path.Combine(RepositoryRoot, "samples", "matrix.json");
        using var document = JsonDocument.Parse(File.ReadAllText(path));
        foreach (var sample in document.RootElement.GetProperty("samples").EnumerateArray())
        {
            var name = sample.GetProperty("name").GetString();
            Assert.Equal("net10.0", sample.GetProperty("targetFramework").GetString());
            Assert.NotEqual("net8.0", sample.GetProperty("targetFramework").GetString());
        }
    }

    [Fact]
    public void Template_content_targets_net10_with_pinned_packages()
    {
        var contentRoot = Path.Combine(RepositoryRoot, "templates", "platform-application-starter");
        foreach (var project in Directory.EnumerateFiles(contentRoot, "*.csproj", SearchOption.AllDirectories))
        {
            var text = File.ReadAllText(project);
            Assert.DoesNotContain("net8.0", text, StringComparison.Ordinal);
            Assert.Contains("net10.0", text, StringComparison.Ordinal);
        }
    }

    [Fact]
    public void Package_manifest_reports_net10()
    {
        var path = Path.Combine(RepositoryRoot, "eng", "package-manifest.json");
        using var document = JsonDocument.Parse(File.ReadAllText(path));
        foreach (var package in document.RootElement.GetProperty("packages").EnumerateArray())
        {
            var packagePath = package.GetProperty("path").GetString() ?? string.Empty;
            foreach (var tfm in package.GetProperty("targetFrameworks").EnumerateArray())
            {
                Assert.Equal("net10.0", tfm.GetString());
            }
        }
    }

    [Fact]
    public void Current_docs_describe_dotnet10_baseline()
    {
        var readme = File.ReadAllText(Path.Combine(RepositoryRoot, "README.md"));
        Assert.DoesNotContain("net8.0", readme, StringComparison.Ordinal);
        Assert.Contains("net10.0", readme, StringComparison.Ordinal);
    }

    private static IEnumerable<string> EnumeratePlatformProjects()
    {
        foreach (var root in new[] { "src", "tests", "samples", "tools", "templates" })
        {
            var directory = Path.Combine(RepositoryRoot, root);
            if (!Directory.Exists(directory))
            {
                continue;
            }

            foreach (var project in Directory.EnumerateFiles(directory, "*.csproj", SearchOption.AllDirectories))
            {
                var relative = Path.GetRelativePath(RepositoryRoot, project);
                var segments = relative.Split(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
                if (segments.Any(s => s.Equals("bin", StringComparison.OrdinalIgnoreCase) || s.Equals("obj", StringComparison.OrdinalIgnoreCase)))
                {
                    continue;
                }

                // Test fixtures under tests/**/Fixtures/** are consumer-side
                // scenarios, not platform projects. They intentionally cover
                // unsupported targets, opt-outs, and other negative cases,
                // so they must be excluded from baseline target-framework
                // and content invariants.
                if (segments.Any(s => s.Equals("Fixtures", StringComparison.OrdinalIgnoreCase)))
                {
                    continue;
                }

                yield return project;
            }
        }
    }

    private static string Relative(string fullPath) => Path.GetRelativePath(RepositoryRoot, fullPath);

    private static string LocateRepositoryRoot()
    {
        var current = new DirectoryInfo(AppContext.BaseDirectory);
        while (current is not null)
        {
            if (File.Exists(Path.Combine(current.FullName, "Directory.Build.props"))
                && File.Exists(Path.Combine(current.FullName, "Platform.sln")))
            {
                return current.FullName;
            }

            current = current.Parent;
        }

        throw new InvalidOperationException("Unable to locate the repository root from " + AppContext.BaseDirectory);
    }
}
