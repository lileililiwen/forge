using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.ChangeTracking;
using Microsoft.EntityFrameworkCore.Diagnostics;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Persistence.EfCore.Audit;
using Platform.Persistence.EfCore.Deletion;

namespace Platform.Persistence.EfCore.Interceptors;

/// <summary>Applies audit and soft-delete metadata when explicitly added to a context.</summary>
public sealed class PlatformSaveChangesInterceptor : SaveChangesInterceptor
{
    private readonly IClock _clock;
    private readonly IActorAccessor _actor;
    private readonly PersistenceOptions? _options;

    /// <summary>Creates an interceptor; direct construction enables both behaviors.</summary>
    public PlatformSaveChangesInterceptor(IClock clock, IActorAccessor actor, IOptions<PersistenceOptions>? options = null)
    {
        _clock = clock ?? throw new ArgumentNullException(nameof(clock));
        _actor = actor ?? throw new ArgumentNullException(nameof(actor));
        _options = options?.Value;
    }

    /// <inheritdoc />
    public override InterceptionResult<int> SavingChanges(DbContextEventData eventData, InterceptionResult<int> result)
    {
        Apply(eventData.Context);
        return base.SavingChanges(eventData, result);
    }

    /// <inheritdoc />
    public override ValueTask<InterceptionResult<int>> SavingChangesAsync(
        DbContextEventData eventData,
        InterceptionResult<int> result,
        CancellationToken cancellationToken = default)
    {
        Apply(eventData.Context);
        return base.SavingChangesAsync(eventData, result, cancellationToken);
    }

    private void Apply(DbContext? context)
    {
        if (context is null) return;
        foreach (var entry in context.ChangeTracker.Entries())
        {
            ApplySoftDelete(entry);
            ApplyAudit(entry);
        }
    }

    private void ApplyAudit(EntityEntry entry)
    {
        if ((_options is not null && !_options.EnableAuditInterception)
            || entry.Entity is not IAuditableEntity entity) return;
        if (entry.State == EntityState.Added)
        {
            entity.CreatedAt = _clock.UtcNow;
            entity.CreatedBy = _actor.SubjectId;
        }
        else if (entry.State == EntityState.Modified)
        {
            entity.UpdatedAt = _clock.UtcNow;
            entity.UpdatedBy = _actor.SubjectId;
        }
    }

    private void ApplySoftDelete(EntityEntry entry)
    {
        if ((_options is not null && !_options.EnableSoftDeleteInterception)
            || entry.State != EntityState.Deleted || entry.Entity is not ISoftDeletable entity) return;
        entity.IsDeleted = true;
        entity.DeletedAt = _clock.UtcNow;
        entity.DeletedBy = _actor.SubjectId;
        entry.State = EntityState.Modified;
    }
}
