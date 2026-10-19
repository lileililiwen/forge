namespace Platform.Jobs.Tests;

public class JobPayloadTests
{
    [Fact]
    public void Ctor_stores_name_and_arguments()
    {
        var arguments = new Dictionary<string, object?> { ["tenant"] = "acme" };

        var payload = new JobPayload("billing-renew", arguments);

        Assert.Equal("billing-renew", payload.Name);
        Assert.Same(arguments, payload.Arguments);
    }

    [Fact]
    public void Ctor_allows_null_arguments()
    {
        var payload = new JobPayload("billing-renew");

        Assert.Equal("billing-renew", payload.Name);
        Assert.Null(payload.Arguments);
    }

    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    [InlineData(null)]
    public void Create_rejects_invalid_name(string? name)
    {
        Assert.ThrowsAny<ArgumentException>(() => JobPayload.Create(name!));
    }

    [Fact]
    public void Create_stores_arguments_when_provided()
    {
        var arguments = new Dictionary<string, object?> { ["key"] = "value" };

        var payload = JobPayload.Create("billing-renew", arguments);

        Assert.Equal("billing-renew", payload.Name);
        Assert.Same(arguments, payload.Arguments);
    }
}
