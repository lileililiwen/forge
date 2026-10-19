namespace Platform.Adoption;

/// <summary>
/// Thrown when the requested adoption target directory is missing, relative,
/// or otherwise unusable. Carries no filesystem contents.
/// </summary>
public sealed class AdoptionTargetException : Exception
{
    /// <summary>Initializes the exception with the offending target path.</summary>
    /// <param name="target">The supplied target directory value.</param>
    /// <param name="message">Why the target is unusable.</param>
    public AdoptionTargetException(string target, string message)
        : base(message)
    {
        Target = target;
    }

    /// <summary>Gets the supplied target directory value.</summary>
    public string Target { get; }
}
