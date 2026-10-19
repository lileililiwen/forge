using System.Net;
using System.Net.Http;
using Microsoft.Extensions.Options;
using Platform.Mailing.SendGrid;
using SendGrid;
using SendGrid.Helpers.Mail;

namespace Platform.Mailing.ProviderAdapters.Tests;

public sealed class SendGridMailServiceTests
{
    private static MailMessage Message(
        MailAddress? from = null,
        IReadOnlyList<MailAddress>? to = null,
        string? textBody = "Hello from the platform.",
        string? htmlBody = null,
        MailAttachment? attachment = null,
        string? correlationId = "corr-1") => new(
        from ?? new MailAddress("noreply@platform.test", "Platform"),
        to ?? [new MailAddress("recipient@example.test")],
        "Hi",
        textBody,
        htmlBody,
        attachment is null ? null : [attachment],
        correlationId);

    private static SendGridMailService BuildService(FakeSendGridClient client, Action<SendGridMailOptions>? configure = null)
    {
        var options = new SendGridMailOptions { ApiKey = "SG.test-key", OperationTimeout = TimeSpan.FromSeconds(1) };
        configure?.Invoke(options);
        return new SendGridMailService(client, Options.Create(options));
    }

    private static Response ResponseWith(HttpStatusCode statusCode, string? messageId = null)
    {
        if (messageId is null)
        {
            return new Response(statusCode, null, null);
        }

        var response = new HttpResponseMessage();
        response.Headers.TryAddWithoutValidation("X-Message-Id", messageId);
        return new Response(statusCode, new StringContent("{}"), response.Headers);
    }

    [Fact]
    public async Task Accepted_response_returns_sent_with_the_provider_message_id()
    {
        var client = new FakeSendGridClient(ResponseWith(HttpStatusCode.OK, messageId: "sg-123"));
        var service = BuildService(client);

        var result = await service.SendAsync(Message());

        Assert.Equal(MailSendOutcome.Sent, result.Outcome);
        Assert.Equal("sg-123", result.ProviderMessageId);
        Assert.Null(result.ErrorCode);
        Assert.Equal(SendGridMailProviderState.Healthy, service.Status.State);
    }

    [Fact]
    public async Task Rate_limit_returns_a_transient_failure()
    {
        var client = new FakeSendGridClient(ResponseWith(HttpStatusCode.TooManyRequests));
        var service = BuildService(client);

        var result = await service.SendAsync(Message());

        Assert.Equal(MailSendOutcome.TransientFailure, result.Outcome);
        Assert.Equal("mail.sendgrid.rate_limited", result.ErrorCode);
        Assert.Equal("The SendGrid provider rate-limited the request.", result.ErrorMessage);
    }

    [Fact]
    public async Task Server_error_returns_a_transient_failure()
    {
        var client = new FakeSendGridClient(ResponseWith(HttpStatusCode.InternalServerError));
        var service = BuildService(client);

        var result = await service.SendAsync(Message());

        Assert.Equal(MailSendOutcome.TransientFailure, result.Outcome);
        Assert.Equal("mail.sendgrid.server_error", result.ErrorCode);
    }

    [Fact]
    public async Task Rejection_returns_a_permanent_failure_with_a_safe_message()
    {
        var client = new FakeSendGridClient(ResponseWith(HttpStatusCode.BadRequest));
        var service = BuildService(client);

        var result = await service.SendAsync(Message());

        Assert.Equal(MailSendOutcome.PermanentFailure, result.Outcome);
        Assert.Equal("mail.sendgrid.rejected", result.ErrorCode);
        Assert.Equal("The SendGrid provider rejected the message.", result.ErrorMessage);
        Assert.DoesNotContain("Hello from the platform.", result.ErrorMessage, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Message_mapping_carries_recipients_bodies_attachments_and_correlation()
    {
        var client = new FakeSendGridClient();
        var service = BuildService(client);

        await service.SendAsync(Message(
            htmlBody: "<p>Hello</p>",
            attachment: MailAttachment.Create("report.pdf", "application/pdf", [1, 2, 3])));

        var sent = Assert.Single(client.SentMessages);
        Assert.Equal("noreply@platform.test", sent.From?.Email);
        Assert.Equal("Platform", sent.From?.Name);
        var recipient = Assert.Single(sent.Personalizations[0].Tos);
        Assert.Equal("recipient@example.test", recipient.Email);
        Assert.Equal("Hi", sent.Subject);
        Assert.Equal("Hello from the platform.", sent.PlainTextContent);
        Assert.Equal("<p>Hello</p>", sent.HtmlContent);
        var attachment = Assert.Single(sent.Attachments);
        Assert.Equal("report.pdf", attachment.Filename);
        Assert.Equal("application/pdf", attachment.Type);
        Assert.Equal(Convert.ToBase64String([1, 2, 3]), attachment.Content);
        Assert.Equal("corr-1", sent.Headers["X-Correlation-Id"]);
    }

    [Fact]
    public async Task Transport_failure_returns_a_transient_failure()
    {
        var client = new FakeSendGridClient
        {
            OnSend = _ => throw new HttpRequestException("connection refused"),
        };
        var service = BuildService(client);

        var result = await service.SendAsync(Message());

        Assert.Equal(MailSendOutcome.TransientFailure, result.Outcome);
        Assert.Equal("mail.sendgrid.unavailable", result.ErrorCode);
        Assert.Equal(SendGridMailProviderState.Unavailable, service.Status.State);
    }

    [Fact]
    public async Task Caller_cancellation_is_preserved()
    {
        var client = new FakeSendGridClient();
        var service = BuildService(client);
        using var cts = new CancellationTokenSource();
        await cts.CancelAsync();

        var send = () => service.SendAsync(Message(), cts.Token);

        await Assert.ThrowsAnyAsync<OperationCanceledException>(send);
        Assert.Empty(client.SentMessages);
    }

    [Fact]
    public async Task Missing_sender_returns_a_configuration_failure_without_calling_the_provider()
    {
        var client = new FakeSendGridClient();
        var service = BuildService(client);

        var result = await service.SendAsync(Message(from: new MailAddress(" ")));

        Assert.Equal(MailSendOutcome.PermanentFailure, result.Outcome);
        Assert.Equal("mail.configuration.missing_sender", result.ErrorCode);
        Assert.Equal(0, client.CallCount);
    }

    [Fact]
    public async Task Invalid_recipient_returns_a_configuration_failure_without_calling_the_provider()
    {
        var client = new FakeSendGridClient();
        var service = BuildService(client);

        var result = await service.SendAsync(Message(to: [new MailAddress(" ")]));

        Assert.Equal(MailSendOutcome.PermanentFailure, result.Outcome);
        Assert.Equal("mail.configuration.invalid_recipient", result.ErrorCode);
        Assert.Equal(0, client.CallCount);
    }

    [Fact]
    public async Task Invalid_attachment_content_type_returns_a_configuration_failure_without_calling_the_provider()
    {
        var client = new FakeSendGridClient();
        var service = BuildService(client);

        var result = await service.SendAsync(Message(attachment: new MailAttachment("file.bin", " ", [1])));

        Assert.Equal(MailSendOutcome.PermanentFailure, result.Outcome);
        Assert.Equal("mail.configuration.invalid_attachment", result.ErrorCode);
        Assert.Equal(0, client.CallCount);
    }
}
