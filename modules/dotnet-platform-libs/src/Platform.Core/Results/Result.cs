namespace Platform.Core.Results;

/// <summary>
/// Framework-neutral outcome of an operation that does not return a
/// value. Use <see cref="Success"/> for the successful state and
/// <see cref="Failure(Error)"/> for the failed state.
/// </summary>
public sealed class Result
{
    private Result(bool isSuccess, Error? error)
    {
        IsSuccess = isSuccess;
        Error = error;
    }

    /// <summary>
    /// Gets a value indicating whether the operation succeeded.
    /// </summary>
    public bool IsSuccess { get; }

    /// <summary>
    /// Gets the failure description when <see cref="IsSuccess"/> is
    /// <c>false</c>; otherwise <c>null</c>.
    /// </summary>
    public Error? Error { get; }

    /// <summary>
    /// Creates a successful result.
    /// </summary>
    /// <returns>A successful <see cref="Result"/>.</returns>
    public static Result Success() => new(true, null);

    /// <summary>
    /// Creates a failed result carrying the supplied
    /// <paramref name="error"/>.
    /// </summary>
    /// <param name="error">The failure description.</param>
    /// <returns>A failed <see cref="Result"/>.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="error"/> is <c>null</c>.</exception>
    public static Result Failure(Error error)
    {
        ArgumentNullException.ThrowIfNull(error);
        return new(false, error);
    }
}
