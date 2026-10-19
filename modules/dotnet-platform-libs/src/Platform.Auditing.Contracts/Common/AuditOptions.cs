using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;

namespace Platform.Auditing.Contracts.Common;

/// <summary>Validation and runtime settings for audit capture and publishing.</summary>
public sealed class AuditOptions
{
    /// <summary>Configuration section name.</summary>
    public const string SectionName = "Auditing";

    /// <summary>What happens when the sink fails. Default <see cref="AuditFailurePolicy.FailOpen"/>.</summary>
    public AuditFailurePolicy FailurePolicy { get; set; } = AuditFailurePolicy.FailOpen;

    /// <summary>How events are dispatched to the sink. Default <see cref="AuditPublishMode.Synchronous"/>.</summary>
    public AuditPublishMode PublishMode { get; set; } = AuditPublishMode.Synchronous;

    /// <summary>Bounded channel capacity when <see cref="PublishMode"/> is <see cref="AuditPublishMode.BoundedAsync"/>. Default 1024.</summary>
    public int BoundedCapacity { get; set; } = 1024;

    /// <summary>Maximum number of metadata entries retained per event. Default 64.</summary>
    public int MaxMetadataEntries { get; set; } = 64;

    /// <summary>Maximum length of any single metadata value after masking. Default 4096.</summary>
    public int MaxMetadataValueLength { get; set; } = 4096;

    /// <summary>Whether the EF Core change-capture interceptor is enabled. Default <c>true</c>.</summary>
    public bool EnableEntityCapture { get; set; } = true;

    /// <summary>Whether HTTP middleware captures a truncated request-body preview. Default <c>false</c>.</summary>
    public bool CaptureRequestBodyPreview { get; set; }

    /// <summary>Categories that are never recorded. Empty by default.</summary>
    public IReadOnlyList<string> ExcludedCategories { get; set; } = new List<string>();

    /// <summary>When non-empty, only these categories are recorded. Empty by default (record all).</summary>
    public IReadOnlyList<string> EnabledCategories { get; set; } = new List<string>();

    /// <summary>Returns the validation errors. Empty when the options instance is valid.</summary>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (BoundedCapacity <= 0) errors.Add($"{nameof(BoundedCapacity)} must be positive.");
        if (MaxMetadataEntries <= 0) errors.Add($"{nameof(MaxMetadataEntries)} must be positive.");
        if (MaxMetadataValueLength <= 0) errors.Add($"{nameof(MaxMetadataValueLength)} must be positive.");
        if (ExcludedCategories.Any(string.IsNullOrWhiteSpace))
        {
            errors.Add($"{nameof(ExcludedCategories)} must not contain empty entries.");
        }

        if (EnabledCategories.Any(string.IsNullOrWhiteSpace))
        {
            errors.Add($"{nameof(EnabledCategories)} must not contain empty entries.");
        }

        if (ExcludedCategories.Count > 0 && EnabledCategories.Count > 0)
        {
            errors.Add($"{nameof(ExcludedCategories)} and {nameof(EnabledCategories)} are mutually exclusive.");
        }

        return errors;
    }

    /// <summary>Returns <c>true</c> when the supplied category is not excluded and, when enabled categories are
    /// configured, is one of them.</summary>
    /// <param name="category">The category to check.</param>
    /// <returns><c>true</c> when the category should be recorded.</returns>
    public bool IsCategoryEnabled(string category)
    {
        if (ExcludedCategories.Count > 0 && ExcludedCategories.Contains(category, StringComparer.OrdinalIgnoreCase))
        {
            return false;
        }

        if (EnabledCategories.Count > 0 && !EnabledCategories.Contains(category, StringComparer.OrdinalIgnoreCase))
        {
            return false;
        }

        return true;
    }
}
