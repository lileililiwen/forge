using System.Diagnostics;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Platform.Core.Time;
using Platform.Quota.AspNetCore.Contracts;
using Platform.Quota.AspNetCore.DependencyInjection;
using Platform.Quota.DependencyInjection;
using Platform.Quota.Contracts;

namespace Platform.Quota.AspNetCore.Tests;

/// <summary>Test doubles and a WebApplication builder for quota enforcement tests.</summary>
internal static class TestDoubles
{
    public static QuotaWindow UtcWindow() =>
        new(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero), new DateTimeOffset(2026, 2, 1, 0, 0, 0, TimeSpan.Zero));

    public static WebApplication BuildApp(
        IQuotaStore store,
        IQuotaSubjectResolver subjectResolver,
        IQuotaResourceResolver resourceResolver,
        Action<QuotaEnforcementOptions>? configure = null,
        bool registerHealth = true)
    {
        var builder = WebApplication.CreateBuilder();
        builder.WebHost.UseTestServer();

        builder.Services.AddPlatformQuota(options => { });
        builder.Services.AddSingleton(store);
        builder.Services.AddSingleton<IClock>(new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero)));
        builder.Services.AddPlatformQuotaAspNetCore(configure ?? (_ => { }));
        builder.Services.AddSingleton(subjectResolver);
        builder.Services.AddSingleton(resourceResolver);

        var app = builder.Build();
        app.UsePlatformQuota();
        app.MapGet("/resource", () => Results.Ok("allowed"));
        if (registerHealth) app.MapGet("/health", () => Results.Ok("healthy"));
        app.MapPost("/write", () => Results.Ok("written"));
        app.MapGet("/exempt", () => Results.Ok("exempt")).WithMetadata(new QuotaExemptAttribute());

        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}

/// <summary>Subject resolver that returns a fixed subject or reports missing context.</summary>
internal sealed class FixedSubjectResolver : IQuotaSubjectResolver
{
    private readonly QuotaSubject? _subject;
    public FixedSubjectResolver(QuotaSubject? subject) => _subject = subject;
    public Task<QuotaSubjectResolution> ResolveAsync(HttpContext context, CancellationToken cancellationToken = default) =>
        Task.FromResult(_subject is null ? QuotaSubjectResolution.Missing : new QuotaSubjectResolution(_subject));
}

/// <summary>Resource resolver returning a fixed set of quota operations.</summary>
internal sealed class FixedResourceResolver : IQuotaResourceResolver
{
    private readonly IReadOnlyList<QuotaRequest> _requests;
    public FixedResourceResolver(IReadOnlyList<QuotaRequest> requests) => _requests = requests;
    public Task<IReadOnlyList<QuotaRequest>> ResolveAsync(HttpContext context, QuotaSubject subject, CancellationToken cancellationToken = default) =>
        Task.FromResult(_requests);
}

/// <summary>Resource resolver that records invocation for cancellation assertions.</summary>
internal sealed class ObservingResourceResolver : IQuotaResourceResolver
{
    public bool Invoked { get; private set; }
    public Task<IReadOnlyList<QuotaRequest>> ResolveAsync(HttpContext context, QuotaSubject subject, CancellationToken cancellationToken = default)
    {
        Invoked = true;
        return Task.FromResult<IReadOnlyList<QuotaRequest>>(Array.Empty<QuotaRequest>());
    }
}

/// <summary>Store that records the last cancellation token it received and delegates to an inner store.</summary>
internal sealed class TokenObservingStore : IQuotaStore
{
    private readonly IQuotaStore _inner;
    public CancellationToken? LastToken { get; private set; }
    public TokenObservingStore(IQuotaStore inner) => _inner = inner;

    public Task<QuotaDecision> CheckAsync(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, long requested, CancellationToken cancellationToken = default)
    {
        LastToken = cancellationToken;
        return _inner.CheckAsync(subject, resource, window, limit, requested, cancellationToken);
    }

    public Task<QuotaLifecycleResult> ReserveAsync(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, string operationKey, long amount, TimeSpan? reservationLifetime = null, CancellationToken cancellationToken = default)
    {
        LastToken = cancellationToken;
        return _inner.ReserveAsync(subject, resource, window, limit, operationKey, amount, reservationLifetime, cancellationToken);
    }

    public Task<QuotaLifecycleResult> SettleAsync(string operationKey, CancellationToken cancellationToken = default)
    {
        LastToken = cancellationToken;
        return _inner.SettleAsync(operationKey, cancellationToken);
    }

    public Task<QuotaLifecycleResult> ReleaseAsync(string operationKey, CancellationToken cancellationToken = default)
    {
        LastToken = cancellationToken;
        return _inner.ReleaseAsync(operationKey, cancellationToken);
    }

    public Task<QuotaSnapshot> GetSnapshotAsync(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, CancellationToken cancellationToken = default)
    {
        LastToken = cancellationToken;
        return _inner.GetSnapshotAsync(subject, resource, window, limit, cancellationToken);
    }
}

/// <summary>Store that fails every call so provider-unavailable policies can be exercised.</summary>
internal sealed class ThrowingQuotaStore : IQuotaStore
{
    private static readonly InvalidOperationException Failure = new("quota store unavailable");
    public Task<QuotaDecision> CheckAsync(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, long requested, CancellationToken cancellationToken = default) => throw Failure;
    public Task<QuotaLifecycleResult> ReserveAsync(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, string operationKey, long amount, TimeSpan? reservationLifetime = null, CancellationToken cancellationToken = default) => throw Failure;
    public Task<QuotaLifecycleResult> SettleAsync(string operationKey, CancellationToken cancellationToken = default) => throw Failure;
    public Task<QuotaLifecycleResult> ReleaseAsync(string operationKey, CancellationToken cancellationToken = default) => throw Failure;
    public Task<QuotaSnapshot> GetSnapshotAsync(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, CancellationToken cancellationToken = default) => throw Failure;
}
