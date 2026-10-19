namespace Platform.Mailing.Tests;

public class MailTemplateIdTests
{
    [Fact]
    public void Ctor_stores_value()
    {
        var id = new MailTemplateId("welcome");

        Assert.Equal("welcome", id.Value);
    }

    [Fact]
    public void Implicit_string_conversion_returns_value()
    {
        MailTemplateId id = "welcome";

        string asString = id;
        Assert.Equal("welcome", asString);
    }

    [Fact]
    public void Implicit_string_to_id_constructor_validates()
    {
        Assert.Throws<ArgumentException>(() => _ = (MailTemplateId)"");
        Assert.Throws<ArgumentException>(() => _ = (MailTemplateId)"   ");
    }

    [Fact]
    public void ToString_returns_value()
    {
        var id = new MailTemplateId("welcome");

        Assert.Equal("welcome", id.ToString());
    }

    [Fact]
    public void Direct_constructor_validates_input()
    {
        Assert.Throws<ArgumentException>(() => new MailTemplateId(""));
        Assert.Throws<ArgumentException>(() => new MailTemplateId("   "));
    }
}
