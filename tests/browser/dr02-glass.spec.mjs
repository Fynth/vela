/**
 * DR02 acceptance: one main scroller under two glass bands.
 *
 * DR02 is the first package that lets the catalog pass under the topbar and
 * the player. The shell keeps its grid, its sidebar, its one HTMLMediaElement
 * and its one scroller; what changes is the geometry of that scroller and the
 * surface of the two floating bands.
 *
 * The journeys below prove seven things and nothing more.
 *
 *  1. G1 underlap. The content column spans every shell row, the one
 *     scroller starts above the topbar's bottom edge and ends at the bottom
 *     of the viewport, the first heading keeps its old y at scroll 0, real
 *     rows pass under both bands while scrolling, and the last row can still
 *     be scrolled entirely above the player.
 *  2. G2 filter budget. Exactly two shell consumers of `backdrop-filter`
 *     (the player and the topbar) and at most three visible ones while one
 *     overlay is open. No long list carries a filter.
 *  3. G3 composite contrast. The labels on the glass are measured against
 *     the colour that is actually painted under them: the translucent stack
 *     of the element is composited over the real backdrop pixels, and over
 *     the worst realistic backdrops (a bright cover, the placeholder, the
 *     page surface), in both themes.
 *  4. G4 effective rectangle. The rectangle between the two bands is
 *     `scroller.top + topbar` to `scroller.bottom - reserve`, and the last
 *     focused 44px target lands inside it with 8px to spare.
 *  5. G5 modes. Reduced motion leaves no transition and no animation on the
 *     bands, forced colours and print drop the blur, and the bands keep an
 *     opaque surface in every one of those modes.
 *  6. G6 preservation. Scrolling, the theme and the overlay keep the same
 *     media element, the same source and the same position.
 *  7. G7 frame budget. The same 10s scroll is measured with the glass and
 *     with the opaque fallback in the same viewport; the difference in the
 *     share of frames longer than 33ms is the budget this package spends.
 *     This one journey runs its measurement in a process of its own: the
 *     Playwright test runner attaches to every browser it starts, and that
 *     harness cost is worth 15.7–24.4 points of frames over 33ms for the
 *     same page. `dr02-frame-budget.mjs` is that process.
 *
 * The spec owns its server, its account and its real uploaded tracks. The
 * six journeys that use the page fixture never launch a browser of their
 * own: the engine that runs the spec is the engine it measures, and the
 * engine name is part of every artifact.
 *
 * Artifacts (relative to the repository root):
 *   target/checks/dr02-glass/<engine>/geometry.json
 *   target/checks/dr02-glass/<engine>/contrast.json
 *   target/checks/dr02-glass/<engine>/filters.json
 *   target/checks/dr02-glass/<engine>/focus.json
 *   target/checks/dr02-glass/<engine>/modes.json
 *   target/checks/dr02-glass/<engine>/preservation.json
 *   target/checks/dr02-glass/<engine>/performance.json
 *   target/checks/dr02-glass/shots/<engine>/*.png
 */
import { test, expect } from '@playwright/test';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { navigate, settle, startServer } from './dr04-density.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const output = resolve(root, process.env.VELA_DR02_EVIDENCE || 'target/checks/dr02-glass');
/** The bright cover fixture: a real FLAC with an embedded 800x800 cover. */
const brightCover = process.env.VELA_DR02_BRIGHT_COVER
  || '/root/.hermes/profiles/vela/cache/scratch/vela-tl/dr02/bright-cover.flac';

/* ------------------------------------------------------------------ colour */

/** Parses a computed colour into channels and an alpha. */
const parseColour = (value) => {
  const parts = String(value).match(/[\d.]+/g);
  if (!parts) return null;
  return {
    r: Number(parts[0]), g: Number(parts[1]), b: Number(parts[2]),
    a: parts.length > 3 ? Number(parts[3]) : 1,
  };
};

/** The WCAG relative luminance of a colour. */
const luminance = ({ r, g, b }) => [r, g, b]
  .map(value => value / 255)
  .map(value => (value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4))
  .reduce((total, value, at) => total + value * [0.2126, 0.7152, 0.0722][at], 0);

/** The `over` operator of the alpha compositing model. */
const over = (top, bottom) => ({
  r: top.r * top.a + bottom.r * (1 - top.a),
  g: top.g * top.a + bottom.g * (1 - top.a),
  b: top.b * top.a + bottom.b * (1 - top.a),
  a: 1,
});

/** The WCAG contrast ratio between two opaque colours. */
const ratio = (foreground, background) => {
  const a = luminance(foreground);
  const b = luminance(background);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
};

/* ------------------------------------------------------------------ fixture */

/** A real PCM wave, so every upload is a real original. */
function pcmWave(duration, frequency) {
  const samples = 44100 * duration, wave = Buffer.alloc(44 + samples * 2);
  wave.write('RIFF', 0); wave.writeUInt32LE(36 + samples * 2, 4); wave.write('WAVEfmt ', 8);
  wave.writeUInt16LE(16, 16); wave.writeUInt16LE(1, 20); wave.writeUInt16LE(1, 22);
  wave.writeUInt32LE(44100, 24); wave.writeUInt32LE(88200, 28); wave.writeUInt16LE(2, 32);
  wave.writeUInt16LE(16, 34); wave.write('data', 36); wave.writeUInt32LE(samples * 2, 40);
  for (let at = 0; at < samples; at += 1) {
    wave.writeInt16LE(Math.round(1500 * Math.sin(2 * Math.PI * frequency * at / 44100)), 44 + at * 2);
  }
  return wave;
}

/**
 * A real account with real originals. The bright cover is optional: when the
 * fixture file is missing the caller skips its own journey and says why.
 */
async function setupAccount(page, runtime, { tracks = 12, cover = null, long = false } = {}) {
  const setup = await page.request.post(runtime.origin + '/api/setup', {
    headers: { Origin: runtime.origin },
    data: { username: 'admin', password: 'private glass fixture password' },
  });
  if (!setup.ok() && setup.status() !== 409) throw new Error('setup failed: ' + setup.status());
  const me = setup.ok() ? await setup.json() : await (await page.request.get(runtime.origin + '/api/me')).json();
  const headers = { 'X-CSRF-Token': me.csrf_token, Origin: runtime.origin };
  let coverTrack = null;
  if (cover) {
    const upload = await page.request.post(runtime.origin + '/api/upload', {
      headers, multipart: { file: { name: 'Bright cover.flac', mimeType: 'audio/flac', buffer: await readFile(cover) } },
    });
    if (!upload.ok()) throw new Error('cover upload failed: ' + upload.status());
    coverTrack = (await upload.json()).track;
  }
  for (let at = 0; at < tracks; at += 1) {
    const upload = await page.request.post(runtime.origin + '/api/upload', {
      headers,
      multipart: {
        file: {
          name: `Glass ${String(at + 1).padStart(2, '0')}.wav`,
          mimeType: 'audio/wav',
          buffer: pcmWave(long ? 30 : 4, 420 + at * 17.5),
        },
      },
    });
    if (!upload.ok()) throw new Error('upload failed: ' + upload.status());
  }
  return { csrf: me.csrf_token, coverTrack };
}

/* ------------------------------------------------------- in-page geometry */

/**
 * The shell geometry. Everything is measured with getBoundingClientRect on
 * the running document, never with a declared CSS value.
 */
const GEOMETRY = () => {
  const box = selector => {
    const element = document.querySelector(selector);
    if (!element) return null;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return {
      top: +rect.top.toFixed(2), bottom: +rect.bottom.toFixed(2),
      left: +rect.left.toFixed(2), right: +rect.right.toFixed(2),
      width: +rect.width.toFixed(2), height: +rect.height.toFixed(2),
      display: style.display, position: style.position, zIndex: style.zIndex,
      background: style.backgroundColor, backdropFilter: style.backdropFilter || style.webkitBackdropFilter,
      paddingTop: parseFloat(style.paddingTop), paddingBottom: parseFloat(style.paddingBottom),
      overflowY: style.overflowY, scrollHeight: element.scrollHeight, clientHeight: element.clientHeight,
    };
  };
  /** A probe resolves a token (and any calc/env inside it) to real pixels. */
  const probe = property => {
    const element = document.createElement('div');
    element.style.cssText = `position:absolute;left:-9999px;top:0;height:${property};width:10px`;
    document.body.append(element);
    const value = element.getBoundingClientRect().height;
    element.remove();
    return +value.toFixed(2);
  };
  const scroller = document.querySelector('#main-scroll');
  const rows = [...document.querySelectorAll('.track-row')].map(element => {
    const rect = element.getBoundingClientRect();
    return {
      top: +rect.top.toFixed(2), bottom: +rect.bottom.toFixed(2),
      title: (element.querySelector('.track-title strong') || {}).textContent || '',
    };
  });
  const headingElement = document.querySelector('.page-heading h1');
  const heading = headingElement ? headingElement.getBoundingClientRect() : null;
  const bodyScrollers = [...document.querySelectorAll('*')]
    .filter(element => {
      const style = getComputedStyle(element);
      if (!['auto', 'scroll'].includes(style.overflowY)) return false;
      if (element.scrollHeight <= element.clientHeight + 1) return false;
      const rect = element.getBoundingClientRect();
      return style.display !== 'none' && rect.width > 0 && rect.height > 0;
    })
    .map(element => (element.id === 'main-scroll' ? 'main-scroll' : String(element.className || element.tagName)));
  return {
    viewport: { width: innerWidth, height: innerHeight },
    shell: box('.app-shell'),
    main: box('main.main-content'),
    topbar: box('.topbar'),
    scroller: box('#main-scroll'),
    inner: box('.content-inner'),
    player: box('.player-bar'),
    nav: box('.mobile-nav'),
    rows,
    heading: heading ? { top: +heading.top.toFixed(2), bottom: +heading.bottom.toFixed(2) } : null,
    bodyScrollers,
    reserve: probe('var(--vela-bottom-reserve)'),
    contentTop: probe('var(--vela-content-top)'),
    scrollTop: scroller ? +scroller.scrollTop.toFixed(2) : 0,
    scrollHeight: scroller ? scroller.scrollHeight : 0,
    clientHeight: scroller ? scroller.clientHeight : 0,
  };
};

/* ------------------------------------------------------ in-page filters */

/**
 * Every visible consumer of a backdrop filter, pseudo-elements included.
 * A pseudo-element counts only when it is generated (`content` is not none)
 * and the `::backdrop` of a dialog counts only while the dialog is open.
 */
const FILTER_USERS = () => {
  const describe = element => {
    const id = element.id ? '#' + element.id : '';
    const cls = typeof element.className === 'string' && element.className.trim()
      ? '.' + element.className.trim().split(/\s+/).join('.') : '';
    return element.tagName.toLowerCase() + id + cls;
  };
  const shown = element => {
    const style = getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    return style.display !== 'none' && style.visibility !== 'hidden'
      && rect.width > 0 && rect.height > 0;
  };
  const users = [];
  const push = (where, kind, value) => users.push({ where, kind, value });
  for (const element of document.querySelectorAll('*')) {
    if (shown(element)) {
      const style = getComputedStyle(element);
      const value = style.backdropFilter || style.webkitBackdropFilter || 'none';
      if (value && value !== 'none') push(describe(element), 'element', value);
    }
    for (const pseudo of ['::before', '::after']) {
      let style;
      try { style = getComputedStyle(element, pseudo); } catch { continue; }
      if (!style) continue;
      const value = style.backdropFilter || style.webkitBackdropFilter || 'none';
      if (!value || value === 'none') continue;
      if (style.content === 'none') continue;
      if (style.display === 'none' || style.visibility === 'hidden') continue;
      push(describe(element) + pseudo, pseudo, value);
    }
    // A dialog's backdrop is only painted while the dialog is open.
    if (element.tagName === 'DIALOG' && element.open) {
      let style;
      try { style = getComputedStyle(element, '::backdrop'); } catch { style = null; }
      const value = style && (style.backdropFilter || style.webkitBackdropFilter || 'none');
      if (value && value !== 'none') push(describe(element) + '::backdrop', '::backdrop', value);
    }
  }
  return users;
};

/* ------------------------------------------------------- in-page contrast */

/**
 * The labels of the two bands with their own colour, their band and the
 * translucent stack of backgrounds between the label and the band.
 */
const LABELS = () => {
  const selectors = [
    ['.player-time', 'text'], ['.player-artist', 'text'], ['.player-album', 'text'],
    ['.player-quality small', 'text'], ['.player-quality-format', 'text'],
    ['.now-playing strong.player-title', 'text'], ['.topbar .breadcrumb', 'text'],
    ['.player-buttons .icon-button .vela-icon', 'glyph'],
    ['.player-right .icon-button .vela-icon', 'glyph'],
    ['.player-favorite .vela-icon', 'glyph'],
  ];
  const parse = value => {
    const parts = String(value).match(/[\d.]+/g);
    if (!parts) return null;
    return {
      r: Number(parts[0]), g: Number(parts[1]), b: Number(parts[2]),
      a: parts.length > 3 ? Number(parts[3]) : 1,
    };
  };
  const out = [];
  for (const [selector, kind] of selectors) {
    for (const element of document.querySelectorAll(selector)) {
      if (!element.checkVisibility()) continue;
      const rect = element.getBoundingClientRect();
      if (rect.width < 2 || rect.height < 2) continue;
      const style = getComputedStyle(element);
      const band = element.closest('.player-bar') || element.closest('.topbar');
      if (!band) continue;
      // The translucent stack between the label and the band.
      const stack = [];
      for (let node = element; node; node = node.parentElement) {
        const background = parse(getComputedStyle(node).backgroundColor);
        if (background && background.a > 0) stack.push(background);
        if (node === band) break;
      }
      out.push({
        selector, kind, band: band.className,
        text: (element.textContent || '').trim().slice(0, 40),
        colour: style.color, fontSize: parseFloat(style.fontSize), weight: style.fontWeight,
        glass: getComputedStyle(band).backgroundColor,
        rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
        stack: stack.reverse(),
      });
    }
  }
  return out;
};

/**
 * Samples the painted backdrop under every label and composites the
 * translucent stack of the label over it. The result is the colour the label
 * really stands on, and the worst (lowest) ratio over the sampled pixels is
 * the honest number for that label.
 */
const SAMPLE_BACKDROP = async ({ url, labels }) => {
  const image = new Image();
  image.src = url;
  await image.decode();
  const canvas = document.createElement('canvas');
  canvas.width = image.width;
  canvas.height = image.height;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  context.drawImage(image, 0, 0);
  const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
  const parse = value => {
    const parts = String(value).match(/[\d.]+/g);
    if (!parts) return null;
    return {
      r: Number(parts[0]), g: Number(parts[1]), b: Number(parts[2]),
      a: parts.length > 3 ? Number(parts[3]) : 1,
    };
  };
  const lum = colour => [colour.r, colour.g, colour.b]
    .map(value => value / 255)
    .map(value => (value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4))
    .reduce((total, value, at) => total + value * [0.2126, 0.7152, 0.0722][at], 0);
  const over = (top, bottom) => ({
    r: top.r * top.a + bottom.r * (1 - top.a),
    g: top.g * top.a + bottom.g * (1 - top.a),
    b: top.b * top.a + bottom.b * (1 - top.a),
    a: 1,
  });
  const ratio = (foreground, background) => {
    const a = lum(foreground), b = lum(background);
    return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
  };
  return labels.map(label => {
    const foreground = parse(label.colour);
    const x0 = Math.max(0, Math.floor(label.rect.x));
    const y0 = Math.max(0, Math.floor(label.rect.y));
    const x1 = Math.min(canvas.width, Math.ceil(label.rect.x + label.rect.width));
    const y1 = Math.min(canvas.height, Math.ceil(label.rect.y + label.rect.height));
    let worst = null, sampled = 0, mean = { r: 0, g: 0, b: 0 };
    for (let y = y0; y < y1; y += 1) {
      for (let x = x0; x < x1; x += 1) {
        const at = (y * canvas.width + x) * 4;
        const pixel = { r: pixels[at], g: pixels[at + 1], b: pixels[at + 2], a: 1 };
        sampled += 1;
        mean = { r: mean.r + pixel.r, g: mean.g + pixel.g, b: mean.b + pixel.b, a: 1 };
        let composed = pixel;
        for (const layer of label.stack) composed = over(layer, composed);
        const value = ratio(foreground, composed);
        if (worst === null || value < worst.ratio) worst = { ratio: value, composed };
      }
    }
    if (sampled > 0) mean = { r: mean.r / sampled, g: mean.g / sampled, b: mean.b / sampled, a: 1 };
    return {
      selector: label.selector, kind: label.kind, text: label.text,
      fontSize: label.fontSize, weight: label.weight, colour: label.colour,
      sampled, worst: worst ? worst.ratio : null, worstColour: worst ? worst.composed : null,
      meanColour: sampled ? mean : null,
    };
  });
};

/* ------------------------------------------------------------------ tests */

test('DR02 the catalog runs under the topbar and the player, and the first frame does not jump', async ({ page }) => {
  test.setTimeout(240000);
  const engine = page.context().browser().browserType().name();
  const engineOutput = join(output, engine);
  const shotOutput = join(output, 'shots', engine);
  await mkdir(engineOutput, { recursive: true });
  await mkdir(shotOutput, { recursive: true });
  const runtime = await startServer(engineOutput);
  const evidence = { engine, frames: [] };
  try {
    await setupAccount(page, runtime, { tracks: 20 });
    await page.setViewportSize({ width: 1284, height: 856 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible' });

    for (const [width, height] of [[1284, 856], [1440, 900], [1728, 1000], [390, 844], [360, 800], [320, 780]]) {
      await page.setViewportSize({ width, height });
      await page.waitForTimeout(350);
      const mobile = width <= 760;

      // The first frame: the heading keeps the y it had before the underlap.
      const first = await page.evaluate(GEOMETRY);
      expect.soft(first.scroller.top, `${width} the scroller starts under the topbar`)
        .toBeLessThan(first.topbar.bottom);
      expect.soft(first.scroller.bottom, `${width} the scroller reaches the viewport bottom`)
        .toBeCloseTo(height, 0);
      expect.soft(first.main.bottom, `${width} the content column spans every row`)
        .toBeCloseTo(height, 0);
      expect.soft(first.inner.top, `${width} the content starts below the topbar`)
        .toBeCloseTo(first.topbar.bottom, 0);
      expect.soft(first.inner.paddingTop, `${width} the first heading keeps its inset`)
        .toBeCloseTo(first.contentTop, 0);
      expect.soft(first.heading.top, `${width} the first heading keeps its y`)
        .toBeGreaterThanOrEqual(first.inner.top + first.contentTop - 0.5);
      expect.soft(first.heading.top, `${width} the first heading keeps its y`)
        .toBeLessThanOrEqual(first.inner.top + first.contentTop + 8);
      expect.soft(first.inner.paddingBottom, `${width} the content owns the bottom reserve`)
        .toBeCloseTo(first.reserve + (mobile ? 20 : 24), 0);
      expect.soft(first.bodyScrollers.filter(name => name !== 'sidebar'), `${width} one central scroller`)
        .toEqual(['main-scroll']);
      expect.soft(first.scroller.overflowY, `${width} the central scroller scrolls`).toMatch(/auto|scroll/);
      if (mobile) {
        expect.soft(first.nav.bottom, `${width} the navigation still ends at the viewport`).toBeCloseTo(height, 0);
        expect.soft(first.player.bottom, `${width} the mini-player still sits above the navigation`)
          .toBeCloseTo(first.nav.top, 0);
        expect.soft(first.reserve, `${width} the mobile reserve`).toBeCloseTo(190, 0);
      } else {
        expect.soft(first.player.bottom, `${width} the player still ends at the viewport`).toBeCloseTo(height, 0);
        expect.soft(first.reserve, `${width} the desktop reserve`).toBeCloseTo(88, 0);
      }
      await page.screenshot({ path: join(shotOutput, `underlap-${width}x${height}-top.png`), animations: 'disabled' });

      // Scrolled: real rows pass under both bands, and the bands stay sharp.
      // The scroll position is computed from the first real row, so a row is
      // under the topbar by construction and the assertion is about the
      // geometry rather than about a lucky scroll offset.
      const offset = await page.evaluate(() => {
        const scroller = document.querySelector('#main-scroll');
        const first = document.querySelector('.track-row');
        if (!first) return null;
        return {
          top: first.getBoundingClientRect().top + scroller.scrollTop,
          band: document.querySelector('.topbar').getBoundingClientRect().height,
        };
      });
      expect(offset, `${width} the list has a first row`).not.toBeNull();
      await page.locator('#main-scroll').evaluate((element, value) => { element.scrollTop = value; },
        Math.max(0, Math.round(offset.top - offset.band + 20)));
      await page.waitForTimeout(250);
      const scrolled = await page.evaluate(GEOMETRY);
      const underTop = scrolled.rows.filter(row => row.top < scrolled.topbar.bottom && row.bottom > scrolled.topbar.top);
      const underPlayer = scrolled.rows.filter(row => row.top < scrolled.player.bottom && row.bottom > scrolled.player.top);
      expect.soft(underTop.length, `${width} a real row passes under the topbar`).toBeGreaterThan(0);
      expect.soft((underTop[0]?.title || '').trim(), `${width} the row under the topbar is a real track`).not.toBe('');
      expect.soft(underPlayer.length, `${width} a real row passes under the player`).toBeGreaterThan(0);
      expect.soft((underPlayer[0]?.title || '').trim(), `${width} the row under the player is a real track`).not.toBe('');
      expect.soft(scrolled.scrollTop, `${width} the scroller really moved`).toBeGreaterThan(0);
      await page.screenshot({ path: join(shotOutput, `underlap-${width}x${height}-scrolled.png`), animations: 'disabled' });

      // The end of the list: the last row is entirely above the bands.
      await page.locator('#main-scroll').evaluate(element => { element.scrollTop = element.scrollHeight; });
      await page.waitForTimeout(250);
      const end = await page.evaluate(GEOMETRY);
      const lastRow = end.rows[end.rows.length - 1];
      // The content has to clear the topmost band it floats over: the player
      // on the desktop and on the mobile composition alike.
      const bandTop = end.player.top;
      expect.soft(lastRow.bottom, `${width} the last row is above the bands`).toBeLessThanOrEqual(bandTop);
      expect.soft(end.inner.bottom, `${width} the content ends above the bands`)
        .toBeLessThanOrEqual(end.scroller.bottom + 1);
      expect.soft(end.scrollTop, `${width} the scroller reached its end`).toBeGreaterThan(0);
      await page.screenshot({ path: join(shotOutput, `underlap-${width}x${height}-end.png`), animations: 'disabled' });

      evidence.frames.push({ width, height, first, scrolled, end, underTop, underPlayer });
      await page.locator('#main-scroll').evaluate(element => { element.scrollTop = 0; });
    }
    await writeFile(join(engineOutput, 'geometry.json'), JSON.stringify(evidence, null, 2));
  } finally {
    await runtime.close();
  }
});

test('DR02 exactly two shell filters, and at most three with one overlay open', async ({ page }) => {
  test.setTimeout(240000);
  const engine = page.context().browser().browserType().name();
  const engineOutput = join(output, engine);
  await mkdir(engineOutput, { recursive: true });
  const runtime = await startServer(engineOutput);
  const evidence = { engine, states: [] };
  try {
    await setupAccount(page, runtime, { tracks: 6 });
    await page.setViewportSize({ width: 1284, height: 856 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible' });

    const users = await page.evaluate(FILTER_USERS);
    evidence.states.push({ state: 'library', users });
    expect.soft(users.length, 'exactly two shell consumers of a backdrop filter').toBe(2);
    expect.soft(users.map(user => user.where).sort().join(','), 'the two consumers are the player and the topbar')
      .toBe('div.player-bar,header.topbar');

    // No filter on a long list.
    const listFilters = await page.evaluate(() => [...document.querySelectorAll('.track-row, .catalog-grid, .playlist-grid, .panel, .listening-card')]
      .map(element => {
        const style = getComputedStyle(element);
        return { where: element.className, value: style.backdropFilter || style.webkitBackdropFilter || 'none' };
      })
      .filter(entry => entry.value && entry.value !== 'none'));
    expect.soft(listFilters.length, 'no long list carries a backdrop filter').toBe(0);

    // One overlay open: the dialog backdrop is the third and last layer.
    await page.getByRole('button', { name: 'Show playback queue', exact: true }).first().click();
    await expect(page.getByRole('complementary', { name: 'Playback queue', exact: true })).toBeVisible();
    await page.waitForTimeout(250);
    const withQueue = await page.evaluate(FILTER_USERS);
    evidence.states.push({ state: 'queue', users: withQueue });
    expect.soft(withQueue.length, 'three visible filters with one overlay open').toBeLessThanOrEqual(3);
    expect.soft(withQueue.length, 'the queue backdrop is the third layer').toBe(3);
    await page.getByRole('button', { name: 'Close playback queue', exact: true }).click();
    await page.waitForTimeout(250);

    // Now Playing is the other overlay; the two never stack.
    await page.locator('.track-row .track-play').first().click();
    await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
    await page.locator('.now-playing-trigger').click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.waitForTimeout(250);
    const withNowPlaying = await page.evaluate(FILTER_USERS);
    evidence.states.push({ state: 'now-playing', users: withNowPlaying });
    expect.soft(withNowPlaying.length, 'three visible filters with the sheet open').toBeLessThanOrEqual(3);
    expect.soft(
      withNowPlaying.filter(user => user.kind === '::backdrop').length,
      'the two overlays never stack: one dialog backdrop at a time',
    ).toBe(1);
    await page.keyboard.press('Escape');
    await page.waitForTimeout(250);
    const closed = await page.evaluate(FILTER_USERS);
    expect.soft(closed.length, 'the shell keeps its two filters after the overlay closed').toBe(2);

    await writeFile(join(engineOutput, 'filters.json'), JSON.stringify(evidence, null, 2));
  } finally {
    await runtime.close();
  }
});

test('DR02 the labels on the glass stay readable over a real cover and over the real backdrop', async ({ page }) => {
  test.setTimeout(300000);
  const engine = page.context().browser().browserType().name();
  const engineOutput = join(output, engine);
  await mkdir(engineOutput, { recursive: true });
  const fixture = await readFile(brightCover).catch(() => null);
  test.skip(fixture === null, `the bright cover fixture is missing: ${brightCover}`);
  const runtime = await startServer(engineOutput);
  const evidence = { engine, cover: brightCover, cases: [] };
  try {
    const { coverTrack } = await setupAccount(page, runtime, { tracks: 10, cover: brightCover });
    await page.setViewportSize({ width: 1284, height: 856 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible' });
    await page.getByRole('button', { name: `Play ${coverTrack.title}`, exact: true }).first().click();
    await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
    await page.waitForTimeout(300);

    // The real colour of the uploaded cover, read from the image itself.
    const coverColour = await page.evaluate(async () => {
      const image = document.querySelector('.now-playing .artwork img');
      if (!image) return null;
      await image.decode();
      const canvas = document.createElement('canvas');
      canvas.width = image.naturalWidth;
      canvas.height = image.naturalHeight;
      const context = canvas.getContext('2d');
      context.drawImage(image, 0, 0);
      const data = context.getImageData(
        Math.floor(image.naturalWidth / 2), Math.floor(image.naturalHeight / 2), 1, 1,
      ).data;
      return { r: data[0], g: data[1], b: data[2] };
    });
    expect(coverColour, 'the cover of the fixture is readable').not.toBeNull();
    evidence.coverColour = coverColour;

    // The hidden-band style lets the test photograph the real backdrop.
    await page.addStyleTag({ content: 'html[data-dr02-hide-bands] .player-bar, html[data-dr02-hide-bands] .topbar { visibility: hidden !important; }' });

    const setTheme = async theme => {
      const current = await page.locator('html').getAttribute('data-theme');
      if (current !== theme) {
        await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
        await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
        await page.waitForTimeout(300);
      }
    };

    for (const theme of ['dark', 'light']) {
      await setTheme(theme);
      for (const scroll of [0, 400, 'end']) {
        await page.locator('#main-scroll').evaluate((element, top) => {
          element.scrollTop = top === 'end' ? element.scrollHeight : top;
        }, scroll);
        await page.waitForTimeout(250);
        const labels = await page.evaluate(LABELS);
        expect(labels.length, `${theme} ${scroll} the bands carry labels`).toBeGreaterThan(0);

        // The focus ring of a control in the band is painted on the band, so
        // it stands on the same composited surface and needs the same 3:1.
        const ring = await page.evaluate(() => {
          const toggle = document.querySelector('#player-toggle');
          const band = document.querySelector('.player-bar');
          if (!toggle || !band) return null;
          toggle.focus();
          const style = getComputedStyle(toggle);
          const rect = toggle.getBoundingClientRect();
          const offset = parseFloat(style.outlineOffset) || 0;
          const width = parseFloat(style.outlineWidth) || 0;
          const background = getComputedStyle(band).backgroundColor;
          const parts = background.match(/[\d.]+/g);
          return {
            selector: '#player-toggle focus ring', kind: 'glyph', band: 'player-bar',
            text: 'focus ring', colour: style.outlineColor, fontSize: 0, weight: 0,
            glass: background,
            rect: {
              x: rect.x - offset - width, y: rect.y - offset - width,
              width: rect.width + 2 * (offset + width), height: rect.height + 2 * (offset + width),
            },
            stack: [{
              r: Number(parts[0]), g: Number(parts[1]), b: Number(parts[2]),
              a: parts.length > 3 ? Number(parts[3]) : 1,
            }],
          };
        });
        if (ring) labels.push(ring);

        // The real painted backdrop under every label.
        await page.evaluate(() => document.documentElement.setAttribute('data-dr02-hide-bands', '1'));
        await page.waitForTimeout(120);
        const shot = await page.screenshot({ animations: 'disabled' });
        await page.evaluate(() => document.documentElement.removeAttribute('data-dr02-hide-bands'));
        await page.waitForTimeout(120);
        const sampled = await page.evaluate(SAMPLE_BACKDROP, {
          url: 'data:image/png;base64,' + shot.toString('base64'),
          labels,
        });

        for (const label of sampled) {
          if (label.worst === null) continue;
          const threshold = label.kind === 'glyph' ? 3 : 4.5;
          expect.soft(
            label.worst,
            `${theme} ${scroll} ${label.selector} "${label.text}" over the real backdrop`,
          ).toBeGreaterThanOrEqual(threshold);
        }

        // The worst realistic backdrops: the bright cover of the fixture, the
        // placeholder of a track without artwork and the page surface.
        const surfaces = await page.evaluate(() => ({
          placeholder: getComputedStyle(document.querySelector('.track-row .artwork') || document.body).backgroundColor,
          page: getComputedStyle(document.body).backgroundColor,
        }));
        for (const label of labels) {
          const foreground = parseColour(label.colour);
          const backdrops = {
            brightCover: { ...coverColour, a: 1 },
            placeholder: parseColour(surfaces.placeholder) || { r: 0, g: 0, b: 0, a: 1 },
            page: parseColour(surfaces.page) || { r: 0, g: 0, b: 0, a: 1 },
          };
          for (const [name, backdrop] of Object.entries(backdrops)) {
            let composed = { ...backdrop, a: 1 };
            for (const layer of label.stack) composed = over(layer, composed);
            const value = ratio(foreground, composed);
            const threshold = label.kind === 'glyph' ? 3 : 4.5;
            expect.soft(
              value,
              `${theme} ${scroll} ${label.selector} "${label.text}" over the ${name}`,
            ).toBeGreaterThanOrEqual(threshold);
            evidence.cases.push({
              theme, scroll, selector: label.selector, kind: label.kind, text: label.text,
              backdrop: name, ratio: +value.toFixed(3), composed,
            });
          }
        }
        evidence.cases.push({ theme, scroll, sampled });
      }
    }
    await writeFile(join(engineOutput, 'contrast.json'), JSON.stringify(evidence, null, 2));
  } finally {
    await runtime.close();
  }
});

test('DR02 the effective rectangle keeps the last 44px target reachable', async ({ page }) => {
  test.setTimeout(240000);
  const engine = page.context().browser().browserType().name();
  const engineOutput = join(output, engine);
  await mkdir(engineOutput, { recursive: true });
  const runtime = await startServer(engineOutput);
  const evidence = { engine, frames: [] };
  try {
    await setupAccount(page, runtime, { tracks: 12 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible' });

    for (const [width, height] of [[320, 780], [390, 844], [1284, 856]]) {
      await page.setViewportSize({ width, height });
      await page.waitForTimeout(350);
      const mobile = width <= 760;

      // The last focusable target of the one scroller.
      const target = await page.evaluate(() => {
        const scroller = document.querySelector('#main-scroll');
        const nodes = [...scroller.querySelectorAll('button, a, input, select, [role="button"], summary')]
          .filter(element => {
            const style = getComputedStyle(element);
            const rect = element.getBoundingClientRect();
            return style.display !== 'none' && style.visibility !== 'hidden'
              && rect.width > 0 && rect.height > 0 && !element.disabled;
          });
        const last = nodes[nodes.length - 1];
        if (!last) return null;
        last.dataset.dr02Last = 'true';
        const rect = last.getBoundingClientRect();
        return {
          label: (last.getAttribute('aria-label') || last.textContent || '').trim().slice(0, 40),
          height: +rect.height.toFixed(2), width: +rect.width.toFixed(2),
        };
      });
      expect(target, `${width} the list ends with a focusable target`).not.toBeNull();
      expect(target.height, `${width} the last target is a 44px target`).toBeGreaterThanOrEqual(43.5);

      await page.locator('[data-dr02-last="true"]').focus();
      await page.waitForTimeout(350);
      const measured = await page.evaluate(GEOMETRY);
      const focused = await page.evaluate(() => {
        const element = document.querySelector('[data-dr02-last="true"]');
        const rect = element.getBoundingClientRect();
        return { top: +rect.top.toFixed(2), bottom: +rect.bottom.toFixed(2), height: +rect.height.toFixed(2) };
      });
      const effectiveTop = measured.scroller.top + measured.topbar.height;
      const effectiveBottom = measured.scroller.bottom - measured.reserve;
      // The content has to clear the topmost floating band: the player on
      // the desktop and on the mobile composition alike.
      const bandTop = measured.player.top;
      expect.soft(effectiveTop, `${width} the effective top is the bottom of the topbar`)
        .toBeCloseTo(measured.topbar.bottom, 0);
      expect.soft(effectiveBottom, `${width} the effective bottom is the reserve above the bands`)
        .toBeCloseTo(bandTop, 0);
      expect.soft(focused.top, `${width} the focused target is below the topbar`)
        .toBeGreaterThanOrEqual(effectiveTop - 0.5);
      expect.soft(focused.bottom, `${width} the focused target clears the bands with 8px`)
        .toBeLessThanOrEqual(effectiveBottom - 8 + 0.5);
      expect.soft(focused.height, `${width} the focused target is whole`).toBeGreaterThanOrEqual(43.5);
      evidence.frames.push({ width, height, target, focused, effectiveTop, effectiveBottom });
    }
    await writeFile(join(engineOutput, 'focus.json'), JSON.stringify(evidence, null, 2));
  } finally {
    await runtime.close();
  }
});

test('DR02 reduced motion, forced colours and print keep the bands static and opaque', async ({ page }) => {
  test.setTimeout(240000);
  const engine = page.context().browser().browserType().name();
  const engineOutput = join(output, engine);
  await mkdir(engineOutput, { recursive: true });
  const runtime = await startServer(engineOutput);
  const evidence = { engine, modes: [] };
  try {
    await setupAccount(page, runtime, { tracks: 6 });
    await page.setViewportSize({ width: 1284, height: 856 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible' });

    /** The transition and animation state of the bands and their pseudo-elements. */
    const motion = () => page.evaluate(() => {
      const read = (element, pseudo) => {
        const style = getComputedStyle(element, pseudo);
        return {
          transition: style.transitionDuration, animation: style.animationDuration,
        };
      };
      const out = [];
      for (const selector of ['.player-bar', '.topbar']) {
        const element = document.querySelector(selector);
        if (!element) continue;
        out.push({ where: selector, ...read(element) });
        for (const pseudo of ['::before', '::after']) out.push({ where: selector + pseudo, ...read(element, pseudo) });
      }
      return out;
    });

    const durations = value => String(value).split(',')
      .map(part => parseFloat(part) || 0);

    // Reduced motion: nothing on the bands animates.
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.waitForTimeout(250);
    const reduced = await motion();
    for (const entry of reduced) {
      for (const value of durations(entry.transition)) {
        expect.soft(value, `${entry.where} has no transition in reduce`).toBe(0);
      }
      for (const value of durations(entry.animation)) {
        expect.soft(value, `${entry.where} has no animation in reduce`).toBe(0);
      }
    }
    evidence.modes.push({ mode: 'reduce', reduced });

    // The static glass survives the reduce: the band is still glass, it just
    // never moves.
    const stillGlass = await page.evaluate(() => {
      const player = getComputedStyle(document.querySelector('.player-bar'));
      return { background: player.backgroundColor, filter: player.backdropFilter || player.webkitBackdropFilter };
    });
    expect.soft(stillGlass.background, 'reduce keeps the static glass surface').not.toBe('rgba(0, 0, 0, 0)');
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    await page.waitForTimeout(250);

    // Forced colours: the bands go back to an opaque surface and lose the blur.
    await page.emulateMedia({ forcedColors: 'active' });
    await page.waitForTimeout(250);
    const forced = await page.evaluate(() => {
      const read = selector => {
        const style = getComputedStyle(document.querySelector(selector));
        return {
          background: style.backgroundColor,
          filter: style.backdropFilter || style.webkitBackdropFilter || 'none',
          shadow: style.boxShadow,
        };
      };
      return { player: read('.player-bar'), topbar: read('.topbar') };
    });
    for (const [name, band] of Object.entries(forced)) {
      const alpha = parseColour(band.background);
      expect.soft(alpha.a, `${name} is opaque in forced colours`).toBe(1);
      expect.soft(band.filter, `${name} has no blur in forced colours`).toBe('none');
    }
    evidence.modes.push({ mode: 'forced-colors', forced });
    await page.emulateMedia({ forcedColors: 'none' });
    await page.waitForTimeout(250);

    // Print: no blur and no shadow.
    await page.emulateMedia({ media: 'print' });
    await page.waitForTimeout(250);
    const printed = await page.evaluate(() => {
      const read = selector => {
        const style = getComputedStyle(document.querySelector(selector));
        return {
          filter: style.backdropFilter || style.webkitBackdropFilter || 'none',
          shadow: style.boxShadow,
        };
      };
      return { player: read('.player-bar'), topbar: read('.topbar') };
    });
    for (const [name, band] of Object.entries(printed)) {
      expect.soft(band.filter, `${name} has no blur in print`).toBe('none');
      expect.soft(band.shadow, `${name} has no shadow in print`).toBe('none');
    }
    evidence.modes.push({ mode: 'print', printed });
    await page.emulateMedia({ media: 'screen' });
    await page.waitForTimeout(250);

    await writeFile(join(engineOutput, 'modes.json'), JSON.stringify(evidence, null, 2));
  } finally {
    await runtime.close();
  }
});

test('DR02 scrolling, the theme and the overlay keep the source and the position', async ({ page }) => {
  test.setTimeout(240000);
  const engine = page.context().browser().browserType().name();
  const engineOutput = join(output, engine);
  await mkdir(engineOutput, { recursive: true });
  const runtime = await startServer(engineOutput);
  const evidence = { engine, steps: [] };
  try {
    await setupAccount(page, runtime, { tracks: 8, long: true });
    await page.setViewportSize({ width: 1284, height: 856 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible' });

    const audio = page.locator('#vela-audio');
    await page.getByRole('button', { name: /^Play /, exact: false }).first().click();
    await expect.poll(() => audio.evaluate(node => node.currentTime)).toBeGreaterThan(0.2);
    await page.evaluate(() => { document.querySelector('#vela-audio').dataset.dr02Identity = 'root-player'; });

    const read = () => page.evaluate(() => {
      const element = document.querySelector('#vela-audio');
      return {
        marker: element.dataset.dr02Identity,
        count: document.querySelectorAll('audio').length,
        src: element.getAttribute('src'), currentSrc: element.currentSrc,
        generation: element.getAttribute('data-source-generation'),
        time: element.currentTime, paused: element.paused,
      };
    });

    const before = await read();
    const steps = [];

    // Scroll: the catalog moves, the source does not.
    await page.locator('#main-scroll').evaluate(element => { element.scrollTop = 320; });
    await page.waitForTimeout(400);
    steps.push({ step: 'scroll', ...(await read()) });

    // The theme: the bands change their surface, the source does not.
    await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
    await page.waitForTimeout(400);
    steps.push({ step: 'theme', ...(await read()) });

    // The overlay: the queue opens and closes over the glass.
    await page.getByRole('button', { name: 'Show playback queue', exact: true }).first().click();
    await expect(page.getByRole('complementary', { name: 'Playback queue', exact: true })).toBeVisible();
    await page.waitForTimeout(400);
    steps.push({ step: 'queue-open', ...(await read()) });
    await page.getByRole('button', { name: 'Close playback queue', exact: true }).click();
    await page.waitForTimeout(400);
    steps.push({ step: 'queue-close', ...(await read()) });

    for (const step of steps) {
      expect.soft(step.marker, `${step.step} keeps the same media element`).toBe('root-player');
      expect.soft(step.count, `${step.step} keeps one audio element`).toBe(1);
      expect.soft(step.src, `${step.step} keeps the same source`).toBe(before.src);
      expect.soft(step.currentSrc, `${step.step} keeps the same resolved source`).toBe(before.currentSrc);
      expect.soft(step.generation, `${step.step} keeps the same source generation`).toBe(before.generation);
      expect.soft(step.time, `${step.step} never rewinds the position`).toBeGreaterThanOrEqual(before.time - 0.2);
    }
    evidence.steps = steps;

    // The queue and the sheet are still reachable over the glass.
    await page.locator('.now-playing-trigger').click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.keyboard.press('Escape');
    await page.waitForTimeout(250);
    evidence.steps.push({ step: 'now-playing', ...(await read()) });

    await writeFile(join(engineOutput, 'preservation.json'), JSON.stringify(evidence, null, 2));
  } finally {
    await runtime.close();
  }
});

test.describe('DR02 the frame budget', () => {
  test('DR02 the glass costs no more than five points of the frame budget', async ({ page }) => {
    test.setTimeout(300000);
    const engine = page.context().browser().browserType().name();
    const engineOutput = join(output, engine);
    await mkdir(engineOutput, { recursive: true });
    const runtime = await startServer(engineOutput);
    const evidence = { engine, runs: [] };
    try {
      await setupAccount(page, runtime, { tracks: 24 });
      await page.goto(runtime.origin);
      await settle(page);
      await navigate(page, 'Library');
      await page.locator('.track-row').first().waitFor({ state: 'visible' });

      // The budget is measured in a process of its own. The Playwright test
      // runner attaches to every browser it starts — its client, its trace
      // agent and its own machinery sit next to the renderer while a
      // measurement runs — and on this host that is worth 15 to 24 points of
      // frames over 33ms for the same page, the same browser, the same
      // driver and the same session. The product's own cost is the number a
      // reader's browser produces, so the measurement runs in a plain
      // process: `dr02-frame-budget.mjs`.
      const directory = await mkdtemp(join(tmpdir(), 'vela-dr02-'));
      const statePath = join(directory, 'state.json');
      await writeFile(statePath, JSON.stringify(await page.context().storageState()));
      const child = spawn(process.execPath, [
        resolve(root, 'tests/browser/dr02-frame-budget.mjs'),
        runtime.origin, statePath, engine,
      ], { stdio: ['ignore', 'pipe', 'inherit'] });
      let output = '';
      child.stdout.on('data', chunk => { output += chunk; });
      const code = await new Promise(done => child.on('exit', done));
      await rm(directory, { recursive: true, force: true });
      expect(code, 'the frame budget helper finished').toBe(0);
      const measured = JSON.parse(output.trim().split('\n').pop());

      evidence.clean = measured;
      evidence.runs = measured.runs;
      // The surface the measurement ran on: the two filters and the glass
      // under them. A budget number without them would be a number about
      // nothing.
      expect(measured.painted.player.filter, 'the player band blurs its backdrop')
        .toMatch(/blur\(/);
      expect(measured.painted.topbar.filter, 'the topbar band blurs its backdrop')
        .toMatch(/blur\(/);
      expect(measured.runs.length, 'the glass and the opaque fallback were both measured').toBe(2);

      const glass = measured.runs.find(run => run.mode === 'glass');
      const opaque = measured.runs.find(run => run.mode === 'opaque');
      expect(glass.frames, 'the glass run produced frames').toBeGreaterThan(100);
      expect(opaque.frames, 'the opaque run produced frames').toBeGreaterThan(100);
      expect(glass.range, 'both runs covered the same scroll range').toBe(opaque.range);

      const difference = +(glass.slowShare - opaque.slowShare).toFixed(2);
      evidence.best = { glass: glass.slowShare, opaque: opaque.slowShare, difference };
      evidence.environment = {
        engine,
        viewport: { width: 1284, height: 856 },
        display: 'headless, no display server',
        renderer: measured.renderer,
        userAgent: measured.userAgent,
        note: 'a local budget on this stand, not a promise about any other device',
      };
      await writeFile(join(engineOutput, 'performance.json'), JSON.stringify(evidence, null, 2));

      expect(
        difference,
        `the glass adds ${difference} points of frames over 33ms against the opaque fallback`,
      ).toBeLessThanOrEqual(5);
    } finally {
      await runtime.close();
    }
  });
});
