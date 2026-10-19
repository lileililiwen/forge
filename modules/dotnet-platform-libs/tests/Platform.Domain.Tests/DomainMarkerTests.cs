namespace Platform.Domain.Tests;

public class DomainMarkerTests
{
    [Fact]
    public void Application_entity_exposes_soft_delete_state_without_platform_behavior()
    {
        var entity = new SoftDeletableOrder(Guid.NewGuid());

        Assert.False(entity.IsDeleted);
        Assert.Null(entity.DeletedOnUtc);
        Assert.Null(entity.DeletedBy);

        entity.Delete("operator-1", DateTimeOffset.UtcNow);

        Assert.True(entity.IsDeleted);
        Assert.NotNull(entity.DeletedOnUtc);
        Assert.Equal("operator-1", entity.DeletedBy);
    }

    [Fact]
    public void Application_entity_exposes_tenant_ownership_without_platform_behavior()
    {
        IHasTenant entity = new TenantOrder(Guid.NewGuid(), "tenant-1");

        Assert.Equal("tenant-1", entity.TenantId);
    }

    private sealed class SoftDeletableOrder : Entity<Guid>, ISoftDeletable
    {
        public SoftDeletableOrder(Guid id)
            : base(id)
        {
        }

        public bool IsDeleted { get; private set; }

        public DateTimeOffset? DeletedOnUtc { get; private set; }

        public string? DeletedBy { get; private set; }

        public void Delete(string deletedBy, DateTimeOffset deletedOnUtc)
        {
            IsDeleted = true;
            DeletedBy = deletedBy;
            DeletedOnUtc = deletedOnUtc;
        }
    }

    private sealed class TenantOrder : Entity<Guid>, IHasTenant
    {
        public TenantOrder(Guid id, string tenantId)
            : base(id)
        {
            TenantId = tenantId;
        }

        public string TenantId { get; }
    }
}
