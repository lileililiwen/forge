#!/usr/bin/env node
// Real-browser check for Gate profile selection, request execution and result.

const [webBase, email, password, projectId] = process.argv.slice(2);
if (!webBase || !email || !password || !projectId) {
  console.error('usage: gate-profile-workbench-check.mjs <web-base> <email> <password> <project-id>');
  process.exit(2);
}

let chromium;
try {
  ({ chromium } = await import('playwright'));
} catch (err) {
  console.error(`UNVERIFIED: playwright unavailable: ${err.message}`);
  process.exit(2);
}

let browser;
try {
  browser = await chromium.launch({ args: ['--no-sandbox'] });
} catch (err) {
  console.error(`UNVERIFIED: Chromium unavailable: ${err.message}`);
  process.exit(2);
}

try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto(`${webBase}/login.html`, { waitUntil: 'networkidle', timeout: 30000 });
  await page.getByLabel('Email address').fill(email);
  await page.getByLabel('Password').fill(password);
  await page.getByRole('button', { name: /sign in to your workspace/i }).click();
  await page.waitForURL('**/index.html', { timeout: 30000 });
  await page.getByRole('link', { name: 'Workbench', exact: true }).click();
  await page.waitForURL('**/workbench', { timeout: 30000 });
  await page.locator('#workbench:not([hidden])').waitFor({ timeout: 30000 });
  await page.locator(`#workbench-project option[value="${projectId}"]`).waitFor({ state: 'attached' });
  await page.locator('#workbench-project').selectOption(projectId);
  await page.getByRole('button', { name: 'Open project' }).click();

  const profile = page.getByLabel('Gate profile');
  await profile.waitFor({ state: 'visible', timeout: 30000 });
  if ((await profile.inputValue()) !== 'full') throw new Error('Gate profile selector must default to full');
  await profile.focus();
  await page.keyboard.press('ArrowDown');
  if ((await profile.inputValue()) !== 'fast') throw new Error('keyboard selection did not choose fast');
  const rerun = page.getByRole('button', { name: 'Re-run gate' });
  await rerun.focus();
  const focused = await rerun.evaluate((button) => {
    const style = getComputedStyle(button);
    return document.activeElement === button && style.outlineStyle !== 'none' && parseFloat(style.outlineWidth) > 0;
  });
  if (!focused) throw new Error('rerun button does not expose a visible keyboard focus indicator');
  await page.keyboard.press('Enter');
  await page.getByText('fast', { exact: true }).waitFor({ timeout: 30000 });
  await page.route('**/gate/run', (route) => route.fulfill({
    status: 400,
    contentType: 'application/json',
    body: JSON.stringify({ error: { code: 'gate-profile-invalid', message: 'Profile request rejected for browser recovery check.' } }),
  }));
  await profile.selectOption('full');
  await page.getByRole('button', { name: 'Re-run gate' }).click();
  await page.getByText('Profile request rejected for browser recovery check.').waitFor({ timeout: 30000 });
  const preserved = await page.locator('#wb-gate-status').innerText();
  if (!preserved.includes('fast')) throw new Error(`failed rerun replaced prior evidence: ${preserved}`);

  await page.setViewportSize({ width: 320, height: 760 });
  const quality = await page.locator('.wb-quality').evaluate((card) => ({
    right: card.getBoundingClientRect().right,
    scrollWidth: card.scrollWidth,
    clientWidth: card.clientWidth,
    viewportWidth: document.documentElement.clientWidth,
    profileRight: document.querySelector('#wb-gate-profile').getBoundingClientRect().right,
    offenders: [...card.querySelectorAll('*')]
      .filter((node) => node.getBoundingClientRect().right > card.getBoundingClientRect().right + 1)
      .slice(0, 8)
      .map((node) => `${node.tagName.toLowerCase()}#${node.id}.${String(node.className).replace(/\s+/g, '.')} right=${Math.round(node.getBoundingClientRect().right)} width=${Math.round(node.getBoundingClientRect().width)}`),
  }));
  if (quality.right > quality.viewportWidth + 1 || quality.scrollWidth > quality.clientWidth + 1 || quality.profileRight > quality.viewportWidth + 1) {
    throw new Error(`Quality card overflows horizontally at 320 CSS px: ${JSON.stringify(quality)}`);
  }
  const status = await page.locator('#wb-gate-status').innerText();
  if (!status.includes('Profile') || !status.includes('fast')) throw new Error(`fast profile not displayed: ${status}`);
  console.log('VERIFIED: labelled profile control, keyboard fast selection, real Gate run, preserved error state and 320px reflow');
} catch (err) {
  console.error(`FAIL: ${err.message}`);
  process.exitCode = 1;
} finally {
  await browser.close();
}
