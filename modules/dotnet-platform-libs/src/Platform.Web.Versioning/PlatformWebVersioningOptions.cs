namespace Platform.Web.Versioning;

/// <summary>Configures the opt-in platform web API versioning registration.</summary>
public sealed class PlatformWebVersioningOptions
{
    /// <summary>The default major version applied when callers do not provide one.</summary>
    public const int DefaultMajorVersion = 1;

    /// <summary>The default minor version applied when callers do not provide one.</summary>
    public const int DefaultMinorVersion = 0;

    /// <summary>The default route constraint token substituted into URL segments.</summary>
    public const string DefaultRouteConstraintName = "apiVersion";

    /// <summary>The default format string used to render an API version as an API Explorer group name.</summary>
    public const string DefaultGroupNameFormat = "'v'VVV";

    /// <summary>The default HTTP header used when <see cref="Reader"/> is <see cref="PlatformVersionReaderKind.Header"/>.</summary>
    public const string DefaultHeaderName = "X-Api-Version";

    /// <summary>The default query string parameter used when <see cref="Reader"/> is <see cref="PlatformVersionReaderKind.QueryString"/>.</summary>
    public const string DefaultQueryParameterName = "api-version";

    /// <summary>Gets or sets the major component of the default API version. Defaults to <c>1</c>.</summary>
    public int DefaultMajor { get; set; } = DefaultMajorVersion;

    /// <summary>Gets or sets the minor component of the default API version. Defaults to <c>0</c>.</summary>
    public int DefaultMinor { get; set; } = DefaultMinorVersion;

    /// <summary>Gets or sets whether requests without a version are treated as the default version. Defaults to <c>true</c>.</summary>
    public bool AssumeDefaultVersionWhenUnspecified { get; set; } = true;

    /// <summary>Gets or sets whether responses advertise supported and deprecated versions in headers. Defaults to <c>false</c>.</summary>
    public bool ReportApiVersions { get; set; }

    /// <summary>Gets or sets the route constraint name used for the URL-segment reader. Defaults to <c>apiVersion</c>.</summary>
    public string RouteConstraintName { get; set; } = DefaultRouteConstraintName;

    /// <summary>Gets or sets the API Explorer group name format. Defaults to <c>'v'VVV</c> (e.g. <c>v1</c>, <c>v1.1</c>).</summary>
    public string GroupNameFormat { get; set; } = DefaultGroupNameFormat;

    /// <summary>Gets or sets the version reader. Defaults to <see cref="PlatformVersionReaderKind.UrlSegment"/>.</summary>
    public PlatformVersionReaderKind Reader { get; set; } = PlatformVersionReaderKind.UrlSegment;

    /// <summary>Gets or sets the header name when <see cref="Reader"/> is <see cref="PlatformVersionReaderKind.Header"/>. Defaults to <c>X-Api-Version</c>.</summary>
    public string HeaderName { get; set; } = DefaultHeaderName;

    /// <summary>Gets or sets the query parameter name when <see cref="Reader"/> is <see cref="PlatformVersionReaderKind.QueryString"/>. Defaults to <c>api-version</c>.</summary>
    public string QueryParameterName { get; set; } = DefaultQueryParameterName;

    /// <summary>Validates the configured values and returns a redacted list of human-readable errors.</summary>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (DefaultMajor < 0) errors.Add("DefaultMajor must be non-negative.");
        if (DefaultMinor is < 0 or > 99) errors.Add("DefaultMinor must be between 0 and 99.");
        if (string.IsNullOrWhiteSpace(RouteConstraintName)) errors.Add("RouteConstraintName is required.");
        if (string.IsNullOrWhiteSpace(GroupNameFormat)) errors.Add("GroupNameFormat is required.");
        if (Reader is PlatformVersionReaderKind.Header && string.IsNullOrWhiteSpace(HeaderName))
            errors.Add("HeaderName is required when Reader is Header.");
        if (Reader is PlatformVersionReaderKind.QueryString && string.IsNullOrWhiteSpace(QueryParameterName))
            errors.Add("QueryParameterName is required when Reader is QueryString.");
        return errors;
    }
}
