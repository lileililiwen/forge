using System.Text.Json;
using System.Xml.Linq;

namespace Platform.Adoption;

/// <summary>
/// A package reference discovered during inventory.
/// </summary>
/// <param name="Id">Package identifier.</param>
/// <param name="Version">Declared version, or <c>(centrally managed)</c> when declared without one.</param>
public sealed record InventoriedPackage(string Id, string Version);

/// <summary>
/// A project discovered during inventory with its package references.
/// </summary>
/// <param name="RelativePath">Project file path relative to the target directory.</param>
/// <param name="Packages">Package references in project-file order.</param>
public sealed record InventoriedProject(string RelativePath, IReadOnlyList<InventoriedPackage> Packages);

/// <summary>
/// Read-only project and package inventory over an explicit target directory.
/// </summary>
public static class AdoptionInventory
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true,
    };

    /// <summary>Lists projects and their package references under the target.</summary>
    /// <param name="projectDir">Absolute path to the project/repository directory.</param>
    /// <exception cref="AdoptionTargetException">The target is not an absolute existing directory.</exception>
    public static IReadOnlyList<InventoriedProject> ListProjects(string projectDir)
    {
        var target = AdoptionAnalyzer.ResolveTarget(projectDir);
        var result = new List<InventoriedProject>();
        foreach (var project in Directory.EnumerateFiles(target, "*.csproj", SearchOption.AllDirectories)
                     .Where(path => !IsExcluded(target, path))
                     .OrderBy(path => path, StringComparer.Ordinal))
        {
            var packages = new List<InventoriedPackage>();
            var document = TryLoad(project);
            if (document is not null)
            {
                foreach (var reference in document.Descendants().Where(e => e.Name.LocalName == "PackageReference"))
                {
                    var id = reference.Attribute("Include")?.Value ?? string.Empty;
                    if (id.Length == 0)
                    {
                        continue;
                    }

                    packages.Add(new InventoriedPackage(id, reference.Attribute("Version")?.Value ?? "(centrally managed)"));
                }
            }

            result.Add(new InventoriedProject(Path.GetRelativePath(target, project), packages));
        }

        return result;
    }

    /// <summary>Serializes an inventory to stable machine-readable JSON.</summary>
    public static string ToJson(string targetDirectory, IReadOnlyList<InventoriedProject> projects)
    {
        ArgumentNullException.ThrowIfNull(projects);
        return JsonSerializer.Serialize(
            new { target = targetDirectory, toolVersion = AdoptionAnalyzer.ToolVersion, projects },
            JsonOptions);
    }

    private static bool IsExcluded(string target, string path)
    {
        var segments = Path.GetRelativePath(target, path)
            .Split(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        return segments.Any(segment =>
            segment.Equals("bin", StringComparison.OrdinalIgnoreCase)
            || segment.Equals("obj", StringComparison.OrdinalIgnoreCase)
            || segment.Equals(".git", StringComparison.OrdinalIgnoreCase));
    }

    private static XDocument? TryLoad(string path)
    {
        try
        {
            return XDocument.Load(path);
        }
        catch (Exception ex) when (ex is IOException || ex is UnauthorizedAccessException || ex is System.Xml.XmlException)
        {
            return null;
        }
    }
}
