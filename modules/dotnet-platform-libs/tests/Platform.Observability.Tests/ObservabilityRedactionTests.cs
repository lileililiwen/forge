using Platform.Observability;
using Platform.Observability.Redaction;

namespace Platform.Observability.Tests;

public sealed class ObservabilityRedactionTests
{
    [Fact]
    public void Default_redactor_returns_placeholder_for_non_empty()
    {
        var redactor = new DefaultPlatformObservabilityRedactor();
        Assert.Equal("[REDACTED]", redactor.Redact("secret"));
    }

    [Fact]
    public void Default_redactor_returns_empty_for_null_or_empty()
    {
        var redactor = new DefaultPlatformObservabilityRedactor();
        Assert.Equal(string.Empty, redactor.Redact(null));
        Assert.Equal(string.Empty, redactor.Redact(string.Empty));
    }

    [Fact]
    public void Safe_value_policy_redacts_and_bounds()
    {
        var redactor = new ReplaceRedactor("***");
        var policy = new PlatformObservabilitySafeValuePolicy(redactor, 6, 4, truncateOversizedValues: true);
        Assert.Equal("***", policy.RedactTag("input"));
        Assert.Equal("***", policy.RedactOperation("input"));
    }

    [Fact]
    public void Safe_value_policy_drops_when_truncation_disabled()
    {
        var redactor = new ReplaceRedactor("***-redacted-***");
        var policy = new PlatformObservabilitySafeValuePolicy(redactor, 4, 2, truncateOversizedValues: false);
        Assert.Equal(string.Empty, policy.RedactTag("input"));
        Assert.Equal(string.Empty, policy.RedactOperation("input"));
    }

    [Fact]
    public void Safe_value_policy_requires_non_empty_operation()
    {
        var policy = new PlatformObservabilitySafeValuePolicy(new DefaultPlatformObservabilityRedactor(), 64, 64, truncateOversizedValues: true);
        Assert.Throws<ArgumentException>(() => policy.RequireOperation(""));
        Assert.Throws<ArgumentException>(() => policy.RequireOperation("   "));
    }

    [Fact]
    public void Safe_value_policy_bounds_correlation_id()
    {
        var policy = new PlatformObservabilitySafeValuePolicy(new DefaultPlatformObservabilityRedactor(), 8, 64, truncateOversizedValues: true);
        var bounded = policy.RequireCorrelationId(new string('a', 64));
        Assert.Equal(8, bounded.Length);
    }

    [Fact]
    public void Safe_value_policy_rejects_empty_correlation_id()
    {
        var policy = new PlatformObservabilitySafeValuePolicy(new DefaultPlatformObservabilityRedactor(), 64, 64, truncateOversizedValues: true);
        Assert.Throws<ArgumentException>(() => policy.RequireCorrelationId(""));
    }

    private sealed class ReplaceRedactor : IPlatformObservabilityRedactor
    {
        private readonly string _replacement;
        public ReplaceRedactor(string replacement) => _replacement = replacement;
        public string Redact(string? value) => string.IsNullOrEmpty(value) ? string.Empty : _replacement;
    }
}
