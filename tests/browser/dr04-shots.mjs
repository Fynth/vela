/**
 * DR04 capture: the real web composition at the reference viewports, both
 * themes, both pointer modalities and reduced motion.
 *
 * Every shot is taken from the running application against a real account with
 * real uploaded tracks. No demo catalog, no fabricated cover, no invented
 * numbers: the fixture only owns the files it uploaded itself.
 */
import { chromium, firefox } from '@playwright/test';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { navigate, settle, setupWithTracks, startServer } from './dr04-density.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const output = resolve(root, process.env.VELA_DENSITY_EVIDENCE || 'target/checks/dr04-density');
const shots = join(output, 'shots');
await mkdir(shots, { recursive: true });

const runtime = await startServer(output);
const log = [];

async function shoot(page, name, width, height, note) {
  await page.setViewportSize({ width, height });
  await page.waitForTimeout(400);
  const theme = await page.evaluate(() => document.documentElement.getAttribute('data-theme'));
  const file = join(shots, `${name}.png`);
  await page.screenshot({ path: file, animations: 'disabled' });
  log.push({ name, width, height, theme, note: note || '' });
  console.log('SHOT', name, `${width}x${height}`, 'theme=' + theme, note || '');
}

async function toggleTheme(page) {
  await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
  await page.waitForTimeout(350);
}

const browser = await chromium.launch();
let sessionCookies = [];
try {
  const context = await browser.newContext({ viewport: { width: 1284, height: 856 } });
  const page = await context.newPage();
  await setupWithTracks(page, runtime, 3);
  await page.goto(runtime.origin);
  await settle(page);

  // --- Library, idle: the transport is present but nothing is playing.
  await navigate(page, 'Library');
  await page.locator('.track-row').first().waitFor({ state: 'visible' });
  await shoot(page, 'dr04-library-idle-1284x856-dark', 1284, 856, 'library idle');

  // --- Library with a real current track: the favorite and the transport are
  //     enabled and measurable.
  const title = await page.locator('.track-row .track-title strong').first().innerText();
  await page.getByRole('button', { name: `Play ${title}`, exact: true }).click();
  await page.waitForTimeout(800);
  await shoot(page, 'dr04-library-current-1284x856-dark', 1284, 856, 'library current track');
  await toggleTheme(page);
  await shoot(page, 'dr04-library-current-1284x856-light', 1284, 856, 'library current track');
  await toggleTheme(page);

  // --- The seek rail with a real pointer over it: the native control and the
  //     drawn rail have to agree.
  const range = page.locator('.seek-control input[type=range]');
  const box = await range.boundingBox();
  if (box) {
    await page.mouse.move(box.x + box.width * 0.4, box.y + box.height / 2);
    await page.waitForTimeout(400);
    await shoot(page, 'dr04-library-seek-hover-1284x856-dark', 1284, 856, 'seek rail hovered');
    await page.mouse.move(10, 10);
  }

  // --- Home at the three reference widths, in both themes.
  await navigate(page, 'Home');
  await page.locator('.home-header').waitFor({ state: 'visible' });
  for (const [width, height] of [[1284, 856], [1440, 900], [1728, 1000]]) {
    await shoot(page, `dr04-home-${width}x${height}-dark`, width, height, 'home');
    await toggleTheme(page);
    await shoot(page, `dr04-home-${width}x${height}-light`, width, height, 'home');
    await toggleTheme(page);
  }

  // --- Regression widths: the mobile composition keeps its 120px band.
  await navigate(page, 'Library');
  await page.locator('.track-row').first().waitFor({ state: 'visible' });
  for (const [width, height] of [[320, 700], [360, 800], [390, 844], [768, 1024]]) {
    await shoot(page, `dr04-library-${width}x${height}-dark`, width, height, 'regression width');
  }

  // --- Reduced motion keeps the same geometry.
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.setViewportSize({ width: 1284, height: 856 });
  await page.waitForTimeout(400);
  await shoot(page, 'dr04-library-1284x856-reduced-motion', 1284, 856, 'prefers-reduced-motion: reduce');
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  sessionCookies = await context.cookies();
  await context.close();

  // --- Coarse pointer: the touch composition keeps its own 44px targets.
  const coarse = await browser.newContext({ viewport: { width: 1024, height: 900 }, hasTouch: true });
  const coarsePage = await coarse.newPage();
  // The account already exists on this server; the session is the only thing
  // the new context needs to continue as the same owner.
  await coarse.addCookies(sessionCookies);
  await setupWithTracks(coarsePage, runtime, 1);
  await coarsePage.goto(runtime.origin);
  await settle(coarsePage);
  await navigate(coarsePage, 'Library');
  await coarsePage.locator('.track-row').first().waitFor({ state: 'visible' });
  await shoot(coarsePage, 'dr04-library-1024x900-coarse-dark', 1024, 900, 'pointer: coarse');
  await coarse.close();
} finally {
  await browser.close();
}

// --- Firefox: the native range control geometry is engine specific, so the
//     same composition is captured there as well.
const ff = await firefox.launch();
try {
  const context = await ff.newContext({ viewport: { width: 1284, height: 856 } });
  const page = await context.newPage();
  await context.addCookies(sessionCookies);
  await setupWithTracks(page, runtime, 2);
  await page.goto(runtime.origin);
  await settle(page);
  await navigate(page, 'Library');
  await page.locator('.track-row').first().waitFor({ state: 'visible' });
  const title = await page.locator('.track-row .track-title strong').first().innerText();
  await page.getByRole('button', { name: `Play ${title}`, exact: true }).click();
  await page.waitForTimeout(800);
  const range = page.locator('.seek-control input[type=range]');
  const box = await range.boundingBox();
  if (box) {
    await page.mouse.move(box.x + box.width * 0.4, box.y + box.height / 2);
    await page.waitForTimeout(400);
  }
  await shoot(page, 'dr04-library-current-1284x856-firefox-dark', 1284, 856, 'firefox');
  await context.close();
} finally {
  await ff.close();
  await runtime.close();
}

await writeFile(join(output, 'dr04-shots.json'), JSON.stringify(log, null, 2));
console.log('SHOTS', log.length, 'written to', shots);
