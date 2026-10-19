namespace Platform.Mailing;

/// <summary>
/// Documented value type for a single mail attachment.
/// </summary>
/// <param name="FileName">The attachment file name. MUST be non-null and non-empty.</param>
/// <param name="ContentType">The MIME content type. MUST be non-null and non-empty.</param>
/// <param name="Content">The raw attachment bytes.</param>
public sealed record MailAttachment(string FileName, string ContentType, byte[] Content)
{
    /// <summary>
    /// Creates a <see cref="MailAttachment"/>, validating the file name
    /// and content type are non-empty.
    /// </summary>
    /// <param name="fileName">The attachment file name.</param>
    /// <param name="contentType">The MIME content type.</param>
    /// <param name="content">The raw attachment bytes.</param>
    /// <returns>The constructed <see cref="MailAttachment"/>.</returns>
    /// <exception cref="ArgumentException">A string argument is null, empty, or whitespace.</exception>
    /// <exception cref="ArgumentNullException"><paramref name="content"/> is <c>null</c>.</exception>
    public static MailAttachment Create(string fileName, string contentType, byte[] content)
    {
        if (string.IsNullOrWhiteSpace(fileName))
        {
            throw new ArgumentException("File name must be a non-empty string.", nameof(fileName));
        }

        if (string.IsNullOrWhiteSpace(contentType))
        {
            throw new ArgumentException("Content type must be a non-empty string.", nameof(contentType));
        }

        ArgumentNullException.ThrowIfNull(content);

        return new MailAttachment(fileName, contentType, content);
    }
}
