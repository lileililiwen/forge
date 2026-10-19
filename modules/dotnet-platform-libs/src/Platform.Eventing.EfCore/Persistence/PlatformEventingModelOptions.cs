namespace Platform.Eventing.EfCore;

/// <summary>Application-configurable names for durable eventing tables.</summary>
public sealed class PlatformEventingModelOptions
{
    /// <summary>Outbox table name.</summary>
    public string OutboxTableName { get; set; } = "platform_outbox";

    /// <summary>Inbox table name.</summary>
    public string InboxTableName { get; set; } = "platform_inbox";

    internal void Validate()
    {
        ValidateName(OutboxTableName, nameof(OutboxTableName));
        ValidateName(InboxTableName, nameof(InboxTableName));
    }

    private static void ValidateName(string value, string parameterName)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Any(c => !(char.IsLetterOrDigit(c) || c == '_')))
            throw new ArgumentException("Table name must contain only letters, digits, and underscores.", parameterName);
    }
}
