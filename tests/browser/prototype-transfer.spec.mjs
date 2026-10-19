/**
 * TR01 + TR02 evidence for the approved prototype transfer (web slice only).
 *
 * This spec is deliberately additive: it owns an isolated server/data directory
 * and proves the canonical role layer, the self-hosted Manrope subsets, the
 * desktop shell geometry (216/240 sidebar, 75 topbar, 88 player, one scroll
 * owner) and route reachability. It does not restyle Home or the player and it
 * never replaces the existing playback/queue/privacy suites.
 */
import { test, expect } from '@playwright/test';
import { mkdir, mkdtemp, open, readFile, rm, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';

const root = fileURLToPath(new URL('../../', import.meta.url));
const LATIN = { path: '/assets/fonts/manrope-latin.woff2', sha256: '2436b9c2b6fe71529530bb1e5b0139d9a9af81b9507bc1b4e09113d3119b5315' };
const CYRILLIC = { path: '/assets/fonts/manrope-cyrillic.woff2', sha256: 'd00bd0cdc67a28c770972d97c6a5f85bbed7f66c99541135d0ec967d11e77e87' };
const OFL = { path: '/assets/fonts/OFL.txt', sha256: '58172e0c0fac2cda8a37b348164bb55e44b0e69051e557e92b1d3f6910141f7b' };

async function isolatedServer(output) {
  const directory = await mkdtemp(join(tmpdir(), 'vela-transfer-'));
  const listener = createServer();
  await new Promise(resolve => listener.listen(0, '127.0.0.1', resolve));
  const port = listener.address().port;
  await new Promise(resolve => listener.close(resolve));
  const log = await open(join(output, 'server.log'), 'w', 0o600);
  const server = spawn(resolve(root, process.env.VELA_TEST_BINARY || 'target/debug/vela-server'), [], {
    cwd: root, stdio: ['ignore', log.fd, log.fd],
    env: {
      ...process.env,
      VELA_BIND: '127.0.0.1:' + port,
      VELA_DATA_DIR: join(directory, 'data'),
      VELA_WEB_DIR: resolve(root, process.env.VELA_TEST_WEB_DIR || 'dist'),
      VELA_SCAN_ROOTS: '[]',
    },
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

function pcmWave(duration, frequency) {
  const samples = 44100 * duration, wave = Buffer.alloc(44 + samples * 2);
  wave.write('RIFF', 0); wave.writeUInt32LE(36 + samples * 2, 4); wave.write('WAVEfmt ', 8);
  wave.writeUInt32LE(16, 16); wave.writeUInt16LE(1, 20); wave.writeUInt16LE(1, 22);
  wave.writeUInt32LE(44100, 24); wave.writeUInt32LE(88200, 28); wave.writeUInt16LE(2, 32);
  wave.writeUInt16LE(16, 34); wave.write('data', 36); wave.writeUInt32LE(samples * 2, 40);
  for (let at = 0; at < samples; at++) wave.writeInt16LE(Math.round(1500 * Math.sin(2 * Math.PI * frequency * at / 44100)), 44 + at * 2);
  return wave;
}

async function signedInFixture(output) {
  const runtime = await isolatedServer(output);
  await expect.poll(async () => {
    try { return (await (await fetch(runtime.origin + '/health')).ok); } catch { return false; }
  }, { timeout: 15000 }).toBeTruthy();
  return runtime;
}

async function setupWithTracks(page, runtime, count = 12) {
  const setup = await page.request.post(runtime.origin + '/api/setup', {
    headers: { Origin: runtime.origin },
    data: { username: 'admin', password: 'private shell geometry fixture password' },
  });
  expect(setup.ok()).toBeTruthy();
  const me = await setup.json();
  let first = null;
  for (let at = 0; at < count; at++) {
    const upload = await page.request.post(runtime.origin + '/api/upload', {
      headers: { 'X-CSRF-Token': me.csrf_token, Origin: runtime.origin },
      multipart: {
        file: {
          name: `Shell geometry ${String(at + 1).padStart(2, '0')}.wav`,
          mimeType: 'audio/wav',
          buffer: pcmWave(4, 420 + at * 17.5),
        },
      },
    });
    expect(upload.ok()).toBeTruthy();
    const track = (await upload.json()).track;
    if (!first) first = track;
  }
  return first;
}

async function settle(page) {
  await expect(page.locator('.app-shell')).toBeVisible();
  await page.evaluate(() => document.fonts.ready);
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

async function navigate(page, name) {
  const mobile = page.getByRole('navigation', { name: 'Mobile navigation', exact: true });
  if (await mobile.isVisible()) {
    const tab = mobile.getByRole('button', { name, exact: true });
    if (await tab.count()) await tab.click();
    else {
      await mobile.getByRole('button', { name: 'More destinations', exact: true }).click();
      await page.getByRole('dialog', { name: 'More from your library', exact: true })
        .getByRole('button', { name, exact: true }).click();
    }
  } else {
    await page.locator('.sidebar').getByRole('button', { name, exact: true }).click();
  }
}

function near(actual, expected, tolerance = 0.75) {
  // Soft: one wrong band must not hide the remaining viewport measurements.
  expect.soft(Math.abs(actual - expected), `${actual} should be ${expected}±${tolerance}`).toBeLessThanOrEqual(tolerance);
}

test('TR01 self-hosted Manrope subsets load with no runtime font CDN and keep role names collision-safe', async ({ page }) => {
  test.setTimeout(90000);
  // Keep evidence outside Playwright's own outputDir so a concurrent suite run
  // cannot clear measurements that this transfer test just produced.
  const output = resolve(root, process.env.VELA_TRANSFER_EVIDENCE || 'target/checks/prototype-transfer-web');
  await mkdir(output, { recursive: true });
  const runtime = await signedInFixture(output);
  const requests = [];
  page.on('request', request => requests.push(request.url()));
  try {
    await setupWithTracks(page, runtime, 4);
    await page.goto(runtime.origin);
    await settle(page);

    const fontState = await page.evaluate(async () => {
      await document.fonts.ready;
      const latin = await document.fonts.load('600 14px Manrope', 'Vela private listening');
      const cyrillic = await document.fonts.load('400 14px Manrope', 'Привет, музыка. Ёё № «»');
      const faces = [...document.fonts].map(face => ({ family: face.family, weight: face.weight, status: face.status }));
      return {
        family: getComputedStyle(document.body).fontFamily,
        check: document.fonts.check('600 14px Manrope', 'Vela'),
        latin: latin.length,
        cyrillic: cyrillic.length,
        faces,
      };
    });
    expect(fontState.family).toMatch(/Manrope/);
    expect(fontState.family).toMatch(/Arial/);
    expect(fontState.check).toBe(true);
    expect(fontState.latin, 'Latin Manrope face must resolve').toBeGreaterThan(0);
    expect(fontState.cyrillic, 'Cyrillic Manrope face must resolve').toBeGreaterThan(0);
    const manrope = fontState.faces.filter(face => face.family.includes('Manrope'));
    expect(manrope.length).toBeGreaterThanOrEqual(2);
    expect(manrope.filter(face => face.status === 'loaded').length).toBeGreaterThanOrEqual(2);

    for (const asset of [LATIN, CYRILLIC]) {
      const response = await page.request.get(runtime.origin + asset.path);
      expect(response.status(), asset.path).toBe(200);
      expect(response.headers()['content-type'], asset.path).toBe('font/woff2');
      const body = Buffer.from(await response.body());
      expect(body.subarray(0, 4).toString('latin1'), asset.path).toBe('wOF2');
      expect(createHash('sha256').update(body).digest('hex'), asset.path).toBe(asset.sha256);
    }
    const license = await page.request.get(runtime.origin + OFL.path);
    expect(license.status()).toBe(200);
    const licenseBody = Buffer.from(await license.body());
    expect(createHash('sha256').update(licenseBody).digest('hex')).toBe(OFL.sha256);
    expect(licenseBody.toString('utf8')).toContain('SIL Open Font License');
    expect(licenseBody.toString('utf8')).toContain('Manrope');
    // The allowlist is exact: unlisted neighbours and the repo-only provenance
    // note are not exposed, even though they sit next to the shipped files.
    for (const denied of ['/assets/fonts/manrope-greek.woff2', '/assets/fonts/SOURCE.md']) {
      expect((await page.request.get(runtime.origin + denied)).status(), denied).toBe(404);
    }

    const sameOriginFonts = requests.filter(url => url.includes('/assets/fonts/'));
    expect(sameOriginFonts.length).toBeGreaterThanOrEqual(2);
    expect(sameOriginFonts.every(url => url.startsWith(runtime.origin))).toBeTruthy();
    const foreign = requests.filter(url => /^https?:/.test(url) && !url.startsWith(runtime.origin));
    expect(foreign, 'runtime must not fetch a font CDN or any foreign origin').toEqual([]);
    expect(requests.some(url => /fonts\.(googleapis|gstatic)|use\.typekit|cdn\.jsdelivr/i.test(url))).toBe(false);

    const css = await (await page.request.get(runtime.origin + '/style.css')).text();
    expect(css).not.toMatch(/@import\s+url\(\s*["']?https?:/i);
    expect(css).not.toMatch(/url\(\s*["']?https?:/i);
    expect(css).toContain('--vela-sidebar-width');

    const roles = await page.evaluate(() => {
      const style = getComputedStyle(document.documentElement);
      const read = name => style.getPropertyValue(name).trim();
      return {
        sidebarLength: read('--sidebar'),
        sidebarWidth: read('--vela-sidebar-width'),
        muted: read('--muted'),
        quiet: read('--quiet'),
        accent: read('--accent'),
        sidebarBackground: read('--vela-sidebar-bg'),
        playerBackground: read('--vela-player-bg'),
        sidebarComputed: read('--vela-muted-text'),
      };
    });
    expect(roles.sidebarLength).toBe('216px');
    expect(roles.sidebarWidth).toBe('216px');
    expect(roles.muted).toBe('#9aa296');
    expect(roles.quiet).toBe('#9aa296');
    expect(roles.accent).toBe('#d3e9a5');
    expect(roles.sidebarBackground).toBe('#0e100f');
    expect(roles.playerBackground).toBe('#10130f');
    expect(roles.sidebarComputed).toBe('#9aa296');

    const surfaces = await page.evaluate(() => ({
      body: getComputedStyle(document.body).backgroundColor,
      sidebar: getComputedStyle(document.querySelector('.sidebar')).backgroundColor,
      player: getComputedStyle(document.querySelector('.player-bar')).backgroundColor,
      topbarBorder: getComputedStyle(document.querySelector('.topbar')).borderBottomColor,
    }));
    expect(surfaces.body).toBe('rgb(19, 21, 19)');
    expect(surfaces.sidebar).toBe('rgb(14, 16, 15)');
    expect(surfaces.player).toBe('rgb(16, 19, 15)');

    // Light stays a real, separate adaptation of the same canonical roles.
    await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
    const light = await page.evaluate(() => {
      const style = getComputedStyle(document.documentElement);
      return {
        body: getComputedStyle(document.body).backgroundColor,
        sidebar: getComputedStyle(document.querySelector('.sidebar')).backgroundColor,
        accent: style.getPropertyValue('--accent').trim(),
        sidebarLength: style.getPropertyValue('--sidebar').trim(),
      };
    });
    expect(light.body).toBe('rgb(244, 247, 243)');
    expect(light.sidebar).toBe('rgb(255, 255, 255)');
    expect(light.accent).toBe('#08754c');
    expect(light.sidebarLength).toBe('216px');
    await page.screenshot({ path: `${output}/roles-light-1284.png`, animations: 'disabled' });
    await writeFile(`${output}/font-and-role-evidence.json`, JSON.stringify({ fontState, roles, surfaces, light }, null, 2));
  } finally { await runtime.close(); }
});

test('TR02 shell reserves one sidebar offset and one player band with one central scroller', async ({ page }) => {
  test.setTimeout(180000);
  // Keep evidence outside Playwright's own outputDir so a concurrent suite run
  // cannot clear measurements that this transfer test just produced.
  const output = resolve(root, process.env.VELA_TRANSFER_EVIDENCE || 'target/checks/prototype-transfer-web');
  await mkdir(output, { recursive: true });
  const runtime = await signedInFixture(output);
  try {
    const track = await setupWithTracks(page, runtime, 12);
    await page.setViewportSize({ width: 1284, height: 856 });
    await page.goto(runtime.origin);
    await settle(page);

    // Navigation must not replace or restart the persistent media element.
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(12);
    await page.getByRole('button', { name: `Play ${track.title}`, exact: true }).click();
    await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
    const source = await page.locator('#vela-audio').evaluate(audio => {
      audio.dataset.transferIdentity = 'same-root-player';
      return audio.currentSrc;
    });
    for (const destination of ['Home', 'Settings', 'Library', 'Albums', 'Library']) {
      await navigate(page, destination);
    }
    await expect(page.locator('#vela-audio')).toHaveAttribute('data-transfer-identity', 'same-root-player');
    expect(await page.locator('#vela-audio').evaluate(audio => audio.currentSrc)).toBe(source);
    expect(await page.locator('#vela-audio').evaluate(audio => audio.paused)).toBe(false);

    const measurements = [];
    for (const { width, height, sidebar, gutter, contentTop } of [
      { width: 1284, height: 856, sidebar: 216, gutter: 33, contentTop: 24 },
      { width: 1500, height: 900, sidebar: 240, gutter: 48, contentTop: 35 },
      { width: 1501, height: 900, sidebar: 240, gutter: 48, contentTop: 35 },
      { width: 3440, height: 1000, sidebar: 240, gutter: 48, contentTop: 35 },
      { width: 390, height: 844, sidebar: 0, gutter: 20, contentTop: 25 },
      { width: 320, height: 780, sidebar: 0, gutter: 16, contentTop: 25 },
    ]) {
      await page.setViewportSize({ width, height });
      await settle(page);
      const measured = await page.evaluate(() => {
        const box = selector => {
          const element = document.querySelector(selector);
          if (!element) return null;
          const rect = element.getBoundingClientRect();
          const style = getComputedStyle(element);
          return {
            x: rect.x, y: rect.y, top: rect.top, width: rect.width, height: rect.height,
            right: rect.right, bottom: rect.bottom,
            display: style.display, position: style.position,
            paddingLeft: parseFloat(style.paddingLeft), paddingTop: parseFloat(style.paddingTop),
            backgroundColor: style.backgroundColor,
          };
        };
        const scroller = document.querySelector('#main-scroll');
        const inner = document.querySelector('.content-inner');
        const main = document.querySelector('main.main-content');
        const player = document.querySelector('.player-bar');
        const nav = document.querySelector('.mobile-nav');
        const topbar = document.querySelector('.topbar');
        const mainRect = main.getBoundingClientRect();
        const innerRect = inner.getBoundingClientRect();
        const rootStyle = getComputedStyle(document.documentElement);
        return {
          innerWidth, innerHeight,
          sidebar: box('.sidebar'), topbar: box('.topbar'), player: box('.player-bar'),
          topbarBox: (() => {
            const element = document.querySelector('.topbar');
            return { clientHeight: element.clientHeight, scrollHeight: element.scrollHeight };
          })(),
          nav: box('.mobile-nav'), main: box('main.main-content'), inner: box('.content-inner'),
          scroller: scroller ? {
            top: scroller.getBoundingClientRect().top,
            bottom: scroller.getBoundingClientRect().bottom,
            clientHeight: scroller.clientHeight, scrollHeight: scroller.scrollHeight,
            clientWidth: scroller.clientWidth, scrollWidth: scroller.scrollWidth,
            overflowY: getComputedStyle(scroller).overflowY,
          } : null,
          topbarScroll: { before: box('.topbar').y },
          documentOverflow: document.documentElement.scrollWidth - innerWidth,
          bodyOverflow: document.body.scrollWidth - innerWidth,
          sidebarLeftGap: innerRect.left - mainRect.left,
          sidebarRightGap: mainRect.right - innerRect.right,
          roles: {
            sidebar: rootStyle.getPropertyValue('--vela-sidebar-width').trim(),
            player: rootStyle.getPropertyValue('--vela-player-height').trim(),
            topbar: rootStyle.getPropertyValue('--vela-topbar-height').trim(),
            legacySidebar: rootStyle.getPropertyValue('--sidebar').trim(),
            legacyPlayer: rootStyle.getPropertyValue('--player').trim(),
          },
        };
      });
      await page.locator('#main-scroll').evaluate(element => { element.scrollTop = element.scrollHeight; });
      const clearance = await page.evaluate(() => {
        const scroller = document.querySelector('#main-scroll').getBoundingClientRect();
        const footer = document.querySelector('.content-footer').getBoundingClientRect();
        const player = document.querySelector('.player-bar').getBoundingClientRect();
        const topbar = document.querySelector('.topbar').getBoundingClientRect();
        return {
          scrollerBottom: scroller.bottom, footerTop: footer.top, footerBottom: footer.bottom,
          playerTop: player.top, topbarTop: topbar.top, topbarHeight: topbar.height,
        };
      });
      const mobile = width <= 760;
      measurements.push({ width, height, mobile, ...measured, clearance });

      expect.soft(measured.documentOverflow, `${width} document overflow`).toBeLessThanOrEqual(1);
      expect.soft(measured.topbarBox.scrollHeight, `${width} topbar content fits`).toBeLessThanOrEqual(measured.topbarBox.clientHeight + 1);
      expect.soft(measured.inner.top, `${width} content starts below topbar`).toBeGreaterThanOrEqual(measured.topbar.bottom - 1);
      expect.soft(measured.bodyOverflow, `${width} body overflow`).toBeLessThanOrEqual(1);
      expect.soft(measured.scroller.overflowY, `${width} scroller`).toMatch(/auto|scroll/);
      expect.soft(measured.scroller.scrollWidth, `${width} scroller horizontal overflow`).toBeLessThanOrEqual(measured.scroller.clientWidth + 1);
      expect.soft(measured.scroller.scrollHeight, `${width} content should be scrollable`).toBeGreaterThan(measured.scroller.clientHeight);
      expect.soft(measured.inner.paddingLeft, `${width} content gutter`).toBeCloseTo(gutter, 0);
      expect.soft(measured.inner.paddingTop, `${width} content top`).toBeCloseTo(contentTop, 0);
      expect.soft(measured.roles.legacySidebar, `${width} legacy sidebar stays a length`).toBe(`${sidebar}px`);
      expect.soft(measured.roles.legacyPlayer, `${width} legacy player stays a length`).toMatch(/^\d+px$/);

      if (mobile) {
        expect.soft(measured.sidebar.display, `${width} sidebar hidden`).toBe('none');
        near(measured.topbar.height, 63);
        near(measured.main.x, 0);
        near(measured.player.height, 120);
        near(measured.player.bottom, measured.nav.y);
        near(measured.nav.height, 70);
        near(measured.nav.bottom, height);
        expect.soft(measured.nav.display, `${width} mobile navigation visible`).toBe('flex');
        near(measured.scroller.bottom, measured.player.top);
        expect.soft(measured.sidebarLeftGap, `${width} no double left offset`).toBeCloseTo(0, 0);
      } else {
        expect.soft(measured.sidebar.display, `${width} sidebar visible`).not.toBe('none');
        near(measured.sidebar.width, sidebar);
        near(measured.main.x, sidebar);
        near(measured.topbar.height, 75);
        near(measured.topbar.paddingLeft, gutter);
        near(measured.player.height, 88);
        near(measured.player.bottom, height);
        near(measured.scroller.bottom, measured.player.top);
        expect.soft(measured.nav.display, `${width} no mobile navigation`).toBe('none');
        // Centered inside the main column only: equal side gaps, never a
        // window-centered band that ignores the sidebar offset.
        near(measured.sidebarLeftGap, measured.sidebarRightGap, 1.5);
        if (width === 3440) near(measured.inner.width, 1560);
        if (width < 1560 + sidebar) near(measured.inner.width, width - sidebar);
      }

      // The topbar and player stay put while the single scroller moves, and the
      // final footer row can be scrolled entirely above the player/navigation.
      const afterScroll = await page.evaluate(() => ({
        topbar: document.querySelector('.topbar').getBoundingClientRect().y,
        player: document.querySelector('.player-bar').getBoundingClientRect().bottom,
      }));
      near(afterScroll.topbar, measured.topbarScroll.before);
      near(afterScroll.player, mobile ? height - 70 : height);
      expect.soft(clearance.footerBottom, `${width} last row above player`).toBeLessThanOrEqual(clearance.scrollerBottom + 1);
      expect.soft(clearance.footerTop, `${width} footer starts below topbar`).toBeGreaterThanOrEqual(clearance.topbarTop + clearance.topbarHeight - 1);

      await page.locator('#main-scroll').evaluate(element => { element.scrollTop = 0; });
      await page.screenshot({ path: `${output}/shell-${width}x${height}.png`, animations: 'disabled' });
    }

    // Every currently shipped destination remains reachable at the reference
    // desktop width and on the mobile composition. Jobs stays administrator-only.
    const routes = [
      ['Home', null],
      ['Search', 'Search'],
      ['Library', 'Your library'],
      ['Albums', 'Albums'],
      ['Artists', 'Artists'],
      ['Favorites', 'Favorites'],
      ['Playlists', 'Playlists'],
      ['Listening history', 'Listening history'],
      ['Add music', 'Add music'],
      ['Jobs', 'Jobs'],
      ['Network', 'Connected music'],
      ['Settings', 'Settings'],
    ];
    for (const [width, height] of [[1284, 856], [390, 844]]) {
      await page.setViewportSize({ width, height });
      await page.goto(runtime.origin);
      await settle(page);
      for (const [destination, heading] of routes) {
        await navigate(page, destination);
        if (heading) await expect(page.getByRole('heading', { name: heading, exact: true, level: 1 })).toBeVisible();
        else await expect(page.getByRole('heading', { name: 'New in your library', exact: true })).toBeVisible();
        expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), `${width} ${destination} overflow`).toBeLessThanOrEqual(1);
      }
    }

    await writeFile(`${output}/shell-geometry.json`, JSON.stringify(measurements, null, 2));
  } finally { await runtime.close(); }
});
