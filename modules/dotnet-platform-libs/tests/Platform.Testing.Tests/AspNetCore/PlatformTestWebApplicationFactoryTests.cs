using System.Net;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Platform.Testing.AspNetCore;
using Platform.Testing.TestApplication;

namespace Platform.Testing.Tests.AspNetCore;

/// <summary>
/// Coverage for <see cref="PlatformTestWebApplicationFactory{TEntryPoint}"/>.
/// <para>
/// Every test here boots the real in-repo application
/// (<c>Platform.Testing.TestApplication</c>). The two tests that previously
/// asserted the factory's own empty <c>MapGet("/")</c> application have been
/// rewritten: that behaviour is deliberately removed, so they now assert the
/// contract that replaced it — a request reaches a route declared by the
/// application itself, and the factory adds no route of its own.
/// </para>
/// </summary>
public sealed class PlatformTestWebApplicationFactoryTests
{
    // Rewritten from the old empty-application round-trip: the factory must boot
    // the consumer's application, not an application of its own.
    [Fact]
    public async Task CreateClient_reaches_a_route_declared_by_the_application()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        using var client = factory.CreateClient();
        var response = await client.GetAsync("/");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("Platform.Testing.TestApplication", body, StringComparison.Ordinal);
    }

    // Rewritten from the old "/" -> "ok" assertion: "/" previously returned the
    // factory's own MapGet. The application now owns that route, and the
    // factory's previous synthetic route must be gone.
    [Fact]
    public async Task Factory_adds_no_route_of_its_own_to_the_application()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        using var client = factory.CreateClient();

        // The application declares this route itself.
        var declared = await client.GetAsync("/live");
        Assert.Equal(HttpStatusCode.OK, declared.StatusCode);

        // Nothing else is registered on the application's behalf: an unrouted
        // path is 404 rather than the factory's old catch-all "ok".
        var unrouted = await client.GetAsync("/factory-owned-route");
        Assert.Equal(HttpStatusCode.NotFound, unrouted.StatusCode);
    }

    [Fact]
    public async Task Factory_boots_the_application_with_the_default_testing_environment()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        var environment = factory.Services.GetRequiredService<IHostEnvironment>();
        Assert.Equal(PlatformTestEnvironments.Testing, environment.EnvironmentName);

        using var client = factory.CreateClient();
        var reported = await client.GetStringAsync("/environment");
        Assert.Equal(PlatformTestEnvironments.Testing, reported);
    }

    [Fact]
    public async Task In_memory_configuration_is_visible_to_the_booted_application()
    {
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .WithConfiguration("Platform:Test", "ok");

        var configuration = factory.Services.GetRequiredService<IConfiguration>();
        Assert.Equal("ok", configuration["Platform:Test"]);

        using var client = factory.CreateClient();
        Assert.Equal("ok", await client.GetStringAsync("/configuration/Platform:Test"));
    }

    [Fact]
    public async Task Test_services_callback_can_replace_registered_services()
    {
        var marker = new ServiceMarker();
        await using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>()
            .ConfigureTestServices((services, _) => services.AddSingleton(marker));

        var resolved = factory.Services.GetRequiredService<ServiceMarker>();
        Assert.Same(marker, resolved);
    }

    [Fact]
    public void Static_environment_constant_is_Testing()
    {
        Assert.Equal("Testing", PlatformTestEnvironments.Testing);
    }

    private sealed class ServiceMarker
    {
    }
}
