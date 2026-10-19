using Platform.Auditing.Contracts;
using Platform.Auditing.Contracts.Common;

namespace Platform.Auditing.Tests.Contracts;

public class InMemoryAuditSinkAndOptionsTests
{
    [Fact]
    public async Task InMemorySink_evicts_oldest_when_over_capacity()
    {
        var sink = new InMemoryAuditSink(capacity: 2);
        await sink.RecordAsync(AuditEvent.Create("a", "http", AuditOutcome.Success, DateTimeOffset.UtcNow));
        await sink.RecordAsync(AuditEvent.Create("b", "http", AuditOutcome.Success, DateTimeOffset.UtcNow));
        await sink.RecordAsync(AuditEvent.Create("c", "http", AuditOutcome.Success, DateTimeOffset.UtcNow));

        var recorded = sink.GetRecorded();
        Assert.Equal(2, recorded.Count);
        Assert.Equal("b", recorded[0].Action);
        Assert.Equal("c", recorded[1].Action);
        Assert.Equal(3, sink.AcceptedCount);
    }

    [Fact]
    public void AuditOptions_defaults_are_valid()
    {
        var errors = new AuditOptions().Validate();
        Assert.Empty(errors);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(-1)]
    public void AuditOptions_rejects_non_positive_capacity(int capacity)
    {
        var options = new AuditOptions { BoundedCapacity = capacity };
        Assert.Contains(options.Validate(), e => e.Contains("BoundedCapacity", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void AuditOptions_rejects_mutually_exclusive_category_filters()
    {
        var options = new AuditOptions
        {
            ExcludedCategories = new List<string> { "security" },
            EnabledCategories = new List<string> { "http" },
        };
        Assert.Contains(options.Validate(), e => e.Contains("mutually exclusive", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void IsCategoryEnabled_respects_excluded_and_enabled_filters()
    {
        var options = new AuditOptions { ExcludedCategories = new List<string> { "security" } };
        Assert.True(options.IsCategoryEnabled("http"));
        Assert.False(options.IsCategoryEnabled("security"));

        var enabled = new AuditOptions { EnabledCategories = new List<string> { "http" } };
        Assert.True(enabled.IsCategoryEnabled("http"));
        Assert.False(enabled.IsCategoryEnabled("entity"));
    }
}

public class AuditAspNetCoreOptionsTests
{
    [Fact]
    public void Defaults_are_valid()
    {
        Assert.Empty(new Platform.Auditing.AspNetCore.Common.AuditAspNetCoreOptions().Validate());
    }

    [Fact]
    public void Rejects_empty_subject_header_and_out_of_range_status_codes()
    {
        var options = new Platform.Auditing.AspNetCore.Common.AuditAspNetCoreOptions
        {
            SubjectHeaderName = string.Empty,
            SecurityStatusCodes = new List<int> { 99, 600 },
        };
        var errors = options.Validate();
        Assert.Contains(errors, e => e.Contains("SubjectHeaderName", StringComparison.OrdinalIgnoreCase));
        Assert.Contains(errors, e => e.Contains("SecurityStatusCodes", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void IsExempt_matches_configured_prefixes()
    {
        var options = new Platform.Auditing.AspNetCore.Common.AuditAspNetCoreOptions();
        Assert.True(options.IsExempt("/health"));
        Assert.True(options.IsExempt("/metrics"));
        Assert.False(options.IsExempt("/api/users"));
    }
}
