using Microsoft.Extensions.DependencyInjection;
using Platform.Tenancy.Sample;
using Platform.Tenant.Lifecycle;
using Platform.Tenant.Lifecycle.Contracts;

// Stage 4: tenant scope. The platform owns the orchestration contract;
// the application owns the workflow, the steps, the scope callback, and
// the durable store (in-memory here, a real database in production).
var store = new SampleTenantLifecycleStore();
var workflow = new SampleProvisioningWorkflow();
var tenant = args.FirstOrDefault() ?? "tenant-matrix-1";

var services = new ServiceCollection();
services.AddSingleton<ITenantLifecycleStore>(store);
services.AddPlatformTenantLifecycle();
await using var provider = services.BuildServiceProvider();
provider.GetRequiredService<TenantLifecycleWorkflowRegistry>().Register(workflow);
var orchestrator = provider.GetRequiredService<ITenantLifecycleOrchestrator>();
var status = await orchestrator.StartAsync(workflow, tenant, new Dictionary<string, string>());

Console.WriteLine($"tenant={tenant} state={status.State} completed={status.CompletedStepCount}");
