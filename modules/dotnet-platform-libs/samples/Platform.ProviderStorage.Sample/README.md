# Provider adapter sample (stage 5)

Deterministic object storage through the provider-neutral
`Platform.Storage` contracts with the local filesystem adapter.
No credentials, no network, no Docker.

## Owned by the application

- Adapter selection (`LocalFileStorage`) and the root path.
- Object keys, content types, retention, and authorization around
  the storage calls.
- The swap to `Platform.Storage.S3` later: same `IObjectStorage`
  contract, application-owned bucket, region, and credentials.

## Owned by the platform

- The `IObjectStorage` contract with safe outcomes (status plus
  redacted failure, never provider internals).
- The local adapter's atomic-write and key-validation behavior.

## Rollback

Remove the `Platform.Storage*` references and call
`LocalFileStorage` (or `System.IO` directly); the keys and root
layout stay application-owned.

## Non-goals

No S3, presigned URLs, lifecycle rules, or multipart uploads. Live
provider checks stay outside the deterministic suite.
