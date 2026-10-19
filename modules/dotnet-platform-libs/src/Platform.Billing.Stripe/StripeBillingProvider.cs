using System.Globalization;
using System.Net.Http.Headers;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Options;
using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Events;
using Platform.Billing.Contracts.Plans;
using Platform.Billing.Contracts.Providers;
using Platform.Billing.Contracts.Subscriptions;
using Platform.Core.Time;

namespace Platform.Billing.Stripe;

/// <summary>HTTP Stripe adapter using provider-neutral billing contracts.</summary>
public sealed class StripeBillingProvider : IBillingProvider
{
    private static readonly ProviderName Provider = ProviderName.Create("stripe");
    private readonly HttpClient _http;
    private readonly StripeOptions _options;
    private readonly IClock _clock;

    /// <summary>Initializes the adapter with an HTTP client and options.</summary>
    public StripeBillingProvider(HttpClient httpClient, StripeOptions options, IClock? clock = null)
    {
        _http = httpClient ?? throw new ArgumentNullException(nameof(httpClient));
        _options = options ?? throw new ArgumentNullException(nameof(options));
        _clock = clock ?? new SystemClock();
        _http.BaseAddress ??= _options.ApiBaseAddress;
        _http.Timeout = _options.Timeout;
    }
    /// <inheritdoc />
    public ProviderName Name => Provider;

    /// <inheritdoc />
    public async ValueTask<CheckoutSession> CreateCheckoutAsync(CheckoutRequest request, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        var price = _options.PlanCatalog.ReferenceFor(request.Plan, Provider).ProviderValue;
        using var response = await SendAsync(HttpMethod.Post, "v1/checkout/sessions", [new("mode", "subscription"), new("line_items[0][price]", price), new("line_items[0][quantity]", "1"), new("success_url", request.SuccessUrl.ToString()), new("cancel_url", request.CancelUrl.ToString()), new("client_reference_id", request.Customer.Subject.Value)], cancellationToken);
        using var document = await ReadJsonAsync(response, cancellationToken);
        var root = document.RootElement;
        return new CheckoutSession(Name, Required(root, "id"), new Uri(Required(root, "url")), root.TryGetProperty("expires_at", out var expires) ? DateTimeOffset.FromUnixTimeSeconds(expires.GetInt64()) : _clock.UtcNow.AddMinutes(30));
    }

    /// <inheritdoc />
    public async ValueTask<PortalSession> CreatePortalAsync(BillingCustomer customer, Uri returnUrl, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(customer); ArgumentNullException.ThrowIfNull(returnUrl);
        using var response = await SendAsync(HttpMethod.Post, "v1/billing_portal/sessions", [new("customer", customer.Subject.Value), new("return_url", returnUrl.ToString())], cancellationToken);
        using var document = await ReadJsonAsync(response, cancellationToken);
        return new PortalSession(Name, Required(document.RootElement, "id"), new Uri(Required(document.RootElement, "url")), _clock.UtcNow.AddMinutes(30));
    }

    /// <inheritdoc />
    public ValueTask<Subscription?> GetSubscriptionAsync(SubscriptionLookup lookup, CancellationToken cancellationToken = default) =>
        GetSubscriptionCoreAsync(lookup, cancellationToken);

    private async ValueTask<Subscription?> GetSubscriptionCoreAsync(SubscriptionLookup lookup, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(lookup);
        if (string.IsNullOrWhiteSpace(lookup.ProviderSubscriptionId)) return null;
        using var response = await SendAsync(HttpMethod.Get, "v1/subscriptions/" + Uri.EscapeDataString(lookup.ProviderSubscriptionId), null, cancellationToken);
        using var document = await ReadJsonAsync(response, cancellationToken);
        return NormalizeSubscription(document.RootElement, lookup.Subject);
    }

    /// <inheritdoc />
    public ValueTask<Subscription?> CancelSubscriptionAsync(CancellationRequest request, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        return ValueTask.FromResult<Subscription?>(null);
    }

    /// <inheritdoc />
    public ValueTask<WebhookNormalizationResult> VerifyAndNormalizeWebhookAsync(string payload, IReadOnlyDictionary<string, string> headers, CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrEmpty(payload); ArgumentNullException.ThrowIfNull(headers);
        if (!VerifySignature(payload, headers.TryGetValue("Stripe-Signature", out var signature) ? signature : null)) return ValueTask.FromResult(new WebhookNormalizationResult(null, "invalid_webhook_signature"));
        try
        {
            using var document = JsonDocument.Parse(payload); var root = document.RootElement; var type = root.GetProperty("type").GetString() ?? "unknown";
            var normalizedType = type switch { "customer.subscription.deleted" => "subscription.canceled", "customer.subscription.created" => "subscription.created", "customer.subscription.updated" => "subscription.updated", _ => type };
            var id = ProviderEventId.Create(root.GetProperty("id").GetString() ?? throw new FormatException());
            var occurred = root.TryGetProperty("created", out var created) ? DateTimeOffset.FromUnixTimeSeconds(created.GetInt64()) : _clock.UtcNow;
            var subject = FindSubject(root);
            return ValueTask.FromResult(new WebhookNormalizationResult(new ProviderEvent(id, Name, normalizedType, occurred, payload, subject)));
        }
        catch (Exception ex) when (ex is KeyNotFoundException or FormatException or InvalidOperationException or JsonException)
        { return ValueTask.FromResult(new WebhookNormalizationResult(null, "malformed_webhook")); }
    }

    /// <inheritdoc />
    public ValueTask<BillingProviderStatus> GetStatusAsync(CancellationToken cancellationToken = default) => ValueTask.FromResult(new BillingProviderStatus(Name, !string.IsNullOrWhiteSpace(_options.ApiKey), string.IsNullOrWhiteSpace(_options.ApiKey) ? "not_configured" : null));

    private bool VerifySignature(string payload, string? header)
    {
        if (string.IsNullOrWhiteSpace(_options.WebhookSecret) || string.IsNullOrWhiteSpace(header)) return false;
        var parts = header.Split(',', StringSplitOptions.TrimEntries); var timestamp = parts.FirstOrDefault(x => x.StartsWith("t=", StringComparison.Ordinal))?[2..]; var value = parts.FirstOrDefault(x => x.StartsWith("v1=", StringComparison.Ordinal))?[3..];
        if (!long.TryParse(timestamp, out var seconds) || string.IsNullOrWhiteSpace(value) || Math.Abs((_clock.UtcNow - DateTimeOffset.FromUnixTimeSeconds(seconds)).TotalSeconds) > _options.WebhookTolerance.TotalSeconds) return false;
        using var hmac = new HMACSHA256(Encoding.UTF8.GetBytes(_options.WebhookSecret)); var expected = Convert.ToHexString(hmac.ComputeHash(Encoding.UTF8.GetBytes(timestamp + "." + payload))).ToLowerInvariant();
        return CryptographicOperations.FixedTimeEquals(Encoding.UTF8.GetBytes(expected), Encoding.UTF8.GetBytes(value));
    }
    private async Task<HttpResponseMessage> SendAsync(HttpMethod method, string path, IEnumerable<KeyValuePair<string, string>>? form, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(_options.ApiKey)) throw new InvalidOperationException("Stripe API is not configured.");
        for (var attempt = 0; ; attempt++)
        {
            using var request = new HttpRequestMessage(method, path); request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", _options.ApiKey);
            if (form is not null) request.Content = new FormUrlEncodedContent(form);
            try { var response = await _http.SendAsync(request, cancellationToken); if ((int)response.StatusCode >= 500 && attempt < _options.MaxRetries) { response.Dispose(); await Task.Delay(TimeSpan.FromMilliseconds(50 * (attempt + 1)), cancellationToken); continue; } response.EnsureSuccessStatusCode(); return response; }
            catch (HttpRequestException) when (attempt < _options.MaxRetries) { await Task.Delay(TimeSpan.FromMilliseconds(50 * (attempt + 1)), cancellationToken); }
        }
    }
    private static async Task<JsonDocument> ReadJsonAsync(HttpResponseMessage response, CancellationToken token) => JsonDocument.Parse(await response.Content.ReadAsStringAsync(token));
    private static string Required(JsonElement root, string property) => root.GetProperty(property).GetString() ?? throw new FormatException();
    private static SubjectKey? FindSubject(JsonElement root) => root.ToString().Contains("\"subject\"", StringComparison.Ordinal) ? SubjectKey.Create(FindString(root, "subject") ?? throw new FormatException()) : null;
    private static string? FindString(JsonElement root, string property) { if (root.ValueKind == JsonValueKind.Object) foreach (var p in root.EnumerateObject()) { if (p.NameEquals(property) && p.Value.ValueKind == JsonValueKind.String) return p.Value.GetString(); var found = FindString(p.Value, property); if (found is not null) return found; } else if (root.ValueKind == JsonValueKind.Array) foreach (var item in root.EnumerateArray()) { var found = FindString(item, property); if (found is not null) return found; } return null; }
    private static Subscription NormalizeSubscription(JsonElement root, SubjectKey subject) => new(subject, PlanId.Create(FindString(root, "plan") ?? "unknown"), MapStatus(FindString(root, "status")), Provider, Required(root, "id"), DateTimeOffset.FromUnixTimeSeconds(root.GetProperty("current_period_start").GetInt64()), DateTimeOffset.FromUnixTimeSeconds(root.GetProperty("current_period_end").GetInt64()));
    private static SubscriptionStatus MapStatus(string? value) => value switch { "active" => SubscriptionStatus.Active, "past_due" => SubscriptionStatus.PastDue, "canceled" => SubscriptionStatus.Canceled, "paused" => SubscriptionStatus.Suspended, _ => SubscriptionStatus.Unknown };
}
