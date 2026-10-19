using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Outbound;

namespace Platform.Webhooks.EfCore.Outbound;

/// <summary>EF Core entity used to persist <see cref="WebhookDelivery"/>.</summary>
public sealed class EfWebhookDeliveryEntry
{
    /// <summary>Delivery identifier.</summary>
    public string Id { get; set; } = string.Empty;
    /// <summary>Subscription identifier.</summary>
    public string SubscriptionId { get; set; } = string.Empty;
    /// <summary>Event type.</summary>
    public string EventType { get; set; } = string.Empty;
    /// <summary>Event identifier.</summary>
    public string EventId { get; set; } = string.Empty;
    /// <summary>Created time.</summary>
    public DateTimeOffset CreatedAt { get; set; }
    /// <summary>Delivery status.</summary>
    public WebhookDeliveryStatus Status { get; set; }
    /// <summary>Number of attempts.</summary>
    public int AttemptCount { get; set; }
    /// <summary>Next eligible attempt time.</summary>
    public DateTimeOffset? NextAttemptAt { get; set; }
    /// <summary>Last response status code.</summary>
    public int? LastResponseStatus { get; set; }
    /// <summary>Last failure (code|message|transient).</summary>
    public string? LastFailure { get; set; }
    /// <summary>Materializes a domain delivery from this entry.</summary>
    public WebhookDelivery ToDelivery() => WebhookDelivery.Create(new WebhookDeliveryId(Id), new WebhookSubscriptionId(SubscriptionId), EventType, EventId, CreatedAt)
        .WithPersistedState(Status, AttemptCount, NextAttemptAt, ParseFailure(LastFailure), LastResponseStatus);
    /// <summary>Materializes an entry from a domain delivery.</summary>
    public static EfWebhookDeliveryEntry FromDelivery(WebhookDelivery delivery)
    {
        ArgumentNullException.ThrowIfNull(delivery);
        return new EfWebhookDeliveryEntry
        {
            Id = delivery.Id.Value,
            SubscriptionId = delivery.SubscriptionId.Value,
            EventType = delivery.EventType,
            EventId = delivery.EventId,
            CreatedAt = delivery.CreatedAt,
            Status = delivery.Status,
            AttemptCount = delivery.AttemptCount,
            NextAttemptAt = delivery.NextAttemptAt,
            LastResponseStatus = delivery.LastResponseStatus,
            LastFailure = delivery.LastFailure == null ? null : delivery.LastFailure.Code + "|" + delivery.LastFailure.Message + "|" + (delivery.LastFailure.Transient ? "1" : "0"),
        };
    }
    private static WebhookFailure? ParseFailure(string? value)
    {
        if (string.IsNullOrEmpty(value)) return null;
        var parts = value.Split('|');
        if (parts.Length != 3) return null;
        return new WebhookFailure(parts[0], parts[1], parts[2] == "1");
    }
}

/// <summary>EF Core configuration for <see cref="EfWebhookDeliveryEntry"/>.</summary>
public sealed class WebhookDeliveryEntityConfiguration : IEntityTypeConfiguration<EfWebhookDeliveryEntry>
{
    /// <inheritdoc />
    public void Configure(EntityTypeBuilder<EfWebhookDeliveryEntry> builder)
    {
        ArgumentNullException.ThrowIfNull(builder);
        builder.ToTable("webhook_deliveries");
        builder.HasKey(d => d.Id);
        builder.Property(d => d.Id).HasMaxLength(128).IsRequired();
        builder.Property(d => d.SubscriptionId).HasMaxLength(128).IsRequired();
        builder.Property(d => d.EventType).HasMaxLength(128).IsRequired();
        builder.Property(d => d.EventId).HasMaxLength(256).IsRequired();
        builder.Property(d => d.CreatedAt).IsRequired();
        builder.Property(d => d.Status).HasConversion<int>().IsRequired();
        builder.Property(d => d.AttemptCount).IsRequired();
        builder.Property(d => d.NextAttemptAt);
        builder.Property(d => d.LastResponseStatus);
        builder.Property(d => d.LastFailure);
        builder.HasIndex(d => new { d.SubscriptionId, d.Status });
    }
}

/// <summary>EF Core <see cref="IWebhookDeliveryStore"/> adapter. The application owns the <see cref="DbContext"/>.</summary>
public sealed class EfCoreWebhookDeliveryStore : IWebhookDeliveryStore
{
    private readonly DbContext _context;
    private readonly DbSet<EfWebhookDeliveryEntry> _set;

    /// <summary>Creates a delivery store using the supplied <see cref="DbContext"/>.</summary>
    public EfCoreWebhookDeliveryStore(DbContext context)
    {
        _context = context ?? throw new ArgumentNullException(nameof(context));
        _set = _context.Set<EfWebhookDeliveryEntry>();
    }

    /// <inheritdoc />
    public async Task<WebhookDelivery> CreateAsync(WebhookDelivery delivery, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(delivery);
        var entry = EfWebhookDeliveryEntry.FromDelivery(delivery);
        await _set.AddAsync(entry, cancellationToken).ConfigureAwait(false);
        await _context.SaveChangesAsync(cancellationToken).ConfigureAwait(false);
        return entry.ToDelivery();
    }

    /// <inheritdoc />
    public async Task UpdateAsync(WebhookDelivery delivery, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(delivery);
        var existing = await _set.FindAsync(new object?[] { delivery.Id.Value }, cancellationToken).ConfigureAwait(false);
        if (existing is null)
        {
            await _set.AddAsync(EfWebhookDeliveryEntry.FromDelivery(delivery), cancellationToken).ConfigureAwait(false);
        }
        else
        {
            existing.SubscriptionId = delivery.SubscriptionId.Value;
            existing.EventType = delivery.EventType;
            existing.EventId = delivery.EventId;
            existing.CreatedAt = delivery.CreatedAt;
            existing.Status = delivery.Status;
            existing.AttemptCount = delivery.AttemptCount;
            existing.NextAttemptAt = delivery.NextAttemptAt;
            existing.LastResponseStatus = delivery.LastResponseStatus;
            existing.LastFailure = delivery.LastFailure == null ? null : delivery.LastFailure.Code + "|" + delivery.LastFailure.Message + "|" + (delivery.LastFailure.Transient ? "1" : "0");
        }
        await _context.SaveChangesAsync(cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc />
    public async Task<WebhookDelivery?> GetAsync(WebhookDeliveryId id, CancellationToken cancellationToken = default)
    {
        var entry = await _set.FindAsync(new object?[] { id.Value }, cancellationToken).ConfigureAwait(false);
        return entry?.ToDelivery();
    }
}
