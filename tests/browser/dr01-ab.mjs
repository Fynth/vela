/**
 * DR01 controlled A/B probe (not a gate): one live document, one account, one
 * dataset, and the only variable is the stylesheet that is actually in force.
 *
 * The library's default order is `ORDER BY t.id ASC` over random UUIDs, so two
 * independent runs show the same rows in a different order and a naive
 * before/after diff measures the UUIDs, not the CSS. This probe therefore:
 *
 *   1. captures the frame with the shipped stylesheet in force;
 *   2. injects the `origin/main` stylesheet (read from git, not from a temp
 *      file) into the same document as the last `<style>`, and captures
 *      again — the DOM, the scroll position and the media state never change;
 *   3. repeats step 2 with the shipped stylesheet, which is the control: two
 *      swaps of identical text must produce the identical diff.
 *
 * A pixel that differs between the shipped and the origin/main frame, but not
 * between the two control frames, would be a real rendering difference.
 *
 * Usage: node dr01-ab.mjs        Artifacts: target/checks/dr01-tokens/ab/
 */
import { chromium } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setupWithTracks, settle, navigate, startServer } from './dr04-density.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const output = resolve(root, 'target/checks/dr01-tokens/ab');
await mkdir(output, { recursive: true });

const shipped = await readFile(resolve(root, 'apps/web/style.css'), 'utf8');
const original = execFileSync('git', ['show', 'origin/main:apps/web/style.css'], { cwd: root, encoding: 'utf8' });
const sha = (text) => createHash('sha256').update(text).digest('hex');
console.log('shipped', sha(shipped), shipped.length);
console.log('origin/main', sha(original), original.length);

const SWAP = (css) => {
  let node = document.getElementById('dr01-ab-style');
  if (!node) { node = document.createElement('style'); node.id = 'dr01-ab-style'; document.head.appendChild(node); }
  node.textContent = css;
  return document.head.contains(node);
};
const REMOVE = () => {
  const node = document.getElementById('dr01-ab-style');
  if (node) node.remove();
};

const runtime = await startServer(output);
const browser = await chromium.launch();
const record = { shippedSha256: sha(shipped), originMainSha256: sha(original), frames: [] };
try {
  const context = await browser.newContext({ viewport: { width: 1284, height: 856 } });
  const page = await context.newPage();
  await setupWithTracks(page, runtime, 3);
  await page.goto(runtime.origin);
  await settle(page);
  await navigate(page, 'Library');
  await page.locator('.track-row').first().waitFor({ state: 'visible' });
  await page.evaluate(() => document.fonts.ready);

  await page.mouse.move(640, 20); // inert: the topbar band, no control under it
  await page.waitForTimeout(200);

  const capture = async (name, css) => {
    if (css === null) await page.evaluate(REMOVE); else await page.evaluate(SWAP, css);
    await page.waitForTimeout(350);
    await page.screenshot({ path: join(output, `${name}.png`), animations: 'disabled' });
    record.frames.push(name);
  };

  await capture('shipped-1', null);
  await capture('original', original);
  await capture('shipped-2', null);
  await capture('original-2', original);
  await capture('shipped-3', null);

  // Same DOM, same scroll, same media state: only the stylesheet text differs.
  record.state = await page.evaluate(() => ({
    rows: [...document.querySelectorAll('.track-row .track-title strong')].map((n) => n.textContent.trim()),
    scrollTop: document.querySelector('.main-scroll').scrollTop,
    theme: document.documentElement.getAttribute('data-theme'),
  }));
  console.log('state:', JSON.stringify(record.state));
} finally {
  await browser.close();
  await runtime.close();
  await writeFile(join(output, 'ab.json'), JSON.stringify(record, null, 2));
  console.log('WROTE', join(output, 'ab.json'));
}
