using System.Net;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Platform.Core.Time;
using Platform.Testing.AspNetCore;
using Platform.Testing.TestApplication;

namespace Platform.Testing.Tests.AspNetCore;

/// <summary>
/// Host-configuration coverage: the configuration seam, environment selection,
/// service replacement and decoration, and the named failure boundaries.
/// </summary>
public sealed class PlatformTestWebApplicationFactoryHostConfigurationTests
{
    [Fact]
    public async Task UseSetting_through_the_configuration_seam_is_visible_to_the_application()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .ConfigureWebHost(builder => builder.UseSetting("Platform:Seam", "applied"));

        var configuration = factory.Services.GetRequiredService<IConfiguration>();
        Assert.Equal("applied", configuration["Platform:Seam"]);
    }

    [Fact]
    public async Task In_memory_configuration_outranks_the_applications_own_source()
    {
        // appsettings.json ships PlatformTest:Source = "appsettings".
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .WithConfiguration("PlatformTest:Source", "test-overlay");

        using var client = factory.CreateClient();
        Assert.Equal("test-overlay", await client.GetStringAsync("/configuration/PlatformTest:Source"));
    }

    [Fact]
    public async Task Configuration_callback_runs_after_the_applications_own_registrations()
    {
        // The application registers IGreetingSender twice; the callback can only
        // see and remove those registrations if it runs after them.
        var observed = 0;
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .ConfigureTestServices((services, _) =>
            {
                observed = services.Count(d => d.ServiceType == typeof(IGreetingSender));
            });

        _ = factory.Services;
        Assert.Equal(2, observed);
    }

    [Fact]
    public async Task A_throwing_configuration_callback_propagates_to_the_caller()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .ConfigureWebHost(_ => throw new InvalidOperationException("callback failed"));

        var failure = Assert.Throws<InvalidOperationException>(() => factory.Services);
        Assert.Equal("callback failed", failure.Message);
    }

    [Fact]
    public async Task Configuration_after_the_host_was_created_fails_with_a_named_error()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();
        _ = factory.Services;

        var failure = Assert.Throws<InvalidOperationException>(() => factory.WithEnvironment("Staging"));

        Assert.Contains("before its first", failure.Message, StringComparison.Ordinal);
        Assert.Contains("WithWebHostBuilder", failure.Message, StringComparison.Ordinal);
    }

    [Fact]
    public async Task The_applications_own_static_assets_are_served_from_its_output_directory()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        using var client = factory.CreateClient();
        var response = await client.GetAsync("/asset.txt");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Equal(PlatformTestAssets.Marker, body.Trim());
    }

    // Environment selection.

    [Fact]
    public async Task Explicit_environment_is_applied_with_the_applications_own_configuration()
    {
        // appsettings.Staging.json ships Scenario = "staging".
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .WithEnvironment("Staging");

        var environment = factory.Services.GetRequiredService<IHostEnvironment>();
        Assert.Equal("Staging", environment.EnvironmentName);

        using var client = factory.CreateClient();
        Assert.Equal("Staging", await client.GetStringAsync("/environment"));
        Assert.Equal("staging", await client.GetStringAsync("/configuration/Scenario"));
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("   ")]
    public void Missing_or_empty_environment_name_throws_before_any_host_is_built(string? environmentName)
    {
        var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        // The spec requires ArgumentException; ArgumentNullException is the
        // ArgumentException subtype used for the null case.
        Assert.ThrowsAny<ArgumentException>(() => factory.WithEnvironment(environmentName!));

        // No host was created, so the default environment is untouched.
        Assert.Equal(PlatformTestEnvironments.Testing, factory.EnvironmentName);
    }

    // Service replacement and decoration.

    [Fact]
    public async Task ReplaceService_removes_every_prior_registration()
    {
        var fake = new FakeGreetingSender();
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .ReplaceService<IGreetingSender>(fake);

        Assert.Same(fake, factory.Services.GetRequiredService<IGreetingSender>());

        using var client = factory.CreateClient();
        Assert.Equal("fake", await client.GetStringAsync("/greeting"));
    }

    [Fact]
    public async Task ReplaceService_with_a_type_resolves_that_implementation()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .ReplaceService<IGreetingSender, FakeGreetingSender>();

        Assert.IsType<FakeGreetingSender>(factory.Services.GetRequiredService<IGreetingSender>());
    }

    [Fact]
    public async Task DecorateService_retains_the_original_inside_the_wrapper()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .DecorateService<IAuditedOperation, DecoratingAuditedOperation>(
                original => new DecoratingAuditedOperation(original));

        var resolved = factory.Services.GetRequiredService<IAuditedOperation>();

        var wrapper = Assert.IsType<DecoratingAuditedOperation>(resolved);
        Assert.Equal("application", wrapper.Inner.Describe());
        Assert.Equal("decorated:application", resolved.Describe());
    }

    [Fact]
    public async Task ReplaceService_for_an_unregistered_service_fails_loudly()
    {
        // IUnregisteredContract is never registered by the application under test,
        // so the host build must fail with a named error rather than silently
        // adding a registration the application never had.
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .ReplaceService<IUnregisteredContract, UnregisteredContract>();

        var failure = Assert.Throws<InvalidOperationException>(() => factory.Services);

        Assert.Contains("never registered", failure.Message, StringComparison.Ordinal);
        Assert.Contains(nameof(IUnregisteredContract), failure.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void ReplaceService_rejects_a_null_instance()
    {
        var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        Assert.Throws<ArgumentNullException>(() => factory.ReplaceService<IGreetingSender>(null!));
    }

    // Clock.

    [Fact]
    public async Task Supplied_clock_is_resolved_by_the_application()
    {
        var instant = new DateTimeOffset(2024, 5, 6, 7, 8, 9, TimeSpan.Zero);
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .WithClock(new FixedClock(instant));

        using var client = factory.CreateClient();

        Assert.Equal(instant.ToString("O"), await client.GetStringAsync("/clock"));
        // Deterministic: a second read returns the same instant.
        Assert.Equal(instant.ToString("O"), await client.GetStringAsync("/clock"));
    }

    [Fact]
    public async Task No_clock_supplied_leaves_the_applications_own_clock_in_place()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        Assert.IsType<SystemClock>(factory.Services.GetRequiredService<IClock>());
    }

    [Fact]
    public void Null_clock_is_rejected_at_the_call_site()
    {
        var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        Assert.Throws<ArgumentNullException>(() => factory.WithClock((IClock)null!));
        Assert.Throws<ArgumentNullException>(() => factory.WithClock((TimeProvider)null!));
    }

    // Teardown.

    [Fact]
    public async Task Teardown_actions_run_after_the_host_stops_in_reverse_order()
    {
        var order = new List<string>();
        var hostHadStoppedWhenTeardownRan = false;
        StopProbeHostedService? probe = null;

        var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .OnDispose(() => order.Add("first"))
            .OnDispose(() => order.Add("second"))
            .OnDispose(() =>
            {
                order.Add("third");
                hostHadStoppedWhenTeardownRan = probe!.HasStopped;
                return ValueTask.CompletedTask;
            });

        probe = Assert.Single(
            factory.Services.GetServices<IHostedService>().OfType<StopProbeHostedService>());
        Assert.False(probe.HasStopped, "The host is still running before disposal.");

        await factory.DisposeAsync();

        Assert.Equal(new[] { "third", "second", "first" }, order);
        Assert.True(hostHadStoppedWhenTeardownRan, "Teardown actions must run only after the host has stopped.");
    }

    [Fact]
    public async Task Asynchronous_teardown_is_awaited_before_disposal_returns()
    {
        var completed = false;
        var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .OnDispose(async () =>
            {
                await Task.Delay(20);
                completed = true;
            });

        _ = factory.Services;
        await factory.DisposeAsync();

        Assert.True(completed);
    }

    [Fact]
    public async Task A_failing_teardown_action_does_not_stop_the_others_and_is_surfaced()
    {
        var ran = false;
        var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .OnDispose(() => ran = true)
            .OnDispose(() => throw new InvalidOperationException("cleanup failed"));

        _ = factory.Services;

        var aggregate = await Assert.ThrowsAsync<AggregateException>(async () => await factory.DisposeAsync());

        Assert.True(ran, "The remaining teardown actions must still run.");
        Assert.Contains(aggregate.InnerExceptions, e => e.Message == "cleanup failed");
    }

    [Fact]
    public void Null_teardown_action_is_rejected_at_the_call_site()
    {
        var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        Assert.Throws<ArgumentNullException>(() => factory.OnDispose((Action)null!));
        Assert.Throws<ArgumentNullException>(() => factory.OnDispose((Func<ValueTask>)null!));
    }

    // Temporary directory.

    [Fact]
    public async Task Temporary_directories_are_distinct_and_beneath_the_system_temp_root()
    {
        await using var first = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .UseTemporaryDirectory();
        await using var second = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .UseTemporaryDirectory();

        Assert.NotEqual(first.TemporaryDirectory, second.TemporaryDirectory);
        Assert.StartsWith(Path.GetTempPath(), first.TemporaryDirectory, StringComparison.Ordinal);
        Assert.Empty(Directory.GetFileSystemEntries(first.TemporaryDirectory));
        File.WriteAllText(Path.Combine(first.TemporaryDirectory, "writable.txt"), "ok");
        Assert.True(File.Exists(Path.Combine(first.TemporaryDirectory, "writable.txt")));
    }

    [Fact]
    public async Task Temporary_directory_is_removed_on_disposal()
    {
        var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .UseTemporaryDirectory();
        var path = factory.TemporaryDirectory;

        _ = factory.Services;
        await factory.DisposeAsync();

        Assert.False(Directory.Exists(path));
    }

    [Fact]
    public async Task Temporary_directory_does_not_change_the_content_root_or_working_directory()
    {
        var workingDirectory = Environment.CurrentDirectory;
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .UseTemporaryDirectory();

        var contentRoot = factory.Services.GetRequiredService<IHostEnvironment>().ContentRootPath;

        Assert.NotEqual(factory.TemporaryDirectory, contentRoot);
        Assert.Equal(workingDirectory, Environment.CurrentDirectory);
    }

    [Fact]
    public void Reading_the_temporary_directory_before_requesting_one_fails_with_a_named_error()
    {
        var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        var failure = Assert.Throws<InvalidOperationException>(() => factory.TemporaryDirectory);

        Assert.Contains("UseTemporaryDirectory", failure.Message, StringComparison.Ordinal);
    }

    private sealed class FakeGreetingSender : IGreetingSender
    {
        public string LastGreeting => "fake";
    }

    private sealed class DecoratingAuditedOperation : IAuditedOperation
    {
        public DecoratingAuditedOperation(IAuditedOperation inner) => Inner = inner;

        public IAuditedOperation Inner { get; }

        public string Describe() => $"decorated:{Inner.Describe()}";
    }

    private interface IUnregisteredContract
    {
    }

    private sealed class UnregisteredContract : IUnregisteredContract
    {
    }
}
