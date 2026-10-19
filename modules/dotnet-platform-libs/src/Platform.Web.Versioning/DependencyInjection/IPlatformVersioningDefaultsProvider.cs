using Asp.Versioning;
using Microsoft.Extensions.Options;

namespace Platform.Web.Versioning.DependencyInjection;

/// <summary>Surface used by tests and applications to inspect the configured default API version.</summary>
public interface IPlatformVersioningDefaultsProvider
{
    /// <summary>The currently configured default <see cref="ApiVersion"/>.</summary>
    ApiVersion DefaultApiVersion { get; }

    /// <summary>Whether the platform assumes the default version when the request does not specify one.</summary>
    bool AssumeDefaultVersionWhenUnspecified { get; }
}

internal sealed class PlatformVersioningDefaultsProvider : IPlatformVersioningDefaultsProvider
{
    private readonly IOptions<PlatformWebVersioningOptions> _options;
    public PlatformVersioningDefaultsProvider(IOptions<PlatformWebVersioningOptions> options) => _options = options;

    public ApiVersion DefaultApiVersion => new(_options.Value.DefaultMajor, _options.Value.DefaultMinor);
    public bool AssumeDefaultVersionWhenUnspecified => _options.Value.AssumeDefaultVersionWhenUnspecified;
}
