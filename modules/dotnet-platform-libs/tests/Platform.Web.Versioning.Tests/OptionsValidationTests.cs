using Platform.Web.Versioning;

namespace Platform.Web.Versioning.Tests;

public sealed class OptionsValidationTests
{
    [Fact]
    public void Default_options_are_valid()
    {
        var options = new PlatformWebVersioningOptions();
        Assert.Empty(options.Validate());
    }

    [Fact]
    public void Negative_default_major_is_rejected()
    {
        var options = new PlatformWebVersioningOptions { DefaultMajor = -1 };
        Assert.Contains(options.Validate(), e => e.Contains("DefaultMajor", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Out_of_range_default_minor_is_rejected()
    {
        var options = new PlatformWebVersioningOptions { DefaultMinor = 100 };
        Assert.Contains(options.Validate(), e => e.Contains("DefaultMinor", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Header_reader_requires_a_header_name()
    {
        var options = new PlatformWebVersioningOptions { Reader = PlatformVersionReaderKind.Header, HeaderName = " " };
        Assert.Contains(options.Validate(), e => e.Contains("HeaderName", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Query_reader_requires_a_parameter_name()
    {
        var options = new PlatformWebVersioningOptions { Reader = PlatformVersionReaderKind.QueryString, QueryParameterName = "" };
        Assert.Contains(options.Validate(), e => e.Contains("QueryParameterName", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Empty_group_format_is_rejected()
    {
        var options = new PlatformWebVersioningOptions { GroupNameFormat = "" };
        Assert.Contains(options.Validate(), e => e.Contains("GroupNameFormat", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Empty_route_constraint_is_rejected()
    {
        var options = new PlatformWebVersioningOptions { RouteConstraintName = "" };
        Assert.Contains(options.Validate(), e => e.Contains("RouteConstraintName", StringComparison.OrdinalIgnoreCase));
    }
}
