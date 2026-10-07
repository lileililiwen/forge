#!/usr/bin/env node
// Browser oracle for the staged project-delivery workbench flow
// (`forge-web-project-delivery`).
//
// Usage:
//   node delivery-workbench-check.mjs <web-base> <email> <password> \
//     <project-id> <deployment-url> <secret-ref> <fixture-root>
//
// The harness signs in through the shipped login page, opens the fixture
// project, checks the read-only delivery card, then previews/confirms the
// four staged verbs in order. It also checks keyboard entry/focus, a keyboard
// activation, measured text contrast on the delivery card, and that the
// absolute fixture root is never rendered.
//
// Exit codes: 0 verified, 1 a check failed, 2 Playwright or a browser engine
// is unavailable (`UNVERIFIED`). The Rust caller reports exit 2 as
// UNVERIFIED, never a pass.

const [
  webBase,
  email,
  password,
  projectId,
  deploymentUrl,
  secretRef,
  fixtureRoot,
] = process.argv.slice(2);

if (!webBase || !email || !password || !projectId || !deploymentUrl || !secretRef || !fixtureRoot) {
  console.error('usage: delivery-workbench-check.mjs <web-base> <email> <password> <project-id> <deployment-url> <secret-ref> <fixture-root>');
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
const note = (message) => console.log(`delivery-workbench: ${message}`);

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

  await page.locator('#workbench-project').selectOption(projectId);
  await page.getByRole('button', { name: 'Open project' }).click();
  const delivery = page.locator('#wb-delivery');
  await delivery.getByText('Phase: draft', { exact: false }).waitFor({ timeout: 30000 });
  const draftNext = await delivery.innerText();
  if (!draftNext.includes('Next: Delivery preflight')) {
    fail(`draft delivery card did not name preflight: ${draftNext}`);
  }

  const card = (name) =>
    page.locator('.wb-action-card', { hasText: `forge delivery ${name}` }).first();
  const previewAndRun = async (name, fill) => {
    const action = card(name);
    await action.getByRole('heading').waitFor({ timeout: 30000 });
    if (fill) await fill(action);
    await action.getByRole('button', { name: new RegExp(`^preview ${name}$`, 'i') }).click();
    await action.getByRole('checkbox').check();
    await action.getByRole('button', { name: 'Run confirmed action' }).click();
    note(`${name} confirmed`);
  };

  await previewAndRun('preflight');
  await delivery.getByText('Phase: awaiting-stage-confirmation', { exact: false }).waitFor({ timeout: 30000 });
  const stagePhaseText = await delivery.innerText();
  if (/confirm_operation_id:/.test(stagePhaseText)) {
    fail('delivery card rendered the staged operation id as text');
  }
  // Staged confirmations arrive pre-filled from delivery status: the stage
  // input must already hold the op id from `next.confirm_operation_id`.
  const stageAction = card('stage');
  await stageAction.getByRole('heading').waitFor({ timeout: 30000 });
  const expectedOp = await page.evaluate(async (pid) => {
    const base = (window.FORGE_API_BASE || '').replace(/\/$/, '');
    const res = await fetch(`${base}/v1/admin/projects/${encodeURIComponent(pid)}/delivery/status`, {
      headers: { Accept: 'application/json' },
      credentials: 'include',
    });
    if (!res.ok) throw new Error(`delivery status ${res.status}`);
    const data = await res.json();
    return data?.next?.confirm_operation_id ?? null;
  }, projectId).catch((err) => fail(`could not read staged operation id from status: ${err.message}`));
  if (expectedOp === null || expectedOp === undefined || String(expectedOp) === '') {
    fail('delivery status did not carry a staged confirm_operation_id');
  }
  const stageValue = await stageAction.getByLabel(/confirm_operation_id \(required\)/i).inputValue();
  if (stageValue !== String(expectedOp)) {
    fail(`stage confirmation was not pre-filled from status: input=${stageValue} expected=${expectedOp}`);
  }
  note(`stage confirmation pre-filled with operation ${stageValue}`);

  await previewAndRun('stage');
  await delivery.getByText('Phase: awaiting-production-approval', { exact: false }).waitFor({ timeout: 30000 });
  const promotePhaseText = await delivery.innerText();
  if (/confirm_revision:/.test(promotePhaseText)) {
    fail('delivery card rendered the promotion revision as text');
  }
  // The promote input must already hold the revision from
  // `next.confirm_revision`.
  const promoteAction = card('promote');
  await promoteAction.getByRole('heading').waitFor({ timeout: 30000 });
  const expectedRevision = await page.evaluate(async (pid) => {
    const base = (window.FORGE_API_BASE || '').replace(/\/$/, '');
    const res = await fetch(`${base}/v1/admin/projects/${encodeURIComponent(pid)}/delivery/status`, {
      headers: { Accept: 'application/json' },
      credentials: 'include',
    });
    if (!res.ok) throw new Error(`delivery status ${res.status}`);
    const data = await res.json();
    return data?.next?.confirm_revision ?? null;
  }, projectId).catch((err) => fail(`could not read staged revision from status: ${err.message}`));
  if (!expectedRevision) {
    fail('delivery status did not carry a staged confirm_revision');
  }
  const promoteValue = await promoteAction.getByLabel(/confirm_revision \(required\)/i).inputValue();
  if (promoteValue !== String(expectedRevision)) {
    fail('promote confirmation was not pre-filled from status');
  }
  note('promote confirmation pre-filled from status');

  await previewAndRun('promote');
  await delivery.getByText('Phase: healthy', { exact: false }).waitFor({ timeout: 30000 });

  // The final mutation is keyboard-driven: fill the two typed fields, focus
  // the preview control, activate it with Enter, tick confirmation with
  // Space, then activate the confirmed run with Enter.
  const hermora = card('hermora-retry');
  await hermora.getByRole('heading').waitFor({ timeout: 30000 });
  await hermora.getByLabel(/deployment_url \(required\)/i).fill(deploymentUrl);
  await hermora.getByLabel(/secret_ref \(required\)/i).fill(secretRef);
  const hermoraPreview = hermora.getByRole('button', { name: /^preview hermora-retry$/i });
  await hermoraPreview.focus();
  await page.keyboard.press('Enter');
  const hermoraPreviewText = await hermora.locator('.wb-plan-result').innerText();
  if (hermoraPreviewText.includes('[object Object]')) {
    fail(`Hermora preview rendered an unexpanded object: ${hermoraPreviewText}`);
  }
  const hermoraConfirm = hermora.getByRole('checkbox');
  await hermoraConfirm.focus();
  await page.keyboard.press('Space');
  if (!(await hermoraConfirm.isChecked())) {
    fail('Hermora confirmation checkbox did not stay checked after Space');
  }
  const hermoraRun = hermora.getByRole('button', { name: 'Run confirmed action' });
  await hermoraRun.focus();
  await hermoraRun.click();
  note('hermora-retry confirmed');
  try {
    await delivery.getByText('Phase: hermora-connected', { exact: false }).waitFor({ timeout: 30000 });
  } catch (err) {
    const deliveryText = await delivery.innerText().catch(() => '<delivery unreadable>');
    const actionText = await hermora.locator('.wb-plan-result').innerText().catch(() => '<result unreadable>');
    fail(`final phase wait timed out; delivery=${deliveryText}; hermora result=${actionText}; ${err.message}`);
  }

  const workbenchText = await page.locator('#workbench-body').innerText();
  if (workbenchText.includes(fixtureRoot)) {
    fail('workbench rendered the absolute fixture root');
  }
  if (workbenchText.includes(secretRef)) {
    fail('workbench rendered the Hermora secret reference');
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
    const title = document.querySelector('#wb-delivery-title');
    const note = document.querySelector('#wb-delivery p');
    return [title, note].filter(Boolean).map((element) => ({
      text: (element.textContent || '').trim().slice(0, 48),
      foreground: parse(getComputedStyle(element).color),
      background: background(element),
    }));
  });
  for (const sample of samples) {
    if (!sample.foreground) fail(`unmeasurable delivery text color: ${sample.text}`);
    const ratio = contrast(sample.foreground, sample.background);
    if (ratio + 0.02 < 4.5) {
      fail(`delivery text contrast ${ratio.toFixed(2)}:1 below 4.5:1 for ${sample.text}`);
    }
  }

  console.log('VERIFIED: staged delivery completed in Chromium through the workbench');
} catch (err) {
  fail(err.message);
} finally {
  await browser.close();
}
