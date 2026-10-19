using Platform.Core.Results;

namespace Platform.Core.Tests.Results;

public class ErrorTests
{
    [Fact]
    public void Validation_uses_validation_code()
    {
        var error = Error.Validation("name is required");

        Assert.Equal("platform.validation", error.Code);
        Assert.Equal("name is required", error.Message);
        Assert.Null(error.Metadata);
    }

    [Fact]
    public void NotFound_uses_not_found_code()
    {
        var error = Error.NotFound("user not found");

        Assert.Equal("platform.not_found", error.Code);
        Assert.Equal("user not found", error.Message);
        Assert.Null(error.Metadata);
    }

    [Fact]
    public void Constructor_preserves_metadata()
    {
        var metadata = new Dictionary<string, object?>
        {
            ["field"] = "email",
            ["attempt"] = 3,
        };

        var error = new Error("platform.validation", "bad email", metadata);

        Assert.Equal("platform.validation", error.Code);
        Assert.Equal("bad email", error.Message);
        Assert.Same(metadata, error.Metadata);
        Assert.Equal("email", error.Metadata!["field"]);
        Assert.Equal(3, error.Metadata["attempt"]);
    }

    [Fact]
    public void Error_is_immutable_record()
    {
        var original = new Error("platform.test", "boom");
        var copy = original with { Message = "different" };

        Assert.Equal("boom", original.Message);
        Assert.Equal("different", copy.Message);
    }
}
