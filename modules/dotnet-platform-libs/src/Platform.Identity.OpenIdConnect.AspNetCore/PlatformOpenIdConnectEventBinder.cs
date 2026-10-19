using System.Security.Claims;
using System.Text.RegularExpressions;
using Microsoft.AspNetCore.Authentication;
using Microsoft.AspNetCore.Authentication.OpenIdConnect;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Microsoft.IdentityModel.Protocols.OpenIdConnect;

namespace Platform.Identity.OpenIdConnect.AspNetCore;

/// <summary>
/// The adapter's OIDC event pipeline. It runs the adapter's own validation before any
/// application handler, so an application can observe events but cannot skip the
/// adapter's guarantees.
/// </summary>
internal sealed class PlatformOpenIdConnectEventBinder
{
    private const string ErrorCodeItem = "platform.oidc.error_code";

    private static readonly Regex ErrorCodePattern = new(
        @"Error:\s*(?<code>[a-zA-Z_]{1,64})",
        RegexOptions.Compiled | RegexOptions.CultureInvariant);

    private static readonly string[] ProviderUnavailableCodes =
    {
        "temporarily_unavailable",
        "server_error",
    };

    private readonly PlatformOpenIdConnectOptions _options;

    public PlatformOpenIdConnectEventBinder(PlatformOpenIdConnectOptions options)
    {
        _options = options;
    }

    public void Bind(OpenIdConnectEvents events)
    {
        var applicationMessageReceived = events.OnMessageReceived;
        var applicationTokenValidated = events.OnTokenValidated;
        var applicationCodeReceived = events.OnAuthorizationCodeReceived;
        var applicationRemoteFailure = events.OnRemoteFailure;
        var applicationAccessDenied = events.OnAccessDenied;

        events.OnMessageReceived = async context =>
        {
            var error = context.ProtocolMessage.Error;
            if (!string.IsNullOrWhiteSpace(error) && context.Properties is not null)
            {
                context.Properties.Items[ErrorCodeItem] = error;
            }

            if (applicationMessageReceived is not null)
            {
                await applicationMessageReceived(context);
            }
        };

        events.OnTokenValidated = async context =>
        {
            await ValidateTokenAsync(context);
            if (applicationTokenValidated is not null)
            {
                await applicationTokenValidated(context);
            }
        };

        events.OnAuthorizationCodeReceived = async context =>
        {
            await ConsumeStateAsync(context);
            if (applicationCodeReceived is not null)
            {
                await applicationCodeReceived(context);
            }
        };

        events.OnRemoteFailure = async context =>
        {
            if (applicationRemoteFailure is not null)
            {
                await applicationRemoteFailure(context);
            }

            throw Classify(context.Failure, context.Properties);
        };

        events.OnAccessDenied = async context =>
        {
            if (applicationAccessDenied is not null)
            {
                await applicationAccessDenied(context);
            }

            throw new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.AccessDenied,
                "access_denied",
                Messages.AccessDenied);
        };
    }

    private static async Task ConsumeStateAsync(AuthorizationCodeReceivedContext context)
    {
        var state = context.Request.Query["state"].ToString();
        if (string.IsNullOrEmpty(state))
        {
            throw new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.InvalidResponse,
                "missing_state",
                Messages.InvalidResponse);
        }

        var guard = context.HttpContext.RequestServices.GetRequiredService<IPlatformOpenIdConnectStateReplayGuard>();
        if (!await guard.TryConsumeAsync(state, context.HttpContext.RequestAborted))
        {
            var logger = context.HttpContext.RequestServices
                .GetRequiredService<ILoggerFactory>()
                .CreateLogger<PlatformOpenIdConnectEventBinder>();
            PlatformOpenIdConnectLog.ReplayedState(logger);
            throw new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.InvalidResponse,
                "replayed_state",
                Messages.InvalidResponse);
        }
    }

    private async Task ValidateTokenAsync(TokenValidatedContext context)
    {
        var principal = context.Principal;
        if (principal is null)
        {
            throw new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.InvalidResponse,
                "missing_principal",
                Messages.InvalidResponse);
        }

        var issuer = principal.FindFirst(PlatformOpenIdConnectPrincipal.IssuerClaimType)?.Value;
        if (!string.Equals(issuer, _options.Issuer.TrimEnd('/'), StringComparison.Ordinal))
        {
            throw new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.InvalidResponse,
                "issuer_mismatch",
                Messages.InvalidResponse);
        }

        if (!HasAudience(principal, _options.ClientId))
        {
            throw new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.InvalidResponse,
                "audience_mismatch",
                Messages.InvalidResponse);
        }

        var subject = principal.FindFirst(PlatformOpenIdConnectPrincipal.SubjectClaimType)?.Value;
        if (string.IsNullOrWhiteSpace(subject))
        {
            throw new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.InvalidResponse,
                "missing_subject",
                Messages.InvalidResponse);
        }

        if (principal.FindFirst("nonce") is null)
        {
            throw new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.InvalidResponse,
                "missing_nonce",
                Messages.InvalidResponse);
        }

        RemoveUnmappedRoleClaims(principal);

        // The ASP.NET Core OpenID Connect handler removes the protocol claims such as
        // 'iss' from the signed-in principal, so the validated identity is stored in the
        // ticket properties where the application can read it back.
        if (context.Properties is not null)
        {
            context.Properties.Items[PlatformOpenIdConnectPrincipal.IssuerPropertyKey] = issuer ?? string.Empty;
            context.Properties.Items[PlatformOpenIdConnectPrincipal.SubjectPropertyKey] = subject;
        }

        if (_options.IdentityResolution != PlatformOpenIdConnectIdentityResolutionMode.Required)
        {
            return;
        }

        var resolver = context.HttpContext.RequestServices.GetService<IPlatformOpenIdConnectIdentityResolver>();
        if (resolver is null)
        {
            throw new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.ApplicationMapping,
                "resolver_missing",
                Messages.ApplicationMapping);
        }

        var identity = new PlatformOpenIdConnectExternalIdentity(issuer!, subject);
        var resolution = await resolver.ResolveAsync(
            identity,
            principal,
            context.HttpContext.RequestAborted);

        switch (resolution.Status)
        {
            case PlatformOpenIdConnectIdentityStatus.Authenticated:
                return;
            case PlatformOpenIdConnectIdentityStatus.Conflict:
                throw new PlatformOpenIdConnectException(
                    PlatformOpenIdConnectFailureKind.ApplicationMapping,
                    "duplicate_link",
                    Messages.ApplicationMapping);
            default:
                throw new PlatformOpenIdConnectException(
                    PlatformOpenIdConnectFailureKind.ApplicationMapping,
                    resolution.Reason,
                    Messages.ApplicationMapping);
        }
    }

    private void RemoveUnmappedRoleClaims(ClaimsPrincipal principal)
    {
        if (!string.IsNullOrWhiteSpace(_options.RoleClaimType))
        {
            return;
        }

        foreach (var identity in principal.Identities)
        {
            foreach (var claim in identity.Claims.Where(IsRoleClaim).ToArray())
            {
                identity.TryRemoveClaim(claim);
            }
        }
    }

    private static bool IsRoleClaim(Claim claim) =>
        string.Equals(claim.Type, ClaimTypes.Role, StringComparison.Ordinal)
        || string.Equals(claim.Type, "role", StringComparison.Ordinal)
        || string.Equals(claim.Type, "roles", StringComparison.Ordinal);

    private static bool HasAudience(ClaimsPrincipal principal, string clientId)
    {
        foreach (var claim in principal.FindAll("aud"))
        {
            if (string.Equals(claim.Value, clientId, StringComparison.Ordinal))
            {
                return true;
            }
        }

        return false;
    }

    private static PlatformOpenIdConnectException Classify(Exception? failure, AuthenticationProperties? properties)
    {
        if (failure is PlatformOpenIdConnectException typed)
        {
            return typed;
        }

        if (failure is HttpRequestException or TimeoutException or TaskCanceledException)
        {
            return new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.ProviderUnavailable,
                "provider_unavailable",
                Messages.ProviderUnavailable);
        }

        var providerError = ReadErrorCode(properties) ?? ReadErrorCode(failure?.Message);
        if (providerError is "access_denied")
        {
            return new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.AccessDenied,
                "access_denied",
                Messages.AccessDenied);
        }

        if (providerError is not null && ProviderUnavailableCodes.Contains(providerError, StringComparer.Ordinal))
        {
            return new PlatformOpenIdConnectException(
                PlatformOpenIdConnectFailureKind.ProviderUnavailable,
                "provider_unavailable",
                Messages.ProviderUnavailable);
        }

        return new PlatformOpenIdConnectException(
            PlatformOpenIdConnectFailureKind.InvalidResponse,
            "invalid_response",
            Messages.InvalidResponse);
    }

    private static string? ReadErrorCode(AuthenticationProperties? properties)
    {
        if (properties is null)
        {
            return null;
        }

        return properties.Items.TryGetValue(ErrorCodeItem, out var value) ? value : null;
    }

    private static string? ReadErrorCode(string? message)
    {
        if (string.IsNullOrEmpty(message))
        {
            return null;
        }

        var match = ErrorCodePattern.Match(message);
        if (!match.Success)
        {
            return null;
        }

        var code = match.Groups["code"].Value;
        return code.Length is > 0 and <= 64 ? code : null;
    }

    private static class Messages
    {
        internal const string InvalidResponse = "The OpenID Connect response was rejected.";
        internal const string AccessDenied = "The identity provider refused the sign-in request.";
        internal const string ProviderUnavailable = "The identity provider is unavailable.";
        internal const string ApplicationMapping = "The application refused the external identity.";
    }
}

internal static partial class PlatformOpenIdConnectLog
{
    [LoggerMessage(
        Level = LogLevel.Warning,
        Message = "Rejected a replayed OpenID Connect callback state.")]
    public static partial void ReplayedState(ILogger logger);
}
