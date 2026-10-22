/**
 * DR04 acceptance: a dense desktop composition with real 44px targets.
 *
 * The package replaces a decorative density change with measured geometry, so
 * the gate measures the rendered rectangles instead of reading the stylesheet:
 * every visible interactive target, the transport band, the navigation step
 * and the section heading are read from the live document.
 */
import { test, expect } from '@playwright/test';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { measure, navigate, pointerSeek, settle, setupWithTracks, startServer, transportActions } from './dr04-density.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const output = resolve(root, process.env.VELA_DENSITY_EVIDENCE || 'target/checks/dr04-density');

test('DR04 desktop composition: 44px targets, a tiled transport band and a 20px section heading', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  try {
    await setupWithTracks(page, runtime, 3);
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(3);
    await page.setViewportSize({ width: 1284, height: 856 });

    // --- The idle library: the same target surface, with the transport still
    //     disabled because nothing is playing.
    const idle = await measure(page);
    const idleSmall = idle.targets.filter((target) => target.width < 44 || target.height < 44);
    expect(idleSmall.map((target) => `${target.selector} ${target.width}x${target.height}`)).toEqual([]);

    // --- A real current track: the same controls are enabled and measured
    //     again, including the favorite, which does not exist while idle.
    const title = await page.locator('.track-row .track-title strong').first().innerText();
    await page.getByRole('button', { name: `Play ${title}`, exact: true }).click();
    await expect.poll(() => page.locator('#vela-audio').evaluate((audio) => audio.currentTime)).toBeGreaterThan(0);
    const current = await measure(page);
    const currentSmall = current.targets.filter((target) => target.width < 44 || target.height < 44);
    expect(currentSmall.map((target) => `${target.selector} ${target.width}x${target.height}`)).toEqual([]);

    // --- The transport band: 88px tall, tiled into a 44px control row and a
    //     44px seek row, with no intersection between any two targets.
    expect(current.player.barHeight).toBeGreaterThanOrEqual(87);
    expect(current.player.barHeight).toBeLessThanOrEqual(89);
    expect(current.player.buttons.height).toBe(44);
    expect(current.player.seekRow.height).toBe(44);
    expect(current.player.seekInput.height).toBe(44);
    expect(current.player.center.y).toBeCloseTo(current.player.bar.y, 1);
    expect(current.player.center.bottom).toBeCloseTo(current.player.bar.bottom, 1);
    expect(current.player.buttons.bottom).toBeCloseTo(current.player.seekRow.y, 1);
    expect(current.transportOverlapsWithSeek).toEqual([]);

    // The hairline replaces the physical border and takes no vertical space.
    expect(current.player.borderTopWidth).toBe('0px');
    expect(current.player.hairline).toBe('1px');

    // --- The compact glyphs and the 32px Play face survive the bigger target.
    const glyphs = current.player.glyphs.map((glyph) => glyph.width);
    expect(glyphs).toContain(15);
    expect(glyphs).toContain(17);
    expect(current.player.playFaceSize).toBe(32);
    expect(current.player.volume.width).toBe(65);
    expect(current.player.volume.height).toBe(44);

    // --- Five 44px controls and four 4px gaps are 236px of transport.
    expect(current.player.transportExtent.count).toBe(5);
    expect(current.player.transportExtent.gap).toBe(4);
    expect(current.player.transportExtent.to - current.player.transportExtent.from).toBe(236);

    // --- The navigation step is the 44px item itself; the 24px group gap is
    //     the only spacing between the groups.
    expect(current.sidebar.navCount).toBe(12);
    expect(current.sidebar.step).toBe(44);
    expect(current.sidebar.groupGaps.filter((gap) => gap === 24).length).toBeGreaterThanOrEqual(2);
    const navHeight = current.sidebar.navRects[0].height;
    expect(navHeight).toBe(44);
    expect(current.sidebar.navRects[0].height * 12).toBe(528);

    // --- The section heading is 20px/600 and the page heading stays fluid.
    const heading = current.textLeaves.find((leaf) => leaf.selector.startsWith('h2'));
    expect(heading.fontSize).toBe('20px');
    expect(heading.weight).toBe('600');
    const pageHeading = current.textLeaves.find((leaf) => leaf.selector.startsWith('h1'));
    expect(parseFloat(pageHeading.fontSize)).toBeGreaterThanOrEqual(28);
    expect(pageHeading.weight).toBe('650');

    // --- A real focus action reaches every transport control and keeps a
    //     visible focus ring.
    const focused = await transportActions(page);
    for (const control of focused) {
      if (control.skipped) continue;
      expect(control.focused, `${control.label} takes focus`).toBe(true);
      expect(control.focusVisible, `${control.label} shows a focus-visible ring`).toBe(true);
      expect(parseFloat(control.outlineWidth), `${control.label} focus ring`).toBeGreaterThan(0);
    }

    // --- A real pointer action on the seek rail moves the position.
    const seek = await pointerSeek(page, 0.5);
    expect(seek, 'the seek range has a real hit box').not.toBeNull();
    expect(seek.box.height).toBe(44);
    expect(Math.abs(Number(seek.after) - seek.max / 2), 'the click landed on the rail').toBeLessThanOrEqual(seek.max * 0.08);

    // --- The geometry change never touches the source, the queue, the cursor
    //     or the position. The queue is read from the drawer's own
    //     projection: its count, the identity of its rows and the row that
    //     carries the current occurrence.
    const readState = () => page.evaluate(() => {
      const audio = document.querySelector('#vela-audio');
      return {
        src: audio.getAttribute('src'),
        generation: audio.getAttribute('data-source-generation'),
        currentTime: Number(audio.currentTime.toFixed(3)),
        paused: audio.paused,
      };
    });
    const readQueue = async () => {
      await page.getByRole('button', { name: /^Show playback queue$/ }).click();
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
    for (const destination of ['Home', 'Search', 'Settings']) await navigate(page, destination);
    await navigate(page, 'Library');
    const after = await readState();
    const queueAfter = await readQueue();
    expect(after.src).toBe(before.src);
    expect(after.generation).toBe(before.generation);
    expect(after.paused, 'the navigation did not pause the playback').toBe(before.paused);
    expect(await page.locator('#vela-audio').count()).toBe(1);
    // The queue and the cursor survive the navigation: the same rows, the
    // same count, and the same row is still the current occurrence.
    expect(queueAfter.count).toBe(queueBefore.count);
    expect(queueAfter.rows).toEqual(queueBefore.rows);
    expect(queueAfter.currentRows).toEqual(queueBefore.currentRows);
    expect(queueAfter.currentRows.length, 'the queue still has a current occurrence').toBe(1);
    // The position is not reset by the navigation: the audio element is the
    // same one and it kept playing.
    expect(after.currentTime, 'the position was not reset by the navigation').toBeGreaterThan(0);

    await writeFile(join(output, 'dr04-density-desktop.json'), JSON.stringify({ idle, current }, null, 2));
  } finally {
    await runtime.close();
  }
});

test('DR04 responsive matrix: app size 80/100/150 keeps zero overflow and zero overlapping controls', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  const viewports = [
    { width: 768, height: 1024 },
    { width: 1284, height: 856 },
    { width: 1440, height: 900 },
    { width: 1728, height: 1000 },
  ];
  try {
    await setupWithTracks(page, runtime, 3);
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(3);
    const title = await page.locator('.track-row .track-title strong').first().innerText();
    await page.getByRole('button', { name: `Play ${title}`, exact: true }).click();
    await expect.poll(() => page.locator('#vela-audio').evaluate((audio) => audio.currentTime)).toBeGreaterThan(0);

    // The app's own size control is the real path to 80/100/150 percent.
    const setAppSize = async (steps) => {
      await navigate(page, 'Settings');
      const label = steps < 0 ? 'Decrease app size' : 'Increase app size';
      for (let at = 0; at < Math.abs(steps); at += 1) {
        await page.getByRole('button', { name: label, exact: true }).click();
      }
      if (steps === 0) await page.getByRole('button', { name: 'Reset to 100%', exact: true }).click();
      await page.waitForTimeout(200);
    };

    const evidence = [];
    // The app size moves in fixed 5-point steps, so 100 → 150 is ten presses
    // of `Increase app size` and 100 → 80 is four presses of the other one.
    for (const [steps, percent] of [[-4, 80], [0, 100], [10, 150]]) {
      await setAppSize(steps);
      await navigate(page, 'Library');
      for (const viewport of viewports) {
        await page.setViewportSize(viewport);
        await page.waitForTimeout(250);
        const measured = await measure(page);
        evidence.push({ percent, viewport, measured });
        // The app size is a CSS zoom on the document root, so the reported
        // rectangles are in the zoomed space; the band is read back in CSS
        // pixels before it is compared with the 88px reference.
        const band = measured.player.barHeight / (measured.zoom || 1);
        // The size control must really have applied the requested size: a
        // broken switch would otherwise leave the whole matrix green while
        // measuring the wrong layout.
        expect.soft(measured.zoom, `${percent}% ${viewport.width} the app size was applied`).toBeCloseTo(percent / 100, 2);
        expect.soft(measured.overflow.document, `${percent}% ${viewport.width} no horizontal overflow`).toBeLessThanOrEqual(1);
        expect.soft(measured.overflow.body, `${percent}% ${viewport.width} no body overflow`).toBeLessThanOrEqual(1);
        expect.soft(measured.transportOverlapsWithSeek, `${percent}% ${viewport.width} no overlapping controls`).toEqual([]);
        expect.soft(band, `${percent}% ${viewport.width} player band`).toBeGreaterThanOrEqual(87);
        expect.soft(band, `${percent}% ${viewport.width} player band`).toBeLessThanOrEqual(89);
      }
    }
    await writeFile(join(output, 'dr04-density-matrix.json'), JSON.stringify(evidence, null, 2));
  } finally {
    await runtime.close();
  }
});

test('DR04 browser zoom 80/100/150 keeps zero overflow and zero overlapping controls', async ({ browser }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  const windows = [
    { width: 768, height: 1024 },
    { width: 1284, height: 856 },
    { width: 1440, height: 900 },
    { width: 1728, height: 1000 },
  ];
  const evidence = [];
  let sessionCookies = [];
  try {
    for (const zoom of [0.8, 1, 1.5]) {
      // A browser zoom keeps the device window and shrinks the CSS viewport,
      // so the media queries and the layout agree again.
      const context = await browser.newContext({
        viewport: { width: 1284, height: 856 },
        deviceScaleFactor: zoom,
      });
      const page = await context.newPage();
      if (sessionCookies.length) await context.addCookies(sessionCookies);
      await setupWithTracks(page, runtime, 2);
      sessionCookies = await context.cookies();
      await page.goto(runtime.origin);
      await settle(page);
      await navigate(page, 'Library');
      await expect(page.locator('.track-row')).toHaveCount(2);
      const title = await page.locator('.track-row .track-title strong').first().innerText();
      await page.getByRole('button', { name: `Play ${title}`, exact: true }).click();
      await expect.poll(() => page.locator('#vela-audio').evaluate((audio) => audio.currentTime)).toBeGreaterThan(0);
      for (const window of windows) {
        const cssWidth = Math.round(window.width / zoom);
        await page.setViewportSize({
          width: cssWidth,
          height: Math.round(window.height / zoom),
        });
        await page.waitForTimeout(250);
        const measured = await measure(page);
        evidence.push({ zoom, window, cssWidth, measured });
        expect.soft(measured.overflow.document, `${zoom} ${window.width} no horizontal overflow`).toBeLessThanOrEqual(1);
        expect.soft(measured.overflow.body, `${zoom} ${window.width} no body overflow`).toBeLessThanOrEqual(1);
        expect.soft(measured.transportOverlapsWithSeek, `${zoom} ${window.width} no overlapping controls`).toEqual([]);
        const band = measured.player.barHeight / (measured.zoom || 1);
        // A layout narrower than 760px is the mobile composition with its own
        // 120px band; the desktop band is 88px.
        const expected = cssWidth <= 760 ? 120 : 88;
        expect.soft(band, `${zoom} ${window.width} player band`).toBeGreaterThanOrEqual(expected - 1);
        expect.soft(band, `${zoom} ${window.width} player band`).toBeLessThanOrEqual(expected + 1);
      }
      await context.close();
    }
    await writeFile(join(output, 'dr04-density-browser-zoom.json'), JSON.stringify(evidence, null, 2));
  } finally {
    await runtime.close();
  }
});

test('DR04 mobile regression: 320 and 390 keep their 44px transport and navigation', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  try {
    await setupWithTracks(page, runtime, 2);
    await page.goto(runtime.origin);
    await settle(page);
    for (const [width, height] of [[320, 700], [390, 844]]) {
      await page.setViewportSize({ width, height });
      await navigate(page, 'Library');
      await expect(page.locator('.track-row')).toHaveCount(2);
      const measured = await measure(page);
      const small = measured.targets.filter((target) => target.width < 44 || target.height < 44);
      expect(small.map((target) => `${target.selector} ${target.width}x${target.height}`)).toEqual([]);
      expect(measured.player.barHeight).toBe(120);
      expect(measured.transportOverlapsWithSeek).toEqual([]);
      expect(measured.overflow.document).toBeLessThanOrEqual(1);
    }
  } finally {
    await runtime.close();
  }
});

test('DR04 the size map: the cover radius and the volume are the documented ones', async ({ page }) => {
  test.setTimeout(240000);
  await mkdir(output, { recursive: true });
  const runtime = await startServer(output);
  // The same facts the size map in DESIGN_SYSTEM.md claims: the base cover
  // radius and the three states of the volume.
  const coverRadii = () => page.evaluate(() => {
    const radius = (selector) => {
      const element = document.querySelector(selector);
      return element ? getComputedStyle(element).borderRadius : null;
    };
    return { row: radius('.track-row .artwork'), player: radius('.now-playing .artwork') };
  });
  const volumeBox = () => page.evaluate(() => {
    const input = document.querySelector('#player-volume');
    if (!input) return null;
    const rect = input.getBoundingClientRect();
    return { width: +rect.width.toFixed(2), height: +rect.height.toFixed(2), display: getComputedStyle(input).display };
  });
  try {
    await setupWithTracks(page, runtime, 2);
    await page.goto(runtime.origin);
    await settle(page);
    await page.setViewportSize({ width: 1284, height: 856 });
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(2);
    const title = await page.locator('.track-row .track-title strong').first().innerText();
    await page.getByRole('button', { name: `Play ${title}`, exact: true }).click();
    await expect.poll(() => page.locator('#vela-audio').evaluate((audio) => audio.currentTime)).toBeGreaterThan(0);

    // The base cover radius is the 6px role.
    const covers = await coverRadii();
    expect(covers.row, 'the library cover is the 6px role').toBe('6px');
    expect(covers.player, "the player bar's cover is the 6px role").toBe('6px');

    // The volume is 65×44 above 1100px of viewport width.
    const wide = await volumeBox();
    expect(wide.width, 'the volume is 65px wide on the desktop composition').toBe(65);
    expect(wide.height, 'the volume is 44px tall').toBe(44);

    // It is 44×44 from 1100px down to the mobile composition.
    await page.setViewportSize({ width: 1000, height: 856 });
    await page.waitForTimeout(300);
    const narrow = await volumeBox();
    expect(narrow.width, 'the volume is 44px wide below 1100px').toBe(44);
    expect(narrow.height, 'the volume stays 44px tall').toBe(44);

    // Below 600px of layout width the volume yields first. The app's own
    // size control is the real path to 150 percent: 900px of window at
    // 150% is 600px of layout. The page is reloaded after the size change,
    // so the container query is evaluated on a settled layout: Firefox
    // re-evaluates it one layout late when a zoomed root is resized.
    await page.setViewportSize({ width: 900, height: 856 });
    await navigate(page, 'Settings');
    for (let at = 0; at < 10; at += 1) {
      await page.getByRole('button', { name: 'Increase app size', exact: true }).click();
    }
    await page.waitForTimeout(300);
    await page.reload();
    await page.waitForTimeout(700);
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(2);
    const yielded = await page.evaluate(() => {
      const input = document.querySelector('#player-volume');
      const shell = document.querySelector('.app-shell');
      return {
        display: input ? getComputedStyle(input).display : null,
        layoutWidth: shell ? shell.offsetWidth : null,
        queue: !!document.querySelector('.player-right button[aria-label="Show playback queue"]')?.getClientRects().length,
        mute: !!document.querySelector('.player-mute')?.getClientRects().length,
      };
    });
    expect(yielded.layoutWidth, 'the layout is 600px wide at 150% in a 900px window').toBe(600);
    expect(yielded.display, 'the volume yields below 600px of layout width').toBe('none');
    expect(yielded.queue, 'the queue stays').toBe(true);
    expect(yielded.mute, 'the mute stays').toBe(true);

    // The mobile composition: the player bar's cover is 5px and the utility
    // zone is not in the player bar at all. The app size goes back to 100%
    // first, through the same control.
    await navigate(page, 'Settings');
    await page.getByRole('button', { name: 'Reset to 100%', exact: true }).click();
    await page.waitForTimeout(300);
    await page.reload();
    await page.waitForTimeout(700);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.waitForTimeout(400);
    const mobile = await page.evaluate(() => {
      const cover = document.querySelector('.now-playing .artwork');
      const volume = document.querySelector('#player-volume');
      return {
        cover: cover ? getComputedStyle(cover).borderRadius : null,
        volumeWidth: volume ? +volume.getBoundingClientRect().width.toFixed(2) : null,
      };
    });
    expect(mobile.cover, "the mobile player bar's cover is 5px").toBe('5px');
    expect(mobile.volumeWidth, 'the volume is not in the mobile player bar').toBe(0);
    await writeFile(join(output, 'dr04-density-size-map.json'), JSON.stringify({ covers, wide, narrow, yielded, mobile }, null, 2));
  } finally {
    await runtime.close();
  }
});
