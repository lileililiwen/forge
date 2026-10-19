using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Platform.Observability.DependencyInjection;

namespace Platform.Observability.Tests;

public sealed class RegistrationTests
{
    [Fact]
    public void AddPlatformObservability_registers_required_services()
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformObservability(o => o.ApplicationName = "test-app");

        using var sp = services.BuildServiceProvider();
        var recorder = sp.GetService<Platform.Observability.Diagnostics.IPlatformActivityRecorder>();
        var providerRecorder = sp.GetService<Platform.Observability.Diagnostics.IPlatformObservabilityProviderRecorder>();
        var status = sp.GetService<Platform.Observability.Diagnostics.IPlatformObservabilityProviderStatusSource>();
        var redactor = sp.GetService<Platform.Observability.Redaction.IPlatformObservabilityRedactor>();
        var generator = sp.GetService<Platform.Observability.Hosting.ICorrelationIdGenerator>();
        var accessor = sp.GetService<Platform.Observability.Hosting.IPlatformCorrelationAccessor>();
        var lifetime = sp.GetServices<IHostedService>().OfType<Platform.Observability.Hosting.PlatformObservabilityHostLifetime>().Single();

        Assert.NotNull(recorder);
        Assert.NotNull(providerRecorder);
        Assert.NotNull(status);
        Assert.NotNull(redactor);
        Assert.NotNull(generator);
        Assert.NotNull(accessor);
        Assert.NotNull(lifetime);
    }

    [Fact]
    public void Default_redactor_can_be_replaced()
    {
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddSingleton<Platform.Observability.Redaction.IPlatformObservabilityRedactor, EchoRedactor>();
        services.AddPlatformObservability();

        using var sp = services.BuildServiceProvider();
        var redactor = sp.GetRequiredService<Platform.Observability.Redaction.IPlatformObservabilityRedactor>();
        Assert.IsType<EchoRedactor>(redactor);
    }

    [Fact]
    public void Default_status_source_reports_all_sources()
    {
        var source = new Platform.Observability.Diagnostics.DefaultPlatformObservabilityProviderStatusSource();
        var statuses = source.GetStatuses();
        Assert.Equal(4, statuses.Count);
        Assert.All(statuses, s => Assert.True(s.Available));
        Assert.All(statuses, s => Assert.DoesNotContain(s.Name, ":"));
    }

    [Fact]
    public void Default_correlation_generator_emits_distinct_values()
    {
        var generator = new Platform.Observability.Hosting.DefaultCorrelationIdGenerator();
        var first = generator.Generate();
        var second = generator.Generate();
        Assert.NotEqual(first, second);
        Assert.Equal(32, first.Length);
    }

    [Fact]
    public void Default_correlation_generator_normalizes_supplied_value()
    {
        var generator = new Platform.Observability.Hosting.DefaultCorrelationIdGenerator();
        Assert.Equal("abc", generator.Normalize("  abc  ", 16));
        var trimmed = generator.Normalize(new string('a', 32), 8);
        Assert.Equal(8, trimmed.Length);
    }

    private sealed class EchoRedactor : Platform.Observability.Redaction.IPlatformObservabilityRedactor
    {
        public string Redact(string? value) => value ?? string.Empty;
    }
}
