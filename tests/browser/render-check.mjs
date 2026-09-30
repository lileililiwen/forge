#!/usr/bin/env node
// Playwright DOM assertion for the Forge Studio `react-web` live preview.
//
// Usage:
//   node render-check.mjs <preview-url> [expected-text]
//
// Exit codes:
//   0  the app rendered; VERIFIED
//   1  the app rendered but did not match the expectation; FAIL
//   2  Playwright or a browser engine is unavailable; UNVERIFIED
//
// A static `import "playwright"` would throw before this file runs when the
// dependency is missing, so the import is dynamic and its failure is mapped
// to the `unverified` exit code the Rust harness understands.

const url = process.argv[2] ?? process.env.FORGE_STUDIO_PREVIEW_URL;
const expected = process.argv[3] ?? process.env.FORGE_STUDIO_EXPECT_TEXT ?? null;

if (!url) {
  console.error("UNVERIFIED: no preview URL supplied");
  process.exit(2);
}

let chromium;
try {
  ({ chromium } = await import("playwright"));
} catch (err) {
  console.error(`UNVERIFIED: Playwright is not installed (${err.code ?? err.message})`);
  process.exit(2);
}

let browser;
try {
  browser = await chromium.launch();
} catch (err) {
  console.error(`UNVERIFIED: could not launch a browser engine (${err.message})`);
  process.exit(2);
}

try {
  const page = await browser.newPage();
  await page.goto(url, { waitUntil: "networkidle", timeout: 30000 });
  await page.waitForSelector('[data-testid="forge-app"]', { timeout: 30000 });
  const text = (await page.textContent('[data-testid="forge-app"]'))?.trim() ?? "";
  if (expected && text !== expected) {
    console.error(
      `FAIL: rendered ${JSON.stringify(text)}, expected ${JSON.stringify(expected)}`,
    );
    process.exit(1);
  }
  console.log(`VERIFIED: rendered ${JSON.stringify(text)} from ${url}`);
} catch (err) {
  console.error(`FAIL: ${err.message}`);
  process.exit(1);
} finally {
  await browser.close();
}
