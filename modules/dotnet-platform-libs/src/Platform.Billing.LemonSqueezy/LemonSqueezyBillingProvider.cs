using System.Globalization;
using System.Net.Http.Headers;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Platform.Billing.Contracts.Events;
using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Plans;
using Platform.Billing.Contracts.Providers;
using Platform.Billing.Contracts.Subscriptions;
using Platform.Core.Time;

namespace Platform.Billing.LemonSqueezy;

/// <summary>HTTP Lemon Squeezy adapter using provider-neutral billing contracts.</summary>
public sealed class LemonSqueezyBillingProvider : IBillingProvider
{
    private static readonly ProviderName Provider = ProviderName.Create("lemon-squeezy");
    private readonly HttpClient _http; private readonly LemonSqueezyOptions _options; private readonly IClock _clock;
    /// <summary>Initializes the adapter.</summary>
    public LemonSqueezyBillingProvider(HttpClient httpClient, LemonSqueezyOptions options, IClock? clock = null)
    { _http = httpClient ?? throw new ArgumentNullException(nameof(httpClient)); _options = options ?? throw new ArgumentNullException(nameof(options)); _clock = clock ?? new SystemClock(); _http.BaseAddress ??= _options.ApiBaseAddress; _http.Timeout = _options.Timeout; }
    /// <inheritdoc />
    public ProviderName Name => Provider;
    /// <inheritdoc />
    public async ValueTask<CheckoutSession> CreateCheckoutAsync(CheckoutRequest request, CancellationToken cancellationToken = default)
    { ArgumentNullException.ThrowIfNull(request); var variant = _options.PlanCatalog.ReferenceFor(request.Plan, Provider).ProviderValue; var body = JsonSerializer.Serialize(new { data = new { type = "checkouts", attributes = new { checkout_data = new { custom = new { subject = request.Customer.Subject.Value } }, product_options = new { enabled_variants = new[] { variant } }, checkout_options = new { embed = false }, checkout_redirect_url = request.SuccessUrl.ToString() }, relationships = new { store = new { data = new { type = "stores", id = request.Tenant ?? "0" } } } } }); using var response = await SendAsync(HttpMethod.Post, "checkouts", body, cancellationToken); using var doc = await JsonDocument.ParseAsync(await response.Content.ReadAsStreamAsync(cancellationToken), cancellationToken: cancellationToken); var attributes = doc.RootElement.GetProperty("data").GetProperty("attributes"); return new CheckoutSession(Name, doc.RootElement.GetProperty("data").GetProperty("id").GetString() ?? throw new FormatException(), new Uri(attributes.GetProperty("url").GetString() ?? throw new FormatException()), _clock.UtcNow.AddMinutes(30)); }
    /// <inheritdoc />
    public ValueTask<PortalSession> CreatePortalAsync(BillingCustomer customer, Uri returnUrl, CancellationToken cancellationToken = default) => throw new NotSupportedException("Lemon Squeezy does not expose a customer portal session.");
    /// <inheritdoc />
    public ValueTask<Subscription?> GetSubscriptionAsync(SubscriptionLookup lookup, CancellationToken cancellationToken = default) => GetCoreAsync(lookup, cancellationToken);
    private async ValueTask<Subscription?> GetCoreAsync(SubscriptionLookup lookup, CancellationToken token) { ArgumentNullException.ThrowIfNull(lookup); if (string.IsNullOrWhiteSpace(lookup.ProviderSubscriptionId)) return null; using var response = await SendAsync(HttpMethod.Get, "subscriptions/" + Uri.EscapeDataString(lookup.ProviderSubscriptionId), null, token); using var doc = await JsonDocument.ParseAsync(await response.Content.ReadAsStreamAsync(token), cancellationToken: token); var root = doc.RootElement.GetProperty("data"); var a = root.GetProperty("attributes"); return new Subscription(lookup.Subject, PlanId.Create(a.GetProperty("variant_id").ToString()), MapStatus(a.GetProperty("status").GetString()), Name, root.GetProperty("id").GetString() ?? throw new FormatException(), DateTimeOffset.Parse(a.GetProperty("created_at").GetString() ?? throw new FormatException(), CultureInfo.InvariantCulture), DateTimeOffset.Parse(a.GetProperty("renews_at").GetString() ?? throw new FormatException(), CultureInfo.InvariantCulture)); }
    /// <inheritdoc />
    public ValueTask<Subscription?> CancelSubscriptionAsync(CancellationRequest request, CancellationToken cancellationToken = default) => throw new NotSupportedException("Lemon Squeezy cancellation is application-mediated through subscription update policy.");
    /// <inheritdoc />
    public ValueTask<WebhookNormalizationResult> VerifyAndNormalizeWebhookAsync(string payload, IReadOnlyDictionary<string, string> headers, CancellationToken cancellationToken = default)
    { ArgumentException.ThrowIfNullOrEmpty(payload); ArgumentNullException.ThrowIfNull(headers); if (string.IsNullOrWhiteSpace(_options.WebhookSecret) || !headers.TryGetValue("X-Signature", out var supplied) || !Verify(payload, supplied)) return ValueTask.FromResult(new WebhookNormalizationResult(null, "invalid_webhook_signature")); try { using var doc = JsonDocument.Parse(payload); var root = doc.RootElement; var meta = root.GetProperty("meta"); var data = root.GetProperty("data"); var type = meta.GetProperty("event_name").GetString() ?? throw new FormatException(); var normalized = type switch { "subscription_cancelled" or "subscription_expired" => "subscription.canceled", "subscription_created" => "subscription.created", "subscription_updated" => "subscription.updated", _ => type }; var subject = FindString(root, "subject"); var occurred = DateTimeOffset.TryParse(FindString(root, "updated_at"), out var parsed) ? parsed : _clock.UtcNow; return ValueTask.FromResult(new WebhookNormalizationResult(new ProviderEvent(ProviderEventId.Create(data.GetProperty("id").GetString() ?? throw new FormatException()), Name, normalized, occurred, payload, subject is null ? null : SubjectKey.Create(subject)))); } catch (Exception ex) when (ex is KeyNotFoundException or FormatException or InvalidOperationException or JsonException) { return ValueTask.FromResult(new WebhookNormalizationResult(null, "malformed_webhook")); } }
    /// <inheritdoc />
    public ValueTask<BillingProviderStatus> GetStatusAsync(CancellationToken cancellationToken = default) => ValueTask.FromResult(new BillingProviderStatus(Name, !string.IsNullOrWhiteSpace(_options.ApiKey), string.IsNullOrWhiteSpace(_options.ApiKey) ? "not_configured" : null));
    private bool Verify(string payload, string supplied) { using var hmac = new HMACSHA256(Encoding.UTF8.GetBytes(_options.WebhookSecret!)); var expected = Convert.ToHexString(hmac.ComputeHash(Encoding.UTF8.GetBytes(payload))).ToLowerInvariant(); return CryptographicOperations.FixedTimeEquals(Encoding.UTF8.GetBytes(expected), Encoding.UTF8.GetBytes(supplied)); }
    private async Task<HttpResponseMessage> SendAsync(HttpMethod method, string path, string? body, CancellationToken token) { if (string.IsNullOrWhiteSpace(_options.ApiKey)) throw new InvalidOperationException("Lemon Squeezy API is not configured."); using var request = new HttpRequestMessage(method, path); request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", _options.ApiKey); request.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("application/vnd.api+json")); if (body is not null) { request.Content = new StringContent(body, Encoding.UTF8, "application/vnd.api+json"); } var response = await _http.SendAsync(request, token); response.EnsureSuccessStatusCode(); return response; }
    private static string? FindString(JsonElement root, string name) { if (root.ValueKind == JsonValueKind.Object) foreach (var p in root.EnumerateObject()) { if (p.NameEquals(name) && p.Value.ValueKind == JsonValueKind.String) return p.Value.GetString(); var found = FindString(p.Value, name); if (found is not null) return found; } else if (root.ValueKind == JsonValueKind.Array) foreach (var item in root.EnumerateArray()) { var found = FindString(item, name); if (found is not null) return found; } return null; }
    private static SubscriptionStatus MapStatus(string? value) => value switch { "active" => SubscriptionStatus.Active, "past_due" => SubscriptionStatus.PastDue, "cancelled" or "expired" => SubscriptionStatus.Canceled, _ => SubscriptionStatus.Unknown };
}
