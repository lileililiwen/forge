#!/bin/sh
# Fixture: the real Workspace Governance adapter's observation for an
# adopted project whose audit has no ERROR findings. Captured verbatim
# from `forge_governance_adapter.py` (sibling HEAD 204d140) for project
# `forge`; only the project id is echoed from the request. See NOTES.md.
req=$(cat)
pid=$(printf '%s' "$req" | sed -n 's/.*"project_id":"\([^"]*\)".*/\1/p')
printf '{"provider":"workspace-governance","protocol_version":"0.1.0","project_id":"%s","status":"pass","evidence":[],"detail":"0 errors, 0 warnings, adoption=adopted","metadata":{"registry_schema_version":1,"observed_via":"workspace_check"},"source_revision":"888d8058e7a2615e0c6525a533463e5029d56487"}\n' "$pid"
