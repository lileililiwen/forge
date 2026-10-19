using System.Text;
using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Inbound;

namespace Platform.Webhooks.Tests.Inbound;

public sealed class WebhookInboundProcessorTests
{
    [Fact]
    public async Task Valid_signature_and_handler_completes()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var store = new InMemoryWebhookInboxStore();
        var provider = new WebhookProviderId("test");
        var verifier = new HmacWebhookSignatureVerifier(provider, "X-Signature", "X-Timestamp", new WebhookOptions());
        var secretStore = new ConfigurationWebhookSecretResolver();
        var secret = Encoding.UTF8.GetBytes("super-secret");
        secretStore.Add(provider, "default", secret);
        var processor = new WebhookInboundProcessor(store, verifier, secretStore, Options.Create(new WebhookOptions()), clock, NullLogger<WebhookInboundProcessor>.Instance, "worker-1", TimeSpan.FromMinutes(1));
        var (request, _) = BuildSignedRequest(provider, secret, "evt_1");

        var result = await processor.ProcessAsync(request, (req, _, _) => new ValueTask<WebhookInboundHandlerResult>(WebhookInboundHandlerResult.Succeeded));

        Assert.Equal(WebhookInboundOutcome.Accepted, result.Outcome);
        var stored = await store.GetAsync(request.Provider.Value + "\u001f" + request.EventId);
        Assert.Equal(WebhookInboxState.Completed, stored!.State);
    }

    [Fact]
    public async Task Duplicate_event_returns_duplicate_and_does_not_invoke_handler()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var store = new InMemoryWebhookInboxStore();
        var provider = new WebhookProviderId("test");
        var verifier = new HmacWebhookSignatureVerifier(provider, "X-Signature", "X-Timestamp", new WebhookOptions());
        var secretStore = new ConfigurationWebhookSecretResolver();
        var secret = Encoding.UTF8.GetBytes("super-secret");
        secretStore.Add(provider, "default", secret);
        var processor = new WebhookInboundProcessor(store, verifier, secretStore, Options.Create(new WebhookOptions()), clock, NullLogger<WebhookInboundProcessor>.Instance, "worker-1", TimeSpan.FromMinutes(1));
        var (request, _) = BuildSignedRequest(provider, secret, "evt_dup");
        await processor.ProcessAsync(request, (req, _, _) => new ValueTask<WebhookInboundHandlerResult>(WebhookInboundHandlerResult.Succeeded));
        var invoked = false;

        var result = await processor.ProcessAsync(request, (req, _, _) => { invoked = true; return new ValueTask<WebhookInboundHandlerResult>(WebhookInboundHandlerResult.Succeeded); });

        Assert.Equal(WebhookInboundOutcome.Duplicate, result.Outcome);
        Assert.False(invoked);
    }

    [Fact]
    public async Task Invalid_signature_is_rejected_safely()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var store = new InMemoryWebhookInboxStore();
        var provider = new WebhookProviderId("test");
        var verifier = new HmacWebhookSignatureVerifier(provider, "X-Signature", "X-Timestamp", new WebhookOptions());
        var secretStore = new ConfigurationWebhookSecretResolver();
        secretStore.Add(provider, "default", Encoding.UTF8.GetBytes("super-secret"));
        var processor = new WebhookInboundProcessor(store, verifier, secretStore, Options.Create(new WebhookOptions()), clock, NullLogger<WebhookInboundProcessor>.Instance, "worker-1", TimeSpan.FromMinutes(1));
        var request = new WebhookVerificationRequest(provider, "evt_2", new Dictionary<string, string>
        {
            ["X-Signature"] = "deadbeef",
            ["X-Timestamp"] = DateTimeOffset.UtcNow.ToUnixTimeSeconds().ToString(),
        }, Encoding.UTF8.GetBytes("{}"));

        var result = await processor.ProcessAsync(request, (req, _, _) => new ValueTask<WebhookInboundHandlerResult>(WebhookInboundHandlerResult.Succeeded));

        Assert.Equal(WebhookInboundOutcome.Rejected, result.Outcome);
        Assert.NotNull(result.Failure);
        Assert.Equal(0, store.Count);
    }

    [Fact]
    public async Task Unknown_provider_secret_rejected()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var store = new InMemoryWebhookInboxStore();
        var provider = new WebhookProviderId("test");
        var verifier = new HmacWebhookSignatureVerifier(provider, "X-Signature", "X-Timestamp", new WebhookOptions());
        var secretStore = new ConfigurationWebhookSecretResolver();
        var processor = new WebhookInboundProcessor(store, verifier, secretStore, Options.Create(new WebhookOptions()), clock, NullLogger<WebhookInboundProcessor>.Instance, "worker-1", TimeSpan.FromMinutes(1));
        var (request, _) = BuildSignedRequest(provider, Encoding.UTF8.GetBytes("super-secret"), "evt_3");

        var result = await processor.ProcessAsync(request, (req, _, _) => new ValueTask<WebhookInboundHandlerResult>(WebhookInboundHandlerResult.Succeeded));

        Assert.Equal(WebhookInboundOutcome.Rejected, result.Outcome);
        Assert.Equal("webhook.secret_not_found", result.Failure!.Code);
    }

    [Fact]
    public async Task Transient_handler_failure_schedules_retry()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var store = new InMemoryWebhookInboxStore();
        var provider = new WebhookProviderId("test");
        var verifier = new HmacWebhookSignatureVerifier(provider, "X-Signature", "X-Timestamp", new WebhookOptions());
        var secretStore = new ConfigurationWebhookSecretResolver();
        var secret = Encoding.UTF8.GetBytes("super-secret");
        secretStore.Add(provider, "default", secret);
        var processor = new WebhookInboundProcessor(store, verifier, secretStore, Options.Create(new WebhookOptions { DefaultRetryBaseDelay = TimeSpan.FromSeconds(2), DefaultRetryMaxDelay = TimeSpan.FromMinutes(5) }), clock, NullLogger<WebhookInboundProcessor>.Instance, "worker-1", TimeSpan.FromMinutes(1));
        var (request, _) = BuildSignedRequest(provider, secret, "evt_4");

        var result = await processor.ProcessAsync(request, (req, _, _) => new ValueTask<WebhookInboundHandlerResult>(WebhookInboundHandlerResult.TransientFailure));

        Assert.Equal(WebhookInboundOutcome.Busy, result.Outcome);
        var stored = await store.GetAsync(request.Provider.Value + "\u001f" + request.EventId);
        Assert.Equal(WebhookInboxState.Pending, stored!.State);
        Assert.NotNull(stored.NextAttemptAt);
    }

    private static (WebhookVerificationRequest Request, byte[] Secret) BuildSignedRequest(WebhookProviderId provider, byte[] secret, string eventId)
    {
        var timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds().ToString();
        var body = Encoding.UTF8.GetBytes("{\"id\":\"" + eventId + "\"}");
        var prefix = Encoding.UTF8.GetBytes(timestamp + ".");
        var combined = new byte[prefix.Length + body.Length];
        Buffer.BlockCopy(prefix, 0, combined, 0, prefix.Length);
        body.CopyTo(combined, prefix.Length);
        using var hmac = new System.Security.Cryptography.HMACSHA256(secret);
        var digest = Convert.ToHexString(hmac.ComputeHash(combined)).ToLowerInvariant();
        var request = new WebhookVerificationRequest(provider, eventId, new Dictionary<string, string>
        {
            ["X-Signature"] = digest,
            ["X-Timestamp"] = timestamp,
        }, body);
        return (request, secret);
    }
}
