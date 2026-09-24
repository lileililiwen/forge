#!/bin/sh
# Fixture: the real adapter's observation when the audit attributes an
# ERROR to the project (status `fail`, ERROR code first in evidence).
# Captured verbatim for sibling project `crossalheart`
# (`DECLARATION_MISSING: adopted project must contain .project.json`).
# See NOTES.md.
req=$(cat)
pid=$(printf '%s' "$req" | sed -n 's/.*"project_id":"\([^"]*\)".*/\1/p')
printf '{"provider":"workspace-governance","protocol_version":"0.1.0","project_id":"%s","status":"fail","evidence":["DECLARATION_MISSING: adopted project must contain .project.json"],"detail":"1 errors, 0 warnings, adoption=unknown","metadata":{"registry_schema_version":1,"observed_via":"workspace_check"},"source_revision":"916f794204be9a26e20680dbd99527cde4afe14c"}\n' "$pid"
