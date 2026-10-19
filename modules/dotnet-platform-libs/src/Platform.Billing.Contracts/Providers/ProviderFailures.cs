namespace Platform.Billing.Contracts.Providers;

/// <summary>Safe categories for failures at a billing-provider boundary.</summary>
public enum ProviderFailureKind
{
    /// <summary>A network, timeout, or provider availability failure.</summary>
    Transient,
    /// <summary>A non-retryable provider operation failure.</summary>
    Permanent,
    /// <summary>The adapter is missing valid configuration.</summary>
    Configuration,
    /// <summary>The provider rejected configured credentials.</summary>
    Authentication,
    /// <summary>The provider response could not be safely parsed.</summary>
    MalformedResponse,
}

/// <summary>A provider failure with safe diagnostic metadata.</summary>
public sealed record ProviderFailure(ProviderFailureKind Kind, string Operation, string SafeMessage);

/// <summary>Classifies provider exceptions without retaining secrets or response bodies.</summary>
public static class ProviderFailureClassifier
{
    /// <summary>Returns a safe classification for an exception and operation.</summary>
    public static ProviderFailure Classify(Exception exception, string operation)
    {
        ArgumentNullException.ThrowIfNull(exception);
        ArgumentException.ThrowIfNullOrWhiteSpace(operation);
        var kind = exception switch
        {
            OperationCanceledException => ProviderFailureKind.Transient,
            TimeoutException or HttpRequestException => ProviderFailureKind.Transient,
            UnauthorizedAccessException => ProviderFailureKind.Authentication,
            FormatException or System.Text.Json.JsonException => ProviderFailureKind.MalformedResponse,
            ArgumentException => ProviderFailureKind.Configuration,
            _ => ProviderFailureKind.Permanent,
        };
        return new ProviderFailure(kind, operation, kind == ProviderFailureKind.Transient ? "The billing provider request could not be completed." : "The billing provider operation failed.");
    }
}
