namespace Platform.Webhooks.Contracts.Security;

/// <summary>Validates outbound webhook delivery targets for SSRF safety.</summary>
public interface ISsrfTargetValidator
{
    /// <summary>Validates the supplied <paramref name="target"/>; returns <c>null</c> when the target is safe, otherwise a failure describing the rejection.</summary>
    WebhookTargetValidation Validate(Uri target);
}

/// <summary>Validation result for a webhook target.</summary>
public sealed record WebhookTargetValidation(bool Allowed, WebhookTargetRejection? Rejection = null)
{
    /// <summary>Creates a success result.</summary>
    public static WebhookTargetValidation Success() => new(true);
    /// <summary>Creates a rejection result.</summary>
    public static WebhookTargetValidation Reject(WebhookTargetRejection rejection) => new(false, rejection);
}

/// <summary>Rejection category for a webhook target.</summary>
public enum WebhookTargetRejection
{
    /// <summary>Target is not an absolute URI.</summary>
    NotAbsolute,
    /// <summary>Target scheme is not HTTPS.</summary>
    InsecureScheme,
    /// <summary>Target resolves to a loopback address.</summary>
    Loopback,
    /// <summary>Target resolves to a private network address.</summary>
    PrivateNetwork,
    /// <summary>Target resolves to a link-local address.</summary>
    LinkLocal,
    /// <summary>Target resolved to zero addresses.</summary>
    NoAddresses,
    /// <summary>Target was denied by the configured allow-list.</summary>
    NotAllowListed
}
