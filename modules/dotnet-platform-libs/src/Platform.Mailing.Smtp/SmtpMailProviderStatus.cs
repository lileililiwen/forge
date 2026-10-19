namespace Platform.Mailing.Smtp;

/// <summary>Documented health of the SMTP adapter derived from the most recent send attempt.</summary>
public enum SmtpMailProviderState
{
    /// <summary>The last send attempt was accepted or was rejected by the server (the server itself is reachable).</summary>
    Healthy,

    /// <summary>The last send attempt failed to reach the server or timed out.</summary>
    Unavailable,
}

/// <summary>
/// Documented snapshot of the SMTP adapter status. The snapshot carries
/// only the provider name, health state, and the last stable error code;
/// it never carries server responses or credentials.
/// </summary>
/// <param name="Provider">The stable provider name, always <c>smtp</c>.</param>
/// <param name="State">The derived health state.</param>
/// <param name="LastErrorCode">The last stable error code, or <c>null</c> when the last attempt succeeded.</param>
public sealed record SmtpMailProviderStatus(string Provider, SmtpMailProviderState State, string? LastErrorCode = null)
{
    /// <summary>The stable provider name reported by <see cref="SmtpMailService"/>.</summary>
    public const string ProviderName = "smtp";
}
