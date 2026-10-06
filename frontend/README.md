# Forge browser preview

The browser pages live in this directory and are served by Forge's Rust static
web listener, separately from the Rust JSON API. Both listeners read the **same
registry database**, so you never pass a special database path to preview your
real projects.

## Where the registry lives

Forge uses one registry database by default; the web server and API server both
read it. Resolution order (src/registry/mod.rs `default_registry_path`):

1. `$FORGE_REGISTRY` if set and non-empty,
2. else `$XDG_DATA_HOME/forge/registry.db`,
3. else `~/.local/share/forge/registry.db`.

Do **not** put the registry under `/tmp` for normal use. If you want a throwaway
preview that leaves your real data untouched, point `FORGE_REGISTRY` at a scratch
file for all three commands below, e.g. `export FORGE_REGISTRY="$PWD/.preview.db"`.

## Run it (three steps, default registry)

From the repository root, in three terminals:

```sh
# 1. Initialize the one Forge-wide administrator (password read without echo,
#    min 12 chars, typed twice). Skip this if an admin already exists.
cargo run -- identity setup --email you@example.com

# 2. Serve the JSON API. Port 8766 matches the committed frontend/config.js.
cargo run -- api serve --port 8766

# 3. Serve the standalone frontend on its own listener.
cargo run -- web serve --bind 127.0.0.1 --port 4173 --root frontend
```

Open <http://127.0.0.1:4173/>, then sign in with the email and password from
step 1.

## How the two listeners connect

- The Rust **web** server (`forge web serve`) serves only files under `frontend/`.
  It runs no project logic and reads no database.
- The Rust **API** server (`forge api serve`) handles login and all project JSON
  at `http://127.0.0.1:8766`. It accepts exactly one browser origin for CORS and
  cookie issuance: `http://127.0.0.1:4173` (the `--port`/`--bind` web listener).
- `frontend/config.js` sets `window.FORGE_API_BASE` to the API origin. The
  frontend sends the session cookie with credentialed requests. The session
  cookie is host-scoped (`Path=/; HttpOnly; SameSite=Lax`), so **keep both
  listeners on the same hostname** (`127.0.0.1`) even though they use different
  ports.

To use a different origin or port, before starting the API set
`FORGE_FRONTEND_ORIGIN` to the exact web origin (e.g.
`http://127.0.0.1:4173`) and update `FORGE_API_BASE` in `frontend/config.js` to
the exact API origin. The API rejects requests from any other `Origin`.

## What you will see

- **No admin configured:** the sign-in page shows the exact
  `forge identity setup --email …` command to run first.
- **Empty registry:** after sign-in the dashboard shows just the Forge-self row.
- **Populated registry (your real case):** the fleet lists every registered
  project, and the Workbench, Portfolio, Commands and Delivery sections read the
  same registry the CLI does.

Optional read-only sources are environment-driven only, never directory scans:
`FORGE_INVENTORY_SOURCE` (portable inventory) and `FORGE_WORKSPACE_REGISTRY`
(workspace fleet observer). Leave them unset to preview your local registry.
