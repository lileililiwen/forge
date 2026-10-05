namespace Platform.Privacy.Contracts;

/// <summary>Consent categories. Fixed vocabulary; finer purposes live in application data.</summary>
public enum ConsentCategory
{
    /// <summary>Strictly necessary: auth, security, load balancing. Always granted.</summary>
    Necessary = 0,

    /// <summary>Remembered preferences: language, theme, layout.</summary>
    Preferences = 1,

    /// <summary>First-party measurement the subject can decline.</summary>
    Analytics = 2,

    /// <summary>Advertising and cross-site tracking.</summary>
    Marketing = 3,
}
