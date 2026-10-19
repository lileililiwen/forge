using Microsoft.Extensions.Configuration;

namespace Platform.Starter;

/// <summary>Describes the capabilities explicitly enabled by a host.</summary>
public interface IPlatformApplicationStatus
{
    /// <summary>Gets enabled capability identifiers in registration order.</summary>
    IReadOnlyList<string> EnabledCapabilities { get; }
}

/// <summary>Provides safe starter configuration validation to the web runtime.</summary>
public sealed class PlatformApplicationConfigurationValidator : Platform.Web.IPlatformConfigurationValidator
{
    private readonly Microsoft.Extensions.Options.IOptions<PlatformApplicationOptions> options;
    /// <summary>Initializes the validator.</summary>
    public PlatformApplicationConfigurationValidator(Microsoft.Extensions.Options.IOptions<PlatformApplicationOptions> options) => this.options = options;
    /// <inheritdoc />
    public IReadOnlyList<string> Validate(IConfiguration configuration)
    {
        ArgumentNullException.ThrowIfNull(configuration);
        return options.Value.Validate();
    }
}

internal sealed class PlatformApplicationStatus(PlatformApplicationOptions options) : IPlatformApplicationStatus
{
    public IReadOnlyList<string> EnabledCapabilities { get; } = Build(options);
    private static List<string> Build(PlatformApplicationOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        var values = new List<string>();
        if (options.EnableWeb) values.Add("web");
        if (options.EnableIdentity) values.Add("identity");
        if (options.EnableAdmin) values.Add("admin");
        if (options.EnableBilling) values.Add("billing");
        if (options.EnableAi) values.Add("ai");
        if (options.EnableNotifications) values.Add("notifications");
        if (options.EnableSms) values.Add("sms");
        return values;
    }
}
