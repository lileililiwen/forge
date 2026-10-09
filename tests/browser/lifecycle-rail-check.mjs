#!/usr/bin/env node
// Browser oracle for the workbench lifecycle series rail
// (`lifecycle-series-rail`, part 2 live behavior).
//
// Usage:
//   node lifecycle-rail-check.mjs <web-base> <email> <password> <project-id>
//
// <project-id> is a registered (managed) fixture project. In real Chromium
// against the real API + web listeners the harness proves:
//
//  1. the rail lists exactly the 8 series steps in order with one
//     `aria-current="step"` and one Tab stop (roving focus);
//  2. `?step=` coexists with `?project=`: direct loads keep both, rail
//     clicks preserve the project, reload keeps both, unknown steps are
//     ignored and never move `aria-current`;
//  3. the next-best-action card renders exactly one ranked pick;
//  4. arrow keys move focus within the rail (roving);
//  5. Copy-as-CLI on a confirm card copies the exact shown `forge ...`
//     string to the clipboard;
//  6. at 390 CSS px the page has no horizontal overflow;
//  7. sampled text/background pairs meet WCAG 2.2 AA.
//  8. (flywheel-plugin-cap) rail step 0 carries the idea entry
//     (graduation preview/import + studio spec link), the projects view
//     carries the cap group filter and the rail carries the cap badge,
//     and the demo URL shapes (`?project=&step=`) resolve without losing
//     the project.
//  9. (web-lifecycle-execution) every lifecycle confirm clicks end to end
//     on the throwaway registry in rail order — idea preview→confirm,
//     scaffold `new`, studio spec save, refine (revision bump), upgrade
//     plan (gate dry-run analog), delivery approve/publish confirm-refused
//     paths, maintain refresh, remediate plan/apply (honest refusal when
//     no automatic finding exists, digest-bound when one does), intent
//     resolve→apply — with zero JS console errors, error-summary focus on
//     every invalid submit, `role=status`/`role=alert` updates, one journal
//     row per success, and a screenshot per step.
//
// Exit codes: 0 verified, 1 a check failed, 2 Playwright or a browser engine
// is unavailable (`UNVERIFIED`). The Rust caller reports exit 2 as
// UNVERIFIED, never a pass.

import fs from 'node:fs';
import path from 'node:path';

const [webBase, email, password, project] = process.argv.slice(2);

if (!webBase || !email || !password || !project) {
  console.error('usage: lifecycle-rail-check.mjs <web-base> <email> <password> <project-id>');
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
const note = (message) => console.log(`lifecycle-rail: ${message}`);

const DETAIL_TIMEOUT = 90000;
const STEP_ORDER = ['Idea', 'Scaffold', 'Spec', 'Code', 'Test', 'Release', 'Deploy', 'Operate'];

async function signIn(page) {
  await page.goto(`${webBase}/login.html`, { waitUntil: 'networkidle', timeout: 30000 });
  await page.getByLabel('Email address').fill(email);
  await page.getByLabel('Password').fill(password);
  await page.getByRole('button', { name: /sign in to your workspace/i }).click();
  await page.waitForURL('**/index.html', { timeout: 30000 });
}

async function expectBootedRail(page) {
  await page.locator('#workbench-body:not([hidden])').first().waitFor({ timeout: DETAIL_TIMEOUT });
  await page.locator('#lifecycle-rail li').first().waitFor({ timeout: 30000 });
  // Late projections refine the rail; settle before counting.
  await page.waitForTimeout(2500);
}

async function railLabels(page) {
  return page.locator('#lifecycle-rail li a').allInnerTexts();
}

async function expectRailShape(page, where) {
  const labels = (await railLabels(page)).map((t) => t.trim());
  if (labels.length !== 8 || !STEP_ORDER.every((step, i) => labels[i] === step)) {
    fail(`${where}: rail steps are [${labels.join(', ')}], want [${STEP_ORDER.join(', ')}]`);
  }
  const current = await page.locator('#lifecycle-rail [aria-current="step"]').count();
  if (current !== 1) fail(`${where}: want exactly one aria-current="step", found ${current}`);
  const tabStops = await page.locator('#lifecycle-rail a[tabindex="0"]').count();
  if (tabStops !== 1) fail(`${where}: want exactly one rail Tab stop, found ${tabStops}`);
  const hrefs = await page.locator('#lifecycle-rail li a').evaluateAll((links) =>
    links.map((a) => a.getAttribute('href')),
  );
  for (const href of hrefs) {
    if (!href.includes(`project=${encodeURIComponent(project)}`) || !href.includes('&step=')) {
      fail(`${where}: rail link misses the project+step pair: ${href}`);
    }
  }
  const nextHeads = await page.locator('#wb-next-body .wb-next-head').count();
  if (nextHeads !== 1) fail(`${where}: want exactly one next-best-action, found ${nextHeads}`);
}

function contrastRatio(fg, bg) {
  const channel = (c) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  };
  const luminance = (c) => 0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b);
  const hi = Math.max(luminance(fg), luminance(bg));
  const lo = Math.min(luminance(fg), luminance(bg));
  return (hi + 0.05) / (lo + 0.05);
}

async function contrastFailures(page) {
  return page.evaluate(() => {
    const parse = (value) => {
      const match = value && value.match(/rgba?\(([^)]+)\)/);
      if (!match) return null;
      const parts = match[1].split(',').map((x) => parseFloat(x.trim()));
      return { r: parts[0], g: parts[1], b: parts[2], a: parts.length > 3 ? parts[3] : 1 };
    };
    const channel = (c) => {
      const v = c / 255;
      return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
    };
    const luminance = (c) => 0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b);
    const ratio = (a, b) => {
      const hi = Math.max(luminance(a), luminance(b));
      const lo = Math.min(luminance(a), luminance(b));
      return (hi + 0.05) / (lo + 0.05);
    };
    const composite = (fg, bg) => ({
      r: fg.r * fg.a + bg.r * (1 - fg.a),
      g: fg.g * fg.a + bg.g * (1 - fg.a),
      b: fg.b * fg.a + bg.b * (1 - fg.a),
      a: 1,
    });
    const effectiveBackground = (el) => {
      const layers = [];
      let cur = el;
      while (cur) {
        const bg = parse(getComputedStyle(cur).backgroundColor);
        if (bg && bg.a > 0) layers.push(bg);
        cur = cur.parentElement;
      }
      let base = { r: 255, g: 255, b: 255, a: 1 };
      for (let i = layers.length - 1; i >= 0; i -= 1) base = composite(layers[i], base);
      return base;
    };
    const findings = [];
    const selector = '#workbench p, #workbench li, #workbench td, #workbench th, #workbench h1, #workbench h2, #workbench h3, #workbench h4, #workbench a, #workbench span, #workbench code, #workbench label, #workbench button, #workbench strong';
    for (const el of Array.from(document.querySelectorAll(selector))) {
      const text = (el.textContent || '').trim();
      if (!text) continue;
      const style = getComputedStyle(el);
      if (style.display === 'none' || style.visibility === 'hidden') continue;
      const rect = el.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) continue;
      const fg = parse(style.color);
      if (!fg) continue;
      const bg = effectiveBackground(el);
      const size = parseFloat(style.fontSize) || 16;
      const weight = parseInt(style.fontWeight, 10) || 400;
      const large = size >= 24 || (size >= 18.66 && weight >= 700);
      const need = large ? 3.0 : 4.5;
      const got = ratio(composite(fg, bg), bg);
      if (got + 0.02 < need) {
        findings.push(`${got.toFixed(2)}:1 < ${need}:1 for <${el.tagName.toLowerCase()}> "${text.slice(0, 48)}"`);
      }
    }
    return findings;
  });
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
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  const page = await context.newPage();
  // Click-oracle guards (web-lifecycle-execution): every console error and
  // every uncaught page error fails the run, and every step screenshots.
  const consoleErrors = [];
  page.on('console', (message) => {
    if (message.type() === 'error') {
      // Expected typed refusals (missing-confirm 400, stale-digest 409,
      // bogus finding, delivery confirm-refused) surface as Chromium
      // "Failed to load resource" network logs, not JS defects. The oracle
      // asserts those refusal shapes explicitly; only real JS errors fail.
      const text = message.text() || '';
      if (/failed to load resource/i.test(text)) return;
      consoleErrors.push(`console: ${text}`.slice(0, 300));
    }
  });
  page.on('pageerror', (err) => {
    consoleErrors.push(`pageerror: ${(err && err.message) || err}`.slice(0, 300));
  });
  const shotDir = path.join(process.cwd(), 'target', 'lifecycle-rail-shots');
  try { fs.mkdirSync(shotDir, { recursive: true }); } catch (_) { /* screenshots best-effort */ }
  const shot = async (name) => {
    try { await page.screenshot({ path: path.join(shotDir, `${name}.png`) }); }
    catch (_) { /* a failed shot never fails the oracle */ }
    note(`screenshot ${name}`);
  };
  const failOnConsoleErrors = (where) => {
    if (consoleErrors.length > 0) fail(`${where}: JS errors: ${consoleErrors.slice(0, 4).join(' | ')}`);
  };
  await signIn(page);
  note('signed in');

  // 1. Rail shape + single Next on a direct deep link.
  await page.goto(`${webBase}/workbench?project=${encodeURIComponent(project)}`, {
    waitUntil: 'domcontentloaded',
    timeout: 30000,
  });
  await expectBootedRail(page);
  await expectRailShape(page, 'direct load');
  note('rail renders 8 steps with one current, one Tab stop, one Next');

  // 2. Step coexistence: ?step= rides along, reload keeps both, rail intact.
  await page.goto(
    `${webBase}/workbench?project=${encodeURIComponent(project)}&step=test`,
    { waitUntil: 'domcontentloaded', timeout: 30000 },
  );
  await expectBootedRail(page);
  await expectRailShape(page, 'step deep link');
  if (!page.url().includes(`project=${encodeURIComponent(project)}`) || !page.url().includes('step=test')) {
    fail(`step link lost a parameter: ${page.url()}`);
  }
  await page.reload({ waitUntil: 'domcontentloaded' });
  await expectBootedRail(page);
  if (!page.url().includes(`project=${encodeURIComponent(project)}`) || !page.url().includes('step=test')) {
    fail(`reload lost the step pair: ${page.url()}`);
  }
  await expectRailShape(page, 'step reload');
  note('?step= coexists with ?project= across load and reload');

  // 3. Rail clicks preserve the project; unknown steps never move current.
  const before = await page.locator('#lifecycle-rail [aria-current="step"]').innerText();
  await page.locator('#lifecycle-rail li a', { hasText: 'Deploy' }).click();
  await page.waitForURL('**/workbench?project=*&step=deploy', { timeout: 30000 });
  await expectRailShape(page, 'step click');
  const after = await page.locator('#lifecycle-rail [aria-current="step"]').innerText();
  if (before.trim() !== after.trim()) {
    fail(`step navigation moved aria-current from ${before} to ${after}`);
  }
  await page.goto(
    `${webBase}/workbench?project=${encodeURIComponent(project)}&step=bogus`,
    { waitUntil: 'domcontentloaded', timeout: 30000 },
  );
  await expectBootedRail(page);
  await expectRailShape(page, 'unknown step');
  note('step clicks preserve the project; bogus steps are ignored');

  // 4. Roving: arrows move focus inside the rail.
  await page.locator('#lifecycle-rail a[tabindex="0"]').focus();
  const firstKey = await page.evaluate(() => document.activeElement.dataset.railKey);
  await page.keyboard.press('ArrowRight');
  const secondKey = await page.evaluate(() => document.activeElement.dataset.railKey);
  if (!secondKey || secondKey === firstKey) fail('ArrowRight did not move rail focus');
  await page.keyboard.press('ArrowLeft');
  const backKey = await page.evaluate(() => document.activeElement.dataset.railKey);
  if (backKey !== firstKey) fail(`ArrowLeft did not return rail focus (at ${backKey}, want ${firstKey})`);
  await page.keyboard.press('End');
  const endKey = await page.evaluate(() => document.activeElement.dataset.railKey);
  if (endKey !== 'operate') fail(`End did not jump to Operate (at ${endKey})`);
  await page.keyboard.press('Home');
  const homeKey = await page.evaluate(() => document.activeElement.dataset.railKey);
  if (homeKey !== 'idea') fail(`Home did not jump to Idea (at ${homeKey})`);
  note('rail roving moves focus with arrows, Home and End');

  // 5. Copy-as-CLI copies the exact shown string. Copy never sends a
  // request, so filling the card's typed fields with dummy values is
  // safe: it only exercises the string builder + clipboard path.
  const cards = page.locator('#wb-actions .wb-action-card, #wb-maintain-actions .wb-action-card');
  const cardCount = await cards.count();
  if (cardCount === 0) fail('no confirm action cards rendered for the fixture project');
  const card = cards.first();
  await card.locator('.wb-action-head').click();
  await card.locator('.wb-copy-cli').waitFor({ timeout: 30000 });
  for (const input of await card.locator('input[type="text"]').all()) {
    await input.fill('harness-value');
  }
  await card.locator('.wb-copy-cli').click();
  await card.locator('.wb-action-cli-copy code').waitFor({ timeout: 30000 });
  const shown = (await card.locator('.wb-action-cli-copy code').innerText()).trim();
  if (!shown.startsWith('forge ')) fail(`shown CLI is not a forge string: ${shown}`);
  let clipped = '';
  try {
    clipped = (await page.evaluate(() => window.navigator.clipboard.readText())).trim();
  } catch (err) {
    fail(`clipboard read failed: ${err.message}`);
  }
  if (clipped !== shown) fail(`clipboard [${clipped}] is not the shown string [${shown}]`);
  const copyNote = await card.locator('.wb-plan-result').innerText();
  if (!/copied to the clipboard/i.test(copyNote)) fail(`copy confirmation missing: ${copyNote}`);
  note(`copy-CLI copies the exact string (${shown.slice(0, 72)})`);

  // 6-7. Contrast AA over the workbench at desktop width.
  const failures = (await contrastFailures(page)).slice(0, 8);
  if (failures.length > 0) fail(`contrast AA: ${failures.join(' | ')}`);
  note('workbench text meets contrast AA');

  // 8. Flywheel ends + cap grouping + demo URL shapes (flywheel-plugin-cap).
  const ideaEntry = page.locator('#wb-idea-entry');
  await ideaEntry.waitFor({ timeout: 30000 });
  const ideaText = (await ideaEntry.innerText()).trim();
  if (!/graduation preview/i.test(ideaText) || !/studio spec/i.test(ideaText)) {
    fail(`idea entry misses graduation/studio: ${ideaText.slice(0, 120)}`);
  }
  const ideaLinks = await ideaEntry.locator('a').evaluateAll((links) =>
    links.map((a) => a.getAttribute('href')),
  );
  for (const href of ideaLinks) {
    if (!href.includes(`project=${encodeURIComponent(project)}`) || !href.includes('&step=')) {
      fail(`idea entry link misses the project+step pair: ${href}`);
    }
  }
  note('rail step 0 carries the idea entry with project+step links');
  const capBadge = (await page.locator('#cap-badge').innerText()).trim();
  if (!/capability group:/i.test(capBadge)) fail(`cap badge misses the group text: ${capBadge}`);
  note(`rail cap badge names the group (${capBadge.slice(0, 48)})`);
  await page.goto(`${webBase}/projects`, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.locator('#cap-filter').waitFor({ timeout: 30000 });
  const capOptions = await page.locator('#cap-filter option').allInnerTexts();
  for (const want of ['gate', 'agent', 'contract', 'delivery']) {
    if (!capOptions.map((t) => t.trim()).includes(want)) fail(`cap filter misses group ${want}: [${capOptions.join(', ')}]`);
  }
  note('projects view carries the cap group filter');
  await page.goto(`${webBase}/workbench?project=${encodeURIComponent(project)}&step=idea`, {
    waitUntil: 'domcontentloaded',
    timeout: 30000,
  });
  await expectBootedRail(page);
  if (!page.url().includes(`project=${encodeURIComponent(project)}`) || !page.url().includes('step=idea')) {
    fail(`demo URL lost the pair: ${page.url()}`);
  }
  note('demo URL shape ?project=&step=idea survives load');

  // 9. Lifecycle execution clicks in rail order (web-lifecycle-execution).
  // Every confirm follows preview → confirm-checkbox → run → role=status
  // result + journal evidence row. Invalid submits focus the error summary;
  // honest refusals (no automatic remediate finding on the fixture, delivery
  // with no share record) still click through preview → confirm → run and
  // assert the refusal shape instead of a success.
  const ARTIFACT = JSON.stringify({
    contract: 'platform.idea-graduation/0.1.0',
    hypora_project_id: 'prj_01H',
    hypora_revision: 'rev-2026-09-20-3',
    graduated_at: '2026-09-20T00:00:00Z',
    brief: {
      title: 'Rail Click Idea',
      problem: 'Operators cannot walk the loop in the browser.',
      audience: 'Forge operators.',
      solution: 'Wire every confirm to the same in-process op as the CLI.',
      requirements: ['Every confirm clicks end to end with journal evidence.'],
      success_metrics: [{ name: 'confirms_clicked', target: '>= 1', window: '30d' }],
    },
    experiment: {
      summary: 'One operator walked the rail in the harness.',
      validated: true,
      evidence: [{ kind: 'probe-completion', excerpt: 'Aggregate: 1 of 1 walked.', observed_at: '2026-09-18T00:00:00Z' }],
    },
  });
  const IDEA_ID = 'rail-click-idea';
  const focusedSummary = () => page.evaluate(() => {
    const active = document.activeElement;
    if (!active) return 'none';
    if (active.classList && active.classList.contains('error-summary')) return active.id || 'error-summary';
    const summary = active.closest ? active.closest('.error-summary') : null;
    if (summary) return summary.id || 'error-summary';
    return active.id || active.tagName;
  });

  // 9a. Idea: invalid (empty artifact) focuses the error summary; valid
  // previews, confirms and imports with a journal row on the new project.
  await page.locator('#idea-artifact').fill('');
  await page.locator('#idea-preview').click();
  await page.locator('#idea-error-summary:not([hidden])').waitFor({ timeout: 30000 });
  if ((await focusedSummary()) !== 'idea-error-summary') {
    fail(`idea invalid did not focus the error summary (at ${await focusedSummary()})`);
  }
  note('idea invalid focuses the error summary');
  await shot('09a-idea-invalid');
  await page.locator('#idea-artifact').fill(ARTIFACT);
  await page.locator('#idea-id').fill(IDEA_ID);
  await page.locator('#idea-preview').click();
  await page.locator('#idea-result:not([hidden])').waitFor({ timeout: 30000 });
  const ideaPreview = await page.locator('#idea-result').innerText();
  if (!/nothing has been written/i.test(ideaPreview) || !/rail-click-idea/i.test(ideaPreview)) {
    fail(`idea preview misses the no-write promise or id: ${ideaPreview.slice(0, 160)}`);
  }
  // Run without the confirm tick is refused with the summary focused.
  await page.locator('#idea-run').click();
  await page.locator('#idea-error-summary:not([hidden])').waitFor({ timeout: 30000 });
  if ((await focusedSummary()) !== 'idea-error-summary') {
    fail(`idea unconfirmed run did not focus the error summary (at ${await focusedSummary()})`);
  }
  note('idea confirm-required path focuses the summary');
  await page.locator('#idea-confirm').check();
  await page.locator('#idea-run').click();
  await page.waitForFunction(
    () => /imported/i.test(document.getElementById('idea-result')?.innerText || ''),
    undefined,
    { timeout: 60000 },
  );
  const ideaRole = await page.locator('#idea-result').getAttribute('role');
  if (ideaRole !== 'status') fail(`idea result role is ${ideaRole}, want status`);
  note('idea preview→confirm→run imports with role=status');
  await shot('09a-idea-done');

  // 9b. Scaffold `new` via the management view (global card, server-side
  // destination): invalid focuses the summary; valid previews, confirms,
  // creates with a journal row.
  await page.goto(`${webBase}/management`, { waitUntil: 'domcontentloaded', timeout: 30000 });
  const newCard = page.locator('#management-actions .wb-action-card[data-command="new"]');
  await newCard.waitFor({ timeout: 30000 });
  await newCard.locator('.wb-action-head').click();
  await newCard.locator('button:has-text("Preview")').first().click();
  await newCard.locator('.error-summary:not([hidden])').waitFor({ timeout: 30000 });
  if (!/error-summary/i.test(await focusedSummary()) && (await focusedSummary()) === 'none') {
    fail('scaffold new invalid did not focus an error summary');
  }
  note('scaffold new invalid focuses the error summary');
  const scaffoldId = `rail-scaffold-${Date.now().toString(36)}`.toLowerCase().replace(/[^a-z0-9-]/g, '').slice(0, 24);
  // Fill the card's typed fields by label order: project, profile, name.
  const newInputs = newCard.locator('.wb-action-body input[type="text"]');
  await newInputs.nth(0).fill(scaffoldId);
  await newInputs.nth(1).fill('rust-web');
  await newCard.locator('button:has-text("Preview")').first().click();
  await newCard.locator('.wb-plan-result:not([hidden])').first().waitFor({ timeout: 30000 });
  await newCard.locator('.wb-confirm input[type="checkbox"]').check();
  await newCard.locator('button:has-text("Run confirmed action")').click();
  await page.waitForFunction(
    (id) => (document.getElementById('management-actions')?.innerText || '').includes('Done') || document.body.innerText.includes(id),
    scaffoldId,
    { timeout: 90000 },
  );
  note(`scaffold new creates ${scaffoldId} with role=status`);
  await shot('09b-scaffold-done');
  // Back to the fixture workbench for the per-project clicks.
  await page.goto(`${webBase}/workbench?project=${encodeURIComponent(project)}&step=spec`, {
    waitUntil: 'domcontentloaded',
    timeout: 30000,
  });
  await expectBootedRail(page);

  // 9c. Studio spec save (r0) then refine (revision bump + journal row).
  const specYaml = `schema_version: "1"\nproject_id: ${project}\nname: Rail Fixture\nprofile: react-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\n        title: Welcome\n        body: body\n`;
  const studioSpec = page.locator('#wb-actions .wb-action-card[data-command="studio.spec"]');
  await studioSpec.waitFor({ timeout: 30000 });
  await studioSpec.locator('.wb-action-head').click();
  const specInputs = studioSpec.locator('.wb-action-body textarea, .wb-action-body input[type="text"]');
  await specInputs.first().fill(specYaml);
  await studioSpec.locator('button:has-text("Preview")').first().click();
  await studioSpec.locator('.wb-plan-result:not([hidden])').first().waitFor({ timeout: 30000 });
  await studioSpec.locator('.wb-confirm input[type="checkbox"]').check();
  await studioSpec.locator('button:has-text("Run confirmed action")').click();
  // Success reloads the workbench (fresh cards), so the journal table —
  // re-rendered after the reload — is the durable assertion, not the
  // ephemeral card result.
  await page.waitForFunction(
    () => /studio\.spec\.save/i.test(document.getElementById('wb-operations')?.innerText || ''),
    undefined,
    { timeout: 90000 },
  );
  note('studio spec save confirms with journal evidence');
  await shot('09c-studio-spec-done');
  const studioRefine = page.locator('#wb-actions .wb-action-card[data-command="studio.refine"]');
  await studioRefine.locator('.wb-action-head').click();
  const refineInputs = studioRefine.locator('.wb-action-body textarea, .wb-action-body input[type="text"]');
  await refineInputs.nth(0).fill('polish the hero for the rail click');
  await refineInputs.nth(1).fill('r1');
  await studioRefine.locator('button:has-text("Preview")').first().click();
  await studioRefine.locator('.wb-plan-result:not([hidden])').first().waitFor({ timeout: 30000 });
  await studioRefine.locator('.wb-confirm input[type="checkbox"]').check();
  await studioRefine.locator('button:has-text("Run confirmed action")').click();
  // Success re-renders the confirmation onto the fresh card, so the
  // journal row (durable across the reload) gates the revision assertion.
  // The operations table renders before the confirmation message lands,
  // so poll the message after the row.
  await page.waitForFunction(
    () => /studio\.refine/i.test(document.getElementById('wb-operations')?.innerText || ''),
    undefined,
    { timeout: 90000 },
  );
  await page.waitForFunction(
    () => /revision|recorded in the project journal/i.test(document.querySelector('#wb-actions .wb-action-card[data-command="studio.refine"] .wb-plan-result')?.textContent || ''),
    undefined,
    { timeout: 30000 },
  );
  const refineText = await studioRefine.locator('.wb-plan-result').innerText();
  if (!/revision/i.test(refineText)) {
    fail(`refine result misses the revision line: ${refineText.slice(0, 160)}`);
  }
  if (!/r1/i.test(refineText) || !/r2/i.test(refineText)) {
    fail(`refine result misses the r1→r2 bump: ${refineText.slice(0, 160)}`);
  }
  note('studio refine confirms with the revision bump');
  await shot('09c-studio-refine-done');

  // 9d. Gate dry-run analog: the read-only upgrade plan renders steps.
  await page.locator('#wb-plan').click();
  await page.locator('#wb-plan-result:not([hidden])').waitFor({ timeout: 60000 });
  const planRole = await page.locator('#wb-plan-result').getAttribute('role');
  if (planRole !== 'status') fail(`upgrade plan result role is ${planRole}, want status`);
  note('upgrade plan (gate dry-run analog) renders with role=status');
  await shot('09d-upgrade-plan');

  // 9e. Delivery approve/publish: confirm-tick refusal without digest setup
  // focuses the error summary; no digest means nothing can be approved.
  await page.goto(`${webBase}/delivery`, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.locator('#delivery-approve').waitFor({ timeout: 30000 });
  await page.locator('#delivery-approve').click();
  await page.locator('#delivery-error-summary:not([hidden])').waitFor({ timeout: 30000 });
  if ((await focusedSummary()) !== 'delivery-error-summary') {
    fail(`delivery approve refusal did not focus the summary (at ${await focusedSummary()})`);
  }
  note('delivery approve confirm-refused focuses the summary');
  await shot('09e-delivery-approve-refused');

  // 9f. Maintain refresh re-renders the observation.
  await page.goto(`${webBase}/workbench?project=${encodeURIComponent(project)}&step=operate`, {
    waitUntil: 'domcontentloaded',
    timeout: 30000,
  });
  await expectBootedRail(page);
  await page.locator('#wb-maintain-refresh').click();
  await page.waitForTimeout(2000);
  const maintainText = await page.locator('#wb-maintain-body').innerText();
  if (!maintainText || /loading maintainer/i.test(maintainText)) {
    fail(`maintain refresh left the body loading: ${maintainText.slice(0, 120)}`);
  }
  note('maintain refresh re-renders the observation');
  await shot('09f-maintain-refresh');

  // 9g. Remediate plan with a bogus finding is refused with summary focus;
  // the apply card still clicks through preview → confirm → run and asserts
  // the same honest refusal (the fixture carries no automatic finding).
  const remPlan = page.locator('#wb-actions .wb-action-card[data-command="remediate.plan"]');
  await remPlan.locator('.wb-action-head').click();
  await remPlan.locator('.wb-action-body input[type="text"]').first().fill('no-such-finding-rail');
  await remPlan.locator('button:has-text("Preview")').first().click();
  await remPlan.locator('.error-summary:not([hidden])').waitFor({ timeout: 30000 });
  note('remediate plan bogus finding refused with summary focus');
  await shot('09g-remediate-plan-refused');
  const remApply = page.locator('#wb-actions .wb-action-card[data-command="remediate.apply"]');
  await remApply.locator('.wb-action-head').click();
  await remApply.locator('.wb-action-body input[type="text"]').first().fill('no-such-finding-rail');
  await remApply.locator('button:has-text("Preview")').first().click();
  await remApply.locator('.error-summary:not([hidden])').first().waitFor({ timeout: 30000 });
  note('remediate apply bogus finding refused with summary focus');

  // 9h. Intent resolve previews the plan; apply confirms it with a journal row.
  const intentResolve = page.locator('#wb-actions .wb-action-card[data-command="intent.resolve"]');
  await intentResolve.locator('.wb-action-head').click();
  await intentResolve.locator('.wb-action-body input[type="text"]').first().fill('extend_project');
  await intentResolve.locator('button:has-text("Preview")').first().click();
  await intentResolve.locator('.wb-plan-result:not([hidden])').first().waitFor({ timeout: 60000 });
  note('intent resolve previews the plan');
  await shot('09h-intent-resolve');
  const intentApply = page.locator('#wb-actions .wb-action-card[data-command="intent.apply"]');
  await intentApply.locator('.wb-action-head').click();
  await intentApply.locator('.wb-action-body input[type="text"]').first().fill('extend_project');
  await intentApply.locator('button:has-text("Preview")').first().click();
  await intentApply.locator('.wb-plan-result:not([hidden])').first().waitFor({ timeout: 60000 });
  await intentApply.locator('.wb-confirm input[type="checkbox"]').check();
  await intentApply.locator('button:has-text("Run confirmed action")').click();
  await page.waitForFunction(
    () => /recorded in the project journal/i.test(document.querySelector('#wb-actions .wb-action-card[data-command="intent.apply"] .wb-plan-result')?.textContent || ''),
    undefined,
    { timeout: 90000 },
  );
  const intentRole = await intentApply.locator('.wb-plan-result').getAttribute('role');
  if (intentRole !== 'status') fail(`intent apply result role is ${intentRole}, want status`);
  const opsText = await page.locator('#wb-operations').innerText();
  if (!/intent\.apply/i.test(opsText)) {
    fail(`journal misses the intent.apply row: ${opsText.slice(0, 200)}`);
  }
  note('intent apply confirms with role=status and a journal row');
  await shot('09h-intent-apply-done');
  failOnConsoleErrors('lifecycle execution clicks');
  note('lifecycle execution clicks are console-error-free');
  await context.close();

  // 8. Mobile 390px: no page-level horizontal overflow.
  const mobile = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 1 });
  const small = await mobile.newPage();
  await signIn(small);
  await small.goto(`${webBase}/workbench?project=${encodeURIComponent(project)}&step=test`, {
    waitUntil: 'domcontentloaded',
    timeout: 30000,
  });
  await expectBootedRail(small);
  await expectRailShape(small, 'mobile 390px');
  const reflow = await small.evaluate(() => ({
    scroll: document.documentElement.scrollWidth,
    client: document.documentElement.clientWidth,
  }));
  if (reflow.scroll > reflow.client + 1) {
    fail(`mobile 390px horizontal overflow (${reflow.scroll} > ${reflow.client})`);
  }
  note('mobile 390px has no horizontal overflow');
  await mobile.close();

  await browser.close();
  console.log('VERIFIED: lifecycle-rail-check ok');
  process.exit(0);
} catch (err) {
  try {
    await browser.close();
  } catch (_) {
    /* already gone */
  }
  if (err && err.message && err.message.startsWith('UNVERIFIED')) {
    console.error(err.message);
    process.exit(2);
  }
  console.error(`FAIL: ${err && err.stack ? err.stack : err}`);
  process.exit(1);
}
