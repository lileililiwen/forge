using Platform.Observability;
using Platform.Observability.Redaction;

namespace Platform.Observability.Tests;

public sealed class ObservabilityOptionsTests
{
    [Fact]
    public void Default_options_validate()
    {
        var options = new PlatformObservabilityOptions();
        Assert.Empty(options.Validate());
    }

    [Fact]
    public void Application_name_is_required()
    {
        var options = new PlatformObservabilityOptions { ApplicationName = string.Empty };
        Assert.Contains(options.Validate(), e => e.Contains("ApplicationName is required."));
    }

    [Fact]
    public void Correlation_header_is_required()
    {
        var options = new PlatformObservabilityOptions { CorrelationHeader = " " };
        Assert.Contains(options.Validate(), e => e.Contains("CorrelationHeader is required."));
    }

    [Theory]
    [InlineData(8, true)]
    [InlineData(16, false)]
    [InlineData(1024, false)]
    [InlineData(1025, true)]
    public void Max_tag_length_must_be_bounded(int value, bool shouldFail)
    {
        var options = new PlatformObservabilityOptions { MaxTagLength = value };
        var errors = options.Validate();
        Assert.Equal(shouldFail, errors.Any(e => e.Contains("MaxTagLength")));
    }

    [Theory]
    [InlineData(8, true)]
    [InlineData(16, false)]
    [InlineData(256, false)]
    [InlineData(257, true)]
    public void Max_operation_length_must_be_bounded(int value, bool shouldFail)
    {
        var options = new PlatformObservabilityOptions { MaxOperationLength = value };
        var errors = options.Validate();
        Assert.Equal(shouldFail, errors.Any(e => e.Contains("MaxOperationLength")));
    }

    [Theory]
    [InlineData(8, true)]
    [InlineData(16, false)]
    [InlineData(1024, false)]
    [InlineData(1025, true)]
    public void Max_correlation_id_length_must_be_bounded(int value, bool shouldFail)
    {
        var options = new PlatformObservabilityOptions { MaxCorrelationIdLength = value };
        var errors = options.Validate();
        Assert.Equal(shouldFail, errors.Any(e => e.Contains("MaxCorrelationIdLength")));
    }
}
