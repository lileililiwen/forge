using System.Net;
using System.Security.Claims;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.Logging;
using Microsoft.IdentityModel.JsonWebTokens;
using Microsoft.IdentityModel.Tokens;

namespace Platform.Identity.OpenIdConnect.AspNetCore.Tests;

/// <summary>
/// A local OpenID Connect issuer with its own signing key. It serves a discovery
/// document, a JWKS, an authorization endpoint and a token endpoint, so the adapter is
/// exercised against a real signed token instead of a stub.
/// </summary>
internal sealed class LocalOidcTestIssuer : IAsyncDisposable
{
    private readonly RSA _key = RSA.Create(2048);
    private readonly string _keyId = "local-test-key";
    private readonly Dictionary<string, AuthorizationCode> _codes = new(StringComparer.Ordinal);
    private readonly WebApplication _app;

    public LocalOidcTestIssuer()
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Testing" });
        builder.WebHost.UseTestServer();
        builder.Logging.ClearProviders();
        _app = builder.Build();
        _app.MapGet("/.well-known/openid-configuration", () => Results.Json(DiscoveryDocument));
        _app.MapGet("/jwks", () => Results.Json(JsonWebKeySet()));
        _app.MapGet("/authorize", (Func<HttpContext, IResult>)Authorize);
        _app.MapPost("/token", (Func<HttpContext, Task<IResult>>)Token);
        _app.StartAsync().GetAwaiter().GetResult();
    }

    /// <summary>The issuer identifier used in the discovery document and the tokens.</summary>
    public string Issuer { get; } = "https://issuer.test";

    /// <summary>The client identifier the test application registers.</summary>
    public string ClientId { get; } = "test-client";

    /// <summary>The subject the issued tokens carry.</summary>
    public string Subject { get; set; } = "user-1";

    /// <summary>When set, the token endpoint answers with a server error.</summary>
    public bool TokenEndpointServerError { get; set; }

    /// <summary>When set, the issued token carries a different issuer.</summary>
    public string? IssuerOverride { get; set; }

    /// <summary>When set, the issued token carries a different audience.</summary>
    public string? AudienceOverride { get; set; }

    /// <summary>When set, the issued token carries a different nonce.</summary>
    public string? NonceOverride { get; set; }

    /// <summary>When set, the issued token is already expired.</summary>
    public bool IssueExpiredToken { get; set; }

    /// <summary>When set, the token is signed with a key that is not published in the JWKS.</summary>
    public bool SignWithUnpublishedKey { get; set; }

    /// <summary>Extra claims added to the issued token.</summary>
    public IDictionary<string, string> ExtraClaims { get; } = new Dictionary<string, string>(StringComparer.Ordinal);

    /// <summary>The number of authorization codes the issuer handed out.</summary>
    public int IssuedCodeCount { get; private set; }

    /// <summary>The number of token requests the issuer answered.</summary>
    public int TokenRequestCount { get; private set; }

    /// <summary>Creates the callback URL for a challenge the application produced.</summary>
    public string CreateCallbackUrl(Uri challengeUri)
    {
        var query = Microsoft.AspNetCore.WebUtilities.QueryHelpers.ParseQuery(challengeUri.Query);
        var state = query["state"].ToString();
        var nonce = query["nonce"].ToString();
        var challenge = query["code_challenge"].ToString();
        var redirectUri = query["redirect_uri"].ToString();
        var clientId = query["client_id"].ToString();

        if (string.IsNullOrEmpty(state) || string.IsNullOrEmpty(redirectUri))
        {
            throw new InvalidOperationException("The challenge did not carry a state and a redirect URI.");
        }

        if (!string.Equals(clientId, ClientId, StringComparison.Ordinal))
        {
            throw new InvalidOperationException($"The challenge used an unexpected client '{clientId}'.");
        }

        var code = Guid.NewGuid().ToString("N");
        _codes[code] = new AuthorizationCode(nonce, challenge, redirectUri, clientId);
        IssuedCodeCount++;

        return $"{redirectUri}?code={Uri.EscapeDataString(code)}&state={Uri.EscapeDataString(state)}";
    }

    /// <summary>Creates a callback URL that carries a provider error instead of a code.</summary>
    public string CreateErrorCallbackUrl(Uri challengeUri, string error)
    {
        var query = Microsoft.AspNetCore.WebUtilities.QueryHelpers.ParseQuery(challengeUri.Query);
        var state = query["state"].ToString();
        var redirectUri = query["redirect_uri"].ToString();
        return $"{redirectUri}?error={Uri.EscapeDataString(error)}&state={Uri.EscapeDataString(state)}";
    }

    /// <summary>Creates a backchannel handler that talks to this issuer.</summary>
    public HttpMessageHandler CreateHandler() => _app.GetTestServer().CreateHandler();

    public async ValueTask DisposeAsync()
    {
        await _app.StopAsync();
        await _app.DisposeAsync();
        _key.Dispose();
    }

    private object DiscoveryDocument => new
    {
        issuer = Issuer,
        authorization_endpoint = $"{Issuer}/authorize",
        token_endpoint = $"{Issuer}/token",
        jwks_uri = $"{Issuer}/jwks",
        response_types_supported = new[] { "code" },
        grant_types_supported = new[] { "authorization_code" },
        subject_types_supported = new[] { "public" },
        id_token_signing_alg_values_supported = new[] { "RS256" },
        code_challenge_methods_supported = new[] { "S256" },
        token_endpoint_auth_methods_supported = new[] { "none", "client_secret_post" },
    };

    private object JsonWebKeySet()
    {
        var parameters = _key.ExportParameters(false);
        return new
        {
            keys = new[]
            {
                new
                {
                    kty = "RSA",
                    use = "sig",
                    alg = "RS256",
                    kid = _keyId,
                    n = Base64UrlEncoder.Encode(parameters.Modulus!),
                    e = Base64UrlEncoder.Encode(parameters.Exponent!),
                },
            },
        };
    }

    private IResult Authorize(HttpContext context)
    {
        var state = context.Request.Query["state"].ToString();
        var redirectUri = context.Request.Query["redirect_uri"].ToString();
        if (string.IsNullOrEmpty(state) || string.IsNullOrEmpty(redirectUri))
        {
            return Results.BadRequest();
        }

        var code = Guid.NewGuid().ToString("N");
        _codes[code] = new AuthorizationCode(
            context.Request.Query["nonce"].ToString(),
            context.Request.Query["code_challenge"].ToString(),
            redirectUri,
            context.Request.Query["client_id"].ToString());
        IssuedCodeCount++;

        return Results.Redirect($"{redirectUri}?code={Uri.EscapeDataString(code)}&state={Uri.EscapeDataString(state)}");
    }

    private async Task<IResult> Token(HttpContext context)
    {
        TokenRequestCount++;

        if (TokenEndpointServerError)
        {
            return Results.Json(
                new { error = "temporarily_unavailable" },
                statusCode: StatusCodes.Status503ServiceUnavailable);
        }

        var form = await context.Request.ReadFormAsync();
        var code = form["code"].ToString();
        var codeVerifier = form["code_verifier"].ToString();
        var redirectUri = form["redirect_uri"].ToString();
        var clientId = form["client_id"].ToString();

        if (!_codes.TryGetValue(code, out var authorization))
        {
            return Results.Json(new { error = "invalid_grant" }, statusCode: StatusCodes.Status400BadRequest);
        }

        if (!string.Equals(authorization.ClientId, clientId, StringComparison.Ordinal)
            || !string.Equals(authorization.RedirectUri, redirectUri, StringComparison.Ordinal))
        {
            return Results.Json(new { error = "invalid_grant" }, statusCode: StatusCodes.Status400BadRequest);
        }

        if (!string.IsNullOrEmpty(authorization.CodeChallenge))
        {
            var expected = Base64UrlEncoder.Encode(SHA256.HashData(Encoding.ASCII.GetBytes(codeVerifier)));
            if (!string.Equals(expected, authorization.CodeChallenge, StringComparison.Ordinal))
            {
                return Results.Json(new { error = "invalid_grant" }, statusCode: StatusCodes.Status400BadRequest);
            }
        }

        var idToken = CreateIdToken(authorization.Nonce);
        return Results.Json(new
        {
            access_token = "test-access-token",
            token_type = "Bearer",
            expires_in = 3600,
            id_token = idToken,
        });
    }

    private string CreateIdToken(string nonce)
    {
        var signingKey = SignWithUnpublishedKey ? RSA.Create(2048) : _key;
        var descriptor = new SecurityTokenDescriptor
        {
            Issuer = IssuerOverride ?? Issuer,
            Audience = AudienceOverride ?? ClientId,
            IssuedAt = DateTime.UtcNow.AddMinutes(-1),
            NotBefore = DateTime.UtcNow.AddMinutes(-1),
            Expires = IssueExpiredToken ? DateTime.UtcNow.AddMinutes(-5) : DateTime.UtcNow.AddMinutes(10),
            Claims = new Dictionary<string, object>(StringComparer.Ordinal)
            {
                ["nonce"] = NonceOverride ?? nonce,
                ["sub"] = Subject,
            },
            SigningCredentials = new SigningCredentials(new RsaSecurityKey(signingKey) { KeyId = _keyId }, SecurityAlgorithms.RsaSha256),
        };

        foreach (var claim in ExtraClaims)
        {
            descriptor.Claims[claim.Key] = claim.Value;
        }

        return new JsonWebTokenHandler().CreateToken(descriptor);
    }

    private sealed record AuthorizationCode(
        string Nonce,
        string CodeChallenge,
        string RedirectUri,
        string ClientId);
}
