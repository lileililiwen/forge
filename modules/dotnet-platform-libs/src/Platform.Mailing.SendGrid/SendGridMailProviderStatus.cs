namespace Platform.Mailing.SendGrid;

/// <summary>Documented health of the SendGrid adapter derived from the most recent send attempt.</summary>
public enum SendGridMailProviderState
{
    /// <summary>The last send attempt reached the provider and produced an HTTP answer.</summary>
    Healthy,

    /// <summary>The last send attempt could not reach the provider or timed out.</summary>
    Unavailable,
}

/// <summary>
/// Documented snapshot of the SendGrid adapter status. The snapshot carries
/// only the provider name, health state, and the last stable error code;
/// it never carries response bodies or the API key.
/// </summary>
/// <param name="Provider">The stable provider name, always <c>sendgrid</c>.</param>
/// <param name="State">The derived health state.</param>
/// <param name="LastErrorCode">The last stable error code, or <c>null</c> when the last attempt succeeded.</param>
public sealed record SendGridMailProviderStatus(string Provider, SendGridMailProviderState State, string? LastErrorCode = null)
{
    /// <summary>The stable provider name reported by <see cref="SendGridMailService"/>.</summary>
    public const string ProviderName = "sendgrid";
}
