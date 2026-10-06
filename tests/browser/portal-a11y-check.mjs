#!/usr/bin/env node
// Browser oracle for the portal responsive/accessibility baseline
// (`portal-accessible-responsive-ui`).
//
// Usage: node portal-a11y-check.mjs <dir>
//
// <dir> holds `manifest.json` ([{ "name", "file" }, …]) and the rendered
// HTML files produced by `tests/portal_browser_a11y.rs`. Each page is loaded
// from disk — the portal stylesheet is inline, so no server is required — and
// checked at a narrow and a wide viewport in both colour schemes for:
//
//   * reflow  — no page-level horizontal overflow at 320 CSS px
//   * semantics — one each of main/header/footer/h1, a named nav, a skip
//     link that targets the single main, ordered headings, labelled
//     controls, and captioned/scoped tables inside named keyboard regions
//   * keyboard — the first focusable element is the skip link and every
//     early tab stop has a visible, unobscured focus indicator
//   * contrast — measured text/background pairs meet WCAG 2.2 AA
//     (4.5:1 normal text, 3:1 large text and interactive boundaries)
//
// Exit codes: 0 all checks pass, 1 a check failed, 2 the browser is
// unavailable. The Rust caller reports exit 2 as UNVERIFIED, never a pass.

import { readFileSync, mkdirSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import path from 'node:path';

let chromium;
try {
  ({ chromium } = await import('playwright'));
} catch (err) {
  console.error(`UNVERIFIED: playwright is unavailable: ${err.message}`);
  process.exit(2);
}

const dir = process.argv[2];
if (!dir) {
  console.error('usage: portal-a11y-check.mjs <dir>');
  process.exit(2);
}

let manifest;
try {
  manifest = JSON.parse(readFileSync(path.join(dir, 'manifest.json'), 'utf8'));
} catch (err) {
  console.error(`UNVERIFIED: cannot read manifest: ${err.message}`);
  process.exit(2);
}

let browser;
try {
  browser = await chromium.launch();
} catch (err) {
  console.error(`UNVERIFIED: cannot launch Chromium: ${err.message}`);
  process.exit(2);
}

const issues = [];
const record = (ok, message) => {
  if (!ok) issues.push(message);
};

async function semantics(page) {
  return page.evaluate(() => {
    const all = (selector) => Array.from(document.querySelectorAll(selector));
    const tables = all('table').map((table) => ({
      hasCaption: !!table.querySelector('caption'),
      hasThead: !!table.querySelector('thead'),
      scopedHeaders: Array.from(table.querySelectorAll('th')).every((th) => {
        const scope = th.getAttribute('scope');
        return scope === 'col' || scope === 'row';
      }),
      wrapped: !!table.closest('[role="region"][aria-label]'),
      regionTabindex: table.closest('[role="region"]')?.getAttribute('tabindex') === '0',
    }));
    const inputs = all('input:not([type="hidden"]), select, textarea').map((el) => ({
      name: el.getAttribute('name') || el.getAttribute('type') || el.tagName.toLowerCase(),
      labelled: !!(el.labels && el.labels.length > 0),
    }));
    const levels = all('h1, h2, h3, h4, h5, h6').map((h) => Number(h.tagName[1]));
    let ordered = levels.length > 0 && levels[0] === 1;
    for (let i = 1; i < levels.length; i += 1) {
      if (levels[i] > levels[i - 1] + 1) ordered = false;
    }
    const skip = all('.skip-link')[0];
    const navs = all('nav');
    return {
      lang: document.documentElement.getAttribute('lang'),
      main: all('main').length,
      header: all('header').length,
      footer: all('footer').length,
      h1: all('h1').length,
      navNamed: navs.length > 0 && navs.every((n) => n.getAttribute('aria-label') || n.getAttribute('aria-labelledby')),
      skip: skip
        ? { href: skip.getAttribute('href'), target: !!document.querySelector(skip.getAttribute('href')) }
        : null,
      tables,
      inputs,
      levels,
      ordered,
    };
  });
}

async function contrastIssues(page) {
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
      const l1 = luminance(a);
      const l2 = luminance(b);
      const hi = Math.max(l1, l2);
      const lo = Math.min(l1, l2);
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
    const selector = 'p, li, td, th, caption, h1, h2, h3, h4, a, span, code, small, label, button, legend, strong';
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
        findings.push({
          text: text.slice(0, 48),
          tag: el.tagName.toLowerCase(),
          got: Math.round(got * 100) / 100,
          need,
        });
      }
    }
    return findings;
  });
}

async function focusIssues(page) {
  const findings = [];
  const first = await page.evaluate(() => {
    const el = document.querySelector(
      'a[href], button, input:not([type="hidden"]), select, textarea, [tabindex]',
    );
    return el ? { cls: String(el.className), href: el.getAttribute('href') } : null;
  });
  if (!first || !first.cls.includes('skip-link')) {
    findings.push(`first focusable is not the skip link: ${JSON.stringify(first)}`);
  }
  const count = await page.$$eval(
    'a[href], button, input:not([type="hidden"]), select, textarea, [tabindex="0"]',
    (els) => els.length,
  );
  const steps = Math.min(count, 14);
  for (let i = 0; i < steps; i += 1) {
    await page.keyboard.press('Tab');
    const info = await page.evaluate(() => {
      const el = document.activeElement;
      if (!el || el === document.body) return null;
      el.scrollIntoView({ block: 'nearest', inline: 'nearest' });
      const style = getComputedStyle(el);
      const rect = el.getBoundingClientRect();
      return {
        tag: el.tagName.toLowerCase(),
        label: (el.textContent || el.getAttribute('name') || '').trim().slice(0, 32),
        outlineWidth: parseFloat(style.outlineWidth) || 0,
        outlineStyle: style.outlineStyle,
        boxShadow: style.boxShadow,
        inViewport:
          rect.bottom > 0 &&
          rect.top < window.innerHeight &&
          rect.right > 0 &&
          rect.left < window.innerWidth,
      };
    });
    if (!info) continue;
    const visible =
      (info.outlineWidth >= 2 && info.outlineStyle !== 'none') ||
      (info.boxShadow && info.boxShadow !== 'none');
    if (!visible) {
      findings.push(`tab stop ${i + 1} (<${info.tag}> ${info.label}) has no visible focus indicator`);
    }
    if (!info.inViewport) {
      findings.push(`tab stop ${i + 1} (<${info.tag}> ${info.label}) is scrolled out of view`);
    }
  }
  return findings;
}

async function axIssues(page, sem) {
  const snapshot = await page.locator('body').ariaSnapshot();
  const has = (role) => new RegExp(`(^|\\n)\\s*- ${role}(?=[:\\s]|$)`).test(snapshot);
  const findings = [];
  if (!has('main')) findings.push('accessibility tree has no main landmark');
  if (!has('banner')) findings.push('accessibility tree has no banner landmark');
  if (!has('contentinfo')) findings.push('accessibility tree has no contentinfo landmark');
  if (!has('navigation')) findings.push('accessibility tree has no navigation landmark');
  if (!/heading /.test(snapshot)) findings.push('accessibility tree has no heading');
  if (sem.tables.length > 0 && !has('table')) {
    findings.push('accessibility tree exposes no table role');
  }
  if (sem.inputs.length > 0 && !/(textbox|combobox|searchbox)/.test(snapshot)) {
    findings.push('accessibility tree exposes no text or combobox control');
  }
  return findings;
}

const viewports = [
  { name: '320', width: 320, height: 640 },
  { name: '375', width: 375, height: 667 },
  { name: '640-zoom200', width: 640, height: 900 },
  { name: '768', width: 768, height: 1024 },
  { name: '1280', width: 1280, height: 800 },
];

let screenshots = 0;

for (const entry of manifest) {
  const url = pathToFileURL(path.join(dir, entry.file)).href;
  for (const scheme of ['light', 'dark']) {
    for (const viewport of viewports) {
      const context = await browser.newContext({
        colorScheme: scheme,
        viewport: { width: viewport.width, height: viewport.height },
        deviceScaleFactor: 1,
      });
      const page = await context.newPage();
      const tag = `${entry.name} [${scheme} ${viewport.name}]`;
      try {
        await page.goto(url, { waitUntil: 'load' });

        const reflow = await page.evaluate(() => ({
          scroll: document.documentElement.scrollWidth,
          client: document.documentElement.clientWidth,
        }));
        record(
          reflow.scroll <= reflow.client + 1,
          `${tag}: horizontal overflow (${reflow.scroll} > ${reflow.client})`,
        );

        const sem = await semantics(page);
        record(sem.lang === 'en', `${tag}: html lang is ${sem.lang}`);
        record(sem.main === 1, `${tag}: expected exactly one main, found ${sem.main}`);
        record(sem.header === 1, `${tag}: expected exactly one header, found ${sem.header}`);
        record(sem.footer === 1, `${tag}: expected exactly one footer, found ${sem.footer}`);
        record(sem.h1 === 1, `${tag}: expected exactly one h1, found ${sem.h1}`);
        record(sem.navNamed, `${tag}: navigation landmark is not named`);
        record(!!(sem.skip && sem.skip.target), `${tag}: skip link does not target the main landmark`);
        record(sem.ordered, `${tag}: heading order skips a level (${sem.levels.join(',')})`);
        for (const table of sem.tables) {
          record(
            table.hasCaption && table.hasThead && table.scopedHeaders && table.wrapped && table.regionTabindex,
            `${tag}: table is missing a caption, thead, scoped headers, or its named keyboard region`,
          );
        }
        for (const control of sem.inputs) {
          record(control.labelled, `${tag}: control ${control.name} has no associated label`);
        }

        if (viewport.width === 320) {
          for (const finding of (await contrastIssues(page)).slice(0, 8)) {
            record(
              false,
              `${tag}: contrast ${finding.got}:1 < ${finding.need}:1 for <${finding.tag}> "${finding.text}"`,
            );
          }
        }
        if (viewport.width === 1280) {
          for (const finding of await focusIssues(page)) {
            record(false, `${tag}: ${finding}`);
          }
          for (const finding of await axIssues(page, sem)) {
            record(false, `${tag}: ${finding}`);
          }
        }
        if (viewport.width === 1280 && scheme === 'light') {
          const shots = path.join(dir, 'screenshots');
          mkdirSync(shots, { recursive: true });
          await page.screenshot({ path: path.join(shots, `${entry.name}.png`), fullPage: true });
          screenshots += 1;
        }
      } catch (err) {
        record(false, `${tag}: check threw: ${err.message}`);
      } finally {
        await context.close();
      }
    }
  }
}

await browser.close();

if (issues.length > 0) {
  console.error(`portal-a11y-check: ${issues.length} issue(s)`);
  for (const issue of issues) console.error(`  - ${issue}`);
  process.exit(1);
}
console.log(
  `portal-a11y-check: ok (${manifest.length} pages, light+dark, ${viewports.length} viewports, ${screenshots} screenshots)`,
);
process.exit(0);
