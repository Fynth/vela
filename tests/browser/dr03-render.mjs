/**
 * DR-03 real render: the surfaces this package changed, in the sizes, the
 * themes, the motion preferences and the content shapes the plan asks for.
 *
 * The matrix is the one in the mandate: 320×700, 360×800, 390×844, 768×1024,
 * 1284×856, 1440×900, 1728×1000; dark and light; reduced motion; a long
 * Cyrillic title; an empty library; a track without a cover. Every artifact
 * name carries the theme, the viewport and the engine it was really taken in.
 *
 * The pictures are of the real application with real uploaded originals, and
 * every screenshot is taken after the state it shows was reached by a real
 * command — a click on the row, the menu trigger, the queue trigger.
 */
import { chromium, firefox } from 'playwright';
import { mkdir, readFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { deflateSync } from 'node:zlib';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { settle, navigate, startServer } from './dr04-density.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const brightCover = resolve(root, 'crates/media/tests/fixtures/bright-cover.flac');

const VIEWPORTS = [
  { width: 320, height: 700 },
  { width: 360, height: 800 },
  { width: 390, height: 844 },
  { width: 768, height: 1024 },
  { width: 1284, height: 856 },
  { width: 1440, height: 900 },
  { width: 1728, height: 1000 },
];
const THEMES = ['dark', 'light'];

/** The transport label, waited for without pulling `expect` into the probe. */
async function expectLabel(page, label) {
  await page.waitForFunction(value => document.querySelector('#player-toggle')
    ?.getAttribute('aria-label') === value, label, { timeout: 5000 });
}

/** Freeze the real media clock before photographing a selected picture or title. */
async function pausePlayback(page) {
  const toggle = page.locator('#player-toggle');
  await toggle.waitFor({ state: 'visible' });
  if (await toggle.getAttribute('aria-label') === 'Pause') await toggle.click();
  await expectLabel(page, 'Play');
  await page.waitForFunction(() => document.querySelector('#vela-audio')?.paused === true);
}

/** A real PNG of one solid colour, for the second cover. */
function solidPng(size, [red, green, blue]) {
  const table = (() => {
    const values = new Int32Array(256);
    for (let n = 0; n < 256; n += 1) {
      let c = n;
      for (let step = 0; step < 8; step += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      values[n] = c;
    }
    return values;
  })();
  const crc = buffer => {
    let c = -1;
    for (const byte of buffer) c = table[(c ^ byte) & 0xff] ^ (c >>> 8);
    return (c ^ -1) >>> 0;
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
  for (let row = 0; row < size; row += 1) {
    raw[row * (size * 3 + 1)] = 0;
    for (let column = 0; column < size; column += 1) {
      const at = row * (size * 3 + 1) + 1 + column * 3;
      raw[at] = red; raw[at + 1] = green; raw[at + 2] = blue;
    }
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', header),
    chunk('IDAT', deflateSync(raw)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

/** A real original without any picture block: a plain 4-second WAV. */
function plainWave() {
  const samples = 44100 * 4;
  const wave = Buffer.alloc(44 + samples * 2);
  wave.write('RIFF', 0); wave.writeUInt32LE(36 + samples * 2, 4); wave.write('WAVEfmt ', 8);
  wave.writeUInt16LE(16, 16); wave.writeUInt16LE(1, 20); wave.writeUInt16LE(1, 22);
  wave.writeUInt32LE(44100, 24); wave.writeUInt32LE(88200, 28); wave.writeUInt16LE(2, 32);
  wave.writeUInt16LE(16, 34); wave.write('data', 36); wave.writeUInt32LE(samples * 2, 40);
  for (let sample = 0; sample < samples; sample += 1) {
    wave.writeInt16LE(Math.round(2000 * Math.sin(2 * Math.PI * 220 * sample / 44100)), 44 + sample * 2);
  }
  return wave;
}

/** The tracked fixture with its picture replaced and its title rewritten. */
async function rewrite({ png, title }) {
  const flac = await readFile(brightCover);
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
  const image = png || await readFile(resolve(root, 'crates/media/tests/fixtures/cover.png'));
  const mime = Buffer.from('image/png', 'utf8');
  const mimeField = Buffer.alloc(4);
  mimeField.writeUInt32BE(mime.length, 0);
  const dimensions = Buffer.alloc(16);
  dimensions.writeUInt32BE(image.readUInt32BE(16), 0);
  dimensions.writeUInt32BE(image.readUInt32BE(20), 4);
  dimensions.writeUInt32BE(24, 8);
  dimensions.writeUInt32BE(0, 12);
  const dataLength = Buffer.alloc(4);
  dataLength.writeUInt32BE(image.length, 0);
  const block = Buffer.concat([
    Buffer.from([0, 0, 0, 3]), mimeField, mime, Buffer.alloc(4), dimensions, dataLength, image,
  ]);
  let out = Buffer.concat([
    flac.subarray(0, picture.start),
    Buffer.from([0x06, (block.length >> 16) & 0xff, (block.length >> 8) & 0xff, block.length & 0xff]),
    block,
    flac.subarray(picture.start + 4 + picture.length),
  ]);
  if (title) {
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
    const fields = [`TITLE=${title}`, 'ARTIST=Motion fixtures'];
    count.writeUInt32LE(fields.length, 0);
    parts.push(count);
    for (const field of fields) {
      const bytes = Buffer.from(field, 'utf8');
      const size = Buffer.alloc(4);
      size.writeUInt32LE(bytes.length, 0);
      parts.push(size, bytes);
    }
    const body = Buffer.concat(parts);
    out = Buffer.concat([
      out.subarray(0, comment.start),
      Buffer.from([0x04, (body.length >> 16) & 0xff, (body.length >> 8) & 0xff, body.length & 0xff]),
      body,
      out.subarray(comment.start + 4 + comment.length),
    ]);
  }
  return out;
}

/** A real account: a cover, a second cover, a long Cyrillic title, no cover. */
async function account(page, runtime, { empty = false } = {}) {
  const setup = await page.request.post(runtime.origin + '/api/setup', {
    headers: { Origin: runtime.origin },
    data: { username: 'render', password: 'private render password' },
  });
  if (!setup.ok() && setup.status() !== 409) throw new Error('setup failed: ' + setup.status());
  const me = setup.ok() ? await setup.json() : await (await page.request.get(runtime.origin + '/api/me')).json();
  const headers = { 'X-CSRF-Token': me.csrf_token, Origin: runtime.origin };
  if (empty) return [];
  const uploads = [
    { name: 'Bright cover.flac', mimeType: 'audio/flac', buffer: await readFile(brightCover) },
    { name: 'Second cover.flac', mimeType: 'audio/flac', buffer: await rewrite({ png: solidPng(64, [200, 40, 90]), title: 'Второй рассвет над длинной рекой' }) },
    // A real original with no picture block at all. Rewriting a FLAC keeps
    // the picture it already has, so the previous version of this fixture
    // uploaded a cover and photographed it as "no cover".
    { name: 'No cover.wav', mimeType: 'audio/wav', buffer: plainWave() },
  ];
  const tracks = [];
  for (const upload of uploads) {
    const response = await page.request.post(runtime.origin + '/api/upload', {
      headers, multipart: { file: { name: upload.name, mimeType: upload.mimeType, buffer: upload.buffer } },
    });
    if (!response.ok()) throw new Error('upload failed: ' + response.status());
    tracks.push((await response.json()).track);
  }
  // The "no cover" state is only real when the server really stored no
  // picture: a fixture that silently gains a picture block would turn the
  // shot into a shot of a cover, and the DOM check later would only see the
  // consequence. The upload answer is the first place it can be caught.
  const uncovered = tracks.find(track => track.title === 'No cover');
  if (!uncovered) throw new Error('the no-cover fixture did not upload');
  if (uncovered.artwork_id) {
    throw new Error('the "no cover" original was stored with an artwork: ' + uncovered.artwork_id);
  }
  return tracks;
}

/**
 * Wait until the surface is settled: its own animations are over, so the
 * picture is of the state the reader is left with.
 */
async function settled(page, selector) {
  await page.waitForFunction(sel => {
    const element = document.querySelector(sel);
    if (!element) return false;
    return element.getAnimations({ subtree: false }).every(animation => animation.playState === 'finished');
  }, selector, { timeout: 5000 }).catch(() => {});
  await page.waitForTimeout(180);
}

const state = () => ({
  layers: document.querySelectorAll('.cover-fade .cover-fade-layer').length,
  leaving: document.querySelectorAll('.cover-fade .cover-fade-layer.is-leaving').length,
  ready: document.querySelectorAll('.cover-fade .cover-fade-layer.is-ready').length,
  pending: document.querySelectorAll('.cover-fade .cover-fade-layer.is-pending').length,
  menu: document.querySelectorAll('.track-menu-panel').length,
  dialog: document.querySelectorAll('.now-playing-dialog').length,
  audio: document.querySelectorAll('audio').length,
  overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
});

/** Wait until the crossfade surface holds one layer and nothing is leaving. */
async function settledFade(page) {
  for (let attempt = 0; attempt < 200; attempt += 1) {
    const state = await page.evaluate(() => ({
      layers: document.querySelectorAll('.cover-fade .cover-fade-layer').length,
      leaving: document.querySelectorAll('.cover-fade .cover-fade-layer.is-leaving').length,
      pending: document.querySelectorAll('.cover-fade .cover-fade-layer.is-pending').length,
    }));
    if (state.layers <= 1 && state.leaving === 0 && state.pending === 0) return state;
    await page.waitForTimeout(50);
  }
  throw new Error('the crossfade surface never settled');
}

/**
 * One engine, one theme, the whole matrix.
 *
 * The pictures of a state are taken after the state was reached: the row was
 * clicked, the menu was opened, the queue drawer was opened, the sheet was
 * opened. Nothing is painted by the script.
 */
async function render({ engineName, engine, output, theme, reduced }) {
  const runtime = await startServer(output);
  const browser = await engine.launch();
  const report = { engine: engineName, theme, reduced, viewports: [], empty: null, longTitle: null };
  try {
    const context = await browser.newContext({ viewport: { width: 1284, height: 856 }, reducedMotion: reduced ? 'reduce' : 'no-preference' });
    const page = await context.newPage();
    page.setDefaultTimeout(20000);
    const tracks = await account(page, runtime);
    await page.goto(runtime.origin);
    await settle(page);
    if (await page.locator('.auth-card').count()) {
      await page.locator('#username').fill('render');
      await page.locator('#password').fill('private render password');
      await page.getByRole('button', { name: /sign in|create your music space/i }).first().click();
      await settle(page);
    }
    // The theme is switched by the application's own control, not by a hand
    // written attribute: the picture has to show what a reader really gets.
    const current = await page.evaluate(() => document.documentElement.dataset.theme || 'dark');
    if (current !== theme) {
      await page.getByRole('button', { name: 'Change color theme' }).click();
      await page.waitForTimeout(250);
    }
    const applied = await page.evaluate(() => document.documentElement.dataset.theme || 'dark');
    if (applied !== theme) throw new Error(`the theme did not change: ${applied} != ${theme}`);
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible' });

    const rowFor = title => page.locator('.track-row').filter({
      has: page.locator('.track-title strong').filter({ hasText: new RegExp(`^\\s*${title}\\s*$`) }),
    });

    // The player holds a real cover, so the crossfade has something to fade.
    await rowFor('Bright cover').locator('.track-title').click();
    await page.locator('.cover-fade .cover-fade-layer.is-ready').first().waitFor({ state: 'visible' });
    // The matrix is about surfaces, not about the media clock: the fixture is
    // three seconds long and a matrix run takes longer than that. Left playing,
    // the track reaches natural EOF while the loop is still taking pictures,
    // the root advances the cursor by contract, and any shot after that moment
    // is a shot of a fade in progress rather than of the state that was asked
    // for (measured: one viewport read 2 layers, 1 leaving). Playback is paused
    // here so every shot in the matrix shows the same, settled surface.
    await pausePlayback(page);

    for (const viewport of VIEWPORTS) {
      await page.setViewportSize(viewport);
      await page.waitForTimeout(300);
      const name = `${engineName}-${theme}${reduced ? '-reduce' : ''}-${viewport.width}x${viewport.height}`;
      await page.screenshot({ path: join(output, `library-${name}.png`) });
      // The menu, opened by its own trigger.
      await rowFor('Bright cover').locator('button[aria-haspopup="menu"]').click();
      await page.locator('.track-menu-panel').waitFor({ state: 'visible' });
      // The picture is of the settled surface, not of the first frame of an
      // entrance: the effect is 120ms and an early shot would photograph the
      // scrim over an almost invisible panel.
      await settled(page, '.track-menu-panel');
      await page.screenshot({ path: join(output, `menu-${name}.png`) });
      await page.keyboard.press('Escape');
      // The queue drawer, opened by its own trigger, with the current
      // occurrence and two more rows to move.
      // The transport keeps one queue trigger in both layouts, with its own
      // label: the mobile one also owns the transport it uncovers.
      await page.locator('button[aria-label^="Show playback queue"]:visible').first().click();
      await page.locator('#queue-drawer-dialog').waitFor({ state: 'visible' });
      await settled(page, '#queue-drawer-dialog .queue-panel');
      await page.screenshot({ path: join(output, `queue-${name}.png`) });
      await page.keyboard.press('Escape');
      await page.locator('#queue-drawer-dialog').waitFor({ state: 'detached' });
      // Read the state of a settled surface: the effect of the last
      // navigation is over, so a second layer in the report would be a real
      // fault, not a photograph of a fade in flight.
      await settledFade(page);
      report.viewports.push({ viewport, name, state: await page.evaluate(state) });
    }

    // A track without a cover: the placeholder is the honest face. The
    // state is asserted before the picture, because a photograph of a state
    // nobody checked is not evidence of it.
    await page.setViewportSize({ width: 1284, height: 856 });
    await rowFor('No cover').locator('.track-title').click();
    await page.waitForFunction(() => document.querySelector('.player-title')?.textContent.trim() === 'No cover');
    await pausePlayback(page);
    await settledFade(page);
    report.noCover = await page.evaluate(state);
    report.noCoverFace = await page.evaluate(() => ({
      images: document.querySelectorAll('.cover-fade .cover-fade-layer img').length,
      icons: document.querySelectorAll('.cover-fade .cover-fade-layer .vela-icon').length,
    }));
    if (report.noCoverFace.images !== 0 || report.noCoverFace.icons < 1) {
      throw new Error('the "no cover" track really has a cover: ' + JSON.stringify(report.noCoverFace));
    }
    await page.screenshot({ path: join(output, `player-no-cover-${engineName}-${theme}${reduced ? '-reduce' : ''}.png`) });

    // The long Cyrillic title in the player and in the queue.
    await rowFor('Второй рассвет над длинной рекой').locator('.track-title').click();
    await page.waitForFunction(() => document.querySelector('.player-title')?.textContent.trim() === 'Второй рассвет над длинной рекой');
    await pausePlayback(page);
    await settledFade(page);
    await page.screenshot({ path: join(output, `long-cyrillic-${engineName}-${theme}${reduced ? '-reduce' : ''}.png`) });
    report.longTitle = await page.evaluate(() => {
      const title = document.querySelector('.player-title, .player-track strong, .player strong');
      const rect = title?.getBoundingClientRect();
      return {
        text: title?.textContent?.trim() || '',
        width: rect ? +rect.width.toFixed(1) : null,
        overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      };
    });
    await context.close();

    // An empty library: the honest empty state, in the same theme. It needs
    // its own server, because this one already holds the account and its
    // originals; an empty library is a real state of a real account, not a
    // styled page.
    const emptyRuntime = await startServer(join(output, `empty-${engineName}-${theme}`));
    const emptyContext = await browser.newContext({ viewport: { width: 1284, height: 856 }, reducedMotion: reduced ? 'reduce' : 'no-preference' });
    const emptyPage = await emptyContext.newPage();
    emptyPage.setDefaultTimeout(20000);
    try {
      const emptySetup = await emptyPage.request.post(emptyRuntime.origin + '/api/setup', {
        headers: { Origin: emptyRuntime.origin },
        data: { username: 'empty', password: 'private empty password' },
      });
      if (!emptySetup.ok()) throw new Error('empty setup failed: ' + emptySetup.status());
      await emptyPage.goto(emptyRuntime.origin);
      await settle(emptyPage);
      if (await emptyPage.locator('.auth-card').count()) {
        await emptyPage.locator('#username').fill('empty');
        await emptyPage.locator('#password').fill('private empty password');
        await emptyPage.getByRole('button', { name: /sign in|create your music space/i }).first().click();
        await settle(emptyPage);
      }
      const emptyTheme = await emptyPage.evaluate(() => document.documentElement.dataset.theme || 'dark');
      if (emptyTheme !== theme) {
        await emptyPage.getByRole('button', { name: 'Change color theme' }).click();
        await emptyPage.waitForTimeout(250);
      }
      const appliedEmpty = await emptyPage.evaluate(() => document.documentElement.dataset.theme || 'dark');
      if (appliedEmpty !== theme) throw new Error(`the empty theme did not change: ${appliedEmpty} != ${theme}`);
      await navigate(emptyPage, 'Library');
      await emptyPage.waitForTimeout(500);
      for (const viewport of [{ width: 1284, height: 856 }, { width: 390, height: 844 }]) {
        await emptyPage.setViewportSize(viewport);
        await emptyPage.waitForTimeout(300);
        await emptyPage.screenshot({ path: join(output, `empty-${engineName}-${theme}${reduced ? '-reduce' : ''}-${viewport.width}x${viewport.height}.png`) });
      }
      report.empty = await emptyPage.evaluate(state);
    } finally {
      await emptyContext.close();
      await emptyRuntime.close();
    }
  } finally {
    await browser.close();
    await runtime.close();
  }
  return report;
}

if (import.meta.url === pathToFileURL(process.argv[1] || '').href) {
  const engineName = process.argv[2] || 'chromium';
  const engine = engineName === 'firefox' ? firefox : chromium;
  const output = resolve(root, process.env.VELA_DR03_RENDER || `target/checks/dr03-motion/render-${engineName}`);
  await mkdir(output, { recursive: true });
  const reports = [];
  for (const reduced of [false, true]) {
    for (const theme of THEMES) {
      reports.push(await render({ engineName, engine, output, theme, reduced }));
    }
  }
  const { writeFile } = await import('node:fs/promises');
  await writeFile(join(output, `render-${engineName}.json`), JSON.stringify(reports, null, 2));
  for (const report of reports) {
    console.log(`${report.engine} ${report.theme}${report.reduced ? ' reduced' : ''}: viewports ${report.viewports.length}, layers ` +
      `${report.viewports.map(view => view.state.layers).join('/')}, overflow ` +
      `${report.viewports.map(view => view.state.overflow).join('/')}${report.noCover ? `, no-cover layers ${report.noCover.layers} leaving ${report.noCover.leaving}` : ''}`);
  }
  console.log('written to', output);
}
