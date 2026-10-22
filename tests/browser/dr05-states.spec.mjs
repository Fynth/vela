import { test, expect } from '@playwright/test';
import { mkdir, mkdtemp, open, readFile, rm, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { randomUUID } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';

const root = fileURLToPath(new URL('../../', import.meta.url));

// DR05: the empty editorial and wave cards must be readable in both themes,
// and the root's recovery status, Undo receipt and ordinary receipt must share
// one column that never overlaps itself, the player or the mobile navigation.
// This journey owns its server, data and sessions.
async function isolatedServer(output) {
  const directory = await mkdtemp(join(tmpdir(), 'vela-dr05-'));
  const listener = createServer();
  await new Promise(resolve => listener.listen(0, '127.0.0.1', resolve));
  const port = listener.address().port;
  await new Promise(resolve => listener.close(resolve));
  const log = await open(join(output, 'server.log'), 'w', 0o600);
  const server = spawn(resolve(root, process.env.VELA_TEST_BINARY || 'target/debug/vela-server'), [], {
    cwd: root, stdio: ['ignore', log.fd, log.fd],
    env: { ...process.env, VELA_BIND: '127.0.0.1:' + port, VELA_DATA_DIR: join(directory, 'data'),
      VELA_WEB_DIR: resolve(root, process.env.VELA_TEST_WEB_DIR || 'dist'), VELA_SCAN_ROOTS: '[]' },
  });
  const exited = new Promise(resolve => { server.once('exit', resolve); server.once('error', resolve); });
  return {
    origin: 'http://127.0.0.1:' + port,
    async close() {
      if (server.exitCode === null && server.signalCode === null) {
        server.kill('SIGTERM');
        await Promise.race([exited, delay(5000)]);
        if (server.exitCode === null && server.signalCode === null) { server.kill('SIGKILL'); await exited; }
      }
      await log.close();
      await rm(directory, { recursive: true, force: true });
    },
  };
}

async function navigate(page, name) {
  const mobile = page.getByRole('navigation', { name: 'Mobile navigation', exact: true });
  if (await mobile.isVisible()) {
    const tab = mobile.getByRole('button', { name, exact: true });
    if (await tab.count()) await tab.click();
    else {
      await mobile.getByRole('button', { name: 'More destinations', exact: true }).click();
      await page.getByRole('navigation', { name: 'More from Vela', exact: true })
        .getByRole('button', { name, exact: true }).click();
    }
  } else await page.getByRole('button', { name, exact: true }).first().click();
}

async function setTheme(page, theme) {
  if (await page.locator('html').getAttribute('data-theme') !== theme) {
    await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
  }
  await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

// The live DOM's own contrast: an opaque ancestor is the background, the
// text's own computed colour is the foreground.
const CARD_CONTRAST = () => {
  const parse = value => { const parts = value.match(/[\d.]+/g); return parts ? { r: +parts[0], g: +parts[1], b: +parts[2], a: parts[3] === undefined ? 1 : +parts[3] } : null; };
  const luminance = colour => [colour.r, colour.g, colour.b]
    .map(value => value / 255)
    .map(value => value <= .04045 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4)
    .reduce((total, value, at) => total + value * [.2126, .7152, .0722][at], 0);
  const background = element => {
    for (let parent = element; parent; parent = parent.parentElement) {
      const colour = parse(getComputedStyle(parent).backgroundColor);
      if (colour && colour.a === 1) return colour;
    }
    return parse(getComputedStyle(document.body).backgroundColor);
  };
  const readings = [];
  for (const selector of ['.home-feature-empty .home-feature-title', '.home-feature-empty .home-eyebrow',
    '.home-feature-empty .home-feature-subtitle', '.home-wave-empty .home-wave-title',
    '.home-wave-empty .home-eyebrow', '.home-wave-empty .home-wave-subtitle',
    '.home-wave-empty .home-wave-note']) {
    for (const element of document.querySelectorAll(selector)) {
      if (!element.checkVisibility()) continue;
      const style = getComputedStyle(element);
      const foreground = parse(style.color);
      const back = background(element);
      const a = luminance(foreground), b = luminance(back);
      readings.push({
        selector, text: element.textContent.trim().slice(0, 48),
        ratio: (Math.max(a, b) + .05) / (Math.min(a, b) + .05),
        size: parseFloat(style.fontSize), title: element.tagName === 'H2',
      });
    }
  }
  return readings;
};

// The real rectangles of the root's three feedback signals.
const STACK_RECTS = () => {
  const rectOf = element => { const r = element.getBoundingClientRect(); return { top: r.top, bottom: r.bottom, left: r.left, right: r.right }; };
  const overlap = (a, b) => {
    const w = Math.min(a.right, b.right) - Math.max(a.left, b.left);
    const h = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
    return w > 0 && h > 0 ? w * h : 0;
  };
  const stack = document.querySelector('.feedback-stack');
  if (!stack) return { present: false };
  const children = [...stack.children].map(child => ({ className: String(child.className), rect: rectOf(child) }));
  const recovery = children.find(child => child.className.includes('playback-status'));
  const undo = children.find(child => child.className.includes('playlist-undo'));
  const ordinary = children.find(child => child.className.includes('alert')
    && !child.className.includes('playback-status') && !child.className.includes('playlist-undo'));
  const player = document.querySelector('.player-bar');
  const navigation = document.querySelector('.mobile-nav');
  const stackRect = rectOf(stack);
  return {
    present: true, children, stackRect,
    recoveryVsUndo: recovery && undo ? { overlap: overlap(recovery.rect, undo.rect), gap: undo.rect.top - recovery.rect.bottom } : null,
    undoVsOrdinary: undo && ordinary ? { overlap: overlap(undo.rect, ordinary.rect), gap: ordinary.rect.top - undo.rect.bottom } : null,
    recoveryVsOrdinary: recovery && ordinary ? { overlap: overlap(recovery.rect, ordinary.rect), gap: ordinary.rect.top - recovery.rect.bottom } : null,
    overlapWithPlayer: player ? overlap(stackRect, rectOf(player)) : 0,
    overlapWithNavigation: navigation ? overlap(stackRect, rectOf(navigation)) : 0,
    documentOverflow: document.documentElement.scrollWidth - innerWidth,
    buttons: [...stack.querySelectorAll('button')].map(button => {
      const r = button.getBoundingClientRect();
      return { label: (button.getAttribute('aria-label') || button.textContent).trim().slice(0, 40), width: r.width, height: r.height };
    }),
  };
};

// One permanent row of the root's feedback stack, as the browser sees it:
// the node, the live region it carries, the text of that region and the
// space the row takes. A row with no message is in the DOM, is empty, leaves
// the flow and takes no space: the announcement is a change of the text of a
// region that was already there and never the insertion of the region
// together with its content.
const REGION = (selector) => {
  const row = document.querySelector(selector);
  const region = row ? row.querySelector('[role="status"]') : null;
  const style = row ? getComputedStyle(row) : null;
  const stack = document.querySelector('.feedback-stack');
  return {
    present: !!row,
    region: !!region,
    line: region ? region.textContent.trim() : null,
    collapsed: row ? row.classList.contains('is-empty') : null,
    position: style ? style.position : null,
    height: row ? row.getBoundingClientRect().height : null,
    controls: row ? row.querySelectorAll('button').length : 0,
    stackHeight: stack ? stack.getBoundingClientRect().height : null,
  };
};

// The Undo failure is its own permanent alert region inside the Undo row:
// the node is in the DOM before the failure, the failure is a change of its
// text, and the empty region takes no space.
const ALERT = () => {
  const node = document.querySelector('.feedback-stack .playlist-undo p[role="alert"]');
  const style = node ? getComputedStyle(node) : null;
  return {
    present: !!node,
    line: node ? node.textContent.trim() : null,
    collapsed: node ? node.classList.contains('is-empty') : null,
    position: style ? style.position : null,
    height: node ? node.getBoundingClientRect().height : null,
  };
};

test('DR05 empty cards are readable and the root feedback never overlaps itself', async ({ page }) => {
  test.setTimeout(240000);
  const output = process.env.VELA_DR05_EVIDENCE || 'test-results/dr05-states';
  await mkdir(output, { recursive: true });
  const runtime = await isolatedServer(output);
  try {
    await expect.poll(async () => {
      try { return (await page.request.get(runtime.origin + '/health', { timeout: 1000 })).ok(); }
      catch { return false; }
    }).toBeTruthy();
    const setup = await page.request.post(runtime.origin + '/api/setup', {
      headers: { Origin: runtime.origin }, data: { username: 'reader', password: 'private dr05 fixture password' },
    });
    expect(setup.ok()).toBeTruthy();
    const me = await setup.json();
    const headers = { 'X-CSRF-Token': me.csrf_token, Origin: runtime.origin };
    // Two real tracks and no playlist: the empty editorial card is real.
    for (const [name, mime] of [['signal.flac', 'audio/flac'], ['beacon.mp3', 'audio/mpeg']]) {
      const upload = await page.request.post(runtime.origin + '/api/upload', {
        headers, multipart: { file: { name, mimeType: mime, buffer: await readFile(resolve(root, 'crates/media/tests/fixtures/' + name)) } },
      });
      expect(upload.ok(), name).toBeTruthy();
    }
    const playlists = await (await page.request.get(runtime.origin + '/api/playlists')).json();
    expect(playlists).toHaveLength(0);

    await page.goto(runtime.origin);
    await expect(page.locator('.auth-card, .app-shell')).toBeVisible();
    if (await page.locator('.app-shell').count() === 0) {
      await page.locator('#username').fill('reader');
      await page.locator('#password').fill('private dr05 fixture password');
      await page.getByRole('button', { name: /sign in|create your music space/i }).first().click();
    }
    await expect(page.locator('.app-shell')).toBeVisible();

    // --- Scene A: the empty editorial card, measured in both themes.
    for (const theme of ['light', 'dark']) {
      await setTheme(page, theme);
      await navigate(page, 'Home');
      await expect(page.locator('.home-feature-empty')).toHaveCount(1);
      const readings = await page.evaluate(CARD_CONTRAST);
      const editorial = readings.filter(reading => reading.selector.includes('home-feature-empty'));
      expect(editorial.length, theme + ' editorial readings').toBeGreaterThan(0);
      for (const reading of editorial) {
        expect.soft(reading.ratio, `${theme} ${reading.selector} contrast`).toBeGreaterThanOrEqual(4.5);
      }
      await page.screenshot({ path: `${output}/home-empty-editorial-1284x856-${theme}.png`, animations: 'disabled' });
    }

    // --- Scene B: the empty wave card. Favoriting both tracks makes them the
    // recommendation anchors, so the settled wave has no candidate left.
    const tracks = await (await page.request.get(runtime.origin + '/api/tracks?sort=newest&limit=50')).json();
    for (const track of tracks.items) {
      const favorite = await page.request.post(runtime.origin + '/api/tracks/' + track.id + '/favorite', { headers, data: { favorite: true } });
      expect(favorite.ok(), track.title).toBeTruthy();
    }
    // The wave reads its own snapshot on mount, so the page is reloaded to
    // make it read the confirmed favorite set.
    await page.reload();
    await expect(page.locator('.app-shell')).toBeVisible();
    for (const theme of ['light', 'dark']) {
      await setTheme(page, theme);
      await navigate(page, 'Home');
      await expect(page.locator('.home-wave-empty')).toHaveCount(1);
      const readings = await page.evaluate(CARD_CONTRAST);
      const wave = readings.filter(reading => reading.selector.includes('home-wave-empty'));
      expect(wave.length, theme + ' wave readings').toBeGreaterThan(0);
      for (const reading of wave) {
        expect.soft(reading.ratio, `${theme} ${reading.selector} contrast`).toBeGreaterThanOrEqual(4.5);
      }
      await page.screenshot({ path: `${output}/home-empty-wave-1284x856-${theme}.png`, animations: 'disabled' });
    }

    // --- Scene C: one real playlist, a real recovery status and a real Undo
    // receipt share the root's feedback stack.
    const created = await page.request.post(runtime.origin + '/api/playlists', { headers, data: { name: 'Evening signals' } });
    expect(created.ok()).toBeTruthy();
    const playlist = await created.json();
    const added = await page.request.post(runtime.origin + '/api/playlists/' + playlist.id + '/items', {
      headers, data: { revision: playlist.revision, mutation_id: randomUUID(), track_id: tracks.items[0].id },
    });
    expect(added.ok()).toBeTruthy();

    await page.route('**/api/tracks/*/stream', async route => { await route.abort('connectionreset'); });
    await navigate(page, 'Library');
    await expect(page.locator('.track-row').first()).toBeVisible();
    // The real Undo receipt: add the other track to the real playlist.
    await page.locator('[id^="track-menu-trigger-"]').last().click();
    await page.getByRole('menuitem', { name: 'Add to playlist…', exact: true }).click();
    const picker = page.locator('#playlist-picker');
    await expect(picker).toBeVisible();
    await picker.getByRole('option').first().click();
    await picker.locator('#playlist-picker-add').click();
    await expect(page.locator('.feedback-stack .playlist-undo')).toBeVisible();
    // The real ordinary receipt: add a track to the playback queue.
    await page.getByRole('button', { name: /^Add .* to playback queue$/ }).first().click();
    await expect(page.locator('.feedback-stack > .alert.success:not(.playlist-undo)')).toBeVisible();
    // The real recovery status: the transport fails and the root's own bounded
    // retry budget runs out.
    await page.locator('.track-play').first().click();
    await expect(page.locator('.feedback-stack .playback-status')).toBeVisible({ timeout: 30000 });

    for (const [width, height] of [[1284, 856], [390, 844], [360, 800], [320, 700]]) {
      await page.setViewportSize({ width, height });
      await page.waitForTimeout(600);
      const state = await page.evaluate(STACK_RECTS);
      expect(state.present, width + 'px feedback stack').toBe(true);
      expect(state.children.length, width + 'px feedback signals').toBe(3);
      expect(state.recoveryVsUndo, width + 'px recovery vs undo').not.toBeNull();
      expect(state.recoveryVsUndo.overlap, width + 'px recovery/undo intersection').toBe(0);
      expect(state.recoveryVsUndo.gap, width + 'px recovery/undo gap').toBeGreaterThanOrEqual(8);
      expect(state.undoVsOrdinary, width + 'px undo vs receipt').not.toBeNull();
      expect(state.undoVsOrdinary.overlap, width + 'px undo/receipt intersection').toBe(0);
      expect(state.undoVsOrdinary.gap, width + 'px undo/receipt gap').toBeGreaterThanOrEqual(8);
      expect(state.recoveryVsOrdinary.overlap, width + 'px recovery/receipt intersection').toBe(0);
      expect(state.overlapWithPlayer, width + 'px stack/player intersection').toBe(0);
      expect(state.overlapWithNavigation, width + 'px stack/navigation intersection').toBe(0);
      expect(state.documentOverflow, width + 'px document overflow').toBeLessThanOrEqual(3);
      for (const button of state.buttons) {
        expect.soft(button.height, `${width}px "${button.label}" height`).toBeGreaterThanOrEqual(44);
      }
      await page.screenshot({ path: `${output}/feedback-stack-${width}x${height}.png`, animations: 'disabled' });
    }
    await writeFile(`${output}/stack-state.json`, JSON.stringify(await page.evaluate(STACK_RECTS), null, 2));
  } finally { await runtime.close(); }
});

test('DR05 the recovery status and the Undo receipt are permanent live regions', async ({ page }) => {
  test.setTimeout(240000);
  const output = process.env.VELA_DR05_EVIDENCE || 'test-results/dr05-states';
  await mkdir(output, { recursive: true });
  const runtime = await isolatedServer(output);
  try {
    await expect.poll(async () => {
      try { return (await page.request.get(runtime.origin + '/health', { timeout: 1000 })).ok(); }
      catch { return false; }
    }).toBeTruthy();
    const setup = await page.request.post(runtime.origin + '/api/setup', {
      headers: { Origin: runtime.origin }, data: { username: 'regions', password: 'private dr05 regions fixture password' },
    });
    expect(setup.ok()).toBeTruthy();
    const me = await setup.json();
    const headers = { 'X-CSRF-Token': me.csrf_token, Origin: runtime.origin };
    for (const [name, mime] of [['signal.flac', 'audio/flac'], ['beacon.mp3', 'audio/mpeg']]) {
      const upload = await page.request.post(runtime.origin + '/api/upload', {
        headers, multipart: { file: { name, mimeType: mime, buffer: await readFile(resolve(root, 'crates/media/tests/fixtures/' + name)) } },
      });
      expect(upload.ok(), name).toBeTruthy();
    }
    const created = await page.request.post(runtime.origin + '/api/playlists', { headers, data: { name: 'Region signals' } });
    expect(created.ok()).toBeTruthy();

    await page.goto(runtime.origin);
    await expect(page.locator('.auth-card, .app-shell')).toBeVisible();
    if (await page.locator('.app-shell').count() === 0) {
      await page.locator('#username').fill('regions');
      await page.locator('#password').fill('private dr05 regions fixture password');
      await page.getByRole('button', { name: /sign in|create your music space/i }).first().click();
    }
    await expect(page.locator('.app-shell')).toBeVisible();
    await navigate(page, 'Library');
    await expect(page.locator('.track-row').first()).toBeVisible();

    // --- Before any message both rows are in the DOM, empty and collapsed.
    //     A live region that arrives with its content is not announced by
    //     most screen readers, so the node has to be there first.
    const recoveryBefore = await page.evaluate(REGION, '.feedback-stack .playback-status');
    expect(recoveryBefore.present, 'the recovery row is a permanent node').toBe(true);
    expect(recoveryBefore.region, 'the recovery row carries a live region').toBe(true);
    expect(recoveryBefore.line, 'the recovery region starts empty').toBe('');
    expect(recoveryBefore.collapsed, 'the empty recovery row is collapsed').toBe(true);
    expect(recoveryBefore.position, 'the empty recovery row leaves the flow').toBe('absolute');
    expect(recoveryBefore.height, 'the empty recovery row takes no space').toBe(0);
    expect(recoveryBefore.controls, 'an empty recovery row owns no control').toBe(0);
    expect(recoveryBefore.stackHeight, 'the empty stack takes no space').toBe(0);

    const undoBefore = await page.evaluate(REGION, '.feedback-stack .playlist-undo');
    expect(undoBefore.present, 'the Undo row is a permanent node').toBe(true);
    expect(undoBefore.region, 'the Undo row carries a live region').toBe(true);
    expect(undoBefore.line, 'the Undo region starts empty').toBe('');
    expect(undoBefore.collapsed, 'the empty Undo row is collapsed').toBe(true);
    expect(undoBefore.position, 'the empty Undo row leaves the flow').toBe('absolute');
    expect(undoBefore.height, 'the empty Undo row takes no space').toBe(0);
    expect(undoBefore.controls, 'an empty Undo row owns no control').toBe(0);

    // The regions are marked, so the messages below prove that the text
    // arrives in the same node and that the node is not re-created for it.
    await page.evaluate(() => {
      for (const selector of ['.feedback-stack .playback-status [role="status"]', '.feedback-stack .playlist-undo [role="status"]']) {
        const node = document.querySelector(selector);
        if (node) node.dataset.probe = 'permanent';
      }
    });

    // --- The real Undo receipt: a real track added to a real playlist.
    await page.locator('[id^="track-menu-trigger-"]').last().click();
    await page.getByRole('menuitem', { name: 'Add to playlist…', exact: true }).click();
    const picker = page.locator('#playlist-picker');
    await expect(picker).toBeVisible();
    await picker.getByRole('option').first().click();
    await picker.locator('#playlist-picker-add').click();
    await expect(page.locator('.feedback-stack .playlist-undo')).toBeVisible();
    const undoAfter = await page.evaluate(REGION, '.feedback-stack .playlist-undo');
    expect(undoAfter.line, 'the Undo message is in the region').toContain('Added “');
    expect(undoAfter.collapsed, 'the Undo row is not empty any more').toBe(false);
    expect(undoAfter.position, 'the Undo row is back in the flow').toBe('static');
    expect(undoAfter.height, 'the Undo row takes a line of the stack').toBeGreaterThan(0);
    expect(undoAfter.controls, 'the Undo row owns its two controls').toBe(2);
    expect(undoAfter.stackHeight, 'the stack grew with the Undo receipt').toBeGreaterThan(0);
    expect(await page.evaluate(() => document.querySelector('.feedback-stack .playlist-undo [role="status"]')?.dataset.probe),
      'the Undo live region was not re-created for the message').toBe('permanent');
    await page.screenshot({ path: `${output}/feedback-stack-undo-receipt-1284x856.png`, animations: 'disabled' });

    // --- The real Undo failure: the DELETE never reaches the server, so the
    //     failure has to arrive in the permanent alert region of the row.
    const failureBefore = await page.evaluate(ALERT);
    expect(failureBefore.present, 'the Undo failure region is a permanent node').toBe(true);
    expect(failureBefore.line, 'the Undo failure region starts empty').toBe('');
    expect(failureBefore.collapsed, 'the empty Undo failure region is collapsed').toBe(true);
    expect(failureBefore.position, 'the empty Undo failure region leaves the flow').toBe('absolute');
    expect(failureBefore.height, 'the empty Undo failure region takes no space').toBe(0);
    await page.evaluate(() => {
      const node = document.querySelector('.feedback-stack .playlist-undo p[role="alert"]');
      if (node) node.dataset.probe = 'permanent';
    });
    await page.route('**/api/playlists/*/items/*', async route => { await route.abort('connectionreset'); });
    await page.locator('.feedback-stack .playlist-undo').getByRole('button', { name: 'Undo', exact: true }).click();
    await expect.poll(() => page.evaluate(ALERT).then(state => state.line), { timeout: 30000 }).toContain('Connection interrupted');
    const failureAfter = await page.evaluate(ALERT);
    expect(failureAfter.collapsed, 'the Undo failure region is not empty any more').toBe(false);
    expect(failureAfter.position, 'the Undo failure region is back in the flow').toBe('static');
    expect(failureAfter.height, 'the Undo failure region takes a line of the row').toBeGreaterThan(0);
    expect(await page.evaluate(() => document.querySelector('.feedback-stack .playlist-undo p[role="alert"]')?.dataset.probe),
      'the Undo failure region was not re-created for the failure').toBe('permanent');
    await page.screenshot({ path: `${output}/feedback-stack-undo-failure-1284x856.png`, animations: 'disabled' });
    await page.unroute('**/api/playlists/*/items/*');

    // --- The dismissal empties the region and the row takes no space again:
    //     the stack is exactly as tall as it was before the message, and the
    //     failure leaves the row with it.
    await page.locator('.feedback-stack button[aria-label="Dismiss added-track receipt"]').click();
    await expect.poll(() => page.evaluate(REGION, '.feedback-stack .playlist-undo')).toMatchObject({ line: '', collapsed: true, height: 0 });
    const dismissed = await page.evaluate(REGION, '.feedback-stack .playlist-undo');
    expect(dismissed.stackHeight, 'the empty stack is as tall as it was before the message').toBeCloseTo(0, 1);
    expect(await page.evaluate(() => document.querySelector('.feedback-stack .playlist-undo [role="status"]')?.dataset.probe),
      'the Undo live region survived the dismissal').toBe('permanent');
    const failureDismissed = await page.evaluate(ALERT);
    expect(failureDismissed.line, 'the dismissed receipt takes its failure with it').toBe('');
    expect(failureDismissed.collapsed, 'the Undo failure region is collapsed again').toBe(true);
    expect(failureDismissed.height, 'the empty Undo failure region takes no space again').toBe(0);

    // --- The real recovery status: the transport fails for real.
    await page.route('**/api/tracks/*/stream', async route => { await route.abort('connectionreset'); });
    await page.locator('.track-play').first().click();
    await expect.poll(() => page.evaluate(REGION, '.feedback-stack .playback-status').then(state => state.line), { timeout: 30000 })
      .toContain('Connection interrupted');
    const recoveryAfter = await page.evaluate(REGION, '.feedback-stack .playback-status');
    expect(recoveryAfter.collapsed, 'the recovery row is not empty any more').toBe(false);
    expect(recoveryAfter.position, 'the recovery row is back in the flow').toBe('static');
    expect(recoveryAfter.height, 'the recovery row takes a line of the stack').toBeGreaterThan(0);
    expect(recoveryAfter.controls, 'the recovery row owns its retry control').toBe(1);
    expect(recoveryAfter.stackHeight, 'the stack grew with the recovery status').toBeGreaterThan(0);
    expect(await page.evaluate(() => document.querySelector('.feedback-stack .playback-status [role="status"]')?.dataset.probe),
      'the recovery live region was not re-created for the message').toBe('permanent');
    await page.screenshot({ path: `${output}/feedback-stack-recovery-1284x856.png`, animations: 'disabled' });

    await writeFile(`${output}/stack-regions.json`, JSON.stringify({
      recoveryBefore, undoBefore, undoAfter, failureBefore, failureAfter, dismissed, failureDismissed, recoveryAfter,
    }, null, 2));
  } finally { await runtime.close(); }
});
