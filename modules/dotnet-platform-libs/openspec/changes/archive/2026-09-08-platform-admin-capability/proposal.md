# Proposal: Reusable administration capability

## Why

Most products need a consistent operator surface for users, roles, permissions, sessions,
audit, health, provider status, and sometimes subscriptions. FullStackHero has a substantial
admin module and React dashboard, but those features are currently tied to its identity and
module model.

## Scope

Add an optional admin backend capability with contracts, secure endpoint conventions, query
bounds, audit hooks, and frontend metadata. A separate UI package will implement the shared
React/Razor presentation layer.

## Non-goals

- no default business-domain admin pages;
- no unrestricted impersonation;
- no bypass of authorization or tenant boundaries;
- no mandatory frontend framework in the backend package.

## API impact

Adds public admin contracts, endpoint registration, options, authorization requirements, and
provider-health projection interfaces.
