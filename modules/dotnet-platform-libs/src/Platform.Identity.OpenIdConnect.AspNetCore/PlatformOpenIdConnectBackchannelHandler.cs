using System.Net;
using Microsoft.AspNetCore.Authentication.OpenIdConnect;

namespace Platform.Identity.OpenIdConnect.AspNetCore;

/// <summary>
/// Wraps the provider backchannel so a transport failure or a provider server error is
/// reported as a provider-unavailable outcome instead of an opaque exception.
/// </summary>
internal sealed class PlatformOpenIdConnectBackchannelHandler : DelegatingHandler
{
    public PlatformOpenIdConnectBackchannelHandler(HttpMessageHandler innerHandler)
        : base(innerHandler)
    {
    }

    protected override async Task<HttpResponseMessage> SendAsync(
        HttpRequestMessage request,
        CancellationToken cancellationToken)
    {
        HttpResponseMessage response;
        try
        {
            response = await base.SendAsync(request, cancellationToken);
        }
        catch (HttpRequestException)
        {
            throw new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.ProviderUnavailable,
                "provider_unavailable",
                "The identity provider is unavailable.");
        }
        catch (TaskCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            throw new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.ProviderUnavailable,
                "provider_timeout",
                "The identity provider is unavailable.");
        }

        if (response.StatusCode >= HttpStatusCode.InternalServerError)
        {
            response.Dispose();
            throw new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.ProviderUnavailable,
                "provider_unavailable",
                "The identity provider is unavailable.");
        }

        return response;
    }
}
