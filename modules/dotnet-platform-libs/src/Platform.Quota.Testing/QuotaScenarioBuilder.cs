using Platform.Core.Time;
using Platform.Quota.Contracts;

namespace Platform.Quota.Testing;

/// <summary>Builds deterministic quota scenarios without product plan models.</summary>
public sealed class QuotaScenarioBuilder
{
    private QuotaSubject _subject = new("subject-1");
    private QuotaResource _resource = new("resource-1");
    private QuotaWindow _window = new(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero), new DateTimeOffset(2026, 2, 1, 0, 0, 0, TimeSpan.Zero));
    private long _limit = 100;
    private long _consumed;
    /// <summary>Scenario clock.</summary>
    public IClock Clock { get; private set; } = new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero));
    /// <summary>Scenario subject.</summary>
    public QuotaSubject Subject => _subject;
    /// <summary>Scenario resource.</summary>
    public QuotaResource Resource => _resource;
    /// <summary>Scenario window.</summary>
    public QuotaWindow Window => _window;
    /// <summary>Scenario limit.</summary>
    public long Limit => _limit;
    /// <summary>Scenario consumed amount.</summary>
    public long Consumed => _consumed;
    /// <summary>Sets the quota limit.</summary>
    public QuotaScenarioBuilder WithLimit(long limit) { _limit = limit; return this; }
    /// <summary>Sets the opaque subject.</summary>
    public QuotaScenarioBuilder WithSubject(string subject) { _subject = new QuotaSubject(subject); return this; }
    /// <summary>Sets the opaque resource.</summary>
    public QuotaScenarioBuilder WithResource(string resource) { _resource = new QuotaResource(resource); return this; }
    /// <summary>Sets initial consumption.</summary>
    public QuotaScenarioBuilder WithConsumed(long consumed) { _consumed = consumed; return this; }
    /// <summary>Builds the current scenario.</summary>
    public QuotaScenarioBuilder Build() => this;
}
