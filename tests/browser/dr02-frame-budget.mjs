/**
 * The DR02 frame budget, measured in a browser the test runner does not
 * instrument.
 *
 * The budget question is: what does the *product* cost when the catalog
 * scrolls under two blurred bands? The answer has to come from a browser
 * that carries nothing but the product — and the Playwright test runner
 * attaches to every browser it starts: its client process, its trace
 * agent and its own machinery all sit next to the renderer while the
 * measurement runs. On this host the difference is not small: the same
 * page, the same browser, the same driver, the same server and the same
 * session read 0.17 points of frames over 33ms in a plain process and
 * 15–24 points inside the runner. The plain number is the product's
 * number; the runner's number is the harness's.
 *
 * So the measurement runs here, in a process of its own, and reports:
 * the two runs (the glass and the opaque fallback), the surface the page
 * actually painted, and the scroll range both runs covered.
 *
 * Usage: node dr02-frame-budget.mjs <origin> <storage-state.json> [engine]
 * Prints one JSON line on stdout.
 */
import { chromium, firefox } from '@playwright/test';

const [origin, statePath, engine = 'chromium'] = process.argv.slice(2);
if (!origin || !statePath) {
  console.error('usage: dr02-frame-budget.mjs <origin> <storage-state.json> [engine]');
  process.exit(2);
}

const launcher = engine === 'firefox' ? firefox : chromium;
const browser = await launcher.launch();
const context = await browser.newContext({
  viewport: { width: 1284, height: 856 },
  storageState: statePath,
});
const page = await context.newPage();

await page.goto(origin);
await page.waitForSelector('.app-shell', { timeout: 30000 });
await page.locator('.sidebar').getByRole('button', { name: 'Library', exact: true }).click();
await page.waitForSelector('.track-row', { timeout: 30000 });
await page.evaluate(() => document.fonts.ready);

/** The surface the page really paints: the filters and the glass under them. */
const painted = await page.evaluate(() => {
  const read = selector => {
    const style = getComputedStyle(document.querySelector(selector));
    return {
      background: style.backgroundColor,
      filter: style.backdropFilter || style.webkitBackdropFilter || 'none',
    };
  };
  return { player: read('.player-bar'), topbar: read('.topbar') };
}).catch(() => null);

/** Ten seconds of scrolling, measured with the page's own animation clock. */
const scrollFor = async (milliseconds) => await page.evaluate(async (ms) => {
  const scroller = document.querySelector('#main-scroll');
  const range = Math.max(1, scroller.scrollHeight - scroller.clientHeight);
  const deltas = [];
  let previous = performance.now();
  const start = previous;
  return await new Promise((done) => {
    const step = now => {
      deltas.push(now - previous);
      previous = now;
      const phase = (now - start) / ms;
      scroller.scrollTop = (Math.sin(phase * Math.PI * 8) * 0.5 + 0.5) * range;
      if (now - start < ms) requestAnimationFrame(step);
      else {
        const slow = deltas.filter(delta => delta > 33).length;
        done({
          frames: deltas.length,
          slow,
          slowShare: +(slow / Math.max(1, deltas.length) * 100).toFixed(2),
          median: +deltas.slice().sort((a, b) => a - b)[Math.floor(deltas.length / 2)].toFixed(2),
          longest: +Math.max(...deltas).toFixed(2),
          range,
          elements: document.querySelectorAll('*').length,
        });
      }
    };
    requestAnimationFrame(step);
  });
}, milliseconds);

const runs = [];
runs.push({ mode: 'glass', ...(await scrollFor(10000)) });
await page.addStyleTag({
  content: '.player-bar,.topbar{backdrop-filter:none !important;-webkit-backdrop-filter:none !important;}',
});
await page.waitForTimeout(300);
runs.push({ mode: 'opaque', ...(await scrollFor(10000)) });

const renderer = await page.evaluate(() => {
  const canvas = document.createElement('canvas');
  const gl = canvas.getContext('webgl');
  if (!gl) return 'no webgl';
  const debug = gl.getExtension('WEBGL_debug_renderer_info');
  return debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER);
}).catch(() => 'unknown');

console.log(JSON.stringify({
  engine, origin, painted, runs, renderer,
  userAgent: await page.evaluate(() => navigator.userAgent).catch(() => 'unknown'),
}));

await context.close();
await browser.close();
