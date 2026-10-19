using Microsoft.AspNetCore.Http;
using System.Text.Json;
using Platform.Privacy.Contracts;

namespace Platform.Privacy.AspNetCore.Cookies;

/// <summary>Reads and writes the plain-JSON consent cookie.</summary>
/// <remarks>
/// The cookie carries grants, not secrets: tampering degrades to
/// Necessary-only rather than failing the request. Signed cookies are
/// host-coupled and out of scope.
/// </remarks>
public sealed class ConsentCookieService
{
    private readonly ConsentCookieOptions _options;

    /// <summary>Creates the service.</summary>
    public ConsentCookieService(ConsentCookieOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        options.Validate();
        _options = options;
    }

    /// <summary>Reads the record from the request cookie, or null when absent or unparsable.</summary>
    public ConsentRecord? Read(HttpRequest request)
    {
        ArgumentNullException.ThrowIfNull(request);
        if (!request.Cookies.TryGetValue(_options.CookieName, out var value) || string.IsNullOrWhiteSpace(value))
        {
            return null;
        }

        try
        {
            var payload = JsonSerializer.Deserialize<ConsentPayload>(value);
            if (payload is null || string.IsNullOrWhiteSpace(payload.SubjectId))
            {
                return null;
            }

            return new ConsentRecord(
                payload.SubjectId,
                payload.Granted.Distinct().ToHashSet(),
                payload.GrantedAt,
                payload.PolicyVersion);
        }
        catch (JsonException)
        {
            return null;
        }
    }

    /// <summary>Writes the record to the response cookie.</summary>
    public void Write(HttpResponse response, ConsentRecord record)
    {
        ArgumentNullException.ThrowIfNull(response);
        ArgumentNullException.ThrowIfNull(record);
        var payload = new ConsentPayload(record.SubjectId, record.Granted.ToArray(), record.GrantedAt, record.PolicyVersion);
        response.Cookies.Append(_options.CookieName, JsonSerializer.Serialize(payload), new CookieOptions
        {
            HttpOnly = true,
            Secure = true,
            SameSite = SameSiteMode.Lax,
            MaxAge = _options.Lifetime,
        });
    }

    private sealed record ConsentPayload(string SubjectId, ConsentCategory[] Granted, DateTimeOffset GrantedAt, int PolicyVersion);
}
