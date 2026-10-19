using System.Data;
using Microsoft.EntityFrameworkCore;
using Platform.Core.Time;
using Platform.Eventing.Contracts;

namespace Platform.Eventing.EfCore;

/// <summary>EF Core outbox store using application-owned DbContext instances.</summary>
public sealed class EfCoreOutboxStore : IOutboxStore
{
    private readonly DbContext _db;
    private readonly IClock _clock;
    private readonly DurableEventingOptions _options;

    /// <summary>Creates an EF Core outbox store.</summary>
    public EfCoreOutboxStore(DbContext db, IClock clock, DurableEventingOptions? options = null)
    {
        ArgumentNullException.ThrowIfNull(db);
        ArgumentNullException.ThrowIfNull(clock);
        _db = db;
        _clock = clock;
        _options = options ?? new DurableEventingOptions();
        _options.Validate();
    }

    /// <inheritdoc />
    public async Task AddAsync(OutboxMessage message, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(message);
        var set = _db.Set<PlatformOutboxMessage>();
        if (!await set.AnyAsync(item => item.MessageId == message.MessageId, cancellationToken).ConfigureAwait(false))
        {
            set.Add(ToEntity(message));
            await _db.SaveChangesAsync(cancellationToken).ConfigureAwait(false);
        }
    }

    /// <inheritdoc />
    public async Task<IReadOnlyList<OutboxMessage>> ClaimAsync(DateTimeOffset now, string leaseOwnerId, TimeSpan leaseDuration, int maximum, CancellationToken cancellationToken = default)
    {
        ValidateClaim(leaseOwnerId, leaseDuration, maximum);
        await using var transaction = await _db.Database.BeginTransactionAsync(IsolationLevel.Serializable, cancellationToken).ConfigureAwait(false);
        var entities = await _db.Set<PlatformOutboxMessage>()
            .Where(item => item.State == DurableMessageState.Pending
                || item.State == DurableMessageState.RetryableFailure
                || item.State == DurableMessageState.Leased)
            .Take(maximum)
            .ToListAsync(cancellationToken)
            .ConfigureAwait(false);

        var eligible = entities
            .Where(item => item.State == DurableMessageState.Pending
                || (item.State == DurableMessageState.RetryableFailure && item.NextAttemptAt is not null && item.NextAttemptAt <= now)
                || (item.State == DurableMessageState.Leased && item.LeaseExpiresAt is not null && item.LeaseExpiresAt <= now))
            .OrderBy(item => item.CreatedAt)
            .Take(maximum)
            .ToArray();

        foreach (var entity in eligible)
        {
            entity.State = DurableMessageState.Leased;
            entity.AttemptCount++;
            entity.NextAttemptAt = null;
            entity.LeaseOwnerId = leaseOwnerId;
            entity.LeaseExpiresAt = now.Add(leaseDuration);
        }

        await _db.SaveChangesAsync(cancellationToken).ConfigureAwait(false);
        await transaction.CommitAsync(cancellationToken).ConfigureAwait(false);
        return eligible.Select(ToContract).ToArray();
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
    public async Task<OutboxMessage?> GetAsync(string messageId, CancellationToken cancellationToken = default)
    {
        var entity = await _db.Set<PlatformOutboxMessage>().AsNoTracking().SingleOrDefaultAsync(item => item.MessageId == messageId, cancellationToken).ConfigureAwait(false);
        return entity is null ? null : ToContract(entity);
    }

    private async Task<PlatformOutboxMessage> GetOwnedAsync(string messageId, string leaseOwnerId, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(messageId)) throw new ArgumentException("Message id is required.", nameof(messageId));
        if (string.IsNullOrWhiteSpace(leaseOwnerId)) throw new ArgumentException("Lease owner id is required.", nameof(leaseOwnerId));
        var entity = await _db.Set<PlatformOutboxMessage>().SingleOrDefaultAsync(item => item.MessageId == messageId, cancellationToken).ConfigureAwait(false)
            ?? throw new KeyNotFoundException(messageId);
        if (entity.State != DurableMessageState.Leased || !string.Equals(entity.LeaseOwnerId, leaseOwnerId, StringComparison.Ordinal))
            throw new InvalidOperationException("The message is not leased by this worker.");
        return entity;
    }

    private static PlatformOutboxMessage ToEntity(OutboxMessage message) => new()
    {
        MessageId = message.MessageId,
        PayloadType = message.Envelope.PayloadType,
        PayloadJson = message.Envelope.PayloadJson,
        OccurredAt = message.Envelope.OccurredAt,
        CreatedAt = message.CreatedAt,
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

    private static OutboxMessage ToContract(PlatformOutboxMessage message) => OutboxMessage.Create(
        new DurableEventEnvelope(message.MessageId, message.PayloadType, message.PayloadJson, message.OccurredAt, message.TenantId, message.CorrelationId), message.CreatedAt)
        .WithPersistedState(
            message.State,
            message.AttemptCount,
            message.NextAttemptAt,
            message.LeaseOwnerId,
            message.LeaseExpiresAt,
            message.LastFailureCode is null || message.LastFailureMessage is null
                ? null
                : new DurableDispatchFailure(message.LastFailureCode, message.LastFailureMessage, message.LastFailurePermanent));

    private static void ValidateClaim(string owner, TimeSpan leaseDuration, int maximum)
    {
        if (string.IsNullOrWhiteSpace(owner)) throw new ArgumentException("Lease owner id is required.", nameof(owner));
        ArgumentOutOfRangeException.ThrowIfLessThanOrEqual(leaseDuration, TimeSpan.Zero, nameof(leaseDuration));
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(maximum, nameof(maximum));
    }
}
