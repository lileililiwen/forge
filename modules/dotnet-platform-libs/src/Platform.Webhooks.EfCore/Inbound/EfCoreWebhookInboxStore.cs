using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;
using Platform.Core.Time;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Inbound;

namespace Platform.Webhooks.EfCore.Inbound;

/// <summary>EF Core entity used to persist <see cref="WebhookInboxMessage"/>. Owns its own parameterless constructor for materialization.</summary>
public sealed class EfWebhookInboxEntry
{
    /// <summary>Stable composite key.</summary>
    public string ReplayKey { get; set; } = string.Empty;
    /// <summary>Event identifier.</summary>
    public string EventId { get; set; } = string.Empty;
    /// <summary>Receive time.</summary>
    public DateTimeOffset ReceivedAt { get; set; }
    /// <summary>Current state.</summary>
    public WebhookInboxState State { get; set; }
    /// <summary>Number of attempts.</summary>
    public int AttemptCount { get; set; }
    /// <summary>Next eligible attempt time.</summary>
    public DateTimeOffset? NextAttemptAt { get; set; }
    /// <summary>Current lease owner.</summary>
    public string? LeaseOwnerId { get; set; }
    /// <summary>Current lease expiry.</summary>
    public DateTimeOffset? LeaseExpiresAt { get; set; }
    /// <summary>Last failure code, message, and transient flag (pipe-delimited).</summary>
    public string? LastFailure { get; set; }
    /// <summary>Materializes an entry from a domain message.</summary>
    public static EfWebhookInboxEntry FromMessage(WebhookInboxMessage message)
    {
        ArgumentNullException.ThrowIfNull(message);
        return new EfWebhookInboxEntry
        {
            ReplayKey = message.ReplayKey,
            EventId = message.EventId,
            ReceivedAt = message.ReceivedAt,
            State = message.State,
            AttemptCount = message.AttemptCount,
            NextAttemptAt = message.NextAttemptAt,
            LeaseOwnerId = message.LeaseOwnerId,
            LeaseExpiresAt = message.LeaseExpiresAt,
            LastFailure = message.LastFailure == null ? null : message.LastFailure.Code + "|" + message.LastFailure.Message + "|" + (message.LastFailure.Transient ? "1" : "0"),
        };
    }
    /// <summary>Materializes a domain message from this entry.</summary>
    public WebhookInboxMessage ToMessage()
    {
        var providerValue = ReplayKey.Split('\u001f', 2)[0];
        var eventIdValue = ReplayKey.Split('\u001f', 2)[1];
        var baseMessage = WebhookInboxMessage.Create(new WebhookProviderId(providerValue), eventIdValue, ReceivedAt);
        return baseMessage.WithPersistedState(State, AttemptCount, NextAttemptAt, LeaseOwnerId, LeaseExpiresAt, ParseFailure(LastFailure));
    }

    private static WebhookFailure? ParseFailure(string? value)
    {
        if (string.IsNullOrEmpty(value)) return null;
        var parts = value.Split('|');
        if (parts.Length != 3) return null;
        return new WebhookFailure(parts[0], parts[1], parts[2] == "1");
    }
}

/// <summary>EF Core configuration for <see cref="EfWebhookInboxEntry"/>.</summary>
public sealed class WebhookInboxEntityConfiguration : IEntityTypeConfiguration<EfWebhookInboxEntry>
{
    /// <inheritdoc />
    public void Configure(EntityTypeBuilder<EfWebhookInboxEntry> builder)
    {
        ArgumentNullException.ThrowIfNull(builder);
        builder.ToTable("webhook_inbox");
        builder.HasKey(m => m.ReplayKey);
        builder.Property(m => m.ReplayKey).HasMaxLength(320).IsRequired();
        builder.Property(m => m.EventId).HasMaxLength(256).IsRequired();
        builder.Property(m => m.ReceivedAt).IsRequired();
        builder.Property(m => m.State).HasConversion<int>().IsRequired();
        builder.Property(m => m.AttemptCount).IsRequired();
        builder.Property(m => m.NextAttemptAt);
        builder.Property(m => m.LeaseOwnerId).HasMaxLength(128);
        builder.Property(m => m.LeaseExpiresAt);
        builder.Property(m => m.LastFailure);
    }
}

/// <summary>EF Core <see cref="IWebhookInboxStore"/> adapter. The application owns the <see cref="DbContext"/>.</summary>
public sealed class EfCoreWebhookInboxStore : IWebhookInboxStore
{
    private readonly DbContext _context;
    private readonly IClock _clock;
    private readonly DbSet<EfWebhookInboxEntry> _set;

    /// <summary>Creates a store using the supplied <see cref="DbContext"/>.</summary>
    public EfCoreWebhookInboxStore(DbContext context, IClock clock)
    {
        _context = context ?? throw new ArgumentNullException(nameof(context));
        _clock = clock ?? throw new ArgumentNullException(nameof(clock));
        _set = _context.Set<EfWebhookInboxEntry>();
    }

    /// <inheritdoc />
    public async Task<WebhookInboxClaimResult> TryClaimAsync(WebhookInboxMessage message, IClock clock, string leaseOwnerId, TimeSpan leaseDuration, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(message);
        ArgumentNullException.ThrowIfNull(clock);
        if (string.IsNullOrWhiteSpace(leaseOwnerId)) throw new ArgumentException("A lease owner is required.", nameof(leaseOwnerId));
        if (leaseDuration <= TimeSpan.Zero || leaseDuration > TimeSpan.FromHours(24)) throw new ArgumentOutOfRangeException(nameof(leaseDuration));
        var now = clock.UtcNow;
        var existing = await _set.FindAsync(new object?[] { message.ReplayKey }, cancellationToken).ConfigureAwait(false);
        if (existing is not null)
        {
            var existingMessage = existing.ToMessage();
            if (existing.State == WebhookInboxState.Completed) return new WebhookInboxClaimResult(WebhookInboxClaimStatus.Duplicate, existingMessage);
            if (existing.State == WebhookInboxState.Leased && existing.LeaseExpiresAt is { } lease && lease > now && !string.Equals(existing.LeaseOwnerId, leaseOwnerId, StringComparison.Ordinal))
                return new WebhookInboxClaimResult(WebhookInboxClaimStatus.Busy, existingMessage);
            if (existing.NextAttemptAt is { } next && next > now) return new WebhookInboxClaimResult(WebhookInboxClaimStatus.Busy, existingMessage);
            existing.State = WebhookInboxState.Leased;
            existing.AttemptCount += 1;
            existing.NextAttemptAt = null;
            existing.LeaseOwnerId = leaseOwnerId;
            existing.LeaseExpiresAt = now.Add(leaseDuration);
            await _context.SaveChangesAsync(cancellationToken).ConfigureAwait(false);
            return new WebhookInboxClaimResult(WebhookInboxClaimStatus.Claimed, existing.ToMessage());
        }
        var newEntry = EfWebhookInboxEntry.FromMessage(message.WithPersistedState(WebhookInboxState.Leased, 1, null, leaseOwnerId, now.Add(leaseDuration), null));
        await _set.AddAsync(newEntry, cancellationToken).ConfigureAwait(false);
        await _context.SaveChangesAsync(cancellationToken).ConfigureAwait(false);
        return new WebhookInboxClaimResult(WebhookInboxClaimStatus.Claimed, newEntry.ToMessage());
    }

    /// <inheritdoc />
    public async Task MarkCompletedAsync(string replayKey, string leaseOwnerId, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(replayKey)) throw new ArgumentException("A replay key is required.", nameof(replayKey));
        if (string.IsNullOrWhiteSpace(leaseOwnerId)) throw new ArgumentException("A lease owner is required.", nameof(leaseOwnerId));
        var existing = await _set.FindAsync(new object?[] { replayKey }, cancellationToken).ConfigureAwait(false);
        if (existing is null) return;
        if (!string.Equals(existing.LeaseOwnerId, leaseOwnerId, StringComparison.Ordinal)) throw new InvalidOperationException("The lease owner does not match the current lease.");
        existing.State = WebhookInboxState.Completed;
        existing.NextAttemptAt = null;
        existing.LeaseOwnerId = null;
        existing.LeaseExpiresAt = null;
        existing.LastFailure = null;
        await _context.SaveChangesAsync(cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc />
    public async Task MarkFailedAsync(string replayKey, string leaseOwnerId, WebhookFailure failure, DateTimeOffset? nextAttemptAt, bool deadLettered, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(replayKey)) throw new ArgumentException("A replay key is required.", nameof(replayKey));
        if (string.IsNullOrWhiteSpace(leaseOwnerId)) throw new ArgumentException("A lease owner is required.", nameof(leaseOwnerId));
        ArgumentNullException.ThrowIfNull(failure);
        var existing = await _set.FindAsync(new object?[] { replayKey }, cancellationToken).ConfigureAwait(false);
        if (existing is null) return;
        if (!string.Equals(existing.LeaseOwnerId, leaseOwnerId, StringComparison.Ordinal)) throw new InvalidOperationException("The lease owner does not match the current lease.");
        existing.State = deadLettered ? WebhookInboxState.DeadLettered : WebhookInboxState.Pending;
        existing.NextAttemptAt = deadLettered ? null : nextAttemptAt;
        existing.LeaseOwnerId = deadLettered ? null : leaseOwnerId;
        existing.LeaseExpiresAt = deadLettered ? null : existing.LeaseExpiresAt;
        existing.LastFailure = failure.Code + "|" + failure.Message + "|" + (failure.Transient ? "1" : "0");
        await _context.SaveChangesAsync(cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc />
    public async Task<WebhookInboxMessage?> GetAsync(string replayKey, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(replayKey)) throw new ArgumentException("A replay key is required.", nameof(replayKey));
        var existing = await _set.FindAsync(new object?[] { replayKey }, cancellationToken).ConfigureAwait(false);
        return existing?.ToMessage();
    }
}
