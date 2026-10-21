import { chromium } from '@playwright/test';
import fs from 'node:fs';

const BASE = 'http://127.0.0.1:3219';
const OUT = '/root/.hermes/profiles/vela/cache/scratch/vela-design-stage/metrics.json';

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1284, height: 856 } });
await page.goto(BASE, { waitUntil: 'domcontentloaded' });
await page.waitForTimeout(2500);
const inputs = page.locator('input');
if (await inputs.count() >= 2) {
  await inputs.nth(0).fill('designer');
  await inputs.nth(1).fill('DesignStage2026!');
  await page.getByRole('button', { name: /sign in/i }).first().click();
}
await page.waitForTimeout(7000);
await page.getByRole('button', { name: 'Library', exact: true }).first().click();
await page.waitForTimeout(2500);

const metrics = await page.evaluate(() => {
  const out = {};
  const seen = new Map();
  // 1. Effective background colors of key surfaces (are they opaque?)
  const surfaces = ['.app-shell', '.sidebar', '.topbar', '.main-content', '.player-bar', '.page', '.panel', '.track-row', '.catalog-grid', '.album-card', '.playlist-card', '.listening-card'];
  out.surfaces = {};
  for (const sel of surfaces) {
    const el = document.querySelector(sel);
    if (!el) continue;
    const cs = getComputedStyle(el);
    out.surfaces[sel] = {
      background: cs.backgroundColor,
      backdropFilter: cs.backdropFilter,
      position: cs.position,
      zIndex: cs.zIndex,
    };
  }
  // 2. Any element using backdrop-filter on the page
  out.backdropUsers = [...document.querySelectorAll('*')].filter((el) => {
    const bf = getComputedStyle(el).backdropFilter;
    return bf && bf !== 'none';
  }).map((el) => ({
    tag: el.tagName.toLowerCase(),
    cls: (el.className || '').toString().slice(0, 60),
    filter: getComputedStyle(el).backdropFilter,
  }));
  // 3. Interactive target sizes below 44px
  out.smallTargets = [...document.querySelectorAll('button, a, input, select, [role=button]')]
    .map((el) => {
      const r = el.getBoundingClientRect();
      return { label: (el.getAttribute('aria-label') || el.textContent || '').trim().slice(0, 40), cls: (el.className || '').toString().slice(0, 50), w: Math.round(r.width), h: Math.round(r.height) };
    })
    .filter((x) => x.w > 0 && x.h > 0 && (x.h < 44 || x.w < 44));
  // 4. Type scale in use
  const typeSizes = new Map();
  for (const el of document.querySelectorAll('body *')) {
    const cs = getComputedStyle(el);
    const key = cs.fontSize + '/' + cs.fontWeight;
    typeSizes.set(key, (typeSizes.get(key) || 0) + 1);
  }
  out.typeScale = [...typeSizes.entries()].sort((a, b) => b[1] - a[1]).slice(0, 18);
  // 5. Contrast of real text on real backgrounds (approximate)
  function lum(rgb) {
    const [r, g, b] = rgb.match(/\d+/g).map(Number).map((v) => {
      const s = v / 255;
      return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  }
  function parse(c) {
    const m = c.match(/[\d.]+/g);
    if (!m) return null;
    return { r: +m[0], g: +m[1], b: +m[2], a: m[3] === undefined ? 1 : +m[3] };
  }
  function bgOf(el) {
    let n = el;
    while (n && n !== document.documentElement) {
      const c = parse(getComputedStyle(n).backgroundColor);
      if (c && c.a > 0.9) return c;
      n = n.parentElement;
    }
    return parse(getComputedStyle(document.body).backgroundColor);
  }
  out.contrast = [];
  const isVisible = (el) => {
    const r = el.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) return false;
    const cs = getComputedStyle(el);
    return cs.visibility !== 'hidden' && cs.display !== 'none' && cs.opacity !== '0';
  };
  for (const el of document.querySelectorAll('p, small, strong, h1, h2, h3, span, button, a, td, div.eyebrow, label')) {
    const txt = (el.textContent || '').trim();
    if (!txt || el.children.length > 0) continue;
    if (!isVisible(el)) continue;
    const cs = getComputedStyle(el);
    const fg = parse(cs.color);
    const bg = bgOf(el);
    if (!fg || !bg) continue;
    const l1 = lum(`rgb(${fg.r}, ${fg.g}, ${fg.b})`);
    const l2 = lum(`rgb(${bg.r}, ${bg.g}, ${bg.b})`);
    const ratio = (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
    out.contrast.push({
      text: txt.slice(0, 32),
      size: cs.fontSize,
      weight: cs.fontWeight,
      color: cs.color,
      bg: `rgb(${bg.r}, ${bg.g}, ${bg.b})`,
      ratio: Math.round(ratio * 100) / 100,
    });
  }
  out.contrast.sort((a, b) => a.ratio - b.ratio);
  // 6. Spacing rhythm: distinct gap/padding values
  const gaps = new Map();
  for (const el of document.querySelectorAll('body *')) {
    const cs = getComputedStyle(el);
    for (const prop of ['gap', 'rowGap', 'columnGap', 'padding', 'marginTop', 'marginBottom']) {
      const v = cs[prop];
      if (v && v !== '0px' && v !== 'normal' && v !== 'auto') gaps.set(v, (gaps.get(v) || 0) + 1);
    }
  }
  out.spacing = [...gaps.entries()].sort((a, b) => b[1] - a[1]).slice(0, 30);
  // 7. Radii
  const radii = new Map();
  for (const el of document.querySelectorAll('body *')) {
    const r = getComputedStyle(el).borderRadius;
    if (r && r !== '0px') radii.set(r, (radii.get(r) || 0) + 1);
  }
  out.radii = [...radii.entries()].sort((a, b) => b[1] - a[1]).slice(0, 15);
  // 8. Transitions/animations
  const trans = new Map();
  let animated = 0;
  for (const el of document.querySelectorAll('body *')) {
    const cs = getComputedStyle(el);
    if (cs.transitionDuration && cs.transitionDuration !== '0s') trans.set(cs.transitionProperty + ' ' + cs.transitionDuration, (trans.get(cs.transitionProperty + ' ' + cs.transitionDuration) || 0) + 1);
    if (cs.animationName && cs.animationName !== 'none') animated++;
  }
  out.transitions = [...trans.entries()].sort((a, b) => b[1] - a[1]).slice(0, 15);
  out.animatedElements = animated;
  return out;
});

fs.writeFileSync(OUT, JSON.stringify(metrics, null, 2));
console.log('WROTE', OUT);
console.log('backdrop users:', JSON.stringify(metrics.backdropUsers));
console.log('small targets:', metrics.smallTargets.length);
console.log('worst contrast:', JSON.stringify(metrics.contrast.slice(0, 12)));
console.log('transitions:', JSON.stringify(metrics.transitions));
await browser.close();
