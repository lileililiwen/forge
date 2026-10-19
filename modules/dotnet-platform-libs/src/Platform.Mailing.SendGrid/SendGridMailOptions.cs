namespace Platform.Mailing.SendGrid;

/// <summary>
/// Configuration for <see cref="SendGridMailService"/>. Values are validated
/// by <see cref="Validate"/> at registration and construction; error messages
/// never echo the API key or any other secret.
/// </summary>
public sealed class SendGridMailOptions
{
    /// <summary>
    /// The configuration section name applications typically bind this
    /// options type from.
    /// </summary>
    public const string SectionName = "Mailing:SendGrid";

    /// <summary>
    /// Gets or sets the SendGrid API key. Required. The value is never
    /// written to diagnostics.
    /// </summary>
    public string ApiKey { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the bounded wait applied to the provider call. Defaults
    /// to 30 seconds; must be between 1 second and 5 minutes.
    /// </summary>
    public TimeSpan OperationTimeout { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>
    /// Validates the configuration. Secret values are never included in the
    /// exception messages.
    /// </summary>
    /// <exception cref="ArgumentException">A required string is missing.</exception>
    /// <exception cref="ArgumentOutOfRangeException"><see cref="OperationTimeout"/> is outside the documented bounds.</exception>
    public void Validate()
    {
        if (string.IsNullOrWhiteSpace(ApiKey))
        {
            throw new ArgumentException("A SendGrid API key is required.", nameof(ApiKey));
        }

        if (OperationTimeout < TimeSpan.FromSeconds(1) || OperationTimeout > TimeSpan.FromMinutes(5))
        {
            throw new ArgumentOutOfRangeException(nameof(OperationTimeout), OperationTimeout, "The SendGrid operation timeout must be between 1 second and 5 minutes.");
        }
    }
}
