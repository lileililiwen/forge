#!/bin/sh
# Fixture: the real adapter's observation for a registered project whose
# audit is clean but whose registry entry is not adopted (status
# `blocked`, detail names the adoption state). Captured by running the
# real adapter against a scratch registry copy with `argoset`'s adoption
# field removed. See NOTES.md.
req=$(cat)
pid=$(printf '%s' "$req" | sed -n 's/.*"project_id":"\([^"]*\)".*/\1/p')
printf '{"provider":"workspace-governance","protocol_version":"0.1.0","project_id":"%s","status":"blocked","evidence":[],"detail":"0 errors, 0 warnings, adoption=unknown","metadata":{"registry_schema_version":1,"observed_via":"workspace_check"},"source_revision":"1c4177c5fdecaac837cea22f9a56adb31774c0cf"}\n' "$pid"
