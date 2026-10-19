namespace Platform.Domain.Tests;

public class AggregateDomainEventTests
{
    [Fact]
    public void Aggregate_records_events_in_insertion_order()
    {
        var aggregate = new TestAggregate(Guid.NewGuid());
        var first = new TestEvent(Guid.NewGuid(), DateTimeOffset.UtcNow);
        var second = new TestEvent(Guid.NewGuid(), DateTimeOffset.UtcNow);

        aggregate.AddDomainEvent(first);
        aggregate.AddDomainEvent(second);

        Assert.Equal(new IDomainEvent[] { first, second }, aggregate.DomainEvents);
    }

    [Fact]
    public void ClearDomainEvents_empties_without_dispatching()
    {
        var aggregate = new TestAggregate(Guid.NewGuid());
        aggregate.AddDomainEvent(new TestEvent(Guid.NewGuid(), DateTimeOffset.UtcNow));

        aggregate.ClearDomainEvents();

        Assert.Empty(aggregate.DomainEvents);
    }

    [Fact]
    public void AddDomainEvent_rejects_null()
    {
        var aggregate = new TestAggregate(Guid.NewGuid());

        Assert.Throws<ArgumentNullException>(() => aggregate.AddDomainEvent(null!));
    }

    [Fact]
    public void Contract_only_aggregate_records_and_clears_without_inheritance()
    {
        IAggregateRoot<Guid> aggregate = new ContractOnlyAggregate(Guid.NewGuid());
        var @event = new TestEvent(Guid.NewGuid(), DateTimeOffset.UtcNow);

        aggregate.AddDomainEvent(@event);

        Assert.Single(aggregate.DomainEvents);
        aggregate.ClearDomainEvents();
        Assert.Empty(aggregate.DomainEvents);
    }

    [Fact]
    public void DomainEvent_Create_supplies_id_and_timestamp()
    {
        var before = DateTimeOffset.UtcNow;

        var @event = DomainEvent.Create((id, at) => new TestEvent(id, at));

        var after = DateTimeOffset.UtcNow;
        Assert.NotEqual(Guid.Empty, @event.EventId);
        Assert.InRange(@event.OccurredOnUtc, before.AddSeconds(-1), after.AddSeconds(1));
    }

    [Fact]
    public void DomainEvent_Create_rejects_null_factory()
    {
        Assert.Throws<ArgumentNullException>(() => DomainEvent.Create<TestEvent>(null!));
    }

    private sealed class TestAggregate : AggregateRoot<Guid>
    {
        public TestAggregate(Guid id)
            : base(id)
        {
        }
    }

    private sealed class ContractOnlyAggregate : IAggregateRoot<Guid>
    {
        private readonly List<IDomainEvent> _events = [];

        public ContractOnlyAggregate(Guid id)
        {
            Id = id;
        }

        public Guid Id { get; }

        public IReadOnlyCollection<IDomainEvent> DomainEvents => _events.AsReadOnly();

        public void AddDomainEvent(IDomainEvent domainEvent)
        {
            ArgumentNullException.ThrowIfNull(domainEvent);
            _events.Add(domainEvent);
        }

        public void ClearDomainEvents() => _events.Clear();
    }

    private sealed record TestEvent(
        Guid EventId,
        DateTimeOffset OccurredOnUtc,
        string? CorrelationId = null,
        string? TenantId = null) : DomainEvent(EventId, OccurredOnUtc, CorrelationId, TenantId);
}
