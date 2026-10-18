# Proposal: Bind governance checks when scaffolding a project

## Why

`init_project.py` writes a required gate manifest but does not invoke the binder, and accepts profile typos that the binder cannot resolve. New repositories therefore start with a gate declaration that has not been reconciled with available build evidence.

## What Changes

Validate `--profile` against the binder's supported profile registry before creating the target. After writing the initial manifest, invoke the binder in apply mode for that repository so derivable commands are recorded immediately. An empty scaffold remains explicitly unresolved where it has no command evidence.

## Ownership and package boundary

This is a Python 3 change in `workspace-governance`: `scripts/init_project.py` owns scaffold sequencing, and `scripts/bind_gate_commands.py` owns profile vocabulary and command derivation. It does not change Driftwatchdog runtime policy, product commands, or claim that unresolved empty-project checks pass. The reusable concern remains in the governance owner; no sibling implementation is warranted.

## Acceptance boundary

Known built-in and workspace profiles are accepted; unknown names are refused before filesystem creation. Successful scaffolding runs the existing binder with `--apply --repo <target-name>` against the workspace root, and propagates binder failure. Tests assert generated manifest state, invocation ordering, and negative typo behavior.
