using System.Text.Json;

namespace Platform.Adoption;

/// <summary>
/// Preview-only package alignment output. Never writes files.
/// </summary>
public static class AdoptionPreview
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true,
    };

    /// <summary>Serializes preview edits to stable machine-readable JSON.</summary>
    public static string ToJson(AdoptionReport report)
    {
        ArgumentNullException.ThrowIfNull(report);
        return JsonSerializer.Serialize(
            new
            {
                target = report.TargetDirectory,
                toolVersion = report.ToolVersion,
                proposedEdits = report.ProposedEdits,
                previewOnly = true,
            },
            JsonOptions);
    }
}
