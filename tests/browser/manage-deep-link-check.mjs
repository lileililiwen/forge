#!/usr/bin/env node
// Browser oracle for the project-scoped management view
// (`/management?project=<identity>`).
//
// Usage:
//   node manage-deep-link-check.mjs <web-base> <email> <password> <project-id>
//
// The harness signs in through the shipped login page and proves, in real
// Chromium against the real API + web listeners:
//
//  1. the fleet's per-row "Manage" action links to
//     `/management?project=<identity>` (the project identity is in the URL);
//  2. clicking it lands on `/management?project=<id>` with the management
//     view visible, the PROJECT-SCOPED card shown first, and the global
//     bulk table hidden (a detail visitor never meets the bulk panel as
//     the primary content);
//  3. the scoped card names the project, summarizes that candidate, and
//     offers only that project's preview action — no bulk row is ticked;
//  4. the scoped preview mentions only that project;
//  5. plain `/management` still renders the global bulk view (bulk shown,
//     scoped hidden);
//  6. an unknown `?project=` id renders the scoped safe notice and ticks
//     nothing — the wrong project is never loaded;
//  7. reload keeps the scoped view; back/forward reconcile scoped with
//     the URL.
//
// Exit codes: 0 verified, 1 a check failed, 2 Playwright or a browser engine
// is unavailable (`UNVERIFIED`). The Rust caller reports exit 2 as
// UNVERIFIED, never a pass.

const [webBase, email, password, projectId] = process.argv.slice(2);

if (!webBase || !email || !password || !projectId) {
  console.error('usage: manage-deep-link-check.mjs <web-base> <email> <password> <project-id>');
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
const note = (message) => console.log(`manage-deep-link: ${message}`);

let browser;
try {
  browser = await chromium.launch({ args: ['--no-sandbox'] });
} catch (err) {
  console.error(`UNVERIFIED: cannot launch Chromium: ${err.message}`);
  process.exit(2);
}

const checkedCount = (page) =>
  page.locator('#ws-rows input[type=checkbox]:checked').count();
const isHidden = (page, selector) =>
  page.locator(selector).evaluate((el) => el.hidden);

try {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();

  await page.goto(`${webBase}/login.html`, { waitUntil: 'networkidle', timeout: 30000 });
  await page.getByLabel('Email address').fill(email);
  await page.getByLabel('Password').fill(password);
  await page.getByRole('button', { name: /sign in to your workspace/i }).click();
  await page.waitForURL('**/index.html', { timeout: 30000 });
  note('signed in');

  // The fleet must list the unmanaged fixture project with a Manage action.
  await page.goto(`${webBase}/projects`, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.locator('#project-rows tr').first().waitFor({ timeout: 30000 });
  await page.waitForTimeout(1000);
  const manage = page.locator('#project-rows a', { hasText: 'Manage' }).first();
  await manage.waitFor({ timeout: 30000 });
  const href = await manage.getAttribute('href');
  if (href !== `/management?project=${projectId}`) {
    fail(`Manage link does not carry the project: href is ${href}`);
  }
  note(`Manage href carries the project (${href})`);

  // Clicking it must land on the project-scoped view, not the bulk panel.
  await manage.click();
  await page.waitForURL(`**/management?project=${projectId}`, { timeout: 30000 });
  if ((await page.locator('#management:not([hidden])').count()) !== 1) {
    fail('management view is not visible after clicking Manage');
  }
  await page.locator('#mgmt-project:not([hidden])').waitFor({ timeout: 30000 });
  await page.waitForTimeout(800);
  if (await isHidden(page, '#mgmt-project')) fail('project-scoped card is hidden on a deep link');
  if (!(await isHidden(page, '#ws-bulk'))) fail('bulk table is the primary content on a project deep link');
  const scopedNotice = await page.locator('#mgmt-project-notice').innerText();
  if (!scopedNotice.includes(projectId)) {
    fail(`scoped notice does not name the project: ${scopedNotice}`);
  }
  const scopedDetail = await page.locator('#mgmt-project-detail').innerText();
  if (!scopedDetail.includes(projectId)) {
    fail(`scoped summary does not describe the project: ${scopedDetail}`);
  }
  if ((await checkedCount(page)) !== 0) {
    fail('deep link ticked a bulk workspace row');
  }
  const previewBtn = page.locator('#mgmt-project-preview');
  if (await previewBtn.isDisabled()) fail('scoped preview is not offered for the onboardable project');
  note('destination renders the project-scoped view with the bulk hidden');

  // The scoped preview covers only that project.
  await previewBtn.click();
  await page.locator('#mgmt-project-preview-result li').first().waitFor({ timeout: 30000 });
  const previewText = await page.locator('#mgmt-project-preview-result').innerText();
  if (!previewText.includes(projectId)) {
    fail(`scoped preview does not cover the project: ${previewText}`);
  }
  if ((await checkedCount(page)) !== 0) {
    fail('scoped preview ticked a bulk workspace row');
  }
  note('scoped preview covers only the requested project');

  // Reload must keep the scoped view and the URL.
  await page.reload({ waitUntil: 'domcontentloaded' });
  await page.locator('#mgmt-project:not([hidden])').waitFor({ timeout: 30000 });
  await page.waitForTimeout(800);
  if (!page.url().includes(`/management?project=${projectId}`)) {
    fail(`reload lost the project parameter: ${page.url()}`);
  }
  if (await isHidden(page, '#mgmt-project')) fail('reload lost the project-scoped view');
  if (!(await isHidden(page, '#ws-bulk'))) fail('reload restored the bulk table on a deep link');
  if ((await checkedCount(page)) !== 0) fail('reload ticked a bulk row');
  note('reload keeps the project-scoped view');

  // Plain /management still renders the global bulk view.
  await page.goto(`${webBase}/management`, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.locator('#ws-rows tr').first().waitFor({ timeout: 30000 });
  await page.waitForTimeout(800);
  if (!(await isHidden(page, '#mgmt-project'))) fail('plain /management shows the scoped card');
  if (await isHidden(page, '#ws-bulk')) fail('plain /management hides the bulk table');
  note('plain /management still renders the global bulk view');

  // An unknown id must render the scoped safe notice and tick nothing.
  await page.goto(`${webBase}/management?project=no-such-project`, {
    waitUntil: 'domcontentloaded',
    timeout: 30000,
  });
  await page.locator('#mgmt-project:not([hidden])').waitFor({ timeout: 30000 });
  await page.waitForTimeout(800);
  if (!(await isHidden(page, '#ws-bulk'))) fail('unknown id shows the bulk table');
  if ((await checkedCount(page)) !== 0) {
    fail('unknown project id ticked a workspace row');
  }
  const unknownNotice = await page.locator('#mgmt-project-notice').innerText();
  if (!unknownNotice.includes('no-such-project') || !/nothing was selected/i.test(unknownNotice)) {
    fail(`unknown id is not handled honestly: ${unknownNotice}`);
  }
  if (!(await page.locator('#mgmt-project-preview').isDisabled())) {
    fail('unknown id offers a preview action');
  }
  note('unknown id selects nothing');

  // Back/forward reconcile the scoped view with the URL.
  await page.goto(`${webBase}/management?project=${projectId}`, {
    waitUntil: 'domcontentloaded',
    timeout: 30000,
  });
  await page.locator('#mgmt-project:not([hidden])').waitFor({ timeout: 30000 });
  await page.waitForTimeout(800);
  await page.goto(`${webBase}/management`, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.locator('#ws-bulk:not([hidden])').waitFor({ timeout: 30000 });
  await page.waitForTimeout(800);
  if (!(await isHidden(page, '#mgmt-project'))) fail('leaving the deep link kept the scoped card');
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await page.locator('#mgmt-project:not([hidden])').waitFor({ timeout: 30000 });
  await page.waitForTimeout(800);
  if (!page.url().includes(`project=${projectId}`)) fail(`back url wrong: ${page.url()}`);
  if (await isHidden(page, '#mgmt-project')) fail('back did not restore the scoped view');
  await page.goForward({ waitUntil: 'domcontentloaded' });
  await page.locator('#ws-bulk:not([hidden])').waitFor({ timeout: 30000 });
  await page.waitForTimeout(800);
  if (!(await isHidden(page, '#mgmt-project'))) fail('forward did not return to the bulk view');
  note('back/forward reconcile the scoped view');

  console.log('VERIFIED: project-scoped management view leads on ?project= and plain /management stays global in Chromium');
} catch (err) {
  fail(err.message);
} finally {
  await browser.close();
}
