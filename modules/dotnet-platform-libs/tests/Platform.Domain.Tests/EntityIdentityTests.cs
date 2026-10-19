namespace Platform.Domain.Tests;

public class EntityIdentityTests
{
    [Fact]
    public void Entity_carries_typed_identity()
    {
        var order = new TestOrder(Guid.NewGuid());

        Assert.NotEqual(Guid.Empty, order.Id);
    }

    [Fact]
    public void Contract_only_entity_needs_no_base_class()
    {
        IEntity<int> entity = new ContractOnlyEntity(42);

        Assert.Equal(42, entity.Id);
    }

    private sealed class TestOrder : Entity<Guid>
    {
        public TestOrder(Guid id)
            : base(id)
        {
        }
    }

    private sealed class ContractOnlyEntity : IEntity<int>
    {
        public ContractOnlyEntity(int id)
        {
            Id = id;
        }

        public int Id { get; }
    }
}
