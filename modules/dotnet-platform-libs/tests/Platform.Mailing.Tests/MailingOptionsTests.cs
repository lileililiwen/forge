namespace Platform.Mailing.Tests;

public class MailingOptionsTests
{
    [Fact]
    public void Defaults_apply_documented_values()
    {
        var options = new MailingOptions();

        Assert.Equal("noreply@example.invalid", options.DefaultFromAddress);
        Assert.Equal("Platform", options.DefaultFromDisplayName);
        Assert.Equal(3, options.MaxAttempts);
        Assert.Equal(5, options.InitialBackoffSeconds);
        Assert.Equal(60, options.MaxBackoffSeconds);
        Assert.Equal("templates", options.TemplatesPath);
    }

    [Fact]
    public void Section_name_constant_is_stable()
    {
        Assert.Equal("Mailing", MailingOptions.SectionName);
    }
}
