#!/usr/bin/env node
// Browser oracle for the workbench deep link (`/workbench?project=`) and the
// login round-trip that preserves it.
//
// Usage:
//   node workbench-deep-link-check.mjs <web-base> <email> <password> <alpha> <beta> <gamma>
//
// <alpha>/<beta> are registered (managed) fixture projects, <gamma> is an
// unregistered workspace sibling. In real Chromium against the real API +
// web listeners the harness proves:
//
//  1. a managed fleet row's "Open" action navigates to
//     `/workbench?project=<identity>` with that project selected and loaded;
//  2. a direct `/workbench?project=<id>` load boots that project;
//  3. a reload keeps the URL and the loaded project;
//  4. an unknown `?project=` id loads nothing and says so;
//  5. back/forward across two managed ids load the URL's project each way;
//  6. `/management?project=<registered-id>` redirects to the workbench boot;
//  7. a signed-out `/management?project=<unregistered-id>` returns through
//     the login page and ticks that workspace row after sign-in;
//  8. a signed-out `/workbench?project=<managed-id>` returns through login
//     and loads that project after sign-in;
//  9. a hostile `?next=` off-origin value falls back to `index.html`.
//
// Exit codes: 0 verified, 1 a check failed, 2 Playwright or a browser engine
// is unavailable (`UNVERIFIED`). The Rust caller reports exit 2 as
// UNVERIFIED, never a pass.

const [webBase, email, password, alpha, beta, gamma] = process.argv.slice(2);

if (!webBase || !email || !password || !alpha || !beta || !gamma) {
  console.error('usage: workbench-deep-link-check.mjs <web-base> <email> <password> <alpha> <beta> <gamma>');
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
const note = (message) => console.log(`workbench-deep-link: ${message}`);

const DETAIL_TIMEOUT = 90000;

async function signIn(page) {
  await page.goto(`${webBase}/login.html`, { waitUntil: 'networkidle', timeout: 30000 });
  await page.getByLabel('Email address').fill(email);
  await page.getByLabel('Password').fill(password);
  await page.getByRole('button', { name: /sign in to your workspace/i }).click();
  await page.waitForURL('**/index.html', { timeout: 30000 });
}

async function expectBooted(page, id) {
  if ((await page.locator('#workbench:not([hidden])').count()) !== 1) {
    fail(`workbench view is not visible for ${id}`);
  }
  // The selector populates asynchronously from the fleet fetch; the deep
  // link applies once its options exist. Options are never "visible", so
  // wait for attachment, not visibility.
  await page.locator(`#workbench-project option[value="${id}"]`).first().waitFor({ state: 'attached', timeout: 30000 });
  const selected = await page.locator('#workbench-project').inputValue();
  if (selected !== id) fail(`workbench selector is ${selected}, want ${id}`);
  await page.locator('#workbench-body:not([hidden])').first().waitFor({ timeout: DETAIL_TIMEOUT });
}

async function expectFleetReady(page, id) {
  await page.locator(`#workbench-project option[value="${id}"]`).first().waitFor({ state: 'attached', timeout: 30000 });
}

let browser;
try {
  browser = await chromium.launch({ args: ['--no-sandbox'] });
} catch (err) {
  console.error(`UNVERIFIED: cannot launch Chromium: ${err.message}`);
  process.exit(2);
}

try {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();
  await signIn(page);
  note('signed in');

  // 1. The managed row's Open action carries the project to the workbench.
  await page.goto(`${webBase}/projects`, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.locator('#project-rows tr').first().waitFor({ timeout: 30000 });
  await page.waitForTimeout(1000);
  const open = page.locator('#project-rows tr', { hasText: alpha }).getByRole('button', { name: /open/i });
  await open.waitFor({ timeout: 30000 });
  await open.click();
  await page.waitForURL(`**/workbench?project=${alpha}`, { timeout: 30000 });
  await expectBooted(page, alpha);
  note('fleet Open boots the clicked managed project');

  // 2-3. Direct load boots, reload keeps.
  await page.goto(`${webBase}/workbench?project=${beta}`, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await expectBooted(page, beta);
  note('direct workbench deep link boots the project');
  await page.reload({ waitUntil: 'domcontentloaded' });
  if (!page.url().includes(`/workbench?project=${beta}`)) {
    fail(`reload lost the project parameter: ${page.url()}`);
  }
  await expectBooted(page, beta);
  note('reload keeps the workbench project');

  // 4. Unknown id loads nothing and says so.
  await page.goto(`${webBase}/workbench?project=no-such-project`, {
    waitUntil: 'domcontentloaded',
    timeout: 30000,
  });
  await expectFleetReady(page, beta);
  await page.waitForTimeout(1500);
  if ((await page.locator('#workbench-body:not([hidden])').count()) !== 0) {
    fail('unknown project id loaded a workbench detail');
  }
  if ((await page.locator('#workbench-empty:not([hidden])').count()) !== 1) {
    fail('unknown project id left the workbench without its empty state');
  }
  const unknownNotice = await page.locator('#workbench-notice').innerText();
  if (!unknownNotice.includes('no-such-project') || !/nothing was loaded/i.test(unknownNotice)) {
    fail(`unknown id is not handled honestly: ${unknownNotice}`);
  }
  note('unknown id loads nothing');

  // 5. Back/forward across two managed ids load the URL's project.
  await page.goto(`${webBase}/workbench?project=${alpha}`, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await expectBooted(page, alpha);
  await page.goto(`${webBase}/workbench?project=${beta}`, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await expectBooted(page, beta);
  await page.goBack({ waitUntil: 'domcontentloaded' });
  if (!page.url().includes(`project=${alpha}`)) fail(`back url wrong: ${page.url()}`);
  await expectBooted(page, alpha);
  await page.goForward({ waitUntil: 'domcontentloaded' });
  if (!page.url().includes(`project=${beta}`)) fail(`forward url wrong: ${page.url()}`);
  await expectBooted(page, beta);
  note('back/forward load the URL project');

  // 6. A registered id on the management view reaches the workbench.
  await page.goto(`${webBase}/management?project=${alpha}`, {
    waitUntil: 'domcontentloaded',
    timeout: 30000,
  });
  await page.waitForURL(`**/workbench?project=${alpha}`, { timeout: 30000 });
  await expectBooted(page, alpha);
  note('registered management param redirects to the workbench');

  // 7. Signed-out management deep link returns through login and boots the
  // project-scoped view (bulk hidden, nothing bulk-ticked).
  {
    const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
    const anon = await ctx.newPage();
    await anon.goto(`${webBase}/management?project=${gamma}`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await anon.waitForURL('**/login.html?next=**', { timeout: 30000 });
    await anon.getByLabel('Email address').fill(email);
    await anon.getByLabel('Password').fill(password);
    await anon.getByRole('button', { name: /sign in to your workspace/i }).click();
    await anon.waitForURL(`**/management?project=${gamma}`, { timeout: 30000 });
    await anon.locator('#mgmt-project:not([hidden])').waitFor({ timeout: 30000 });
    await anon.waitForTimeout(800);
    const bulkHidden = await anon.locator('#ws-bulk').evaluate((el) => el.hidden);
    if (!bulkHidden) {
      fail('login round-trip shows the bulk table on a project deep link');
    }
    if ((await anon.locator('#ws-rows input[type=checkbox]:checked').count()) !== 0) {
      fail('login round-trip ticked a bulk workspace row');
    }
    const notice = await anon.locator('#mgmt-project-notice').innerText();
    if (!notice.includes(gamma)) {
      fail(`login round-trip scoped notice does not name the project: ${notice}`);
    }
    await ctx.close();
  }
  note('signed-out management link boots after sign-in');

  // 8. Signed-out workbench deep link returns through login and loads.
  {
    const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
    const anon = await ctx.newPage();
    await anon.goto(`${webBase}/workbench?project=${beta}`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await anon.waitForURL('**/login.html?next=**', { timeout: 30000 });
    await anon.getByLabel('Email address').fill(email);
    await anon.getByLabel('Password').fill(password);
    await anon.getByRole('button', { name: /sign in to your workspace/i }).click();
    await anon.waitForURL(`**/workbench?project=${beta}`, { timeout: 30000 });
    await expectBooted(anon, beta);
    await ctx.close();
  }
  note('signed-out workbench link boots after sign-in');

  // 9. A hostile next value falls back to the dashboard, never off-origin.
  {
    const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
    const anon = await ctx.newPage();
    await anon.goto(`${webBase}/login.html?next=https://example.invalid/`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });
    await anon.getByLabel('Email address').fill(email);
    await anon.getByLabel('Password').fill(password);
    await anon.getByRole('button', { name: /sign in to your workspace/i }).click();
    await anon.waitForURL('**/index.html', { timeout: 30000 });
    if (!anon.url().startsWith(webBase)) fail(`hostile next escaped the origin: ${anon.url()}`);
    await ctx.close();
  }
  note('hostile next falls back to the dashboard');

  console.log('VERIFIED: workbench deep link boots the managed project and the login round-trip preserves it in Chromium');
} catch (err) {
  fail(err.message);
} finally {
  await browser.close();
}
