# Browser smoke for the react-web live preview

`render-check.mjs` loads a running preview URL in a real Chromium engine and
asserts that the mounted React component (`[data-testid="forge-app"]`) renders
the scaffold greeting. It is the browser oracle for the `react-web-live-preview`
change; the client is rendered by JavaScript, so an HTTP GET alone cannot prove
it.

## Setup

```sh
cd tests/browser
npm install
npx playwright install chromium   # browsers may already be cached
```

## Run against a preview

```sh
node render-check.mjs "http://127.0.0.1:<port>/" "hello from <project-id>"
```

Exit codes: `0` verified, `1` render mismatch, `2` Playwright or a browser
engine is unavailable (`UNVERIFIED`). The Rust harness
(`tests/react_web_native_preview.rs`) invokes this script when
`FORGE_NATIVE_REACT_WEB_PREVIEW=1` is set, and reports exit `2` as `UNVERIFIED`
rather than a pass.
