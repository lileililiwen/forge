namespace Platform.Idempotency.Tests;

public class IdempotencyOptionsTests
{
    [Fact]
    public void Defaults_apply_documented_values()
    {
        var options = new IdempotencyOptions();

        Assert.True(options.Enabled);
        Assert.Equal("memory", options.Storage);
        Assert.Equal(86400, options.RetentionSeconds);
        Assert.Equal(256, options.MaxKeyLength);
        Assert.Equal("Idempotency-Key", options.HeaderName);
    }

    [Fact]
    public void Section_and_metric_name_constants_are_stable()
    {
        Assert.Equal("Idempotency", IdempotencyOptions.SectionName);
        Assert.Equal("idempotency.hit", IdempotencyOptions.HitMetric);
        Assert.Equal("idempotency.miss", IdempotencyOptions.MissMetric);
        Assert.Equal("idempotency.fingerprint_mismatch", IdempotencyOptions.FingerprintMismatchMetric);
    }
}
