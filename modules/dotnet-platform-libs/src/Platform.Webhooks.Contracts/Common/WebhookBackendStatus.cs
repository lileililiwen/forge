using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Common;

/// <summary>Provider-neutral backend status.</summary>
public interface IWebhookBackendStatusProvider
{
    /// <summary>Returns a safe backend status snapshot.</summary>
    WebhookBackendStatus GetStatus();
}

/// <summary>Safe backend status snapshot.</summary>
public sealed record WebhookBackendStatus(string Provider, bool Available, string? Detail = null);
