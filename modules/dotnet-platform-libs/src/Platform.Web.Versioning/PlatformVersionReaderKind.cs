namespace Platform.Web.Versioning;

/// <summary>How API version is read from an inbound request.</summary>
public enum PlatformVersionReaderKind
{
    /// <summary>Read the version from a URL segment. The default.</summary>
    UrlSegment = 0,

    /// <summary>Read the version from an HTTP header.</summary>
    Header = 1,

    /// <summary>Read the version from a query string parameter.</summary>
    QueryString = 2,

    /// <summary>Read the version from the media type of the request body or accept header.</summary>
    MediaType = 3,

    /// <summary>Combine query string and URL segment readers (the Asp.Versioning default).</summary>
    Composite = 4,
}
