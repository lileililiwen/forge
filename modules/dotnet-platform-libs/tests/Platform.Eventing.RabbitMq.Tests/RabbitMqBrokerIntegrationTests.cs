using System.Diagnostics;
using System.Net;
using System.Net.Sockets;
using Microsoft.Extensions.DependencyInjection;
using Platform.Eventing.Contracts;
using Platform.Eventing.RabbitMq.DependencyInjection;

namespace Platform.Eventing.RabbitMq.Tests;

/// <summary>
/// Broker failure classification that needs no external dependency: an
/// unreachable loopback port must surface as a safe transient failure.
/// </summary>
public class UnreachableBrokerTests
{
    [Fact]
    public async Task Unreachable_broker_returns_a_transient_failure()
    {
        var port = GetUnusedPort();
        var services = new ServiceCollection();
        services.AddLogging();
        services.AddPlatformRabbitMqEventing(o =>
        {
            o.Exchange = "platform.events";
            o.Port = port;
            o.ConnectTimeout = TimeSpan.FromSeconds(1);
        });
        services.AddSingleton<IRabbitMqEventTopology>(new RabbitMqEventTopology().Map("order.created", new RabbitMqEventBinding("order.created.v1")));
        await using var provider = services.BuildServiceProvider();
        await using var publisher = (RabbitMqDurableEventPublisher)provider.GetRequiredService<IDurableEventPublisher>();

        var exception = await Assert.ThrowsAsync<RabbitMqPublishException>(
            () => publisher.PublishAsync(TestOptions.Envelope()));

        Assert.False(exception.Failure.Permanent);
        Assert.Equal(RabbitMqEventingProviderState.Unavailable, publisher.Status.State);
        Assert.DoesNotContain("guest", exception.Message, StringComparison.OrdinalIgnoreCase);
    }

    private static int GetUnusedPort()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        try
        {
            return ((IPEndPoint)listener.LocalEndpoint).Port;
        }
        finally
        {
            listener.Stop();
        }
    }
}

/// <summary>
/// Optional RabbitMQ broker integration test. The test is Docker-gated: when
/// a working Docker daemon is unavailable (or the RabbitMQ image cannot be
/// pulled), the test returns without asserting so the suite stays
/// deterministic on machines without Docker.
/// </summary>
public class RabbitMqBrokerIntegrationTests
{
    private const string Image = "rabbitmq:3.13-alpine";

    [Fact]
    public async Task Broker_confirms_a_published_envelope_when_docker_is_available()
    {
        if (!TryStartRabbitMqContainer(out var containerId, out var port))
        {
            return;
        }

        try
        {
            var services = new ServiceCollection();
            services.AddLogging();
            services.AddPlatformRabbitMqEventing(o =>
            {
                o.Exchange = "platform.events";
                o.Port = port;
                o.UserName = "platform";
                o.Password = "platform-tests";
                o.DeclareExchange = true;
                o.ConnectTimeout = TimeSpan.FromSeconds(30);
            });
            services.AddSingleton<IRabbitMqEventTopology>(new RabbitMqEventTopology()
                .Map("order.created", new RabbitMqEventBinding("order.created.v1"))
                .Map("order.paid", new RabbitMqEventBinding("order.paid.v1")));
            await using var provider = services.BuildServiceProvider();
            await using var publisher = (RabbitMqDurableEventPublisher)provider.GetRequiredService<IDurableEventPublisher>();

            await publisher.PublishAsync(TestOptions.Envelope());
            await publisher.PublishAsync(TestOptions.Envelope(payloadType: "order.paid", payloadJson: "{\"orderId\":\"2\"}"));

            Assert.Equal(RabbitMqEventingProviderState.Healthy, publisher.Status.State);
        }
        finally
        {
            Docker("rm", "-f", containerId);
        }
    }

    private static bool TryStartRabbitMqContainer(out string containerId, out int port)
    {
        containerId = string.Empty;
        port = 0;

        if (Docker("info") is null)
        {
            return false;
        }

        var started = Docker("run", "-d", "--rm", "-P", "-e", "RABBITMQ_DEFAULT_USER=platform", "-e", "RABBITMQ_DEFAULT_PASS=platform-tests", Image);
        if (started is null)
        {
            return false;
        }

        containerId = started.Trim();
        for (var attempt = 0; attempt < 90; attempt++)
        {
            var mapping = Docker("port", containerId, "5672");
            if (!string.IsNullOrWhiteSpace(mapping) && int.TryParse(mapping.Trim().Split(':')[^1], out port)
                && Docker("logs", containerId)?.Contains("Server startup complete", StringComparison.Ordinal) == true)
            {
                return true;
            }

            Thread.Sleep(TimeSpan.FromSeconds(1));
        }

        Docker("rm", "-f", containerId);
        return false;
    }

    private static string? Docker(params string[] arguments)
    {
        try
        {
            using var process = Process.Start(
                new ProcessStartInfo("docker", string.Join(' ', arguments))
                {
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    UseShellExecute = false,
                });
            if (process is null)
            {
                return null;
            }

            var output = process.StandardOutput.ReadToEnd();
            process.WaitForExit(TimeSpan.FromSeconds(60));
            return process.ExitCode == 0 ? output : null;
        }
        catch
        {
            return null;
        }
    }
}
