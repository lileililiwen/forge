## Context

Sibling applications independently implement payment webhook inboxes, HMAC verification, outbound delivery, retries, and SSRF defenses. Billing adapters already normalize provider events, so the new capability must stop at transport/lifecycle concerns and avoid duplicating billing behavior.

## Goals / Non-Goals

**Goals:**

- Provide safe inbound and outbound webhook lifecycle primitives.
- Make signature verification, replay identity, delivery attempts, and retry state explicit.
- Support application-owned event payloads and secret stores.

**Non-Goals:**

- No provider-specific signature algorithm beyond composable verifier contracts.
- No billing event mapping, domain handlers, tenant model, dashboard, or hosted service requirement.
- No unrestricted outbound HTTP target support.

## Decisions

- **Separate inbound and outbound concerns.** They share delivery/status vocabulary but have different trust and security models.
- **Verify raw bytes.** Inbound verification receives the original request body and headers before deserialization.
- **Use replay identity.** A provider/event identifier plus provider scope is persisted before handler execution.
- **Protect outbound targets.** Validate absolute HTTPS targets, resolve DNS, reject loopback/private/link-local destinations, and revalidate redirects according to application policy.
- **Keep persistence optional.** Contracts are framework-neutral; ASP.NET Core handles request capture and EF Core handles storage in separate packages.
- **Package layout.** Use `Inbound`, `Outbound`, `Security`, `Persistence`, and `DependencyInjection` folders only inside their respective package roots.

## Risks / Trade-offs

- [Risk] SSRF checks race with DNS changes → [Mitigation] resolve and validate at send time, disable redirects by default, and document network egress controls.
- [Risk] Signature providers differ in canonicalization and timestamp rules → [Mitigation] use provider-supplied verifier implementations with test vectors.
- [Risk] Durable webhook processing duplicates durable eventing → [Mitigation] depend on durable eventing contracts only where needed and keep webhook records distinct from application event records.

## Migration Plan

Start with inbound billing/webhook verification in one application, run duplicate detection in observe-only mode, then enable durable processing. Outbound adoption follows after target validation and retry behavior are tested. Existing subscriptions and secrets remain application-owned.

## Open Questions

- Whether outbound delivery should reuse the durable-eventing dispatcher or expose an independent dispatcher first.
