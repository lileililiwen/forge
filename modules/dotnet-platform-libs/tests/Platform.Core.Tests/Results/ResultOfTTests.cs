using Platform.Core.Results;

namespace Platform.Core.Tests.Results;

public class ResultOfTTests
{
    [Fact]
    public void Success_carries_value_and_has_no_error()
    {
        var result = Result<int>.Success(42);

        Assert.True(result.IsSuccess);
        Assert.Equal(42, result.Value);
        Assert.Null(result.Error);
    }

    [Fact]
    public void Failure_has_default_value_and_carries_error()
    {
        var error = new Error("platform.test", "boom");
        var result = Result<int>.Failure(error);

        Assert.False(result.IsSuccess);
        Assert.Equal(0, result.Value);
        Assert.Same(error, result.Error);
    }

    [Fact]
    public void Failure_rejects_null_error()
    {
        Assert.Throws<ArgumentNullException>(() => Result<int>.Failure(null!));
    }

    [Fact]
    public void ToResult_drops_value_on_success()
    {
        var source = Result<string>.Success("payload");
        var converted = source.ToResult();

        Assert.True(converted.IsSuccess);
        Assert.Null(converted.Error);
    }

    [Fact]
    public void ToResult_preserves_error_on_failure()
    {
        var error = new Error("platform.test", "boom");
        var source = Result<string>.Failure(error);
        var converted = source.ToResult();

        Assert.False(converted.IsSuccess);
        Assert.Same(error, converted.Error);
    }
}
