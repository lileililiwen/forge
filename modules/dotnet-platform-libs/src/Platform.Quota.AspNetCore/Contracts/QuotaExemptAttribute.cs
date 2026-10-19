namespace Platform.Quota.AspNetCore.Contracts;

/// <summary>
/// Marks an endpoint as exempt from quota enforcement. Applied via <c>[QuotaExempt]</c> on a
/// controller, action, or endpoint; the middleware skips any request whose resolved endpoint carries
/// this metadata. Route-level exemptions remain configurable through <see cref="QuotaEnforcementOptions.ExemptPathPrefixes"/>.
/// </summary>
[AttributeUsage(AttributeTargets.Class | AttributeTargets.Method, AllowMultiple = false, Inherited = true)]
public sealed class QuotaExemptAttribute : Attribute;
