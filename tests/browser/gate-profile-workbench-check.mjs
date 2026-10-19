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
  const advanced = page.locator('#assurance-advanced');
  await advanced.locator('summary').click();
  const registry = page.locator('#assurance-registry');
  await registry.selectOption('contracts');
  await page.getByRole('button', { name: 'List or read' }).click();
  const familyList = page.locator('#assurance-list-result');
  await familyList.locator('dd').filter({ hasText: 'platform.job-outcome' }).waitFor({ timeout: 30000 });
  const listedFamilies = await familyList.innerText();
  if (!listedFamilies.includes('unsupported') || !listedFamilies.includes('no job outcome')) {
    throw new Error(`unsupported family status/reason missing: ${listedFamilies}`);
  }
  const inspectId = page.locator('#assurance-inspect-id');
  await inspectId.fill('platform.job-outcome');
  await page.locator('#assurance-inspect').click();
  await page.locator('#assurance-error-summary').getByText(/no job outcome source record/i).waitFor({ timeout: 30000 });
  await inspectId.fill('platform.gate-result');
  await page.locator('#assurance-inspect').click();
  await page.locator('#assurance-inspect-result').getByText(/gate_id/).waitFor({ timeout: 30000 });
  await page.setViewportSize({ width: 320, height: 760 });
  const assuranceWidth = await page.evaluate(() => ({
    viewport: document.documentElement.clientWidth,
    scroll: document.documentElement.scrollWidth,
  }));
  if (assuranceWidth.scroll > assuranceWidth.viewport + 1) {
    throw new Error(`assurance contract catalog overflows at 320px: ${JSON.stringify(assuranceWidth)}`);
  }
  await page.setViewportSize({ width: 1280, height: 800 });
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
  await page.getByRole('link', { name: 'Forge modules' }).click();
  await page.waitForURL('**/modules', { timeout: 30000 });
  await page.locator('#modules:not([hidden])').waitFor({ timeout: 30000 });
  let catalogMode = 'unavailable';
  await page.route('**/v1/admin/modules', (route) => {
    if (catalogMode === 'unavailable') return route.fulfill({ status: 503, contentType: 'application/json', body: JSON.stringify({ error: { code: 'module-catalog-unavailable', message: 'fixture catalog unavailable' } }) });
    const modules = catalogMode === 'empty' ? [] : [{ id: 'browser-fixture', stack: 'test', profile: 'fixture', path: 'modules/browser-fixture', gate: { aggregate: 'unverified', freshness: 'absent', checks: [] } }];
    return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ contract: 'forge-module-workflow/0.1.0', modules }) });
  });
  let moduleRuns = 0;
  let moduleGate = { aggregate: 'unverified', freshness: 'absent', checks: [] };
  await page.route('**/v1/admin/modules/browser-fixture/gate/status', (route) => route.fulfill({
    status: 200,
    contentType: 'application/json',
    body: JSON.stringify({ contract: 'forge-module-workflow/0.1.0', gate: moduleGate }),
  }));
  await page.route('**/v1/admin/modules/browser-fixture/gate/run', async (route) => {
    moduleRuns += 1;
    await new Promise((resolve) => setTimeout(resolve, 150));
    if (moduleRuns === 3) return route.fulfill({ status: 503, contentType: 'application/json', body: JSON.stringify({ error: { code: 'module-gate-unavailable', message: 'fixture runtime unavailable' } }) });
    const aggregate = moduleRuns === 1 ? 'passed' : 'blocked';
    moduleGate = { aggregate, freshness: 'fresh', revision: '0123456789abcdef', observed_at: '2026-10-19T00:00:00Z', checks: [{ id: 'fixture-check', state: aggregate === 'passed' ? 'pass' : 'fail', note: 'fixture evidence' }] };
    return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ contract: 'forge-module-workflow/0.1.0', gate: moduleGate }) });
  });
  await page.locator('#modules-refresh').click();
  await page.getByText('fixture catalog unavailable').waitFor({ timeout: 30000 });
  catalogMode = 'empty';
  await page.locator('#modules-refresh').click();
  await page.getByText('No Forge modules with Gate declarations were found.').waitFor({ timeout: 30000 });
  catalogMode = 'populated';
  await page.locator('#modules-refresh').click();
  await page.getByRole('button', { name: /Open browser-fixture/ }).waitFor({ timeout: 30000 });
  await page.getByRole('button', { name: /Open browser-fixture/ }).click();
  const run = page.getByRole('button', { name: 'Run Gate' });
  const moduleProfile = page.locator('#module-gate-profile');
  await moduleProfile.focus();
  await page.keyboard.press('ArrowUp'); // Choose fast from the default full profile.
  if ((await moduleProfile.inputValue()) !== 'fast') throw new Error('keyboard module profile selection did not choose fast');
  await run.focus();
  await page.waitForFunction(() => document.activeElement?.id === 'module-gate-run');
  const moduleFocus = await run.evaluate((button) => {
    const style = getComputedStyle(button);
    return document.activeElement === button && style.outlineStyle !== 'none' && parseFloat(style.outlineWidth) > 0;
  });
  if (!moduleFocus) throw new Error('module Gate action has no visible keyboard focus');
  await page.keyboard.press('Enter');
  await page.getByText('Running fast Gate for browser-fixture…').waitFor({ timeout: 30000 });
  if (!(await run.isDisabled())) throw new Error('module Gate action stayed enabled during a run');
  await page.getByText('Gate passed', { exact: false }).waitFor({ timeout: 30000 });
  const firstModuleStatus = await page.locator('#module-detail-status').innerText();
  if (!firstModuleStatus.includes('passed')) throw new Error(`keyboard module Gate run did not render PASS: ${firstModuleStatus}`);
  await page.getByRole('button', { name: 'Run Gate' }).click();
  await page.getByText('Gate blocked', { exact: false }).waitFor({ timeout: 30000 });
  if (!(await page.locator('#module-gate-evidence').innerText()).includes('fixture evidence')) throw new Error('module check detail was not rendered');
  await page.getByRole('button', { name: 'Run Gate' }).click();
  await page.getByText('fixture runtime unavailable').waitFor({ timeout: 30000 });
  if (!(await page.locator('#module-gate-evidence').innerText()).includes('blocked')) throw new Error('failed rerun removed the last known module evidence');
  await page.setViewportSize({ width: 360, height: 760 });
  const moduleWidth = await page.evaluate(() => ({ viewport: document.documentElement.clientWidth, scroll: document.documentElement.scrollWidth }));
  if (moduleWidth.scroll > moduleWidth.viewport + 1) throw new Error(`Modules page overflows at 360px: ${JSON.stringify(moduleWidth)}`);
  console.log('VERIFIED: contract list and inspect states, 320px reflow, keyboard Gate profile selection, real project Gate run, preserved error state, authenticated module inventory and 360px Modules reflow');
} catch (err) {
  console.error(`FAIL: ${err.message}`);
  process.exitCode = 1;
} finally {
  await browser.close();
}
