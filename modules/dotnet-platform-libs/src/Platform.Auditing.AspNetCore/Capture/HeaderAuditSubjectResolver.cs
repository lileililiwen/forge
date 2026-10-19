using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Options;
using Platform.Auditing.AspNetCore.Common;

namespace Platform.Auditing.AspNetCore.Capture;

/// <summary>The default subject resolver that reads the configured subject and tenant headers.</summary>
public sealed class HeaderAuditSubjectResolver : IAuditSubjectResolver
{
    private readonly AuditAspNetCoreOptions _options;

    /// <summary>Initializes a new resolver with the supplied options.</summary>
    /// <param name="options">The HTTP audit options.</param>
    public HeaderAuditSubjectResolver(IOptions<AuditAspNetCoreOptions> options)
    {
        ArgumentNullException.ThrowIfNull(options);
        _options = options.Value;
    }

    /// <inheritdoc />
    public AuditSubjectResolution Resolve(HttpContext httpContext)
    {
        ArgumentNullException.ThrowIfNull(httpContext);
        var subject = ReadHeader(httpContext, _options.SubjectHeaderName);
        var tenant = ReadHeader(httpContext, _options.TenantHeaderName);
        return new AuditSubjectResolution(subject, tenant);
    }

    private static string? ReadHeader(HttpContext httpContext, string headerName)
    {
        if (!httpContext.Request.Headers.TryGetValue(headerName, out var values)) return null;
        var value = values.ToString();
        return string.IsNullOrWhiteSpace(value) ? null : value;
    }
}
