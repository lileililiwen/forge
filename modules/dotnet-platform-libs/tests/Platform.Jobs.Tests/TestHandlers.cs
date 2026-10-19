namespace Platform.Jobs.Tests;

[RecurringJob("0 * * * *")]
public sealed class PresenceEvictHandler : IRecurringJobHandler
{
    public Task ExecuteAsync(CancellationToken cancellationToken) => Task.CompletedTask;
}

[RecurringJob("*/5 * * * *", Name = "queue-drain", TimeZone = "Europe/Berlin")]
public sealed class QueueDrainHandler : IRecurringJobHandler
{
    public Task ExecuteAsync(CancellationToken cancellationToken) => Task.CompletedTask;
}

public sealed class UndecoratedHandler : IRecurringJobHandler
{
    public Task ExecuteAsync(CancellationToken cancellationToken) => Task.CompletedTask;
}
