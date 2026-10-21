import { chromium, firefox } from '@playwright/test';
import fs from 'node:fs';

const BASE = 'http://127.0.0.1:3219';
const SHOTS = '/root/.hermes/profiles/vela/cache/scratch/vela-design-stage/shots';
fs.mkdirSync(SHOTS, { recursive: true });

async function login(page) {
  await page.goto(BASE, { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(2500);
  const inputs = page.locator('input');
  if (await inputs.count() >= 2) {
    await inputs.nth(0).fill('designer');
    await inputs.nth(1).fill('DesignStage2026!');
    await page.getByRole('button', { name: /sign in/i }).first().click();
  }
  await page.waitForTimeout(7000); // WASM boot
}

async function shoot(page, name, width, height, theme) {
  await page.setViewportSize({ width, height });
  if (theme) {
    await page.evaluate((t) => {
      document.documentElement.setAttribute('data-theme', t);
    }, theme);
  }
  await page.waitForTimeout(1200);
  await page.screenshot({ path: `${SHOTS}/${name}.png` });
  console.log('SHOT', name, width + 'x' + height, theme || 'default');
}

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1284, height: 856 } });
await login(page);
console.log('URL', page.url());

// library at 320
await page.getByRole('button', { name: 'Library', exact: true }).first().click();
await page.waitForTimeout(2500);
await shoot(page, '11-library-320x700-dark', 320, 700);
await shoot(page, '13-library-320x700-light', 320, 700, 'light');
await shoot(page, '14-library-1284x856-light', 1284, 856, 'light');
await shoot(page, '15-library-1728x1000-dark', 1728, 1000);

// home 320
await page.getByRole('button', { name: 'Home', exact: true }).first().click();
await page.waitForTimeout(2500);
await shoot(page, '16-home-320x700-dark', 320, 700);
await shoot(page, '17-home-1440x900-light', 1440, 900, 'light');

// context menu open (right-click on a track row)
await page.getByRole('button', { name: 'Library', exact: true }).first().click();
await page.waitForTimeout(2500);
const row = page.locator('.track-menu-trigger, [aria-label^="More actions for"]').first();
if (await row.count()) {
  await row.click();
  await page.waitForTimeout(900);
  await shoot(page, '18-context-menu-1284x856-dark', 1284, 856);
  console.log('MENU OPEN:', await page.locator('.track-menu-panel').count());
} else {
  console.log('no row trigger found');
}

// now playing dialog
const np = page.getByRole('button', { name: /open now playing/i }).first();
if (await np.count()) {
  await np.click();
  await page.waitForTimeout(1200);
  await shoot(page, '19-now-playing-1284x856-dark', 1284, 856);
}

// reduced motion
await page.emulateMedia({ reducedMotion: 'reduce' });
await page.waitForTimeout(800);
await shoot(page, '20-library-1284x856-reduced-motion', 1284, 856);

// firefox pass
const ff = await firefox.launch();
const fpage = await ff.newPage({ viewport: { width: 1284, height: 856 } });
await login(fpage);
await fpage.getByRole('button', { name: 'Library', exact: true }).first().click();
await fpage.waitForTimeout(2500);
await fpage.screenshot({ path: `${SHOTS}/21-library-1284x856-firefox.png` });
console.log('SHOT firefox library');
await ff.close();

await browser.close();
console.log('DONE');
