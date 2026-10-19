using Platform.Core.Audit;
using Platform.Core.Time;

namespace Platform.Core.Tests.Audit;

public class AuditableTests
{
    [Fact]
    public void Product_entity_can_implement_IAuditable_without_inheritance()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var entity = new ProductEntity(
            name: "thing",
            createdAt: clock.UtcNow,
            createdBy: "user-1");

        Assert.Equal("thing", entity.Name);
        Assert.Equal(clock.UtcNow, entity.CreatedAt);
        Assert.Equal("user-1", entity.CreatedBy);
        Assert.Null(entity.UpdatedAt);
        Assert.Null(entity.UpdatedBy);
    }

    [Fact]
    public void Entity_audit_metadata_is_mutable_through_application_API()
    {
        var initial = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        var later = initial.AddDays(1);
        var entity = new ProductEntity("thing", initial, "user-1");

        entity.MarkUpdated(later, "user-2");

        Assert.Equal(later, entity.UpdatedAt);
        Assert.Equal("user-2", entity.UpdatedBy);
        Assert.Equal(initial, entity.CreatedAt);
        Assert.Equal("user-1", entity.CreatedBy);
    }

    private sealed class ProductEntity : IAuditable
    {
        public ProductEntity(string name, DateTimeOffset createdAt, string? createdBy)
        {
            Name = name;
            CreatedAt = createdAt;
            CreatedBy = createdBy;
        }

        public string Name { get; }

        public DateTimeOffset CreatedAt { get; }

        public string? CreatedBy { get; }

        public DateTimeOffset? UpdatedAt { get; private set; }

        public string? UpdatedBy { get; private set; }

        public void MarkUpdated(DateTimeOffset at, string? by)
        {
            UpdatedAt = at;
            UpdatedBy = by;
        }
    }
}
