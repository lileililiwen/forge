namespace Platform.Web.Cors;

/// <summary>Configures the platform CORS integration. Defaults are safe-by-construction; production mode adds extra checks.</summary>
public sealed class PlatformWebCorsOptions
{
    /// <summary>Gets or sets the host environment name. When <c>Production</c>, the validator enforces stricter rules.</summary>
    public string? Environment { get; set; }

    /// <summary>Gets or sets the policies to register. Each policy is exposed by name and applied through <c>UseCors</c>.</summary>
    public IList<PlatformWebCorsPolicyOptions> Policies { get; set; } = new List<PlatformWebCorsPolicyOptions>();

    /// <summary>Validates option values and returns human-readable failures.</summary>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (Policies.Count == 0)
        {
            errors.Add("At least one CORS policy must be configured.");
            return errors;
        }

        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var policy in Policies)
        {
            if (policy is null)
            {
                errors.Add("Policies must not contain null entries.");
                continue;
            }
            if (!seen.Add(policy.Name)) errors.Add($"Duplicate CORS policy name: '{policy.Name}'.");
            errors.AddRange(policy.Validate(Environment).Select(e => $"[{policy.Name}] {e}"));
        }
        return errors;
    }
}
