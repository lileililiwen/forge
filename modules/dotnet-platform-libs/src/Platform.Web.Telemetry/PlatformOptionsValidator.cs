using System.Reflection;

namespace Platform.Web.Telemetry;

/// <summary>Shared validator that finds and invokes an instance <c>Validate()</c> method returning <see cref="IReadOnlyList{T}"/> of strings.</summary>
/// <remarks>The platform's options types expose either a parameterless <c>Validate()</c> method or implement <see cref="IValidatablePlatformOptions"/>. This helper centralizes the discovery logic so each package does not redefine its own delegate.</remarks>
public static class PlatformOptionsValidator
{
    /// <summary>Invokes the standard <c>Validate()</c> discovery pattern and returns <c>true</c> when there are no errors.</summary>
    /// <typeparam name="T">The options type.</typeparam>
    /// <param name="options">The options instance.</param>
    /// <returns><c>true</c> when validation passes; <c>false</c> when the options instance fails.</returns>
    public static bool Validate<T>(T options) where T : class
    {
        ArgumentNullException.ThrowIfNull(options);
        if (options is IValidatablePlatformOptions validatable)
            return validatable.Validate().Count == 0;

        var method = options.GetType().GetMethod("Validate", BindingFlags.Public | BindingFlags.Instance, binder: null, types: Type.EmptyTypes, modifiers: null);
        if (method is null || method.ReturnType != typeof(IReadOnlyList<string>))
            return true;
        var result = method.Invoke(options, null) as IReadOnlyList<string>;
        return result is null || result.Count == 0;
    }
}

/// <summary>Marks an options type as having a <c>Validate()</c> method returning <see cref="IReadOnlyList{T}"/> of strings.</summary>
public interface IValidatablePlatformOptions
{
    /// <summary>Returns the validation errors. Empty when the options instance is valid.</summary>
    IReadOnlyList<string> Validate();
}
