using Microsoft.Extensions.DependencyInjection;
using Platform.Web.Telemetry;
using Platform.Web.Telemetry.DependencyInjection;

namespace Platform.Web.Edge.Tests;

public sealed class TelemetryContractTests
{
    [Fact]
    public void Telemetry_names_are_stable()
    {
        Assert.Equal("Platform.Web.Requests", PlatformWebTelemetryNames.RequestActivitySource);
        Assert.Equal("Platform.Web.Providers", PlatformWebTelemetryNames.ProviderActivitySource);
        Assert.Equal("platform.web.request", PlatformWebTelemetryNames.RequestOperation);
        Assert.Equal("platform.web.provider", PlatformWebTelemetryNames.ProviderOperation);
        Assert.Equal("platform.web.cors", PlatformWebTelemetryNames.CorsOperation);
        Assert.Equal("platform.web.resilience", PlatformWebTelemetryNames.ResilienceOperation);
        Assert.Equal("platform.web.openapi", PlatformWebTelemetryNames.OpenApiOperation);
    }

    [Fact]
    public void Default_options_validate()
    {
        var options = new PlatformWebTelemetryOptions();
        Assert.Empty(options.Validate());
    }

    [Theory]
    [InlineData("", "ApplicationName is required.")]
    [InlineData("demo", null)]
    public void Application_name_is_required(string applicationName, string? expected)
    {
        var options = new PlatformWebTelemetryOptions { ApplicationName = applicationName };
        var errors = options.Validate();
        if (expected is null) Assert.DoesNotContain(errors, e => e.Contains("ApplicationName"));
        else Assert.Contains(errors, e => e == expected);
    }

    [Theory]
    [InlineData(8, true)]
    [InlineData(16, false)]
    [InlineData(1024, false)]
    [InlineData(1025, true)]
    public void Max_tag_length_must_be_bounded(int value, bool shouldFail)
    {
        var options = new PlatformWebTelemetryOptions { MaxTagLength = value };
        var errors = options.Validate();
        Assert.Equal(shouldFail, errors.Any(e => e.Contains("MaxTagLength")));
    }

    [Fact]
    public void Safe_value_policy_redacts_and_bounds()
    {
        var longRedactor = new LongRedactor();
        var policy = new PlatformWebTelemetrySafeValuePolicy(longRedactor, 16, 8, truncateOversizedValues: true);
        Assert.Equal(new string('*', 16), policy.RedactTag("input"));
        Assert.Equal(new string('*', 8), policy.RedactOperation("input"));
    }

    [Fact]
    public void Safe_value_policy_drops_when_truncation_is_disabled()
    {
        var longRedactor = new LongRedactor();
        var policy = new PlatformWebTelemetrySafeValuePolicy(longRedactor, 8, 4, truncateOversizedValues: false);
        Assert.Equal(string.Empty, policy.RedactTag("input"));
        Assert.Equal(string.Empty, policy.RedactOperation("input"));
    }

    private sealed class LongRedactor : IPlatformWebTelemetryRedactor
    {
        public string Redact(string? value) => value is null ? string.Empty : new string('*', 64);
    }

    [Fact]
    public void Safe_value_policy_requires_non_empty_operation()
    {
        var policy = new PlatformWebTelemetrySafeValuePolicy(new DefaultPlatformWebTelemetryRedactor(), 64, 64, truncateOversizedValues: true);
        Assert.Throws<ArgumentException>(() => policy.RequireOperation(""));
        Assert.Throws<ArgumentException>(() => policy.RequireOperation("   "));
    }

    [Fact]
    public void Records_round_trip()
    {
        var request = new PlatformWebRequestEvent("GET /api/items", "GET", "/api/items", 200, TimeSpan.FromMilliseconds(42));
        var provider = new PlatformWebProviderEvent("http.fetch", "github", 502, TimeSpan.FromMilliseconds(120), 2, "resilience.retry");
        Assert.Equal("GET /api/items", request.Operation);
        Assert.Equal("http.fetch", provider.Operation);
        Assert.Equal("github", provider.Provider);
        Assert.Equal(2, provider.Attempt);
    }

    [Fact]
    public void Default_telemetry_uses_logger_and_redactor()
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformWebTelemetry();
        var sp = services.BuildServiceProvider();
        var telemetry = sp.GetRequiredService<IPlatformWebTelemetry>();
        Assert.IsType<DefaultPlatformWebTelemetry>(telemetry);
        Assert.IsType<DefaultPlatformWebTelemetryRedactor>(sp.GetRequiredService<IPlatformWebTelemetryRedactor>());
        telemetry.RecordRequest(new PlatformWebRequestEvent("GET /", "GET", "/", 200, TimeSpan.FromMilliseconds(5)));
        telemetry.RecordProvider(new PlatformWebProviderEvent("http", "test", 200, TimeSpan.FromMilliseconds(1), 1));
    }

    [Fact]
    public void Validator_uses_implemented_interface()
    {
        var options = new PlatformWebTelemetryOptions { ApplicationName = string.Empty };
        Assert.False(PlatformOptionsValidator.Validate(options));
    }
}
