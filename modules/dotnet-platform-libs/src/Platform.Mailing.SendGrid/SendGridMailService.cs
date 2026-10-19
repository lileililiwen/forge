using System.Net.Http;
using Microsoft.Extensions.Options;
using SendGrid;
using SendGrid.Helpers.Mail;

namespace Platform.Mailing.SendGrid;

/// <summary>
/// SendGrid implementation of the platform <see cref="IMailService"/> contract
/// over an application-owned <see cref="ISendGridClient"/>. Provider responses
/// are classified into normalized outcomes; response bodies are never surfaced
/// because they can echo message content or PII. Retry ownership stays with the
/// application: <see cref="MailSendOutcome.TransientFailure"/> is the retry
/// signal, <see cref="MailSendOutcome.PermanentFailure"/> is not.
/// </summary>
public sealed class SendGridMailService : IMailService
{
    private const string MissingSenderCode = "mail.configuration.missing_sender";
    private const string InvalidSenderCode = "mail.configuration.invalid_sender";
    private const string InvalidRecipientCode = "mail.configuration.invalid_recipient";
    private const string InvalidAttachmentCode = "mail.configuration.invalid_attachment";
    private const string RateLimitedCode = "mail.sendgrid.rate_limited";
    private const string ServerErrorCode = "mail.sendgrid.server_error";
    private const string RejectedCode = "mail.sendgrid.rejected";
    private const string UnexpectedStatusCode = "mail.sendgrid.unexpected_status";
    private const string UnavailableCode = "mail.sendgrid.unavailable";
    private const string TimeoutCode = "mail.sendgrid.timeout";

    private const string RateLimitedMessage = "The SendGrid provider rate-limited the request.";
    private const string ServerErrorMessage = "The SendGrid provider reported a server error.";
    private const string RejectedMessage = "The SendGrid provider rejected the message.";
    private const string TimeoutMessage = "The SendGrid call timed out before the provider answered.";
    private const string UnavailableMessage = "The SendGrid provider could not be reached.";

    private const string MessageIdHeader = "X-Message-Id";

    private readonly ISendGridClient _client;
    private readonly SendGridMailOptions _options;
    private volatile SendGridMailProviderStatus _status;

    /// <summary>Creates the adapter over an application-owned SendGrid client.</summary>
    /// <param name="client">The SendGrid client.</param>
    /// <param name="options">The SendGrid options.</param>
    /// <exception cref="ArgumentNullException">An argument is <c>null</c> or the options value is missing.</exception>
    public SendGridMailService(ISendGridClient client, IOptions<SendGridMailOptions> options)
    {
        ArgumentNullException.ThrowIfNull(client);
        ArgumentNullException.ThrowIfNull(options);
        var value = options.Value;
        ArgumentNullException.ThrowIfNull(value);
        value.Validate();
        _client = client;
        _options = value;
        _status = new SendGridMailProviderStatus(SendGridMailProviderStatus.ProviderName, SendGridMailProviderState.Healthy);
    }

    /// <summary>
    /// Gets the documented status snapshot derived from the most recent
    /// send attempt.
    /// </summary>
    public SendGridMailProviderStatus Status => _status;

    /// <inheritdoc />
    public async Task<MailSendResult> SendAsync(MailMessage message, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(message);

        var sendGridMessage = TryBuildMessage(message, out var configurationFailure);
        if (configurationFailure is not null)
        {
            return configurationFailure;
        }

        using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeoutCts.CancelAfter(_options.OperationTimeout);
        Response response;
        try
        {
            response = await _client.SendEmailAsync(sendGridMessage!, timeoutCts.Token).ConfigureAwait(false);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (OperationCanceledException) when (timeoutCts.IsCancellationRequested)
        {
            return Unavailable(TimeoutCode, TimeoutMessage);
        }
        catch (HttpRequestException)
        {
            return Unavailable(UnavailableCode, UnavailableMessage);
        }
        catch (TimeoutException)
        {
            return Unavailable(TimeoutCode, TimeoutMessage);
        }

        if (response is null)
        {
            return Transient(UnexpectedStatusCode, "The SendGrid provider returned no response.");
        }

        var statusCode = (int)response.StatusCode;
        if (statusCode is >= 200 and < 300)
        {
            _status = Healthy();
            return new MailSendResult(MailSendOutcome.Sent, TryGetMessageId(response));
        }

        return ClassifyRejection(statusCode);
    }

    private static MailSendResult ClassifyRejection(int statusCode) => statusCode switch
    {
        429 => Transient(RateLimitedCode, RateLimitedMessage),
        >= 500 => Transient(ServerErrorCode, ServerErrorMessage),
        >= 400 => Permanent(RejectedCode, RejectedMessage),
        _ => Transient(UnexpectedStatusCode, "The SendGrid provider returned an unexpected status."),
    };

    private static string? TryGetMessageId(Response response)
    {
        if (response.Headers is { } headers
            && headers.TryGetValues(MessageIdHeader, out var values))
        {
            return values.FirstOrDefault();
        }

        return null;
    }

    private static SendGridMessage? TryBuildMessage(MailMessage message, out MailSendResult? failure)
    {
        failure = null;

        if (string.IsNullOrWhiteSpace(message.From.Address))
        {
            failure = Permanent(MissingSenderCode, "No sender address was supplied or configured.");
            return null;
        }

        if (!IsPlausibleAddress(message.From.Address))
        {
            failure = Permanent(InvalidSenderCode, "The sender address is not a valid mailbox address.");
            return null;
        }

        foreach (var recipient in message.To)
        {
            if (string.IsNullOrWhiteSpace(recipient.Address) || !IsPlausibleAddress(recipient.Address))
            {
                failure = Permanent(InvalidRecipientCode, "One or more recipient addresses are not valid mailbox addresses.");
                return null;
            }
        }

        var sendGridMessage = new SendGridMessage();
        sendGridMessage.SetFrom(new EmailAddress(message.From.Address.Trim(), message.From.DisplayName));
        sendGridMessage.AddTos(message.To
            .Select(recipient => new EmailAddress(recipient.Address.Trim(), recipient.DisplayName))
            .ToList());
        sendGridMessage.Subject = message.Subject;

        if (message.TextBody is not null)
        {
            sendGridMessage.PlainTextContent = message.TextBody;
        }

        if (message.HtmlBody is not null)
        {
            sendGridMessage.HtmlContent = message.HtmlBody;
        }

        foreach (var attachment in message.Attachments)
        {
            if (string.IsNullOrWhiteSpace(attachment.ContentType))
            {
                failure = Permanent(InvalidAttachmentCode, "One or more attachments carry an invalid content type.");
                return null;
            }

            sendGridMessage.AddAttachment(
                attachment.FileName,
                Convert.ToBase64String(attachment.Content),
                attachment.ContentType);
        }

        if (!string.IsNullOrWhiteSpace(message.CorrelationId))
        {
            sendGridMessage.AddGlobalHeader("X-Correlation-Id", message.CorrelationId);
        }

        return sendGridMessage;
    }

    private static bool IsPlausibleAddress(string address)
    {
        var candidate = address.Trim();
        if (candidate.Any(char.IsWhiteSpace) || candidate.Count(ch => ch == '@') != 1)
        {
            return false;
        }

        var separator = candidate.IndexOf('@');
        return separator > 0
            && separator < candidate.Length - 1;
    }

    private static MailSendResult Permanent(string code, string message) => new(MailSendOutcome.PermanentFailure, ErrorCode: code, ErrorMessage: message);

    private static MailSendResult Transient(string code, string message) => new(MailSendOutcome.TransientFailure, ErrorCode: code, ErrorMessage: message);

    private MailSendResult Unavailable(string code, string message)
    {
        _status = new SendGridMailProviderStatus(SendGridMailProviderStatus.ProviderName, SendGridMailProviderState.Unavailable, code);
        return Transient(code, message);
    }

    private static SendGridMailProviderStatus Healthy() => new(SendGridMailProviderStatus.ProviderName, SendGridMailProviderState.Healthy);
}
