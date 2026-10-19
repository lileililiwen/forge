using System.Security.Cryptography;
using System.Text;
using System.Net;
using Platform.Billing;
using Platform.Billing.Contracts.Events;
using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Plans;
using Platform.Billing.Contracts.Subscriptions;
using Platform.Billing.LemonSqueezy;
using Platform.Billing.Stripe;
using Platform.Billing.Contracts.Providers;

namespace Platform.Billing.ProviderAdapters.Tests;

public sealed class ProviderAdapterTests
{
    [Fact]
    public async Task Stripe_accepts_a_fresh_signature_and_normalizes_subscription_event()
    {
        var payload = "{\"id\":\"evt_123\",\"type\":\"customer.subscription.deleted\",\"created\":1767225600,\"data\":{\"object\":{\"id\":\"sub_123\",\"metadata\":{\"subject\":\"user-1\"}}}}";
        var timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var signature = Sign("stripe-secret", $"{timestamp}.{payload}");
        var provider = new StripeBillingProvider(new HttpClient(), new StripeOptions { WebhookSecret = "stripe-secret" });

        var result = await provider.VerifyAndNormalizeWebhookAsync(payload, new Dictionary<string, string> { ["Stripe-Signature"] = $"t={timestamp},v1={signature}" });

        Assert.True(result.Succeeded);
        Assert.Equal("evt_123", result.Event!.Id.Value);
        Assert.Equal("subscription.canceled", result.Event.Type);
        Assert.Equal(SubjectKey.Create("user-1"), result.Event.Subject);
    }

    [Fact]
    public async Task Lemon_squeezy_rejects_an_invalid_signature_without_normalizing()
    {
        var provider = new LemonSqueezyBillingProvider(new HttpClient(), new LemonSqueezyOptions { WebhookSecret = "lemon-secret" });

        var result = await provider.VerifyAndNormalizeWebhookAsync("{\"meta\":{\"event_name\":\"subscription_created\"}}", new Dictionary<string, string> { ["X-Signature"] = "bad" });

        Assert.False(result.Succeeded);
        Assert.Null(result.Event);
        Assert.Equal("invalid_webhook_signature", result.Error);
    }

    [Fact]
    public void Provider_failure_classifier_does_not_include_secret_or_response_body()
    {
        var failure = ProviderFailureClassifier.Classify(new HttpRequestException("request failed: secret-token"), "checkout");

        Assert.Equal(ProviderFailureKind.Transient, failure.Kind);
        Assert.Equal("checkout", failure.Operation);
        Assert.DoesNotContain("secret-token", failure.SafeMessage);
    }

    [Fact]
    public async Task Stripe_checkout_uses_the_application_owned_provider_plan_reference()
    {
        HttpRequestMessage? sent = null;
        string? form = null;
        var handler = new RecordingHandler(request => { sent = request; form = request.Content!.ReadAsStringAsync().GetAwaiter().GetResult(); return new HttpResponseMessage(HttpStatusCode.OK) { Content = new StringContent("{\"id\":\"cs_1\",\"url\":\"https://checkout.test/1\"}") }; });
        var catalog = new PlanCatalog().Add(new BillingPlan(PlanId.Create("pro"), "Pro"), new ProviderPlanReference(ProviderName.Create("stripe"), "price_app_owned"));
        var provider = new StripeBillingProvider(new HttpClient(handler) { BaseAddress = new Uri("https://stripe.test/") }, new StripeOptions { ApiKey = "test-key", PlanCatalog = catalog });

        await provider.CreateCheckoutAsync(new CheckoutRequest(new BillingCustomer(SubjectKey.Create("user-1")), PlanId.Create("pro"), new Uri("https://app.test/success"), new Uri("https://app.test/cancel")));

        Assert.Equal("Bearer", sent!.Headers.Authorization!.Scheme);
        Assert.Contains("price_app_owned", form);
        Assert.DoesNotContain("price_pro", form);
    }

    [Fact]
    public async Task Lemon_squeezy_accepts_raw_body_signature_and_normalizes_subject()
    {
        var payload = "{\"meta\":{\"event_name\":\"subscription_cancelled\"},\"data\":{\"id\":\"ls-1\",\"attributes\":{\"updated_at\":\"2026-01-01T00:00:00Z\"}},\"custom_data\":{\"subject\":\"user-2\"}}";
        var provider = new LemonSqueezyBillingProvider(new HttpClient(), new LemonSqueezyOptions { WebhookSecret = "lemon-secret" });

        var result = await provider.VerifyAndNormalizeWebhookAsync(payload, new Dictionary<string, string> { ["X-Signature"] = Sign("lemon-secret", payload) });

        Assert.True(result.Succeeded);
        Assert.Equal("subscription.canceled", result.Event!.Type);
        Assert.Equal(SubjectKey.Create("user-2"), result.Event.Subject);
    }

    [Fact]
    public async Task Orchestrator_deduplicates_a_replayed_adapter_event()
    {
        var now = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        var eventValue = new ProviderEvent(ProviderEventId.Create("evt-1"), ProviderName.Create("stripe"), "subscription.canceled", now, "{}", SubjectKey.Create("user-1"));
        var store = new InMemoryProcessedEventStore();
        var orchestrator = new BillingEventOrchestrator(store, new RecordingProjector(), new Platform.Core.Time.FixedClock(now));

        Assert.Equal(BillingEventDecision.Applied, await orchestrator.ProcessAsync(new ProviderEventEnvelope(eventValue)));
        Assert.Equal(BillingEventDecision.Duplicate, await orchestrator.ProcessAsync(new ProviderEventEnvelope(eventValue)));
    }

    private static string Sign(string secret, string value)
    {
        using var hmac = new HMACSHA256(Encoding.UTF8.GetBytes(secret));
        return Convert.ToHexString(hmac.ComputeHash(Encoding.UTF8.GetBytes(value))).ToLowerInvariant();
    }

    private sealed class RecordingHandler(Func<HttpRequestMessage, HttpResponseMessage> responder) : HttpMessageHandler
    {
        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken) => Task.FromResult(responder(request));
    }

    private sealed class InMemoryProcessedEventStore : IProcessedEventStore
    {
        private readonly HashSet<(ProviderName, ProviderEventId)> _seen = [];
        public Task<ProcessedEventDecision> MarkProcessedAsync(ProcessedEvent processedEvent, CancellationToken cancellationToken = default) => Task.FromResult(_seen.Add((processedEvent.Provider, processedEvent.EventId)) ? ProcessedEventDecision.FirstDelivery : ProcessedEventDecision.Duplicate);
    }

    private sealed class RecordingProjector : IBillingEventProjector
    {
        public ValueTask<bool> ApplyAsync(ProviderEvent providerEvent, CancellationToken cancellationToken = default) => ValueTask.FromResult(true);
    }
}
