using System.Reflection;
using System.Xml.Linq;

namespace Platform.ConsumerConformance;

public sealed class PackageFeedVerificationTests
{
    [Fact]
    public void Conformance_project_does_not_reference_any_platform_project()
    {
        var projectPath = LocateProjectPath();
        var document = XDocument.Load(projectPath);

        var projectReferences = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "ProjectReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .ToArray();

        Assert.True(
            projectReferences.Length == 0,
            "The conformance fixture must only consume platform packages via <PackageReference>; found: " + string.Join(", ", projectReferences));
    }

    [Fact]
    public void Conformance_project_consumes_platform_packages_via_PackageReference()
    {
        var projectPath = LocateProjectPath();
        var document = XDocument.Load(projectPath);

        var packageReferences = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "PackageReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(value => value.StartsWith("Platform.", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.NotEmpty(packageReferences);
        Assert.Contains(packageReferences, name => name.Equals("Platform.Core", StringComparison.OrdinalIgnoreCase));
        Assert.Contains(packageReferences, name => name.Equals("Platform.AspNetCore", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Loaded_assembly_references_platform_assemblies_only_as_nuget_packages()
    {
        var assembly = typeof(PackageFeedVerificationTests).Assembly;
        var referencedAssemblies = assembly.GetReferencedAssemblies()
            .Where(name => name.Name?.StartsWith("Platform.", StringComparison.OrdinalIgnoreCase) == true)
            .Select(name => name.Name!)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .OrderBy(name => name, StringComparer.OrdinalIgnoreCase)
            .ToArray();

        Assert.NotEmpty(referencedAssemblies);
        Assert.Contains("Platform.Core", referencedAssemblies, StringComparer.OrdinalIgnoreCase);
        Assert.Contains("Platform.AspNetCore", referencedAssemblies, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public void Local_nuget_config_declares_local_platform_feed()
    {
        var configPath = Path.Combine(LocateProjectDirectory(), "nuget.config");
        Assert.True(File.Exists(configPath), "nuget.config must exist next to the conformance project");

        var document = XDocument.Load(configPath);
        var sources = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "add", StringComparison.OrdinalIgnoreCase))
            .Select(e => new { Key = e.Attribute("key")?.Value, Value = e.Attribute("value")?.Value })
            .Where(e => !string.IsNullOrWhiteSpace(e.Key))
            .ToArray();

        var localFeed = Assert.Single(sources, s => string.Equals(s.Key, "local-platform-feed", StringComparison.OrdinalIgnoreCase));
        Assert.Equal(".local-feed", localFeed.Value);
    }

    [Fact]
    public void Local_feed_directory_is_excluded_from_source_control()
    {
        var gitignorePath = Path.Combine(LocateProjectDirectory(), ".gitignore");
        Assert.True(File.Exists(gitignorePath), "conformance project must ship a .gitignore");
        var contents = File.ReadAllText(gitignorePath);
        Assert.Contains(".local-feed", contents, StringComparison.OrdinalIgnoreCase);
    }

    private static string LocateProjectPath()
    {
        var directory = LocateProjectDirectory();
        return Path.Combine(directory, "Platform.ConsumerConformance.csproj");
    }

    private static string LocateProjectDirectory()
    {
        var current = new DirectoryInfo(AppContext.BaseDirectory);
        while (current is not null)
        {
            var candidate = Path.Combine(current.FullName, "Platform.ConsumerConformance.csproj");
            if (File.Exists(candidate))
            {
                return current.FullName;
            }
            current = current.Parent;
        }
        throw new InvalidOperationException(
            "Unable to locate the conformance project root from " + AppContext.BaseDirectory);
    }
}
