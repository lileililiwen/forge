## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The profile status enum and catalog | `forge` | Yes |
| Generation's refusal | `forge` | Yes |
| Doctor's assessment | `forge` | Yes |
| Checker/API readiness projections | `forge` | Yes |
| The governance-profile mapping | `forge`, read only | No |

## Why a Third Status

`Supported` means "selectable and has a tested template". `Planned` means "on
the roadmap, no template yet". A native project is neither: it exists, it
builds with its toolchain, and Forge must never write it from a template.

Overloading `Planned` would make `forge profile list` hide it, doctor report it
as unsupported, and registration refuse — the current bug. Overloading
`Supported` would make `forge new --profile rust-product` selectable and fail
only at asset resolution. `Native` is the honest third value.

## The Descriptor

```text
id             rust-product
support_status Native
adapter        adapter-rust
language       rust
toolchain      cargo (stable)
build_command  cargo build --workspace
test_command   cargo test --workspace
layout         src/, crates/, modules/, Cargo.toml
capabilities   none
description    Forge's own native Rust control plane; not generatable
```

`rust-product` is also the governance profile that `rust-web` maps to in
`.project.json`. The two are different namespaces: the Forge manifest names a
profile descriptor, and the workspace declaration names a governance profile.
The descriptor's description says so, and no code compares the two strings.

## Resolution Rules

| Operation | Native result |
|---|---|
| `inspect_profile` | found |
| `list_profiles` | excluded (not selectable) |
| `resolve_profile(id, [])` | Ok — registration and doctor need it |
| `resolve_profile(id, ["auth"])` | `incompatible-profile` — no generated features |
| `preflight_profile` | allowed; the toolchain is real |
| `generate` / `forge new` | refused before staging: native has no scaffold |
| `register` | allowed through `resolve_profile(id, [])` |

## Doctor

`profile-known` becomes `PASS` with the descriptor's evidence. `features-compatible`
is `PASS` only when the manifest requests nothing; a native project has no
generated capability set, so a request is a real incompatibility. The build
definition table gains `rust-product -> Cargo.toml`, so `build-config` is judged
rather than unavailable.

The health partition treats `UNAVAILABLE` as issues. Every finding that
depended on the missing descriptor must therefore become evaluable or
not-applicable; none may stay unavailable merely because the profile is native.

## Failure Boundaries

- `forge new --profile rust-product` refuses before any file or registry row.
- A capability request on a native profile is refused with the existing
  `incompatible-profile`.
- An unknown profile keeps the current `profile-known FAIL` and unavailable
  checks; only the declared native id changes.
- The CI surfaces job registers the repository in the runner's own registry;
  it writes no repository file.

## Verification Strategy

1. `cargo test -p forge --lib profile::` covers inspect/list/resolve/refusal.
2. `cargo test -p forge --lib doctor::` covers the native assessment.
3. `forge doctor .` on this repository reports `profile-known PASS` and
   `features-compatible PASS`, and the verdict is checked.
4. `forge register .` succeeds; `forge check forge --format json` emits an
   `alerts` document.
5. The repository suite, Gate, formatting and OpenSpec validation pass.
