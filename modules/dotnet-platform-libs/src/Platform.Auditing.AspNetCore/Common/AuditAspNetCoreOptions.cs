using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;

namespace Platform.Auditing.AspNetCore.Common;

/// <summary>HTTP capture settings for the ASP.NET Core auditing adapter.</summary>
public sealed class AuditAspNetCoreOptions
{
    /// <summary>Configuration section name.</summary>
    public const string SectionName = "Auditing:AspNetCore";

    /// <summary>Whether HTTP audit capture is enabled. Default <c>true</c>.</summary>
    public bool Enabled { get; set; } = true;

    /// <summary>Path prefixes that are never audited (health/readiness/metrics).</summary>
    public IReadOnlyList<string> ExemptPathPrefixes { get; set; } = new List<string>
    {
        "/health", "/healthz", "/ready", "/live", "/alive", "/metrics",
    };

    /// <summary>Header used to resolve the subject identifier. Default <c>X-Audit-Subject</c>.</summary>
    public string SubjectHeaderName { get; set; } = "X-Audit-Subject";

    /// <summary>Header used to resolve the tenant identifier. Default <c>X-Audit-Tenant</c>.</summary>
    public string TenantHeaderName { get; set; } = "X-Audit-Tenant";

    /// <summary>Header carrying the correlation identifier. Default <c>X-Correlation-Id</c>.</summary>
    public string CorrelationHeaderName { get; set; } = "X-Correlation-Id";

    /// <summary>When <c>true</c>, a bounded request-body preview is captured. Default <c>false</c>.</summary>
    public bool CaptureRequestBodyPreview { get; set; }

    /// <summary>Maximum request-body size (bytes) eligible for a preview. Default 65536.</summary>
    public long MaxBodyPreviewBytes { get; set; } = 65_536;

    /// <summary>Maximum characters retained in a request-body preview. Default 1024.</summary>
    public int BodyPreviewLimit { get; set; } = 1024;

    /// <summary>Status codes treated as security (denied) events. Default 401 and 403.</summary>
    public IReadOnlyList<int> SecurityStatusCodes { get; set; } = new List<int> { 401, 403 };

    /// <summary>Returns the validation errors. Empty when the options instance is valid.</summary>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (string.IsNullOrWhiteSpace(SubjectHeaderName)) errors.Add($"{nameof(SubjectHeaderName)} must not be empty.");
        if (string.IsNullOrWhiteSpace(TenantHeaderName)) errors.Add($"{nameof(TenantHeaderName)} must not be empty.");
        if (string.IsNullOrWhiteSpace(CorrelationHeaderName)) errors.Add($"{nameof(CorrelationHeaderName)} must not be empty.");
        if (MaxBodyPreviewBytes <= 0) errors.Add($"{nameof(MaxBodyPreviewBytes)} must be positive.");
        if (BodyPreviewLimit <= 0) errors.Add($"{nameof(BodyPreviewLimit)} must be positive.");
        if (SecurityStatusCodes.Any(c => c is < 100 or > 599))
        {
            errors.Add($"{nameof(SecurityStatusCodes)} must contain only HTTP status codes (100-599).");
        }

        return errors;
    }

    /// <summary>Returns <c>true</c> when the path starts with a configured exempt prefix.</summary>
    /// <param name="path">The request path.</param>
    /// <returns><c>true</c> when the path should not be audited.</returns>
    public bool IsExempt(string path)
    {
        if (string.IsNullOrEmpty(path)) return false;
        return ExemptPathPrefixes.Any(prefix =>
            path.StartsWith(prefix, StringComparison.OrdinalIgnoreCase));
    }

    /// <summary>Returns <c>true</c> when the status code is configured as a security (denied) status.</summary>
    /// <param name="statusCode">The HTTP status code.</param>
    /// <returns><c>true</c> when the status is treated as a security event.</returns>
    public bool IsSecurityStatus(int statusCode)
        => SecurityStatusCodes.Contains(statusCode);
}
