# Platform storage

`Platform.Storage` is a provider-neutral object-storage boundary. The consuming application owns
authorization, tenant scoping, object metadata persistence, retention, scanning, and the bucket or
filesystem configuration.

## Packages and organization

- `Platform.Storage` — `Contracts`, `Keys`, `Results`-equivalent result types, and
  `DependencyInjection`. It has no filesystem, cloud SDK, or database dependency.
- `Platform.Storage.Local` — atomic local filesystem implementation for development and single-host
  deployments.
- `Platform.Storage.S3` — optional AWS/S3-compatible implementation using an application-provided
  `IAmazonS3` client and bucket configuration.

## Keys and authorization

Use `StorageObjectKey` and include an opaque tenant/application prefix in every tenant-scoped key.
Keys reject control characters, empty segments, absolute paths, traversal segments, and excessive
length. Storage authorization is deliberately outside the adapter: callers must authorize every
upload, download, delete, and presign request before invoking the store.

## Limits and presigning

`StorageOptions` bounds object size, provider wait time, and presign lifetime. Uploads validate the
declared content length and content type before provider access. Presigned operations carry explicit
method, key, expiry, and optional content constraints; they are capabilities, not authorization
decisions.

## Metadata, retention, and migration

Adapters return portable size, content type, modification time, and optional ETag metadata. Product
file rows, ownership, retention jobs, thumbnails, virus scanning, and CDN policy remain in the
application. Switching between local storage and S3 does not move objects or create migrations;
configure the destination and migrate objects under application control.

Provider failures return redacted transient metadata and expose `IStorageProviderStatus`. Cloud
integration tests require credentials or an external S3-compatible service and are intentionally
not part of the deterministic local test gate.
