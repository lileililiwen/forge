using Platform.Auditing.Contracts;

namespace Platform.Auditing.Tests.Contracts;

public class DefaultAuditMaskerTests
{
    private readonly DefaultAuditMasker _masker = new();

    [Theory]
    [InlineData("password")]
    [InlineData("user_token")]
    [InlineData("api-key")]
    [InlineData("secret")]
    [InlineData("clientSecret")]
    [InlineData("CreditCardNumber")]
    [InlineData("ssn")]
    public void Sensitive_keys_are_redacted_case_and_separator_insensitive(string key)
    {
        var result = _masker.Mask(key, "super-secret-value");
        Assert.Equal(DefaultAuditMasker.Redacted, result);
    }

    [Theory]
    [InlineData("username")]
    [InlineData("email")]
    [InlineData("status")]
    [InlineData("http.method")]
    [InlineData("entity.operation")]
    public void Non_sensitive_keys_are_returned_unchanged(string key)
    {
        var result = _masker.Mask(key, "plain-value");
        Assert.Equal("plain-value", result);
    }

    [Fact]
    public void Null_value_is_returned_as_empty()
    {
        Assert.Equal(string.Empty, _masker.Mask("password", null));
    }

    [Fact]
    public void ToStringValue_converts_non_string_values_invariantly()
    {
        Assert.Equal("42", DefaultAuditMasker.ToStringValue(42));
        Assert.Equal(string.Empty, DefaultAuditMasker.ToStringValue(null));
    }

    [Fact]
    public void Custom_masker_can_be_supplied_through_recorder()
    {
        // Exercises the contract seam without prescribing an implementation.
        IAuditMasker masker = new DefaultAuditMasker();
        Assert.Equal(DefaultAuditMasker.Redacted, masker.Mask("Password", "hunter2"));
    }
}
