namespace Platform.Mailing;

/// <summary>
/// Contract for a template renderer that turns a <see cref="MailTemplateId"/>
/// and a model into a <see cref="RenderedMailTemplate"/>. The platform
/// does NOT ship a default implementation; consumers can implement one
/// against their preferred templating engine (Razor, Liquid, Scriban, …).
/// </summary>
/// <typeparam name="TModel">The model type the renderer accepts.</typeparam>
public interface IMailTemplateRenderer<TModel>
{
    /// <summary>
    /// Renders the template identified by <paramref name="templateId"/>
    /// with the supplied <paramref name="model"/>.
    /// </summary>
    /// <param name="templateId">The opaque template identifier.</param>
    /// <param name="model">The model to feed the renderer.</param>
    /// <param name="cancellationToken">The cancellation token.</param>
    /// <returns>The rendered template.</returns>
    Task<RenderedMailTemplate> RenderAsync(
        MailTemplateId templateId,
        TModel model,
        CancellationToken cancellationToken = default);
}
