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
//
// Exit codes: 0 verified, 1 a check failed, 2 Playwright or a browser engine
// is unavailable (`UNVERIFIED`). The Rust caller reports exit 2 as
// UNVERIFIED, never a pass.

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
