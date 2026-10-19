# Design

Use DTCG-compatible tokens as the source of truth and generate CSS variables, TypeScript
tokens, and Razor static assets. The semantic token layer is public; primitive values remain
replaceable.

Packages:

- `Platform.DesignTokens` source and build validation;
- `@platform/design-tokens` generated TypeScript/CSS;
- `@platform/react-ui` accessible primitives and state components;
- `@platform/react-shell` authenticated app/admin shell, navigation, permissions, and theme;
- `Platform.UI.Razor` static web assets, partial/tag-helper conventions, and equivalent state components.

The first component set is button, input, select, dialog, table, pagination, badge, card,
toast, navigation shell, loading, error, empty, forbidden, and not-found states. Components
must define keyboard behavior, focus behavior, loading/error/disabled states, and dark-mode
tokens.

Frontend packages consume generated API/permission metadata but do not own authorization.
