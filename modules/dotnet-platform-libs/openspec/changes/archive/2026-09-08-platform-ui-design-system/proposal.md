# Proposal: Shared UI design system

## Why

The C# applications split between React, Razor Pages, and other clients, repeatedly creating
buttons, forms, tables, shells, loading/error/empty states, themes, and admin layouts. The
FullStackHero admin and dashboard already contain a useful design-token and component reference,
but the two clients duplicate much of it.

## Scope

Create a shared token source, generated web artifacts, a React package, Razor static web assets,
accessible state components, and admin/dashboard shell contracts. Share visual language and
behavior without forcing React and Razor to share implementation code.

## Non-goals

- no product-specific page composition;
- no mandatory React or Razor runtime for backend packages;
- no copy of the entire FullStackHero client applications;
- no visual redesign of existing products in this change.

## API impact

Adds token schemas, npm package APIs, Razor asset conventions, and UI component contracts.
