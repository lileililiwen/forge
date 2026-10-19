namespace Platform.Webhooks.Contracts.Common;

/// <summary>Stable, opaque provider identifier (e.g. <c>stripe</c>, <c>github</c>).</summary>
public readonly record struct WebhookProviderId
{
    /// <summary>Creates a validated provider identifier.</summary>
    public WebhookProviderId(string value)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Any(char.IsControl) || value.Length > 64) throw new ArgumentException("Webhook provider identifiers must be non-empty, bounded, and contain no control characters.", nameof(value));
        Value = value;
    }
    /// <summary>Identifier value.</summary>
    public string Value { get; }
    /// <inheritdoc />
    public override string ToString() => Value;
}

/// <summary>Stable provider-scoped event identifier used for replay suppression.</summary>
public readonly record struct WebhookEventId
{
    /// <summary>Creates a validated event identifier.</summary>
    public WebhookEventId(string value)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Any(char.IsControl) || value.Length > 256) throw new ArgumentException("Webhook event identifiers must be non-empty, bounded, and contain no control characters.", nameof(value));
        Value = value;
    }
    /// <summary>Event identifier value.</summary>
    public string Value { get; }
    /// <inheritdoc />
    public override string ToString() => Value;
}

/// <summary>Validated subscription identifier assigned by the application.</summary>
public readonly record struct WebhookSubscriptionId
{
    /// <summary>Creates a validated subscription identifier.</summary>
    public WebhookSubscriptionId(string value)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Any(char.IsControl) || value.Length > 128) throw new ArgumentException("Webhook subscription identifiers must be non-empty, bounded, and contain no control characters.", nameof(value));
        Value = value;
    }
    /// <summary>Subscription identifier value.</summary>
    public string Value { get; }
    /// <inheritdoc />
    public override string ToString() => Value;
}

/// <summary>Validated delivery identifier assigned by the application.</summary>
public readonly record struct WebhookDeliveryId
{
    /// <summary>Creates a validated delivery identifier.</summary>
    public WebhookDeliveryId(string value)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Any(char.IsControl) || value.Length > 128) throw new ArgumentException("Webhook delivery identifiers must be non-empty, bounded, and contain no control characters.", nameof(value));
        Value = value;
    }
    /// <summary>Delivery identifier value.</summary>
    public string Value { get; }
    /// <inheritdoc />
    public override string ToString() => Value;
}
