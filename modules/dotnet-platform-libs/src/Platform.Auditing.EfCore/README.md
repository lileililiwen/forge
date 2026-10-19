# Platform.Auditing.EfCore

Optional EF Core change-capture interceptor for the auditing contracts. Entities opt in by
implementing `Platform.Auditing.Contracts.IAuditedEntity`; the interceptor masks sensitive values by
property name and publishes a normalized `entity` audit event through the registered sink. The
application owns the `DbContext` and migrations; the platform does not own an audit store.
