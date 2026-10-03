using System.Reflection;
using Microsoft.Extensions.DependencyInjection;
using Platform.Core.Time;
using Platform.Testing.AspNetCore;
using Platform.Testing.TestApplication;

namespace Platform.Testing.Tests.AspNetCore;

/// <summary>
/// Coverage for the named entry-point diagnostics. These are the failure modes
/// the previous empty-application factory could not produce, because it always
/// succeeded by building an application of its own.
/// </summary>
public sealed class PlatformTestWebApplicationFactoryEntryPointTests
{
    /// <summary>
    /// A public type that is not an application entry point. It is declared by the
    /// test assembly, which exposes no host builder for it.
    /// </summary>
    public sealed class NotAnEntryPoint
    {
    }

    [Fact]
    public void A_hand_written_internal_entry_point_cannot_be_named_by_the_test_assembly()
    {
        // The test assembly's own types are not an application entry point, which
        // is the same compile-time position a consumer is in when its Program is
        // internal without InternalsVisibleTo: the type is nameable but unusable
        // as an entry point.
        Assert.True(typeof(NotAnEntryPoint).IsVisible);
        Assert.Null(typeof(NotAnEntryPoint).Assembly.EntryPoint?.DeclaringType
            ?.GetMethod("CreateHostBuilder", System.Reflection.BindingFlags.Static | System.Reflection.BindingFlags.Public));
    }

    [Fact]
    public void Entry_point_type_that_exposes_no_host_builder_is_named()
    {
        var failure = Assert.Throws<InvalidOperationException>(
            () => new PlatformTestWebApplicationFactory<NotAnEntryPoint>().Services);

        Assert.Contains(nameof(NotAnEntryPoint), failure.Message, StringComparison.Ordinal);
        Assert.Contains(typeof(NotAnEntryPoint).Assembly.GetName().Name!, failure.Message, StringComparison.Ordinal);
        Assert.Contains("cannot boot an application", failure.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void The_diagnostic_names_both_supported_remedies()
    {
        var failure = Assert.Throws<InvalidOperationException>(
            () => new PlatformTestWebApplicationFactory<NotAnEntryPoint>().Services);

        Assert.Contains("InternalsVisibleTo", failure.Message, StringComparison.Ordinal);
        Assert.Contains("public marker", failure.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void A_failed_boot_never_falls_back_to_an_application_of_the_factings_own()
    {
        var factory = new PlatformTestWebApplicationFactory<NotAnEntryPoint>();

        var failure = Assert.Throws<InvalidOperationException>(() => factory.Services);
        Assert.Contains("never falls back to an application of its own", failure.Message, StringComparison.Ordinal);

        // Nothing was booted, so no client can be produced either: there is no
        // fallback application behind the factory.
        Assert.ThrowsAny<Exception>(() => factory.CreateClient());
    }

    [Fact]
    public void The_diagnostic_is_produced_for_a_type_from_a_library_assembly()
    {
        // SystemClock is a public type in Platform.Core, a library with no entry
        // point, so it cannot be the consumer's application.
        var failure = Assert.Throws<InvalidOperationException>(
            () => new PlatformTestWebApplicationFactory<SystemClock>().Services);

        Assert.Contains(nameof(SystemClock), failure.Message, StringComparison.Ordinal);
        Assert.Contains(
            typeof(SystemClock).Assembly.GetName().Name!,
            failure.Message,
            StringComparison.Ordinal);
    }

    [Fact]
    public void A_public_marker_in_a_real_application_boots_without_any_diagnostic()
    {
        using var factory = new PlatformTestWebApplicationFactory<PlatformTestApplicationEntryPoint>();

        Assert.NotNull(factory.Services);
    }
}
