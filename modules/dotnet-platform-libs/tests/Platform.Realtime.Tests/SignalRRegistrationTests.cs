using Microsoft.AspNetCore.SignalR;
using Microsoft.Extensions.DependencyInjection;
using Platform.Realtime.AspNetCore.SignalR;
using Platform.Realtime.AspNetCore.Sse;
using Platform.Realtime.Authorization;
using Platform.Realtime.Status;

namespace Platform.Realtime.Tests;

public class SignalRRegistrationTests
{
    [Fact]
    public async Task Sse_only_registration_exposes_sse_status_without_signalr()
    {
        var services = new ServiceCollection();
        services.AddPlatformRealtimeAspNetCore();

        using var provider = services.BuildServiceProvider();
        var status = provider.GetRequiredService<IRealtimeProviderStatus>();

        var snapshot = await status.GetStatusAsync();

        Assert.Single(snapshot);
        Assert.Equal("SSE", snapshot[0].Transport);
        Assert.True(snapshot[0].Available);
        Assert.False(snapshot[0].BackplaneEnabled);
    }

    [Fact]
    public async Task SignalR_registration_reports_signalr_without_backplane()
    {
        var services = new ServiceCollection();
        services.AddPlatformRealtimeSignalR();

        using var provider = services.BuildServiceProvider();
        var status = provider.GetRequiredService<IRealtimeProviderStatus>();

        var snapshot = await status.GetStatusAsync();
        var signalR = Assert.Single(snapshot.Where(s => s.Transport == "SignalR"));

        Assert.True(signalR.Available);
        Assert.False(signalR.BackplaneEnabled);
        Assert.NotNull(provider.GetService<IRealtimeConnectionAuthorizer>());
        Assert.NotNull(provider.GetService<IHubFilter>());
    }

    [Fact]
    public async Task SignalR_with_backplane_seam_reports_backplane_enabled()
    {
        var services = new ServiceCollection();
        services.AddPlatformRealtimeSignalR(options => options.ConfigureBackplane = _ => { });

        using var provider = services.BuildServiceProvider();
        var status = provider.GetRequiredService<IRealtimeProviderStatus>();

        var snapshot = await status.GetStatusAsync();
        var signalR = Assert.Single(snapshot.Where(s => s.Transport == "SignalR"));

        Assert.True(signalR.BackplaneEnabled);
        Assert.Equal("distributed backplane", signalR.Detail);
    }

    [Fact]
    public async Task SignalR_registration_fails_closed_by_default_authorizer()
    {
        var services = new ServiceCollection();
        services.AddPlatformRealtimeSignalR();

        using var provider = services.BuildServiceProvider();
        var authorizer = provider.GetRequiredService<IRealtimeConnectionAuthorizer>();

        var result = await authorizer.AuthorizeAsync(new RealtimeConnectionRequest(), CancellationToken.None);

        Assert.False(result.Allowed);
    }
}
