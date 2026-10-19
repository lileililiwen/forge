using System.Text.Json;
using System.Xml.Linq;

namespace Platform.ConsumerConformance;

public sealed class UpgradeRollbackConformanceTests
{
    [Fact]
    public void Consumer_upgrade_rollback_script_exists_and_is_executable()
    {
        var path = ResolveScriptPath("consumer-upgrade-rollback.sh");
        Assert.True(File.Exists(path), "Expected upgrade/rollback script at " + path);
        var info = new FileInfo(path);
        Assert.False(info.Length == 0, "Upgrade/rollback script must not be empty.");
    }

    [Fact]
    public void Conformance_project_declares_a_single_Platform_version_set()
    {
        var document = XDocument.Load(LocateProjectDirectory() is { } dir ? Path.Combine(dir, "Platform.ConsumerConformance.csproj") : throw new InvalidOperationException());
        var platformVersions = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "PackageReference", StringComparison.OrdinalIgnoreCase))
            .Where(e => (e.Attribute("Include")?.Value ?? string.Empty).StartsWith("Platform.", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Version")?.Value ?? string.Empty)
            .Where(v => !string.IsNullOrWhiteSpace(v))
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToArray();

        Assert.True(
            platformVersions.Length == 1,
            "The conformance fixture must pin a single Platform.* version. Found: " + string.Join(", ", platformVersions));
    }

    [Fact]
    public void Conformance_manifest_version_matches_every_referenced_Platform_package()
    {
        var manifest = LoadManifest();
        var versions = manifest.Packages
            .Where(p => p.IsPackable)
            .Select(p => p.Version)
            .Where(v => !string.IsNullOrWhiteSpace(v))
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToArray();

        Assert.NotEmpty(versions);
        Assert.True(
            versions.Length == 1,
            "Every packable Platform.* package must share a single declared version. Found: " + string.Join(", ", versions));
    }

    [Fact]
    public void Conformance_package_version_matches_manifest()
    {
        var manifest = LoadManifest();
        var document = XDocument.Load(Path.Combine(LocateProjectDirectory(), "Platform.ConsumerConformance.csproj"));
        var manifestVersion = manifest.Packages
            .Where(p => p.IsPackable)
            .Select(p => p.Version)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .Single();

        var mismatches = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "PackageReference", StringComparison.OrdinalIgnoreCase))
            .Where(e => (e.Attribute("Include")?.Value ?? string.Empty).StartsWith("Platform.", StringComparison.OrdinalIgnoreCase))
            .Select(e => new
            {
                Name = e.Attribute("Include")?.Value ?? string.Empty,
                Version = e.Attribute("Version")?.Value ?? string.Empty,
            })
            .Where(r => !string.Equals(r.Version, manifestVersion, StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.True(
            mismatches.Length == 0,
            "Every Platform.* package reference must match the manifest version '" + manifestVersion + "'. Mismatches: "
                + string.Join(", ", mismatches.Select(m => m.Name + "@" + m.Version)));
    }

    private static string ResolveScriptPath(string fileName)
    {
        var directory = LocateProjectDirectory();
        for (var current = new DirectoryInfo(directory); current is not null; current = current.Parent)
        {
            var candidate = Path.Combine(current.FullName, "scripts", fileName);
            if (File.Exists(candidate))
            {
                return candidate;
            }
        }
        throw new InvalidOperationException("Could not locate " + fileName + " in any ancestor scripts/ directory.");
    }

    private static PackageManifest LoadManifest()
    {
        var projectDirectory = LocateProjectDirectory();
        var path = Path.GetFullPath(Path.Combine(projectDirectory, "../../eng/package-manifest.json"));
        Assert.True(File.Exists(path), "Package manifest must exist at " + path);
        using var stream = File.OpenRead(path);
        var manifest = JsonSerializer.Deserialize<PackageManifest>(stream, new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true,
        });
        Assert.NotNull(manifest);
        return manifest!;
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
        throw new InvalidOperationException("Unable to locate the conformance project root from " + AppContext.BaseDirectory);
    }

    private sealed class PackageManifest
    {
        public string Name { get; set; } = string.Empty;
        public int SchemaVersion { get; set; }
        public List<PackageManifestEntry> Packages { get; set; } = new();
    }

    private sealed class PackageManifestEntry
    {
        public string PackageId { get; set; } = string.Empty;
        public string Version { get; set; } = string.Empty;
        public bool IsPackable { get; set; }
    }
}
