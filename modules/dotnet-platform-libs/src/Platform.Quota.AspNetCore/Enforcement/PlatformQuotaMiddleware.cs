using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Quota.AspNetCore.Contracts;
using Platform.Quota.Contracts;

namespace Platform.Quota.AspNetCore.Enforcement;

/// <summary>
/// Enforces application-selected quota resources over <see cref="IQuotaStore"/>. For each resolved
/// <see cref="QuotaRequest"/> it checks (or reserves) capacity and rejects with a safe RFC 9457
/// 429 problem details plus <c>Retry-After</c> when the window reset is known. Unresolved context and
/// unavailable providers follow the configured fail-closed-or-allow policies. Limits, plans, units,
/// and persistence remain application-owned.
/// </summary>
public sealed class PlatformQuotaMiddleware : IMiddleware
{
    private readonly IQuotaStore _store;
    private readonly IQuotaSubjectResolver _subjectResolver;
    private readonly IQuotaResourceResolver _resourceResolver;
    private readonly QuotaProblemDetailsWriter _writer;
    private readonly QuotaEnforcementOptions _options;
    private readonly IClock _clock;
    private readonly ILogger<PlatformQuotaMiddleware> _logger;

    /// <summary>Creates the middleware.</summary>
    public PlatformQuotaMiddleware(
        IQuotaStore store,
        IQuotaSubjectResolver subjectResolver,
        IQuotaResourceResolver resourceResolver,
        QuotaProblemDetailsWriter writer,
        IOptions<QuotaEnforcementOptions> options,
        IClock clock,
        ILogger<PlatformQuotaMiddleware> logger)
    {
        _store = store ?? throw new ArgumentNullException(nameof(store));
        _subjectResolver = subjectResolver ?? throw new ArgumentNullException(nameof(subjectResolver));
        _resourceResolver = resourceResolver ?? throw new ArgumentNullException(nameof(resourceResolver));
        _writer = writer ?? throw new ArgumentNullException(nameof(writer));
        _options = options?.Value ?? throw new ArgumentNullException(nameof(options));
        _clock = clock ?? throw new ArgumentNullException(nameof(clock));
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }

    /// <inheritdoc />
    public async Task InvokeAsync(HttpContext context, RequestDelegate next)
    {
        ArgumentNullException.ThrowIfNull(context);
        ArgumentNullException.ThrowIfNull(next);

        if (!_options.Enabled || IsExempt(context))
        {
            await next(context).ConfigureAwait(false);
            return;
        }

        var subjectOrNull = await ResolveSubjectAsync(context).ConfigureAwait(false);
        if (subjectOrNull is null) return;
        var subject = subjectOrNull.Value;

        IReadOnlyList<QuotaRequest> requests;
        try
        {
            requests = await _resourceResolver.ResolveAsync(context, subject, context.RequestAborted).ConfigureAwait(false);
        }
        catch (InvalidOperationException)
        {
            // Unconfigured resolver: fail closed with a clear policy response.
            await QuotaProblemDetailsWriter.WritePolicyAsync(
                context,
                _options.ProviderUnavailableStatusCode,
                _options.QuotaExceededType,
                "Quota Unavailable",
                "Quota enforcement is not configured for this host.",
                context.RequestAborted).ConfigureAwait(false);
            return;
        }

        if (requests.Count == 0)
        {
            await next(context).ConfigureAwait(false);
            return;
        }

        var reservedKeys = new List<string>();
        var denied = false;
        foreach (var request in requests)
        {
            if (request.Reserve)
            {
                if (!await TryReserveAsync(context, subject, request, reservedKeys).ConfigureAwait(false))
                {
                    denied = true;
                    break;
                }
            }
            else if (!await TryCheckAsync(context, subject, request).ConfigureAwait(false))
            {
                denied = true;
                break;
            }
        }

        if (denied)
        {
            await ReleaseReservedAsync(subject, reservedKeys, context.RequestAborted).ConfigureAwait(false);
            return;
        }

        try
        {
            await next(context).ConfigureAwait(false);
            await SettleReservedAsync(subject, reservedKeys, context.RequestAborted).ConfigureAwait(false);
        }
        catch
        {
            await ReleaseReservedAsync(subject, reservedKeys, context.RequestAborted).ConfigureAwait(false);
            throw;
        }
    }

    private async Task<QuotaSubject?> ResolveSubjectAsync(HttpContext context)
    {
        QuotaSubjectResolution resolution;
        try
        {
            resolution = await _subjectResolver.ResolveAsync(context, context.RequestAborted).ConfigureAwait(false);
        }
        catch (Exception ex)
        {
            QuotaLogMessages.LogSubjectResolutionFailed(_logger, ex);
            resolution = QuotaSubjectResolution.Missing;
        }

        if (resolution.Resolved) return resolution.Subject;

        if (_options.MissingContextPolicy == MissingContextPolicy.Allow)
        {
            return new QuotaSubject(_options.AnonymousSubject);
        }

        await QuotaProblemDetailsWriter.WritePolicyAsync(
            context,
            _options.MissingContextStatusCode,
            _options.QuotaExceededType,
            "Quota Context Required",
            "The request is missing the subject or tenant context required for quota enforcement.",
            context.RequestAborted).ConfigureAwait(false);
        return null;
    }

    private async Task<bool> TryCheckAsync(HttpContext context, QuotaSubject subject, QuotaRequest request)
    {
        QuotaDecision decision;
        try
        {
            decision = await _store.CheckAsync(subject, request.Resource, request.Window, request.Limit, request.Amount, context.RequestAborted).ConfigureAwait(false);
        }
        catch (Exception ex)
        {
            return await HandleStoreFailureAsync(context, ex).ConfigureAwait(false);
        }

        if (decision.Allowed) return true;

        await _writer.WriteExceededAsync(context, decision, _options.QuotaExceededStatusCode, _options.QuotaExceededType, _options.QuotaExceededTitle, context.RequestAborted).ConfigureAwait(false);
        return false;
    }

    private async Task<bool> TryReserveAsync(HttpContext context, QuotaSubject subject, QuotaRequest request, List<string> reservedKeys)
    {
        QuotaLifecycleResult result;
        try
        {
            var operationKey = new QuotaOperationKey($"{context.TraceIdentifier}:{request.Resource.Value}");
            result = await _store.ReserveAsync(subject, request.Resource, request.Window, request.Limit, operationKey.Value, request.Amount, cancellationToken: context.RequestAborted).ConfigureAwait(false);
        }
        catch (Exception ex)
        {
            return await HandleStoreFailureAsync(context, ex).ConfigureAwait(false);
        }

        if (result.Status == QuotaLifecycleStatus.Denied && result.Decision is not null)
        {
            await _writer.WriteExceededAsync(context, result.Decision, _options.QuotaExceededStatusCode, _options.QuotaExceededType, _options.QuotaExceededTitle, context.RequestAborted).ConfigureAwait(false);
            return false;
        }

        if (result.Status == QuotaLifecycleStatus.Reserved)
        {
            reservedKeys.Add(new QuotaOperationKey($"{context.TraceIdentifier}:{request.Resource.Value}").Value);
            return true;
        }

        // Unexpected state (e.g. duplicate key) — treat as a provider fault.
        return await HandleStoreFailureAsync(context, new InvalidOperationException($"Unexpected quota reservation state: {result.Status}.")).ConfigureAwait(false);
    }

    private async Task<bool> HandleStoreFailureAsync(HttpContext context, Exception ex)
    {
        QuotaLogMessages.LogStoreUnavailable(_logger, ex);

        if (_options.ProviderUnavailablePolicy == QuotaUnavailablePolicy.Allow)
        {
            return true;
        }

        await QuotaProblemDetailsWriter.WritePolicyAsync(
            context,
            _options.ProviderUnavailableStatusCode,
            _options.QuotaExceededType,
            "Quota Unavailable",
            "The quota provider is unavailable.",
            context.RequestAborted).ConfigureAwait(false);
        return false;
    }

    private async Task SettleReservedAsync(QuotaSubject subject, IReadOnlyList<string> reservedKeys, CancellationToken cancellationToken)
    {
        foreach (var key in reservedKeys)
        {
            try
            {
                await _store.SettleAsync(key, cancellationToken).ConfigureAwait(false);
            }
            catch (Exception ex)
            {
                QuotaLogMessages.LogSettleFailed(_logger, key, subject.Value, ex);
            }
        }
    }

    private async Task ReleaseReservedAsync(QuotaSubject subject, IReadOnlyList<string> reservedKeys, CancellationToken cancellationToken)
    {
        foreach (var key in reservedKeys)
        {
            try
            {
                await _store.ReleaseAsync(key, cancellationToken).ConfigureAwait(false);
            }
            catch (Exception ex)
            {
                QuotaLogMessages.LogReleaseFailed(_logger, key, subject.Value, ex);
            }
        }
    }

    private bool IsExempt(HttpContext context)
    {
        if (context.GetEndpoint()?.Metadata.GetMetadata<QuotaExemptAttribute>() is not null)
            return true;

        if (_options.ExemptMethods.Count != 0
            && _options.ExemptMethods.Any(m => string.Equals(m, context.Request.Method, StringComparison.OrdinalIgnoreCase)))
        {
            return true;
        }

        if (_options.ExemptPathPrefixes.Count != 0)
        {
            var path = context.Request.Path.ToString();
            if (_options.ExemptPathPrefixes.Any(prefix => path.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)))
                return true;
        }

        return false;
    }
}
