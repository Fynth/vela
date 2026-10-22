/**
 * DR04 density evidence: real 44px control targets, the desktop transport
 * geometry and the text-leaf table, measured from the running web app.
 *
 * One implementation backs both the standalone before/after capture and the
 * permanent DR04 Playwright spec, so the numbers written into the report and
 * the numbers the gate asserts can never drift apart.
 *
 * Method (the audit method the package inherits):
 *   - a target is measured with getBoundingClientRect(), never with a
 *     pseudo-element and never with a declared CSS value;
 *   - a target is "visible" when it has a non-empty rect, is not clipped away
 *     by the screen-reader utility class and is not `visibility:hidden`;
 *   - the text table collects only text-bearing leaves and h1-h3, and never
 *     an icon span or an SVG;
 *   - two 44px targets overlap only when their intersection area is > 0.
 */
import { chromium } from '@playwright/test';
import { mkdir, mkdtemp, open, rm, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';

const root = fileURLToPath(new URL('../../', import.meta.url));

async function isolatedServer(output) {
  const directory = await mkdtemp(join(tmpdir(), 'vela-dr04-'));
  const listener = createServer();
  await new Promise((done) => listener.listen(0, '127.0.0.1', done));
  const port = listener.address().port;
  await new Promise((done) => listener.close(done));
  const log = await open(join(output, 'server.log'), 'w', 0o600);
  const server = spawn(resolve(root, process.env.VELA_TEST_BINARY || 'target/debug/vela-server'), [], {
    cwd: root,
    stdio: ['ignore', log.fd, log.fd],
    env: {
      ...process.env,
      VELA_BIND: '127.0.0.1:' + port,
      VELA_DATA_DIR: join(directory, 'data'),
      VELA_WEB_DIR: resolve(root, process.env.VELA_TEST_WEB_DIR || 'dist'),
      VELA_SCAN_ROOTS: '[]',
    },
  });
  const exited = new Promise((done) => { server.once('exit', done); server.once('error', done); });
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

function pcmWave(duration, frequency) {
  const samples = 44100 * duration, wave = Buffer.alloc(44 + samples * 2);
  wave.write('RIFF', 0); wave.writeUInt32LE(36 + samples * 2, 4); wave.write('WAVEfmt ', 8);
  wave.writeUInt32LE(16, 16); wave.writeUInt16LE(1, 20); wave.writeUInt16LE(1, 22);
  wave.writeUInt32LE(44100, 24); wave.writeUInt32LE(88200, 28); wave.writeUInt16LE(2, 32);
  wave.writeUInt16LE(16, 34); wave.write('data', 36); wave.writeUInt32LE(samples * 2, 40);
  for (let at = 0; at < samples; at += 1) {
    wave.writeInt16LE(Math.round(1500 * Math.sin(2 * Math.PI * frequency * at / 44100)), 44 + at * 2);
  }
  return wave;
}

/** A real account with real tracks: no demo data, no fabricated catalog. */
export async function setupWithTracks(page, runtime, count = 3) {
  const setup = await page.request.post(runtime.origin + '/api/setup', {
    headers: { Origin: runtime.origin },
    data: { username: 'admin', password: 'private density fixture password' },
  });
  let me = null;
  if (setup.ok()) {
    me = await setup.json();
  } else if (setup.status() === 409) {
    // The account already exists on this server; the same session continues.
    const session = await page.request.get(runtime.origin + '/api/me');
    if (!session.ok()) throw new Error('setup conflict without a session: ' + session.status());
    me = await session.json();
  } else {
    throw new Error('setup failed: ' + setup.status());
  }
  let first = null;
  for (let at = 0; at < count; at += 1) {
    const upload = await page.request.post(runtime.origin + '/api/upload', {
      headers: { 'X-CSRF-Token': me.csrf_token, Origin: runtime.origin },
      multipart: {
        file: {
          name: `Density ${String(at + 1).padStart(2, '0')}.wav`,
          mimeType: 'audio/wav',
          buffer: pcmWave(4, 420 + at * 17.5),
        },
      },
    });
    if (!upload.ok()) throw new Error('upload failed: ' + upload.status());
    const track = (await upload.json()).track;
    if (!first) first = track;
  }
  return first;
}

export async function settle(page) {
  await page.locator('.app-shell').waitFor({ state: 'visible', timeout: 30000 });
  await page.evaluate(() => document.fonts.ready);
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
}

export async function navigate(page, name) {
  const mobile = page.getByRole('navigation', { name: 'Mobile navigation', exact: true });
  if (await mobile.isVisible()) {
    const tab = mobile.getByRole('button', { name, exact: true });
    if (await tab.count()) await tab.click();
    else {
      await mobile.getByRole('button', { name: 'More destinations', exact: true }).click();
      await page.getByRole('navigation', { name: 'More from Vela', exact: true })
        .getByRole('button', { name, exact: true }).click();
    }
  } else {
    await page.locator('.sidebar').getByRole('button', { name, exact: true }).click();
  }
}

/**
 * Runs inside the page. It reports every visible interactive target, the
 * text-bearing leaves and the geometry the package gates on.
 */
const COLLECT = () => {
  const TEXT_ROLES = ['h1', 'h2', 'h3'];
  const rect = (el) => {
    const r = el.getBoundingClientRect();
    return { x: +r.x.toFixed(2), y: +r.y.toFixed(2), width: +r.width.toFixed(2), height: +r.height.toFixed(2), bottom: +r.bottom.toFixed(2), right: +r.right.toFixed(2) };
  };
  const visible = (el) => {
    const style = getComputedStyle(el);
    if (style.visibility === 'hidden' || style.display === 'none') return false;
    if (el.closest('.sr-only') || el.classList.contains('sr-only')) return false;
    if (style.clipPath && style.clipPath.includes('inset(50%')) return false;
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0;
  };
  const describe = (el) => {
    const id = el.id ? '#' + el.id : '';
    const cls = typeof el.className === 'string' && el.className.trim()
      ? '.' + el.className.trim().split(/\s+/).join('.')
      : '';
    return el.tagName.toLowerCase() + id + cls;
  };
  const label = (el) => el.getAttribute('aria-label') || (el.textContent || '').trim().slice(0, 48) || describe(el);

  const interactive = [...document.querySelectorAll('button, a, input, select, [role="button"], summary')];
  const targets = interactive.filter(visible).map((el) => ({
    selector: describe(el),
    label: label(el),
    disabled: 'disabled' in el ? !!el.disabled : el.getAttribute('aria-disabled') === 'true',
    ...rect(el),
  }));

  const textLeaves = [];
  for (const el of document.querySelectorAll('body *')) {
    if (!visible(el)) continue;
    const tag = el.tagName.toLowerCase();
    if (tag === 'svg' || el.closest('svg')) continue;
    if (!TEXT_ROLES.includes(tag)) {
      // A leaf counts only when it directly holds text; an icon span and a
      // container that only wraps other elements never do.
      const own = [...el.childNodes].some((node) => node.nodeType === 3 && node.textContent.trim());
      if (!own) continue;
    }
    const style = getComputedStyle(el);
    const text = (el.textContent || '').trim();
    if (!text) continue;
    textLeaves.push({
      selector: describe(el),
      text: text.slice(0, 60),
      fontSize: style.fontSize,
      weight: style.fontWeight,
      ...rect(el),
    });
  }

  const seek = document.querySelector('.seek-control input[type=range]');
  const play = document.querySelector('#player-toggle');
  const transport = [...document.querySelectorAll('.player-buttons .icon-button, #player-toggle, .player-right .icon-button, .player-favorite')]
    .filter(visible).map((el) => ({ selector: describe(el), label: label(el), ...rect(el) }));
  const overlap = (a, b) => {
    const w = Math.min(a.right, b.right) - Math.max(a.x, b.x);
    const h = Math.min(a.bottom, b.bottom) - Math.max(a.y, b.y);
    return w > 0 && h > 0 ? +(w * h).toFixed(2) : 0;
  };
  const pairs = [];
  for (let i = 0; i < transport.length; i += 1) {
    for (let j = i + 1; j < transport.length; j += 1) {
      const area = overlap(transport[i], transport[j]);
      if (area > 0) pairs.push({ a: transport[i].label, b: transport[j].label, area });
    }
  }
  const seekRect = seek && visible(seek) ? rect(seek) : null;
  const playRect = play && visible(play) ? rect(play) : null;
  // The seek range is a target too: DV-15 measured it overlapping Play.
  const withSeek = seekRect ? [...transport, { selector: describe(seek), label: 'Playback position', ...seekRect }] : transport;
  const seekPairs = [];
  for (let i = 0; i < withSeek.length; i += 1) {
    for (let j = i + 1; j < withSeek.length; j += 1) {
      const area = overlap(withSeek[i], withSeek[j]);
      if (area > 0) seekPairs.push({ a: withSeek[i].label, b: withSeek[j].label, area });
    }
  }

  const bar = document.querySelector('.player-bar');
  const center = document.querySelector('.player-center');
  const buttons = document.querySelector('.player-buttons');
  const seekRow = document.querySelector('.seek-row');
  const rail = document.querySelector('.seek-rail');
  const face = document.querySelector('.play-toggle-face');
  const rowControls = [...document.querySelectorAll('.player-buttons > *')].filter(visible);
  const transportExtent = rowControls.length ? {
    count: rowControls.length,
    from: +rowControls[0].getBoundingClientRect().left.toFixed(2),
    to: +rowControls[rowControls.length - 1].getBoundingClientRect().right.toFixed(2),
    gap: rowControls.length > 1
      ? +(rowControls[1].getBoundingClientRect().left - rowControls[0].getBoundingClientRect().right).toFixed(2)
      : null,
  } : null;
  const navItems = [...document.querySelectorAll('.sidebar .nav-item')].filter(visible);
  const groupGaps = [];
  const sidebar = document.querySelector('.sidebar');
  if (sidebar) {
    const kids = [...sidebar.children].filter(visible);
    for (let i = 1; i < kids.length; i += 1) {
      const previous = kids[i - 1].getBoundingClientRect();
      const current = kids[i].getBoundingClientRect();
      groupGaps.push(+(current.top - previous.bottom).toFixed(2));
    }
  }

  return {
    // The app size is a CSS `zoom` on the document root, so every rectangle
    // below is reported in the zoomed coordinate space. The zoom factor is
    // reported next to them so a caller can read a CSS pixel.
    zoom: parseFloat(getComputedStyle(document.documentElement).zoom) || 1,
    targets,
    textLeaves,
    transport,
    transportOverlaps: pairs,
    transportOverlapsWithSeek: seekPairs,
    player: {
      bar: bar ? rect(bar) : null,
      barHeight: bar ? +bar.getBoundingClientRect().height.toFixed(2) : null,
      center: center ? rect(center) : null,
      buttons: buttons ? rect(buttons) : null,
      seekRow: seekRow ? rect(seekRow) : null,
      seekControl: document.querySelector('.seek-control') ? rect(document.querySelector('.seek-control')) : null,
      seekInput: seekRect,
      rail: rail ? rect(rail) : null,
      playButton: playRect,
      playFace: face ? rect(face) : null,
      playFaceSize: face ? +face.getBoundingClientRect().width.toFixed(2) : null,
      glyphs: [...document.querySelectorAll('.player-buttons .icon-button .vela-icon, .player-right .icon-button .vela-icon, .player-favorite .vela-icon')]
        .filter(visible).map((el) => ({ selector: describe(el), ...rect(el) })),
      volume: document.querySelector('.player-right input[type=range]') && visible(document.querySelector('.player-right input[type=range]'))
        ? rect(document.querySelector('.player-right input[type=range]')) : null,
      transportExtent,
      borderTopWidth: bar ? getComputedStyle(bar).borderTopWidth : null,
      hairline: bar ? getComputedStyle(bar, '::before').height : null,
    },
    sidebar: {
      navCount: navItems.length,
      navRects: navItems.map((el) => ({ label: label(el), ...rect(el) })),
      step: navItems.length > 1
        ? +(navItems[1].getBoundingClientRect().top - navItems[0].getBoundingClientRect().top).toFixed(2)
        : null,
      groupGaps,
      playlists: [...document.querySelectorAll('.sidebar-playlist')].filter(visible).map((el) => ({ label: label(el), ...rect(el) })),
    },
    overflow: {
      // Both overflows are measured inside one coordinate space: the document
      // against its own client width, the body against its own client width.
      // The body reports CSS pixels while the root reports the zoomed space,
      // so mixing them would fake an overflow at any app size other than 100%.
      document: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      body: document.body.scrollWidth - document.body.clientWidth,
      clientWidth: document.documentElement.clientWidth,
      innerWidth: window.innerWidth,
    },
  };
};

/** Focusable, real pointer and focus evidence for the desktop transport. */
const TRANSPORT_ACTIONS = () => {
  const results = [];
  const controls = [...document.querySelectorAll('.player-buttons .icon-button, #player-toggle, .player-right .icon-button, .player-favorite')];
  for (const el of controls) {
    const rect = el.getBoundingClientRect();
    const shown = rect.width > 0 && rect.height > 0 && getComputedStyle(el).display !== 'none';
    if (!shown) { results.push({ label: el.getAttribute('aria-label'), skipped: 'not rendered' }); continue; }
    if (el.disabled) { results.push({ label: el.getAttribute('aria-label'), skipped: 'disabled' }); continue; }
    el.focus();
    const active = document.activeElement === el;
    const visible = el.matches(':focus-visible');
    const style = getComputedStyle(el);
    results.push({
      label: el.getAttribute('aria-label'),
      focused: active,
      focusVisible: visible,
      outlineWidth: style.outlineWidth,
    });
  }
  return results;
};

export async function measure(page) {
  return page.evaluate(COLLECT);
}

export async function transportActions(page) {
  // A real keypress first, so the browser's focus-visible modality is the
  // keyboard one and the ring is the ring a keyboard user actually sees.
  await page.keyboard.press('Tab');
  return page.evaluate(TRANSPORT_ACTIONS);
}

/** A real pointer action on the seek rail: the position must move. */
export async function pointerSeek(page, fraction) {
  const range = page.locator('.seek-control input[type=range]');
  const box = await range.boundingBox();
  if (!box) return null;
  const before = await range.inputValue();
  const max = await range.getAttribute('max');
  await page.mouse.click(box.x + box.width * fraction, box.y + box.height / 2);
  await page.waitForTimeout(250);
  const after = await range.inputValue();
  return { before, after, max: Number(max), box };
}

export async function startServer(output) {
  await mkdir(output, { recursive: true });
  const runtime = await isolatedServer(output);
  let lastError = '';
  const healthy = async () => {
    try {
      const response = await fetch(runtime.origin + '/health');
      return response.ok;
    } catch (error) {
      lastError = error && error.message ? error.message : String(error);
      return false;
    }
  };
  for (let at = 0; at < 60 && !(await healthy()); at += 1) await delay(250);
  if (!(await healthy())) throw new Error(`server did not become healthy at ${runtime.origin}: ${lastError}`);
  return runtime;
}

export async function capture({ output, label, viewports, zoom = 1, withTrack = true }) {
  const runtime = await startServer(output);
  const browser = await chromium.launch();
  const report = { label, zoom, viewports: [] };
  try {
    const context = await browser.newContext({ viewport: { width: 1284, height: 856 } });
    const page = await context.newPage();
    await setupWithTracks(page, runtime, 3);
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible' });
    if (withTrack) {
      const title = await page.locator('.track-row .track-title strong').first().innerText();
      await page.getByRole('button', { name: `Play ${title}`, exact: true }).click();
      await page.waitForTimeout(600);
    }
    for (const viewport of viewports) {
      await page.setViewportSize(viewport);
      await page.evaluate((value) => { document.documentElement.style.setProperty('--app-zoom', String(value)); }, zoom);
      await page.waitForTimeout(350);
      const measured = await measure(page);
      report.viewports.push({ viewport, zoom, ...measured });
    }
    await context.close();
  } finally {
    await browser.close();
    await runtime.close();
  }
  await writeFile(join(output, `dr04-density-${label}.json`), JSON.stringify(report, null, 2));
  return report;
}

if (import.meta.url === pathToFileURL(process.argv[1] || '').href) {
  const output = resolve(root, process.env.VELA_DENSITY_EVIDENCE || 'target/checks/dr04-density');
  const label = process.argv[2] || 'capture';
  const report = await capture({
    output,
    label,
    viewports: [
      { width: 1284, height: 856 },
      { width: 1440, height: 900 },
      { width: 1728, height: 1000 },
      { width: 768, height: 1024 },
      { width: 390, height: 844 },
      { width: 320, height: 700 },
    ],
  });
  const at = report.viewports[0];
  console.log('label', report.label);
  console.log('small targets at 1284:', at.targets.filter((t) => t.width < 44 || t.height < 44).length);
  console.log('transport overlaps:', JSON.stringify(at.transportOverlaps));
  console.log('player bar height:', at.player.barHeight, 'border-top:', at.player.borderTopWidth, 'hairline:', at.player.hairline);
  console.log('nav step:', at.sidebar.step, 'nav count:', at.sidebar.navCount, 'group gaps:', JSON.stringify(at.sidebar.groupGaps));
  console.log('written to', join(output, `dr04-density-${label}.json`));
}
