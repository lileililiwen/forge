namespace Platform.Idempotency.Tests;

public class RequestFingerprintTests
{
    [Fact]
    public void Compute_returns_stable_fingerprint_for_same_inputs()
    {
        var first = RequestFingerprint.Compute("POST", "/v1/orders", "abc123");
        var second = RequestFingerprint.Compute("POST", "/v1/orders", "abc123");

        Assert.Equal(first, second);
    }

    [Fact]
    public void Compute_normalises_method_to_uppercase()
    {
        var upper = RequestFingerprint.Compute("POST", "/v1/orders", "abc123");
        var lower = RequestFingerprint.Compute("post", "/v1/orders", "abc123");

        Assert.Equal(upper, lower);
    }

    [Fact]
    public void Compute_differs_when_any_input_differs()
    {
        var baseFingerprint = RequestFingerprint.Compute("POST", "/v1/orders", "abc123");

        Assert.NotEqual(baseFingerprint, RequestFingerprint.Compute("PUT", "/v1/orders", "abc123"));
        Assert.NotEqual(baseFingerprint, RequestFingerprint.Compute("POST", "/v1/orders/2", "abc123"));
        Assert.NotEqual(baseFingerprint, RequestFingerprint.Compute("POST", "/v1/orders", "def456"));
    }

    [Theory]
    [InlineData("", "/v1/orders", "abc123")]
    [InlineData("POST", "", "abc123")]
    [InlineData("POST", "/v1/orders", "")]
    [InlineData(null, "/v1/orders", "abc123")]
    public void Compute_rejects_invalid_inputs(string? method, string? route, string? bodyHash)
    {
        Assert.ThrowsAny<ArgumentException>(
            () => RequestFingerprint.Compute(method!, route!, bodyHash!));
    }

    [Fact]
    public void ComputeBodyHash_returns_stable_hex_string()
    {
        var first = RequestFingerprint.ComputeBodyHash("hello");
        var second = RequestFingerprint.ComputeBodyHash("hello");

        Assert.Equal(first, second);
        Assert.Equal(64, first.Length);
    }

    [Fact]
    public void ComputeBodyHash_differs_for_different_inputs()
    {
        var first = RequestFingerprint.ComputeBodyHash("hello");
        var second = RequestFingerprint.ComputeBodyHash("world");

        Assert.NotEqual(first, second);
    }
}
