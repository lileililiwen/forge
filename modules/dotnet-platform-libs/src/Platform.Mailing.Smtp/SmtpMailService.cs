using System.Net.Sockets;
using MailKit;
using MailKit.Net.Smtp;
using MailKit.Security;
using Microsoft.Extensions.Options;
using MimeKit;

namespace Platform.Mailing.Smtp;

/// <summary>
/// SMTP implementation of the platform <see cref="IMailService"/> contract.
/// Every outcome is reported through the normalized <see cref="MailSendResult"/>;
/// the adapter throws only when the caller cancels. Retry ownership stays
/// with the application: <see cref="MailSendOutcome.TransientFailure"/> is the
/// retry signal, <see cref="MailSendOutcome.PermanentFailure"/> is not.
/// Provider responses are never surfaced; diagnostics carry stable error
/// codes and fixed safe messages only.
/// </summary>
public sealed class SmtpMailService : IMailService
{
    private const string MissingSenderCode = "mail.configuration.missing_sender";
    private const string InvalidSenderCode = "mail.configuration.invalid_sender";
    private const string InvalidRecipientCode = "mail.configuration.invalid_recipient";
    private const string InvalidAttachmentCode = "mail.configuration.invalid_attachment";
    private const string RecipientRejectedCode = "mail.smtp.recipient_rejected";
    private const string SenderRejectedCode = "mail.smtp.sender_rejected";
    private const string MessageRejectedCode = "mail.smtp.message_rejected";
    private const string AuthenticationFailedCode = "mail.smtp.authentication_failed";
    private const string ConnectionFailedCode = "mail.smtp.connection_failed";
    private const string TlsFailedCode = "mail.smtp.tls_failed";
    private const string TimeoutCode = "mail.smtp.timeout";
    private const string UnexpectedFailureCode = "mail.smtp.unexpected_failure";

    private const string TimeoutMessage = "The SMTP operation timed out before the server accepted the message.";
    private const string ConnectionFailedMessage = "The SMTP server could not be reached.";
    private const string UnexpectedFailureMessage = "The SMTP server failed the send attempt.";

    private readonly SmtpMailOptions _options;
    private volatile SmtpMailProviderStatus _status;

    /// <summary>Creates the adapter and validates the supplied options.</summary>
    /// <param name="options">The SMTP options.</param>
    /// <exception cref="ArgumentNullException"><paramref name="options"/> is <c>null</c>.</exception>
    public SmtpMailService(IOptions<SmtpMailOptions> options)
        : this(OptionsHelpers.RequireValue(options))
    {
    }

    /// <summary>Creates the adapter from the supplied options instance.</summary>
    /// <param name="options">The SMTP options.</param>
    /// <exception cref="ArgumentNullException"><paramref name="options"/> is <c>null</c>.</exception>
    public SmtpMailService(SmtpMailOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        options.Validate();
        _options = options;
        _status = new SmtpMailProviderStatus(SmtpMailProviderStatus.ProviderName, SmtpMailProviderState.Healthy);
    }

    /// <summary>
    /// Gets the documented status snapshot derived from the most recent
    /// send attempt.
    /// </summary>
    public SmtpMailProviderStatus Status => _status;

    /// <inheritdoc />
    public async Task<MailSendResult> SendAsync(MailMessage message, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(message);

        var mime = TryBuildMessage(message, out var configurationFailure);
        if (configurationFailure is not null)
        {
            return configurationFailure;
        }

        using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeoutCts.CancelAfter(_options.OperationTimeout);
        try
        {
            using SmtpClient client = _options.ClientFactory is { } factory ? factory() : new SmtpClient();
            await client.ConnectAsync(_options.Host, _options.Port, MapSecureMode(_options.SecureMode), timeoutCts.Token).ConfigureAwait(false);
            try
            {
                if (!string.IsNullOrWhiteSpace(_options.UserName))
                {
                    await client.AuthenticateAsync(_options.UserName, _options.Password!, timeoutCts.Token).ConfigureAwait(false);
                }

                await client.SendAsync(mime!, timeoutCts.Token).ConfigureAwait(false);
            }
            finally
            {
                if (client.IsConnected)
                {
                    await client.DisconnectAsync(true, CancellationToken.None).ConfigureAwait(false);
                }
            }

            _status = Healthy();
            return new MailSendResult(MailSendOutcome.Sent);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (OperationCanceledException) when (timeoutCts.IsCancellationRequested)
        {
            return Unavailable(TimeoutCode, TimeoutMessage);
        }
        catch (SmtpCommandException exception)
        {
            return ClassifySmtpCommandException(exception);
        }
        catch (AuthenticationException)
        {
            _status = Healthy();
            return Permanent(AuthenticationFailedCode, "The SMTP server rejected the configured credentials.");
        }
        catch (ServiceNotAuthenticatedException)
        {
            _status = Healthy();
            return Permanent(AuthenticationFailedCode, "The SMTP server requires authentication that did not succeed.");
        }
        catch (System.Security.Authentication.AuthenticationException)
        {
            return Unavailable(TlsFailedCode, "The SMTP transport could not establish a secure connection.");
        }
        catch (SocketException)
        {
            return Unavailable(ConnectionFailedCode, ConnectionFailedMessage);
        }
        catch (IOException)
        {
            return Unavailable(ConnectionFailedCode, ConnectionFailedMessage);
        }
        catch (Exception)
        {
            return Unavailable(UnexpectedFailureCode, UnexpectedFailureMessage);
        }
    }

    private static MailSendResult ClassifySmtpCommandException(SmtpCommandException exception)
    {
        var statusCode = (int)exception.StatusCode;
        return exception.ErrorCode switch
        {
            SmtpErrorCode.RecipientNotAccepted when statusCode >= 500 => Permanent(RecipientRejectedCode, "The SMTP server rejected one or more recipients."),
            SmtpErrorCode.SenderNotAccepted when statusCode >= 500 => Permanent(SenderRejectedCode, "The SMTP server rejected the sender."),
            SmtpErrorCode.MessageNotAccepted when statusCode >= 500 => Permanent(MessageRejectedCode, "The SMTP server rejected the message content."),
            _ => ClassifyByStatusCode(statusCode),
        };
    }

    private static MailSendResult ClassifyByStatusCode(int statusCode)
    {
        if (statusCode is >= 400 and < 500)
        {
            return Transient(MessageRejectedCode, "The SMTP server temporarily failed the send attempt.");
        }

        if (statusCode >= 500)
        {
            return Permanent(MessageRejectedCode, "The SMTP server rejected the message.");
        }

        return Transient(UnexpectedFailureCode, UnexpectedFailureMessage);
    }

    private static SecureSocketOptions MapSecureMode(SmtpSecureMode mode) => mode switch
    {
        SmtpSecureMode.StartTls => SecureSocketOptions.StartTls,
        SmtpSecureMode.SslOnConnect => SecureSocketOptions.SslOnConnect,
        SmtpSecureMode.None => SecureSocketOptions.None,
        _ => throw new ArgumentOutOfRangeException(nameof(mode), mode, "Unknown SMTP secure mode."),
    };

    private static MimeMessage? TryBuildMessage(MailMessage message, out MailSendResult? failure)
    {
        failure = null;

        if (string.IsNullOrWhiteSpace(message.From.Address))
        {
            failure = Permanent(MissingSenderCode, "No sender address was supplied or configured.");
            return null;
        }

        if (!MailboxAddress.TryParse(message.From.Address.Trim(), out var from) || from is null)
        {
            failure = Permanent(InvalidSenderCode, "The sender address is not a valid mailbox address.");
            return null;
        }

        from.Name = message.From.DisplayName;

        var recipients = new List<MailboxAddress>(message.To.Count);
        foreach (var recipient in message.To)
        {
            if (string.IsNullOrWhiteSpace(recipient.Address)
                || !MailboxAddress.TryParse(recipient.Address.Trim(), out var parsed) || parsed is null)
            {
                failure = Permanent(InvalidRecipientCode, "One or more recipient addresses are not valid mailbox addresses.");
                return null;
            }

            parsed.Name = recipient.DisplayName;
            recipients.Add(parsed);
        }

        var mime = new MimeMessage();
        mime.From.Add(from);
        foreach (var recipient in recipients)
        {
            mime.To.Add(recipient);
        }

        mime.Subject = message.Subject;

        var builder = new BodyBuilder
        {
            TextBody = message.TextBody,
            HtmlBody = message.HtmlBody,
        };
        foreach (var attachment in message.Attachments)
        {
            ContentType contentType;
            try
            {
                contentType = ContentType.Parse(attachment.ContentType);
            }
            catch (FormatException)
            {
                failure = Permanent(InvalidAttachmentCode, "One or more attachments carry an invalid content type.");
                return null;
            }

            using var content = new MemoryStream(attachment.Content, writable: false);
            builder.Attachments.Add(attachment.FileName, content, contentType);
        }

        mime.Body = builder.ToMessageBody();

        if (!string.IsNullOrWhiteSpace(message.CorrelationId))
        {
            mime.Headers.Add("X-Correlation-Id", message.CorrelationId);
        }

        return mime;
    }

    private static MailSendResult Permanent(string code, string message) => new(MailSendOutcome.PermanentFailure, ErrorCode: code, ErrorMessage: message);

    private static MailSendResult Transient(string code, string message) => new(MailSendOutcome.TransientFailure, ErrorCode: code, ErrorMessage: message);

    private MailSendResult Unavailable(string code, string message)
    {
        _status = new SmtpMailProviderStatus(SmtpMailProviderStatus.ProviderName, SmtpMailProviderState.Unavailable, code);
        return Transient(code, message);
    }

    private static SmtpMailProviderStatus Healthy() => new(SmtpMailProviderStatus.ProviderName, SmtpMailProviderState.Healthy);
    private static class OptionsHelpers
    {
        public static SmtpMailOptions RequireValue(IOptions<SmtpMailOptions>? options)
        {
            ArgumentNullException.ThrowIfNull(options);
            var value = options.Value;
            ArgumentNullException.ThrowIfNull(value);
            return value;
        }
    }
}
