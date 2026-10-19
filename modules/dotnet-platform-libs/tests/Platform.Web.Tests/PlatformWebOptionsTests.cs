using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Web;
using Platform.Web.DependencyInjection;

namespace Platform.Web.Tests;

public sealed class PlatformWebOptionsTests
{
    [Fact]
    public void Defaults_are_safe_and_runtime_paths_are_explicit()
    {
        var options = new PlatformWebOptions();

        Assert.Equal("X-Correlation-Id", options.CorrelationHeader);
        Assert.Equal("/live", options.LivePath);
        Assert.Equal("/ready", options.ReadinessPath);
        Assert.True(options.EnableSecurityHeaders);
        Assert.Equal(1_048_576, options.MaxRequestBodyBytes);
        Assert.Equal(TimeSpan.FromSeconds(30), options.RequestTimeout);
    }

    [Fact]
    public void Registration_exposes_replaceable_runtime_contracts()
    {
        var services = new ServiceCollection();

        services.AddPlatformWeb();

        var provider = services.BuildServiceProvider();
        Assert.IsType<DefaultPlatformRedactor>(provider.GetRequiredService<IPlatformRedactor>());
        Assert.IsType<DefaultPlatformConfigurationValidator>(provider.GetRequiredService<IPlatformConfigurationValidator>());
        Assert.IsType<EmptyProviderStatusSource>(provider.GetRequiredService<IProviderStatusSource>());
        Assert.NotNull(provider.GetRequiredService<IOptions<PlatformWebOptions>>().Value);
    }

    [Fact]
    public void Host_registration_before_platform_registration_wins_for_replaceable_services()
    {
        var services = new ServiceCollection();
        services.AddSingleton<IPlatformRedactor, HostRedactor>();

        services.AddPlatformWeb();

        Assert.IsType<HostRedactor>(services.BuildServiceProvider().GetRequiredService<IPlatformRedactor>());
    }

    [Fact]
    public void Invalid_options_are_rejected_when_options_are_resolved()
    {
        var services = new ServiceCollection();
        services.AddPlatformWeb(options => options.ReadinessPath = "ready");
        using var provider = services.BuildServiceProvider();

        Assert.Throws<OptionsValidationException>(() => provider.GetRequiredService<IOptions<PlatformWebOptions>>().Value);
    }

    private sealed class HostRedactor : IPlatformRedactor
    {
        public string Redact(string? value) => value ?? string.Empty;
    }
}
