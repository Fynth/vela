/**
 * DR06 baseline probe: the first-screen budget on the mobile composition.
 * Read-only measurement; not a gate. Prints the numbers the package plans
 * against (first track top, both rows bottom, header/filter/toolbar heights).
 */
import { chromium } from '@playwright/test';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { setupWithTracks, settle, navigate, startServer } from './dr04-density.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));

const COLLECT = () => {
  const box = (selector) => {
    const el = document.querySelector(selector);
    if (!el) return null;
    const r = el.getBoundingClientRect();
    const s = getComputedStyle(el);
    return {
      top: +r.top.toFixed(2), bottom: +r.bottom.toFixed(2), height: +r.height.toFixed(2),
      left: +r.left.toFixed(2), width: +r.width.toFixed(2),
      fontSize: s.fontSize, weight: s.fontWeight, display: s.display,
    };
  };
  const rows = [...document.querySelectorAll('.track-row')].map((el) => el.getBoundingClientRect());
  return {
    viewport: { width: innerWidth, height: innerHeight },
    contentTop: getComputedStyle(document.documentElement).getPropertyValue('--vela-content-top').trim(),
    topbar: box('.topbar'),
    scroller: box('.main-scroll'),
    inner: box('.content-inner'),
    heading: box('.page-heading'),
    h1: box('.page-heading h1'),
    toolbar: box('.section-toolbar'),
    toolbarH2: box('.section-toolbar h2'),
    toolbarP: box('.section-toolbar p'),
    search: box('.section-toolbar .search-form'),
    filters: box('.library-filters'),
    filterToolbar: box('.filter-toolbar'),
    filterSummary: box('.filter-summary'),
    trackHeader: box('.track-header'),
    rows: rows.map((r) => ({ top: +r.top.toFixed(2), bottom: +r.bottom.toFixed(2), height: +r.height.toFixed(2) })),
    player: box('.player-bar'),
    nav: box('.mobile-nav'),
    overflow: document.documentElement.scrollWidth - innerWidth,
    filterSummaryOrder: document.querySelector('.filter-summary')?.getAttribute('data-default-order') ?? null,
  };
};

async function main() {
  const output = resolve(root, 'target/checks/dr06-probe');
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  const browser = await chromium.launch();
  const context = await browser.newContext({ viewport: { width: 390, height: 844 } });
  try {
    const page = await context.newPage();
    await setupWithTracks(page, runtime, 2);
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible' });
    const report = [];
    for (const viewport of [{ width: 320, height: 700 }, { width: 390, height: 844 }, { width: 360, height: 800 }]) {
      await page.setViewportSize(viewport);
      await settle(page);
      report.push(await page.evaluate(COLLECT));
    }
    // The report is an artifact, not a console line: the before/after pair is
    // reproducible by running this probe with VELA_TEST_WEB_DIR pointing at
    // either the pre-change stylesheets or the current ones.
    const label = process.env.VELA_DR06_PROBE_LABEL || 'probe';
    await writeFile(resolve(output, `dr06-${label}.json`), JSON.stringify(report, null, 2));
    console.log(JSON.stringify(report, null, 2));
  } finally {
    await context.close().catch(() => {});
    await browser.close();
    await runtime.close();
  }
}

if (import.meta.url === pathToFileURL(process.argv[1] || '').href) await main();
