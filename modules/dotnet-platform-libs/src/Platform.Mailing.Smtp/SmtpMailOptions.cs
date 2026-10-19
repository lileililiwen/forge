using MailKit.Net.Smtp;

namespace Platform.Mailing.Smtp;

/// <summary>
/// Configuration for <see cref="SmtpMailService"/>. All values are bounded
/// and validated by <see cref="Validate"/> at registration and construction;
/// error messages never echo secrets.
/// </summary>
public sealed class SmtpMailOptions
{
    /// <summary>
    /// The configuration section name applications typically bind this
    /// options type from.
    /// </summary>
    public const string SectionName = "Mailing:Smtp";

    /// <summary>Gets or sets the SMTP server host. Required.</summary>
    public string Host { get; set; } = string.Empty;

    /// <summary>Gets or sets the SMTP server port. Defaults to <c>587</c>.</summary>
    public int Port { get; set; } = 587;

    /// <summary>
    /// Gets or sets the transport-security mode. Defaults to
    /// <see cref="SmtpSecureMode.StartTls"/>; no auto-negotiation is performed.
    /// </summary>
    public SmtpSecureMode SecureMode { get; set; } = SmtpSecureMode.StartTls;

    /// <summary>
    /// Gets or sets the user name used when the server requires
    /// authentication. When set, <see cref="Password"/> must be set too.
    /// </summary>
    public string? UserName { get; set; }

    /// <summary>
    /// Gets or sets the password used when the server requires
    /// authentication. When set, <see cref="UserName"/> must be set too.
    /// The value is never written to diagnostics.
    /// </summary>
    public string? Password { get; set; }

    /// <summary>
    /// Gets or sets the bounded wait applied to the whole connect,
    /// authenticate, and send sequence. Defaults to 30 seconds; must be
    /// between 1 second and 5 minutes.
    /// </summary>
    public TimeSpan OperationTimeout { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>
    /// Gets or sets an optional factory used to create the per-send
    /// <see cref="SmtpClient"/>. The default creates a new
    /// <see cref="SmtpClient"/> for every send; applications can supply
    /// their own factory to customize client behavior.
    /// </summary>
    public Func<SmtpClient>? ClientFactory { get; set; }

    /// <summary>
    /// Validates the configuration. Secret values are never included in
    /// the exception messages.
    /// </summary>
    /// <exception cref="ArgumentException">A required string is missing, the user name and password are configured inconsistently, or the port is out of range.</exception>
    /// <exception cref="ArgumentOutOfRangeException"><see cref="OperationTimeout"/> is outside the documented bounds.</exception>
    public void Validate()
    {
        if (string.IsNullOrWhiteSpace(Host))
        {
            throw new ArgumentException("An SMTP host is required.", nameof(Host));
        }

        if (Port is < 1 or > 65535)
        {
            throw new ArgumentOutOfRangeException(nameof(Port), Port, "The SMTP port must be between 1 and 65535.");
        }

        if (OperationTimeout < TimeSpan.FromSeconds(1) || OperationTimeout > TimeSpan.FromMinutes(5))
        {
            throw new ArgumentOutOfRangeException(nameof(OperationTimeout), OperationTimeout, "The SMTP operation timeout must be between 1 second and 5 minutes.");
        }

        if ((UserName is null) != (Password is null))
        {
            throw new ArgumentException("SMTP user name and password must be configured together.", nameof(UserName));
        }
    }
}
