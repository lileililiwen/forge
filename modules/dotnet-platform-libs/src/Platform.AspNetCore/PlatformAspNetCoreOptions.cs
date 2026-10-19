namespace Platform.AspNetCore;

/// <summary>
/// Configures the ASP.NET Core integration. All properties have safe
/// defaults; consumers override only what they need.
/// </summary>
public sealed class PlatformAspNetCoreOptions
{
    /// <summary>
    /// The default options instance.
    /// </summary>
    public static PlatformAspNetCoreOptions Default { get; } = new();

    /// <summary>
    /// Gets or sets the request and response header used to carry the
    /// correlation identifier. Defaults to <c>X-Correlation-Id</c>.
    /// </summary>
    public string CorrelationHeader { get; set; } = "X-Correlation-Id";

    /// <summary>
    /// Gets or sets a value indicating whether the host accepts an
    /// incoming correlation header from the client. When <c>false</c>,
    /// the middleware always generates a fresh identifier. Defaults to
    /// <c>false</c> to avoid accepting client-supplied values that
    /// could pollute server-side logs.
    /// </summary>
    public bool AcceptIncomingCorrelationHeader { get; set; }

    /// <summary>
    /// Gets or sets the maximum length accepted for an incoming
    /// correlation identifier. Defaults to 128 characters.
    /// </summary>
    public int MaxCorrelationIdLength { get; set; } = 128;

    /// <summary>
    /// Gets or sets the endpoint path used for the platform health
    /// check. Defaults to <c>/health</c>.
    /// </summary>
    public string HealthCheckPath { get; set; } = "/health";
}
