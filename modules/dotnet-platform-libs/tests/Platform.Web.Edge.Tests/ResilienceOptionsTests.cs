using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Web.Resilience;
using Platform.Web.Resilience.DependencyInjection;

namespace Platform.Web.Edge.Tests;

public sealed class ResilienceOptionsTests
{
    [Fact]
    public void Defaults_are_safe_and_bounded()
    {
        var options = new PlatformHttpResilienceOptions();
        var errors = options.Validate();
        Assert.Empty(errors);
        Assert.Equal(TimeSpan.FromSeconds(5), options.AttemptTimeout);
        Assert.Equal(3, options.MaxRetryAttempts);
        Assert.Equal(0.5, options.CircuitBreakerFailureRatio);
        Assert.Equal(TimeSpan.FromSeconds(30), options.CircuitBreakerSamplingDuration);
    }

    [Theory]
    [InlineData(0, true)]
    [InlineData(-1, true)]
    [InlineData(11, true)]
    [InlineData(3, false)]
    [InlineData(10, false)]
    public void Max_retry_attempts_must_be_in_range(int value, bool shouldFail)
    {
        var options = new PlatformHttpResilienceOptions { MaxRetryAttempts = value };
        var errors = options.Validate();
        Assert.Equal(shouldFail, errors.Any(e => e.Contains("MaxRetryAttempts")));
    }

    [Theory]
    [InlineData(0.0, true)]
    [InlineData(0.25, false)]
    [InlineData(1.0, false)]
    [InlineData(1.5, true)]
    public void Circuit_breaker_failure_ratio_must_be_in_range(double value, bool shouldFail)
    {
        var options = new PlatformHttpResilienceOptions { CircuitBreakerFailureRatio = value };
        var errors = options.Validate();
        Assert.Equal(shouldFail, errors.Any(e => e.Contains("CircuitBreakerFailureRatio")));
    }

    [Fact]
    public void Options_validation_is_invoked_at_resolution_time()
    {
        var services = new ServiceCollection();
        services.AddPlatformHttpResilience(options => options.MaxRetryAttempts = 0);
        using var sp = services.BuildServiceProvider();
        Assert.Throws<OptionsValidationException>(() => sp.GetRequiredService<IOptions<PlatformHttpResilienceOptions>>().Value);
    }
}
