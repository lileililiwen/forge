using System.Text.Json;

namespace Platform.Architecture.Tests;

/// <summary>
/// Covers the platform contract-conformance boundary: canonical envelope
/// fixtures exist for every shared contract, identity/admin packages stay
/// opt-in without owning application users, roles, migrations, or credentials,
/// and the adoption-evidence surface is explicit.
/// </summary>
public sealed class ContractConformanceTests
{
    private static readonly string RepositoryRoot = LocateRepositoryRoot();

    private static readonly string[] SharedContracts =
    {
        "identity-subject",
        "permission",
        "tenant",
        "audit",
        "gate-result",
        "release-evidence",
    };

    [Theory]
    [MemberData(nameof(SharedContractsData))]
    public void Shared_contract_has_valid_and_invalid_fixtures(string contract)
    {
        foreach (var variant in new[] { "valid", "invalid" })
        {
            var path = Path.Combine(RepositoryRoot, "tests", "Platform.ConsumerConformance", "Fixtures", "contracts", $"{contract}.{variant}.json");
            Assert.True(File.Exists(path), $"Contract fixture must exist at {path}");
            using var document = JsonDocument.Parse(File.ReadAllText(path));
            Assert.Equal(JsonValueKind.Object, document.RootElement.ValueKind);
        }
    }

    public static IEnumerable<object[]> SharedContractsData() =>
        SharedContracts.Select(c => new object[] { c });

    [Fact]
    public void Identity_and_admin_packages_own_no_migrations()
    {
        var violations = new List<string>();
        foreach (var root in new[] { "src" })
        {
            var directory = Path.Combine(RepositoryRoot, root);
            foreach (var project in Directory.EnumerateDirectories(directory, "Platform.Identity.*")
                         .Concat(Directory.EnumerateDirectories(directory, "Platform.Admin.*")))
            {
                foreach (var migration in Directory.EnumerateDirectories(project, "Migrations", SearchOption.AllDirectories))
                {
                    if (!IsOutputOrVersionControl(migration))
                    {
                        violations.Add(Path.GetRelativePath(RepositoryRoot, migration));
                    }
                }
            }
        }

        Assert.True(
            violations.Count == 0,
            "Identity/admin packages must not own EF migrations; applications own them. Found: " + string.Join(", ", violations));
    }

    [Fact]
    public void Identity_and_admin_packages_own_no_concrete_user_store_or_context()
    {
        var violations = new List<string>();
        foreach (var file in EnumerateSourceFiles("src", "Platform.Identity.*").Concat(EnumerateSourceFiles("src", "Platform.Admin.*")))
        {
            var name = Path.GetFileName(file);
            if (name.Contains("UserStore", StringComparison.OrdinalIgnoreCase)
                || name.Contains("UserEntity", StringComparison.OrdinalIgnoreCase))
            {
                violations.Add(Path.GetRelativePath(RepositoryRoot, file));
            }

            if (!name.EndsWith(".cs", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            var content = File.ReadAllText(file);
            if (content.Contains(": DbContext", StringComparison.Ordinal)
                && !content.Contains("abstract class", StringComparison.Ordinal))
            {
                violations.Add(Path.GetRelativePath(RepositoryRoot, file) + " (concrete DbContext)");
            }
        }

        Assert.True(
            violations.Count == 0,
            "Identity/admin packages must not own concrete user stores or contexts. Found: " + string.Join(", ", violations));
    }

    [Fact]
    public void Adoption_evidence_surface_is_explicit()
    {
        foreach (var file in new[]
                 {
                     "src/Platform.Adoption/AdoptionEvidenceLevel.cs",
                     "src/Platform.Adoption/AdoptionEvidence.cs",
                     "src/Platform.Adoption/AdoptionEvidenceClassifier.cs",
                 })
        {
            Assert.True(File.Exists(Path.Combine(RepositoryRoot, file)), $"Adoption evidence surface must exist at {file}");
        }

        var levels = File.ReadAllText(Path.Combine(RepositoryRoot, "src/Platform.Adoption/AdoptionEvidenceLevel.cs"));
        foreach (var level in new[] { "Absent", "Configured", "Incompatible", "Unverified", "Verified" })
        {
            Assert.Contains(level, levels, StringComparison.Ordinal);
        }
    }

    private static IEnumerable<string> EnumerateSourceFiles(string root, string pattern)
    {
        var directory = Path.Combine(RepositoryRoot, root);
        foreach (var project in Directory.EnumerateDirectories(directory, pattern))
        {
            foreach (var file in Directory.EnumerateFiles(project, "*", SearchOption.AllDirectories))
            {
                if (!IsOutputOrVersionControl(file))
                {
                    yield return file;
                }
            }
        }
    }

    private static bool IsOutputOrVersionControl(string path)
    {
        var segments = path.Split(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        return segments.Any(s =>
            s.Equals("bin", StringComparison.OrdinalIgnoreCase)
            || s.Equals("obj", StringComparison.OrdinalIgnoreCase)
            || s.Equals(".git", StringComparison.OrdinalIgnoreCase));
    }

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
