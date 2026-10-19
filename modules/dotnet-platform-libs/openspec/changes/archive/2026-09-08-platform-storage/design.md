## Context

The starter’s `IStorageService`, local presign token store, S3 implementation, metadata DTOs, and quota wrapper provide a useful reference. They are coupled to starter configuration and domain assumptions, so the platform should extract the stable storage lifecycle rather than copy the whole subsystem.

## Goals / Non-Goals

**Goals:**

- Make object operations testable and provider-neutral.
- Support local development and S3-compatible production storage.
- Make upload constraints, content metadata, and presigned URL lifetime explicit.

**Non-Goals:**

- No product file entities, ownership authorization, thumbnails, virus scanning, CDN policy, or retention workflow.
- No mandatory AWS SDK, filesystem layout, or database metadata store.

## Decisions

- **Model objects, not files.** `StorageObjectKey`, `StorageObjectMetadata`, `UploadRequest`, `DownloadResult`, and `PresignedOperation` are provider-neutral.
- **Keep authorization outside.** The storage service receives an already-authorized key or request; it does not infer tenant ownership.
- **Separate adapters.** Local and S3-compatible implementations are separate packages so consumers do not pay for or configure unused providers.
- **Presigning is capability-based.** Adapters issue short-lived upload/download operations with explicit method, content type, size, and expiry constraints.
- **Package layout.** Base package uses `Contracts`, `Keys`, `Results`, and `DependencyInjection`; local and S3 adapters use their own roots.

## Risks / Trade-offs

- [Risk] A presigned URL can be used outside application authorization → [Mitigation] require the application to authorize every issuance and bind expected key/content constraints.
- [Risk] Local and object-store semantics differ → [Mitigation] define portable behavior and adapter-specific documented limitations.
- [Risk] Object keys may leak tenant or user identifiers → [Mitigation] provide opaque key composition helpers and prohibit raw control characters.

## Migration Plan

Applications wrap existing storage services behind the contract, migrate metadata and key construction, then switch adapters independently. Existing objects remain untouched because bucket and filesystem layout are consumer-configured.

## Open Questions

- Whether multipart upload belongs in the first contract or a later capability.
