# Design

`Platform.Admin.Contracts` defines user, role, permission, session, audit-summary, provider
status, and subscription-summary DTOs plus service interfaces. `Platform.Admin.AspNetCore`
provides opt-in endpoints and consistent pagination/problem responses. An optional EF Core
adapter supplies default stores but allows replacement.

Every endpoint requires an explicit platform permission. Search, sort, and page sizes are
bounded. Impersonation is disabled by default, requires a reason and target permission, emits
start/end audit events, and cannot cross a tenant boundary unless an application explicitly
provides a resolver.

The backend exposes a permission catalog and navigation metadata so React and Razor clients can
render only features the current operator may use.
