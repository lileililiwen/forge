using Platform.Core.Time;

namespace Platform.Testing.TestApplication;

/// <summary>
/// Public entry-point marker for the in-repo test application. Exists so the
/// factory can be pointed at a dedicated, visible type rather than a
/// top-level-statements <c>Program</c> that is internal to this assembly.
/// </summary>
public sealed class PlatformTestApplicationEntryPoint
{
}

/// <summary>Contract registered twice by the application so replacement is observable.</summary>
public interface IGreetingSender
{
    string LastGreeting { get; }
}

/// <summary>The application's own greeting implementation.</summary>
public sealed class ApplicationGreetingSender : IGreetingSender
{
    public string LastGreeting => "application";
}

/// <summary>A second registration, so tests can prove every prior registration is removed.</summary>
public sealed class DuplicateGreetingSender : IGreetingSender
{
    public string LastGreeting => "duplicate";
}

/// <summary>Marker contract registered once, used for decoration tests.</summary>
public interface IAuditedOperation
{
    string Describe();
}

/// <summary>The application's own audited operation.</summary>
public sealed class ApplicationAuditedOperation : IAuditedOperation
{
    public string Describe() => "application";
}

/// <summary>Static asset served from the application's own output directory.</summary>
public static class PlatformTestAssets
{
    /// <summary>Content of the static asset used to prove the content root is the application's.</summary>
    public const string Marker = "platform-test-application-asset";
}

/// <summary>
/// Hosted service that records its own lifecycle on the instance, so a test can
/// observe that the host started it and that the host had already stopped when
/// factory teardown actions ran. Instance state keeps parallel test classes from
/// interfering with one another.
/// </summary>
public sealed class StopProbeHostedService : Microsoft.Extensions.Hosting.IHostedService
{
    /// <summary>Gets a value indicating whether this instance has been stopped.</summary>
    public bool HasStopped { get; private set; }

    /// <summary>Gets the number of times this instance has been started.</summary>
    public int StartCount { get; private set; }

    /// <inheritdoc />
    public Task StartAsync(CancellationToken cancellationToken)
    {
        StartCount++;
        return Task.CompletedTask;
    }

    /// <inheritdoc />
    public Task StopAsync(CancellationToken cancellationToken)
    {
        HasStopped = true;
        return Task.CompletedTask;
    }
}
