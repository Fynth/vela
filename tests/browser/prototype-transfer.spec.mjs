/**
 * TR01 + TR02 evidence for the approved prototype transfer (web slice only).
 *
 * This spec is deliberately additive: it owns an isolated server/data directory
 * and proves the canonical role layer, the self-hosted Manrope subsets, the
 * desktop shell geometry (216/240 sidebar, 75 topbar, 88 player, one scroll
 * owner), route reachability and the TR03 reference persistent player. TR03
 * assertions project the existing root engine: they never replace the
 * playback/queue/privacy suites and never start a second audio owner.
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
const COVERED_FLAC = fileURLToPath(new URL('../../crates/media/tests/fixtures/signal.flac', import.meta.url));

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
    await page.setViewportSize({ width: 1284, height: 856 });
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
    for (const { width, height, sidebar, gutter, contentTop, playerHeight } of [
      { width: 1284, height: 856, sidebar: 216, gutter: 33, contentTop: 24, playerHeight: 88 },
      { width: 1500, height: 900, sidebar: 240, gutter: 48, contentTop: 35, playerHeight: 88 },
      { width: 1501, height: 900, sidebar: 240, gutter: 48, contentTop: 35, playerHeight: 88 },
      { width: 3440, height: 1000, sidebar: 240, gutter: 48, contentTop: 35, playerHeight: 88 },
      { width: 1101, height: 900, sidebar: 216, gutter: 33, contentTop: 24, playerHeight: 88 },
      { width: 1100, height: 900, sidebar: 190, gutter: 25, contentTop: 24, playerHeight: 88 },
      { width: 900, height: 900, sidebar: 175, gutter: 20, contentTop: 24, playerHeight: 88 },
      { width: 761, height: 900, sidebar: 175, gutter: 20, contentTop: 24, playerHeight: 88 },
      { width: 760, height: 900, sidebar: 0, gutter: 20, contentTop: 25, playerHeight: 88 },
      { width: 621, height: 900, sidebar: 0, gutter: 20, contentTop: 25, playerHeight: 88 },
      { width: 620, height: 900, sidebar: 0, gutter: 20, contentTop: 25, playerHeight: 120 },
      { width: 390, height: 844, sidebar: 0, gutter: 20, contentTop: 25, playerHeight: 120 },
      { width: 320, height: 780, sidebar: 0, gutter: 16, contentTop: 25, playerHeight: 120 },
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
          documentScrollHeight: document.documentElement.scrollHeight,
          bodyScrollHeight: document.body.scrollHeight,
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
      expect.soft(measured.documentScrollHeight, `${width} body is not a second scroller`).toBeLessThanOrEqual(height + 1);
      expect.soft(measured.bodyScrollHeight, `${width} body scroll height`).toBeLessThanOrEqual(height + 1);
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
        near(measured.player.height, playerHeight);
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
        near(measured.player.height, playerHeight);
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

    // Device app size composes with the shell exactly once: the 100dvh shell is
    // divided by the same --app-zoom the preference adapter writes, so 80% and
    // 150% still fill one viewport with one player band.
    await page.setViewportSize({ width: 1284, height: 856 });
    await settle(page);
    const zoomChecks = [];
    for (const zoom of [0.8, 1.5]) {
      const previous = await page.evaluate(() => document.documentElement.style.getPropertyValue('--app-zoom'));
      const zoomed = await page.evaluate(async value => {
        document.documentElement.style.setProperty('--app-zoom', String(value));
        await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
        const shell = document.querySelector('.app-shell').getBoundingClientRect();
        const player = document.querySelector('.player-bar').getBoundingClientRect();
        const topbar = document.querySelector('.topbar').getBoundingClientRect();
        const scroller = document.querySelector('#main-scroll').getBoundingClientRect();
        return {
          zoom: value, innerWidth, innerHeight,
          shellHeight: shell.height, shellWidth: shell.width,
          topbarHeight: topbar.height, playerHeight: player.height,
          playerBottom: player.bottom, scrollerBottom: scroller.bottom,
          overflow: document.documentElement.scrollWidth - innerWidth,
          computedZoom: getComputedStyle(document.documentElement).zoom,
        };
      }, zoom);
      zoomChecks.push(zoomed);
      expect.soft(zoomed.computedZoom, `${zoom} only one zoom`).toBe(String(zoom));
      expect.soft(zoomed.shellHeight, `${zoom} shell fills one viewport`).toBeCloseTo(zoomed.innerHeight, 0);
      expect.soft(zoomed.shellWidth, `${zoom} shell fills viewport width`).toBeCloseTo(zoomed.innerWidth, 0);
      expect.soft(zoomed.topbarHeight, `${zoom} topbar scales once`).toBeCloseTo(75 * zoom, 0);
      expect.soft(zoomed.playerHeight, `${zoom} player scales once`).toBeCloseTo(88 * zoom, 0);
      expect.soft(zoomed.playerBottom, `${zoom} player ends at viewport`).toBeCloseTo(zoomed.innerHeight, 0);
      expect.soft(zoomed.scrollerBottom, `${zoom} scroller ends above player`).toBeCloseTo(zoomed.playerBottom - zoomed.playerHeight, 0);
      expect.soft(zoomed.overflow, `${zoom} no zoom overflow`).toBeLessThanOrEqual(1);
      await page.evaluate(previousValue => {
        if (previousValue) document.documentElement.style.setProperty('--app-zoom', previousValue);
        else document.documentElement.style.removeProperty('--app-zoom');
      }, previous);
      await settle(page);
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

    await writeFile(`${output}/shell-geometry.json`, JSON.stringify({ measurements, zoomChecks }, null, 2));
  } finally { await runtime.close(); }
});

async function uploadTrack(page, runtime, csrf, name, mimeType, buffer) {
  const response = await page.request.post(runtime.origin + '/api/upload', {
    headers: { 'X-CSRF-Token': csrf, Origin: runtime.origin },
    multipart: { file: { name, mimeType, buffer } },
  });
  expect(response.ok(), `upload ${name}`).toBeTruthy();
  return (await response.json()).track;
}

function expectedQuality(track) {
  // Mirrors the player's honest projection of the real Track DTO: no invented
  // "lossless"/"bit-perfect" claims, absent values omitted.
  const parts = [];
  if (track.codec) parts.push(track.codec);
  if (track.sample_rate > 0) {
    parts.push(track.sample_rate % 1000 === 0
      ? `${track.sample_rate / 1000} kHz`
      : `${(track.sample_rate / 1000).toFixed(1)} kHz`);
  }
  if (track.bit_depth > 0) parts.push(`${track.bit_depth}-bit`);
  if (parts.length <= 1 && track.bitrate > 0) parts.push(`${track.bitrate} kbps`);
  return parts.join(' · ');
}

test('TR03 persistent player projects the root engine with reference geometry and no source reset', async ({ page }) => {
  test.setTimeout(240000);
  const output = resolve(root, process.env.VELA_TRANSFER_EVIDENCE || 'target/checks/prototype-transfer-web');
  await mkdir(output, { recursive: true });
  const runtime = await signedInFixture(output);
  try {
    const setup = await page.request.post(runtime.origin + '/api/setup', {
      headers: { Origin: runtime.origin },
      data: { username: 'admin', password: 'private player geometry fixture password' },
    });
    expect(setup.ok()).toBeTruthy();
    const me = await setup.json();
    const covered = await uploadTrack(page, runtime, me.csrf_token, 'Covered original.flac', 'audio/flac',
      await readFile(COVERED_FLAC));
    expect(covered.artwork_id, 'fixture must carry embedded artwork').toBeTruthy();

    await page.setViewportSize({ width: 1284, height: 856 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(1);

    const audio = page.locator('#vela-audio');
    await page.getByRole('button', { name: `Play ${covered.title}`, exact: true }).click();
    await expect.poll(() => audio.evaluate(node => node.currentTime), { timeout: 10000 }).toBeGreaterThan(0.2);

    // --- Real artwork, title/artist/album and honest quality from real data.
    const playerTrack = {
      title: await page.locator('.now-playing strong.player-title').innerText(),
      artist: await page.locator('.now-playing .player-artist').innerText(),
      album: await page.locator('.now-playing .player-album').innerText(),
      artwork: await page.locator('.now-playing .artwork img').getAttribute('src'),
      quality: await page.locator('.player-quality-format').innerText(),
    };
    expect(playerTrack.title).toBe(covered.title);
    expect(playerTrack.artist).toBe(covered.artist);
    expect(playerTrack.album).toBe(covered.album);
    expect(playerTrack.artwork).toBe(`/api/artwork/${covered.artwork_id}`);
    expect(playerTrack.quality).toBe(expectedQuality(covered));
    expect(playerTrack.quality).not.toMatch(/lossless|bit-?perfect|hi-?res/i);
    expect(await page.locator('.now-playing .player-album').evaluate(node => parseFloat(getComputedStyle(node).fontSize))).toBeCloseTo(10, 0.5);
    await expect.poll(() => page.locator('.now-playing .artwork img').evaluate(img => img.complete && img.naturalWidth > 0)).toBeTruthy();
    const artworkResponse = await page.request.get(runtime.origin + playerTrack.artwork);
    expect(artworkResponse.status()).toBe(200);
    expect(artworkResponse.headers()['content-type']).toMatch(/^image\//);

    // The player's favorite is the same real mutation as rows and Now Playing;
    // it never touches the source generation or position.
    const playerFavorite = page.locator('.player-favorite');
    await expect(playerFavorite).toHaveAttribute('aria-pressed', 'false');
    const favoriteSource = await page.evaluate(() => {
      const element = document.querySelector('#vela-audio');
      return { generation: element.getAttribute('data-source-generation'), src: element.getAttribute('src') };
    });
    await playerFavorite.click();
    await expect(playerFavorite).toHaveAttribute('aria-pressed', 'true');
    await expect(playerFavorite).toBeEnabled();
    await expect(page.locator('.track-row .favorite')).toHaveAttribute('aria-pressed', 'true');
    await playerFavorite.click();
    await expect(playerFavorite).toHaveAttribute('aria-pressed', 'false');
    expect(await page.evaluate(() => document.querySelector('#vela-audio').getAttribute('data-source-generation'))).toBe(favoriteSource.generation);
    expect(await page.evaluate(() => document.querySelector('#vela-audio').getAttribute('src'))).toBe(favoriteSource.src);

    // A one-second lawful fixture reaches natural EOF: the player keeps the current
    // occurrence and reports the real paused phase instead of inventing playback.
    await expect.poll(() => audio.evaluate(node => node.paused), { timeout: 15000 }).toBeTruthy();
    await expect(page.locator('.now-playing strong.player-title')).toHaveText(covered.title);
    expect(await page.locator('.player-eq').getAttribute('data-state')).toBe('paused');
    await expect(page.locator('#player-toggle')).toHaveAttribute('aria-label', 'Play');
    await expect(page.locator('.track-row.current .track-play')).toHaveAttribute('aria-label', `Play ${covered.title}`);

    // The single root media element survives navigation, theme and app scale.
    await page.evaluate(() => {
      const element = document.querySelector('#vela-audio');
      element.dataset.tr03 = 'root-player';
      window.__tr03Audio = element;
      const art = document.querySelector('.now-playing .artwork img');
      art.dataset.tr03 = 'root-artwork';
      window.__tr03Art = art;
    });
    const before = await page.evaluate(() => {
      const element = document.querySelector('#vela-audio');
      return {
        src: element.getAttribute('src'), currentSrc: element.currentSrc,
        generation: element.getAttribute('data-source-generation'), time: element.currentTime,
      };
    });
    for (const destination of ['Home', 'Search', 'Settings']) await navigate(page, destination);
    await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
    const lightPlayer = await page.evaluate(() => ({
      background: getComputedStyle(document.querySelector('.player-bar')).backgroundColor,
      face: getComputedStyle(document.querySelector('.play-toggle-face')).backgroundColor,
    }));
    expect(lightPlayer.background).toBe('rgb(255, 255, 255)');
    expect(lightPlayer.face).toBe('rgb(8, 117, 76)');
    await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');

    await page.getByRole('button', { name: 'Increase app size', exact: true }).click();
    await expect.poll(() => page.evaluate(() => getComputedStyle(document.documentElement).zoom)).toBe('1.05');
    const scaled = await page.evaluate(() => {
      const player = document.querySelector('.player-bar').getBoundingClientRect();
      return { height: player.height, bottom: player.bottom, innerHeight };
    });
    expect(scaled.height).toBeCloseTo(88 * 1.05, 0);
    expect(scaled.bottom).toBeCloseTo(scaled.innerHeight, 0);
    await page.getByRole('button', { name: 'Reset to 100%', exact: true }).click();
    await expect.poll(() => page.evaluate(() => getComputedStyle(document.documentElement).zoom)).toBe('1');
    await navigate(page, 'Library');
    await expect(page.locator('.now-playing strong.player-title')).toHaveText(covered.title);

    const after = await page.evaluate(() => {
      const element = document.querySelector('#vela-audio');
      return {
        marker: element.dataset.tr03, same: element === window.__tr03Audio,
        count: document.querySelectorAll('audio').length,
        src: element.getAttribute('src'), currentSrc: element.currentSrc,
        generation: element.getAttribute('data-source-generation'), time: element.currentTime,
        artMarker: document.querySelector('.now-playing .artwork img')?.dataset.tr03,
        artSame: document.querySelector('.now-playing .artwork img') === window.__tr03Art,
        art: document.querySelector('.now-playing .artwork img')?.getAttribute('src'),
      };
    });
    expect(after.marker).toBe('root-player');
    expect(after.same).toBe(true);
    expect(after.count).toBe(1);
    expect(after.src).toBe(before.src);
    expect(after.currentSrc).toBe(before.currentSrc);
    expect(after.generation).toBe(before.generation);
    expect(Math.abs(after.time - before.time)).toBeLessThan(0.1);
    expect(after.artMarker).toBe('root-artwork');
    expect(after.artSame).toBe(true);
    expect(after.art).toBe(`/api/artwork/${covered.artwork_id}`);
    await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
    await expect(page.getByRole('complementary', { name: 'Playback queue', exact: true }).locator('.queue-row.current')).toHaveCount(1);
    await expect(page.getByRole('complementary', { name: 'Playback queue', exact: true }).locator('.queue-row')).toHaveCount(1);
    await page.getByRole('button', { name: 'Close playback queue', exact: true }).click();

    // A controlled queue [current occurrence, long A, long B] keeps Next/Previous
    // deterministic: the added occurrences are explicit, not list-order guesses.
    const longA = await uploadTrack(page, runtime, me.csrf_token, 'Player long A.wav', 'audio/wav', pcmWave(30, 523.25));
    const longB = await uploadTrack(page, runtime, me.csrf_token, 'Player long B.wav', 'audio/wav', pcmWave(30, 659.25));
    await navigate(page, 'Home');
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(3);
    await page.getByRole('button', { name: `Add ${longA.title} to playback queue`, exact: true }).click();
    await page.getByRole('button', { name: `Add ${longB.title} to playback queue`, exact: true }).click();

    await page.locator('#player-next').click();
    await expect(page.locator('.now-playing strong.player-title')).toHaveText(longA.title);
    await expect(page.locator('#player-toggle')).toHaveAttribute('aria-label', 'Pause');
    await expect.poll(() => audio.evaluate(node => node.currentTime), { timeout: 10000 }).toBeGreaterThan(0.3);
    expect(await page.locator('.player-eq').getAttribute('data-state')).toBe('playing');
    await expect(page.locator('.track-row.current .track-play')).toHaveAttribute('aria-label', `Pause ${longA.title}`);
    const longQuality = await page.locator('.player-quality-format').innerText();
    expect(longQuality).toBe(expectedQuality(longA));
    expect(longQuality).not.toMatch(/lossless|bit-?perfect|hi-?res/i);

    // Next/Previous move the real queue occurrence on the same element.
    await page.locator('#player-next').click();
    await expect(page.locator('.now-playing strong.player-title')).toHaveText(longB.title);
    expect(await page.evaluate(() => document.querySelectorAll('audio').length)).toBe(1);
    expect(await page.evaluate(() => document.querySelector('#vela-audio') === window.__tr03Audio)).toBe(true);
    await page.locator('#player-previous').click();
    await expect(page.locator('.now-playing strong.player-title')).toHaveText(longA.title);
    await expect(page.locator('#player-toggle')).toHaveAttribute('aria-label', 'Pause');

    // Play/Pause and Seek act on the one existing element and keep its source.
    await page.locator('#player-toggle').click();
    await expect(page.locator('#player-toggle')).toHaveAttribute('aria-label', 'Play');
    expect(await page.locator('.player-eq').getAttribute('data-state')).toBe('paused');
    const paused = await audio.evaluate(node => node.currentTime);
    await page.waitForTimeout(250);
    expect(Math.abs(await audio.evaluate(node => node.currentTime) - paused)).toBeLessThan(0.1);
    // A pause is an explicit cancellation of pending source work, so it owns its
    // own generation. Seek/volume below must not create a new source on top.
    const pausedSource = await page.evaluate(() => {
      const element = document.querySelector('#vela-audio');
      return { src: element.getAttribute('src'), generation: element.getAttribute('data-source-generation') };
    });
    await page.getByRole('slider', { name: 'Playback position', exact: true })
      .evaluate(input => { input.value = '5'; input.dispatchEvent(new Event('input', { bubbles: true })); });
    await expect.poll(() => audio.evaluate(node => node.currentTime)).toBeGreaterThanOrEqual(4.9);
    expect(await audio.evaluate(node => node.currentTime)).toBeLessThan(5.4);
    await expect(page.locator('.player-time').first()).toHaveText('0:05');
    const seek = await page.evaluate(() => {
      const rail = document.querySelector('.seek-rail').getBoundingClientRect();
      const fill = document.querySelector('.seek-fill').getBoundingClientRect();
      const input = document.querySelector('.seek-control input');
      return {
        ratio: rail.width ? fill.width / rail.width : 0,
        rail: rail.height, input: input.getBoundingClientRect().height,
        max: Number(input.getAttribute('max')), value: Number(input.value),
        valuetext: input.getAttribute('aria-valuetext'),
        src: document.querySelector('#vela-audio').getAttribute('src'),
        generation: document.querySelector('#vela-audio').getAttribute('data-source-generation'),
      };
    });
    expect(seek.ratio).toBeCloseTo(5 / seek.max, 2);
    expect(seek.rail).toBeCloseTo(3, 0.75);
    expect(seek.input).toBeGreaterThanOrEqual(44);
    expect(seek.valuetext).toMatch(/^0:0\d of \d+:\d\d$/);
    expect(seek.src).toBe(pausedSource.src);
    expect(seek.generation).toBe(pausedSource.generation);
    await page.locator('#player-toggle').click();
    await expect(page.locator('#player-toggle')).toHaveAttribute('aria-label', 'Pause');
    await expect.poll(() => audio.evaluate(node => node.currentTime)).toBeGreaterThan(5.1);

    // Volume and Mute change the existing element's real level; Unmute restores
    // the user's level instead of a hard-coded default.
    const volumeInput = page.locator('.player-right input[aria-label="Volume"]');
    await volumeInput.evaluate(input => { input.value = '0.35'; input.dispatchEvent(new Event('input', { bubbles: true })); });
    await expect.poll(() => audio.evaluate(node => node.volume)).toBeCloseTo(0.35, 2);
    const mute = page.getByRole('button', { name: 'Mute', exact: true });
    await mute.click();
    await expect.poll(() => audio.evaluate(node => node.volume)).toBe(0);
    await expect(mute).toHaveAttribute('aria-pressed', 'true');
    await expect(volumeInput).toHaveValue('0');
    await mute.click();
    await expect.poll(() => audio.evaluate(node => node.volume)).toBeCloseTo(0.35, 2);
    await expect(mute).toHaveAttribute('aria-pressed', 'false');
    await expect(volumeInput).toHaveValue('0.35');
    expect(await audio.evaluate(node => node.getAttribute('data-source-generation'))).toBe(pausedSource.generation);
    await page.locator('#player-toggle').click();
    await expect(page.locator('#player-toggle')).toHaveAttribute('aria-label', 'Play');

    // Keyboard operability: native button activation and native slider keys, plus
    // the reduced-motion state marker (static bars, same real phase).
    await page.locator('#player-toggle').focus();
    await page.keyboard.press('Enter');
    await expect(page.locator('#player-toggle')).toHaveAttribute('aria-label', 'Pause');
    await page.emulateMedia({ reducedMotion: 'reduce' });
    expect(await page.locator('.player-eq i').first().evaluate(node => getComputedStyle(node).animationName)).toBe('none');
    expect(await page.locator('.player-eq').getAttribute('data-state')).toBe('playing');
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    await page.locator('#player-toggle').focus();
    await page.keyboard.press('Enter');
    await expect(page.locator('#player-toggle')).toHaveAttribute('aria-label', 'Play');
    const beforeArrow = await audio.evaluate(node => node.currentTime);
    const keyboardSource = await page.evaluate(() => {
      const element = document.querySelector('#vela-audio');
      return { src: element.getAttribute('src'), generation: element.getAttribute('data-source-generation') };
    });
    await page.getByRole('slider', { name: 'Playback position', exact: true }).focus();
    await page.keyboard.press('ArrowRight');
    await expect.poll(() => audio.evaluate(node => node.currentTime)).toBeGreaterThan(beforeArrow + 0.05);
    expect(await audio.evaluate(node => node.getAttribute('data-source-generation'))).toBe(keyboardSource.generation);
    expect(await audio.evaluate(node => node.getAttribute('src'))).toBe(keyboardSource.src);
    // Focusing the seek hit box must not scroll the clipping shell by a pixel:
    // the player band stays exactly at the viewport edge.
    expect(await page.evaluate(() => document.querySelector('.app-shell').scrollTop)).toBe(0);
    expect(await page.evaluate(() => document.querySelector('.player-bar').getBoundingClientRect().bottom))
      .toBeCloseTo(await page.evaluate(() => innerHeight), 0.75);

    // Shuffle and Repeat expose real root modes with pressed state and a dot.
    const shuffle = page.getByRole('button', { name: 'Shuffle', exact: true });
    await shuffle.click();
    await expect(shuffle).toHaveAttribute('aria-pressed', 'true');
    await expect(page.locator('.player-shuffle .player-mode-dot')).toHaveCount(1);
    await shuffle.click();
    await expect(shuffle).toHaveAttribute('aria-pressed', 'false');
    await expect(page.locator('.player-shuffle .player-mode-dot')).toHaveCount(0);
    for (const [current, next] of [['off', 'all'], ['all', 'one'], ['one', 'off']]) {
      await page.getByRole('button', { name: `Repeat: ${current}`, exact: true }).click();
      const button = page.getByRole('button', { name: `Repeat: ${next}`, exact: true });
      await expect(button).toBeVisible();
      await expect(button).toHaveAttribute('aria-pressed', next === 'off' ? 'false' : 'true');
      await expect(page.locator('.player-repeat .player-mode-dot')).toHaveCount(next === 'off' ? 0 : 1);
    }

    // Queue and both Now Playing entry points open real root surfaces.
    await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
    const queue = page.getByRole('complementary', { name: 'Playback queue', exact: true });
    await expect(queue).toBeVisible();
    await expect(queue.locator('.queue-row')).toHaveCount(3);
    await expect(queue.locator('.queue-row.current')).toHaveCount(1);
    await page.getByRole('button', { name: 'Close playback queue', exact: true }).click();
    await expect(queue).toHaveCount(0);
    await expect(page.locator('.now-playing strong.player-title')).toHaveText(longA.title);

    const playerAudio = await page.evaluate(() => {
      const element = document.querySelector('#vela-audio');
      return { src: element.getAttribute('src'), generation: element.getAttribute('data-source-generation'), time: element.currentTime };
    });
    await page.getByRole('button', { name: 'Open now playing', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Now playing', exact: true });
    await expect(dialog).toBeVisible();
    await expect.poll(() => dialog.evaluate(element => element.matches(':modal'))).toBeTruthy();
    await page.keyboard.press('Escape');
    await expect(dialog).toHaveCount(0);
    await page.getByRole('button', { name: 'Expand player', exact: true }).click();
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Close now playing', exact: true }).click();
    await expect(dialog).toHaveCount(0);
    const afterOverlays = await page.evaluate(() => {
      const element = document.querySelector('#vela-audio');
      return {
        same: element === window.__tr03Audio, src: element.getAttribute('src'),
        generation: element.getAttribute('data-source-generation'), time: element.currentTime,
      };
    });
    expect(afterOverlays.same).toBe(true);
    expect(afterOverlays.src).toBe(playerAudio.src);
    expect(afterOverlays.generation).toBe(playerAudio.generation);
    expect(Math.abs(afterOverlays.time - playerAudio.time)).toBeLessThan(0.6);

    // --- Exact player geometry at the reference, compact and phone widths.
    const viewports = [
      { width: 1284, height: 856, playerHeight: 88, desktop: true },
      { width: 1500, height: 900, playerHeight: 88, desktop: true },
      { width: 1100, height: 900, playerHeight: 88, desktop: true },
      { width: 760, height: 900, playerHeight: 88, desktop: true },
      { width: 390, height: 844, playerHeight: 120, desktop: false },
      { width: 320, height: 780, playerHeight: 120, desktop: false },
    ];
    const measurements = [];
    for (const viewport of viewports) {
      const { width, height, playerHeight, desktop } = viewport;
      await page.setViewportSize({ width, height });
      await settle(page);
      const measured = await page.evaluate(() => {
        const box = selector => {
          const element = document.querySelector(selector);
          if (!element) return null;
          const rect = element.getBoundingClientRect();
          const style = getComputedStyle(element);
          return {
            x: rect.x, y: rect.y, top: rect.top, left: rect.left, width: rect.width,
            height: rect.height, right: rect.right, bottom: rect.bottom,
            display: style.display, fontSize: parseFloat(style.fontSize),
            color: style.color, backgroundColor: style.backgroundColor,
            paddingLeft: parseFloat(style.paddingLeft), paddingTop: parseFloat(style.paddingTop),
            borderTopWidth: parseFloat(style.borderTopWidth), borderTopColor: style.borderTopColor,
            radius: style.borderRadius, fontVariantNumeric: style.fontVariantNumeric,
          };
        };
        const times = [...document.querySelectorAll('.player-time')];
        const volume = document.querySelector('.player-right input[aria-label="Volume"]');
        return {
          innerWidth, innerHeight,
          player: box('.player-bar'), now: box('.player-now'), center: box('.player-center'),
          right: box('.player-right'), artwork: box('.now-playing .artwork'),
          title: box('.now-playing strong.player-title'), artist: box('.now-playing .player-artist'),
          album: box('.now-playing .player-album'), eq: box('.player-eq'),
          favorite: box('.player-favorite'), toggle: box('#player-toggle'), face: box('.play-toggle-face'),
          previous: box('#player-previous'), next: box('#player-next'),
          shuffle: box('.player-shuffle'), repeat: box('.player-repeat'),
          mobileQueue: box('.player-buttons .mobile-queue'),
          time: times.length ? {
            fontSize: parseFloat(getComputedStyle(times[0]).fontSize),
            variant: getComputedStyle(times[0]).fontVariantNumeric,
          } : null,
          rail: box('.seek-rail'), seekInput: box('.seek-control input'),
          quality: box('.player-quality'), mute: box('.player-mute'), expand: box('.player-expand'),
          volume: volume ? { width: volume.getBoundingClientRect().width, value: volume.value } : null,
          nav: box('.mobile-nav'),
          overflow: document.documentElement.scrollWidth - innerWidth,
          titleText: document.querySelector('.now-playing strong.player-title')?.textContent,
          qualityText: document.querySelector('.player-quality-format')?.textContent,
          audio: (() => {
            const element = document.querySelector('#vela-audio');
            return {
              same: element === window.__tr03Audio, count: document.querySelectorAll('audio').length,
              src: element.getAttribute('src'), generation: element.getAttribute('data-source-generation'),
            };
          })(),
        };
      });
      measurements.push({ width, height, ...measured });
      expect.soft(measured.player.x, `${width} player is full width`).toBeCloseTo(0, 0.75);
      expect.soft(measured.player.width, `${width} player width`).toBeCloseTo(width, 0.75);
      expect.soft(measured.player.height, `${width} player height`).toBeCloseTo(playerHeight, 0.75);
      expect.soft(measured.overflow, `${width} no horizontal overflow`).toBeLessThanOrEqual(1);
      expect.soft(measured.audio.same, `${width} same root media element`).toBe(true);
      expect.soft(measured.audio.count, `${width} one audio element`).toBe(1);
      expect.soft(measured.audio.src, `${width} same source`).toBe(playerAudio.src);
      expect.soft(measured.audio.generation, `${width} same source generation`).toBe(playerAudio.generation);
      expect.soft(measured.titleText, `${width} current title`).toBe(longA.title);

      if (desktop) {
        expect.soft(measured.player.bottom, `${width} player at the end of the shell`)
          .toBeCloseTo(height - (measured.nav.display === 'none' ? 0 : measured.nav.height), 0.75);
        expect.soft(measured.player.paddingLeft, `${width} player gutter`).toBeCloseTo(26, 0.75);
        expect.soft(measured.player.paddingTop, `${width} player vertical padding`).toBeCloseTo(10, 0.75);
        expect.soft(measured.player.backgroundColor, `${width} player surface`).toBe('rgb(16, 19, 15)');
        expect.soft(measured.player.borderTopWidth, `${width} player top border`).toBeCloseTo(1, 0.75);
        expect.soft(measured.player.borderTopColor, `${width} player border color`).toBe('rgb(52, 59, 46)');
        expect.soft(Math.abs(measured.now.width - measured.right.width), `${width} symmetric side zones`).toBeLessThan(1.5);
        expect.soft(measured.now.left, `${width} left zone at padding`).toBeCloseTo(measured.player.left + 26, 0.75);
        expect.soft(measured.right.right, `${width} right zone at padding`).toBeCloseTo(measured.player.right - 26, 0.75);
        expect.soft(measured.center.width, `${width} transport max width`).toBeLessThanOrEqual(470.75);
        expect.soft(measured.center.right, `${width} center inside player`).toBeLessThanOrEqual(measured.player.right - 26 + 0.75);
        expect.soft(measured.now.right, `${width} left zone inside player`).toBeLessThanOrEqual(measured.player.right + 0.75);
        expect.soft(measured.artwork.width, `${width} artwork width`).toBeCloseTo(51, 0.75);
        expect.soft(measured.artwork.height, `${width} artwork height`).toBeCloseTo(51, 0.75);
        expect.soft(measured.artwork.radius, `${width} artwork radius`).toBe('6px');
        expect.soft(measured.title.fontSize, `${width} player title size`).toBeCloseTo(12, 0.5);
        expect.soft(measured.artist.fontSize, `${width} player artist size`).toBeCloseTo(10, 0.5);
        expect.soft(measured.toggle.width, `${width} play hit box width`).toBeGreaterThanOrEqual(44);
        expect.soft(measured.toggle.height, `${width} play hit box height`).toBeGreaterThanOrEqual(44);
        expect.soft(measured.face.width, `${width} 32px main face`).toBeCloseTo(32, 0.75);
        expect.soft(measured.face.height, `${width} 32px main face height`).toBeCloseTo(32, 0.75);
        expect.soft(measured.rail.height, `${width} 3px seek rail`).toBeCloseTo(3, 0.75);
        expect.soft(measured.seekInput.height, `${width} seek hit height`).toBeGreaterThanOrEqual(44);
        expect.soft(measured.time.fontSize, `${width} player time size`).toBeLessThanOrEqual(10.5);
        expect.soft(measured.time.variant, `${width} tabular times`).toBe('tabular-nums');
        expect.soft(measured.mute.display, `${width} mute present`).not.toBe('none');
        expect.soft(measured.volume.width, `${width} volume control`).toBeGreaterThanOrEqual(44);
        expect.soft(measured.favorite.display === 'none', `${width} favorite visibility`).toBe(width < 1101);
        expect.soft(measured.quality.display === 'none', `${width} quality visibility`).toBe(width < 1101);
        expect.soft(measured.expand.display === 'none', `${width} expand visibility`).toBe(width < 1101);
        if (width === 1500) expect.soft(measured.volume.width, '1500 volume 65').toBeCloseTo(65, 0.75);
        if (width === 1284) expect.soft(measured.qualityText, '1284 quality is real data').toBe(expectedQuality(longA));
        if (width === 760) expect.soft(measured.nav.display, '760 navigation visible').toBe('flex');
        expect.soft(measured.player.bottom + (measured.nav.display === 'none' ? 0 : measured.nav.height), `${width} bands tile the viewport exactly`)
          .toBeCloseTo(height, 0.75);
      } else {
        expect.soft(measured.nav.display, `${width} mobile navigation visible`).toBe('flex');
        expect.soft(measured.nav.height, `${width} navigation height`).toBeCloseTo(70, 0.75);
        expect.soft(measured.nav.bottom, `${width} navigation at viewport bottom`).toBeCloseTo(height, 0.75);
        expect.soft(measured.player.bottom, `${width} player ends above navigation`).toBeLessThanOrEqual(measured.nav.top + 0.75);
        expect.soft(measured.player.bottom, `${width} player adjacent to navigation`).toBeCloseTo(measured.nav.top, 0.75);
        expect.soft(measured.player.paddingLeft, `${width} mini-player gutter`).toBeCloseTo(16, 0.75);
        expect.soft(measured.artwork.width, `${width} mini-player artwork`).toBeCloseTo(42, 0.75);
        expect.soft(measured.artwork.radius, `${width} mini-player artwork radius`).toBe('5px');
        expect.soft(measured.artist.display, `${width} mini-player artist`).not.toBe('none');
        expect.soft(measured.album === null || measured.album.display === 'none', `${width} album yields on the mini-player`).toBe(true);
        expect.soft(measured.favorite === null || measured.favorite.display === 'none', `${width} mini-player favorite at ${width}`).toBe(width <= 360);
        expect.soft(measured.mobileQueue.display, `${width} mini-player queue`).not.toBe('none');
        expect.soft(measured.previous.display, `${width} previous stays in the expanded player`).toBe('none');
        expect.soft(measured.shuffle.display, `${width} shuffle stays in the expanded player`).toBe('none');
        expect.soft(measured.repeat.display, `${width} repeat stays in the expanded player`).toBe('none');
        expect.soft(measured.right.display, `${width} desktop volume moves to the expanded player`).toBe('none');
        expect.soft(measured.toggle.height, `${width} mobile play hit box`).toBeGreaterThanOrEqual(44);
        expect.soft(measured.seekInput.height, `${width} mobile seek hit height`).toBeGreaterThanOrEqual(44);
        expect.soft(measured.seekInput.bottom, `${width} seek inside the player`).toBeLessThanOrEqual(measured.player.bottom + 0.75);
        expect.soft(measured.eq.display, `${width} player state marker visible`).not.toBe('none');
      }
      await page.screenshot({ path: `${output}/player-${width}x${height}.png`, animations: 'disabled' });
    }

    // The mobile shell reserves the player and the navigation plus the safe area
    // exactly once, so the mini-player cannot sit behind the navigation.
    const css = (await (await page.request.get(runtime.origin + '/style.css')).text()).replace(/\s+/g, ' ');
    expect(css).toContain('calc(var(--vela-mobile-nav-height) + env(safe-area-inset-bottom))');

    // Queue and Now Playing remain reachable from the phone composition.
    await page.getByRole('button', { name: 'Show playback queue and controls', exact: true }).click();
    const mobileQueue = page.getByRole('complementary', { name: 'Playback queue', exact: true });
    await expect(mobileQueue).toBeVisible();
    await expect(mobileQueue.locator('.queue-row.current')).toHaveCount(1);
    await page.getByRole('button', { name: 'Close playback queue', exact: true }).click();
    await expect(mobileQueue).toHaveCount(0);
    await page.getByRole('button', { name: 'Open now playing', exact: true }).click();
    const mobileDialog = page.getByRole('dialog', { name: 'Now playing', exact: true });
    await expect(mobileDialog).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(mobileDialog).toHaveCount(0);
    expect(await page.evaluate(() => document.querySelectorAll('audio').length)).toBe(1);
    expect(await page.evaluate(() => document.querySelector('#vela-audio') === window.__tr03Audio)).toBe(true);

    await writeFile(`${output}/player-evidence.json`, JSON.stringify({ playerTrack, before, after, seek, measurements }, null, 2));
  } finally { await runtime.close(); }
});

test('TR04a Home composes bounded filter views from real DTOs and keeps one root player', async ({ page }) => {
  test.setTimeout(180000);
  const output = resolve(root, process.env.VELA_TRANSFER_EVIDENCE || 'target/checks/prototype-transfer-web');
  await mkdir(output, { recursive: true });
  const runtime = await signedInFixture(output);
  try {
    const setup = await page.request.post(runtime.origin + '/api/setup', {
      headers: { Origin: runtime.origin },
      data: { username: 'admin', password: 'private home filter fixture password' },
    });
    expect(setup.ok()).toBeTruthy();
    const me = await setup.json();
    const headers = { 'X-CSRF-Token': me.csrf_token, Origin: runtime.origin };

    // One covered release plus owner-edited albums: every shelf card below is a
    // real AlbumSummary. Only the embedded fixture art is a real image; the
    // untagged copies must fall back to the owned neutral mark, never to art.
    const addAlbum = async (index, seconds) => {
      const track = await uploadTrack(page, runtime, me.csrf_token,
        `Shelf ${String(index).padStart(2, '0')}.wav`, 'audio/wav', pcmWave(seconds, 430 + index * 19));
      const metadata = await (await page.request.get(`${runtime.origin}/api/tracks/${track.id}/metadata`)).json();
      const edited = await page.request.patch(`${runtime.origin}/api/tracks/${track.id}/metadata`, {
        headers,
        data: {
          revision: metadata.revision, title: track.title, artist: 'Shelf Artist',
          album: `Shelf album ${index}`, album_artist: 'Shelf Artist', genre: '',
          year: null, track_number: index, disc_number: 1,
        },
      });
      expect(edited.ok(), `album ${index}`).toBeTruthy();
      return track;
    };
    const covered = await uploadTrack(page, runtime, me.csrf_token, 'Covered original.flac', 'audio/flac',
      await readFile(COVERED_FLAC));
    expect(covered.artwork_id, 'fixture must carry embedded artwork').toBeTruthy();
    const shelfTracks = [];
    for (let at = 1; at <= 5; at++) shelfTracks.push(await addAlbum(at, at === 1 ? 60 : 4));
    const seeded = await (await page.request.get(runtime.origin + '/api/albums?limit=100')).json();
    expect(seeded.items.length).toBe(6);
    expect(seeded.items.some(album => album.artwork_id), 'at least one real cover').toBeTruthy();

    const imageRequests = [];
    const homeRequests = [];
    page.on('request', request => {
      imageRequests.push(request.url());
      const url = new URL(request.url());
      if (url.pathname === '/api/tracks' || url.pathname === '/api/albums') homeRequests.push(url.searchParams);
    });
    await page.setViewportSize({ width: 1284, height: 856 });
    // Pending Home must never render a no-results view: hold the real request
    // and open the filter views before it resolves.
    let releaseHome;
    const homeGate = new Promise(resolve => { releaseHome = resolve; });
    let heldHome = false;
    await page.route('**/api/tracks?*', async route => {
      const url = new URL(route.request().url());
      if (!heldHome && url.searchParams.get('sort') === 'newest') {
        heldHome = true;
        await homeGate;
      }
      await route.continue();
    });
    await page.goto(runtime.origin);
    await expect(page.locator('.app-shell')).toBeVisible();
    const tab = name => page.getByRole('tab', { name, exact: true });
    await expect(tab('Playlists')).toBeVisible();
    await tab('Playlists').click();
    await expect(page.getByRole('status', { name: 'Loading Home', exact: true })).toBeVisible();
    await expect(page.locator('.home-playlists-view .empty-state')).toHaveCount(0);
    await expect(page.locator('.home-playlists-view')).toHaveCount(0);
    await tab('Albums').click();
    await expect(page.locator('.track-skeleton-card')).toHaveCount(4);
    await expect(page.locator('.home-albums')).toHaveCount(0);
    releaseHome();
    await expect(page.locator('.home-albums')).toBeVisible();
    await page.unroute('**/api/tracks?*');
    await tab('For you').click();
    await settle(page);
    const initial = homeRequests.map(params => Object.fromEntries(params.entries()));
    expect(initial).toEqual(expect.arrayContaining([
      expect.objectContaining({ sort: 'newest', limit: '8' }),
      expect.objectContaining({ limit: '6' }),
    ]));
    expect(initial.every(params => params.limit && Number(params.limit) <= 8), 'bounded Home requests only').toBe(true);
    await expect(tab('For you')).toHaveAttribute('aria-selected', 'true');

    // The long real WAV carries the cross-view identity check: filters may not
    // create a second source, move the position or rebuild the queue.
    await navigate(page, 'Library');
    await page.getByRole('button', { name: `Play ${shelfTracks[0].title}`, exact: true }).click();
    await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0.2);
    await page.evaluate(() => {
      const element = document.querySelector('#vela-audio');
      element.dataset.tr04 = 'single-root-player';
      window.__tr04Audio = element;
    });
    await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
    const queue = page.getByRole('complementary', { name: 'Playback queue', exact: true });
    await expect(queue.locator('.queue-row')).toHaveCount(6);
    await page.getByRole('button', { name: 'Close playback queue', exact: true }).click();
    await navigate(page, 'Home');
    await expect(page.getByRole('heading', { name: 'New in your library', exact: true })).toBeVisible();

    const readPlayer = () => page.evaluate(() => {
      const element = document.querySelector('#vela-audio');
      return {
        same: element === window.__tr04Audio, marker: element.dataset.tr04,
        count: document.querySelectorAll('audio').length,
        src: element.getAttribute('src'), generation: element.getAttribute('data-source-generation'),
        paused: element.paused, time: element.currentTime,
        title: document.querySelector('.now-playing strong.player-title')?.textContent,
      };
    });
    const assertPlayerHeld = async (label, before) => {
      const after = await readPlayer();
      expect.soft(after.same, `${label} same root element`).toBe(true);
      expect.soft(after.marker, `${label} root marker`).toBe('single-root-player');
      expect.soft(after.count, `${label} one audio element`).toBe(1);
      expect.soft(after.src, `${label} source held`).toBe(before.src);
      expect.soft(after.generation, `${label} generation held`).toBe(before.generation);
      expect.soft(after.title, `${label} current occurrence held`).toBe(before.title);
      expect.soft(after.time, `${label} position not reset`).toBeGreaterThanOrEqual(before.time - 0.35);
      return after;
    };

    // --- Approved Home header, one real filter group, the real feature pair.
    await expect(page.locator('h1')).toHaveCount(1);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText('In your own rhythm');
    await expect(page.locator('.home-heading .eyebrow')).toHaveText('MUSIC. ONLY YOURS.');
    await expect(page.locator('.home-private')).toContainText('Private space');
    await expect(page.locator('.home-private .vela-icon')).toHaveCount(1);
    await expect(page.locator('.home-title-dot')).toHaveCount(1);
    // TR04b: the approved coast/wave artwork now ships, so the honest absent
    // state is gone. With no playlist yet there is no editorial story, and the
    // card says so instead of showing a decorative image with a dead control.
    await expect(page.locator('[data-image-blocked]')).toHaveCount(0);
    await expect(page.locator('.home-features')).toBeVisible();
    await expect(page.locator('.home-feature-empty')).toHaveCount(1);
    await expect(page.locator('.home-feature-empty')).toContainText('Выберите музыку для этой подборки.');
    await expect(page.locator('.home-feature-empty .home-feature-listen')).toHaveCount(0);
    await expect(page.locator('.home-feature-empty')).toContainText('Открыть плейлисты');
    await expect(page.locator('.home-wave')).toHaveCount(1);
    for (const asset of ['/assets/brand/vela-coast.png', '/assets/brand/vela-wave.png']) {
      const response = await page.request.get(runtime.origin + asset);
      expect(response.status(), asset).toBe(200);
      expect(response.headers()['content-type'], asset).toMatch(/^image\//);
    }
    // The unlisted provenance note next to the shipped art stays denied.
    expect((await page.request.get(runtime.origin + '/assets/brand/SOURCE.md')).status()).toBe(404);

    const forYou = page.getByRole('tab', { name: 'For you', exact: true });
    const albumsTab = page.getByRole('tab', { name: 'Albums', exact: true });
    const playlistsTab = page.getByRole('tab', { name: 'Playlists', exact: true });
    await expect(page.getByRole('tablist', { name: 'Home views', exact: true })).toBeVisible();
    await expect(forYou).toHaveCount(1);
    await expect(albumsTab).toHaveCount(1);
    await expect(playlistsTab).toHaveCount(1);
    await expect(forYou).toHaveAttribute('aria-selected', 'true');
    await expect(forYou).toHaveAttribute('tabindex', '0');
    await expect(albumsTab).toHaveAttribute('aria-selected', 'false');
    await expect(albumsTab).toHaveAttribute('tabindex', '-1');
    const filter = await page.evaluate(() => {
      const tabs = [...document.querySelectorAll('.home-filter')];
      const group = document.querySelector('.home-filter-group');
      const box = tabs[0].getBoundingClientRect();
      const style = getComputedStyle(tabs[0]);
      const inactive = getComputedStyle(tabs[1]);
      return {
        role: group.getAttribute('role'), name: group.getAttribute('aria-label'),
        labels: tabs.map(tab => tab.textContent.trim()),
        height: box.height, paddingLeft: parseFloat(style.paddingLeft), paddingRight: parseFloat(style.paddingRight),
        radius: style.borderRadius, gap: parseFloat(getComputedStyle(group).gap),
        activeBackground: style.backgroundColor, activeColor: style.color,
        inactiveBackground: inactive.backgroundColor, inactiveColor: inactive.color,
        panelRole: document.querySelector('#home-view-panel').getAttribute('role'),
        panelLabelledBy: document.querySelector('#home-view-panel').getAttribute('aria-labelledby'),
      };
    });
    expect(filter.role).toBe('tablist');
    expect(filter.name).toBe('Home views');
    expect(filter.labels).toEqual(['For you', 'Albums', 'Playlists']);
    expect(filter.height).toBeCloseTo(30, 0.75);
    expect(filter.paddingLeft).toBeCloseTo(14, 0.75);
    expect(filter.paddingRight).toBeCloseTo(14, 0.75);
    expect(filter.radius).toBe('5px');
    expect(filter.gap).toBeCloseTo(7, 0.75);
    expect(filter.activeBackground).toBe('rgb(211, 233, 165)');
    expect(filter.activeColor).toBe('rgb(27, 37, 17)');
    expect(filter.inactiveBackground).toBe('rgb(34, 39, 31)');
    expect(filter.inactiveColor).toBe('rgb(154, 162, 150)');
    expect(filter.panelRole).toBe('tabpanel');
    expect(filter.panelLabelledBy).toBe('home-tab-for-you');

    // --- For you: real additions, a six-cover shelf and no invented art.
    const shelf = page.locator('.home-album-shelf');
    await expect(shelf).toBeVisible();
    await expect(shelf.locator('.home-album-card')).toHaveCount(6);
    const listed = await (await page.request.get(runtime.origin + '/api/albums?limit=6')).json();
    expect(listed.items.length).toBe(6);
    const cards = await shelf.locator('.home-album-card').evaluateAll(nodes => nodes.map(node => {
      const cover = node.querySelector('.catalog-cover');
      const image = cover.querySelector('img');
      const box = cover.getBoundingClientRect();
      const style = getComputedStyle(node);
      return {
        id: node.dataset.albumId, cover: image ? image.getAttribute('src') : null,
        fallback: Boolean(cover.querySelector('.vela-icon')),
        coverWidth: box.width, coverHeight: box.height, radius: getComputedStyle(cover).borderRadius,
        border: ['Top', 'Right', 'Bottom', 'Left'].map(side => parseFloat(style[`border${side}Width`])),
        background: style.backgroundColor,
      };
    }));
    expect(cards.map(card => card.id)).toEqual(listed.items.map(album => album.id));
    for (const album of listed.items) {
      const card = cards.find(item => item.id === album.id);
      expect(card.cover, `${album.title} real cover`).toBe(album.artwork_id ? `/api/artwork/${album.artwork_id}` : null);
      expect(card.fallback, `${album.title} owned fallback only without art`).toBe(album.artwork_id === null);
    }
    for (const card of cards) {
      expect.soft(Math.abs(card.coverWidth - card.coverHeight), `${card.id} square art`).toBeLessThanOrEqual(1);
      expect.soft(card.radius, `${card.id} art radius`).toBe('7px');
      expect.soft(card.border, `${card.id} borderless`).toEqual([0, 0, 0, 0]);
      expect.soft(card.background, `${card.id} no raised tile`).toBe('rgba(0, 0, 0, 0)');
    }
    expect(cards.some(card => card.cover), 'displayed shelf contains real artwork').toBeTruthy();
    await expect.poll(() => page.locator('.home-album-card .catalog-cover img').evaluateAll(
      nodes => nodes.every(image => image.complete && image.naturalWidth > 0))).toBeTruthy();
    await expect(page.locator('[aria-labelledby="home-additions"]')).toBeVisible();
    await expect(page.locator('.personal-mixes')).toBeVisible();
    await page.screenshot({ path: `${output}/home-for-you-1284.png`, animations: 'disabled' });

    // --- Filter switching rearranges existing DTOs only: no catalog request,
    // no second source, no position reset.
    const before = await readPlayer();
    expect(before.paused).toBe(false);
    const catalogRequests = [];
    page.on('request', request => {
      const path = new URL(request.url()).pathname;
      if (path === '/api/tracks' || path === '/api/albums' || path === '/api/playlists') catalogRequests.push(path);
    });

    await albumsTab.click();
    await expect(albumsTab).toHaveAttribute('aria-selected', 'true');
    await expect(albumsTab).toHaveAttribute('tabindex', '0');
    await expect(forYou).toHaveAttribute('aria-selected', 'false');
    await expect(page.locator('#home-view-panel')).toHaveAttribute('aria-labelledby', 'home-tab-albums');
    await expect(page.locator('.home-albums')).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Your albums', exact: true })).toBeVisible();
    await expect(page.locator('[data-image-blocked]')).toHaveCount(0);
    await expect(page.locator('[aria-labelledby="home-additions"]')).toBeVisible();
    await expect(page.locator('.personal-mixes')).toBeHidden();
    await assertPlayerHeld('albums tab', before);
    await page.screenshot({ path: `${output}/home-albums-1284.png`, animations: 'disabled' });

    await playlistsTab.click();
    await expect(playlistsTab).toHaveAttribute('aria-selected', 'true');
    await expect(page.locator('#home-view-panel')).toHaveAttribute('aria-labelledby', 'home-tab-playlists');
    const playlistsView = page.locator('.home-playlists-view');
    await expect(playlistsView).toBeVisible();
    await expect(page.getByRole('heading', { name: 'For every mood', exact: true })).toBeVisible();
    await expect(page.locator('.home-albums')).toHaveCount(0);
    await expect(page.locator('[aria-labelledby="home-additions"]')).toHaveCount(0);
    const emptyPlaylists = playlistsView.locator('.empty-state');
    await expect(emptyPlaylists).toBeVisible();
    await expect(emptyPlaylists).toContainText('no playlists yet');
    await expect(page.getByRole('button', { name: 'Create a playlist', exact: true })).toBeVisible();
    await assertPlayerHeld('playlists tab', before);
    await page.screenshot({ path: `${output}/home-playlists-empty-1284.png`, animations: 'disabled' });

    // --- Keyboard state: roving tabindex with real focus movement.
    await forYou.click();
    await expect(forYou).toBeFocused();
    await page.keyboard.press('ArrowRight');
    await expect(albumsTab).toBeFocused();
    await expect(albumsTab).toHaveAttribute('aria-selected', 'true');
    await expect(forYou).toHaveAttribute('tabindex', '-1');
    await expect(page.locator('#home-view-panel')).toHaveAttribute('aria-labelledby', 'home-tab-albums');
    await page.keyboard.press('ArrowLeft');
    await expect(forYou).toBeFocused();
    await expect(forYou).toHaveAttribute('aria-selected', 'true');
    await page.keyboard.press('End');
    await expect(playlistsTab).toBeFocused();
    await expect(playlistsTab).toHaveAttribute('aria-selected', 'true');
    await page.keyboard.press('Home');
    await expect(forYou).toBeFocused();
    await expect(forYou).toHaveAttribute('aria-selected', 'true');
    await expect(page.locator('.home-features')).toBeVisible();
    await assertPlayerHeld('keyboard filters', before);
    expect(catalogRequests, 'view switches reuse loaded DTOs').toEqual([]);
    await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
    const queueAfterFilters = page.getByRole('complementary', { name: 'Playback queue', exact: true });
    await expect(queueAfterFilters.locator('.queue-row')).toHaveCount(6);
    await expect(queueAfterFilters.locator('.queue-row.current')).toHaveCount(1);
    await page.getByRole('button', { name: 'Close playback queue', exact: true }).click();

    // Theme and device scale are presentation only: same element and source.
    await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
    const lightFill = await page.evaluate(() => getComputedStyle(
      document.querySelector('.home-filter[aria-selected="true"]')).backgroundColor);
    expect(lightFill).toBe('rgb(8, 117, 76)');
    await page.screenshot({ path: `${output}/home-light-1284.png`, animations: 'disabled' });
    await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
    await page.evaluate(() => document.documentElement.style.setProperty('--app-zoom', '0.8'));
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await assertPlayerHeld('app scale', before);
    await page.evaluate(() => document.documentElement.style.removeProperty('--app-zoom'));
    await expect(page.locator('.home-filter[aria-selected="true"]')).toHaveCount(1);

    // --- Real playlist create/open path: the create control opens the real
    // form, the created playlist appears in Home and opens its real detail.
    await playlistsTab.click();
    await expect(emptyPlaylists).toBeVisible();
    await page.getByRole('button', { name: 'Create a playlist', exact: true }).click();
    await expect(page.locator('#playlist-name')).toBeVisible();
    await page.locator('#playlist-name').fill('Evening shelf');
    await page.getByRole('button', { name: '+ Create', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Evening shelf', exact: true })).toBeVisible();
    await navigate(page, 'Home');
    await page.getByRole('tab', { name: 'Playlists', exact: true }).click();
    const playlistCard = page.locator('[data-playlist-id][aria-label="Open playlist Evening shelf"]');
    await expect(playlistCard).toBeVisible();
    await expect(playlistCard).toContainText('0 tracks');
    await page.screenshot({ path: `${output}/home-playlists-1284.png`, animations: 'disabled' });
    await playlistCard.click();
    await expect(page.locator('.playlist-detail')).toBeVisible();
    await expect(page.locator('.playlist-detail h2')).toHaveText('Evening shelf');

    await navigate(page, 'Home');
    await page.getByRole('tab', { name: 'For you', exact: true }).click();

    // --- TR04b: with a real owned playlist the editorial card becomes the
    // real feature. Its title is the playlist's own name, the approved coast
    // artwork paints behind it, and the story indicator counts real targets.
    const feature = page.locator('.home-feature');
    await expect(feature).toBeVisible();
    await expect(page.locator('.home-feature-empty')).toHaveCount(0);
    await expect(feature.locator('.home-feature-title')).toHaveText('Evening shelf');
    const brandImages = await page.evaluate(() => {
      const read = element => {
        const style = getComputedStyle(element);
        return {
          image: style.backgroundImage,
          position: style.backgroundPosition,
          size: style.backgroundSize,
          radius: style.borderRadius,
          height: element.getBoundingClientRect().height,
        };
      };
      return {
        feature: read(document.querySelector('.home-feature')),
        wave: read(document.querySelector('.home-wave')),
      };
    });
    expect(brandImages.feature.image).toContain('/assets/brand/vela-coast.png');
    expect(brandImages.feature.position).toBe('60% 53%');
    expect(brandImages.feature.size).toBe('cover');
    expect(brandImages.feature.radius).toBe('10px');
    expect(brandImages.feature.height).toBeCloseTo(250, 0.5);
    expect(brandImages.wave.image).toContain('/assets/brand/vela-wave.png');
    expect(brandImages.wave.height).toBeCloseTo(250, 0.5);
    // One real target means exactly one indicator: no dead dots.
    const indicators = feature.locator('.home-indicator');
    await expect(indicators).toHaveCount(1);
    await expect(indicators.first()).toHaveAttribute('aria-current', 'true');
    await expect(indicators.first()).toHaveAttribute('aria-label', 'Показать Evening shelf');
    // The Listen control is real: it plays the playlist's own tracks.
    await expect(feature.locator('.home-feature-listen')).toBeEnabled();
    await expect(feature.locator('.home-feature-arrow')).toBeEnabled();
    // The My Wave card is present and its play control reflects the real state.
    await expect(page.locator('.home-wave')).toBeVisible();
    await expect(page.locator('.home-wave-play')).toBeVisible();

    // --- The shelf stays capped at six even when the library grows past it.
    await addAlbum(6, 4);
    await addAlbum(7, 4);
    const grown = await (await page.request.get(runtime.origin + '/api/albums?limit=100')).json();
    expect(grown.items.length).toBe(8);
    await navigate(page, 'Home');
    await expect(page.locator('.home-album-shelf .home-album-card')).toHaveCount(6);
    await expect(page.locator('.home-album-shelf')).toBeVisible();
    expect(await page.locator('.home-album-shelf .home-album-card').count()).toBeLessThanOrEqual(6);
    expect(imageRequests.some(url => /(coast|your-wave)\.png/i.test(url))).toBe(true);
    await expect(page.locator('.now-playing strong.player-title')).toHaveText(shelfTracks[0].title);
    const final = await readPlayer();
    expect(final.same).toBe(true);
    expect(final.count).toBe(1);
    expect(final.src).toBe(before.src);
    expect(final.title).toBe(before.title);
    await page.screenshot({ path: `${output}/home-bounded-1284.png`, animations: 'disabled' });

    // Phone composition: the tabs stay reachable and the shelf reflows in pairs.
    await page.setViewportSize({ width: 390, height: 844 });
    await settle(page);
    await expect(page.getByRole('tab', { name: 'For you', exact: true })).toBeVisible();
    const phone = await page.evaluate(() => {
      const tabs = [...document.querySelectorAll('.home-filter')];
      return {
        overflow: document.documentElement.scrollWidth - innerWidth,
        tabWidths: tabs.map(tab => tab.getBoundingClientRect().width),
        columns: getComputedStyle(document.querySelector('.home-album-shelf')).gridTemplateColumns.split(' ').length,
        caption: getComputedStyle(document.querySelector('.home-filter-caption')).display,
      };
    });
    expect(phone.overflow).toBeLessThanOrEqual(1);
    expect(phone.tabWidths.every(width => width > 0)).toBe(true);
    expect(phone.columns).toBe(2);
    expect(phone.caption).toBe('none');
    await page.screenshot({ path: `${output}/home-for-you-390.png`, animations: 'disabled' });

    await writeFile(`${output}/home-filter-evidence.json`, JSON.stringify({ filter, cards, phone, absent: 'IMAGE_BLOCKED' }, null, 2));
  } finally { await runtime.close(); }
});

/**
 * TR07b: the row More menu is one signal-backed menu that changes surface, not
 * meaning. At touch widths it is a modal bottom action sheet anchored above the
 * reserved player + navigation bands (backdrop, safe-area aware, full-width
 * >=44px rows); on desktop it keeps the compact anchored panel. Both surfaces
 * dispatch the same real root commands, so the sheet can never become a second
 * audio owner, a new API or a decorative action list.
 */
test('TR07b row More menu is a modal bottom action sheet on phones and an anchored menu on desktop', async ({ page }) => {
  test.setTimeout(180000);
  const output = resolve(root, process.env.VELA_TRANSFER_EVIDENCE || 'target/checks/prototype-transfer-web');
  await mkdir(output, { recursive: true });
  const runtime = await signedInFixture(output);
  try {
    await setupWithTracks(page, runtime, 6);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(6);

    const row = page.locator('.track-row').first();
    const menuTitle = (await row.locator('.track-title strong').innerText()).trim();
    expect(menuTitle.length).toBeGreaterThan(0);
    const trigger = row.getByRole('button', { name: `More actions for ${menuTitle}`, exact: true });
    await expect(trigger).toHaveAttribute('aria-haspopup', 'menu');
    await expect(trigger).toHaveAttribute('aria-expanded', 'false');

    const measure = () => page.evaluate(() => {
      const box = element => {
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return {
          left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom,
          width: rect.width, height: rect.height,
          position: style.position, display: style.display,
          zIndex: parseInt(style.zIndex, 10),
          backgroundColor: style.backgroundColor,
          paddingLeft: parseFloat(style.paddingLeft), paddingRight: parseFloat(style.paddingRight),
        };
      };
      const panel = document.querySelector('.track-menu-panel');
      const backdrop = document.querySelector('.track-menu-backdrop');
      const player = document.querySelector('.player-bar');
      const nav = document.querySelector('.mobile-nav');
      const trigger = document.querySelector(`[data-track-menu] button[aria-haspopup="menu"]`);
      return {
        viewport: { width: innerWidth, height: innerHeight },
        panel: panel ? box(panel) : null,
        backdrop: backdrop ? box(backdrop) : null,
        player: player ? box(player) : null,
        nav: nav ? box(nav) : null,
        trigger: trigger ? box(trigger) : null,
        panelRole: panel ? panel.getAttribute('role') : null,
        panelLabel: panel ? panel.getAttribute('aria-label') : null,
        rows: panel ? [...panel.querySelectorAll(':scope > button')].map(button => ({
          label: button.textContent.trim(), role: button.getAttribute('role'), ...box(button),
        })) : [],
        panelOverflow: panel ? panel.scrollWidth - panel.clientWidth : null,
        documentOverflow: document.documentElement.scrollWidth - innerWidth,
        bodyOverflow: document.body.scrollWidth - innerWidth,
        audios: document.querySelectorAll('audio').length,
        paused: document.querySelector('#vela-audio').paused,
        activeIsPanel: !!panel && document.activeElement === panel,
      };
    });

    // TR07 registers the row command table once: the sheet, the anchored panel,
    // right-click, Shift+F10 and the ContextMenu key all render this exact list.
    const actions = ['Play / Pause', 'Add to queue', 'Favorite / Unfavorite', 'Add to playlist…', 'Start track radio', 'Edit metadata'];

    // --- Phone: right-click opens the sheet without starting playback.
    await row.click({ button: 'right', position: { x: 6, y: 6 } });
    await expect(page.locator('.track-menu-panel')).toBeVisible();
    await expect(page.locator('.track-menu-backdrop')).toBeVisible();
    await expect(trigger).toHaveAttribute('aria-expanded', 'true');
    await expect.poll(() => page.evaluate(() => document.activeElement === document.querySelector('.track-menu-panel'))).toBe(true);
    const phone = await measure();

    expect(phone.panelRole).toBe('menu');
    expect(phone.panelLabel).toBe(`Actions for ${menuTitle}`);
    expect(phone.panel.position, 'phone menu is a fixed sheet').toBe('fixed');
    expect(phone.panel.width).toBeCloseTo(390, 0);
    expect(Math.abs(phone.panel.left)).toBeLessThanOrEqual(1);
    expect(Math.abs(phone.panel.right - 390)).toBeLessThanOrEqual(1);
    // The shell reserves player + navigation (+ one safe area); the sheet ends
    // exactly at the player band's top edge instead of hiding the transport.
    expect(phone.player.height).toBeCloseTo(120, 0);
    expect(phone.nav.height).toBeCloseTo(70, 0);
    expect(Math.abs(phone.panel.bottom - phone.player.top)).toBeLessThanOrEqual(1);
    expect(phone.panel.bottom).toBeLessThanOrEqual(phone.nav.top + 1);
    expect(phone.panel.top).toBeGreaterThanOrEqual(0);

    expect(phone.backdrop.display).toBe('block');
    expect(phone.backdrop.position).toBe('fixed');
    expect(Math.abs(phone.backdrop.left)).toBeLessThanOrEqual(1);
    expect(Math.abs(phone.backdrop.top)).toBeLessThanOrEqual(1);
    expect(phone.backdrop.width).toBeCloseTo(390, 0);
    expect(phone.backdrop.height).toBeCloseTo(844, 0);
    expect(phone.backdrop.backgroundColor).not.toBe('rgba(0, 0, 0, 0)');
    expect(phone.backdrop.zIndex).toBeGreaterThan(phone.player.zIndex);
    expect(phone.backdrop.zIndex).toBeLessThan(phone.panel.zIndex);

    expect(phone.rows.map(action => action.label)).toEqual(actions);
    for (const action of phone.rows) {
      expect(action.role).toBe('menuitem');
      expect(action.height, `${action.label} touch row`).toBeGreaterThanOrEqual(44);
      expect(
        Math.abs(action.width - (phone.panel.width - phone.panel.paddingLeft - phone.panel.paddingRight)),
        `${action.label} full-width row`,
      ).toBeLessThanOrEqual(1);
    }
    expect(phone.panelOverflow).toBeLessThanOrEqual(1);
    expect(phone.documentOverflow).toBeLessThanOrEqual(1);
    expect(phone.bodyOverflow).toBeLessThanOrEqual(1);
    expect(phone.audios, 'the sheet never owns a second media element').toBe(1);
    expect(phone.paused, 'right-click must not start playback').toBe(true);
    await page.screenshot({ path: `${output}/track-menu-sheet-390.png`, animations: 'disabled' });

    // --- Escape closes the sheet and restores the trigger's expanded state.
    await page.keyboard.press('Escape');
    await expect(page.locator('.track-menu-panel')).toHaveCount(0);
    await expect(page.locator('.track-menu-backdrop')).toHaveCount(0);
    await expect(trigger).toHaveAttribute('aria-expanded', 'false');
    await expect(trigger).toHaveAttribute('aria-haspopup', 'menu');

    // --- Visible More opens it again; a backdrop tap closes it.
    await trigger.click();
    await expect(page.locator('.track-menu-panel')).toBeVisible();
    await expect(trigger).toHaveAttribute('aria-expanded', 'true');
    await page.mouse.click(195, 90);
    await expect(page.locator('.track-menu-panel')).toHaveCount(0);
    await expect(trigger).toHaveAttribute('aria-expanded', 'false');

    // --- An action closes the sheet and runs the real root command.
    await trigger.click();
    const panel = page.locator('.track-menu-panel');
    await expect(panel).toBeVisible();
    await panel.getByRole('menuitem', { name: 'Play / Pause', exact: true }).click();
    await expect(page.locator('.track-menu-panel')).toHaveCount(0);
    const audio = page.locator('#vela-audio');
    await expect.poll(() => audio.evaluate(node => node.currentTime), { timeout: 15000 }).toBeGreaterThan(0.2);
    await expect(page.locator('.now-playing strong.player-title')).toHaveText(menuTitle);
    expect(await page.evaluate(() => document.querySelectorAll('audio').length)).toBe(1);

    // --- Desktop keeps the compact menu anchored to its own trigger.
    await page.setViewportSize({ width: 1284, height: 856 });
    await settle(page);
    await trigger.click();
    await expect(page.locator('.track-menu-panel')).toBeVisible();
    await expect.poll(() => page.evaluate(() => document.activeElement === document.querySelector('.track-menu-panel'))).toBe(true);
    const desktop = await measure();
    expect(desktop.panelRole).toBe('menu');
    expect(desktop.panel.position, 'desktop menu stays the anchored panel').toBe('absolute');
    expect(desktop.panel.width).toBeGreaterThanOrEqual(190);
    expect(desktop.panel.width).toBeLessThan(400);
    expect(desktop.backdrop.display, 'no sheet backdrop on desktop').toBe('none');
    expect(Math.abs(desktop.panel.right - desktop.trigger.right)).toBeLessThanOrEqual(3);
    expect(desktop.panel.top).toBeGreaterThanOrEqual(desktop.trigger.bottom - 1);
    expect(desktop.panel.top - desktop.trigger.bottom).toBeLessThanOrEqual(12);
    expect(desktop.panel.left).toBeGreaterThanOrEqual(0);
    expect(desktop.panel.right).toBeLessThanOrEqual(1284);
    expect(desktop.rows.map(action => action.label)).toEqual(actions);
    expect(desktop.documentOverflow).toBeLessThanOrEqual(1);
    expect(desktop.audios).toBe(1);
    await page.screenshot({ path: `${output}/track-menu-anchored-1284.png`, animations: 'disabled' });

    // --- The surface switch is a real responsive boundary, not a second menu:
    // the same open menu is an anchored panel at 671px and a sheet at 670px.
    const surface = () => page.evaluate(() => {
      const panel = document.querySelector('.track-menu-panel');
      const backdrop = document.querySelector('.track-menu-backdrop');
      return {
        position: getComputedStyle(panel).position,
        backdrop: getComputedStyle(backdrop).display,
        rowHeight: panel.querySelector(':scope > button').getBoundingClientRect().height,
        panelBottom: panel.getBoundingClientRect().bottom,
        playerTop: document.querySelector('.player-bar').getBoundingClientRect().top,
        width: panel.getBoundingClientRect().width,
      };
    });
    await page.setViewportSize({ width: 671, height: 844 });
    await settle(page);
    const above = await surface();
    expect(above.position).toBe('absolute');
    expect(above.backdrop).toBe('none');
    await page.setViewportSize({ width: 670, height: 844 });
    await settle(page);
    const at = await surface();
    expect(at.position).toBe('fixed');
    expect(at.backdrop).toBe('block');
    expect(at.width).toBeCloseTo(670, 0);
    expect(Math.abs(at.panelBottom - at.playerTop)).toBeLessThanOrEqual(1);
    expect(at.rowHeight).toBeGreaterThanOrEqual(44);
    await page.setViewportSize({ width: 1284, height: 856 });
    await settle(page);
    expect((await surface()).position).toBe('absolute');

    await page.keyboard.press('Escape');
    await expect(page.locator('.track-menu-panel')).toHaveCount(0);
    await expect(trigger).toHaveAttribute('aria-expanded', 'false');

    await writeFile(`${output}/track-menu-evidence.json`, JSON.stringify({ actions, menuTitle, phone, desktop }, null, 2));
  } finally { await runtime.close(); }
});

/**
 * TR07c: the row menu has a real keyboard path and the add-to-playlist step is a
 * real searchable picker, not a decorative select.
 *
 * Shift+F10 and the ContextMenu key open exactly the menu the visible More
 * trigger, right-click and the touch sheet render (one registered action list),
 * never start playback, trap Tab, and restore focus to the invoking trigger.
 * The picker lists only playlists the server says this account may edit, filters
 * them by real search text, navigates by keyboard, and commits one explicit Add
 * through the existing revision/mutation CAS. Its receipt and the Undo inverse
 * are checked against actual playlist state, not against local UI state.
 */
test('TR07c keyboard row menu and the real searchable add-to-playlist picker', async ({ page }) => {
  test.setTimeout(240000);
  const output = resolve(root, process.env.VELA_TRANSFER_EVIDENCE || 'target/checks/prototype-transfer-web');
  await mkdir(output, { recursive: true });
  const runtime = await signedInFixture(output);
  try {
    await setupWithTracks(page, runtime, 4);
    const auth = await (await page.request.get(runtime.origin + '/api/me')).json();
    const createPlaylist = async (name) => {
      const response = await page.request.post(runtime.origin + '/api/playlists', {
        headers: { 'X-CSRF-Token': auth.csrf_token, Origin: runtime.origin },
        data: { name },
      });
      expect(response.ok()).toBeTruthy();
      return response.json();
    };
    const destination = await createPlaylist('Night drive');
    await createPlaylist('Deep focus');
    const playlistItems = async (id) => (await (await page.request.get(`${runtime.origin}/api/playlists/${id}`)).json()).items;

    await page.setViewportSize({ width: 1284, height: 856 });
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(4);

    const rows = page.locator('.track-row');
    const firstRow = rows.first();
    const menuTitle = (await firstRow.locator('.track-title strong').innerText()).trim();
    const playButton = firstRow.locator('.track-play');
    const trigger = firstRow.getByRole('button', { name: `More actions for ${menuTitle}`, exact: true });
    const panel = page.locator('.track-menu-panel');
    const audio = page.locator('#vela-audio');
    const noPlayback = async () => {
      expect(await audio.evaluate(node => node.paused), 'menu interactions never start playback').toBe(true);
      await expect(page.locator('.player-title')).toHaveCount(0);
      await expect(page.locator('.now-playing')).toContainText('Ready when you are');
    };

    // --- Shift+F10 on a focused row control opens the anchored row menu.
    await playButton.focus();
    await page.keyboard.press('Shift+F10');
    await expect(panel).toBeVisible();
    await expect(panel).toHaveAttribute('role', 'menu');
    await expect(panel).toHaveAttribute('aria-label', `Actions for ${menuTitle}`);
    await expect(trigger).toHaveAttribute('aria-expanded', 'true');
    await expect.poll(() => page.evaluate(() => document.activeElement === document.querySelector('.track-menu-panel'))).toBe(true);
    await noPlayback();

    // Space and Arrow transport shortcuts must not reach the global player from
    // behind the open menu.
    await page.keyboard.press('Space');
    await page.keyboard.press('ArrowRight');
    await page.keyboard.press('ArrowRight');
    expect(await audio.evaluate(node => node.paused)).toBe(true);
    expect(await audio.evaluate(node => node.currentTime)).toBe(0);
    await expect(panel).toBeVisible();
    await noPlayback();

    // --- Tab/Shift+Tab cycle inside the open menu.
    const focusedId = () => page.evaluate(() => document.activeElement?.id || '');
    await page.keyboard.press('Tab');
    expect(await focusedId()).toBe('row-action-play');
    for (const id of ['queue', 'favorite', 'playlist', 'radio', 'edit']) {
      await page.keyboard.press('Tab');
      expect(await focusedId()).toBe(`row-action-${id}`);
    }
    await page.keyboard.press('Tab');
    expect(await focusedId(), 'Tab wraps to the first action').toBe('row-action-play');
    await page.keyboard.press('Shift+Tab');
    expect(await focusedId(), 'Shift+Tab wraps to the last action').toBe('row-action-edit');
    await expect(panel).toBeVisible();

    // --- Escape closes it and returns focus to the invoking control.
    await page.keyboard.press('Escape');
    await expect(panel).toHaveCount(0);
    await expect(trigger).toHaveAttribute('aria-expanded', 'false');
    await expect.poll(() => page.evaluate(() => document.activeElement?.className?.includes('track-play'))).toBe(true);
    expect(await focusedId()).toBe('');

    // --- The ContextMenu key opens the same menu for the focused row.
    const secondRow = rows.nth(1);
    const secondTitle = (await secondRow.locator('.track-title strong').innerText()).trim();
    const secondPlay = secondRow.locator('.track-play');
    await secondPlay.focus();
    await page.keyboard.press('ContextMenu');
    await expect(panel).toBeVisible();
    await expect(panel).toHaveAttribute('aria-label', `Actions for ${secondTitle}`);
    await expect.poll(() => page.evaluate(() => document.activeElement === document.querySelector('.track-menu-panel'))).toBe(true);
    await noPlayback();
    await page.keyboard.press('Escape');
    await expect(panel).toHaveCount(0);

    // --- Right-click still opens that row's menu and starts nothing.
    await playButton.focus();
    await rows.nth(2).click({ button: 'right', position: { x: 6, y: 6 } });
    await expect(panel).toBeVisible();
    await noPlayback();
    await expect.poll(() => page.evaluate(() => document.activeElement?.id === 'track-menu-panel')).toBe(true);
    await page.keyboard.press('Escape');
    await expect(panel).toHaveCount(0);

    // --- Every surface renders the one registered action list.
    await trigger.click();
    await expect(panel).toBeVisible();
    await expect(panel.getByRole('menuitem')).toHaveText([
      'Play / Pause', 'Add to queue', 'Favorite / Unfavorite', 'Add to playlist…', 'Start track radio', 'Edit metadata',
    ]);

    // --- The picker is a real dialog over the real editable playlists.
    await panel.getByRole('menuitem', { name: 'Add to playlist…', exact: true }).click();
    const picker = page.locator('#playlist-picker');
    await expect(picker).toBeVisible();
    await expect(picker).toHaveAttribute('aria-modal', 'true');
    await expect(picker.getByRole('heading', { level: 2 })).toHaveText(`Add “${menuTitle}”`);
    const options = picker.getByRole('option');
    await expect(options).toHaveCount(2);
    const optionNames = async () => (await options.allInnerTexts()).map(text => text.split('\n')[0].trim()).sort();
    expect(await optionNames()).toEqual(['Deep focus', 'Night drive']);
    const nightOption = picker.getByRole('option', { name: /Night drive/ });
    const search = picker.locator('#playlist-picker-search');
    await expect.poll(() => page.evaluate(() => document.activeElement?.id)).toBe('playlist-picker-search');
    await expect(picker.locator('#playlist-picker-add')).toBeDisabled();

    // Search narrows the real list; no destination is chosen for the user.
    await search.fill('focus');
    await expect(options).toHaveCount(1);
    await expect(options.first()).toHaveText(/Deep focus/);
    await search.fill('night');
    await expect(options).toHaveCount(1);
    await expect(options.first()).toHaveText(/Night drive/);
    await search.fill('');
    await expect(options).toHaveCount(2);

    // Tab stays inside the modal picker (the browser's modal containment plus the
    // explicit ring), and the search field regains focus after a full cycle.
    const insidePicker = () => page.evaluate(() => !!document.activeElement?.closest?.('#playlist-picker'));
    for (let at = 0; at < 8; at++) {
      await page.keyboard.press('Tab');
      expect(await insidePicker(), 'Tab stays inside the picker').toBe(true);
    }
    await page.keyboard.press('Shift+Tab');
    expect(await insidePicker(), 'Shift+Tab stays inside the picker').toBe(true);
    await page.keyboard.press('Shift+Tab');
    expect(await insidePicker(), 'the ring wraps forwards and backwards').toBe(true);

    // Keyboard navigation selects a real option and names the Add action after
    // that exact destination, so no playlist is chosen behind the user's back.
    await search.focus();
    await page.keyboard.press('ArrowDown');
    await expect(options.first()).toHaveAttribute('aria-selected', 'true');
    await expect(options.nth(1)).toHaveAttribute('aria-selected', 'false');
    await expect(picker.locator('#playlist-picker-add')).toBeEnabled();
    await page.keyboard.press('ArrowDown');
    await expect(options.nth(1)).toHaveAttribute('aria-selected', 'true');
    await expect(options.first()).toHaveAttribute('aria-selected', 'false');
    await nightOption.click();
    await expect(nightOption).toHaveAttribute('aria-selected', 'true');
    await expect(picker.locator('.playlist-choice.selected')).toHaveCount(1);
    await expect(picker.getByRole('option', { name: /Deep focus/ })).toHaveAttribute('aria-selected', 'false');
    const addButton = picker.getByRole('button', { name: 'Add to Night drive', exact: true });
    await expect(addButton).toBeVisible();
    await page.screenshot({ path: `${output}/playlist-picker-1284.png`, animations: 'disabled' });

    // Nothing was written before the explicit Add.
    expect((await playlistItems(destination.id)).length).toBe(0);
    await addButton.click();
    await expect(picker).toBeHidden();
    const receipt = page.locator('.playlist-undo');
    await expect(receipt).toBeVisible();
    await expect(receipt).toContainText(`Added “${menuTitle}” to Night drive.`);
    const committed = await playlistItems(destination.id);
    expect(committed.length).toBe(1);
    expect(committed[0].track.title).toBe(menuTitle);
    // The committed action closes the layer and focus returns to the invoking
    // trigger, not to a stale control from inside the closed picker.
    await expect(trigger).toBeFocused();
    await noPlayback();

    // --- Undo is a real inverse of the acknowledged add.
    await receipt.getByRole('button', { name: 'Undo', exact: true }).click();
    await expect(receipt).toHaveCount(0);
    await expect(page.getByRole('status').first()).toContainText('Removed the added track.');
    await expect.poll(async () => (await playlistItems(destination.id)).length).toBe(0);

    // --- Escape and the backdrop both close the picker and restore focus.
    await trigger.click();
    await panel.getByRole('menuitem', { name: 'Add to playlist…', exact: true }).click();
    await expect(picker).toBeVisible();
    await expect(picker).toHaveAttribute('open', '');
    await page.keyboard.press('Escape');
    await expect(picker).toBeHidden();
    await expect(picker).not.toHaveAttribute('open', '');
    await expect(trigger).toBeFocused();
    await expect(trigger).toHaveAttribute('aria-expanded', 'false');

    await trigger.click();
    await panel.getByRole('menuitem', { name: 'Add to playlist…', exact: true }).click();
    await expect(picker).toBeVisible();
    await page.mouse.click(6, 6);
    await expect(picker).toBeHidden();

    // --- Phone: the same two layers, with the sheet above the player band.
    await page.setViewportSize({ width: 390, height: 844 });
    await settle(page);
    await navigate(page, 'Library');
    await expect(page.locator('.track-row')).toHaveCount(4);
    const phoneRow = page.locator('.track-row').first();
    const phoneTitle = (await phoneRow.locator('.track-title strong').innerText()).trim();
    const phonePlay = phoneRow.locator('.track-play');
    await phonePlay.focus();
    await page.keyboard.press('Shift+F10');
    await expect(panel).toBeVisible();
    await expect(panel).toHaveAttribute('aria-label', `Actions for ${phoneTitle}`);
    await noPlayback();

    const phoneMenu = await page.evaluate(() => {
      const box = (selector) => {
        const element = document.querySelector(selector);
        const rect = element.getBoundingClientRect();
        return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom, width: rect.width, height: rect.height };
      };
      return {
        panel: box('.track-menu-panel'),
        player: box('.player-bar'),
        nav: box('.mobile-nav'),
        overflow: document.documentElement.scrollWidth - innerWidth,
        rows: [...document.querySelectorAll('.track-menu-panel > button')].map(button => button.getBoundingClientRect().height),
      };
    });
    expect(Math.abs(phoneMenu.panel.width - 390)).toBeLessThanOrEqual(1);
    expect(Math.abs(phoneMenu.panel.bottom - phoneMenu.player.top)).toBeLessThanOrEqual(1);
    expect(phoneMenu.panel.bottom).toBeLessThanOrEqual(phoneMenu.nav.top + 1);
    expect(phoneMenu.overflow).toBeLessThanOrEqual(1);
    expect(phoneMenu.rows.every(height => height >= 44)).toBe(true);
    await page.screenshot({ path: `${output}/track-menu-keyboard-sheet-390.png`, animations: 'disabled' });

    await expect.poll(() => page.evaluate(() => document.activeElement?.id === 'track-menu-panel')).toBe(true);
    await page.keyboard.press('Escape');
    await expect(panel).toHaveCount(0);
    await phonePlay.focus();
    await page.keyboard.press('ContextMenu');
    await expect(panel).toBeVisible();
    await panel.getByRole('menuitem', { name: 'Add to playlist…', exact: true }).click();
    await expect(picker).toBeVisible();
    const phonePicker = await page.evaluate(() => {
      const box = (selector) => {
        const element = document.querySelector(selector);
        const rect = element.getBoundingClientRect();
        return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom, width: rect.width, height: rect.height };
      };
      return {
        sheet: box('.playlist-picker-sheet'),
        dialog: box('#playlist-picker'),
        player: box('.player-bar'),
        backdrop: getComputedStyle(document.querySelector('.playlist-picker-backdrop')).backgroundColor,
        options: [...document.querySelectorAll('.playlist-choice')].map(option => option.getBoundingClientRect().height),
        overflow: document.documentElement.scrollWidth - innerWidth,
        opened: document.querySelector('#playlist-picker').open,
      };
    });
    expect(phonePicker.opened, 'the picker is a genuinely open modal dialog').toBe(true);
    expect(Math.abs(phonePicker.sheet.width - 390)).toBeLessThanOrEqual(1);
    expect(Math.abs(phonePicker.sheet.bottom - phonePicker.player.top)).toBeLessThanOrEqual(1);
    expect(phonePicker.sheet.top).toBeGreaterThanOrEqual(0);
    expect(phonePicker.backdrop).not.toBe('rgba(0, 0, 0, 0)');
    expect(phonePicker.options.every(height => height >= 44)).toBe(true);
    expect(phonePicker.overflow).toBeLessThanOrEqual(1);
    await page.screenshot({ path: `${output}/playlist-picker-390.png`, animations: 'disabled' });

    // The phone sheet shares the same registered actions and the same picker.
    await page.keyboard.press('Escape');
    await expect(picker).toBeHidden();
    await expect(phonePlay).toBeFocused();
    await phonePlay.focus();
    await page.keyboard.press('Shift+F10');
    await expect(panel.getByRole('menuitem')).toHaveText([
      'Play / Pause', 'Add to queue', 'Favorite / Unfavorite', 'Add to playlist…', 'Start track radio', 'Edit metadata',
    ]);
    await expect.poll(() => page.evaluate(() => document.activeElement?.id === 'track-menu-panel')).toBe(true);
    await page.keyboard.press('Escape');
    await expect(panel).toHaveCount(0);
    expect(await page.evaluate(() => document.querySelectorAll('audio').length)).toBe(1);
    await noPlayback();

    await writeFile(`${output}/track-menu-keyboard-evidence.json`, JSON.stringify({
      menuTitle, secondTitle, destination: destination.name, phoneMenu, phonePicker, actionRegistry: 'ROW_ACTIONS',
    }, null, 2));
  } finally { await runtime.close(); }
});
