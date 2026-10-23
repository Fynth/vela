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
import { deflateSync } from 'node:zlib';
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

/**
 * A real PNG of one solid colour, built here.
 *
 * The spec needs more than the two tracked covers: a third picture for the
 * late-answer journey, and a picture that is not a file on disk. The bytes
 * are a valid 8-bit truecolour PNG, so the pipeline decodes them like any
 * other cover and hashes them into its own artwork.
 */
function solidPng(size, [red, green, blue]) {
  const crcTable = (() => {
    const table = new Int32Array(256);
    for (let n = 0; n < 256; n += 1) {
      let c = n;
      for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      table[n] = c;
    }
    return table;
  })();
  const crc = bytes => {
    let value = -1;
    for (const byte of bytes) value = crcTable[(value ^ byte) & 0xff] ^ (value >>> 8);
    return (value ^ -1) >>> 0;
  };
  const chunk = (type, data) => {
    const length = Buffer.alloc(4);
    length.writeUInt32BE(data.length, 0);
    const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
    const sum = Buffer.alloc(4);
    sum.writeUInt32BE(crc(body), 0);
    return Buffer.concat([length, body, sum]);
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(size, 0);
  header.writeUInt32BE(size, 4);
  header[8] = 8;
  header[9] = 2;
  const raw = Buffer.alloc((size * 3 + 1) * size);
  for (let y = 0; y < size; y += 1) {
    const row = y * (size * 3 + 1);
    raw[row] = 0;
    for (let x = 0; x < size; x += 1) {
      raw[row + 1 + x * 3] = red;
      raw[row + 2 + x * 3] = green;
      raw[row + 3 + x * 3] = blue;
    }
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', header),
    chunk('IDAT', deflateSync(raw)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

/**
 * A real original with an embedded picture and an optional own title.
 *
 * The FLAC metadata blocks are walked and rewritten in place: the picture
 * block carries the given PNG, and the Vorbis comment carries the given
 * title. The audio frames of the fixture are untouched, so the file stays a
 * real original the pipeline accepts — and two such files really differ in
 * their picture hash, their title, or neither.
 */
async function rewriteFixture({ png, title }) {
  const flac = await readFile(brightCover);
  let at = 4;
  let found = null;
  while (at < flac.length) {
    const header = flac[at];
    const type = header & 0x7f;
    const length = (flac[at + 1] << 16) | (flac[at + 2] << 8) | flac[at + 3];
    if (type === 6) found = { start: at, length };
    at += 4 + length;
    if ((header & 0x80) !== 0) break;
  }
  if (!found) throw new Error('the fixture carries no picture block');
  let block;
  if (png) {
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
    block = Buffer.concat([
      Buffer.from([0, 0, 0, 3]),
      mimeField,
      mime,
      description,
      dimensions,
      dataLength,
      png,
    ]);
  } else {
    // The picture block of the fixture, kept byte for byte: the second track
    // shares the artwork, not the audio.
    block = Buffer.from(flac.subarray(found.start + 4, found.start + 4 + found.length));
  }
  let out = Buffer.concat([
    flac.subarray(0, found.start),
    Buffer.from([0x06, (block.length >> 16) & 0xff, (block.length >> 8) & 0xff, block.length & 0xff]),
    block,
    flac.subarray(found.start + 4 + found.length),
  ]);
  if (title) {
    // Rewrite the Vorbis comment block so the track has its own title while
    // its picture stays the same one.
    let cursor = 4;
    let comment = null;
    while (cursor < out.length) {
      const header = out[cursor];
      const type = header & 0x7f;
      const length = (out[cursor + 1] << 16) | (out[cursor + 2] << 8) | out[cursor + 3];
      if (type === 4) comment = { start: cursor, length };
      cursor += 4 + length;
      if ((header & 0x80) !== 0) break;
    }
    if (!comment) throw new Error('the fixture carries no comment block');
    const vendorLength = out.readUInt32LE(comment.start + 4);
    const vendor = out.subarray(comment.start + 8, comment.start + 8 + vendorLength);
    const parts = [Buffer.alloc(4)];
    parts[0].writeUInt32LE(vendor.length, 0);
    parts.push(vendor);
    const count = Buffer.alloc(4);
    const fields = [`TITLE=${title}`, 'ARTIST=Shared picture'];
    count.writeUInt32LE(fields.length, 0);
    parts.push(count);
    for (const field of fields) {
      const bytes = Buffer.from(field, 'utf8');
      const size = Buffer.alloc(4);
      size.writeUInt32LE(bytes.length, 0);
      parts.push(size, bytes);
    }
    const body = Buffer.concat(parts);
    const header = Buffer.from([0x04, (body.length >> 16) & 0xff, (body.length >> 8) & 0xff, body.length & 0xff]);
    out = Buffer.concat([
      out.subarray(0, comment.start),
      header,
      body,
      out.subarray(comment.start + 4 + comment.length),
    ]);
  }
  return out;
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
  // A third real original that carries the picture of the first one: two
  // tracks, two occurrences, one artwork. The pipeline hashes the picture, so
  // these two really share one artwork id while staying two tracks.
  const sharedFile = await rewriteFixture({ title: 'Shared picture' });
  // A third picture the browser has never fetched, for the journey that
  // holds the answer of a layer that is already leaving.
  const lateFile = await rewriteFixture({ png: solidPng(64, [200, 40, 90]), title: 'Late answer' });
  const tracks = [];
  for (const upload of [
    { name: 'Bright cover.flac', mimeType: 'audio/flac', buffer: await readFile(brightCover) },
    { name: 'Second cover.flac', mimeType: 'audio/flac', buffer: second },
    { name: 'Shared picture.flac', mimeType: 'audio/flac', buffer: sharedFile },
    { name: 'Late answer.flac', mimeType: 'audio/flac', buffer: lateFile },
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
 * Stop the short fixture through the root control before reading artwork or
 * cursor identity.
 *
 * The application marks the element it wants to play with
 * `data-play-request`, and it sets and removes that mark in the tick of the
 * command. The mark is the truth of the media clock: the transport's label is
 * a render of it, and a render is a tick later, so a pause that is decided on
 * a stale label is skipped — and the clock that then runs is the flake this
 * package closed. A click on the track that is already current stops the
 * transport instead of starting it; that click leaves no mark behind, and
 * there is nothing left to stop.
 */
async function pausePlayback(page) {
  const toggle = page.locator('#player-toggle');
  await expect(toggle).toBeVisible();
  const audio = page.locator('#vela-audio');
  const requested = () => audio.evaluate(node => node.hasAttribute('data-play-request'));
  if (await requested()) await toggle.click();
  await expect.poll(requested, { timeout: 10000 }).toBe(false);
  await expect.poll(() => audio.evaluate(node => node.paused)).toBe(true);
}

async function clickPausedRow(page, title) {
  await rowFor(page, title).locator('.track-title').click({ timeout: 20000 });
  await expect(page.locator('.player-title')).toHaveText(title);
  await pausePlayback(page);
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

test('DR03 the short answer never delays the command', async ({ page, browser, browserName }) => {
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
    expect(tracks.length, 'five real originals').toBe(5);
    const withCover = tracks.filter(track => track.artwork_id);
    expect(withCover.length, 'four real covers').toBe(4);
    expect(new Set(withCover.map(track => track.artwork_id)).size, 'three different artworks').toBe(3);
    expect(tracks.filter(track => !track.artwork_id).length, 'one real original without a cover').toBe(1);
    const sharedArtwork = tracks.find(track => track.title === 'Shared picture').artwork_id;
    expect(withCover.filter(track => track.artwork_id === sharedArtwork).length,
      'two real tracks share one artwork').toBe(2);

    const bright = 'Bright cover';
    const second = 'Second cover';
    const plain = 'Plain signal';
    const late = 'Late answer';

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
    expect(durations(styles.equalizer.animationDuration)[0], 'the equalizer keeps its own loop').toBeCloseTo(800, 0);
    expect(styles.equalizer.height, 'the equalizer bar has one fixed height').toBe('10px');
    await pausePlayback(page);
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

    // The Now Playing sheet and the queue drawer. The row menu is closed on
    // its own trigger instead of with Escape alone: a menu that is still in
    // the document covers the player bar and would swallow the next click.
    await page.keyboard.press('Escape');
    await closeMenu(page);
    await page.locator('.now-playing-trigger').click({ timeout: 20000 });
    await expect(page.locator('.now-playing-dialog[open]')).toBeVisible();
    const withSheet = await page.evaluate(MOTION);
    expect(withSheet.nowPlaying.animationName, 'the sheet enters with its own keyframes').toContain('np-enter');
    expect(durations(withSheet.nowPlaying.animationDuration)[0], 'the sheet enters in the overlay role').toBeCloseTo(140, 0);
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
    await page.locator('#player-toggle').click();
    await expect(page.locator('#player-toggle')).toHaveAttribute('aria-label', 'Pause');
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
    await clickPausedRow(page, bright);
    console.log('DR03 journey 4: the crossfade');
    // The resource timeline is the honest count: a picture the browser has
    // already answered for is not a new entry, and the same URL is asked for
    // by two existing consumers (the layer's image element and the Media
    // Session metadata). What the crossfade must never do is fetch the
    // picture it is fading out a second time, so the assertion is about the
    // outgoing picture, not about a total.
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
    // The page-side helper of the two journeys that sample a picture frame
    // by frame. The media clock of a three-second fixture is not a part of a
    // picture measurement: the application marks the element it wants to
    // play with `data-play-request` and sets that mark in the tick of the
    // command, so the mark is waited for and the transport is stopped
    // through the root control before the first frame is sampled. A clock
    // that is still running after that is a real fault and fails the
    // journey by name.
    await page.evaluate(() => {
      window.dr03StopTheClock = async () => {
        const audio = document.querySelector('#vela-audio');
        const toggle = document.querySelector('#player-toggle');
        for (let step = 0; step < 60 && !audio.hasAttribute('data-play-request'); step += 1) {
          await new Promise(resolve => requestAnimationFrame(resolve));
        }
        if (!audio.hasAttribute('data-play-request')) return;
        toggle.click();
        for (let step = 0; step < 60 && audio.hasAttribute('data-play-request'); step += 1) {
          await new Promise(resolve => requestAnimationFrame(resolve));
        }
        if (audio.hasAttribute('data-play-request') || !audio.paused) {
          throw new Error('the media clock did not stop');
        }
      };
    });
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
      await window.dr03StopTheClock();
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
    // The fade never fetches the picture it is fading out a second time. The
    // total count is not the honest number here — the same URL is asked for
    // by the layer and by the Media Session metadata, and a cached picture
    // adds no entry at all — so the assertion is about which picture was
    // asked for.
    const added = entriesAfter.slice(entriesBefore.length);
    const outgoingCover = tracks.find(track => track.title === bright).artwork_id;
    const newCover = tracks.find(track => track.title === second).artwork_id;
    evidence.journeys.requests = { added, outgoingCover, newCover };
    expect(added.filter(name => name.includes(outgoingCover)).length,
      `the outgoing picture is not fetched again (${added.join(', ')})`).toBe(0);
    expect(entriesAfter.some(name => name.includes(newCover)),
      'the new picture was requested once').toBe(true);

    // Three changes in a row: never more than two layers, one at the end.
    const rapid = await page.evaluate(async (indexes) => {
      const distinct = new Set();
      for (const index of indexes) {
        const titles = [...document.querySelectorAll('.track-row .track-title strong')];
        const at = titles.findIndex(node => node.textContent.trim() === index);
        if (at < 0) throw new Error(`no row named ${index}: ${titles.map(node => JSON.stringify(node.textContent)).join(', ')}`);
        document.querySelectorAll('.track-row .track-title')[at].click();
      }
      await window.dr03StopTheClock();
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
    await clickPausedRow(page, plain);
    await expect.poll(async () => (await layers()).count, { timeout: 10000 }).toBe(1);
    const placeholder = await page.evaluate(() => ({
      leaving: document.querySelectorAll('.cover-fade .cover-fade-layer.is-leaving').length,
      icons: document.querySelectorAll('.cover-fade .cover-fade-layer .artwork .vela-icon').length,
    }));
    evidence.journeys.placeholder = placeholder;
    expect(placeholder.leaving, 'no layer is left behind').toBe(0);
    expect(placeholder.icons, 'the placeholder is there').toBeGreaterThan(0);
    await page.screenshot({ path: `${output}/player-no-cover-1284x856.png` });

    await clickPausedRow(page, bright);
    // Two occurrences of the same track: the drawer keeps them apart. The
    // queue already holds the tracks the earlier journeys played, so the
    // count is measured before the two adds instead of being assumed.
    console.log('DR03 journey 6: the queue move');
    await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
    await expect(page.locator('#queue-drawer-dialog')).toBeVisible();
    const order = () => page.evaluate(QUEUE_ROWS);
    const brightTitle = tracks.find(track => track.title === 'Bright cover').title;
    const seeded = await order();
    const seededBright = seeded.filter(row => row.title === brightTitle).length;
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);
    await page.getByRole('button', { name: `Add ${bright} to playback queue`, exact: true }).click({ timeout: 20000 });
    await page.getByRole('button', { name: `Add ${bright} to playback queue`, exact: true }).click({ timeout: 20000 });
    await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
    await expect(page.locator('#queue-drawer-dialog')).toBeVisible();
    const beforeMove = await order();
    expect(beforeMove.length, 'two occurrences were added').toBe(seeded.length + 2);
    const duplicates = beforeMove.filter(row => row.title === brightTitle).length;
    expect(duplicates, 'the drawer keeps the occurrences apart').toBe(seededBright + 2);
    expect(beforeMove.length, 'the drawer still has rows to move').toBeGreaterThan(3);
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
    expect(settled.length, 'the move did not drop an occurrence').toBe(beforeMove.length);
    expect(settled.filter(row => row.title === brightTitle).length, 'the duplicate occurrences are still apart').toBe(duplicates);
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

    /* --- 6a. a keyboard move of two identical occurrences. */
    console.log('DR03 journey 6a: a move of two identical occurrences');
    // Two adjacent occurrences of the same track keep their markup: a row is
    // keyed by its position and its track, and both positions hold the same
    // track, so a swap leaves every node exactly where it was. The focus
    // then survives the move on the control of the position that now holds
    // the other occurrence, and the next keyboard command would act on the
    // wrong row unless the focus really moves to the destination.
    await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
    await expect(page.locator('#queue-drawer-dialog')).toBeVisible();
    const identical = await order();
    let pair = -1;
    for (let at = 1; at + 1 < identical.length; at += 1) {
      if (identical[at].title === identical[at + 1].title) {
        pair = at;
        break;
      }
    }
    expect(pair >= 1, `two adjacent identical occurrences (${identical.map(row => row.title).join(', ')})`).toBe(true);
    const from = pair + 1;
    await page.locator(`#queue-move-up-${from}`).focus();
    const keyboardBefore = await page.evaluate(() => ({
      id: document.activeElement?.id || '',
      row: document.activeElement?.closest('.queue-row')?.querySelector('.queue-track strong')?.textContent?.trim() || '',
    }));
    expect(keyboardBefore.id, 'the keyboard focuses the control of the second occurrence').toBe(`queue-move-up-${from}`);
    await page.keyboard.press('Enter');
    await expect.poll(async () => (await order()).some(row => row.settle), { timeout: 10000 }).toBe(false);
    const keyboardAfter = await page.evaluate(() => ({
      id: document.activeElement?.id || '',
      body: document.activeElement === document.body,
      row: document.activeElement?.closest('.queue-row')?.querySelector('.queue-track strong')?.textContent?.trim() || '',
    }));
    evidence.journeys.identicalFocus = { pair, before: keyboardBefore, after: keyboardAfter };
    expect(keyboardAfter.body, 'the focus is not parked on the document body').toBe(false);
    expect(
      keyboardAfter.id,
      'the focus moved to the control of the destination position, not the one it was left on',
    ).toBe(`queue-move-up-${pair}`);
    expect(keyboardAfter.row, 'the focus names the row that moved').toBe(keyboardBefore.row);
    // The focused control still names its own row after the move: the next
    // Enter acts on the row under the focus.
    const stillThere = await page.evaluate(id => document.getElementById(id)?.closest('.queue-row')
      ?.querySelector('.queue-track strong')?.textContent?.trim() || '', keyboardAfter.id);
    expect(stillThere, 'the focused control still names its own row').toBe(keyboardAfter.row);

    /* --- 6b. a move that lands on a boundary. */
    console.log('DR03 journey 6b: a move onto a boundary');
    // The second row moves into the first one: the "up" control of the new
    // first row is disabled, so the same control cannot take the focus. The
    // reader is not left on the body.
    const boundary = await order();
    expect(boundary.length, 'the drawer still holds the occurrences').toBe(seeded.length + 2);
    await page.locator('#queue-move-up-1').focus();
    await page.keyboard.press('Enter');
    await expect.poll(async () => (await order()).some(row => row.settle), { timeout: 10000 }).toBe(false);
    const atTop = await page.evaluate(() => ({
      id: document.activeElement?.id || '',
      body: document.activeElement === document.body,
      disabled: document.activeElement?.disabled ?? null,
      label: document.activeElement?.getAttribute('aria-label') || '',
    }));
    evidence.journeys.boundaryFocus = atTop;
    expect(atTop.body, 'the focus is not lost at the top boundary').toBe(false);
    expect(atTop.disabled, 'the focus is not on a disabled control').not.toBe(true);
    expect(atTop.id || atTop.label, 'the focus is on an honest neighbour').toBeTruthy();
    // And the same at the bottom: the row that moves into the last place
    // cannot be moved down any further, so the "down" control of the last
    // row is disabled and the honest neighbour is its "up" control. The last
    // index of the drawer is read, not assumed — the queue holds the
    // occurrences the earlier journeys played.
    const last = boundary.length - 1;
    await page.locator(`#queue-move-down-${last - 1}`).focus();
    await page.keyboard.press('Enter');
    await expect.poll(async () => (await order()).some(row => row.settle), { timeout: 10000 }).toBe(false);
    const atBottom = await page.evaluate(() => ({
      id: document.activeElement?.id || '',
      body: document.activeElement === document.body,
      disabled: document.activeElement?.disabled ?? null,
    }));
    evidence.journeys.bottomFocus = atBottom;
    expect(atBottom.body, 'the focus is not lost at the bottom boundary').toBe(false);
    expect(atBottom.disabled, 'the focus is not on a disabled control').not.toBe(true);
    expect(atBottom.id, 'the focus is on the honest neighbour of the last row')
      .toBe(`queue-move-up-${last}`);
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);

    console.log('DR03 journey 4a: one artwork, two tracks');
    // The second track with the first track's picture: the browser already
    // holds that picture, so the layer is ready at once and no request is
    // made. A ready layer that turned pending here would hide a real cover
    // behind a placeholder for good, because no second load ever comes.
    const beforeShared = await artworkEntries();
    await clickPausedRow(page, 'Shared picture');
    await coverReady(page);
    await expect(page.locator('.cover-fade .cover-fade-layer')).toHaveCount(1);
    await expect(page.locator('.cover-fade .cover-fade-layer')).toHaveCSS('opacity', '1');
    const sharedState = await page.evaluate(() => {
      const layer = document.querySelector('.cover-fade .cover-fade-layer');
      const image = layer?.querySelector('img');
      return {
        layers: document.querySelectorAll('.cover-fade .cover-fade-layer').length,
        leaving: document.querySelectorAll('.cover-fade .cover-fade-layer.is-leaving').length,
        classes: layer?.className || '',
        opacity: layer ? Number(getComputedStyle(layer).opacity) : null,
        complete: image ? image.complete && image.naturalWidth > 0 : null,
      };
    });
    const afterShared = await artworkEntries();
    evidence.journeys.sharedArtwork = { before: beforeShared.length, after: afterShared.length, sharedState };
    expect(sharedState.layers, 'one picture is one layer').toBe(1);
    expect(sharedState.leaving, 'nothing leaves for the same picture').toBe(0);
    expect(sharedState.classes, 'the loaded picture stays ready').toContain('is-ready');
    expect(sharedState.complete, 'the picture is really there').toBe(true);
    expect(sharedState.opacity, 'the picture is painted').toBeGreaterThan(0.9);
    // No new picture is asked for: the track that shares the artwork of the
    // one before it adds no artwork of its own, so the set of requested
    // pictures does not grow. (The same URL can be asked for again by a
    // cached element; the browser answers it from its own cache. What must
    // never happen is a fourth picture appearing, or the known one being
    // hidden behind a placeholder.)
    const distinctBefore = new Set(beforeShared).size;
    const distinctAfter = new Set(afterShared).size;
    expect(distinctAfter, `no new picture is requested (${distinctBefore} distinct before, ${distinctAfter} after)`).toBe(distinctBefore);

    /* --- 4b. a late answer of the layer that is already leaving. */
    console.log('DR03 journey 4b: the late answer of the outgoing layer');
    // The engines disagree about a warm picture, and that is why the first
    // round was not a measurement. Firefox answers a second element of an
    // already painted URL from its own image cache and never asks the
    // network — a route sees **0** requests whether it is installed before
    // or after the paint. Chromium asks the network again for the second
    // element, so a route installed **after** the paint sees **1** request
    // while a route installed before it sees **0** for the repeat (the
    // first answer is already in the memory cache). A hold installed late
    // therefore holds nothing in Firefox and one request in Chromium, and
    // the journey would pass or fail on cache luck. Measured with a real
    // HTTP server in both engines; the numbers are in docs/PROGRESS.md.
    // This journey runs in its own browser context —
    // an empty cache — and installs the hold before the page is opened, so
    // the first request for each of the two pictures really reaches the
    // route and every element that asks for it while it is in flight waits
    // with it.
    const lateTrack = tracks.find(track => track.title === late);
    const secondArtwork = tracks.find(track => track.title === second).artwork_id;
    const heldJourneys = {};
    /**
     * A context whose cache is empty, with the two pictures of the journey
     * held at the network layer from the first document on.
     */
    const heldPage = async ids => {
      const context = await browser.newContext({ viewport: { width: 1284, height: 856 } });
      const held = new Map();
      const state = { holding: true, context, page: await context.newPage() };
      state.page.setDefaultTimeout(20000);
      const hold = (map, request) => new Promise(resolve => {
        const id = request.url().split('/api/artwork/')[1] || '';
        const pending = map.get(id) || [];
        pending.push(resolve);
        map.set(id, pending);
      });
      // Only image loads are held: the same URL is also asked for by the
      // Media Session metadata, and holding that one would hold the player
      // instead of the picture. Every held load of one picture is kept, so a
      // release answers all of them.
      await state.page.route('**/api/artwork/**', async route => {
        const request = route.request();
        const id = request.url().split('/api/artwork/')[1] || '';
        if (!state.holding || request.resourceType() !== 'image' || !ids.has(id)) {
          await route.continue().catch(() => {});
          return;
        }
        await hold(held, request);
        await route.continue().catch(() => {});
      });
      state.release = id => {
        const pending = held.get(id) || [];
        held.delete(id);
        for (const resolve of pending) resolve();
      };
      state.pending = id => held.size > 0 && (held.get(id) || []).length;
      state.close = async () => {
        state.holding = false;
        for (const pending of held.values()) for (const resolve of pending) resolve();
        held.clear();
        await context.close();
      };
      return state;
    };
    /** Open the account in that context and land on the library. */
    const openLibrary = async heldState => {
      const heldDocument = heldState.page;
      await heldDocument.goto(runtime.origin);
      await expect(heldDocument.locator('.auth-card, .app-shell')).toBeVisible();
      if (await heldDocument.locator('.app-shell').count() === 0) {
        await heldDocument.locator('#username').fill('motion');
        await heldDocument.locator('#password').fill('private dr03 fixture password');
        await heldDocument.getByRole('button', { name: /sign in|create your music space/i }).first().click();
      }
      await expect(heldDocument.locator('.app-shell')).toBeVisible();
      await navigate(heldDocument, 'Library');
      await heldDocument.locator('.track-row').first().waitFor({ state: 'visible', timeout: 20000 });
    };

    /**
     * Stop the media clock of the held context.
     *
     * The two held journeys are about pictures, not about time: the fixture is
     * three seconds long and a picture that is held on purpose keeps its own
     * journey waiting. The root advances the cursor at natural EOF — by
     * contract — so a run that reaches the end of the track while the picture
     * is still held selects another occurrence for a reason that has nothing
     * to do with the crossfade, and the fade it was measuring is replaced
     * underneath it. The click is followed by the pause command so each
     * journey measures the picture it asked for. A pause during a held fade is
     * also the point of the package: the command answers while the effect
     * waits.
     */
    const clickHeldRow = async (heldState, title) => {
      await clickPausedRow(heldState.page, title);
    };

    const lateCase = await heldPage(new Set([secondArtwork, lateTrack.artwork_id]));
    try {
      await openLibrary(lateCase);
      // A is already on the screen, B is held, C is held: the reader asks for
      // C before B ever answered.
      await clickHeldRow(lateCase, bright);
      await expect.poll(() => lateCase.page.locator('.cover-fade .cover-fade-layer.is-ready').count(), { timeout: 20000 }).toBe(1);
      await clickHeldRow(lateCase, second);
      await expect.poll(() => lateCase.page.locator('.cover-fade .cover-fade-layer').count(), { timeout: 10000 }).toBe(2);
      await clickHeldRow(lateCase, late);
      const beforeRelease = await lateCase.page.evaluate(() => ({
        layers: [...document.querySelectorAll('.cover-fade .cover-fade-layer')].map(layer => layer.className),
        pending: document.querySelectorAll('.cover-fade .cover-fade-layer.is-pending').length,
        leaving: document.querySelectorAll('.cover-fade .cover-fade-layer.is-leaving').length,
      }));
      expect(lateCase.pending(secondArtwork), 'the outgoing picture is really held at the network layer').toBeGreaterThan(0);
      expect(lateCase.pending(lateTrack.artwork_id), 'the incoming picture is really held at the network layer').toBeGreaterThan(0);
      lateCase.release(secondArtwork);
      // The stale answer has to really arrive before it is asserted: the
      // answer of the picture that is already leaving is a real load, and a
      // fixed pause would read the state before it. The element itself says
      // when its picture is there; two frames let the load handler run.
      await lateCase.page.waitForFunction(id => {
        const images = [...document.querySelectorAll('.cover-fade .cover-fade-layer img')]
          .filter(image => (image.getAttribute('src') || '').includes(id));
        return images.length > 0 && images.every(image => image.complete && image.naturalWidth > 0);
      }, secondArtwork, { timeout: 10000 });
      await lateCase.page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))));
      const afterStale = await lateCase.page.evaluate(() => {
        const incoming = document.querySelector('.cover-fade .cover-fade-layer');
        const image = incoming?.querySelector('img');
        return {
          classes: incoming?.className || '',
          layers: document.querySelectorAll('.cover-fade .cover-fade-layer').length,
          leaving: document.querySelectorAll('.cover-fade .cover-fade-layer.is-leaving').length,
          opacity: incoming ? Number(getComputedStyle(incoming).opacity) : null,
          complete: image ? image.complete && image.naturalWidth > 0 : null,
        };
      });
      heldJourneys.lateAnswer = { beforeRelease, afterStale };
      expect(beforeRelease.pending, 'one picture waits: the one the reader asked for').toBe(1);
      expect(beforeRelease.leaving, 'nothing leaves before the new picture answers').toBe(0);
      expect(afterStale.classes, 'the answer of the leaving layer is not the arrival of the new one').toContain('is-pending');
      expect(afterStale.leaving, 'the leaving layer did not start to leave on somebody else answer').toBe(0);
      expect(afterStale.layers, 'two layers at most').toBeLessThanOrEqual(2);
      expect(afterStale.opacity, 'the incoming picture is not painted from a stale answer').toBe(0);
      expect(afterStale.complete, 'the incoming picture is really not there yet').toBe(false);
      lateCase.release(lateTrack.artwork_id);
      await expect.poll(() => lateCase.page.locator('.cover-fade .cover-fade-layer.is-ready').count(), { timeout: 10000 }).toBeGreaterThan(0);
      await expect.poll(() => lateCase.page.locator('.cover-fade .cover-fade-layer').count(), { timeout: 10000 }).toBe(1);
    } finally {
      await lateCase.close();
    }

    /* --- 4c. reduced motion arrives in the middle of a held fade. */
    console.log('DR03 journey 4c: reduce in the middle of a fade');
    const reduceCase = await heldPage(new Set([secondArtwork]));
    try {
      await openLibrary(reduceCase);
      await clickHeldRow(reduceCase, bright);
      await expect.poll(() => reduceCase.page.locator('.cover-fade .cover-fade-layer.is-ready').count(), { timeout: 20000 }).toBe(1);
      await clickHeldRow(reduceCase, second);
      await expect.poll(() => reduceCase.page.locator('.cover-fade .cover-fade-layer').count(), { timeout: 10000 }).toBe(2);
      expect(reduceCase.pending(secondArtwork), 'the fade is really held at the network layer').toBeGreaterThan(0);
      await reduceCase.page.emulateMedia({ reducedMotion: 'reduce' });
      // The surface answers the reader's request in the next render: the
      // outgoing layer is dropped, and nothing waits for a fade that no
      // longer runs.
      await expect.poll(() => reduceCase.page.evaluate(() => document.querySelectorAll('.cover-fade .cover-fade-layer').length), { timeout: 10000 }).toBe(1);
      const reducePending = await reduceCase.page.evaluate(() => ({
        layers: document.querySelectorAll('.cover-fade .cover-fade-layer').length,
        leaving: document.querySelectorAll('.cover-fade .cover-fade-layer.is-leaving').length,
        pending: document.querySelectorAll('.cover-fade .cover-fade-layer.is-pending').length,
        opacity: Number(getComputedStyle(document.querySelector('.cover-fade .cover-fade-layer')).opacity),
      }));
      reduceCase.release(secondArtwork);
      // The answer is a real load, so it is waited for instead of timed: a
      // fixed pause reads the layer before the browser has finished with it
      // and calls a slow answer a stuck one.
      await expect.poll(() => reduceCase.page.evaluate(() => document.querySelectorAll('.cover-fade .cover-fade-layer.is-pending').length), { timeout: 10000 }).toBe(0);
      const reduceSettled = await reduceCase.page.evaluate(() => ({
        layers: document.querySelectorAll('.cover-fade .cover-fade-layer').length,
        leaving: document.querySelectorAll('.cover-fade .cover-fade-layer.is-leaving').length,
        pending: document.querySelectorAll('.cover-fade .cover-fade-layer.is-pending').length,
      }));
      heldJourneys.reduceInFade = { reducePending, reduceSettled };
      expect(reducePending.layers, 'reduce drops the outgoing layer at once').toBe(1);
      expect(reducePending.leaving, 'nothing leaves under reduce').toBe(0);
      expect(reducePending.opacity, 'the picture is not hidden under reduce').toBe(1);
      expect(reduceSettled.layers, 'still one layer after the answer').toBe(1);
      expect(reduceSettled.leaving, 'still nothing leaving').toBe(0);
      expect(reduceSettled.pending, 'the picture that answered is not left pending').toBe(0);
    } finally {
      await reduceCase.close();
    }
    Object.assign(evidence.journeys, heldJourneys);


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
    // The controls still work. The label before the click is read, not
    // assumed: whether the audio is really playing at this point depends on
    // the output device of the host, and this check is about the control
    // answering, not about the sound.
    await page.keyboard.press('Escape');
    await expect(page.locator('.now-playing-dialog')).toHaveCount(0);
    const labelBefore = await page.locator('#player-toggle').getAttribute('aria-label');
    await page.locator('#player-toggle').click();
    await expect(page.locator('#player-toggle')).toHaveAttribute('aria-label', labelBefore === 'Play' ? 'Pause' : 'Play');
    await page.emulateMedia({ reducedMotion: 'no-preference' });

    await writeFile(join(output, 'dr03-motion.json'), JSON.stringify(evidence, null, 2));
  } finally {
    await runtime.close();
  }
});
