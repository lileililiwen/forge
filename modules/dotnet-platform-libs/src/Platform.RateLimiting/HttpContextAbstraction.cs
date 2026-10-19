namespace Platform.RateLimiting;

/// <summary>
/// Framework-neutral wrapper the bypass resolver consumes. The
/// wrapper exposes only the minimum surface the resolver needs so the
/// platform package does not depend on ASP.NET Core.
/// </summary>
public sealed record HttpContextAbstraction(
    string? BypassToken,
    IReadOnlyDictionary<string, string?>? Headers = null,
    string? RemoteIp = null);
