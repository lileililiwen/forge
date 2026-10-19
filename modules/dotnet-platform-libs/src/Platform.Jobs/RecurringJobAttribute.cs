using System.Reflection;

namespace Platform.Jobs;

/// <summary>
/// Runtime attribute that decorates an <see cref="IRecurringJobHandler"/>
/// implementation with the cron expression and stable name the
/// scheduler will use. The attribute is retained at runtime so the
/// reflection helper can build a <see cref="RecurringJobDescriptor"/>
/// without scanning for an external mapping.
/// </summary>
[AttributeUsage(AttributeTargets.Class, AllowMultiple = false, Inherited = false)]
public sealed class RecurringJobAttribute : Attribute
{
    /// <summary>
    /// Initializes a new <see cref="RecurringJobAttribute"/> with the
    /// documented cron expression.
    /// </summary>
    /// <param name="cron">The cron expression, in the form documented by the scheduler.</param>
    /// <exception cref="ArgumentException"><paramref name="cron"/> is null, empty, or whitespace.</exception>
    public RecurringJobAttribute(string cron)
    {
        if (string.IsNullOrWhiteSpace(cron))
        {
            throw new ArgumentException("Cron expression must be a non-empty string.", nameof(cron));
        }

        Cron = cron;
    }

    /// <summary>
    /// Gets the documented cron expression.
    /// </summary>
    public string Cron { get; }

    /// <summary>
    /// Gets or sets the stable, scheduler-visible name. When
    /// <c>null</c>, the name is derived from the decorated handler
    /// type's full name.
    /// </summary>
    public string? Name { get; set; }

    /// <summary>
    /// Gets or sets an optional time zone identifier the scheduler
    /// applies to <see cref="Cron"/>. Defaults to <c>UTC</c>.
    /// </summary>
    public string TimeZone { get; set; } = "UTC";

    /// <summary>
    /// Gets or sets an optional bag of scheduler-specific options.
    /// </summary>
    public IReadOnlyDictionary<string, object?>? Options { get; set; }

    /// <summary>
    /// Builds a <see cref="RecurringJobDescriptor"/> from the
    /// attribute decorating the supplied <paramref name="handlerType"/>.
    /// </summary>
    /// <param name="handlerType">The handler type to inspect.</param>
    /// <returns>The constructed descriptor.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="handlerType"/> is <c>null</c>.</exception>
    /// <exception cref="InvalidOperationException">The supplied type is not decorated with <see cref="RecurringJobAttribute"/>.</exception>
    public static RecurringJobDescriptor GetDescriptor(Type handlerType)
    {
        ArgumentNullException.ThrowIfNull(handlerType);

        var attribute = handlerType.GetCustomAttribute<RecurringJobAttribute>(inherit: false);
        if (attribute is null)
        {
            throw new InvalidOperationException(
                $"Handler '{handlerType.FullName}' is not decorated with [RecurringJobAttribute].");
        }

        return new RecurringJobDescriptor(
            Name: attribute.Name ?? handlerType.FullName ?? handlerType.Name,
            Cron: attribute.Cron,
            HandlerType: handlerType,
            TimeZone: attribute.TimeZone,
            Options: attribute.Options);
    }
}
