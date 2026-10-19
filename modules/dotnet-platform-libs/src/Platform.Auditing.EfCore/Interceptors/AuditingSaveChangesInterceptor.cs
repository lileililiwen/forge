using System.Collections.Generic;
using System.Linq;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.ChangeTracking;
using Microsoft.EntityFrameworkCore.Diagnostics;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Platform.Auditing.Contracts;
using Platform.Auditing.Contracts.Common;
using Platform.Core.Time;

namespace Platform.Auditing.EfCore.Interceptors;

/// <summary>
/// Opt-in EF Core interceptor that captures changes for entities implementing
/// <see cref="IAuditedEntity"/>. It masks sensitive values by property name before publishing a
/// normalized <c>entity</c> audit event through the registered <see cref="IAuditRecorder"/>. The
/// interceptor is fail-open: when capture is disabled, or publishing throws, it does nothing and
/// never blocks the save.
/// </summary>
public sealed class AuditingSaveChangesInterceptor : SaveChangesInterceptor
{
    private readonly IAuditRecorder _recorder;
    private readonly IAuditMasker _masker;
    private readonly IClock _clock;
    private readonly AuditOptions _options;
    private readonly ILogger<AuditingSaveChangesInterceptor>? _logger;

    /// <summary>Initializes a new interceptor with its dependencies.</summary>
    /// <param name="recorder">The audit recorder used to publish captured events.</param>
    /// <param name="masker">The masker applied to property values before publishing.</param>
    /// <param name="clock">The clock used to stamp events.</param>
    /// <param name="options">The audit options controlling capture.</param>
    /// <param name="logger">An optional logger.</param>
    public AuditingSaveChangesInterceptor(
        IAuditRecorder recorder,
        IAuditMasker masker,
        IClock clock,
        IOptions<AuditOptions> options,
        ILogger<AuditingSaveChangesInterceptor>? logger = null)
    {
        _recorder = recorder;
        _masker = masker;
        _clock = clock;
        _options = options.Value;
        _logger = logger;
    }

    /// <inheritdoc />
    public override InterceptionResult<int> SavingChanges(DbContextEventData eventData, InterceptionResult<int> result)
    {
        Capture(eventData.Context);
        return base.SavingChanges(eventData, result);
    }

    /// <inheritdoc />
    public override ValueTask<InterceptionResult<int>> SavingChangesAsync(
        DbContextEventData eventData,
        InterceptionResult<int> result,
        CancellationToken cancellationToken = default)
    {
        Capture(eventData.Context);
        return base.SavingChangesAsync(eventData, result, cancellationToken);
    }

    private void Capture(DbContext? context)
    {
        if (context is null) return;
        if (!_options.EnableEntityCapture) return;

        foreach (var entry in context.ChangeTracker.Entries())
        {
            if (entry.Entity is not IAuditedEntity) continue;
            if (entry.State is EntityState.Detached or EntityState.Unchanged) continue;
            try
            {
                PublishEntryAsync(entry).GetAwaiter().GetResult();
            }
            catch (Exception ex)
            {
#pragma warning disable CA1848 // Logger source-generator delegates are not used in the capture adapter.
                _logger?.LogWarning(ex, "Audit entity capture failed for {EntityType}; skipped.", entry.Entity.GetType().Name);
#pragma warning restore CA1848
            }
        }
    }

    private Task PublishEntryAsync(EntityEntry entry)
    {
        var entityType = entry.Entity.GetType().Name;
        var keyValues = entry.Metadata.FindPrimaryKey()?.Properties
            .Select(p => DefaultAuditMasker.ToStringValue(entry.Property(p.Name).CurrentValue))
            .ToArray() ?? Array.Empty<string>();
        var entityKey = string.Join(",", keyValues);

        var changes = new List<EntityAuditChange>();
        switch (entry.State)
        {
            case EntityState.Added:
                foreach (var prop in ScalarProperties(entry))
                {
                    var current = _masker.Mask(prop.Name, DefaultAuditMasker.ToStringValue(entry.Property(prop.Name).CurrentValue));
                    changes.Add(new EntityAuditChange(prop.Name, EntityAuditChangeKind.Added, null, current));
                }

                break;
            case EntityState.Modified:
                foreach (var prop in entry.Properties.Where(p => p.IsModified))
                {
                    if (!IsScalar(prop.Metadata)) continue;
                    var original = _masker.Mask(prop.Metadata.Name, DefaultAuditMasker.ToStringValue(prop.OriginalValue));
                    var current = _masker.Mask(prop.Metadata.Name, DefaultAuditMasker.ToStringValue(prop.CurrentValue));
                    changes.Add(new EntityAuditChange(prop.Metadata.Name, EntityAuditChangeKind.Modified, original, current));
                }

                break;
            case EntityState.Deleted:
                foreach (var prop in ScalarProperties(entry))
                {
                    var original = _masker.Mask(prop.Name, DefaultAuditMasker.ToStringValue(entry.Property(prop.Name).OriginalValue));
                    changes.Add(new EntityAuditChange(prop.Name, EntityAuditChangeKind.Deleted, original, null));
                }

                break;
        }

        var operation = entry.State switch
        {
            EntityState.Added => "created",
            EntityState.Deleted => "deleted",
            _ => "updated",
        };

        var metadata = new Dictionary<string, string>
        {
            ["entity.type"] = entityType,
            ["entity.key"] = entityKey,
            ["entity.operation"] = operation,
        };
        foreach (var change in changes)
        {
            metadata[$"entity.change.{change.Property}"] = FormatChange(change);
        }

        var auditEvent = AuditEvent.Create($"entity.{operation}", "entity", AuditOutcome.Success, _clock.UtcNow)
            .WithSeverity(AuditSeverity.Information)
            .WithMetadata(metadata);

        return _recorder.RecordAsync(auditEvent);
    }

    private static bool IsScalar(Microsoft.EntityFrameworkCore.Metadata.IProperty property)
        => property.ClrType == typeof(string) || property.ClrType.IsValueType;

    private static IEnumerable<Microsoft.EntityFrameworkCore.Metadata.IProperty> ScalarProperties(EntityEntry entry)
        => entry.Metadata.GetProperties().Where(IsScalar);

    private static string FormatChange(EntityAuditChange change) => change.Kind switch
    {
        EntityAuditChangeKind.Added => $"set:{change.NewValue}",
        EntityAuditChangeKind.Deleted => $"removed:{change.OldValue}",
        _ => $"{change.OldValue}->{change.NewValue}",
    };
}
