using System.Net;
using System.Net.Sockets;
using Microsoft.Extensions.Options;
using Platform.Mailing.Smtp;

namespace Platform.Mailing.ProviderAdapters.Tests;

public sealed class SmtpMailServiceTests
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

    private static SmtpMailService BuildService(FakeSmtpServer server, Action<SmtpMailOptions>? configure = null)
    {
        var options = new SmtpMailOptions
        {
            Host = "127.0.0.1",
            Port = server.Port,
            SecureMode = SmtpSecureMode.None,
            OperationTimeout = TimeSpan.FromSeconds(1),
        };
        configure?.Invoke(options);
        return new SmtpMailService(Options.Create(options));
    }

    private static int UnusedPort()
    {
        var probe = new TcpListener(IPAddress.Loopback, 0);
        probe.Start();
        var port = ((IPEndPoint)probe.LocalEndpoint).Port;
        probe.Stop();
        return port;
    }

    [Fact]
    public async Task Accepted_send_returns_sent_and_delivers_the_mime_payload()
    {
        using var server = new FakeSmtpServer();
        var service = BuildService(server);

        var result = await service.SendAsync(Message());

        Assert.Equal(MailSendOutcome.Sent, result.Outcome);
        Assert.Null(result.ErrorCode);
        Assert.Null(result.ErrorMessage);
        var conversation = Assert.Single(server.Conversations);
        Assert.Contains(conversation.Commands, command => command.Contains("noreply@platform.test", StringComparison.OrdinalIgnoreCase));
        Assert.Contains(conversation.Commands, command => command.Contains("recipient@example.test", StringComparison.OrdinalIgnoreCase));
        Assert.Contains("Subject: Hi", conversation.Data, StringComparison.Ordinal);
        Assert.Contains("text/plain", conversation.Data, StringComparison.Ordinal);
        Assert.Contains("X-Correlation-Id: corr-1", conversation.Data, StringComparison.Ordinal);
        Assert.Equal(SmtpMailProviderState.Healthy, service.Status.State);
    }

    [Fact]
    public async Task Html_only_message_maps_to_an_html_part()
    {
        using var server = new FakeSmtpServer();
        var service = BuildService(server);

        var result = await service.SendAsync(Message(textBody: null, htmlBody: "<p>Hello</p>"));

        Assert.Equal(MailSendOutcome.Sent, result.Outcome);
        var conversation = Assert.Single(server.Conversations);
        Assert.Contains("text/html", conversation.Data, StringComparison.Ordinal);
        Assert.DoesNotContain("text/plain", conversation.Data, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Attachments_are_carried_in_the_mime_payload()
    {
        using var server = new FakeSmtpServer();
        var service = BuildService(server);

        var result = await service.SendAsync(Message(attachment: MailAttachment.Create("report.pdf", "application/pdf", [1, 2, 3])));

        Assert.Equal(MailSendOutcome.Sent, result.Outcome);
        var conversation = Assert.Single(server.Conversations);
        Assert.Contains("report.pdf", conversation.Data, StringComparison.Ordinal);
        Assert.Contains("application/pdf", conversation.Data, StringComparison.Ordinal);
        Assert.Contains("AQID", conversation.Data, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Authentication_is_performed_when_configured()
    {
        using var server = new FakeSmtpServer { AdvertiseAuth = true };
        var service = BuildService(server, options =>
        {
            options.UserName = "smtp-user";
            options.Password = "smtp-pass";
        });

        var result = await service.SendAsync(Message());

        Assert.Equal(MailSendOutcome.Sent, result.Outcome);
        var conversation = Assert.Single(server.Conversations);
        Assert.NotNull(conversation.AuthPayload);
        Assert.Contains("smtp-user", conversation.AuthPayload, StringComparison.Ordinal);
        Assert.Contains("smtp-pass", conversation.AuthPayload, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Recipient_rejection_is_a_permanent_failure()
    {
        using var server = new FakeSmtpServer { RcptReplyCode = 550 };
        var service = BuildService(server);

        var result = await service.SendAsync(Message());

        Assert.Equal(MailSendOutcome.PermanentFailure, result.Outcome);
        Assert.Equal("mail.smtp.recipient_rejected", result.ErrorCode);
        Assert.Equal("The SMTP server rejected one or more recipients.", result.ErrorMessage);
        Assert.Equal(SmtpMailProviderState.Healthy, service.Status.State);
    }

    [Fact]
    public async Task Transient_mail_rejection_is_a_transient_failure()
    {
        using var server = new FakeSmtpServer { MailReplyCode = 451 };
        var service = BuildService(server);

        var result = await service.SendAsync(Message());

        Assert.Equal(MailSendOutcome.TransientFailure, result.Outcome);
        Assert.Equal("mail.smtp.message_rejected", result.ErrorCode);
    }

    [Fact]
    public async Task Connection_refusal_is_a_transient_failure()
    {
        var service = BuildService(new FakeSmtpServer(), options =>
        {
            options.Port = UnusedPort();
        });

        var result = await service.SendAsync(Message());

        Assert.Equal(MailSendOutcome.TransientFailure, result.Outcome);
        Assert.Equal("mail.smtp.connection_failed", result.ErrorCode);
        Assert.Equal(SmtpMailProviderState.Unavailable, service.Status.State);
    }

    [Fact]
    public async Task Operation_timeout_is_a_transient_failure()
    {
        using var server = new FakeSmtpServer { GreetingDelay = TimeSpan.FromSeconds(5) };
        var service = BuildService(server);

        var result = await service.SendAsync(Message());

        Assert.Equal(MailSendOutcome.TransientFailure, result.Outcome);
        Assert.Equal("mail.smtp.timeout", result.ErrorCode);
        Assert.Equal(SmtpMailProviderState.Unavailable, service.Status.State);
    }

    [Fact]
    public async Task Caller_cancellation_is_preserved()
    {
        using var server = new FakeSmtpServer();
        var service = BuildService(server);
        using var cts = new CancellationTokenSource();
        await cts.CancelAsync();

        var send = () => service.SendAsync(Message(), cts.Token);

        await Assert.ThrowsAnyAsync<OperationCanceledException>(send);
    }

    [Fact]
    public async Task Missing_sender_returns_a_configuration_failure_without_contacting_the_server()
    {
        using var server = new FakeSmtpServer();
        var service = BuildService(server);

        var result = await service.SendAsync(Message(from: new MailAddress(" ")));

        Assert.Equal(MailSendOutcome.PermanentFailure, result.Outcome);
        Assert.Equal("mail.configuration.missing_sender", result.ErrorCode);
        Assert.Empty(server.Conversations);
    }

    [Fact]
    public async Task Invalid_recipient_returns_a_configuration_failure_without_contacting_the_server()
    {
        using var server = new FakeSmtpServer();
        var service = BuildService(server);

        var result = await service.SendAsync(Message(to: [new MailAddress(" ")]));

        Assert.Equal(MailSendOutcome.PermanentFailure, result.Outcome);
        Assert.Equal("mail.configuration.invalid_recipient", result.ErrorCode);
        Assert.Empty(server.Conversations);
    }

    [Fact]
    public async Task Invalid_attachment_content_type_returns_a_configuration_failure_without_contacting_the_server()
    {
        using var server = new FakeSmtpServer();
        var service = BuildService(server);

        var result = await service.SendAsync(Message(attachment: new MailAttachment("file.bin", "not a content type", [1])));

        Assert.Equal(MailSendOutcome.PermanentFailure, result.Outcome);
        Assert.Equal("mail.configuration.invalid_attachment", result.ErrorCode);
        Assert.Empty(server.Conversations);
    }
}
