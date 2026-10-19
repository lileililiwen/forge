using System.Text.Json;
using System.Xml.Linq;

namespace Platform.ConsumerConformance;

public sealed class AdoptionConformanceTests
{
    private const string ManifestRelativePath = "../../eng/package-manifest.json";

    [Fact]
    public void Conformance_project_pins_every_Platform_package_to_an_exact_version()
    {
        var document = XDocument.Load(LocateProjectPath());
        var references = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "PackageReference", StringComparison.OrdinalIgnoreCase))
            .Where(e => (e.Attribute("Include")?.Value ?? string.Empty).StartsWith("Platform.", StringComparison.OrdinalIgnoreCase))
            .ToArray();

        Assert.NotEmpty(references);

        foreach (var reference in references)
        {
            var name = reference.Attribute("Include")?.Value ?? string.Empty;
            var version = reference.Attribute("Version")?.Value ?? string.Empty;
            Assert.False(
                string.IsNullOrWhiteSpace(version),
                $"{name} must declare a pinned Version attribute (no floating versions).");
            Assert.False(
                ContainsFloating(version),
                $"{name} must use an exact version. Floating, range, or wildcard versions are not allowed (saw '{version}').");
        }
    }

    [Fact]
    public void Build_policy_fixtures_pin_every_Platform_package_to_an_exact_version()
    {
        // The exact-version rule applies to every Platform.* package, including
        // the build-policy package, whose fixtures are the first surface in this
        // repository that references it. The main conformance project does not
        // reference Platform.Build - it demonstrates the library packages - so
        // the rule would otherwise be unenforced for the policy route.
        var fixtures = Directory
            .EnumerateFiles(
                Path.Combine(LocateProjectDirectory(), "Fixtures", "BuildPolicy"),
                "*.csproj",
                SearchOption.AllDirectories)
            .ToArray();

        Assert.NotEmpty(fixtures);

        var references = fixtures
            .SelectMany(fixture => XDocument.Load(fixture)
                .Descendants()
                .Where(e => string.Equals(e.Name.LocalName, "PackageReference", StringComparison.OrdinalIgnoreCase))
                .Where(e => (e.Attribute("Include")?.Value ?? string.Empty).StartsWith("Platform.", StringComparison.OrdinalIgnoreCase))
                .Select(e => new { Fixture = fixture, Name = e.Attribute("Include")?.Value ?? string.Empty, Version = e.Attribute("Version")?.Value ?? string.Empty }))
            .ToArray();

        Assert.NotEmpty(references);
        Assert.Contains(references, reference => reference.Name == "Platform.Build");

        foreach (var reference in references)
        {
            var where = Path.GetFileName(reference.Fixture);
            Assert.False(
                string.IsNullOrWhiteSpace(reference.Version),
                $"{reference.Name} in {where} must declare a pinned Version attribute (no floating versions).");
            Assert.False(
                ContainsFloating(reference.Version),
                $"{reference.Name} in {where} must use an exact version. Floating, range, or wildcard versions are not allowed (saw '{reference.Version}').");
        }
    }

    [Fact]
    public void Conformance_project_pins_testing_packages_to_the_same_version_as_their_public_contracts()
    {
        var document = XDocument.Load(LocateProjectPath());
        var references = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "PackageReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => new
            {
                Name = e.Attribute("Include")?.Value ?? string.Empty,
                Version = e.Attribute("Version")?.Value ?? string.Empty,
            })
            .Where(r => IsComponentScopedTesting(r.Name))
            .ToArray();

        Assert.NotEmpty(references);
        var manifest = LoadManifest();
        var versionById = manifest.Packages.ToDictionary(p => p.PackageId, p => p.Version, StringComparer.OrdinalIgnoreCase);

        foreach (var testing in references)
        {
            var partner = ResolvePartnerPackage(testing.Name, versionById.Keys);
            Assert.NotNull(partner);
            var partnerVersion = versionById[partner!];
            Assert.Equal(
                testing.Version,
                partnerVersion,
                StringComparer.OrdinalIgnoreCase);
        }
    }

    [Fact]
    public void Conformance_project_does_not_publish_a_Platform_package()
    {
        var document = XDocument.Load(LocateProjectPath());
        var isPackable = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "IsPackable", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Value?.Trim() ?? string.Empty)
            .FirstOrDefault(value => !string.IsNullOrWhiteSpace(value));

        Assert.False(
            string.Equals(isPackable, "true", StringComparison.OrdinalIgnoreCase),
            "The conformance project must remain unpackable; it consumes Platform.* packages and must not re-publish them.");
    }

    [Fact]
    public void Package_manifest_lists_every_Platform_package_referenced_by_the_fixture()
    {
        var manifest = LoadManifest();
        var manifestIds = manifest.Packages
            .Where(p => p.IsPackable)
            .Select(p => p.PackageId)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);

        var document = XDocument.Load(LocateProjectPath());
        var referenced = document
            .Descendants()
            .Where(e => string.Equals(e.Name.LocalName, "PackageReference", StringComparison.OrdinalIgnoreCase))
            .Select(e => e.Attribute("Include")?.Value ?? string.Empty)
            .Where(name => name.StartsWith("Platform.", StringComparison.OrdinalIgnoreCase))
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToArray();

        Assert.NotEmpty(referenced);
        var missing = referenced.Where(name => !manifestIds.Contains(name)).ToArray();
        Assert.True(
            missing.Length == 0,
            "The conformance fixture references Platform.* packages that the manifest does not declare. Update the manifest or fix the fixture. Missing: " + string.Join(", ", missing));
    }

    [Fact]
    public void Package_manifest_declares_only_pinned_versions_for_packable_packages()
    {
        var manifest = LoadManifest();
        var floating = manifest.Packages
            .Where(p => p.IsPackable && ContainsFloating(p.Version))
            .Select(p => p.PackageId + "@" + p.Version)
            .ToArray();
        Assert.True(
            floating.Length == 0,
            "Packable packages must declare a concrete version. Floating: " + string.Join(", ", floating));
    }

    [Fact]
    public void Package_manifest_testing_packages_have_a_matching_public_contract()
    {
        var manifest = LoadManifest();
        var ids = manifest.Packages.Select(p => p.PackageId).ToHashSet(StringComparer.OrdinalIgnoreCase);
        var testing = manifest.Packages
            .Where(p => p.IsPackable && IsComponentScopedTesting(p.PackageId))
            .ToArray();

        Assert.NotEmpty(testing);
        var missing = new List<string>();
        foreach (var entry in testing)
        {
            var partner = ResolvePartnerPackage(entry.PackageId, ids);
            if (partner is null)
                missing.Add(entry.PackageId);
        }
        Assert.True(
            missing.Count == 0,
            "Testing packages without a matching public contract: " + string.Join(", ", missing));
    }

    [Fact]
    public void Package_manifest_recognises_Platform_Testing_as_a_standalone_test_helper()
    {
        var manifest = LoadManifest();
        var testing = manifest.Packages.SingleOrDefault(p => p.PackageId == "Platform.Testing");
        Assert.NotNull(testing);
        Assert.True(testing!.IsPackable, "Platform.Testing must remain a packable test helper.");
    }

    private static bool IsComponentScopedTesting(string packageId) =>
        packageId.StartsWith("Platform.", StringComparison.OrdinalIgnoreCase)
        && packageId.EndsWith(".Testing", StringComparison.OrdinalIgnoreCase)
        && !string.Equals(packageId, "Platform.Testing", StringComparison.OrdinalIgnoreCase);

    private static string? ResolvePartnerPackage(string testingPackageId, IEnumerable<string> candidatePackageIds)
    {
        var baseId = testingPackageId[..^".Testing".Length];
        var candidates = new HashSet<string>(candidatePackageIds, StringComparer.OrdinalIgnoreCase);
        if (candidates.Contains(baseId))
            return baseId;
        var withContracts = baseId + ".Contracts";
        if (candidates.Contains(withContracts))
            return withContracts;
        return null;
    }

    [Fact]
    public void Package_manifest_never_records_test_assemblies()
    {
        var manifest = LoadManifest();
        var testAssemblies = manifest.Packages
            .Where(p => p.Path.Contains("tests/", StringComparison.OrdinalIgnoreCase) || p.Path.Contains("/Tests/", StringComparison.OrdinalIgnoreCase))
            .ToArray();
        Assert.True(
            testAssemblies.Length == 0,
            "The manifest must not include any tests/ projects. Found: " + string.Join(", ", testAssemblies.Select(p => p.Path)));
    }

    private static PackageManifest LoadManifest()
    {
        var projectDirectory = LocateProjectDirectory();
        var path = Path.GetFullPath(Path.Combine(projectDirectory, ManifestRelativePath));
        Assert.True(File.Exists(path), "Package manifest must exist at " + path);
        using var stream = File.OpenRead(path);
        var manifest = JsonSerializer.Deserialize<PackageManifest>(stream, new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true,
        });
        Assert.NotNull(manifest);
        return manifest!;
    }

    private static bool ContainsFloating(string version) =>
        !string.IsNullOrWhiteSpace(version)
        && (version.Contains('*')
            || version.Contains('[')
            || version.Contains('(')
            || version.Contains(',')
            || version.Equals("latest", StringComparison.OrdinalIgnoreCase));

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
        public string Path { get; set; } = string.Empty;
        public bool IsPackable { get; set; }
    }
}
