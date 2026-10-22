/**
 * DR01 baseline probe (team lead, read-only): capture the token values and
 * reference screenshots for a given serving state of the stylesheet.
 *
 * The script records *what it actually served*, not what it hoped to serve:
 * the SHA-256 of `dist/style.css` and whether any DR01 role resolves at all.
 * The first version of this probe hard-coded the commit and was run after the
 * worker had already edited `apps/web/style.css`, so its artifact claimed to
 * be a pre-edit HEAD capture while containing every new role. That artifact
 * was discarded; this one cannot repeat the mistake, because a capture that
 * sees a DR01 role while claiming the pre-edit baseline reports itself as
 * invalid.
 *
 * Usage (the caller swaps dist/style.css first):
 *   node dr01-baseline.mjs <label>
 */
import { chromium } from '@playwright/test';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setupWithTracks, settle, navigate, startServer } from './dr04-density.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const label = process.argv[2] || 'capture';
const output = resolve(root, 'target/checks/dr01-tokens');
const shots = join(output, label);
await mkdir(shots, { recursive: true });

const ROLES = ['--vela-glass-bg','--vela-glass-blur','--vela-glass-blur-mobile','--vela-glass-blur-topbar',
  '--vela-glass-border','--vela-glass-highlight','--vela-glass-shadow','--vela-glass-text',
  '--vela-motion-instant','--vela-motion-quick','--vela-motion-overlay','--vela-motion-exit',
  '--vela-motion-panel','--vela-motion-skeleton','--vela-motion-reorder','--vela-motion-artwork',
  '--vela-ease-enter','--vela-ease-state'];

const READ = (roles) => {
  const s = getComputedStyle(document.documentElement);
  const out = {};
  for (const role of roles) out[role] = s.getPropertyValue(role).trim();
  return {
    theme: document.documentElement.getAttribute('data-theme'),
    tokens: out,
    geometry: {
      bodyFontSize: getComputedStyle(document.body).fontSize,
      sidebarWidth: s.getPropertyValue('--vela-sidebar-width').trim(),
      topbarHeight: s.getPropertyValue('--vela-topbar-height').trim(),
      playerHeight: s.getPropertyValue('--vela-player-height').trim(),
      mobilePlayerHeight: s.getPropertyValue('--vela-mobile-player-height').trim(),
      mobileNavHeight: s.getPropertyValue('--vela-mobile-nav-height').trim(),
      playerBarHeight: (() => { const p = document.querySelector('.player-bar'); return p ? +p.getBoundingClientRect().height.toFixed(2) : null; })(),
      overflow: document.documentElement.scrollWidth - innerWidth,
    },
  };
};

const servedCss = await readFile(resolve(root, 'dist/style.css'));
const servedSha = createHash('sha256').update(servedCss).digest('hex');

const runtime = await startServer(output);
const browser = await chromium.launch();
const record = {
  label, engine: 'chromium', servedStyleSha256: servedSha,
  servedCssBytes: servedCss.length, frames: [],
};
try {
  const context = await browser.newContext({ viewport: { width: 1284, height: 856 } });
  const page = await context.newPage();
  await setupWithTracks(page, runtime, 3);
  await page.goto(runtime.origin);
  await settle(page);
  await navigate(page, 'Library');
  await page.locator('.track-row').first().waitFor({ state: 'visible' });
  await page.evaluate(() => document.fonts.ready);

  for (const [width, height, name] of [[1284, 856, 'library-1284x856'], [390, 844, 'library-390x844']]) {
    await page.setViewportSize({ width, height });
    await page.waitForTimeout(300);
    for (const theme of ['dark', 'light']) {
      const current = await page.evaluate(() => document.documentElement.getAttribute('data-theme'));
      if (current !== theme) { await page.getByRole('button', { name: 'Change color theme', exact: true }).click(); await page.waitForTimeout(400); }
      await page.waitForTimeout(200);
      const frame = await page.evaluate(READ, ROLES);
      await page.screenshot({ path: join(shots, `${name}-${theme}.png`), animations: 'disabled' });
      record.frames.push({ name, ...frame });
      console.log('FRAME', name, theme, 'player', frame.geometry.playerBarHeight, 'overflow', frame.geometry.overflow);
    }
    if (await page.evaluate(() => document.documentElement.getAttribute('data-theme')) !== 'dark') {
      await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
      await page.waitForTimeout(400);
    }
  }

  await page.setViewportSize({ width: 1284, height: 856 });
  await navigate(page, 'Home');
  await page.waitForTimeout(500);
  if (await page.evaluate(() => document.documentElement.getAttribute('data-theme')) !== 'light') {
    await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
    await page.waitForTimeout(400);
  }
  await page.waitForTimeout(200);
  record.frames.push({ name: 'home-1284x856', ...(await page.evaluate(READ, ROLES)) });
  await page.screenshot({ path: join(shots, 'home-1284x856-light.png'), animations: 'disabled' });

  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForTimeout(200);
  record.frames.push({ name: 'home-1284x856-reduce', ...(await page.evaluate(READ, ROLES)) });
} finally {
  await browser.close();
  await runtime.close();

  // Provenance, computed from what the pages really resolved.
  const resolved = record.frames.flatMap((frame) => Object.entries(frame.tokens))
    .filter(([, value]) => value !== '');
  record.dr01RolesResolved = [...new Set(resolved.map(([role]) => role))];
  record.isPreEditBaseline = record.dr01RolesResolved.length === 0;
  await writeFile(join(output, `${label}.json`), JSON.stringify(record, null, 2));
  console.log('WROTE', join(output, `${label}.json`),
    'servedStyleSha256=' + servedSha, 'dr01RolesResolved=' + record.dr01RolesResolved.length,
    record.isPreEditBaseline ? 'VALID pre-edit baseline' : 'NOT a pre-edit baseline');
}
