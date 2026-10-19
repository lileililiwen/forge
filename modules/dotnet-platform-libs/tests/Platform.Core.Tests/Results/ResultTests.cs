using Platform.Core.Results;

namespace Platform.Core.Tests.Results;

public class ResultTests
{
    [Fact]
    public void Success_is_successful_and_has_no_error()
    {
        var result = Result.Success();

        Assert.True(result.IsSuccess);
        Assert.Null(result.Error);
    }

    [Fact]
    public void Failure_is_unsuccessful_and_carries_error()
    {
        var error = new Error("platform.test", "boom");
        var result = Result.Failure(error);

        Assert.False(result.IsSuccess);
        Assert.Same(error, result.Error);
    }

    [Fact]
    public void Failure_rejects_null_error()
    {
        Assert.Throws<ArgumentNullException>(() => Result.Failure(null!));
    }
}
