#!/usr/bin/env node
// Browser oracle for workspace onboarding (`forge-web-workspace-onboarding`).
//
// Usage:
//   node workspace-onboarding-check.mjs <web-base> <email> <password> \
//     <dirs-csv> <ids-csv> <fixture-root>
//
// The harness signs in through the shipped login page, expects the onboarding
// table to populate itself, selects everything, previews (one combined plan
// per 25-item chunk, digests held in JS memory and never rendered), confirms
// once, then verifies the fleet re-fetches in place with the results still
// visible and the manual Reload control retained as fallback. It also checks
// readable copy (name-first rows, sentence previews, no hash/digest/op-id
// text on screen), dashboard section order, keyboard entry, measured text
// contrast, and that the absolute fixture root is never rendered.
//
// Exit codes: 0 verified, 1 a check failed, 2 Playwright or a browser engine
// is unavailable (`UNVERIFIED`). The Rust caller reports exit 2 as
// UNVERIFIED, never a pass.

const [webBase, email, password, dirsCsv, idsCsv, fixtureRoot] =
  process.argv.slice(2);
const dirs = (dirsCsv || '').split(',').filter(Boolean);
const ids = (idsCsv || '').split(',').filter(Boolean);

if (!webBase || !email || !password || !dirs.length || !ids.length || !fixtureRoot) {
  console.error('usage: workspace-onboarding-check.mjs <web-base> <email> <password> <dirs-csv> <ids-csv> <fixture-root>');
  process.exit(2);
}

let chromium;
try {
  ({ chromium } = await import('playwright'));
} catch (err) {
  console.error(`UNVERIFIED: playwright is unavailable: ${err.message}`);
  process.exit(2);
}

const fail = (message) => {
  console.error(`FAIL: ${message}`);
  process.exit(1);
};
const note = (message) => console.log(`workspace-onboarding: ${message}`);

let browser;
try {
  browser = await chromium.launch({ args: ['--no-sandbox'] });
} catch (err) {
  console.error(`UNVERIFIED: cannot launch Chromium: ${err.message}`);
  process.exit(2);
}

const channel = (value) => {
  const v = value / 255;
  return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
};
const luminance = (color) =>
  0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b);
const contrast = (foreground, background) => {
  const high = Math.max(luminance(foreground), luminance(background));
  const low = Math.min(luminance(foreground), luminance(background));
  return (high + 0.05) / (low + 0.05);
};

try {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();

  await page.goto(`${webBase}/login.html`, { waitUntil: 'networkidle', timeout: 30000 });
  await page.keyboard.press('Tab');
  const firstFocus = await page.evaluate(() => document.activeElement?.className || '');
  if (!String(firstFocus).includes('skip-link')) {
    fail(`keyboard entry did not land on the skip link: ${firstFocus}`);
  }

  await page.getByLabel('Email address').fill(email);
  await page.getByLabel('Password').fill(password);
  await page.getByRole('button', { name: /sign in to your workspace/i }).click();
  await page.waitForURL('**/index.html', { timeout: 30000 });
  note('signed in');

  // Dashboard order: work comes before reference.
  const positions = await page.evaluate(() => {
    const out = {};
    for (const id of ['workbench-title', 'management-title', 'commands-title']) {
      const el = document.getElementById(id);
      out[id] = el ? el.getBoundingClientRect().top : -1;
    }
    return out;
  });
  if (!(positions['workbench-title'] < positions['management-title'] && positions['management-title'] < positions['commands-title'])) {
    fail(`dashboard order is not workbench < management < commands: ${JSON.stringify(positions)}`);
  }
  note('order ok');

  // Auto-discovery: no Discover click. The table populates itself.
  const rows = page.locator('#ws-rows tr');
  await rows.first().waitFor({ timeout: 30000 });
  const tableText = await page.locator('#ws-rows').innerText();
  for (const dir of dirs) {
    if (!tableText.includes(dir)) fail(`auto-discovery did not list ${dir}: ${tableText}`);
  }
  const hint = await page.locator('#ws-fleet-hint').innerText();
  if (!hint.includes('not yet onboarded')) fail(`fleet hint missing: ${hint}`);
  note('discovered');

  // Keyboard-driven preview: focus the preview control and activate it.
  await page.getByRole('button', { name: 'Select all onboardable' }).click();
  const previewBtn = page.getByRole('button', { name: 'Preview selection' });
  await previewBtn.focus();
  await page.keyboard.press('Enter');
  const preview = page.locator('#ws-preview-result');
  await preview.getByText('Preview — nothing has been written yet', { exact: false }).waitFor({ timeout: 30000 });
  // Both 25-item chunks must have landed before asserting on the text.
  await page.waitForFunction(
    (expected) => document.querySelectorAll('#ws-preview-result li').length === expected,
    dirs.length,
    { timeout: 30000 },
  );
  const previewText = await preview.innerText();
  for (const id of ids) {
    if (!previewText.includes(id)) fail(`preview did not plan ${id}`);
  }
  const planLines = await preview.locator('li').count();
  if (planLines !== dirs.length) fail(`preview shows ${planLines} plans for ${dirs.length} dirs`);
  // Human-readable preview: sentences naming the action and target, never
  // a digest, hash or operation id on screen.
  if (/[0-9a-f]{20,}/i.test(previewText)) fail(`preview rendered a hash-like value: ${previewText.slice(0, 200)}`);
  if (/digest:/i.test(previewText)) fail(`preview rendered a digest line: ${previewText.slice(0, 200)}`);
  if (/operation \d+/i.test(previewText)) fail(`preview rendered an operation id: ${previewText.slice(0, 200)}`);
  note('previewed');

  await page.locator('#ws-confirm').check();
  await page.getByRole('button', { name: 'Run confirmed onboarding' }).click();
  const result = page.locator('#ws-apply-result');
  await result.getByText(`Onboarded ${dirs.length} of ${dirs.length}`, { exact: false }).waitFor({ timeout: 60000 });
  const resultText = await result.innerText();
  if (/[0-9a-f]{20,}/i.test(resultText)) fail(`results rendered a hash-like value: ${resultText.slice(0, 200)}`);
  if (/digest:/i.test(resultText)) fail(`results rendered a digest line: ${resultText.slice(0, 200)}`);
  note('applied');

  // The fleet re-fetches in place: results stay visible and the fallback
  // Reload control remains, but no reload is needed to see the new rows.
  if (!(await result.isVisible())) fail('onboarding results did not stay visible after the fleet refresh');
  if (!(await page.getByRole('button', { name: 'Reload project list' }).isVisible())) {
    fail('manual Reload fallback is missing after onboarding');
  }
  const fleet = page.locator('#project-rows');
  await fleet.getByText(ids[0], { exact: false }).waitFor({ timeout: 30000 });
  const fleetText = await fleet.innerText();
  for (const id of ids) {
    if (!fleetText.includes(id)) fail(`fleet does not list ${id} without a reload`);
  }
  // Fleet rows lead with a human name and one plain status line — no
  // source/lifecycle/access/evidence code columns in the primary view.
  const fleetHeaders = await page.locator('#project-rows').evaluate(() => {
    const table = document.querySelector('.table-scroll table thead');
    return table ? table.innerText : '';
  });
  for (const code of ['Source', 'Lifecycle', 'Access', 'Evidence']) {
    if (fleetHeaders.includes(code)) fail(`fleet still shows a ${code} code column`);
  }
  if (/[0-9a-f]{20,}/i.test(fleetText)) fail(`fleet rendered a hash-like value: ${fleetText.slice(0, 200)}`);
  note('fleet lists all');

  const bodyText = await page.locator('#main-content').innerText();
  if (bodyText.includes(fixtureRoot)) {
    fail('dashboard rendered the absolute fixture root');
  }

  const samples = await page.evaluate(() => {
    const parse = (value) => {
      const match = value && value.match(/rgba?\(([^)]+)\)/);
      if (!match) return null;
      const parts = match[1].split(',').map((part) => parseFloat(part.trim()));
      return { r: parts[0], g: parts[1], b: parts[2], a: parts.length > 3 ? parts[3] : 1 };
    };
    const background = (element) => {
      let result = { r: 255, g: 255, b: 255, a: 1 };
      let current = element;
      while (current) {
        const color = parse(getComputedStyle(current).backgroundColor);
        if (color && color.a > 0) {
          result = {
            r: color.r * color.a + result.r * (1 - color.a),
            g: color.g * color.a + result.g * (1 - color.a),
            b: color.b * color.a + result.b * (1 - color.a),
            a: 1,
          };
        }
        current = current.parentElement;
      }
      return result;
    };
    const title = document.querySelector('#ws-title');
    const noteEl = document.querySelector('#ws-notice:not([hidden])') || document.querySelector('#management-title');
    return [title, noteEl].filter(Boolean).map((element) => ({
      text: (element.textContent || '').trim().slice(0, 48),
      foreground: parse(getComputedStyle(element).color),
      background: background(element),
    }));
  });
  for (const sample of samples) {
    if (!sample.foreground) fail(`unmeasurable onboarding text color: ${sample.text}`);
    const ratio = contrast(sample.foreground, sample.background);
    if (ratio + 0.02 < 4.5) {
      fail(`onboarding text contrast ${ratio.toFixed(2)}:1 below 4.5:1 for ${sample.text}`);
    }
  }

  console.log('VERIFIED: workspace onboarding completed in Chromium through the dashboard');
} catch (err) {
  fail(err.message);
} finally {
  await browser.close();
}
