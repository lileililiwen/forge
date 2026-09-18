# Security and privacy

Treat model output, repository files, template paths, external findings and provider payloads as untrusted inputs. Validate typed contracts, constrain paths and invoke processes with argument arrays rather than interpolated shell.

Preserve user-owned files and dirty worktrees. Require explicit targets and scoped authority for destructive operations and remote writes; remote retries cannot broaden the original operation.

Keep secrets outside manifests and artifacts; use credential references and redact logs. Enforce project/action authorization in network transports. Bind plans and evidence to revisions to prevent stale approval reuse.

Do not equate OIDC login with project admin permission. Maintain separate project sessions and validate standard protocol inputs.

Report partial external writes and irreversible migrations accurately, with recovery guidance. Never claim atomic rollback across repositories, registries and databases.
