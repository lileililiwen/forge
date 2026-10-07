#!/usr/bin/env node
// Browser oracle for workspace onboarding (`forge-web-workspace-onboarding`).
//
// Usage:
//   node workspace-onboarding-check.mjs <web-base> <email> <password> \
//     <dir-one> <dir-two> <id-one> <id-two> <fixture-root>
//
// The harness signs in through the shipped login page, opens the workspace
// onboarding panel, discovers the two fixture directories, selects and
// previews them, confirms the batch, then reloads and verifies both projects
// appear in the fleet. It also checks keyboard entry, measured text contrast
// on the onboarding panel, and that the absolute fixture root is never
// rendered.
//
// Exit codes: 0 verified, 1 a check failed, 2 Playwright or a browser engine
// is unavailable (`UNVERIFIED`). The Rust caller reports exit 2 as
// UNVERIFIED, never a pass.

const [webBase, email, password, dirOne, dirTwo, idOne, idTwo, fixtureRoot] =
  process.argv.slice(2);

if (!webBase || !email || !password || !dirOne || !dirTwo || !idOne || !idTwo || !fixtureRoot) {
  console.error('usage: workspace-onboarding-check.mjs <web-base> <email> <password> <dir-one> <dir-two> <id-one> <id-two> <fixture-root>');
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

  await page.getByRole('button', { name: 'Discover workspace' }).click();
  const rows = page.locator('#ws-rows tr');
  await rows.first().waitFor({ timeout: 30000 });
  const tableText = await page.locator('#ws-rows').innerText();
  for (const dir of [dirOne, dirTwo]) {
    if (!tableText.includes(dir)) fail(`discovery did not list ${dir}: ${tableText}`);
  }
  note('discovered');

  // Keyboard-driven preview: focus the preview control and activate it.
  await page.getByRole('button', { name: 'Select all onboardable' }).click();
  const previewBtn = page.getByRole('button', { name: 'Preview selection' });
  await previewBtn.focus();
  await page.keyboard.press('Enter');
  const preview = page.locator('#ws-preview-result');
  await preview.getByText('Action digest:', { exact: false }).waitFor({ timeout: 30000 });
  const previewText = await preview.innerText();
  for (const id of [idOne, idTwo]) {
    if (!previewText.includes(id)) fail(`preview did not plan ${id}: ${previewText}`);
  }
  note('previewed');

  await page.locator('#ws-confirm').check();
  await page.getByRole('button', { name: 'Run confirmed onboarding' }).click();
  const result = page.locator('#ws-apply-result');
  await result.getByText('Onboarded 2 of 2', { exact: false }).waitFor({ timeout: 30000 });
  note('applied');

  await page.getByRole('button', { name: 'Reload project list' }).click();
  await page.waitForLoadState('networkidle', { timeout: 30000 });
  const fleet = page.locator('#project-rows');
  await fleet.getByText(idOne, { exact: false }).waitFor({ timeout: 30000 });
  const fleetText = await fleet.innerText();
  for (const id of [idOne, idTwo]) {
    if (!fleetText.includes(id)) fail(`fleet does not list ${id} after reload`);
  }
  note('fleet lists both');

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
