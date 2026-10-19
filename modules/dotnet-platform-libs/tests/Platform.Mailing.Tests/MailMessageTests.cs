namespace Platform.Mailing.Tests;

public class MailMessageTests
{
    [Fact]
    public void Ctor_accepts_text_body_only()
    {
        var message = NewMessage(textBody: "hello");

        Assert.Equal("hello", message.TextBody);
        Assert.Null(message.HtmlBody);
        Assert.Empty(message.Attachments);
    }

    [Fact]
    public void Ctor_accepts_html_body_only()
    {
        var message = new MailMessage(
            MailAddress.Create("from@example.com"),
            new[] { MailAddress.Create("to@example.com") },
            "subject",
            textBody: null,
            htmlBody: "<p>hello</p>");

        Assert.Null(message.TextBody);
        Assert.Equal("<p>hello</p>", message.HtmlBody);
    }

    [Fact]
    public void Ctor_accepts_both_bodies()
    {
        var message = NewMessage(textBody: "hello", htmlBody: "<p>hello</p>");

        Assert.Equal("hello", message.TextBody);
        Assert.Equal("<p>hello</p>", message.HtmlBody);
    }

    [Fact]
    public void Ctor_rejects_empty_both_bodies()
    {
        var exception = Assert.Throws<ArgumentException>(
            () => new MailMessage(
                MailAddress.Create("from@example.com"),
                new[] { MailAddress.Create("to@example.com") },
                "subject",
                textBody: null,
                htmlBody: null));

        Assert.Contains("TextBody or HtmlBody", exception.Message);
    }

    [Fact]
    public void Ctor_rejects_whitespace_both_bodies()
    {
        Assert.Throws<ArgumentException>(
            () => new MailMessage(
                MailAddress.Create("from@example.com"),
                new[] { MailAddress.Create("to@example.com") },
                "subject",
                textBody: "   ",
                htmlBody: "   "));
    }

    [Fact]
    public void Ctor_rejects_empty_recipient_list()
    {
        Assert.Throws<ArgumentException>(
            () => new MailMessage(
                MailAddress.Create("from@example.com"),
                Array.Empty<MailAddress>(),
                "subject",
                textBody: "hello"));
    }

    [Fact]
    public void Ctor_rejects_null_from()
    {
        Assert.Throws<ArgumentNullException>(
            () => new MailMessage(
                from: null!,
                new[] { MailAddress.Create("to@example.com") },
                "subject",
                textBody: "hello"));
    }

    [Fact]
    public void Ctor_rejects_empty_subject()
    {
        Assert.Throws<ArgumentException>(
            () => new MailMessage(
                MailAddress.Create("from@example.com"),
                new[] { MailAddress.Create("to@example.com") },
                subject: "",
                textBody: "hello"));
    }

    [Fact]
    public void Attachments_default_to_empty()
    {
        var message = NewMessage();

        Assert.NotNull(message.Attachments);
        Assert.Empty(message.Attachments);
    }

    [Fact]
    public void CorrelationId_round_trips()
    {
        var message = NewMessage(correlationId: "corr-1");

        Assert.Equal("corr-1", message.CorrelationId);
    }

    private static MailMessage NewMessage(
        string? textBody = "hello",
        string? htmlBody = null,
        string? correlationId = null)
    {
        return new MailMessage(
            MailAddress.Create("from@example.com"),
            new[] { MailAddress.Create("to@example.com") },
            "subject",
            textBody: textBody,
            htmlBody: htmlBody,
            correlationId: correlationId);
    }
}
