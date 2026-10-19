using System.Collections.Generic;

namespace Platform.Auditing.EfCore;

/// <summary>A normalized, masked diff of one audited entity within a save operation.</summary>
/// <param name="EntityType">The entity CLR type name.</param>
/// <param name="EntityKey">The primary-key value(s), comma-joined.</param>
/// <param name="Operation">One of <c>created</c>, <c>updated</c>, or <c>deleted</c>.</param>
/// <param name="Changes">The per-property masked changes.</param>
public sealed record EntityAuditEntry(
    string EntityType,
    string EntityKey,
    string Operation,
    IReadOnlyList<EntityAuditChange> Changes);
