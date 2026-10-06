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

## Portal accessibility/reflow smoke

`portal-a11y-check.mjs` is the rendered-behavior oracle for
`portal-accessible-responsive-ui`. It uses the same pinned `playwright`
dependency and cached Chromium — no extra package is installed.

```sh
node portal-a11y-check.mjs <dir>
```

`<dir>` contains a `manifest.json` (`[{"name","file"}, …]`) and one HTML file
per portal page. The script loads each page from disk (the portal stylesheet is
inline, so no server is required) at 320, 375, 640, 768 and 1280 CSS px in both
light and dark colour schemes, and asserts: no document-level horizontal
overflow; one `main`/`header`/`footer`/`h1` and a named `nav`; a skip link that
targets the single main; ordered headings; labelled controls; captioned, scoped
tables inside named keyboard regions; a visible, unobscured focus indicator on
the early tab stops; and WCAG 2.2 AA contrast for sampled text/background pairs.
Representative full-page screenshots are written to `<dir>/screenshots`.

Exit codes match `render-check.mjs`: `0` verified, `1` a check failed, `2` the
browser is unavailable (`UNVERIFIED`). `tests/portal_browser_a11y.rs` renders
the shipped pages and invokes the script; it reports exit `2` as `UNVERIFIED`
rather than a pass.
