using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Options;
using Platform.Observability.Diagnostics;
using Platform.Observability.Redaction;

namespace Platform.Observability.Hosting;

/// <summary>Exposes the platform-managed correlation identifier for the current request.</summary>
public interface IPlatformCorrelationAccessor
{
    /// <summary>Returns the current request's correlation identifier, or <c>null</c> when no identifier has been established.</summary>
    string? Current { get; }
}

/// <summary>Default <see cref="IPlatformCorrelationAccessor"/> that stores the identifier on the current <see cref="HttpContext"/>.</summary>
public sealed class HttpPlatformCorrelationAccessor : IPlatformCorrelationAccessor
{
    private readonly IHttpContextAccessor _accessor;

    /// <summary>Item key used to read and write the correlation identifier on the <see cref="HttpContext.Items"/> collection.</summary>
    public const string HttpContextItemKey = "Platform.Observability.CorrelationId";

    /// <summary>Initializes a new instance of the <see cref="HttpPlatformCorrelationAccessor"/> class.</summary>
    public HttpPlatformCorrelationAccessor(IHttpContextAccessor accessor)
    {
        ArgumentNullException.ThrowIfNull(accessor);
        _accessor = accessor;
    }

    /// <inheritdoc />
    public string? Current => _accessor.HttpContext?.Items[HttpContextItemKey] as string;
}
