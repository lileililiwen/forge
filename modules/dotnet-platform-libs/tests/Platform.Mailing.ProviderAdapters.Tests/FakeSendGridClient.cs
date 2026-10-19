using System.Net;
using System.Net.Http;
using System.Net.Http.Headers;
using SendGrid;
using SendGrid.Helpers.Mail;

namespace Platform.Mailing.ProviderAdapters.Tests;

/// <summary>
/// Deterministic in-memory <see cref="ISendGridClient"/> double that records
/// every dispatched <see cref="SendGridMessage"/> and returns a scripted
/// <see cref="Response"/>.
/// </summary>
internal sealed class FakeSendGridClient : ISendGridClient
{
    public FakeSendGridClient()
        : this(new Response(HttpStatusCode.Accepted, null, null))
    {
    }

    public FakeSendGridClient(Response response)
    {
        Response = response;
    }

    public Response Response { get; set; }

    public Func<CancellationToken, Response>? OnSend { get; set; }

    public List<SendGridMessage> SentMessages { get; } = new();

    public int CallCount { get; private set; }

    public string? ApiKey => "fake-key";

    public string? UrlPath { get => "v3/mail/send"; set => throw new NotSupportedException(); }

    public HttpClient? HttpClient => null;

    public Task<Response> SendEmailAsync(SendGridMessage msg, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        CallCount++;
        SentMessages.Add(msg);
        return Task.FromResult(OnSend is null ? Response : OnSend(cancellationToken));
    }

    public Task<Response> SendEmailAsync(string from, string to, string subject, string plainTextContent, string htmlContent, CancellationToken cancellationToken = default)
        => throw new NotSupportedException();

    public Task<Response> SendAsync(string requestBody, Dictionary<string, string>? headers = null, string? url = null, CancellationToken cancellationToken = default)
        => throw new NotSupportedException();

    public Task<Response> SendAsync(SendGridMessage msg, CancellationToken cancellationToken = default)
        => throw new NotSupportedException();

    public Task<Response> MakeRequest(HttpRequestMessage request, CancellationToken cancellationToken = default)
        => throw new NotSupportedException();

    public Task<Response> RequestAsync(BaseClient.Method method, string requestBody, string? url = null, string? additionalHeaders = null, CancellationToken cancellationToken = default)
        => throw new NotSupportedException();

    public string Version { get => throw new NotSupportedException(); set => throw new NotSupportedException(); }

    public string MediaType { get => throw new NotSupportedException(); set => throw new NotSupportedException(); }

    public AuthenticationHeaderValue AddAuthorization(KeyValuePair<string, string> authorization)
        => throw new NotSupportedException();
}
