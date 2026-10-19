using Microsoft.AspNetCore.Http;
using System.Text.Json;
using Platform.Privacy.AspNetCore.Cookies;
using Platform.Privacy.AspNetCore.State;
using Platform.Privacy.Contracts;
using Platform.Privacy.Evaluation;

namespace Platform.Privacy.AspNetCore.Enforcement;

/// <summary>Resolves request consent state and enforces <see cref="RequireConsentAttribute"/>.</summary>
/// <remarks>
/// Source precedence: a server-side store record wins when present;
/// otherwise the consent cookie is the transport. A cookie naming a
/// different subject than the resolver is ignored, never merged.
/// </remarks>
public sealed class ConsentEnforcementMiddleware
{
    private readonly RequestDelegate _next;
    private readonly ConsentCookieOptions _cookieOptions;
    private readonly UnresolvedSubjectBehavior _unresolvedBehavior;

    /// <summary>Creates the middleware.</summary>
    public ConsentEnforcementMiddleware(
        RequestDelegate next,
        ConsentCookieOptions cookieOptions,
        UnresolvedSubjectBehavior unresolvedBehavior = UnresolvedSubjectBehavior.Deny)
    {
        ArgumentNullException.ThrowIfNull(next);
        ArgumentNullException.ThrowIfNull(cookieOptions);
        cookieOptions.Validate();
        _next = next;
        _cookieOptions = cookieOptions;
        _unresolvedBehavior = unresolvedBehavior;
    }

    /// <summary>Resolves state, enforces metadata, then continues.</summary>
    public async Task InvokeAsync(
        HttpContext context,
        ConsentState state,
        IConsentSubjectResolver subjectResolver,
        IConsentStore store)
    {
        ArgumentNullException.ThrowIfNull(context);
        ArgumentNullException.ThrowIfNull(state);
        ArgumentNullException.ThrowIfNull(subjectResolver);
        ArgumentNullException.ThrowIfNull(store);

        var policy = new ConsentPolicy(_cookieOptions.PolicyVersion);
        var subjectId = subjectResolver.Resolve(context);
        ConsentRecord? record = null;
        if (subjectId is not null)
        {
            record = await store.GetAsync(subjectId, context.RequestAborted).ConfigureAwait(false);
            if (record is null)
            {
                record = new ConsentCookieService(_cookieOptions).Read(context.Request);
                if (record is not null && !string.Equals(record.SubjectId, subjectId, StringComparison.Ordinal))
                {
                    record = null;
                }
            }
        }

        var granted = Enum.GetValues<ConsentCategory>()
            .Where(category => policy.IsGranted(category, record))
            .ToHashSet();
        if (subjectId is null && _unresolvedBehavior == UnresolvedSubjectBehavior.Deny)
        {
            granted.RemoveWhere(category => category != ConsentCategory.Necessary);
        }

        state.SubjectId = subjectId;
        state.Record = record;
        state.PolicyVersion = _cookieOptions.PolicyVersion;
        state.Granted = granted;

        var required = context.GetEndpoint()?.Metadata.GetMetadata<RequireConsentAttribute>()?.Categories;
        if (required is not null)
        {
            var missing = required.Where(category => !granted.Contains(category)).ToArray();
            if (missing.Length > 0)
            {
                context.Response.StatusCode = StatusCodes.Status403Forbidden;
                context.Response.ContentType = "application/problem+json";
                await JsonSerializer.SerializeAsync(
                    context.Response.Body,
                    new
                    {
                        title = "Consent required.",
                        status = 403,
                        missing = missing.Select(category => category.ToString()).ToArray(),
                    },
                    cancellationToken: context.RequestAborted).ConfigureAwait(false);
                return;
            }
        }

        await _next(context).ConfigureAwait(false);
    }
}
