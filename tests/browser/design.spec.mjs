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

// This visual journey owns its server/data/sessions: authentication regressions
// in music.spec.mjs revoke shared sessions and deliberately retain login limits.
async function isolatedServer(output) {
  const directory = await mkdtemp(join(tmpdir(), 'vela-design-'));
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
      await page.getByRole('dialog', { name: 'More from your library', exact: true }).getByRole('button', { name, exact: true }).click();
    }
  } else await page.getByRole('button', { name, exact: true }).first().click();
}
function pcmWave(duration, frequency) {
  const samples = 44100 * duration, wave = Buffer.alloc(44 + samples * 2);
  wave.write('RIFF', 0); wave.writeUInt32LE(36 + samples * 2, 4); wave.write('WAVEfmt ', 8);
  wave.writeUInt32LE(16, 16); wave.writeUInt16LE(1, 20); wave.writeUInt16LE(1, 22);
  wave.writeUInt32LE(44100, 24); wave.writeUInt32LE(88200, 28); wave.writeUInt16LE(2, 32);
  wave.writeUInt16LE(16, 34); wave.write('data', 36); wave.writeUInt32LE(samples * 2, 40);
  for (let at = 0; at < samples; at++) wave.writeInt16LE(Math.round(1500 * Math.sin(2 * Math.PI * frequency * at / 44100)), 44 + at * 2);
  return wave;
}

test('DESIGN01 high contrast, responsive music surfaces and navigation retain actual playback', async ({ page }) => {
  test.setTimeout(90000);
  const output = process.env.VELA_DESIGN_EVIDENCE || 'test-results/design-contrast';
  await mkdir(output, { recursive: true });
  const runtime = await isolatedServer(output);
  try {
    await expect.poll(async () => {
      try { return (await page.request.get(runtime.origin + '/health', { timeout: 1000 })).ok(); }
      catch { return false; }
    }).toBeTruthy();
    const setup = await page.request.post(runtime.origin + '/api/setup', {
      headers: { Origin: runtime.origin }, data: { username: 'admin', password: 'private design fixture password' },
    });
    expect(setup.ok()).toBeTruthy();
    const me = await setup.json();
    const headers = { 'X-CSRF-Token': me.csrf_token, Origin: runtime.origin };
    const signal = await page.request.post(runtime.origin + '/api/upload', {
      headers, multipart: { file: { name: 'signal.flac', mimeType: 'audio/flac', buffer: await readFile(resolve(root, 'crates/media/tests/fixtures/signal.flac')) } },
    });
    expect(signal.ok()).toBeTruthy();
    const signalTrack = (await signal.json()).track;
    const created = await page.request.post(runtime.origin + '/api/playlists', { headers, data: { name: 'Evening signals' } });
    expect(created.ok()).toBeTruthy();
    const playlist = await created.json();
    const added = await page.request.post(runtime.origin + '/api/playlists/' + playlist.id + '/items', {
      headers, data: { revision: playlist.revision, mutation_id: randomUUID(), track_id: signalTrack.id },
    });
    expect(added.ok()).toBeTruthy();
    const upload = await page.request.post(runtime.origin + '/api/upload', {
      headers: { 'X-CSRF-Token': me.csrf_token },
      multipart: { file: { name: 'Contrast session.wav', mimeType: 'audio/wav', buffer: pcmWave(30, 740.25) } },
    });
    expect(upload.ok()).toBeTruthy();
    const uploaded = await upload.json();
    await page.goto(runtime.origin);
    await navigate(page, 'Library');
    await page.getByRole('button', { name: `Play ${uploaded.track.title}`, exact: true }).click();
    await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
    const source = await page.locator('#vela-audio').evaluate(audio => {
      audio.dataset.designIdentity = 'same-authoritative-player';
      return audio.currentSrc;
    });
    await navigate(page, 'Home');
    await navigate(page, 'Library');
    await expect(page.locator('#vela-audio')).toHaveAttribute('data-design-identity', 'same-authoritative-player');
    expect(await page.locator('#vela-audio').evaluate(audio => audio.currentSrc)).toBe(source);
    await page.getByRole('button', { name: 'Pause', exact: true }).click();
    await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', true);

    // A short desktop window must scroll the navigation, not paint groups over
    // each other or hide the final settings destination behind a fixed player.
    await page.setViewportSize({ width: 1440, height: 720 });
    const sidebar = page.locator('.sidebar');
    const libraryGroup = await sidebar.locator('.sidebar-library').boundingBox();
    const utilityGroup = await sidebar.locator('.sidebar-bottom').boundingBox();
    expect(libraryGroup.y + libraryGroup.height).toBeLessThanOrEqual(utilityGroup.y);
    await sidebar.getByRole('button', { name: 'Settings', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Settings', exact: true })).toBeVisible();
    await navigate(page, 'Library');
    await sidebar.evaluate(node => { node.scrollTop = 0; });

    const capture = process.env.VELA_DESIGN_CAPTURE_SCRIPT
      ? await readFile(process.env.VELA_DESIGN_CAPTURE_SCRIPT, 'utf8') : null;
    const states = [];
    const measurements = [];
    for (const width of [390, 1440, 1728, 320]) {
      await page.setViewportSize({ width, height: 900 });
      for (const theme of ['dark', 'light']) {
        if (await page.locator('html').getAttribute('data-theme') !== theme) {
          await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
        }
        await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
        await page.evaluate(() => document.fonts.ready);
        await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
        const measured = await page.evaluate(() => {
          const rgb = value => {
            const parts = value.match(/[\d.]+/g)?.map(Number) || [];
            return [parts[0] || 0, parts[1] || 0, parts[2] || 0, parts[3] ?? 1];
          };
          const background = element => {
            for (let parent = element; parent; parent = parent.parentElement) {
              const color = rgb(getComputedStyle(parent).backgroundColor);
              if (color[3] === 1) return color;
            }
            return rgb(getComputedStyle(document.body).backgroundColor);
          };
          const luminance = channels => channels.slice(0, 3).map(value => value / 255)
            .map(value => value <= .04045 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4)
            .reduce((total, value, at) => total + value * [.2126, .7152, .0722][at], 0);
          const readings = [];
          for (const selector of ['.track-title strong', '.track-title small', '.track-duration', '.nav-item.active', '.button.primary', '.now-playing strong']) {
            for (const element of document.querySelectorAll(selector)) {
              if (!element.checkVisibility() || element.matches(':disabled')) continue;
              const style = getComputedStyle(element);
              const a = luminance(rgb(style.color));
              const b = luminance(background(element));
              readings.push({ selector, label: element.textContent.trim().slice(0, 60), foreground: style.color, background: background(element), ratio: (Math.max(a, b) + .05) / (Math.min(a, b) + .05), font: parseFloat(style.fontSize) });
            }
          }
          return { width: innerWidth, overflow: document.documentElement.scrollWidth - innerWidth, readings };
        });
        measurements.push({ theme, ...measured });
        await writeFile(`${output}/contrast.json`, JSON.stringify(measurements, null, 2));
        expect.soft(measured.overflow, `${width}px ${theme} document overflow`).toBeLessThanOrEqual(3);
        expect((await page.locator('#player-toggle').boundingBox()).height).toBeGreaterThanOrEqual(44);
        expect((await page.locator('.seek-row input').boundingBox()).height).toBeGreaterThanOrEqual(44);
        for (const reading of measured.readings) {
          expect.soft(reading.ratio, `${width}px ${theme} ${reading.selector} contrast`).toBeGreaterThanOrEqual(4.5);
          expect.soft(reading.font, `${width}px ${theme} ${reading.selector} legibility`).toBeGreaterThanOrEqual(12);
        }
        await page.screenshot({ path: `${output}/library-${width}-${theme}.png`, animations: 'disabled' });
        if (capture && width !== 320) {
          // Screen-reader-only labels are intentionally clipped, not visual layout failures.
          await page.locator('.sr-only').evaluateAll(nodes => nodes.forEach((node, at) => { node.dataset.bdInspectId = `visually-hidden-${at}`; }));
          const state = await page.evaluate(capture);
          state.elements = state.elements.filter(element => !element.id.startsWith('visually-hidden-'));
          state.name = `library-${width}-${theme}`;
          states.push(state);
        }
        if (width === 390 || width === 1440) {
          await navigate(page, 'Home');
          await expect(page.getByRole('heading', { name: 'New in your library', exact: true })).toBeVisible();
          await page.screenshot({ path: `${output}/home-${width}-${theme}.png`, animations: 'disabled' });
          await navigate(page, 'Playlists');
          await expect(page.locator('#playlist-name')).toBeVisible();
          await page.screenshot({ path: `${output}/playlists-${width}-${theme}.png`, animations: 'disabled' });
          await navigate(page, 'Library');
        }
      }
    }
    await writeFile(`${output}/spacing-states.json`, JSON.stringify(states));
  } finally { await runtime.close(); }
});

test('UX01 current track and queue clicks pause/resume without replacing audio or position', async ({ page }) => {
  const output = process.env.VELA_UX_EVIDENCE || 'test-results/ux-toggle';
  await mkdir(output, { recursive: true });
  const runtime = await isolatedServer(output);
  try {
    await expect.poll(async () => { try { return (await page.request.get(runtime.origin + '/health')).ok(); } catch { return false; } }).toBeTruthy();
    const setup = await page.request.post(runtime.origin + '/api/setup', { headers: { Origin: runtime.origin }, data: { username: 'listener', password: 'private isolated UX fixture password' } });
    expect(setup.ok()).toBeTruthy();
    const me = await setup.json();
    const uploaded = await page.request.post(runtime.origin + '/api/upload', { headers: { 'X-CSRF-Token': me.csrf_token }, multipart: { file: { name: 'Continuous listening.wav', mimeType: 'audio/wav', buffer: pcmWave(30, 523.25) } } });
    expect(uploaded.ok()).toBeTruthy();
    await page.goto(runtime.origin);
    await navigate(page, 'Library');
    await page.getByRole('button', { name: 'Play Continuous listening', exact: true }).click();
    const audio = page.locator('#vela-audio');
    await expect.poll(() => audio.evaluate(node => node.currentTime)).toBeGreaterThan(0);
    const source = await audio.getAttribute('src');
    await page.getByRole('slider', { name: 'Playback position', exact: true }).evaluate(node => { node.value = '7'; node.dispatchEvent(new Event('input', { bubbles: true })); });
    await expect.poll(() => audio.evaluate(node => node.currentTime)).toBeGreaterThanOrEqual(6.9);
    await page.getByRole('button', { name: 'Pause Continuous listening', exact: true }).click();
    await expect(audio).toHaveJSProperty('paused', true);
    const paused = await audio.evaluate(node => node.currentTime);
    expect(paused).toBeGreaterThanOrEqual(6.9);
    await page.waitForTimeout(250);
    expect(Math.abs(await audio.evaluate(node => node.currentTime) - paused)).toBeLessThan(.1);
    await page.getByRole('button', { name: 'Play Continuous listening', exact: true }).click();
    await expect.poll(() => audio.evaluate(node => node.currentTime)).toBeGreaterThan(paused + .1);
    expect(await audio.getAttribute('src')).toBe(source);
    await navigate(page, 'Home');
    await page.getByRole('button', { name: 'Pause Continuous listening', exact: true }).first().click();
    await expect(audio).toHaveJSProperty('paused', true);
    await page.getByRole('button', { name: 'Play Continuous listening', exact: true }).first().click();
    await expect(audio).toHaveJSProperty('paused', false);
    await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
    await page.locator('.queue-row.current .queue-track').click();
    await expect(audio).toHaveJSProperty('paused', true);
    const queuedPosition = await audio.evaluate(node => node.currentTime);
    await page.locator('.queue-row.current .queue-track').click();
    await expect.poll(() => audio.evaluate(node => node.currentTime)).toBeGreaterThan(queuedPosition + .1);
    expect(await audio.getAttribute('src')).toBe(source);
    await page.screenshot({ path: `${output}/queue-toggle-dark.png` });
  } finally { await runtime.close(); }
});
