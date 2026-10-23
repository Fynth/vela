/**
 * DR-03: the short answer that never delays the command.
 *
 * The package is one user-facing source: a command answers in the nearest
 * frame and the effect is a separate, short moment on top of it. This spec
 * measures that on the real application, in a real browser, against a real
 * account with real uploaded originals:
 *
 *   1. the durations and the properties of every changed selector,
 *   2. the time from the event to the first painted frame,
 *   3. the command, the focus and the audio do not wait for the effect,
 *   4. the artwork crossfade keeps two layers, one request and one audio,
 *   5. reduced motion leaves the same surfaces without a movement,
 *   6. a queue move changes the order at once, settles the moved row and
 *      hands the focus to the same control of the destination row.
 *
 * The spec owns its server, its data and its sessions. The two covers are
 * real: the tracked FLAC fixture with its embedded picture and a second
 * real FLAC whose picture block carries the tracked PNG, so two tracks
 * really have two different artworks.
 */
import { test, expect } from '@playwright/test';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { settle, navigate, startServer } from './dr04-density.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const evidenceDir = engine => resolve(root, process.env.VELA_DR03_EVIDENCE || `target/checks/dr03-motion/${engine}`);

/** The tracked fixture with an embedded cover. */
const brightCover = resolve(root, 'crates/media/tests/fixtures/bright-cover.flac');
/** The tracked PNG, used as the picture of the second real FLAC. */
const coverPng = resolve(root, 'crates/media/tests/fixtures/cover.png');

/**
 * A second real original with a second real picture.
 *
 * The FLAC metadata blocks of the fixture are walked and the picture block
 * is replaced with one that carries the PNG. The audio frames are untouched,
 * so the file is still the same real original with a different cover — and
 * the two covers really hash to two different artworks.
 */
async function secondCover() {
  const flac = await readFile(brightCover);
  const png = await readFile(coverPng);
  let at = 4;
  let picture = null;
  while (at < flac.length) {
    const header = flac[at];
    const type = header & 0x7f;
    const length = (flac[at + 1] << 16) | (flac[at + 2] << 8) | flac[at + 3];
    if (type === 6) picture = { start: at, length };
    at += 4 + length;
    if ((header & 0x80) !== 0) break;
  }
  if (!picture) throw new Error('the fixture carries no picture block');
  const mime = Buffer.from('image/png', 'utf8');
  const mimeField = Buffer.alloc(4);
  mimeField.writeUInt32BE(mime.length, 0);
  const description = Buffer.alloc(4);
  const dimensions = Buffer.alloc(16);
  dimensions.writeUInt32BE(png.readUInt32BE(16), 0);
  dimensions.writeUInt32BE(png.readUInt32BE(20), 4);
  dimensions.writeUInt32BE(24, 8);
  dimensions.writeUInt32BE(0, 12);
  const dataLength = Buffer.alloc(4);
  dataLength.writeUInt32BE(png.length, 0);
  const block = Buffer.concat([
    Buffer.from([0, 0, 0, 3]),
    mimeField,
    mime,
    description,
    dimensions,
    dataLength,
    png,
  ]);
  const header = Buffer.from([0x06, (block.length >> 16) & 0xff, (block.length >> 8) & 0xff, block.length & 0xff]);
  return Buffer.concat([
    flac.subarray(0, picture.start),
    header,
    block,
    flac.subarray(picture.start + 4 + picture.length),
  ]);
}

/** A real account with three real originals: two with a cover, one without. */
async function setupAccount(page, runtime) {
  const setup = await page.request.post(runtime.origin + '/api/setup', {
    headers: { Origin: runtime.origin },
    data: { username: 'motion', password: 'private dr03 fixture password' },
  });
  if (!setup.ok() && setup.status() !== 409) throw new Error('setup failed: ' + setup.status());
  const me = setup.ok() ? await setup.json() : await (await page.request.get(runtime.origin + '/api/me')).json();
  const headers = { 'X-CSRF-Token': me.csrf_token, Origin: runtime.origin };
  const second = await secondCover();
  const tracks = [];
  for (const upload of [
    { name: 'Bright cover.flac', mimeType: 'audio/flac', buffer: await readFile(brightCover) },
    { name: 'Second cover.flac', mimeType: 'audio/flac', buffer: second },
  ]) {
    const response = await page.request.post(runtime.origin + '/api/upload', {
      headers, multipart: { file: upload },
    });
    if (!response.ok()) throw new Error('upload failed: ' + response.status());
    const track = (await response.json()).track;
    // Real catalog metadata, so the catalog grid really has an album with a
    // cover to measure. The audio file is untouched.
    const metadata = await (await page.request.get(runtime.origin + `/api/tracks/${track.id}/metadata`)).json();
    const cataloged = await page.request.patch(runtime.origin + `/api/tracks/${track.id}/metadata`, {
      headers,
      data: {
        revision: metadata.revision,
        title: track.title,
        artist: 'Motion fixtures',
        album: 'Motion fixtures',
        album_artist: 'Motion fixtures',
        genre: '',
        year: null,
        track_number: null,
        disc_number: null,
      },
    });
    if (!cataloged.ok()) throw new Error('metadata failed: ' + cataloged.status());
    tracks.push(track);
  }
  // A real original without a picture: the placeholder is the honest face
  // of a track the account has no cover for.
  const samples = 44100 * 4;
  const wave = Buffer.alloc(44 + samples * 2);
  wave.write('RIFF', 0); wave.writeUInt32LE(36 + samples * 2, 4); wave.write('WAVEfmt ', 8);
  wave.writeUInt16LE(16, 16); wave.writeUInt16LE(1, 20); wave.writeUInt16LE(1, 22);
  wave.writeUInt32LE(44100, 24); wave.writeUInt32LE(88200, 28); wave.writeUInt16LE(2, 32);
  wave.writeUInt16LE(16, 34); wave.write('data', 36); wave.writeUInt32LE(samples * 2, 40);
  for (let sample = 0; sample < samples; sample += 1) {
    wave.writeInt16LE(Math.round(2000 * Math.sin(2 * Math.PI * 220 * sample / 44100)), 44 + sample * 2);
  }
  const plain = await page.request.post(runtime.origin + '/api/upload', {
    headers, multipart: { file: { name: 'Plain signal.wav', mimeType: 'audio/wav', buffer: wave } },
  });
  if (!plain.ok()) throw new Error('upload failed: ' + plain.status() + ' ' + await plain.text());
  tracks.push((await plain.json()).track);
  return tracks;
}

/**
 * The library row with this exact title.
 *
 * The rows are found by their name and never by their position: the library
 * is ordered by the server, and three originals uploaded in the same moment
 * have no order of their own.
 */
function rowFor(page, title) {
  return page.locator('.track-row').filter({ has: page.locator('.track-title strong').filter({ hasText: new RegExp(`^\\s*${title}\\s*$`) }) });
}

/** The styles of every changed surface, as the browser computes them. */
const MOTION = () => {
  const read = (selector) => {
    const element = document.querySelector(selector);
    if (!element) return null;
    const style = getComputedStyle(element);
    return {
      transitionProperty: style.transitionProperty,
      transitionDuration: style.transitionDuration,
      animationName: style.animationName,
      animationDuration: style.animationDuration,
      transform: style.transform,
      height: style.height,
      opacity: style.opacity,
    };
  };
  return {
    button: read('.button'),
    iconButton: read('.icon-button'),
    trackRow: read('.track-row'),
    menu: read('.track-menu-panel'),
    menuItem: read('.track-menu-panel > button'),
    nowPlaying: read('.now-playing-dialog[open]'),
    queuePanel: read('#queue-drawer-dialog .queue-panel'),
    catalogCover: read('.catalog-card .catalog-cover'),
    homeAlbumImage: read('.home-album-card .catalog-cover img'),
    equalizer: read('.player-eq i'),
  };
};

const durations = value => String(value || '').split(',').map(part => Number.parseFloat(part) * 1000);
const properties = value => String(value || '').split(',').map(part => part.trim()).filter(Boolean);
const FORBIDDEN = ['height', 'width', 'top', 'left', 'right', 'bottom', 'margin', 'padding', 'background', 'background-position', 'color', 'box-shadow', 'filter'];

/** The queue of the drawer, as the reader sees it. */
const QUEUE_ROWS = () => [...document.querySelectorAll('#queue-drawer-dialog .queue-row')].map(row => ({
  title: row.querySelector('.queue-track strong')?.textContent?.trim() || '',
  current: row.classList.contains('current'),
  settle: row.classList.contains('queue-row-settle'),
  settleFrom: row.style.getPropertyValue('--queue-settle-from').trim(),
}));

/** The artwork of the player is really there: the layer is ready. */
async function coverReady(page) {
  await expect.poll(async () => page.locator('.cover-fade-layer.is-ready').count(), { timeout: 15000 })
    .toBeGreaterThan(0);
}

/**
 * The row menu is a toggle: a click on the trigger of the row that is
 * already open closes it. The journeys therefore close the menu before
 * they open one, so a click is always an opening.
 */
async function closeMenu(page) {
  const open = page.locator('.track-row button[aria-haspopup="menu"][aria-expanded="true"]');
  if (await open.count()) {
    await open.first().click({ timeout: 20000 });
    await expect(page.locator('.track-menu-panel')).toHaveCount(0);
  }
}

async function openMenu(page, title) {
  await closeMenu(page);
  await rowFor(page, title).locator('button[aria-haspopup="menu"]').click({ timeout: 20000 });
  await expect(page.locator('.track-menu-panel')).toBeVisible();
}

test('DR03 the short answer never delays the command', async ({ page, browserName }) => {
  test.setTimeout(900000);
  const output = evidenceDir(browserName);
  await mkdir(output, { recursive: true });
  // Every action of this spec is bounded: a journey that waits for
  // something that never comes fails with a real message instead of
  // hanging until the test timeout.
  page.setDefaultTimeout(20000);
  const runtime = await startServer(output);
  const evidence = { engine: browserName, journeys: {} };
  try {
    const tracks = await setupAccount(page, runtime);
    expect(tracks.length, 'three real originals').toBe(3);
    const withCover = tracks.filter(track => track.artwork_id);
    expect(withCover.length, 'two real covers').toBe(2);
    expect(new Set(withCover.map(track => track.artwork_id)).size, 'two different artworks').toBe(2);
    expect(tracks.filter(track => !track.artwork_id).length, 'one real original without a cover').toBe(1);

    await page.setViewportSize({ width: 1284, height: 856 });
    await page.goto(runtime.origin);
    await expect(page.locator('.auth-card, .app-shell')).toBeVisible();
    if (await page.locator('.app-shell').count() === 0) {
      await page.locator('#username').fill('motion');
      await page.locator('#password').fill('private dr03 fixture password');
      await page.getByRole('button', { name: /sign in|create your music space/i }).first().click();
    }
    await expect(page.locator('.app-shell')).toBeVisible();
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible', timeout: 20000 });

    const bright = 'Bright cover';
    const second = 'Second cover';
    const plain = 'Plain signal';

    /* --- 1. the durations and the properties of every changed surface. */
    await rowFor(page, bright).locator('.track-title').click({ timeout: 20000 });
    await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
    await openMenu(page, bright);
    console.log('DR03 journey 1: the durations');
    const styles = await page.evaluate(MOTION);
    evidence.journeys.styles = styles;
    const expectTransition = (name, value, allowed, milliseconds) => {
      expect(value, `${name} exists`).toBeTruthy();
      for (const property of properties(value.transitionProperty)) {
        expect(allowed, `${name} transitions ${property}`).toContain(property);
        expect(FORBIDDEN, `${name} does not animate ${property}`).not.toContain(property);
      }
      for (const duration of durations(value.transitionDuration)) {
        expect(duration, `${name} transition duration`).toBeCloseTo(milliseconds, 0);
      }
    };
    expectTransition('the button', styles.button, ['transform', 'opacity'], 80);
    expectTransition('the icon button', styles.iconButton, ['transform', 'opacity'], 80);
    expectTransition('the track row', styles.trackRow, ['opacity'], 80);
    expect(styles.menu.animationName, 'the row menu fades in with its own keyframes').toContain('vela-menu-fade');
    expect(durations(styles.menu.animationDuration)[0], 'the row menu enters in the quick role').toBeCloseTo(120, 0);
    expect(styles.menuItem.animationName, 'the row menu content enters with its own keyframes').toContain('vela-menu-enter');
    expect(durations(styles.menuItem.animationDuration)[0], 'the row menu content enters in the quick role').toBeCloseTo(120, 0);
    expectTransition('the row menu', styles.menu, ['transform', 'opacity'], 0);
    expectTransition('the row menu content', styles.menuItem, ['transform', 'opacity'], 0);
    // The pressed state of a control is a transform of the control itself.
    const icon = page.locator('.icon-button').first();
    const box = await icon.boundingBox();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.waitForTimeout(200);
    const pressed = await icon.evaluate(node => getComputedStyle(node).transform);
    // The pointer leaves the control before it is released, so the control
    // answers the press and nothing else: no command is fired.
    await page.mouse.move(box.x + box.width / 2 + 240, box.y + box.height / 2);
    await page.mouse.up();
    expect(pressed, 'the pressed control answers with a scale').toMatch(/matrix\(0\.97/);
    evidence.journeys.pressed = pressed;

    // The Now Playing sheet and the queue drawer.
    await page.keyboard.press('Escape');
    await page.locator('.now-playing-trigger').click({ timeout: 20000 });
    await expect(page.locator('.now-playing-dialog[open]')).toBeVisible();
    const withSheet = await page.evaluate(MOTION);
    expect(withSheet.nowPlaying.animationName, 'the sheet enters with its own keyframes').toContain('np-enter');
    expect(durations(withSheet.nowPlaying.animationDuration)[0], 'the sheet enters in the overlay role').toBeCloseTo(140, 0);
    expect(durations(withSheet.equalizer.animationDuration)[0], 'the equalizer keeps its own loop').toBeCloseTo(800, 0);
    expect(withSheet.equalizer.height, 'the equalizer bar has one fixed height').toBe('10px');
    await coverReady(page);
    await page.screenshot({ path: `${output}/now-playing-1284x856.png` });
    await page.keyboard.press('Escape');
    await expect(page.locator('.now-playing-dialog')).toHaveCount(0);

    await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
    await expect(page.locator('#queue-drawer-dialog')).toBeVisible();
    const withDrawer = await page.evaluate(MOTION);
    expect(withDrawer.queuePanel.animationName, 'the drawer enters with its own keyframes').toContain('vela-drawer-enter');
    expect(durations(withDrawer.queuePanel.animationDuration)[0], 'the drawer enters in the panel role').toBeCloseTo(180, 0);
    expectTransition('the queue drawer', withDrawer.queuePanel, ['transform', 'opacity'], 0);
    await page.screenshot({ path: `${output}/queue-drawer-1284x856.png` });
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);

    // The home album cover and the catalog cover.
    await navigate(page, 'Albums');
    await page.locator('.catalog-card').first().waitFor({ state: 'visible', timeout: 20000 });
    const catalog = await page.evaluate(MOTION);
    expectTransition('the catalog cover', catalog.catalogCover, ['transform'], 120);
    await page.screenshot({ path: `${output}/albums-1284x856.png` });
    await navigate(page, 'Home');
    await page.locator('.home-album-card').first().waitFor({ state: 'visible', timeout: 20000 });
    const home = await page.evaluate(MOTION);
    expect(home.homeAlbumImage, 'the home album cover has an image').toBeTruthy();
    expectTransition('the home album cover', home.homeAlbumImage, ['transform'], 120);
    await page.screenshot({ path: `${output}/home-1284x856.png` });
    await navigate(page, 'Library');

    /* --- 2. the time from the event to the first painted frame. */
    console.log('DR03 journey 2: the event to the first frame');
    const menuTiming = await page.evaluate(async () => {
      const open = document.querySelector('.track-row button[aria-haspopup="menu"][aria-expanded="true"]');
      if (open) {
        open.click();
        await new Promise(resolve => setTimeout(resolve, 300));
      }
      const trigger = document.querySelector('.track-row button[aria-haspopup="menu"][aria-expanded="false"]');
      if (!trigger) throw new Error(`no closed trigger: ${[...document.querySelectorAll('.track-row button[aria-haspopup="menu"]')].map(node => node.getAttribute('aria-expanded')).join(', ')}`);
      const start = performance.now();
      trigger.click();
      const panel = await new Promise(resolve => {
        const check = () => {
          const element = document.querySelector('.track-menu-panel');
          if (element && Number(getComputedStyle(element).opacity) > 0) return resolve(element);
          requestAnimationFrame(check);
        };
        requestAnimationFrame(check);
      });
      const painted = performance.now();
      const animations = panel.getAnimations();
      return {
        elapsed: painted - start,
        opacity: Number(getComputedStyle(panel).opacity),
        animationName: animations.length ? animations[0].animationName : null,
        animationTime: animations.length ? Number(animations[0].currentTime) : null,
      };
    });
    evidence.journeys.menuTiming = menuTiming;
    expect(menuTiming.animationName, 'the menu is animating in the first frame').toContain('vela-menu-fade');
    expect(menuTiming.elapsed, 'the answer is painted within a hundred milliseconds').toBeLessThanOrEqual(100);
    expect(menuTiming.animationTime, 'the effect starts with the frame, not before it').toBeLessThan(50);
    expect(menuTiming.opacity, 'the panel is on the screen').toBeGreaterThan(0);
    await page.keyboard.press('Escape');

    /* --- 3. the command, the focus and the audio do not wait. */
    /* --- 3. the command, the focus and the audio do not wait. */
    const before = await page.evaluate(() => {
      const audio = document.querySelector('#vela-audio');
      audio.dataset.dr03Marker = 'kept';
      return {
        label: document.querySelector('#player-toggle').getAttribute('aria-label'),
        state: document.querySelector('.player-eq').dataset.state,
        audios: document.querySelectorAll('audio').length,
        src: audio.getAttribute('src'),
        generation: audio.getAttribute('data-source-generation'),
      };
    });
    expect(before.label, 'the track is playing').toBe('Pause');
    expect(before.audios, 'one audio element').toBe(1);
    await openMenu(page, bright);
    // The decoration is on the screen now. It is a picture that appears: the
    // audio element, its source and its generation are untouched.
    const decorated = await page.evaluate(() => {
      const audio = document.querySelector('#vela-audio');
      return {
        audios: document.querySelectorAll('audio').length,
        sameNode: audio.dataset.dr03Marker === 'kept',
        src: audio.getAttribute('src'),
        generation: audio.getAttribute('data-source-generation'),
      };
    });
    evidence.journeys.decoration = decorated;
    expect(decorated.audios, 'the decoration added no audio element').toBe(1);
    expect(decorated.sameNode, 'the decoration did not replace the audio element').toBe(true);
    expect(decorated.src, 'the decoration did not touch the source').toBe(before.src);
    expect(decorated.generation, 'the decoration did not restart the source').toBe(before.generation);
    const command = await page.evaluate(async () => {
      const start = performance.now();
      document.querySelector('#player-toggle').click();
      const label = await new Promise(resolve => {
        const check = () => {
          const element = document.querySelector('#player-toggle');
          if (element.getAttribute('aria-label') !== 'Pause') return resolve(element.getAttribute('aria-label'));
          requestAnimationFrame(check);
        };
        requestAnimationFrame(check);
      });
      const audio = document.querySelector('#vela-audio');
      return {
        elapsed: performance.now() - start,
        label,
        state: document.querySelector('.player-eq').dataset.state,
        audios: document.querySelectorAll('audio').length,
        sameNode: audio.dataset.dr03Marker === 'kept',
        src: audio.getAttribute('src'),
      };
    });
    evidence.journeys.command = command;
    expect(command.label, 'the command is answered in the first frame').toBe('Play');
    expect(command.elapsed, 'the command does not wait for an effect').toBeLessThanOrEqual(100);
    expect(command.audios, 'still one audio element').toBe(1);
    expect(command.sameNode, 'the command kept the audio element').toBe(true);
    expect(command.src, 'the command did not reload the source').toBe(before.src);
    await page.keyboard.press('Escape');

    // Escape closes the sheet in the tick of the cancel event.
    await page.locator('.now-playing-trigger').click({ timeout: 20000 });
    await expect(page.locator('.now-playing-dialog[open]')).toBeVisible();
    const cancel = await page.evaluate(async () => {
      const dialog = document.querySelector('#now-playing-dialog');
      const seen = [];
      dialog.addEventListener('cancel', () => { seen.push(dialog.open); });
      return new Promise(resolve => {
        dialog.dispatchEvent(new Event('cancel', { cancelable: true }));
        requestAnimationFrame(() => resolve({ openInCancel: seen[0] ?? null, open: dialog.open }));
      });
    });
    evidence.journeys.cancel = cancel;
    expect(cancel.openInCancel, 'the sheet leaves the top layer in the tick of the cancel').toBe(false);
    await page.waitForTimeout(200);
    expect(await page.locator('.now-playing-dialog').count(), 'the sheet does not open again').toBe(0);
    expect(await page.evaluate(() => document.activeElement?.getAttribute('aria-label')), 'the focus is back on the trigger')
      .toBe('Open now playing');

    /* --- 4. the artwork crossfade. */
    await rowFor(page, bright).locator('.track-title').click({ timeout: 20000 });
    await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
    console.log('DR03 journey 4: the crossfade');
    // The resource timeline is the honest count: a picture that the browser
    // already holds is not a new entry.
    const artworkEntries = () => page.evaluate(() => performance.getEntriesByType('resource')
      .filter(entry => entry.name.includes('/api/artwork/'))
      .map(entry => entry.name));
    const entriesBefore = await artworkEntries();
    const layers = () => page.evaluate(() => ({
      count: document.querySelectorAll('.cover-fade .cover-fade-layer').length,
      leaving: document.querySelectorAll('.cover-fade .cover-fade-layer.is-leaving').length,
      ready: document.querySelectorAll('.cover-fade .cover-fade-layer.is-ready').length,
      audios: document.querySelectorAll('audio').length,
    }));
    const beforeLayers = await layers();
    expect(beforeLayers.count, 'one layer while nothing changes').toBe(1);
    // The next track has the other cover: the crossfade runs. The layers are
    // sampled frame by frame while it happens, so a fade that is over
    // before the first poll is still seen.
    const crossfade = await page.evaluate(async (title) => {
      const counts = [];
      const leaving = [];
      const names = [...document.querySelectorAll('.track-row .track-title strong')];
      const at = names.findIndex(node => node.textContent.trim() === title);
      if (at < 0) throw new Error(`no row named ${title}: ${names.map(node => node.textContent).join(', ')}`);
      document.querySelectorAll('.track-row .track-title')[at].click();
      for (let step = 0; step < 90; step += 1) {
        await new Promise(resolve => requestAnimationFrame(resolve));
        counts.push(document.querySelectorAll('.cover-fade .cover-fade-layer').length);
        leaving.push(document.querySelectorAll('.cover-fade .cover-fade-layer.is-leaving').length);
      }
      return {
        layers: Math.max(...counts),
        leaving: Math.max(...leaving),
        final: counts[counts.length - 1],
        audios: document.querySelectorAll('audio').length,
      };
    }, second);
    evidence.journeys.crossfade = { before: beforeLayers, during: crossfade };
    expect(crossfade.leaving, 'the outgoing layer leaves').toBe(1);
    expect(crossfade.layers, 'two layers at most').toBeLessThanOrEqual(2);
    expect(crossfade.final, 'the outgoing layer is gone').toBe(1);
    expect(crossfade.audios, 'still one audio element').toBe(1);
    const entriesAfter = await artworkEntries();
    // The fade adds the request for the new cover and nothing else: the
    // outgoing picture is not fetched again.
    const added = entriesAfter.slice(entriesBefore.length);
    expect(added.length, `one request for the new cover (${added.join(', ')})`).toBe(1);
    const newCover = tracks.find(track => track.title === second).artwork_id;
    expect(added[0], 'the request is the new cover').toContain(newCover);

    // Three changes in a row: never more than two layers, one at the end.
    const rapid = await page.evaluate(async (indexes) => {
      const distinct = new Set();
      for (const index of indexes) {
        const titles = [...document.querySelectorAll('.track-row .track-title strong')];
        const at = titles.findIndex(node => node.textContent.trim() === index);
        if (at < 0) throw new Error(`no row named ${index}: ${titles.map(node => JSON.stringify(node.textContent)).join(', ')}`);
        document.querySelectorAll('.track-row .track-title')[at].click();
      }
      const counts = [];
      const layers = [];
      for (let step = 0; step < 90; step += 1) {
        await new Promise(resolve => requestAnimationFrame(resolve));
        const audios = [...document.querySelectorAll('audio')];
        for (const audio of audios) {
          if (!audio.dataset.dr03Mark) audio.dataset.dr03Mark = `audio-${distinct.size}`;
          distinct.add(audio.dataset.dr03Mark);
        }
        counts.push(audios.length);
        layers.push(document.querySelectorAll('.cover-fade .cover-fade-layer').length);
      }
      return {
        layers: Math.max(...layers),
        final: layers[layers.length - 1],
        audios: counts[counts.length - 1],
        distinct: distinct.size,
        minimum: Math.min(...counts),
      };
    }, [plain, bright, second]);
    evidence.journeys.rapid = rapid;
    expect(rapid.layers, 'never more than two layers').toBeLessThanOrEqual(2);
    expect(rapid.final, 'one layer at the end').toBe(1);
    expect(rapid.audios, 'still one audio element').toBe(1);
    expect(rapid.distinct, 'the decoration added no audio element').toBe(1);
    expect(rapid.minimum, 'the audio element was never gone').toBe(1);

    // A track without a cover: the placeholder is the honest face and no
    // layer is left behind.
    await rowFor(page, plain).locator('.track-title').click({ timeout: 20000 });
    await expect.poll(async () => (await layers()).count, { timeout: 10000 }).toBe(1);
    const placeholder = await page.evaluate(() => ({
      leaving: document.querySelectorAll('.cover-fade .cover-fade-layer.is-leaving').length,
      icons: document.querySelectorAll('.cover-fade .cover-fade-layer .artwork .vela-icon').length,
    }));
    evidence.journeys.placeholder = placeholder;
    expect(placeholder.leaving, 'no layer is left behind').toBe(0);
    expect(placeholder.icons, 'the placeholder is there').toBeGreaterThan(0);
    await page.screenshot({ path: `${output}/player-no-cover-1284x856.png` });

    /* --- 6. the queue move. */
    await rowFor(page, bright).locator('.track-title').click({ timeout: 20000 });
    await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
    // Two occurrences of the same track: the drawer keeps them apart.
    console.log('DR03 journey 6: the queue move');
    await page.getByRole('button', { name: `Add ${bright} to playback queue`, exact: true }).click({ timeout: 20000 });
    await page.getByRole('button', { name: `Add ${bright} to playback queue`, exact: true }).click({ timeout: 20000 });
    await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
    await expect(page.locator('#queue-drawer-dialog')).toBeVisible();
    const order = () => page.evaluate(QUEUE_ROWS);
    const beforeMove = await order();
    expect(beforeMove.length, 'five occurrences in the drawer').toBe(5);
    const brightTitle = tracks.find(track => track.title === 'Bright cover').title;
    const duplicates = beforeMove.filter(row => row.title === brightTitle).length;
    expect(duplicates, 'the drawer keeps the occurrences apart').toBe(3);
    const audioBefore = await page.evaluate(() => ({
      audios: document.querySelectorAll('audio').length,
      src: document.querySelector('#vela-audio').getAttribute('src'),
      generation: document.querySelector('#vela-audio').getAttribute('data-source-generation'),
    }));
    const currentBefore = beforeMove.findIndex(row => row.current);
    await page.locator('#queue-move-up-2').click({ timeout: 20000 });
    const moved = await order();
    evidence.journeys.move = { before: beforeMove, after: moved };
    expect(moved.map(row => row.title).join('|'), 'the order changed at once')
      .not.toBe(beforeMove.map(row => row.title).join('|'));
    expect(moved[1].title, 'the moved row is the destination').toBe(beforeMove[2].title);
    expect(moved[1].settle, 'the destination row settles').toBe(true);
    expect(moved[1].settleFrom, 'the settle comes from the direction of the move').toBe('-4px');
    await expect.poll(async () => (await order()).some(row => row.settle), { timeout: 10000 }).toBe(false);
    const settled = await order();
    expect(settled[1].title, 'the settle did not move the row again').toBe(beforeMove[2].title);
    expect(settled.length, 'the move did not drop an occurrence').toBe(5);
    expect(settled.filter(row => row.title === brightTitle).length, 'the duplicate occurrences are still apart').toBe(3);
    const focus = await page.evaluate(() => ({
      label: document.activeElement?.getAttribute('aria-label') || '',
      id: document.activeElement?.id || '',
      row: document.activeElement?.closest('.queue-row')?.querySelector('.queue-track strong')?.textContent?.trim() || '',
    }));
    evidence.journeys.moveFocus = focus;
    expect(focus.label, 'the focus is on the same control').toBe('Move track up');
    expect(focus.id, 'the focus is on the destination row').toBe('queue-move-up-1');
    expect(focus.row, 'the focus is on the moved row').toBe(beforeMove[2].title);
    const audioAfter = await page.evaluate(() => ({
      audios: document.querySelectorAll('audio').length,
      src: document.querySelector('#vela-audio').getAttribute('src'),
      generation: document.querySelector('#vela-audio').getAttribute('data-source-generation'),
    }));
    expect(audioAfter.audios, 'still one audio element').toBe(1);
    expect(audioAfter.src, 'the move did not reload the source').toBe(audioBefore.src);
    expect(audioAfter.generation, 'the move did not restart the source').toBe(audioBefore.generation);
    const currentAfter = settled.findIndex(row => row.current);
    // The current occurrence keeps its identity: a row that moved past it
    // takes its place with it, so the marked row follows the swap. The
    // current row was the destination here, so it is now one row lower.
    const movedRow = 2;
    const destination = movedRow - 1;
    const expectedCurrent = currentBefore === movedRow
      ? destination
      : currentBefore === destination
        ? movedRow
        : currentBefore;
    expect(currentAfter, 'the current occurrence is the same one').toBe(expectedCurrent);
    expect(settled[currentAfter].title, 'the current occurrence is the same track')
      .toBe(beforeMove[currentBefore].title);

    // A second move right after the first one: one receipt, one settle.
    await page.locator('#queue-move-up-1').click({ timeout: 20000 });
    const secondMove = await order();
    expect(secondMove.filter(row => row.settle).length, 'one settled row').toBe(1);
    await expect.poll(async () => (await order()).some(row => row.settle), { timeout: 10000 }).toBe(false);
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);

    /* --- 5. reduced motion. */
    console.log('DR03 journey 5: reduced motion');
    await page.emulateMedia({ reducedMotion: 'reduce' });
    // Every changed selector has to be in the document when it is read: a
    // selector that is not there answers `null` and would quietly skip its
    // own assertion. The menu, the drawer and the catalog cover are opened
    // on purpose, in that order, so each of them is measured in reduce.
    const readReduced = () => page.evaluate(() => {
      const read = (selector) => {
        const element = document.querySelector(selector);
        if (!element) return null;
        const style = getComputedStyle(element);
        const pseudo = name => {
          const value = getComputedStyle(element, name);
          return { animationName: value.animationName, transitionDuration: value.transitionDuration };
        };
        return {
          animationName: style.animationName,
          animationDuration: style.animationDuration,
          transitionDuration: style.transitionDuration,
          transform: style.transform,
          running: element.getAnimations().length,
          before: pseudo('::before'),
          after: pseudo('::after'),
        };
      };
      return {
        button: read('.button'),
        iconButton: read('.icon-button'),
        equalizer: read('.player-eq i'),
        layer: read('.cover-fade .cover-fade-layer'),
        menu: read('.track-menu-panel'),
        menuItem: read('.track-menu-panel > button'),
        queuePanel: read('#queue-drawer-dialog .queue-panel'),
        catalogCover: read('.catalog-card .catalog-cover'),
      };
    });
    const checkReduced = (name, value) => {
      expect(value, `${name} is in the document under reduced motion`).toBeTruthy();
      expect(value.animationName, `${name} runs no keyframes`).toBe('none');
      expect(durations(value.animationDuration)[0], `${name} has no animation`).toBe(0);
      for (const duration of durations(value.transitionDuration)) {
        expect(duration, `${name} has no transition`).toBe(0);
      }
      expect(value.running, `${name} has no running animation`).toBe(0);
      expect(value.before.animationName, `${name} has no keyframes before`).toBe('none');
      expect(value.after.animationName, `${name} has no keyframes after`).toBe('none');
    };
    const reduced = { media: await page.evaluate(() => matchMedia('(prefers-reduced-motion: reduce)').matches) };
    expect(reduced.media, 'the media query is active').toBe(true);

    // The base surfaces of the library page.
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible', timeout: 20000 });
    Object.assign(reduced, await readReduced());
    for (const name of ['button', 'iconButton', 'equalizer', 'layer']) checkReduced(name, reduced[name]);
    expect(reduced.equalizer.transform, 'the equalizer keeps its own height').toMatch(/matrix\(1, 0, 0, 0\.[0-9]/);

    // The row menu and its content.
    await openMenu(page, 'Bright cover');
    const menu = await readReduced();
    checkReduced('menu', menu.menu);
    checkReduced('menuItem', menu.menuItem);
    reduced.menu = menu.menu;
    reduced.menuItem = menu.menuItem;
    await page.keyboard.press('Escape');

    // The queue drawer and its panel.
    await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
    await expect(page.locator('#queue-drawer-dialog')).toBeVisible();
    const drawer = await readReduced();
    checkReduced('queuePanel', drawer.queuePanel);
    reduced.queuePanel = drawer.queuePanel;
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);

    // The catalog cover, which only exists on the album shelf.
    await navigate(page, 'Albums');
    await page.locator('.catalog-card').first().waitFor({ state: 'visible', timeout: 20000 });
    const shelf = await readReduced();
    checkReduced('catalogCover', shelf.catalogCover);
    reduced.catalogCover = shelf.catalogCover;
    await navigate(page, 'Library');

    // The Now Playing sheet keeps its surface and loses its movement.
    await page.locator('.now-playing-trigger').click({ timeout: 20000 });
    await expect(page.locator('.now-playing-dialog[open]')).toBeVisible();
    reduced.nowPlaying = await page.evaluate(() => {
      const element = document.querySelector('.now-playing-dialog[open]');
      const style = getComputedStyle(element);
      const pseudo = name => {
        const value = getComputedStyle(element, name);
        return { animationName: value.animationName, transitionDuration: value.transitionDuration };
      };
      return {
        animationName: style.animationName,
        animationDuration: style.animationDuration,
        transitionDuration: style.transitionDuration,
        transform: style.transform,
        running: element.getAnimations().length,
        before: pseudo('::before'),
        after: pseudo('::after'),
      };
    });
    checkReduced('nowPlaying', reduced.nowPlaying);
    evidence.journeys.reduced = reduced;
    // The controls still work.
    await page.keyboard.press('Escape');
    await expect(page.locator('.now-playing-dialog')).toHaveCount(0);
    await page.locator('#player-toggle').click();
    await expect(page.locator('#player-toggle')).toHaveAttribute('aria-label', 'Play');
    await page.emulateMedia({ reducedMotion: 'no-preference' });

    await writeFile(join(output, 'dr03-motion.json'), JSON.stringify(evidence, null, 2));
  } finally {
    await runtime.close();
  }
});
