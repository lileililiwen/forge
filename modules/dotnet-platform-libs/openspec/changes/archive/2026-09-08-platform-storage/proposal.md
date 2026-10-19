## Why

File upload and object storage are recurring application concerns: the starter supports local and S3 storage, while Crossify and VisualFlow own separate media flows. Applications need reusable upload/download and presigning contracts without sharing domain entities or storage buckets.

## What Changes

- Add provider-neutral object-storage contracts for metadata, upload, download, delete, and presigned operations.
- Add local filesystem and S3-compatible adapters in separate packages.
- Add safe object-key validation, content-length limits, and provider status contracts.
- Add deterministic in-memory/testing support.
- Document ownership of authorization, retention, domain metadata, and bucket configuration.

## Capabilities

### New Capabilities

- `platform-storage`: object storage contracts, safe keys, local adapter, and S3-compatible adapter seams.

### Modified Capabilities

- None.

## Impact

- New `Platform.Storage`, `Platform.Storage.Local`, and `Platform.Storage.S3` packages.
- Optional filesystem and AWS-compatible dependencies only in adapters.
- No shared file tables, migration scripts, bucket names, or product upload policies.
