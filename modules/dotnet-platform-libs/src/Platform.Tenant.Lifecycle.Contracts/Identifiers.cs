using System.Globalization;

namespace Platform.Tenant.Lifecycle.Contracts;

/// <summary>Opaque identifier for a tenant lifecycle operation. Generated server-side; never reused across operations.</summary>
public readonly record struct TenantLifecycleOperationId(string Value)
{
    /// <summary>Creates a new operation identifier from a caller-supplied string.</summary>
    public static TenantLifecycleOperationId Parse(string value)
    {
        if (string.IsNullOrWhiteSpace(value))
            throw new ArgumentException("Operation id is required.", nameof(value));
        return new TenantLifecycleOperationId(value);
    }

    /// <summary>Generates a fresh, deterministic-shape operation identifier from an ambient clock-free counter.</summary>
    public static TenantLifecycleOperationId Generate() => new("tlc_" + Interlocked.Increment(ref s_counter).ToString("D20", CultureInfo.InvariantCulture));
    private static long s_counter;

    /// <inheritdoc />
    public override string ToString() => Value;
}

/// <summary>Stable name of a tenant lifecycle step. Used as the idempotency boundary and as the audit subject.</summary>
public readonly record struct TenantLifecycleStepName(string Value)
{
    /// <summary>Creates a new step name from a non-empty string.</summary>
    public static TenantLifecycleStepName Parse(string value)
    {
        if (string.IsNullOrWhiteSpace(value))
            throw new ArgumentException("Step name is required.", nameof(value));
        return new TenantLifecycleStepName(value);
    }

    /// <inheritdoc />
    public override string ToString() => Value;
}

/// <summary>Stable name of a tenant lifecycle workflow. Used as the operation identity together with <see cref="TenantLifecycleOperationId"/>.</summary>
public readonly record struct TenantLifecycleWorkflowName(string Value)
{
    /// <summary>Creates a new workflow name from a non-empty string.</summary>
    public static TenantLifecycleWorkflowName Parse(string value)
    {
        if (string.IsNullOrWhiteSpace(value))
            throw new ArgumentException("Workflow name is required.", nameof(value));
        return new TenantLifecycleWorkflowName(value);
    }

    /// <inheritdoc />
    public override string ToString() => Value;
}
