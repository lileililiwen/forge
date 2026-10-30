# Forge publish-provider contract

Forge is the publish control plane. A publish provider is a standalone executable owned by its own repository. Forge starts it for one request and exchanges one JSON document on stdin and stdout.

The current contract is `forge-publish-provider/0.1.0`.

## Request

```json
{
  "contract": "forge-publish-provider/0.1.0",
  "operation": "publish",
  "provider": "openpanel",
  "project_id": "example",
  "revision": "0123456789abcdef0123456789abcdef01234567",
  "operation_id": "publish-example-1",
  "folder": "/workspace/example",
  "dry_run": false
}
```

`operation` is one of `capabilities`, `preflight`, `publish`, `verify`, or `rollback`. `folder` is a source-data hint; it is never a place where Forge expects provider code to be installed. Secrets and source contents are not part of this protocol.

## Response

```json
{
  "contract": "forge-publish-provider/0.1.0",
  "provider": "openpanel",
  "operation_id": "publish-example-1",
  "status": "done",
  "health": "healthy",
  "evidence": ["runtime health check passed"],
  "recovery": []
}
```

Providers must return redacted, bounded evidence. Forge records the response and never treats process exit alone as deployment health. A provider may keep private credentials and runtime state on its target runtime, but it must not require scripts copied into the runtime host.

## Capabilities

`capabilities` is how a provider says which of the operations it serves. Forge
invokes it once at the start of every delivery verb, after the verb's local
gates and before the delivery operation is reserved. The probe is
side-effect-free, carries no `queue_id`, reserves no operation and changes no
delivery phase.

The answer is the additive `capabilities` field on the normal response:

```json
{
  "contract": "forge-publish-provider/0.1.0",
  "provider": "openpanel",
  "operation_id": "delivery-capabilities-example-0123456789ab",
  "status": "done",
  "health": "unknown",
  "capabilities": ["preflight", "publish"]
}
```

Forge reads three outcomes:

| The provider | Forge reads | Effect |
|---|---|---|
| answers a valid list | **declared** | the listed operations are served; any other operation is refused before it runs |
| answers no `capabilities` field, exits non-zero, or answers no provider response | **unknown** | `preflight` and `publish` are served; nothing newer is attempted |
| answers a provider response with a malformed list | **invalid** | the verb is refused as an invalid provider response |

A declared list is enforced literally. `preflight` is never gated — it is the
entry verb. `stage` and `promote` send `publish`, so they require `publish` in
the declaration. `verify` and `rollback` require their own entry; a provider
that does not declare one is never sent it.

The malformed cases are: more than sixteen names; a name that is empty, longer
than thirty-two characters, or outside `[a-z0-9_-]`; the same name twice. A
valid name this Forge does not recognise is ignored, so a provider may declare
a newer operation without an older Forge refusing it.

The declaration is recorded on the journal row of the verb it gated under
`provider_capabilities`, as `{"state":"declared","operations":[…]}` or
`{"state":"unknown","reason":"…"}`. It is not cached: a provider that changes
its declaration between verbs is read as it is now.

This field is additive and the operation was already declared, so the contract
version does not move. A provider written before it existed is **unknown**, not
broken, and needs no change.

OpenPanel and Jenkins remain standalone projects. They can each implement this contract as an optional Forge provider and can be enabled or disabled independently.

## GitHub push intake

Forge accepts verified push deliveries at `POST /v1/publish/github`. The API
does not use a bearer session for this route; it requires the GitHub
`X-Hub-Signature-256` and `X-GitHub-Delivery` headers and validates the raw
request body before selecting a provider. Configure the boundary with:

```text
FORGE_GITHUB_WEBHOOK_SECRET=...
FORGE_GITHUB_REPOSITORY=owner/repository
FORGE_GITHUB_REF=refs/heads/main
FORGE_GITHUB_PROJECT_ID=project-id
FORGE_PUBLISH_PROVIDER=openpanel
```

The delivery id is the idempotency key. A duplicate delivery is acknowledged
without invoking the provider a second time. The commit SHA from `after` is
passed unchanged to the provider.
