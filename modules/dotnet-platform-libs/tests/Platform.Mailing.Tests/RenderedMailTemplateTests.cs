namespace Platform.Mailing.Tests;

public class RenderedMailTemplateTests
{
    [Fact]
    public void Create_accepts_text_body()
    {
        var template = RenderedMailTemplate.Create("subject", textBody: "hello");

        Assert.Equal("subject", template.Subject);
        Assert.Equal("hello", template.TextBody);
        Assert.Null(template.HtmlBody);
    }

    [Fact]
    public void Create_accepts_html_body()
    {
        var template = RenderedMailTemplate.Create("subject", htmlBody: "<p>hello</p>");

        Assert.Equal("subject", template.Subject);
        Assert.Equal("<p>hello</p>", template.HtmlBody);
        Assert.Null(template.TextBody);
    }

    [Fact]
    public void Create_rejects_empty_subject()
    {
        Assert.Throws<ArgumentException>(
            () => RenderedMailTemplate.Create("", textBody: "hello"));
    }

    [Fact]
    public void Create_rejects_empty_both_bodies()
    {
        Assert.Throws<ArgumentException>(
            () => RenderedMailTemplate.Create("subject"));
        Assert.Throws<ArgumentException>(
            () => RenderedMailTemplate.Create("subject", textBody: "  ", htmlBody: "  "));
    }
}
