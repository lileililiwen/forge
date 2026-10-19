using System.Text;
using Microsoft.AspNetCore.Http;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Inbound;

namespace Platform.Webhooks.AspNetCore.Inbound;

/// <summary>Helpers that convert an <see cref="HttpRequest"/> into the provider-neutral verification request.</summary>
public static class WebhookHttpRequestReader
{
    /// <summary>Reads the raw body and headers into a verification request, returning <c>null</c> when the body is too large.</summary>
    public static async Task<WebhookVerificationRequest?> TryReadAsync(
        HttpRequest request,
        WebhookProviderId provider,
        string eventId,
        long maximumBodyBytes,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        if (string.IsNullOrWhiteSpace(eventId)) throw new ArgumentException("Event identifiers are required.", nameof(eventId));
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(maximumBodyBytes);
        request.EnableBuffering();
        var capacity = (int)Math.Min(maximumBodyBytes, int.MaxValue);
        using var buffer = new MemoryStream(capacity);
        var copyBuffer = new byte[8192];
        long total = 0;
        int read;
        while ((read = await request.Body.ReadAsync(copyBuffer.AsMemory(0, copyBuffer.Length), cancellationToken).ConfigureAwait(false)) > 0)
        {
            total += read;
            if (total > maximumBodyBytes) return null;
            buffer.Write(copyBuffer, 0, read);
        }
        request.Body.Position = 0;
        var headers = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (var header in request.Headers)
        {
            headers[header.Key] = header.Value.ToString();
        }
        return new WebhookVerificationRequest(provider, eventId, headers, buffer.ToArray());
    }

    /// <summary>Reads the raw body using UTF-8 decoding. Caller is responsible for size enforcement.</summary>
    public static string ReadBodyAsString(WebhookVerificationRequest request)
    {
        ArgumentNullException.ThrowIfNull(request);
        return Encoding.UTF8.GetString(request.Body.Span);
    }
}
