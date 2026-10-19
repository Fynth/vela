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
