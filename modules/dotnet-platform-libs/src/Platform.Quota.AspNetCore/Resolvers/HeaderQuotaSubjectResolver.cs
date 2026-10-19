using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Options;
using Platform.Quota.AspNetCore.Contracts;
using Platform.Quota.Contracts;

namespace Platform.Quota.AspNetCore.Resolvers;

/// <summary>
/// Default subject resolver that reads an opaque subject from a configured request header. When the
/// header is absent or blank the request is treated as missing context so the configured
/// <see cref="MissingContextPolicy"/> applies. Applications replace this resolver to map their own
/// identity or tenant context into the quota subject.
/// </summary>
public sealed class HeaderQuotaSubjectResolver : IQuotaSubjectResolver
{
    private readonly QuotaEnforcementOptions _options;

    /// <summary>Creates the resolver.</summary>
    public HeaderQuotaSubjectResolver(IOptions<QuotaEnforcementOptions> options)
    {
        _options = options?.Value ?? throw new ArgumentNullException(nameof(options));
    }

    /// <inheritdoc />
    public Task<QuotaSubjectResolution> ResolveAsync(HttpContext context, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(context);
        if (string.IsNullOrWhiteSpace(_options.SubjectHeaderName))
            return Task.FromResult(QuotaSubjectResolution.Missing);

        var value = context.Request.Headers[_options.SubjectHeaderName].FirstOrDefault();
        if (string.IsNullOrWhiteSpace(value))
            return Task.FromResult(QuotaSubjectResolution.Missing);

        QuotaSubject subject;
        try
        {
            subject = new QuotaSubject(value!);
        }
        catch (ArgumentException)
        {
            return Task.FromResult(QuotaSubjectResolution.Missing);
        }

        return Task.FromResult(new QuotaSubjectResolution(subject));
    }
}
