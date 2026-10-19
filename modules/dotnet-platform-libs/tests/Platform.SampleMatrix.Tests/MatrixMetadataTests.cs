using System.Text.Json;
using System.Xml.Linq;

namespace Platform.SampleMatrix.Tests;

/// <summary>Verifies the published matrix metadata matches the sample projects on disk.</summary>
public sealed class MatrixMetadataTests
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
    };

    /// <summary>Verifies every matrix entry is complete, accurate, and independent.</summary>
    [Fact]
    public void Matrix_entries_match_sample_projects()
    {
        var root = SampleMatrixFixture.RepositoryRoot();
        var manifest = Path.Combine(root, "samples", "matrix.json");
        Assert.True(File.Exists(manifest));
        var matrix = JsonSerializer.Deserialize<MatrixManifest>(
            File.ReadAllText(manifest),
            JsonOptions);
        Assert.NotNull(matrix);
        Assert.Equal("0.1.0", matrix.ToolVersion);
        Assert.Equal(5, matrix.Samples.Count);

        foreach (var sample in matrix.Samples)
        {
            Assert.False(string.IsNullOrWhiteSpace(sample.Name));
            Assert.False(string.IsNullOrWhiteSpace(sample.VerifyCommand));
            Assert.False(string.IsNullOrWhiteSpace(sample.Rollback));
            Assert.Empty(sample.ExternalPrerequisites);
            var directory = Path.Combine(root, sample.Directory.Replace('/', Path.DirectorySeparatorChar));
            Assert.True(Directory.Exists(directory), $"Missing sample directory {sample.Directory}.");
            var project = Path.Combine(directory, sample.Project);
            Assert.True(File.Exists(project), $"Missing sample project {sample.Project}.");

            var document = XDocument.Load(project);
            Assert.Equal(
                sample.TargetFramework,
                document.Descendants("TargetFramework").Single().Value);
            Assert.Equal(
                "false",
                document.Descendants("IsPackable").Single().Value,
                ignoreCase: true);
            var references = document.Descendants("ProjectReference")
                .Select(element => Path.GetFileNameWithoutExtension(
                    element.Attribute("Include")!.Value.Replace('\\', Path.DirectorySeparatorChar)))
                .ToHashSet(StringComparer.Ordinal);
            Assert.Subset(sample.PlatformReferences.ToHashSet(StringComparer.Ordinal), references);
            Assert.DoesNotContain(references, name => name.Contains("Sample", StringComparison.Ordinal));
            Assert.True(
                File.Exists(Path.Combine(directory, "README.md")),
                $"Missing ownership README for {sample.Name}.");
        }
    }

    private sealed record MatrixManifest(string ToolVersion, List<MatrixSample> Samples);

    private sealed record MatrixSample(
        string Name,
        string Directory,
        string Project,
        string TargetFramework,
        List<string> PlatformReferences,
        List<string> ExternalPrerequisites,
        string VerifyCommand,
        string Rollback);
}
