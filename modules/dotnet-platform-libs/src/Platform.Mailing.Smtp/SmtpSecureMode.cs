namespace Platform.Mailing.Smtp;

/// <summary>
/// Explicit SMTP transport-security mode. The adapter never guesses: every
/// option value maps to a single MailKit <see cref="MailKit.Security.SecureSocketOptions"/>
/// so ambiguous configurations are rejected instead of silently negotiated.
/// </summary>
public enum SmtpSecureMode
{
    /// <summary>Connect in the clear and upgrade with STARTTLS before authenticating or sending.</summary>
    StartTls,

    /// <summary>Negotiate TLS immediately on connect (implicit TLS, typically port 465).</summary>
    SslOnConnect,

    /// <summary>Plaintext transport with no TLS. Only appropriate for tests and trusted loopback relays.</summary>
    None,
}
