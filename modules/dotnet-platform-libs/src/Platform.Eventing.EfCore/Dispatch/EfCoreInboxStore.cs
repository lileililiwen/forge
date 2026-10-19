using System.Data;
using Microsoft.EntityFrameworkCore;
using Platform.Core.Time;
using Platform.Eventing.Contracts;

namespace Platform.Eventing.EfCore;

/// <summary>EF Core inbox store using application-owned DbContext instances.</summary>
public sealed class EfCoreInboxStore : IInboxStore
{
    private readonly DbContext _db;
    private readonly IClock _clock;
    private readonly DurableEventingOptions _options;

    /// <summary>Creates an EF Core inbox store.</summary>
    public EfCoreInboxStore(DbContext db, IClock clock, DurableEventingOptions? options = null)
    {
        ArgumentNullException.ThrowIfNull(db);
        ArgumentNullException.ThrowIfNull(clock);
        _db = db;
        _clock = clock;
        _options = options ?? new DurableEventingOptions();
        _options.Validate();
    }

    /// <inheritdoc />
    public async Task<InboxClaimResult> TryClaimAsync(InboxMessage message, DateTimeOffset now, string leaseOwnerId, TimeSpan leaseDuration, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(message);
        if (string.IsNullOrWhiteSpace(leaseOwnerId)) throw new ArgumentException("Lease owner id is required.", nameof(leaseOwnerId));
        ArgumentOutOfRangeException.ThrowIfLessThanOrEqual(leaseDuration, TimeSpan.Zero, nameof(leaseDuration));
        await using var transaction = await _db.Database.BeginTransactionAsync(IsolationLevel.Serializable, cancellationToken).ConfigureAwait(false);
        var set = _db.Set<PlatformInboxMessage>();
        var entity = await set.SingleOrDefaultAsync(item => item.MessageId == message.MessageId, cancellationToken).ConfigureAwait(false);
        if (entity is not null && entity.State == DurableMessageState.Succeeded)
        {
            await transaction.CommitAsync(cancellationToken).ConfigureAwait(false);
            return new InboxClaimResult(InboxClaimDecision.Duplicate, ToContract(entity));
        }

        if (entity is not null && entity.State == DurableMessageState.Leased && entity.LeaseExpiresAt > now)
        {
            await transaction.CommitAsync(cancellationToken).ConfigureAwait(false);
            return new InboxClaimResult(InboxClaimDecision.Busy, ToContract(entity));
        }

        entity ??= ToEntity(message);
        if (entity.MessageId == message.MessageId && !set.Local.Contains(entity))
            set.Add(entity);
        entity.State = DurableMessageState.Leased;
        entity.AttemptCount++;
        entity.NextAttemptAt = null;
        entity.LeaseOwnerId = leaseOwnerId;
        entity.LeaseExpiresAt = now.Add(leaseDuration);
        await _db.SaveChangesAsync(cancellationToken).ConfigureAwait(false);
        await transaction.CommitAsync(cancellationToken).ConfigureAwait(false);
        return new InboxClaimResult(InboxClaimDecision.Claimed, ToContract(entity));
    }

    /// <inheritdoc />
    public async Task MarkSucceededAsync(string messageId, string leaseOwnerId, CancellationToken cancellationToken = default)
    {
        var entity = await GetOwnedAsync(messageId, leaseOwnerId, cancellationToken).ConfigureAwait(false);
        entity.State = DurableMessageState.Succeeded;
        entity.LeaseOwnerId = null;
        entity.LeaseExpiresAt = null;
        await _db.SaveChangesAsync(cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc />
    public async Task MarkFailedAsync(string messageId, string leaseOwnerId, DurableDispatchFailure failure, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(failure);
        var entity = await GetOwnedAsync(messageId, leaseOwnerId, cancellationToken).ConfigureAwait(false);
        var deadLetter = failure.Permanent || entity.AttemptCount >= _options.MaxAttempts;
        entity.State = deadLetter ? DurableMessageState.DeadLetter : DurableMessageState.RetryableFailure;
        entity.NextAttemptAt = deadLetter ? null : _clock.UtcNow.Add(_options.GetRetryDelay(entity.AttemptCount));
        entity.LeaseOwnerId = null;
        entity.LeaseExpiresAt = null;
        entity.LastFailureCode = failure.Code;
        entity.LastFailureMessage = failure.Message;
        entity.LastFailurePermanent = failure.Permanent;
        await _db.SaveChangesAsync(cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc />
    public async Task<InboxMessage?> GetAsync(string messageId, CancellationToken cancellationToken = default)
    {
        var entity = await _db.Set<PlatformInboxMessage>().AsNoTracking().SingleOrDefaultAsync(item => item.MessageId == messageId, cancellationToken).ConfigureAwait(false);
        return entity is null ? null : ToContract(entity);
    }

    private async Task<PlatformInboxMessage> GetOwnedAsync(string messageId, string leaseOwnerId, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(messageId)) throw new ArgumentException("Message id is required.", nameof(messageId));
        if (string.IsNullOrWhiteSpace(leaseOwnerId)) throw new ArgumentException("Lease owner id is required.", nameof(leaseOwnerId));
        var entity = await _db.Set<PlatformInboxMessage>().SingleOrDefaultAsync(item => item.MessageId == messageId, cancellationToken).ConfigureAwait(false)
            ?? throw new KeyNotFoundException(messageId);
        if (entity.State != DurableMessageState.Leased || !string.Equals(entity.LeaseOwnerId, leaseOwnerId, StringComparison.Ordinal))
            throw new InvalidOperationException("The message is not leased by this worker.");
        return entity;
    }

    private static PlatformInboxMessage ToEntity(InboxMessage message) => new()
    {
        MessageId = message.MessageId,
        PayloadType = message.Envelope.PayloadType,
        PayloadJson = message.Envelope.PayloadJson,
        OccurredAt = message.Envelope.OccurredAt,
        ReceivedAt = message.ReceivedAt,
        TenantId = message.Envelope.TenantId,
        CorrelationId = message.Envelope.CorrelationId,
        State = message.State,
        AttemptCount = message.AttemptCount,
        NextAttemptAt = message.NextAttemptAt,
        LeaseOwnerId = message.LeaseOwnerId,
        LeaseExpiresAt = message.LeaseExpiresAt,
        LastFailureCode = message.LastFailure?.Code,
        LastFailureMessage = message.LastFailure?.Message,
        LastFailurePermanent = message.LastFailure?.Permanent ?? false,
    };

    private static InboxMessage ToContract(PlatformInboxMessage message) => InboxMessage.Create(
        new DurableEventEnvelope(message.MessageId, message.PayloadType, message.PayloadJson, message.OccurredAt, message.TenantId, message.CorrelationId), message.ReceivedAt)
        .WithPersistedState(
            message.State,
            message.AttemptCount,
            message.NextAttemptAt,
            message.LeaseOwnerId,
            message.LeaseExpiresAt,
            message.LastFailureCode is null || message.LastFailureMessage is null
                ? null
                : new DurableDispatchFailure(message.LastFailureCode, message.LastFailureMessage, message.LastFailurePermanent));
}
