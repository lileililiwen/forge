using System.Diagnostics;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Xml.Linq;

namespace Platform.ConsumerConformance;

/// <summary>
/// Conformance tests for the platform consumer bootstrap
/// (<c>build/Platform.Consumer.props</c>). Each scenario is driven by a
/// standalone fixture project under <c>Fixtures/Bootstrap/</c> that
/// imports the bootstrap and configures the relevant properties. The
/// tests evaluate the project graph with <c>dotnet msbuild</c> so the
/// assertions match the real MSBuild semantics that a consumer will see.
/// </summary>
public sealed class BootstrapConformanceTests
{
    [Fact]
    public void Bootstrap_file_exposes_the_documented_property_names()
    {
        var document = XDocument.Load(LocateBootstrapFile());
        var references = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var group in document.Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "PropertyGroup", StringComparison.OrdinalIgnoreCase)))
        {
            var groupCondition = group.Attribute("Condition")?.Value;
            if (!string.IsNullOrWhiteSpace(groupCondition))
            {
                foreach (var name in ExtractPropertyNames(groupCondition))
                {
                    references.Add(name);
                }
            }
            foreach (var element in group.Elements())
            {
                var elementCondition = element.Attribute("Condition")?.Value;
                if (!string.IsNullOrWhiteSpace(elementCondition))
                {
                    foreach (var name in ExtractPropertyNames(elementCondition))
                    {
                        references.Add(name);
                    }
                }
            }
        }

        Assert.Contains("PlatformConsumerBootstrap", references);
        Assert.Contains("PlatformConsumerOptOut", references);
        Assert.Contains("PlatformAsSource", references);
        Assert.Contains("PlatformPackageVersion", references);
        Assert.Contains("PlatformConsumerSourceRoot", references);
    }

    [Fact]
    public void Source_mode_injects_a_ProjectReference_to_Platform_Core()
    {
        var project = LocateFixture("SourceMode", "SourceModeConsumer.csproj");
        var projectReferences = QueryProjectReferences(project);
        var target = Assert.Single(projectReferences);
        Assert.EndsWith("Platform.Core.csproj", target, StringComparison.OrdinalIgnoreCase);
        Assert.Contains("dotnet-platform-libs", target.Replace('\\', '/'), StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Source_mode_applies_the_consumer_defaults()
    {
        var project = LocateFixture("SourceMode", "SourceModeConsumer.csproj");
        var properties = QueryProperties(project, "Nullable", "LangVersion", "AnalysisLevel", "TreatWarningsAsErrors", "ManagePackageVersionsCentrally");
        Assert.Equal("enable", properties["Nullable"]);
        Assert.Equal("latest", properties["LangVersion"]);
        Assert.Equal("latest-recommended", properties["AnalysisLevel"]);
        Assert.Equal("true", properties["TreatWarningsAsErrors"]);
        Assert.Equal("true", properties["ManagePackageVersionsCentrally"]);
    }

    [Fact]
    public void Package_mode_injects_a_PackageReference_resolved_under_CPM()
    {
        var project = LocateFixture("PackageMode", "PackageModeConsumer.csproj");
        var packageReferences = QueryPackageReferences(project);
        var target = Assert.Single(packageReferences);
        Assert.Equal("Platform.Core", target.Include);
        // Central package management is the consumer default, so the
        // bootstrap injects the PackageReference without an inline
        // version. The version is declared in Directory.Packages.props
        // at $(PlatformPackageVersion) and NuGet resolves it centrally.
        Assert.Equal(string.Empty, target.Version);
        Assert.Equal("0.1.0", QueryCentralPackageVersion(project, "Platform.Core"));
    }

    [Fact]
    public void Package_mode_does_not_inject_a_ProjectReference()
    {
        var project = LocateFixture("PackageMode", "PackageModeConsumer.csproj");
        var projectReferences = QueryProjectReferences(project);
        Assert.Empty(projectReferences);
    }

    [Fact]
    public void Package_mode_without_CPM_sets_the_PackageReference_version_inline()
    {
        var project = LocateFixture("PackageModeNoCpm", "PackageModeNoCpmConsumer.csproj");
        var packageReferences = QueryPackageReferences(project);
        var target = Assert.Single(packageReferences);
        Assert.Equal("Platform.Core", target.Include);
        Assert.Equal("0.1.0", target.Version);
    }

    [Fact]
    public void Opt_out_consumer_has_no_ProjectReference()
    {
        var project = LocateFixture("OptOut", "OptOutConsumer.csproj");
        var projectReferences = QueryProjectReferences(project);
        Assert.Empty(projectReferences);
    }

    [Fact]
    public void Opt_out_consumer_has_no_PackageReference()
    {
        var project = LocateFixture("OptOut", "OptOutConsumer.csproj");
        var packageReferences = QueryPackageReferences(project);
        Assert.DoesNotContain(packageReferences, r => string.Equals(r.Include, "Platform.Core", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Opt_out_consumer_is_not_diagnosed()
    {
        var project = LocateFixture("OptOut", "OptOutConsumer.csproj");
        var properties = QueryProperties(project, "PlatformConsumerBootstrapFailed");
        Assert.NotEqual("true", properties["PlatformConsumerBootstrapFailed"]);
    }

    [Fact]
    public void Consumer_defaults_apply_when_not_overridden()
    {
        var project = LocateFixture("ConsumerDefaults", "ConsumerDefaultsConsumer.csproj");
        var properties = QueryProperties(project, "Nullable", "LangVersion", "AnalysisLevel", "TreatWarningsAsErrors", "ManagePackageVersionsCentrally");
        Assert.Equal("enable", properties["Nullable"]);
        Assert.Equal("latest", properties["LangVersion"]);
        Assert.Equal("latest-recommended", properties["AnalysisLevel"]);
        Assert.Equal("true", properties["TreatWarningsAsErrors"]);
        Assert.Equal("true", properties["ManagePackageVersionsCentrally"]);
    }

    [Fact]
    public void Consumer_overrides_win_over_the_bootstrap_defaults()
    {
        var project = LocateFixture("ConsumerOverrides", "ConsumerOverridesConsumer.csproj");
        var properties = QueryProperties(project, "Nullable", "LangVersion", "AnalysisLevel", "TreatWarningsAsErrors", "ManagePackageVersionsCentrally");
        Assert.Equal("disable", properties["Nullable"]);
        Assert.Equal("10", properties["LangVersion"]);
        Assert.Equal("none", properties["AnalysisLevel"]);
        Assert.Equal("false", properties["TreatWarningsAsErrors"]);
        Assert.Equal("false", properties["ManagePackageVersionsCentrally"]);
    }

    [Fact]
    public void Consumer_overrides_still_get_a_ProjectReference_in_source_mode()
    {
        var project = LocateFixture("ConsumerOverrides", "ConsumerOverridesConsumer.csproj");
        var projectReferences = QueryProjectReferences(project);
        var target = Assert.Single(projectReferences);
        Assert.EndsWith("Platform.Core.csproj", target, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Not_opted_in_consumer_has_no_ProjectReference()
    {
        var project = LocateFixture("NotOptedIn", "NotOptedInConsumer.csproj");
        var projectReferences = QueryProjectReferences(project);
        Assert.Empty(projectReferences);
    }

    [Fact]
    public void Not_opted_in_consumer_has_no_PackageReference()
    {
        var project = LocateFixture("NotOptedIn", "NotOptedInConsumer.csproj");
        var packageReferences = QueryPackageReferences(project);
        Assert.Empty(packageReferences);
    }

    [Fact]
    public void Not_opted_in_consumer_is_not_diagnosed()
    {
        var project = LocateFixture("NotOptedIn", "NotOptedInConsumer.csproj");
        var properties = QueryProperties(project, "PlatformConsumerBootstrapFailed", "PlatformConsumerBootstrap");
        Assert.NotEqual("true", properties["PlatformConsumerBootstrapFailed"]);
        Assert.NotEqual("true", properties["PlatformConsumerBootstrap"]);
    }

    [Fact]
    public void Unsupported_target_sets_a_named_diagnostic()
    {
        var project = LocateFixture("UnsupportedTarget", "UnsupportedTargetConsumer.csproj");
        var properties = QueryProperties(project, "PlatformConsumerBootstrapFailed", "PlatformConsumerUnsupportedTargetError");
        Assert.Equal("true", properties["PlatformConsumerBootstrapFailed"]);
        Assert.Contains("net8.0", properties["PlatformConsumerUnsupportedTargetError"], StringComparison.OrdinalIgnoreCase);
        Assert.Contains("net10.0", properties["PlatformConsumerUnsupportedTargetError"], StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Unsupported_target_does_not_inject_a_ProjectReference()
    {
        var project = LocateFixture("UnsupportedTarget", "UnsupportedTargetConsumer.csproj");
        var projectReferences = QueryProjectReferences(project);
        Assert.Empty(projectReferences);
    }

    [Fact]
    public void Missing_checkout_sets_a_named_diagnostic()
    {
        var project = LocateFixture("MissingCheckout", "MissingCheckoutConsumer.csproj");
        var properties = QueryProperties(project, "PlatformConsumerBootstrapFailed", "PlatformConsumerMissingCheckoutError");
        Assert.Equal("true", properties["PlatformConsumerBootstrapFailed"]);
        Assert.Contains("/this/path/does/not/exist/dotnet-platform-libs", properties["PlatformConsumerMissingCheckoutError"], StringComparison.OrdinalIgnoreCase);
        Assert.Contains("PlatformAsSource", properties["PlatformConsumerMissingCheckoutError"], StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Missing_version_sets_a_named_diagnostic()
    {
        var project = LocateFixture("MissingVersion", "MissingVersionConsumer.csproj");
        var properties = QueryProperties(project, "PlatformConsumerBootstrapFailed", "PlatformConsumerMissingVersionError");
        Assert.Equal("true", properties["PlatformConsumerBootstrapFailed"]);
        Assert.Contains("PlatformPackageVersion", properties["PlatformConsumerMissingVersionError"], StringComparison.OrdinalIgnoreCase);
    }

    private static IEnumerable<string> ExtractPropertyNames(string condition)
    {
        return Regex.Matches(condition, @"\$\(([A-Za-z][A-Za-z0-9_]*)\)")
            .Select(match => match.Groups[1].Value)
            .Distinct(StringComparer.OrdinalIgnoreCase);
    }

    private static string LocateBootstrapFile()
    {
        var directory = LocatePlatformRepositoryRoot();
        return Path.Combine(directory, "build", "Platform.Consumer.props");
    }

    private static string LocatePlatformRepositoryRoot()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null)
        {
            var candidate = Path.Combine(directory.FullName, "build", "Platform.Consumer.props");
            if (File.Exists(candidate))
            {
                return directory.FullName;
            }
            directory = directory.Parent;
        }
        throw new InvalidOperationException(
            "Unable to locate the platform repository root from " + AppContext.BaseDirectory);
    }

    private static string LocateFixture(string scenario, string projectName)
    {
        var root = LocatePlatformRepositoryRoot();
        return Path.Combine(root, "tests", "Platform.ConsumerConformance", "Fixtures", "Bootstrap", scenario, projectName);
    }

    private sealed record PackageReferenceRecord(string Include, string Version);

    private static IReadOnlyList<string> QueryProjectReferences(string project)
    {
        var args = BuildMsbuildArgs(project, "ProjectReference", null);
        var (exitCode, stdout, stderr) = RunMsbuild(args);
        Assert.True(exitCode == 0, $"dotnet msbuild failed.\nCommand: dotnet {string.Join(' ', args)}\nstderr:\n{stderr}\nstdout:\n{stdout}");
        using var document = JsonDocument.Parse(stdout);
        if (document.RootElement.ValueKind != JsonValueKind.Object)
        {
            return Array.Empty<string>();
        }
        if (!document.RootElement.TryGetProperty("Items", out var items) || items.ValueKind != JsonValueKind.Object)
        {
            return Array.Empty<string>();
        }
        if (!items.TryGetProperty("ProjectReference", out var entries) || entries.ValueKind != JsonValueKind.Array)
        {
            return Array.Empty<string>();
        }
        return entries.EnumerateArray()
            .Select(element => element.TryGetProperty("Identity", out var identity) && identity.ValueKind == JsonValueKind.String
                ? identity.GetString() ?? string.Empty
                : string.Empty)
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .ToArray();
    }

    private static IReadOnlyList<PackageReferenceRecord> QueryPackageReferences(string project)
    {
        var args = BuildMsbuildArgs(project, "PackageReference", null);
        var (exitCode, stdout, stderr) = RunMsbuild(args);
        Assert.True(exitCode == 0, $"dotnet msbuild failed.\nCommand: dotnet {string.Join(' ', args)}\nstderr:\n{stderr}\nstdout:\n{stdout}");
        using var document = JsonDocument.Parse(stdout);
        if (document.RootElement.ValueKind != JsonValueKind.Object)
        {
            return Array.Empty<PackageReferenceRecord>();
        }
        if (!document.RootElement.TryGetProperty("Items", out var items) || items.ValueKind != JsonValueKind.Object)
        {
            return Array.Empty<PackageReferenceRecord>();
        }
        if (!items.TryGetProperty("PackageReference", out var entries) || entries.ValueKind != JsonValueKind.Array)
        {
            return Array.Empty<PackageReferenceRecord>();
        }
        return entries.EnumerateArray()
            .Select(element => new PackageReferenceRecord(
                element.TryGetProperty("Identity", out var identity) && identity.ValueKind == JsonValueKind.String
                    ? identity.GetString() ?? string.Empty
                    : string.Empty,
                element.TryGetProperty("Version", out var version) && version.ValueKind == JsonValueKind.String
                    ? version.GetString() ?? string.Empty
                    : string.Empty))
            .Where(record => !string.IsNullOrWhiteSpace(record.Include))
            .ToArray();
    }

    private static string QueryCentralPackageVersion(string project, string packageId)
    {
        var args = BuildMsbuildArgs(project, "PackageVersion", null);
        var (exitCode, stdout, stderr) = RunMsbuild(args);
        Assert.True(exitCode == 0, $"dotnet msbuild failed.\nCommand: dotnet {string.Join(' ', args)}\nstderr:\n{stderr}\nstdout:\n{stdout}");
        using var document = JsonDocument.Parse(stdout);
        if (!document.RootElement.TryGetProperty("Items", out var items) || items.ValueKind != JsonValueKind.Object)
        {
            return string.Empty;
        }
        if (!items.TryGetProperty("PackageVersion", out var entries) || entries.ValueKind != JsonValueKind.Array)
        {
            return string.Empty;
        }
        foreach (var element in entries.EnumerateArray())
        {
            var identity = element.TryGetProperty("Identity", out var idElement) && idElement.ValueKind == JsonValueKind.String
                ? idElement.GetString() ?? string.Empty
                : string.Empty;
            if (!string.Equals(identity, packageId, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }
            if (element.TryGetProperty("Version", out var versionElement) && versionElement.ValueKind == JsonValueKind.String)
            {
                return versionElement.GetString() ?? string.Empty;
            }
        }
        return string.Empty;
    }

    private static IReadOnlyDictionary<string, string> QueryProperties(string project, params string[] names)
    {
        var args = BuildMsbuildArgs(project, null, names);
        var (exitCode, stdout, stderr) = RunMsbuild(args);
        Assert.True(exitCode == 0, $"dotnet msbuild failed.\nCommand: dotnet {string.Join(' ', args)}\nstderr:\n{stderr}\nstdout:\n{stdout}");
        var results = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        var trimmed = stdout?.Trim() ?? string.Empty;
        if (trimmed.StartsWith("{", StringComparison.Ordinal))
        {
            using var document = JsonDocument.Parse(trimmed);
            var propertiesElement = document.RootElement.ValueKind == JsonValueKind.Object
                && document.RootElement.TryGetProperty("Properties", out var props)
                && props.ValueKind == JsonValueKind.Object
                ? props
                : document.RootElement;
            if (propertiesElement.ValueKind == JsonValueKind.Object)
            {
                foreach (var name in names)
                {
                    if (propertiesElement.TryGetProperty(name, out var element) && element.ValueKind == JsonValueKind.String)
                    {
                        results[name] = element.GetString() ?? string.Empty;
                    }
                    else
                    {
                        results[name] = string.Empty;
                    }
                }
                return results;
            }
        }
        // Single-property mode: the output is the raw value, one per line.
        var lines = trimmed.Split('\n', StringSplitOptions.RemoveEmptyEntries);
        if (lines.Length >= names.Length)
        {
            for (var i = 0; i < names.Length; i++)
            {
                results[names[i]] = lines[i].Trim();
            }
            return results;
        }
        foreach (var name in names)
        {
            results[name] = string.Empty;
        }
        return results;
    }

    private static List<string> BuildMsbuildArgs(string project, string? itemName, IReadOnlyList<string>? propertyNames)
    {
        var args = new List<string>
        {
            "msbuild",
            project,
            "-nologo",
            "-m:1",
        };
        if (itemName is not null)
        {
            args.Add($"-getItem:{itemName}");
        }
        if (propertyNames is not null)
        {
            foreach (var name in propertyNames)
            {
                args.Add($"-getProperty:{name}");
            }
        }
        return args;
    }

    private static (int ExitCode, string Stdout, string Stderr) RunMsbuild(IReadOnlyList<string> args)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = "dotnet",
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true,
        };
        foreach (var arg in args)
        {
            startInfo.ArgumentList.Add(arg);
        }
        using var process = Process.Start(startInfo) ?? throw new InvalidOperationException("Failed to start dotnet.");
        var stdout = process.StandardOutput.ReadToEnd();
        var stderr = process.StandardError.ReadToEnd();
        process.WaitForExit();
        return (process.ExitCode, stdout, stderr);
    }
}
