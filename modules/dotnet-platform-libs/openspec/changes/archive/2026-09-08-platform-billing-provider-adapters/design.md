# Design

Each provider adapter implements `IBillingProvider` and a webhook verifier/normalizer. Stripe
uses its official SDK where useful; Lemon Squeezy uses its documented API/webhook model. The
adapter maps external IDs to normalized identifiers and never stores application plan names.

Webhook endpoints are opt-in and accept raw request bodies for signature verification. The
adapter must reject missing, invalid, expired, or replayed signatures. Provider event IDs are
passed to the billing projector for idempotency. Provider HTTP clients use timeouts and bounded
retry policies only for safe operations.

Tests use captured synthetic payloads with fake secrets, not live provider accounts. A provider
health status reports configured, reachable, and last-error state without exposing credentials.
