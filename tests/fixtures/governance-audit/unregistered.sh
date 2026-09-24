#!/bin/sh
# Fixture: the real adapter's observation for a project id absent from
# the registry and not discovered on disk (status `unknown`, still exit
# 0 so Forge distinguishes "answered unknown" from "could not answer").
# Captured verbatim for an invented id; the PROJECT_UNKNOWN evidence line
# embeds the id, so it is echoed from the request. See NOTES.md.
req=$(cat)
pid=$(printf '%s' "$req" | sed -n 's/.*"project_id":"\([^"]*\)".*/\1/p')
printf '{"provider":"workspace-governance","protocol_version":"0.1.0","project_id":"%s","status":"unknown","evidence":["PROJECT_UNKNOWN: project not found in registry: %s"],"detail":"not registered in workspace registry","metadata":{"registry_schema_version":1,"observed_via":"workspace_check"}}\n' "$pid" "$pid"
