using System.Net.Http;
using System.Net.Http.Headers;
using System.Text;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Outbound;

namespace Platform.Webhooks.AspNetCore.Outbound;

/// <summary>Default <see cref="IWebhookHttpSender"/> that uses an application-owned <see cref="HttpClient"/>.</summary>
public sealed class HttpClientWebhookSender : IWebhookHttpSender
{
    private readonly HttpClient _http;

    /// <summary>Creates a sender that uses the supplied <see cref="HttpClient"/>.</summary>
    public HttpClientWebhookSender(HttpClient httpClient)
    {
        _http = httpClient ?? throw new ArgumentNullException(nameof(httpClient));
    }

    /// <inheritdoc />
    public async ValueTask<WebhookHttpSendResult> SendAsync(WebhookHttpSendRequest request, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        if (request.Timeout <= TimeSpan.Zero) throw new ArgumentOutOfRangeException(nameof(request));
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(request.Timeout);
        using var message = new HttpRequestMessage(HttpMethod.Post, request.Target);
        message.Content = new StringContent(request.Body ?? string.Empty, Encoding.UTF8, "application/json");
        if (!string.IsNullOrEmpty(request.SignatureHeaderName)) message.Headers.TryAddWithoutValidation(request.SignatureHeaderName, request.SignatureHeaderValue);
        if (!string.IsNullOrEmpty(request.EventIdHeaderName)) message.Headers.TryAddWithoutValidation(request.EventIdHeaderName, request.EventIdHeaderValue);
        if (!string.IsNullOrEmpty(request.EventTypeHeaderName)) message.Headers.TryAddWithoutValidation(request.EventTypeHeaderName, request.EventTypeHeaderValue);
        try
        {
            using var response = await _http.SendAsync(message, HttpCompletionOption.ResponseHeadersRead, timeout.Token).ConfigureAwait(false);
            var status = (int)response.StatusCode;
            if (status >= 200 && status < 300) return WebhookHttpSendResult.Success(status);
            var failure = new WebhookFailure(ClassifyResponse(status), "The delivery target returned a non-success status.", IsRetryableStatus(status));
            return WebhookHttpSendResult.ResponseFailure(status, failure);
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            return WebhookHttpSendResult.TransportFailure(new WebhookFailure("webhook.timeout", "The delivery request timed out.", true));
        }
        catch (HttpRequestException ex)
        {
            return WebhookHttpSendResult.TransportFailure(new WebhookFailure("webhook.transport_error", ex.Message ?? "The delivery request failed.", true));
        }
    }

    private static string ClassifyResponse(int status) => status switch
    {
        408 => "webhook.timeout_response",
        429 => "webhook.rate_limited",
        >= 500 => "webhook.target_unavailable",
        _ => "webhook.rejected_by_target"
    };

    private static bool IsRetryableStatus(int status) => status is >= 500 or 408 or 429;
}
