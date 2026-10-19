using Microsoft.Extensions.Configuration;

namespace Platform.Web;

/// <summary>Validates host configuration without owning its schema.</summary>
public interface IPlatformConfigurationValidator
{
    /// <summary>Returns configuration validation failures.</summary>
    IReadOnlyList<string> Validate(IConfiguration configuration);
}

/// <summary>Redacts sensitive values before logging or returning diagnostics.</summary>
public interface IPlatformRedactor
{
    /// <summary>Returns a safe representation of a value.</summary>
    string Redact(string? value);
}

/// <summary>Describes the availability of an optional external provider.</summary>
public sealed record ProviderStatus(string Name, bool Available, string? Detail = null);

/// <summary>Supplies provider health without requiring any provider package.</summary>
public interface IProviderStatusSource
{
    /// <summary>Gets current provider statuses.</summary>
    IReadOnlyList<ProviderStatus> GetStatuses();
}

/// <summary>Default configuration validator that validates only platform options.</summary>
public sealed class DefaultPlatformConfigurationValidator : IPlatformConfigurationValidator
{
    private readonly Microsoft.Extensions.Options.IOptions<PlatformWebOptions> _options;

    /// <summary>Initializes the validator.</summary>
    public DefaultPlatformConfigurationValidator(Microsoft.Extensions.Options.IOptions<PlatformWebOptions> options) => _options = options;

    /// <inheritdoc />
    public IReadOnlyList<string> Validate(IConfiguration configuration)
    {
        ArgumentNullException.ThrowIfNull(configuration);
        return _options.Value.Validate();
    }
}

/// <summary>Default redactor that never returns a supplied secret verbatim.</summary>
public sealed class DefaultPlatformRedactor : IPlatformRedactor
{
    /// <inheritdoc />
    public string Redact(string? value) => string.IsNullOrEmpty(value) ? string.Empty : "[REDACTED]";
}

/// <summary>Provider source used when no external provider has been configured.</summary>
public sealed class EmptyProviderStatusSource : IProviderStatusSource
{
    /// <inheritdoc />
    public IReadOnlyList<ProviderStatus> GetStatuses() => Array.Empty<ProviderStatus>();
}
