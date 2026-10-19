using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Platform.Identity.Contracts;

namespace Platform.Identity.AspNetCore;

/// <summary>
/// Minimal-API helpers for the platform identity lifecycle. The platform never owns
/// the consumer's authentication scheme; these helpers merely route lifecycle
/// operations to the registered services.
/// </summary>
public static class IdentityLifecycleEndpointRouteBuilderExtensions
{
    private static readonly System.Text.Json.JsonSerializerOptions JsonOptionsInstance = new()
    {
        PropertyNameCaseInsensitive = true,
    };
    /// <summary>Maps the platform identity refresh-token rotation endpoint at the supplied path.</summary>
    public static RouteHandlerBuilder MapPlatformRefreshTokenRotation(this IEndpointRouteBuilder endpoints, string path = "/platform/identity/refresh")
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("A path is required.", nameof(path));
        return endpoints.MapPost(path, async (HttpRequest request, IIdentityLifecycleCoordinator coordinator, CancellationToken cancellationToken) =>
        {
            var handle = ExtractHandle(request);
            if (string.IsNullOrWhiteSpace(handle))
                return Results.Problem(statusCode: StatusCodes.Status400BadRequest, title: "Invalid request", detail: "A refresh handle is required.");
            var result = await coordinator.RefreshTokens.RotateAsync(handle, cancellationToken).ConfigureAwait(false);
            return ToHttpResult(result, handle => Results.Ok(new { handle }));
        });
    }

    /// <summary>Maps the platform identity refresh-token revocation endpoint at the supplied path.</summary>
    public static RouteHandlerBuilder MapPlatformRefreshTokenRevocation(this IEndpointRouteBuilder endpoints, string path = "/platform/identity/refresh/revoke")
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("A path is required.", nameof(path));
        return endpoints.MapPost(path, async (HttpRequest request, IIdentityLifecycleCoordinator coordinator, CancellationToken cancellationToken) =>
        {
            var handle = ExtractHandle(request);
            if (string.IsNullOrWhiteSpace(handle))
                return Results.Problem(statusCode: StatusCodes.Status400BadRequest, title: "Invalid request", detail: "A refresh handle is required.");
            var outcome = await coordinator.RefreshTokens.RevokeAsync(handle, cancellationToken).ConfigureAwait(false);
            return outcome switch
            {
                IdentityLifecycleOutcome.Succeeded => Results.NoContent(),
                IdentityLifecycleOutcome.InvalidHandle => Results.NotFound(),
                _ => Results.Problem(statusCode: StatusCodes.Status409Conflict, title: "Refresh token revocation refused", detail: outcome.ToString()),
            };
        });
    }

    /// <summary>Maps the platform password-recovery initiation endpoint at the supplied path.</summary>
    public static RouteHandlerBuilder MapPlatformPasswordRecoveryInitiation(this IEndpointRouteBuilder endpoints, string path = "/platform/identity/password-recovery")
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("A path is required.", nameof(path));
        return endpoints.MapPost(path, async (PasswordRecoveryRequest request, IIdentityLifecycleCoordinator coordinator, CancellationToken cancellationToken) =>
        {
            if (request is null || string.IsNullOrWhiteSpace(request.SubjectIdentifier))
                return Results.Problem(statusCode: StatusCodes.Status400BadRequest, title: "Invalid request", detail: "A subject identifier is required.");
            var result = await coordinator.PasswordRecovery.InitiateAsync(request.SubjectIdentifier, cancellationToken).ConfigureAwait(false);
            if (!result.Succeeded)
                return Results.Problem(statusCode: StatusCodes.Status409Conflict, title: "Password recovery refused", detail: result.Outcome.ToString());
            return Results.Accepted(value: new { result.Value!.ChallengeId });
        });
    }

    /// <summary>Maps the platform password-recovery completion endpoint at the supplied path.</summary>
    public static RouteHandlerBuilder MapPlatformPasswordRecoveryCompletion(this IEndpointRouteBuilder endpoints, string path = "/platform/identity/password-recovery/complete")
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("A path is required.", nameof(path));
        return endpoints.MapPost(path, async (PasswordRecoveryCompletion request, IIdentityLifecycleCoordinator coordinator, CancellationToken cancellationToken) =>
        {
            if (request is null || string.IsNullOrWhiteSpace(request.ChallengeId) || string.IsNullOrEmpty(request.Code) || string.IsNullOrEmpty(request.NewPassword))
                return Results.Problem(statusCode: StatusCodes.Status400BadRequest, title: "Invalid request", detail: "challengeId, code, and newPassword are required.");
            var result = await coordinator.PasswordRecovery.CompleteAsync(request.ChallengeId, request.Code, request.NewPassword, cancellationToken).ConfigureAwait(false);
            return result.Succeeded
                ? Results.NoContent()
                : Results.Problem(statusCode: StatusCodes.Status409Conflict, title: "Password recovery refused", detail: result.Outcome.ToString());
        });
    }

    /// <summary>Maps the platform two-factor challenge endpoint at the supplied path.</summary>
    public static RouteHandlerBuilder MapPlatformTwoFactorChallenge(this IEndpointRouteBuilder endpoints, string path = "/platform/identity/two-factor")
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("A path is required.", nameof(path));
        return endpoints.MapPost(path, async (TwoFactorChallengeRequest request, IIdentityLifecycleCoordinator coordinator, CancellationToken cancellationToken) =>
        {
            if (request is null || string.IsNullOrWhiteSpace(request.SubjectId))
                return Results.Problem(statusCode: StatusCodes.Status400BadRequest, title: "Invalid request", detail: "A subjectId is required.");
            var result = await coordinator.TwoFactor.IssueAsync(request.SubjectId, cancellationToken).ConfigureAwait(false);
            if (!result.Succeeded)
                return Results.Problem(statusCode: StatusCodes.Status409Conflict, title: "Two-factor challenge refused", detail: result.Outcome.ToString());
            return Results.Accepted(value: new { result.Value!.ChallengeId, result.Value.ExpiresAt });
        });
    }

    /// <summary>Maps the platform two-factor verification endpoint at the supplied path.</summary>
    public static RouteHandlerBuilder MapPlatformTwoFactorVerification(this IEndpointRouteBuilder endpoints, string path = "/platform/identity/two-factor/verify")
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("A path is required.", nameof(path));
        return endpoints.MapPost(path, async (TwoFactorVerification request, IIdentityLifecycleCoordinator coordinator, CancellationToken cancellationToken) =>
        {
            if (request is null || string.IsNullOrWhiteSpace(request.ChallengeId) || string.IsNullOrEmpty(request.Code))
                return Results.Problem(statusCode: StatusCodes.Status400BadRequest, title: "Invalid request", detail: "challengeId and code are required.");
            var result = await coordinator.TwoFactor.VerifyAsync(request.ChallengeId, request.Code, cancellationToken).ConfigureAwait(false);
            return result.Succeeded
                ? Results.NoContent()
                : Results.Problem(statusCode: StatusCodes.Status409Conflict, title: "Two-factor verification refused", detail: result.Outcome.ToString());
        });
    }

    /// <summary>Maps the platform impersonation start endpoint at the supplied path.</summary>
    public static RouteHandlerBuilder MapPlatformImpersonationStart(this IEndpointRouteBuilder endpoints, string path = "/platform/identity/impersonation")
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("A path is required.", nameof(path));
        return endpoints.MapPost(path, async (HttpContext context, IIdentityLifecycleCoordinator coordinator, ICurrentUserAccessor user, CancellationToken cancellationToken) =>
        {
            var current = user.GetCurrentUser();
            if (!current.IsAuthenticated)
                return Results.Problem(statusCode: StatusCodes.Status401Unauthorized, title: "Caller is not authenticated.");
            var payload = await ReadJsonAsync<ImpersonationStartRequest>(context, cancellationToken).ConfigureAwait(false);
            if (payload is null)
                return Results.Problem(statusCode: StatusCodes.Status400BadRequest, title: "Invalid request", detail: "A JSON body with targetSubjectId, reason, and durationMinutes is required.");
            var request = new ImpersonationAuthorizationRequest(
                CallerSubjectId: current.SubjectId ?? string.Empty,
                TargetSubjectId: payload.TargetSubjectId ?? string.Empty,
                Reason: payload.Reason,
                Duration: TimeSpan.FromMinutes(payload.DurationMinutes));
            var result = await coordinator.Impersonation.StartAsync(request, cancellationToken).ConfigureAwait(false);
            return result.Succeeded
                ? Results.Created($"/platform/identity/impersonation/{result.Value!.GrantId}", new { result.Value.GrantId, result.Value.ExpiresAt })
                : Results.Problem(statusCode: StatusCodes.Status409Conflict, title: "Impersonation refused", detail: result.Outcome.ToString());
        });
    }

    /// <summary>Maps the platform impersonation end endpoint at the supplied path.</summary>
    public static RouteHandlerBuilder MapPlatformImpersonationEnd(this IEndpointRouteBuilder endpoints, string path = "/platform/identity/impersonation/{grantId}/end")
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("A path is required.", nameof(path));
        return endpoints.MapPost(path, async (string grantId, IIdentityLifecycleCoordinator coordinator, CancellationToken cancellationToken) =>
        {
            if (string.IsNullOrWhiteSpace(grantId))
                return Results.Problem(statusCode: StatusCodes.Status400BadRequest, title: "Invalid request", detail: "A grantId is required.");
            var outcome = await coordinator.Impersonation.EndAsync(grantId, cancellationToken).ConfigureAwait(false);
            return outcome switch
            {
                IdentityLifecycleOutcome.Succeeded => Results.NoContent(),
                IdentityLifecycleOutcome.InvalidHandle => Results.NotFound(),
                _ => Results.Problem(statusCode: StatusCodes.Status409Conflict, title: "Impersonation end refused", detail: outcome.ToString()),
            };
        });
    }

    private static string? ExtractHandle(HttpRequest request)
    {
        if (request.Headers.TryGetValue("X-Refresh-Token", out var header) && header.Count > 0)
            return header[0];
        return null;
    }

    private static IResult ToHttpResult<T>(IdentityLifecycleResult<T> result, Func<T, IResult> success)
    {
        if (result.Succeeded)
            return success(result.Value!);
        return result.Outcome switch
        {
            IdentityLifecycleOutcome.InvalidHandle => Results.Problem(statusCode: StatusCodes.Status400BadRequest, title: "Invalid handle"),
            IdentityLifecycleOutcome.Expired => Results.Problem(statusCode: StatusCodes.Status401Unauthorized, title: "Refresh token expired"),
            IdentityLifecycleOutcome.Revoked => Results.Problem(statusCode: StatusCodes.Status401Unauthorized, title: "Refresh token revoked"),
            IdentityLifecycleOutcome.Replayed => Results.Problem(statusCode: StatusCodes.Status401Unauthorized, title: "Refresh token replayed"),
            IdentityLifecycleOutcome.PolicyDenied => Results.Problem(statusCode: StatusCodes.Status403Forbidden, title: "Refresh token refused"),
            _ => Results.Problem(statusCode: StatusCodes.Status409Conflict, title: "Refresh token refused", detail: result.Outcome.ToString()),
        };
    }

    private static async Task<T?> ReadJsonAsync<T>(HttpContext context, CancellationToken cancellationToken) where T : class
    {
        try
        {
            return await System.Text.Json.JsonSerializer.DeserializeAsync<T>(context.Request.Body, JsonOptionsInstance, cancellationToken).ConfigureAwait(false);
        }
        catch (System.Text.Json.JsonException)
        {
            return null;
        }
    }
}

/// <summary>Request body for the password-recovery initiation endpoint.</summary>
public sealed class PasswordRecoveryRequest
{
    /// <summary>The subject identifier (email, phone, or other opaque subject) initiating recovery.</summary>
    public string? SubjectIdentifier { get; set; }
}

/// <summary>Request body for the password-recovery completion endpoint.</summary>
public sealed class PasswordRecoveryCompletion
{
    /// <summary>The opaque challenge identifier returned by the initiation endpoint.</summary>
    public string? ChallengeId { get; set; }
    /// <summary>The one-time code delivered to the subject.</summary>
    public string? Code { get; set; }
    /// <summary>The new password to apply.</summary>
    public string? NewPassword { get; set; }
}

/// <summary>Request body for the two-factor challenge endpoint.</summary>
public sealed class TwoFactorChallengeRequest
{
    /// <summary>The subject identifier that requires two-factor verification.</summary>
    public string? SubjectId { get; set; }
}

/// <summary>Request body for the two-factor verification endpoint.</summary>
public sealed class TwoFactorVerification
{
    /// <summary>The opaque challenge identifier returned by the challenge endpoint.</summary>
    public string? ChallengeId { get; set; }
    /// <summary>The one-time code delivered to the subject.</summary>
    public string? Code { get; set; }
}

/// <summary>Request body for the impersonation start endpoint.</summary>
public sealed class ImpersonationStartRequest
{
    /// <summary>The subject identifier to impersonate.</summary>
    public string? TargetSubjectId { get; set; }
    /// <summary>The reason for the impersonation grant; required by the platform.</summary>
    public string? Reason { get; set; }
    /// <summary>The grant duration in minutes.</summary>
    public int DurationMinutes { get; set; }
}
