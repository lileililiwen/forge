namespace Platform.Testing.FailureInjection;

/// <summary>
/// Outcome of a single injected failure. <see cref="Transient"/> failures
/// exhaust the configured count; <see cref="Permanent"/> failures never
/// exhaust and must be cleared with <see cref="TransientFailureInjector.Reset"/>.
/// </summary>
public enum InjectedFailureKind
{
    /// <summary>Counted, retryable failure.</summary>
    Transient = 0,

    /// <summary>Uncounted, non-retryable failure.</summary>
    Permanent = 1,
}

/// <summary>
/// One recorded injection. The label is the public, exception-message-free
/// identifier a test asserts on; tests MUST NOT compare exception text.
/// </summary>
/// <param name="Label">The configured label for the injection.</param>
/// <param name="Kind">The kind of failure injected.</param>
/// <param name="Sequence">The order in which the injector emitted this failure.</param>
public sealed record InjectedFailure(string Label, InjectedFailureKind Kind, int Sequence);

/// <summary>
/// Configurable transient/permanent failure injector for tests. Wraps a
/// delegate and emits a configured exception until its budget is exhausted.
/// Counters are observable for assertions; safe diagnostics (label, kind,
/// sequence) are returned through <see cref="InjectedFailure"/> records.
/// </summary>
public sealed class TransientFailureInjector
{
    private readonly object _gate = new();
    private readonly Dictionary<string, PendingInjection> _pending = new(StringComparer.Ordinal);
    private readonly List<InjectedFailure> _history = new();
    private int _sequence;

    /// <summary>Configures a single transient failure with the supplied label.</summary>
    public TransientFailureInjector WithTransient(string label, Exception exception)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(label);
        ArgumentNullException.ThrowIfNull(exception);
        lock (_gate)
        {
            _pending[label] = new PendingInjection(++_sequence, InjectedFailureKind.Transient, exception);
        }
        return this;
    }

    /// <summary>Configures a single permanent failure with the supplied label.</summary>
    public TransientFailureInjector WithPermanent(string label, Exception exception)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(label);
        ArgumentNullException.ThrowIfNull(exception);
        lock (_gate)
        {
            _pending[label] = new PendingInjection(++_sequence, InjectedFailureKind.Permanent, exception);
        }
        return this;
    }

    /// <summary>
    /// Runs the supplied <paramref name="action"/>; if a failure is pending
    /// for the supplied <paramref name="label"/>, the next call throws the
    /// configured exception. Transient failures are consumed on emission;
    /// permanent failures persist until <see cref="Reset"/>.
    /// </summary>
    public void Run(string label, Action action)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(label);
        ArgumentNullException.ThrowIfNull(action);
        var exception = Consume(label);
        if (exception is not null)
        {
            throw exception;
        }
        action();
    }

    /// <summary>Asynchronous overload of <see cref="Run"/>.</summary>
    public async Task RunAsync(string label, Func<Task> action)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(label);
        ArgumentNullException.ThrowIfNull(action);
        var exception = Consume(label);
        if (exception is not null)
        {
            throw exception;
        }
        await action().ConfigureAwait(false);
    }

    /// <summary>Returns a snapshot of all emissions in order.</summary>
    public IReadOnlyList<InjectedFailure> History
    {
        get
        {
            lock (_gate)
            {
                return _history.ToArray();
            }
        }
    }

    /// <summary>Clears every pending and historical injection.</summary>
    public void Reset()
    {
        lock (_gate)
        {
            _pending.Clear();
            _history.Clear();
            _sequence = 0;
        }
    }

    private Exception? Consume(string label)
    {
        lock (_gate)
        {
            if (!_pending.TryGetValue(label, out var pending))
            {
                return null;
            }
            _history.Add(new InjectedFailure(label, pending.Kind, pending.Sequence));
            if (pending.Kind == InjectedFailureKind.Transient)
            {
                _pending.Remove(label);
            }
            return pending.Exception;
        }
    }

    private readonly record struct PendingInjection(int Sequence, InjectedFailureKind Kind, Exception Exception);
}
