namespace Platform.AspNetCore.Correlation;

/// <summary>
/// Exposes the current request's correlation identifier to downstream
/// code. The accessor never returns <c>null</c>; the middleware always
/// ensures a value is available.
/// </summary>
public interface ICorrelationAccessor
{
    /// <summary>
    /// Gets the current request's correlation identifier.
    /// </summary>
    string Current { get; }
}
