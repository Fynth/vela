/**
 * DR06 acceptance: the first phone screen shows the music.
 *
 * The package changes the density of the mobile composition, so the gate
 * measures the rendered rectangles instead of reading the stylesheet: the
 * first track's top and bottom, the table header's own height, the
 * transport's bands and targets, the filter summary's real visibility, the
 * empty editorial card's own height and the safe-area arithmetic are all read
 * from the live document at the widths the plan names.
 *
 * Nothing here is a demo: the account is a real one, the tracks are real
 * uploads and the catalog is the account's own.
 */
import { test, expect } from '@playwright/test';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { navigate, settle, setupWithTracks, startServer } from './dr04-density.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const output = resolve(root, process.env.VELA_DR06_EVIDENCE || 'target/checks/dr06-mobile');

/** A real 16-bit PCM wave, so the fixture track is a real file. */
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
 * The first-screen budget. Every number is a rendered rectangle: the first
 * track, its two inner rows, the table header, the filter summary and the
 * horizontal overflow of the document.
 */
const FIRST_SCREEN = async () => {
  // The self-hosted Manrope subset swaps in after the first paint, and the
  // text metrics differ from the fallback's. The measurement waits for the
  // real font, so the numbers describe the shipped composition.
  await document.fonts.load('600 12px Manrope');
  await document.fonts.ready;
  const rect = (element) => {
    const box = element.getBoundingClientRect();
    return {
      top: +box.top.toFixed(2), bottom: +box.bottom.toFixed(2),
      left: +box.left.toFixed(2), right: +box.right.toFixed(2),
      width: +box.width.toFixed(2), height: +box.height.toFixed(2),
    };
  };
  const pick = (selector) => {
    const element = document.querySelector(selector);
    if (!element) return null;
    const style = getComputedStyle(element);
    return {
      ...rect(element), display: style.display, text: (element.textContent || '').trim(),
      gridTemplateRows: style.gridTemplateRows, gridTemplateColumns: style.gridTemplateColumns,
    };
  };
  const row = document.querySelector('.track-row');
  const summary = document.querySelector('.filter-summary');
  const actions = document.querySelector('.track-row .track-actions');
  const idle = document.querySelector('.now-playing-idle');
  const idleTitle = idle ? idle.querySelector('.now-playing-meta strong') : null;
  const currentTitle = document.querySelector('.now-playing-meta strong.player-title');
  return {
    zoom: parseFloat(getComputedStyle(document.documentElement).zoom) || 1,
    viewport: { width: innerWidth, height: innerHeight },
    contentTop: getComputedStyle(document.documentElement).getPropertyValue('--vela-content-top').trim(),
    heading: pick('.page-heading'),
    toolbar: pick('.section-toolbar'),
    filterBlock: pick('.library-filters'),
    filterToolbar: pick('.filter-toolbar'),
    header: pick('.track-header'),
    firstRow: row ? rect(row) : null,
    rows: [...document.querySelectorAll('.track-row')].map((element) => rect(element)),
    playerTop: (() => {
      const player = document.querySelector('.player-bar');
      return player ? +player.getBoundingClientRect().top.toFixed(2) : null;
    })(),
    title: pick('.track-row .track-title'),
    duration: pick('.track-row .track-duration'),
    actions: actions ? rect(actions) : null,
    actionButtons: actions
      ? [...actions.querySelectorAll('button')].map((button) => ({
        ...rect(button), label: (button.getAttribute('aria-label') || '').trim(),
      }))
      : [],
    summary: summary ? {
      ...rect(summary),
      clipPath: getComputedStyle(summary).clipPath,
      position: getComputedStyle(summary).position,
      text: (summary.textContent || '').trim(),
      marker: summary.getAttribute('data-default-order'),
      role: summary.getAttribute('role'),
      scrollWidth: summary.scrollWidth,
      clientWidth: summary.clientWidth,
    } : null,
    idleTitle: idleTitle ? {
      ...rect(idleTitle),
      fontSize: getComputedStyle(idleTitle).fontSize,
      scrollWidth: idleTitle.scrollWidth,
      clientWidth: idleTitle.clientWidth,
      text: (idleTitle.textContent || '').trim(),
    } : null,
    currentTitle: currentTitle ? {
      ...rect(currentTitle),
      fontSize: getComputedStyle(currentTitle).fontSize,
      scrollWidth: currentTitle.scrollWidth,
      clientWidth: currentTitle.clientWidth,
      text: (currentTitle.textContent || '').trim(),
      full: currentTitle.getAttribute('title'),
    } : null,
    overflow: document.documentElement.scrollWidth - innerWidth,
  };
};

/** The transport bands and their real targets, at any composition. */
const TRANSPORT = async () => {
  await document.fonts.load('600 12px Manrope');
  await document.fonts.ready;
  const rect = (element) => {
    const box = element.getBoundingClientRect();
    return {
      top: +box.top.toFixed(2), bottom: +box.bottom.toFixed(2),
      left: +box.left.toFixed(2), right: +box.right.toFixed(2),
      width: +box.width.toFixed(2), height: +box.height.toFixed(2),
    };
  };
  const shown = (element) => {
    if (!element) return false;
    const style = getComputedStyle(element);
    const box = element.getBoundingClientRect();
    return style.display !== 'none' && style.visibility !== 'hidden' && box.width > 0 && box.height > 0;
  };
  const targets = {};
  for (const [name, selector] of [
    ['play', '#player-toggle'],
    ['next', '#player-next'],
    ['queue', 'button[aria-label^="Show playback queue"]'],
    ['seek', '.seek-control input[type=range]'],
  ]) {
    const element = document.querySelector(selector);
    targets[name] = shown(element) ? rect(element) : null;
  }
  const player = document.querySelector('.player-bar');
  const nav = document.querySelector('.mobile-nav');
  const scroller = document.querySelector('#main-scroll');
  // A probe resolves the token (and the calc/env inside it) to real pixels.
  const probe = document.createElement('div');
  probe.style.cssText = 'position:absolute;left:-9999px;top:0;height:var(--vela-bottom-reserve);width:10px';
  document.body.append(probe);
  const reserve = probe.getBoundingClientRect().height;
  probe.remove();
  return {
    zoom: parseFloat(getComputedStyle(document.documentElement).zoom) || 1,
    viewport: { width: innerWidth, height: innerHeight },
    overflow: document.documentElement.scrollWidth - innerWidth,
    player: player ? { ...rect(player), display: getComputedStyle(player).display } : null,
    nav: nav ? { ...rect(nav), display: getComputedStyle(nav).display } : null,
    navButtons: nav
      ? [...nav.querySelectorAll('button')].filter(shown).map((button) => ({
        ...rect(button), label: (button.getAttribute('aria-label') || button.textContent || '').trim(),
      }))
      : [],
    targets,
    reserve: +reserve.toFixed(2),
    scrollerBottom: scroller ? +scroller.getBoundingClientRect().bottom.toFixed(2) : null,
    safeBottom: getComputedStyle(document.documentElement).getPropertyValue('--vela-safe-bottom').trim(),
  };
};

/** The empty editorial card: its own height, its own action, no clipping. */
const EDITORIAL = async () => {
  await document.fonts.load('600 12px Manrope');
  await document.fonts.ready;
  const rect = (element) => {
    const box = element.getBoundingClientRect();
    return {
      top: +box.top.toFixed(2), bottom: +box.bottom.toFixed(2),
      left: +box.left.toFixed(2), right: +box.right.toFixed(2),
      width: +box.width.toFixed(2), height: +box.height.toFixed(2),
    };
  };
  const shown = (element) => {
    if (!element) return false;
    const style = getComputedStyle(element);
    const box = element.getBoundingClientRect();
    return style.display !== 'none' && box.width > 0 && box.height > 0;
  };
  const read = (card) => {
    if (!card) return null;
    const action = card.querySelector('.home-feature-open, .home-feature-listen, .home-wave-play, .home-wave-note button');
    const copy = card.querySelector('.home-feature-copy, .home-wave-copy');
    return {
      card: rect(card),
      copy: copy ? { ...rect(copy), scrollHeight: copy.scrollHeight, clientHeight: copy.clientHeight } : null,
      action: action ? { ...rect(action), label: (action.getAttribute('aria-label') || action.textContent || '').trim() } : null,
      actionButtons: [...card.querySelectorAll('button')].filter(shown).map((button) => ({
        ...rect(button), label: (button.getAttribute('aria-label') || button.textContent || '').trim(),
      })),
      text: (card.textContent || '').trim().slice(0, 120),
    };
  };
  return {
    zoom: parseFloat(getComputedStyle(document.documentElement).zoom) || 1,
    viewport: { width: innerWidth, height: innerHeight },
    emptyFeature: read(document.querySelector('.home-feature-empty')),
    feature: read(document.querySelector('.home-feature:not(.home-feature-empty)')),
    emptyWave: read(document.querySelector('.home-wave-empty')),
    wave: read(document.querySelector('.home-wave:not(.home-wave-empty)')),
  };
};

test('DR06 the first phone screen shows the first track and its own actions', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  const evidence = [];
  try {
    await setupWithTracks(page, runtime, 2);
    await page.goto(runtime.origin);
    await settle(page);
    for (const scene of [
      // 320x700: the first track and its own actions end above the 120px
      // mini-player, which starts at 510px.
      { width: 320, height: 700, firstTop: 390, rowBottom: 510, twoRows: false },
      // 390x844: the first track starts higher, and the plan's second
      // criterion is that two complete 113px rows fit above the player band,
      // which starts at 654px here. The 510px limit belongs to 320 only.
      { width: 390, height: 844, firstTop: 350, rowBottom: 654, twoRows: true },
    ]) {
      await page.setViewportSize({ width: scene.width, height: scene.height });
      await settle(page);
      await navigate(page, 'Library');
      await page.locator('.track-row').first().waitFor({ state: 'visible' });
      const measured = await page.evaluate(FIRST_SCREEN);
      evidence.push({ scene, measured });
      const where = `${scene.width}x${scene.height}`;

      expect(measured.contentTop, `${where} the mobile content top`).toBe('16px');
      expect(measured.header.height, `${where} the table header is one row`).toBeGreaterThanOrEqual(35);
      expect(measured.header.height, `${where} the table header is one row`).toBeLessThanOrEqual(37);
      // One grid row: the empty actions heading does not open a second one.
      expect(measured.header.gridTemplateRows.split(' ').length, `${where} the header is one grid row`)
        .toBe(1);
      expect(measured.firstRow.top, `${where} the first track starts inside the first screen`)
        .toBeLessThanOrEqual(scene.firstTop);
      expect(measured.firstRow.bottom, `${where} the first track and its actions end above the mini-player`)
        .toBeLessThanOrEqual(scene.rowBottom);
      if (scene.twoRows) {
        // The plan's own wording: two full rows fit before the player band.
        expect(measured.rows.length, `${where} the scene has a second row`).toBeGreaterThanOrEqual(2);
        expect(measured.rows[1].height, `${where} the second row is a full row`)
          .toBeGreaterThanOrEqual(112.5);
        expect(measured.rows[1].bottom, `${where} two full rows fit above the player band`)
          .toBeLessThanOrEqual(measured.playerTop);
      }
      // The row is the track and its own actions: two 44px rows, 8px apart.
      expect(measured.actions.height, `${where} the action row is a real target`).toBeGreaterThanOrEqual(44);
      expect(measured.actions.top, `${where} the actions are their own row`).toBeGreaterThanOrEqual(measured.title.bottom);
      expect(measured.duration.bottom, `${where} the time is not covered by the actions`)
        .toBeLessThanOrEqual(measured.actions.top);
      expect(measured.actions.right, `${where} the actions stay inside the row`)
        .toBeLessThanOrEqual(measured.firstRow.right + 1);
      for (const button of measured.actionButtons) {
        expect(button.width, `${where} ${button.label} width`).toBeGreaterThanOrEqual(43.5);
        expect(button.height, `${where} ${button.label} height`).toBeGreaterThanOrEqual(43.5);
        expect(button.right, `${where} ${button.label} is not clipped`).toBeLessThanOrEqual(measured.actions.right + 1);
      }
      expect(measured.overflow, `${where} no horizontal overflow`).toBeLessThanOrEqual(1);
      // The idle sentence of the mini-player reads in full at 320px: it is
      // never cut horizontally, and it never grows the 120px band.
      expect(measured.idleTitle, `${where} the idle mini-player has a title`).not.toBeNull();
      expect(measured.idleTitle.text, `${where} the idle title is the short DR-05 wording`).toBe('Nothing playing');
      expect(measured.idleTitle.fontSize, `${where} the idle title role`).toBe('12px');
      expect(measured.idleTitle.scrollWidth, `${where} the idle title is not truncated`)
        .toBeLessThanOrEqual(measured.idleTitle.clientWidth + 1);
      expect(measured.idleTitle.height, `${where} the idle title takes at most two lines`)
        .toBeLessThanOrEqual(37);
      await page.screenshot({ path: join(output, `dr06-library-${scene.width}x${scene.height}.png`), animations: 'disabled' });
    }
    await writeFile(join(output, 'dr06-first-screen.json'), JSON.stringify(evidence, null, 2));
  } finally { await runtime.close(); }
});

test('DR06 the width matrix keeps zero overflow and a real transport', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  const evidence = [];
  try {
    await setupWithTracks(page, runtime, 2);
    await page.goto(runtime.origin);
    await settle(page);
    for (const width of [320, 360, 390, 760, 761, 768]) {
      const height = width <= 760 ? 844 : 900;
      await page.setViewportSize({ width, height });
      await settle(page);
      await navigate(page, 'Library');
      await page.locator('.track-row').first().waitFor({ state: 'visible' });
      const measured = await page.evaluate(TRANSPORT);
      evidence.push({ width, height, measured });
      const mobile = width <= 760;

      expect(measured.overflow, `${width} no horizontal overflow`).toBeLessThanOrEqual(1);
      expect(measured.player.height, `${width} the player band`).toBeCloseTo(mobile ? 120 : 88, 0);
      expect(measured.targets.play, `${width} the play control is rendered`).not.toBeNull();
      expect(measured.targets.play.width, `${width} play width`).toBeGreaterThanOrEqual(43.5);
      expect(measured.targets.play.height, `${width} play height`).toBeGreaterThanOrEqual(43.5);
      expect(measured.targets.next, `${width} the next control is rendered`).not.toBeNull();
      expect(measured.targets.next.width, `${width} next width`).toBeGreaterThanOrEqual(43.5);
      expect(measured.targets.next.height, `${width} next height`).toBeGreaterThanOrEqual(43.5);
      expect(measured.targets.seek, `${width} the seek rail is rendered`).not.toBeNull();
      expect(measured.targets.seek.height, `${width} seek height`).toBeGreaterThanOrEqual(43.5);
      if (mobile) {
        // The mobile composition owns the queue control in the transport.
        expect(measured.targets.queue, `${width} the queue control is rendered`).not.toBeNull();
        expect(measured.targets.queue.width, `${width} queue width`).toBeGreaterThanOrEqual(43.5);
        expect(measured.targets.queue.height, `${width} queue height`).toBeGreaterThanOrEqual(43.5);
        expect(measured.nav, `${width} the mobile navigation is rendered`).not.toBeNull();
        expect(measured.nav.display, `${width} one navigation surface`).toBe('flex');
        expect(measured.nav.height, `${width} the navigation band`).toBeCloseTo(70, 0);
        expect(measured.nav.bottom, `${width} the navigation ends at the viewport bottom`).toBeCloseTo(height, 0);
        expect(measured.player.bottom, `${width} the mini-player sits above the navigation`)
          .toBeCloseTo(measured.nav.top, 0);
        expect(measured.navButtons.length, `${width} five equal destinations`).toBe(5);
        for (const button of measured.navButtons) {
          expect(button.width, `${width} ${button.label} width`).toBeGreaterThanOrEqual(43.5);
          expect(button.height, `${width} ${button.label} height`).toBeGreaterThanOrEqual(43.5);
        }
        expect(measured.reserve, `${width} the reserved bottom band`).toBeCloseTo(190, 0);
        // DR02: the one scroller runs under the two floating bands; the
        // effective bottom of the content is the top of the mini-player.
        expect(measured.scrollerBottom, `${width} the scroller reaches the viewport bottom`)
          .toBeCloseTo(height, 0);
        expect(measured.scrollerBottom - measured.reserve, `${width} the effective bottom is the player top`)
          .toBeCloseTo(measured.player.top, 0);
      } else {
        expect(measured.nav.display, `${width} no duplicate navigation surface`).toBe('none');
      }

      // The last focusable row — its actions are the last focusable control
      // of the list — scrolls fully above the bands with room to spare.
      await page.locator('#main-scroll').evaluate((element) => { element.scrollTop = element.scrollHeight; });
      const clearance = await page.evaluate(() => {
        const scroller = document.querySelector('#main-scroll').getBoundingClientRect();
        const player = document.querySelector('.player-bar').getBoundingClientRect();
        const rows = [...document.querySelectorAll('.track-row')];
        const last = rows[rows.length - 1].getBoundingClientRect();
        const actions = rows[rows.length - 1].querySelector('.track-actions').getBoundingClientRect();
        return {
          scrollerBottom: scroller.bottom, lastBottom: last.bottom, actionsBottom: actions.bottom,
          playerTop: player.top,
          gap: scroller.bottom - actions.bottom,
        };
      });
      // DR02: the scroller reaches the bottom of the viewport, so the last
      // action is measured against the top of the floating mini-player.
      expect(clearance.playerTop - clearance.actionsBottom, `${width} the last action clears the bands`)
        .toBeGreaterThanOrEqual(8);
      expect(clearance.gap, `${width} the last action stays inside the scroller`).toBeGreaterThanOrEqual(8);
      await page.locator('#main-scroll').evaluate((element) => { element.scrollTop = 0; });
    }
    await writeFile(join(output, 'dr06-transport.json'), JSON.stringify(evidence, null, 2));
  } finally { await runtime.close(); }
});

test('DR06 the default library order is announced but not shown, and an active filter is', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  const evidence = [];
  try {
    await setupWithTracks(page, runtime, 2);
    await page.goto(runtime.origin);
    await settle(page);
    for (const { width, height } of [{ width: 320, height: 700 }, { width: 390, height: 844 }]) {
      await page.setViewportSize({ width, height });
      await settle(page);
      await navigate(page, 'Library');
      await page.locator('.track-row').first().waitFor({ state: 'visible' });
      const measured = await page.evaluate(FIRST_SCREEN);
      evidence.push({ width, height, measured });
      const where = `${width}x${height}`;

      expect(measured.summary, `${where} the applied order is a status line`).not.toBeNull();
      expect(measured.summary.role, `${where} the summary is a status region`).toBe('status');
      expect(measured.summary.marker, `${where} the default order is marked`).toBe('true');
      expect(measured.summary.text, `${where} the default order sentence`).toBe('Library order');
      // Visually hidden with the existing screen-reader utility: the text
      // stays in the DOM, the box is one pixel and the clip is applied.
      expect(measured.summary.width, `${where} the default order leaves the flow`).toBeLessThanOrEqual(1);
      expect(measured.summary.height, `${where} the default order leaves the flow`).toBeLessThanOrEqual(1);
      expect(measured.summary.position, `${where} the default order is taken out of the flow`).toBe('absolute');
      expect(measured.summary.clipPath, `${where} the default order is clipped`).toContain('inset(50%');
      expect(measured.filterToolbar.height, `${where} the filter row is the real action only`)
        .toBeLessThanOrEqual(45);
    }

    // An active sort is a decision the reader made: it is shown, it may
    // wrap, and it is no longer the default order.
    await page.getByRole('button', { name: 'Filter and sort', exact: true }).click();
    await page.locator('#filter-sort').selectOption('newest');
    await page.getByRole('button', { name: 'Apply filters', exact: true }).click();
    await expect.poll(() => page.locator('.filter-summary').innerText()).toBe('Recently added');
    for (const { width, height } of [{ width: 320, height: 700 }, { width: 390, height: 844 }]) {
      await page.setViewportSize({ width, height });
      await settle(page);
      const active = await page.evaluate(FIRST_SCREEN);
      evidence.push({ width, height, active });
      const where = `${width}x${height}`;
      expect(active.summary.marker, `${where} an active sort is not the default order`).toBeNull();
      expect(active.summary.text, `${where} the active sort is named`).toBe('Recently added');
      expect(active.summary.width, `${where} the active sort is visible`).toBeGreaterThan(1);
      expect(active.summary.clipPath, `${where} the active sort is not clipped away`).not.toContain('inset(50%');
      expect(active.summary.scrollWidth, `${where} the active sort is not truncated`)
        .toBeLessThanOrEqual(active.summary.clientWidth + 1);
    }
    await writeFile(join(output, 'dr06-filter-summary.json'), JSON.stringify(evidence, null, 2));
  } finally { await runtime.close(); }
});

test('DR06 the empty editorial card keeps its own height and a real action', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  const evidence = [];
  try {
    await setupWithTracks(page, runtime, 2);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Home');
    await page.locator('.home-feature-empty').waitFor({ state: 'visible' });

    const at100 = await page.evaluate(EDITORIAL);
    evidence.push({ zoom: 1, at100 });
    expect(at100.emptyFeature, 'the account without an owned playlist states it honestly').not.toBeNull();
    expect(at100.emptyFeature.card.height, 'the empty editorial card is compact on a phone')
      .toBeLessThanOrEqual(240);
    expect(at100.emptyFeature.action, 'the empty card keeps its real action').not.toBeNull();
    expect(at100.emptyFeature.action.height, 'the action is a 44px target').toBeGreaterThanOrEqual(43.5);
    expect(at100.emptyFeature.action.top, 'the action is inside the card')
      .toBeGreaterThanOrEqual(at100.emptyFeature.card.top - 1);
    expect(at100.emptyFeature.action.bottom, 'the action is inside the card')
      .toBeLessThanOrEqual(at100.emptyFeature.card.bottom + 1);
    expect(at100.emptyFeature.action.left, 'the action is inside the card')
      .toBeGreaterThanOrEqual(at100.emptyFeature.card.left - 1);
    expect(at100.emptyFeature.action.right, 'the action is inside the card')
      .toBeLessThanOrEqual(at100.emptyFeature.card.right + 1);
    if (at100.emptyWave) {
      expect(at100.emptyWave.card.height, 'the empty wave keeps its own minimum height')
        .toBeGreaterThanOrEqual(160);
      expect(at100.emptyWave.copy.scrollHeight, 'the empty wave copy is not clipped')
        .toBeLessThanOrEqual(at100.emptyWave.copy.clientHeight + 1);
    }
    if (at100.wave) {
      // The image-backed wave keeps its own 160px; only the empty one grows.
      expect(at100.wave.card.height, 'the image-backed wave keeps its 160px')
        .toBeCloseTo(160, 0);
    }

    // A 150% app size is a real layout change: the card grows with its own
    // text instead of clipping it.
    await page.evaluate(() => document.documentElement.style.setProperty('--app-zoom', '1.5'));
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    const at150 = await page.evaluate(EDITORIAL);
    evidence.push({ zoom: 1.5, at150 });
    expect(at150.zoom, 'the app size is applied once').toBe(1.5);
    expect(at150.emptyFeature.card.height, 'the empty card keeps its own height at 150%')
      .toBeGreaterThanOrEqual(at100.emptyFeature.card.height);
    expect(at150.emptyFeature.copy.scrollHeight, 'the empty card copy is not clipped at 150%')
      .toBeLessThanOrEqual(at150.emptyFeature.copy.clientHeight + 1);
    expect(at150.emptyFeature.action.height / at150.zoom, 'the action is still a 44px target at 150%')
      .toBeGreaterThanOrEqual(43.5);
    expect(at150.emptyFeature.action.bottom, 'the action stays inside the card at 150%')
      .toBeLessThanOrEqual(at150.emptyFeature.card.bottom + 1);
    await page.screenshot({ path: join(output, 'dr06-home-empty-390-150.png'), animations: 'disabled' });
    await page.evaluate(() => document.documentElement.style.removeProperty('--app-zoom'));
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    await page.screenshot({ path: join(output, 'dr06-home-empty-390.png'), animations: 'disabled' });

    // The image-backed feature is the other half of the card: it keeps its
    // own 288px and its own composition on a phone.
    const me = await (await page.request.get(runtime.origin + '/api/me')).json();
    const created = await page.request.post(runtime.origin + '/api/playlists', {
      headers: { 'X-CSRF-Token': me.csrf_token, Origin: runtime.origin },
      data: { name: 'DR06 owned playlist' },
    });
    expect(created.ok(), 'the owned playlist is real').toBeTruthy();
    await page.reload();
    await settle(page);
    await navigate(page, 'Home');
    await page.locator('.home-feature:not(.home-feature-empty)').waitFor({ state: 'visible' });
    const owned = await page.evaluate(EDITORIAL);
    evidence.push({ owned });
    expect(owned.feature, 'the owned playlist gives the card its own story').not.toBeNull();
    expect(owned.feature.card.height, 'the image-backed feature keeps its 288px')
      .toBeCloseTo(288, 0);
    await page.screenshot({ path: join(output, 'dr06-home-feature-390.png'), animations: 'disabled' });
    await writeFile(join(output, 'dr06-editorial.json'), JSON.stringify(evidence, null, 2));
  } finally { await runtime.close(); }
});

test('DR06 the safe-area inset is counted once by the bands and the reserve', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  const evidence = [];
  try {
    await setupWithTracks(page, runtime, 2);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible' });

    const baseline = await page.evaluate(TRANSPORT);
    evidence.push({ baseline });
    expect(Number.parseFloat(baseline.safeBottom), 'the host has no cutout').toBe(0);
    expect(baseline.nav.height, 'the reference navigation band').toBeCloseTo(70, 0);
    expect(baseline.reserve, 'the reference reserve').toBeCloseTo(190, 0);

    // A hardware-style inset is emulated through the same variable the shell
    // consumes. This is simulated geometry, not a real device.
    await page.evaluate(async () => {
      document.documentElement.style.setProperty('--vela-safe-bottom', '34px');
      await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
    });
    const emulated = await page.evaluate(TRANSPORT);
    evidence.push({ emulated });
    expect(emulated.safeBottom, 'the inset is applied once').toBe('34px');
    expect(emulated.nav.height, 'the navigation row owns the bottom inset once').toBeCloseTo(70 + 34, 0);
    expect(emulated.nav.bottom, 'the navigation still ends at the viewport bottom').toBeCloseTo(844, 0);
    expect(emulated.player.bottom, 'the mini-player stays above the navigation band')
      .toBeCloseTo(emulated.nav.top, 0);
    // The two bands fill the viewport exactly: the 34px is inside the
    // navigation row, not added a second time below it.
    expect(emulated.player.bottom + emulated.nav.height, 'the bands fill the viewport once')
      .toBeCloseTo(844, 0);
    expect(emulated.reserve, 'the reserve keeps the inset exactly once').toBeCloseTo(120 + 70 + 34, 0);
    // DR02: the scroller reaches the bottom of the viewport and the bands
    // float over it; the reserve is the height of the bands it floats over.
    expect(emulated.scrollerBottom, 'the scroller reaches the viewport bottom')
      .toBeCloseTo(844, 0);
    expect(emulated.scrollerBottom - emulated.reserve, 'the effective bottom is the player top')
      .toBeCloseTo(emulated.player.top, 0);
    expect(emulated.overflow, 'the inset does not create an overflow').toBeLessThanOrEqual(1);

    const css = (await (await page.request.get(runtime.origin + '/style.css')).text()).replace(/\s+/g, ' ');
    expect(css, 'one reserve authority with one inset').toContain(
      '--vela-bottom-reserve:calc( var(--vela-player-height) + var(--vela-mobile-nav-height) + var(--vela-safe-bottom) )',
    );
    await page.screenshot({ path: join(output, 'dr06-safe-area-390.png'), animations: 'disabled' });

    // The top and the side insets belong to the topbar and the gutters; they
    // must not crop a control. The side inset is deliberately **wider than
    // the 20px gutter**: the shell's rule is `max(gutter, inset)`, so a 16px
    // inset against a 20px gutter would be a no-op and the assertion could
    // never fail. At 28px the inset is the winning term, and a control that
    // ignored it would be measured outside the expected edge.
    const sideInset = 28;
    await page.evaluate(async (inset) => {
      const root = document.documentElement;
      root.style.setProperty('--vela-safe-top', '24px');
      root.style.setProperty('--vela-safe-left', `${inset}px`);
      root.style.setProperty('--vela-safe-right', `${inset}px`);
      await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
    }, sideInset);
    const sides = await page.evaluate(() => {
      const boxes = {};
      for (const [name, selector] of [
        ['play', '#player-toggle'], ['next', '#player-next'],
        ['queue', 'button[aria-label^="Show playback queue"]'],
        ['nav', '.mobile-nav button'], ['topbar', '.topbar .button'],
        ['track', '.track-row .track-play'], ['actions', '.track-row .track-actions'],
      ]) {
        const element = document.querySelector(selector);
        if (!element) continue;
        const box = element.getBoundingClientRect();
        boxes[name] = { left: box.left, right: box.right, top: box.top, bottom: box.bottom };
      }
      const topbar = document.querySelector('.topbar').getBoundingClientRect();
      const scroller = document.querySelector('#main-scroll').getBoundingClientRect();
      const inner = document.querySelector('.content-inner').getBoundingClientRect();
      return {
        boxes, topbar: { height: topbar.height, bottom: topbar.bottom },
        scrollerTop: scroller.top, innerTop: inner.top,
      };
    });
    evidence.push({ sides });
    expect(sides.topbar.height, 'the topbar owns the top inset once').toBeCloseTo(63 + 24, 0);
    // DR02: the scroller runs under the topbar; the content is what starts
    // below it, and the top inset is counted exactly once.
    expect(sides.scrollerTop, 'the scroller runs under the topbar')
      .toBeLessThan(sides.topbar.bottom);
    expect(sides.innerTop, 'the content starts below the topbar inset')
      .toBeCloseTo(sides.topbar.bottom, 1);
    // The effective edge is the larger of the gutter and the inset; the
    // player and the navigation use their own smaller gutter (16px and 6px),
    // so the bar's own content is the only thing checked against the inset.
    const gutter = Math.max(20, sideInset);
    for (const [name, box] of Object.entries(sides.boxes)) {
      expect(box.left, `${name} is not cropped by the left inset`).toBeGreaterThanOrEqual(gutter - 0.5);
      expect(box.right, `${name} is not cropped by the right inset`).toBeLessThanOrEqual(390 - gutter + 0.5);
    }
    await page.screenshot({ path: join(output, 'dr06-safe-area-sides-390.png'), animations: 'disabled' });
    await writeFile(join(output, 'dr06-safe-area.json'), JSON.stringify(evidence, null, 2));
  } finally { await runtime.close(); }
});

test('DR06 the composition renders at the reference widths in both themes', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  const evidence = [];
  try {
    await setupWithTracks(page, runtime, 2);
    await page.goto(runtime.origin);
    await settle(page);
    for (const viewport of [
      { width: 320, height: 700 }, { width: 360, height: 800 }, { width: 390, height: 844 },
      { width: 1284, height: 856 }, { width: 1440, height: 900 }, { width: 1728, height: 1000 },
    ]) {
      await page.setViewportSize(viewport);
      await settle(page);
      await navigate(page, 'Library');
      await page.locator('.track-row').first().waitFor({ state: 'visible' });
      for (const theme of ['dark', 'light']) {
        const current = await page.locator('html').getAttribute('data-theme');
        if (current !== theme) {
          await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
          await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
        }
        await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
        const measured = await page.evaluate(TRANSPORT);
        // The pre-list heights the design system cites are measured here, in
        // the same scene as the screenshots, so the number has an artifact.
        const preList = await page.evaluate(() => {
          const box = (selector) => {
            const element = document.querySelector(selector);
            if (!element) return null;
            const rect = element.getBoundingClientRect();
            const style = getComputedStyle(element);
            return {
              height: +rect.height.toFixed(2), top: +rect.top.toFixed(2),
              fontSize: style.fontSize, fontWeight: style.fontWeight, gap: style.gap,
            };
          };
          return {
            pageHeading: box('.page-heading'), toolbar: box('.library-section .section-toolbar'),
            toolbarHeading: box('.library-section .section-toolbar h2'),
            filters: box('.library-filters'), header: box('.track-header'),
            row: box('.track-row'),
          };
        });
        evidence.push({ viewport, theme, measured, preList });
        const mobile = viewport.width <= 760;
        expect(measured.overflow, `${viewport.width} ${theme} no horizontal overflow`).toBeLessThanOrEqual(1);
        expect(measured.player.height, `${viewport.width} ${theme} the player band`)
          .toBeCloseTo(mobile ? 120 : 88, 0);
        if (mobile) {
          // The page heading is two rows at 320 and one from 360, and the
          // library toolbar is 100px because the search field owns a row.
          expect(preList.pageHeading.height, `${viewport.width} ${theme} the page heading`)
            .toBeCloseTo(viewport.width <= 320 ? 83.91 : 44, 0);
          expect(preList.toolbar.height, `${viewport.width} ${theme} the library toolbar`).toBeCloseTo(100, 0);
          expect(preList.header.height, `${viewport.width} ${theme} the table header`).toBeCloseTo(36, 0);
          expect(preList.row.height, `${viewport.width} ${theme} the track row`).toBeCloseTo(113, 0);
          expect(measured.nav.height, `${viewport.width} ${theme} the navigation band`).toBeCloseTo(70, 0);
          expect(measured.navButtons.length, `${viewport.width} ${theme} five destinations`).toBe(5);
        } else {
          expect(measured.nav.display, `${viewport.width} ${theme} no navigation band`).toBe('none');
        }
        await page.screenshot({
          path: join(output, `dr06-library-${viewport.width}x${viewport.height}-${theme}.png`),
          animations: 'disabled',
        });
      }
    }
    await writeFile(join(output, 'dr06-reference-widths.json'), JSON.stringify(evidence, null, 2));
  } finally { await runtime.close(); }
});

test('DR06 the empty wave keeps its own height and its own action', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  try {
    await setupWithTracks(page, runtime, 2);
    // A failed recommendations request is a real state of the wave: there is
    // no candidate, so the card states that honestly instead of showing an
    // empty picture, and it has to keep its own text.
    await page.route('**/api/recommendations*', async route => { await route.abort('connectionreset'); });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Home');
    await page.locator('.home-wave-empty').waitFor({ state: 'visible' });
    const measured = await page.evaluate(EDITORIAL);
    expect(measured.emptyWave, 'the wave without a candidate states it honestly').not.toBeNull();
    expect(measured.emptyWave.card.height, 'the empty wave keeps its own minimum height')
      .toBeGreaterThanOrEqual(160);
    expect(measured.emptyWave.copy.scrollHeight, 'the empty wave copy is not clipped')
      .toBeLessThanOrEqual(measured.emptyWave.copy.clientHeight + 1);
    for (const button of measured.emptyWave.actionButtons) {
      expect(button.height, `${button.label} is a real target`).toBeGreaterThanOrEqual(43.5);
      expect(button.bottom, `${button.label} is inside the card`)
        .toBeLessThanOrEqual(measured.emptyWave.card.bottom + 1);
    }
    await page.screenshot({ path: join(output, 'dr06-home-wave-empty-390.png'), animations: 'disabled' });
  } finally { await runtime.close(); }
});

test('DR06 a long current title is ellipsized and keeps its full name', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  try {
    const long = 'A deliberately very long track title that has to be shortened in the mini player';
    await setupWithTracks(page, runtime, 0);
    const me = await (await page.request.get(runtime.origin + '/api/me')).json();
    const upload = await page.request.post(runtime.origin + '/api/upload', {
      headers: { 'X-CSRF-Token': me.csrf_token, Origin: runtime.origin },
      multipart: { file: { name: long + '.wav', mimeType: 'audio/wav', buffer: pcmWave(3, 260) } },
    });
    expect(upload.ok(), 'the long track is a real upload').toBeTruthy();
    const track = (await upload.json()).track;
    await page.setViewportSize({ width: 320, height: 700 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await page.getByRole('button', { name: `Play ${track.title}`, exact: true }).click();
    await expect.poll(() => page.locator('#vela-audio').evaluate((audio) => audio.currentTime)).toBeGreaterThan(0);
    const measured = await page.evaluate(FIRST_SCREEN);
    expect(measured.currentTitle, 'the current track has a title in the mini-player').not.toBeNull();
    expect(measured.currentTitle.full, 'the full name stays available').toBe(track.title);
    expect(measured.currentTitle.text, 'the visible text is the full name in the DOM').toBe(track.title);
    expect(measured.currentTitle.clientWidth, 'the long title is shortened, not pushed out')
      .toBeLessThan(measured.currentTitle.scrollWidth);
    expect(measured.currentTitle.height, 'the long title stays on one line').toBeLessThanOrEqual(20);
    expect(measured.overflow, 'the long title does not create an overflow').toBeLessThanOrEqual(1);
    await writeFile(join(output, 'dr06-long-title.json'), JSON.stringify({ track: track.title, measured }, null, 2));
    await page.screenshot({ path: join(output, 'dr06-long-title-320.png'), animations: 'disabled' });
  } finally { await runtime.close(); }
});

test('DR06 the composition survives a viewport change and a page change', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  try {
    const track = await setupWithTracks(page, runtime, 3);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(3);
    await page.getByRole('button', { name: `Play ${track.title}`, exact: true }).click();
    await expect.poll(() => page.locator('#vela-audio').evaluate((audio) => audio.currentTime)).toBeGreaterThan(0);

    const readState = () => page.evaluate(() => {
      const audio = document.querySelector('#vela-audio');
      return {
        src: audio.getAttribute('src'),
        generation: audio.getAttribute('data-source-generation'),
        currentTime: Number(audio.currentTime.toFixed(3)),
        paused: audio.paused,
        audioElements: document.querySelectorAll('audio').length,
      };
    });
    const readQueue = async () => {
      await page.getByRole('button', { name: 'Show playback queue and controls', exact: true }).click();
      const drawer = page.locator('#queue-drawer-dialog');
      await expect(drawer).toBeVisible();
      const state = await drawer.evaluate((node) => ({
        count: node.querySelector('.queue-count')?.textContent?.trim() ?? null,
        rows: [...node.querySelectorAll('.queue-row .queue-track strong')].map((row) => row.textContent.trim()),
        currentRows: [...node.querySelectorAll('.queue-row.current .queue-track strong')].map((row) => row.textContent.trim()),
      }));
      await drawer.getByRole('button', { name: 'Close playback queue', exact: true }).click();
      await expect(drawer).toHaveCount(0);
      return state;
    };

    const before = await readState();
    const queueBefore = await readQueue();
    // The density change is a layout change: the viewport moves, the page
    // moves, and the source, the queue, the cursor and the position stay.
    for (const viewport of [{ width: 320, height: 700 }, { width: 390, height: 844 }, { width: 360, height: 800 }]) {
      await page.setViewportSize(viewport);
      await settle(page);
    }
    await navigate(page, 'Home');
    await page.locator('.home-header').waitFor({ state: 'visible' });
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(3);
    const after = await readState();
    const queueAfter = await readQueue();

    expect(after.audioElements, 'one root media element').toBe(1);
    expect(after.src, 'the same source').toBe(before.src);
    expect(after.generation, 'the same source generation').toBe(before.generation);
    expect(after.paused, 'the navigation did not pause the playback').toBe(before.paused);
    expect(after.currentTime, 'the position was not reset').toBeGreaterThan(0);
    expect(queueAfter.count, 'the same queue').toBe(queueBefore.count);
    expect(queueAfter.rows, 'the same queue rows').toEqual(queueBefore.rows);
    expect(queueAfter.currentRows, 'the same current occurrence').toEqual(queueBefore.currentRows);
    expect(queueAfter.currentRows.length, 'the queue still has a current occurrence').toBe(1);
  } finally { await runtime.close(); }
});

test('DR06 the phone composition holds at 80% app size and under reduced motion', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  const evidence = [];
  try {
    await setupWithTracks(page, runtime, 2);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(2);

    // The application's own app size is the real path to 80%: four presses of
    // the documented control, in Settings. On the phone band the "More" tab
    // opens Settings directly, so the helper's sheet path does not apply; the
    // real tab is clicked instead.
    const openSettings = async () => {
      await page.getByRole('navigation', { name: 'Mobile navigation', exact: true })
        .getByRole('button', { name: 'More destinations', exact: true }).click();
      await page.getByRole('heading', { name: 'Settings', exact: true }).waitFor();
    };
    await openSettings();
    for (let at = 0; at < 4; at += 1) {
      await page.getByRole('button', { name: 'Decrease app size', exact: true }).click();
    }
    await page.waitForTimeout(250);
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(2);
    const at80 = await page.evaluate(FIRST_SCREEN);
    evidence.push({ scene: 'app-size-80', measured: at80 });
    // The app size is a CSS `zoom` on the document root, so a reported
    // rectangle is already in the zoomed space: a 44 CSS px target measures
    // 44 * 0.8 rendered px here. The target is therefore read back in CSS px
    // by dividing once — multiplying would count the zoom twice.
    expect(at80.zoom, 'the app size control really moved').toBeCloseTo(0.8, 2);
    expect(at80.overflow, '80% app size does not create an overflow').toBeLessThanOrEqual(1);
    expect(at80.actionButtons.length, 'the row still exposes its real actions').toBeGreaterThan(0);
    for (const button of at80.actionButtons) {
      expect(button.width / at80.zoom, `${button.label} stays a 44px target at 80%`)
        .toBeGreaterThanOrEqual(43.5);
      expect(button.height / at80.zoom, `${button.label} stays 44px tall at 80%`)
        .toBeGreaterThanOrEqual(43.5);
    }
    // Back to 100% for the reduced-motion scene, so the numbers stay readable.
    await openSettings();
    await page.getByRole('button', { name: 'Reset to 100%', exact: true }).click();
    await page.waitForTimeout(250);

    // Reduced motion: the mobile composition must not animate anything. The
    // shell's own rule already forces `animation: none` and `transition: none`;
    // this asserts it on the surfaces this package touched.
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(2);
    const motion = await page.evaluate(() => {
      const read = (selector) => {
        const element = document.querySelector(selector);
        if (!element) return null;
        const style = getComputedStyle(element);
        return {
          animationName: style.animationName, animationDuration: style.animationDuration,
          transitionDuration: style.transitionDuration, transitionProperty: style.transitionProperty,
        };
      };
      return {
        row: read('.track-row'), header: read('.track-header'),
        toolbar: read('.library-section .section-toolbar'),
        play: read('#player-toggle'), player: read('.player-bar'),
        media: matchMedia('(prefers-reduced-motion: reduce)').matches,
      };
    });
    evidence.push({ scene: 'reduced-motion', motion });
    expect(motion.media, 'the reduced-motion media query is active').toBe(true);
    for (const [name, value] of Object.entries(motion)) {
      if (name === 'media' || value === null) continue;
      expect(value.animationName, `${name} runs no keyframes under reduced motion`).toBe('none');
      expect(parseFloat(value.animationDuration), `${name} has no animation duration`).toBe(0);
      expect(parseFloat(value.transitionDuration), `${name} has no transition duration`).toBe(0);
    }
    // The composition is still the measured one, not a frozen broken layout.
    const reduced = await page.evaluate(FIRST_SCREEN);
    evidence.push({ scene: 'reduced-motion-layout', measured: reduced });
    expect(reduced.header.height, 'reduced motion keeps the 36px header').toBeCloseTo(36, 0);
    expect(reduced.firstRow.height, 'reduced motion keeps the 113px row').toBeCloseTo(113, 0);
    expect(reduced.overflow, 'reduced motion keeps zero overflow').toBeLessThanOrEqual(1);
    await writeFile(join(output, 'dr06-app-size-and-motion.json'), JSON.stringify(evidence, null, 2));
  } finally { await runtime.close(); }
});
