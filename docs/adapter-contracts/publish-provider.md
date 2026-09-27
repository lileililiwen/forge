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

OpenPanel and Jenkins remain standalone projects. They can each implement this contract as an optional Forge provider and can be enabled or disabled independently.

