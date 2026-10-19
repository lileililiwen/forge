using Platform.Core.Time;
using Platform.Quota.Contracts;

namespace Platform.Quota.Stores;

/// <summary>Thread-safe in-memory quota store for local use and deterministic tests.</summary>
public sealed class InMemoryQuotaStore : IQuotaStore
{
    private readonly object _gate = new();
    private readonly Dictionary<BucketKey, Bucket> _buckets = new();
    private readonly Dictionary<string, ReservationEntry> _operations = new(StringComparer.Ordinal);
    private readonly IClock _clock;
    private readonly QuotaOptions _options;

    /// <summary>Creates an empty quota store.</summary>
    public InMemoryQuotaStore(IClock clock, QuotaOptions? options = null, long initialConsumed = 0)
    {
        _clock = clock ?? throw new ArgumentNullException(nameof(clock));
        _options = options ?? new QuotaOptions();
        _options.Validate();
        if (initialConsumed < 0 || initialConsumed > _options.MaximumAmount) throw new ArgumentOutOfRangeException(nameof(initialConsumed));
        InitialConsumed = initialConsumed;
    }

    /// <summary>Initial settled amount applied to newly created buckets.</summary>
    public long InitialConsumed { get; }

    /// <summary>Returns a retained reservation for deterministic test or reconciliation inspection.</summary>
    public QuotaReservation? GetReservation(string operationKey)
    {
        var key = new QuotaOperationKey(operationKey);
        lock (_gate) return _operations.TryGetValue(key.Value, out var entry) ? entry.Reservation : null;
    }

    /// <inheritdoc />
    public Task<QuotaDecision> CheckAsync(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, long requested, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        ValidateRequest(subject, resource, window, limit, requested);
        lock (_gate)
        {
            var bucket = GetBucket(subject, resource, window, limit);
            Expire(bucket);
            return Task.FromResult(Decision(subject, resource, window, limit, bucket, requested));
        }
    }

    /// <inheritdoc />
    public Task<QuotaLifecycleResult> ReserveAsync(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, string operationKey, long amount, TimeSpan? reservationLifetime = null, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        ValidateRequest(subject, resource, window, limit, amount);
        var key = new QuotaOperationKey(operationKey);
        if (key.Value.Length > _options.MaximumOperationKeyLength) throw new ArgumentException("The operation key exceeds the configured limit.", nameof(operationKey));
        var lifetime = reservationLifetime ?? TimeSpan.FromHours(1);
        if (lifetime <= TimeSpan.Zero || lifetime > _options.MaximumReservationLifetime) throw new ArgumentOutOfRangeException(nameof(reservationLifetime));
        lock (_gate)
        {
            if (_operations.TryGetValue(key.Value, out var existing)) return Task.FromResult(existing.ToResult());
            var bucket = GetBucket(subject, resource, window, limit);
            Expire(bucket);
            var decision = Decision(subject, resource, window, limit, bucket, amount);
            if (!decision.Allowed) return Task.FromResult(new QuotaLifecycleResult(QuotaLifecycleStatus.Denied, Decision: decision));
            bucket.Reserved += amount;
            var reservation = new QuotaReservation(key, subject, resource, window, amount, _clock.UtcNow.Add(lifetime), QuotaReservationStatus.Reserved);
            var entry = new ReservationEntry(reservation, bucket, decision);
            _operations.Add(key.Value, entry);
            return Task.FromResult(entry.ToResult());
        }
    }

    /// <inheritdoc />
    public Task<QuotaLifecycleResult> SettleAsync(string operationKey, CancellationToken cancellationToken = default)
        => Transition(operationKey, QuotaReservationStatus.Settled, cancellationToken);

    /// <inheritdoc />
    public Task<QuotaLifecycleResult> ReleaseAsync(string operationKey, CancellationToken cancellationToken = default)
        => Transition(operationKey, QuotaReservationStatus.Released, cancellationToken);

    /// <inheritdoc />
    public Task<QuotaSnapshot> GetSnapshotAsync(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        ValidateRequest(subject, resource, window, limit, 0);
        lock (_gate)
        {
            var bucket = GetBucket(subject, resource, window, limit);
            Expire(bucket);
            return Task.FromResult(new QuotaSnapshot(subject, resource, window, limit, bucket.Consumed, bucket.Reserved));
        }
    }

    private Task<QuotaLifecycleResult> Transition(string operationKey, QuotaReservationStatus target, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var key = new QuotaOperationKey(operationKey);
        lock (_gate)
        {
            if (!_operations.TryGetValue(key.Value, out var entry)) return Task.FromResult(new QuotaLifecycleResult(QuotaLifecycleStatus.NotFound));
            if (entry.Reservation.Status == target) return Task.FromResult(new QuotaLifecycleResult(target == QuotaReservationStatus.Settled ? QuotaLifecycleStatus.AlreadySettled : QuotaLifecycleStatus.AlreadyReleased, entry.Reservation, Decision: entry.Decision));
            if (entry.Reservation.Status != QuotaReservationStatus.Reserved) return Task.FromResult(new QuotaLifecycleResult(QuotaLifecycleStatus.InvalidTransition, entry.Reservation, new QuotaFailure("quota.invalid_transition", "The quota reservation is no longer active."), entry.Decision));
            if (entry.Reservation.ExpiresAt <= _clock.UtcNow) { entry.Bucket.Reserved -= entry.Reservation.Amount; entry.Reservation = entry.Reservation with { Status = QuotaReservationStatus.Expired }; return Task.FromResult(new QuotaLifecycleResult(QuotaLifecycleStatus.InvalidTransition, entry.Reservation, new QuotaFailure("quota.reservation_expired", "The quota reservation has expired."), entry.Decision)); }
            entry.Bucket.Reserved -= entry.Reservation.Amount;
            if (target == QuotaReservationStatus.Settled) entry.Bucket.Consumed += entry.Reservation.Amount;
            entry.Reservation = entry.Reservation with { Status = target };
            return Task.FromResult(new QuotaLifecycleResult(target == QuotaReservationStatus.Settled ? QuotaLifecycleStatus.Settled : QuotaLifecycleStatus.Released, entry.Reservation, Decision: entry.Decision));
        }
    }

    private Bucket GetBucket(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit)
    {
        var key = new BucketKey(subject, resource, window);
        if (!_buckets.TryGetValue(key, out var bucket)) { bucket = new Bucket(InitialConsumed) { Limit = limit }; _buckets.Add(key, bucket); }
        if (bucket.Limit != limit) throw new ArgumentException("The limit for an existing quota bucket cannot change.", nameof(limit));
        return bucket;
    }

    private void Expire(Bucket bucket)
    {
        foreach (var entry in _operations.Values.Where(item => ReferenceEquals(item.Bucket, bucket) && item.Reservation.Status == QuotaReservationStatus.Reserved && item.Reservation.ExpiresAt <= _clock.UtcNow).ToArray())
        {
            bucket.Reserved -= entry.Reservation.Amount;
            entry.Reservation = entry.Reservation with { Status = QuotaReservationStatus.Expired };
        }
    }

    private static QuotaDecision Decision(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, Bucket bucket, long requested)
        => new(bucket.Consumed + bucket.Reserved + requested <= limit, subject, resource, window, limit, bucket.Consumed, bucket.Reserved, requested);

    private void ValidateRequest(QuotaSubject subject, QuotaResource resource, QuotaWindow window, long limit, long amount)
    {
        window.Validate();
        if (limit < 0 || limit > _options.MaximumAmount) throw new ArgumentOutOfRangeException(nameof(limit));
        if (amount < 0 || amount > _options.MaximumAmount) throw new ArgumentOutOfRangeException(nameof(amount));
    }

    private readonly record struct BucketKey(QuotaSubject Subject, QuotaResource Resource, QuotaWindow Window);
    private sealed class Bucket(long consumed) { public long Limit { get; set; } = -1; public long Consumed { get; set; } = consumed; public long Reserved { get; set; } }
    private sealed class ReservationEntry(QuotaReservation reservation, Bucket bucket, QuotaDecision decision)
    {
        public QuotaReservation Reservation { get; set; } = reservation;
        public Bucket Bucket { get; } = bucket;
        public QuotaDecision Decision { get; } = decision;
        public QuotaLifecycleResult ToResult() => new(Reservation.Status switch { QuotaReservationStatus.Reserved => QuotaLifecycleStatus.Reserved, QuotaReservationStatus.Settled => QuotaLifecycleStatus.AlreadySettled, QuotaReservationStatus.Released => QuotaLifecycleStatus.AlreadyReleased, _ => QuotaLifecycleStatus.InvalidTransition }, Reservation, Decision: Decision);
    }
}
