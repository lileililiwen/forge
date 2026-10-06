# Forge browser preview

The browser pages live in this directory and are served by Forge's Rust static web listener, separately from the Rust JSON API. From the repository root, use two terminals and one temporary registry:

```sh
cargo run -- --registry /tmp/forge-browser-preview.db identity setup --email you@example.com
```

Enter the Forge administrator password twice when prompted. Then start the API:

```sh
cargo run -- --registry /tmp/forge-browser-preview.db api serve --port 8766
```

Start the frontend web server in another terminal:

```sh
cargo run --offline -- web serve --bind 127.0.0.1 --port 4173 --root frontend
```

Open <http://127.0.0.1:4173/>. The Rust web server serves only frontend files; the Rust API at `http://127.0.0.1:8766` handles login and project JSON and accepts the default frontend origin `http://127.0.0.1:4173`. The frontend calls the API origin in `config.js` and sends the session cookie with credentialed requests. For another origin, set `FORGE_FRONTEND_ORIGIN` before starting the API and update `FORGE_API_BASE` in `config.js`. Use the same hostname for both URLs so the host-only cookie is shared across ports.

To preview an empty fleet, skip `identity setup`; the login page displays the exact setup command. To view registered projects, import them first using the same `--registry /tmp/forge-browser-preview.db` path.
