namespace Platform.Mailing.Tests;

public class MailAddressTests
{
    [Fact]
    public void Ctor_stores_address_and_display_name()
    {
        var address = new MailAddress("hello@example.com", "Hello");

        Assert.Equal("hello@example.com", address.Address);
        Assert.Equal("Hello", address.DisplayName);
    }

    [Fact]
    public void Ctor_allows_null_display_name()
    {
        var address = new MailAddress("hello@example.com");

        Assert.Equal("hello@example.com", address.Address);
        Assert.Null(address.DisplayName);
    }

    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    public void Create_rejects_invalid_address(string value)
    {
        Assert.Throws<ArgumentException>(() => MailAddress.Create(value));
    }
}
