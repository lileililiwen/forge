namespace Platform.Core.Results;

#pragma warning disable CA1000 // Intentional: Success/Failure factories are required by the Result<T> pattern.

/// <summary>
/// Framework-neutral outcome of an operation that returns a value on
/// success. Use <see cref="Success"/> for the successful state and
/// <see cref="Failure(Error)"/> for the failed state.
/// </summary>
/// <typeparam name="T">The success value type.</typeparam>
public sealed class Result<T>
{
    private Result(bool isSuccess, T? value, Error? error)
    {
        IsSuccess = isSuccess;
        Value = value;
        Error = error;
    }

    /// <summary>
    /// Gets a value indicating whether the operation succeeded.
    /// </summary>
    public bool IsSuccess { get; }

    /// <summary>
    /// Gets the success value when <see cref="IsSuccess"/> is
    /// <c>true</c>; otherwise the default value of <typeparamref name="T"/>.
    /// </summary>
    public T? Value { get; }

    /// <summary>
    /// Gets the failure description when <see cref="IsSuccess"/> is
    /// <c>false</c>; otherwise <c>null</c>.
    /// </summary>
    public Error? Error { get; }

    /// <summary>
    /// Creates a successful result carrying <paramref name="value"/>.
    /// </summary>
    /// <param name="value">The success value.</param>
    /// <returns>A successful <see cref="Result{T}"/>.</returns>
    public static Result<T> Success(T value) => new(true, value, null);

    /// <summary>
    /// Creates a failed result carrying the supplied
    /// <paramref name="error"/>.
    /// </summary>
    /// <param name="error">The failure description.</param>
    /// <returns>A failed <see cref="Result{T}"/>.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="error"/> is <c>null</c>.</exception>
    public static Result<T> Failure(Error error)
    {
        ArgumentNullException.ThrowIfNull(error);
        return new(false, default, error);
    }

    /// <summary>
    /// Converts this result to its non-generic <see cref="Result"/>
    /// counterpart. The success value, if any, is discarded.
    /// </summary>
    /// <returns>The corresponding <see cref="Result"/>.</returns>
    public Result ToResult() => IsSuccess
        ? Result.Success()
        : Result.Failure(Error!);
}

#pragma warning restore CA1000
