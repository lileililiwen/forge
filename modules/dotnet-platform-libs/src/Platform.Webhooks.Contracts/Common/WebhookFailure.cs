namespace Platform.Webhooks.Contracts.Common;

/// <summary>Safe failure information that excludes secrets, raw payloads, and stack traces.</summary>
public sealed record WebhookFailure
{
    /// <summary>Creates a validated, redacted failure.</summary>
    public WebhookFailure(string code, string message, bool transient)
    {
        if (string.IsNullOrWhiteSpace(code)) throw new ArgumentException("A failure code is required.", nameof(code));
        if (string.IsNullOrWhiteSpace(message)) throw new ArgumentException("A failure message is required.", nameof(message));
        Code = code; Message = message; Transient = transient;
    }
    /// <summary>Failure category code (e.g. <c>webhook.signature_invalid</c>).</summary>
    public string Code { get; }
    /// <summary>Safe failure message.</summary>
    public string Message { get; }
    /// <summary>Whether retrying may succeed.</summary>
    public bool Transient { get; }
}
