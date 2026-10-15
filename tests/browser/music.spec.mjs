import { test, expect } from '@playwright/test';
import { fileURLToPath } from 'node:url';
const fixture = fileURLToPath(new URL('../../crates/media/tests/fixtures/signal.flac', import.meta.url));
const password = 'local browser fixture password';
let adminCookies;
let secondaryAdminCookies;

async function openAdminFixture(page, cookies = adminCookies) {
  // Reuse only this suite's own isolated sessions, retaining production login limits.
  expect(cookies).toBeDefined();
  await page.context().addCookies(cookies);
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Library', exact: true }).first()).toBeVisible();
}

test.describe.configure({ mode: 'serial' });
test('first-run setup, real FLAC upload, artwork, decode, favorites and safe sharing', async ({ page, browser }) => {
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.goto('/');
  await page.locator('#username').fill('admin');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Create your music space →', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Your music. Your space.' })).toBeVisible();
  await page.getByRole('button', { name: '+ Add music', exact: true }).first().click();
  await page.locator('#audio-files').setInputFiles(fixture);
  await page.getByRole('button', { name: 'Upload to my library', exact: true }).click();
  await expect(page.getByText('1 files processed.', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Close upload dialog' }).click();
  await expect(page.getByRole('button', { name: 'Play Signal', exact: true })).toBeVisible();
  const cover = page.locator('.track-title img');
  await expect(cover).toBeVisible();
  await expect.poll(() => cover.evaluate(image => image.complete && image.naturalWidth > 0)).toBeTruthy();
  await page.getByRole('button', { name: 'Play Signal', exact: true }).click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => Number.isFinite(audio.duration) && audio.duration > 0 && audio.error === null)).toBeTruthy();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
  await page.getByRole('button', { name: 'Add Signal to playback queue', exact: true }).click();
  await page.getByRole('button', { name: 'Add Signal to playback queue', exact: true }).click();
  await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
  await expect(page.locator('.queue-row')).toHaveCount(3);
  await page.locator('.queue-row').nth(1).getByRole('button', { name: 'Move track up', exact: true }).click();
  await expect(page.locator('.queue-row').nth(1)).toHaveClass(/current/);
  await page.getByRole('button', { name: 'Close playback queue', exact: true }).click();
  await page.getByRole('button', { name: 'Favorite track', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Remove favorite', exact: true })).toBeVisible();
  await page.locator('#library-search').fill('Signal');
  await page.locator('.search-form').evaluate(form => form.requestSubmit());
  await expect(page.getByRole('button', { name: 'Play Signal', exact: true })).toBeVisible();
  await page.getByRole('button', { name: /Settings/ }).first().click();
  await page.locator('#new-user').fill('listener');
  await page.locator('#new-password').fill(password);
  await page.getByRole('button', { name: 'Create account', exact: true }).click();
  await expect(page.getByText(/Account listener created/)).toBeVisible();
  await page.getByRole('button', { name: /Playlists/ }).first().click();
  await page.locator('#playlist-name').fill('Evening signals');
  await page.getByRole('button', { name: '+ Create', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Evening signals' })).toBeVisible();
  await page.getByRole('button', { name: /Library/ }).first().click();
  await page.locator('#playlist-target').selectOption({ label: 'Evening signals' });
  await page.getByRole('button', { name: 'Add to selected playlist', exact: true }).click();
  await expect(page.getByText("Track added to your playlist.", { exact: true })).toBeVisible();
  await page.getByRole('button', { name: /Playlists/ }).first().click();
  await page.locator('.playlist-card').filter({ hasText: 'Evening signals' }).click();
  await page.locator('#collaborator').fill('listener');
  await page.getByLabel('Can edit', { exact: true }).check();
  await page.getByRole('button', { name: 'Grant access', exact: true }).click();
  await expect(page.getByText('Playlist access updated.', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: "Revoke listener's playlist access", exact: true })).toBeVisible();
  const listenerContext = await browser.newContext();
  const listenerPage = await listenerContext.newPage();
  await listenerPage.goto('/');
  await listenerPage.locator('#username').fill('listener');
  await listenerPage.locator('#password').fill(password);
  await listenerPage.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await expect(listenerPage.getByRole('button', { name: 'Play Signal', exact: true })).toHaveCount(0);
  await listenerPage.getByRole('button', { name: /Playlists/ }).first().click();
  await listenerPage.locator('.playlist-card').filter({ hasText: 'Evening signals' }).click();
  await expect(listenerPage.getByText('Metadata only', { exact: true })).toBeVisible();
  await expect(listenerPage.getByRole('button', { name: 'Play playlist', exact: false })).toHaveCount(0);
  await expect(listenerPage.getByRole('button', { name: 'Leave playlist', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Create view link', exact: true }).click();
  const sharedUrl = await page.locator('#share-link').inputValue();
  expect(sharedUrl).toContain('/#share=');
  const publicContext = await browser.newContext();
  const publicPage = await publicContext.newPage();
  await publicPage.goto(sharedUrl);
  await expect(publicPage.getByRole('heading', { name: 'Evening signals' })).toBeVisible();
  await expect(publicPage.getByText('Signal', { exact: true })).toBeVisible();
  await expect(publicPage.locator('#vela-audio')).toHaveCount(0);
  await page.getByRole('button', { name: 'Revoke all links', exact: true }).click();
  await expect(page.getByText('All share links for this playlist were revoked.', { exact: true })).toBeVisible();
  await publicPage.reload();
  await expect(publicPage.getByRole('alert')).toContainText('Not found');
  await page.getByRole('button', { name: "Revoke listener's playlist access", exact: true }).click();
  await expect(page.getByText('Collaborator access revoked.', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: "Revoke listener's playlist access", exact: true })).toHaveCount(0);
  await listenerPage.reload();
  await listenerPage.getByRole('button', { name: /Playlists/ }).first().click();
  await expect(listenerPage.locator('.playlist-card').filter({ hasText: 'Evening signals' })).toHaveCount(0);
  await listenerContext.close();
  await publicContext.close();
  await page.screenshot({ path: 'test-results/library-desktop.png', fullPage: true });
  expect(errors).toEqual([]);
});

test('malicious URL import rejected through UI and small-screen layout remains usable', async ({ page }) => {
  await page.goto('/');
  await page.locator('#username').fill('admin');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await page.getByRole('button', { name: /Add music/ }).first().click();
  await page.locator('#import-url').fill('http://169.254.169.254/latest/meta-data/');
  await page.getByRole('button', { name: 'Preview download →', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Import to my library', exact: true })).toHaveCount(0);
  await page.setViewportSize({ width: 390, height: 844 });
  const mobile = page.getByRole('navigation', { name: 'Mobile navigation' });
  await expect(mobile).toBeVisible();
  await mobile.getByRole('button', { name: /Library/ }).click();
  await expect(page.getByRole('button', { name: 'Play Signal', exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBeTruthy();
  await page.screenshot({ path: 'test-results/library-mobile.png', fullPage: true });
});

test('direct public downloadable audio URL imports without console access', async ({ page }) => {
  test.skip(!process.env.VELA_LIVE_IMPORT_TEST, 'Opt-in network integration; deterministic transport tests run in Rust.');
  test.setTimeout(120000);
  await page.goto('/');
  await page.locator('#username').fill('admin');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await page.getByRole('button', { name: /Add music/ }).first().click();
  await page.locator('#import-url').fill('https://upload.wikimedia.org/wikipedia/commons/c/c8/Example.ogg');
  await page.getByRole('button', { name: 'Preview download →', exact: true }).click();
  await page.getByRole('button', { name: 'Import to my library', exact: true }).click();
  await expect(page.getByText('Your import is ready in the library.', { exact: true })).toBeVisible({ timeout: 90000 });
  await page.getByRole('button', { name: /Library/ }).first().click();
  await expect(page.locator('.track-row')).toHaveCount(2);
  await page.locator('.track-play:not([aria-label="Play Signal"])').click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.duration > 5 && audio.error === null)).toBeTruthy();
});


test('late older library responses cannot replace the current search', async ({ page }) => {
  await page.goto('/');
  await page.locator('#username').fill('admin');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Play Signal', exact: true })).toBeVisible();
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  let seen;
  const held = new Promise(resolve => { seen = resolve; });
  await page.route('**/api/tracks?*', async route => {
    if (new URL(route.request().url()).searchParams.get('q') === 'NoTrackWithThisTitle') {
      const response = await route.fetch();
      seen();
      await gate;
      await route.fulfill({ response });
    } else { await route.continue(); }
  });
  await page.locator('#library-search').fill('NoTrackWithThisTitle');
  await page.locator('#library-search').press('Enter');
  await held;
  const newer = page.waitForResponse(r => r.url().includes('/api/tracks?') && new URL(r.url()).searchParams.get('q') === 'Signal');
  await page.locator('#library-search').fill('Signal');
  await page.locator('#library-search').press('Enter');
  await newer;
  const older = page.waitForResponse(r => r.url().includes('/api/tracks?') && new URL(r.url()).searchParams.get('q') === 'NoTrackWithThisTitle');
  release();
  await older;
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await expect(page.getByRole('button', { name: 'Play Signal', exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'No matches yet', exact: true })).toHaveCount(0);
});


test('pending private responses are discarded when another account signs in', async ({ page }) => {
  await page.goto('/');
  await page.locator('#username').fill('admin');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await page.getByRole('button', { name: /Playlists/ }).first().click();
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  let seen;
  const held = new Promise(resolve => { seen = resolve; });
  await page.route('**/api/playlists/*', async route => {
    const url = new URL(route.request().url());
    if (/^\/api\/playlists\/[^/]+$/.test(url.pathname) && route.request().method() === 'GET') {
      const response = await route.fetch();
      seen();
      await gate;
      await route.fulfill({ response });
    } else { await route.continue(); }
  });
  await page.locator('.playlist-card').filter({ hasText: 'Evening signals' }).click();
  await held;
  await page.getByRole('button', { name: /Settings/ }).first().click();
  await page.getByRole('button', { name: 'Sign out', exact: true }).click();
  await page.locator('#username').fill('listener');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  const older = page.waitForResponse(r => /^\/api\/playlists\/[^/]+$/.test(new URL(r.url()).pathname));
  release();
  await older;
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.getByRole('button', { name: /Playlists/ }).first().click();
  await expect(page.locator('.playlist-detail')).toHaveCount(0);
  await expect(page.getByRole('heading', { name: 'Evening signals', exact: true })).toHaveCount(0);
  await expect(page.locator('.queue-row')).toHaveCount(0);
});

test('album and artist pages use real metadata and play owned audio', async ({ page }) => {
  await page.goto('/');
  await page.locator('#username').fill('admin');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await page.getByRole('button', { name: 'Albums', exact: true }).first().click();
  await page.getByRole('button', { name: 'Open album Real Fixtures by Vela Test', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Real Fixtures', exact: true })).toBeVisible();
  await expect(page.getByText('2026', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Play Signal', exact: true }).click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
  await expect(page.locator('#vela-audio')).toHaveJSProperty('error', null);
  await page.getByRole('button', { name: 'All albums', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Open album Real Fixtures by Vela Test', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Artists', exact: true }).first().click();
  await page.getByRole('button', { name: 'Open artist Vela Test', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Vela Test', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Play Signal', exact: true })).toBeVisible();
  await page.screenshot({ path: 'test-results/artist-desktop.png', fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByRole('button', { name: 'All artists', exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBeTruthy();
  await page.screenshot({ path: 'test-results/artist-mobile.png', fullPage: true });
});

test('pending album responses cannot replace a newer artist view', async ({ page }) => {
  await page.goto('/');
  await page.locator('#username').fill('admin');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await page.getByRole('button', { name: 'Albums', exact: true }).first().click();
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  let seen;
  const held = new Promise(resolve => { seen = resolve; });
  await page.route('**/api/albums/*/tracks?*', async route => {
    const response = await route.fetch();
    seen();
    await gate;
    await route.fulfill({ response });
  });
  await page.getByRole('button', { name: 'Open album Real Fixtures by Vela Test', exact: true }).click();
  await held;
  await page.getByRole('button', { name: 'Artists', exact: true }).first().click();
  await page.getByRole('button', { name: 'Open artist Vela Test', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Play Signal', exact: true })).toBeVisible();
  const older = page.waitForResponse(r => /\/api\/albums\/[^/]+\/tracks/.test(new URL(r.url()).pathname));
  release();
  await older;
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await expect(page.getByRole('heading', { name: 'Vela Test', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'All albums', exact: true })).toHaveCount(0);
});

test('catalog metadata editor saves without changing audio and handles a stale draft', async ({ page, browser }) => {
  await page.goto('/');
  await page.locator('#username').fill('admin');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await page.getByRole('button', { name: 'Edit metadata for Signal', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Edit metadata', exact: true });
  await expect(dialog.getByLabel('Title', { exact: true })).toHaveValue('Signal');
  await expect(dialog.getByLabel('Year', { exact: true })).toHaveValue('2026');
  const otherContext = await browser.newContext();
  const other = await otherContext.newPage();
  await other.goto('/');
  await other.locator('#username').fill('admin');
  await other.locator('#password').fill(password);
  await other.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await other.getByRole('button', { name: 'Edit metadata for Signal', exact: true }).click();
  const otherDialog = other.getByRole('dialog', { name: 'Edit metadata', exact: true });
  await expect(otherDialog.getByLabel('Title', { exact: true })).toHaveValue('Signal');
  secondaryAdminCookies = await otherContext.cookies();
  await otherDialog.getByLabel('Title', { exact: true }).fill('Signal from another tab');
  await otherDialog.getByRole('button', { name: 'Save metadata', exact: true }).click();
  await expect(otherDialog).toHaveCount(0);
  await dialog.getByLabel('Title', { exact: true }).fill('My draft signal');
  await dialog.getByRole('button', { name: 'Save metadata', exact: true }).click();
  await expect(dialog.getByRole('alert')).toContainText('Metadata changed');
  await expect(dialog.getByLabel('Title', { exact: true })).toHaveValue('My draft signal');
  await dialog.getByRole('button', { name: 'Reload current metadata', exact: true }).click();
  await expect(dialog.getByLabel('Title', { exact: true })).toHaveValue('Signal from another tab');
  await dialog.getByLabel('Title', { exact: true }).fill('Signal edited in Vela');
  await dialog.getByLabel('Album', { exact: true }).fill('Browser edits');
  await dialog.getByLabel('Year', { exact: true }).fill('2025');
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBeTruthy();
  await page.screenshot({ path: 'test-results/metadata-mobile.png', fullPage: true });
  await dialog.getByRole('button', { name: 'Save metadata', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByText('Metadata saved. Your original audio file is unchanged.', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Albums', exact: true }).last().click();
  await page.getByRole('button', { name: 'Open album Browser edits by Vela Test', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Browser edits', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Play Signal edited in Vela', exact: true }).click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
  await page.getByRole('button', { name: 'Edit metadata for Signal edited in Vela', exact: true }).click();
  await expect(dialog.getByLabel('Title', { exact: true })).toHaveValue('Signal edited in Vela');
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await page.reload();
  await expect(page.getByRole('button', { name: 'Play Signal edited in Vela', exact: true })).toBeVisible();
  await otherContext.close();
});

test('cancelled metadata responses stay closed across account changes', async ({ page }) => {
  await page.goto('/');
  await page.locator('#username').fill('admin');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  let seen;
  const held = new Promise(resolve => { seen = resolve; });
  await page.route('**/api/tracks/*/metadata', async route => {
    const response = await route.fetch();
    seen();
    await gate;
    await route.fulfill({ response });
  });
  await page.getByRole('button', { name: 'Edit metadata for Signal edited in Vela', exact: true }).click();
  await held;
  await page.getByRole('dialog', { name: 'Edit metadata', exact: true }).getByRole('button', { name: 'Cancel', exact: true }).click();
  await page.getByRole('button', { name: /Settings/ }).first().click();
  await page.getByRole('button', { name: 'Sign out', exact: true }).click();
  await page.locator('#username').fill('listener');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  const older = page.waitForResponse(r => /\/api\/tracks\/[^/]+\/metadata/.test(new URL(r.url()).pathname));
  release();
  await older;
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await expect(page.getByRole('dialog', { name: 'Edit metadata', exact: true })).toHaveCount(0);
  await expect(page.getByText('Signal edited in Vela', { exact: true })).toHaveCount(0);
  await expect(page.locator('.queue-row')).toHaveCount(0);
});

test('administrator scans a configured folder and rescan preserves catalog edits', async ({ page }) => {
  await page.goto('/');
  await page.locator('#username').fill('admin');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await page.getByRole('button', { name: /Settings/ }).first().click();
  await page.getByLabel('Folder', { exact: true }).selectOption('fixture');
  await page.getByLabel('Source name', { exact: true }).fill('Mounted collection');
  await page.getByRole('button', { name: 'Add source', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Scan Mounted collection', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Scan Mounted collection', exact: true }).click();
  await expect(page.locator('.scanner-job-status').filter({ hasText: 'Scan complete' })).toHaveCount(1);
  await page.getByRole('button', { name: 'Library', exact: true }).first().click();
  await expect(page.getByRole('button', { name: 'Play Beacon', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Play Signal edited in Vela', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Play Beacon', exact: true }).click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
  await page.getByRole('button', { name: /Settings/ }).first().click();
  await page.getByRole('button', { name: 'Scan Mounted collection', exact: true }).click();
  await expect(page.locator('.scanner-job-status').filter({ hasText: 'Scan complete' })).toHaveCount(2);
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBeTruthy();
  await page.screenshot({ path: 'test-results/scanner-mobile.png', fullPage: true });
  await page.getByRole('button', { name: 'Remove Mounted collection', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Scan Mounted collection', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Library', exact: true }).last().click();
  await expect(page.getByRole('button', { name: 'Play Beacon', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Play Signal edited in Vela', exact: true })).toBeVisible();
  adminCookies = await page.context().cookies();
});

test('library filters combine real tags, technical properties, favorites and sorting', async ({ page }) => {
  await openAdminFixture(page);
  await page.getByRole('button', { name: 'Edit metadata for Signal edited in Vela', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Edit metadata', exact: true });
  await dialog.getByLabel('Genre', { exact: true }).fill('Browser filters');
  await dialog.getByRole('button', { name: 'Save metadata', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await page.getByRole('button', { name: 'Filter and sort', exact: true }).click();
  await expect(page.locator('#library-genre-options option[value="Browser filters"]')).toHaveCount(1);
  await page.getByLabel('Genre', { exact: true }).fill('browser filters');
  await page.getByLabel('Format', { exact: true }).selectOption('FLAC');
  await page.getByLabel('Year from', { exact: true }).fill('2025');
  await page.getByLabel('Year to', { exact: true }).fill('2025');
  await page.getByLabel('Minimum bitrate (kbps)', { exact: true }).fill('1');
  await page.getByLabel('Minimum sample rate (Hz)', { exact: true }).fill('44100');
  await page.getByLabel('Minimum bit depth', { exact: true }).fill('16');
  await page.getByLabel('Sort', { exact: true }).selectOption('title');
  const filtered = page.waitForResponse(r => r.url().includes('/api/tracks?') && new URL(r.url()).searchParams.get('genre') === 'browser filters');
  await page.getByRole('button', { name: 'Apply filters', exact: true }).click();
  expect((await filtered).ok()).toBeTruthy();
  await expect(page.getByRole('button', { name: 'Play Signal edited in Vela', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Play Beacon', exact: true })).toHaveCount(0);
  await expect(page.locator('.filter-summary')).toContainText('Title A–Z');
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBeTruthy();
  await page.screenshot({ path: 'test-results/filters-mobile.png', fullPage: true });
  await page.getByRole('button', { name: 'Favorites', exact: true }).last().click();
  await expect(page.getByRole('button', { name: 'Play Signal edited in Vela', exact: true })).toBeVisible();
  await expect(page.locator('.filter-summary')).toContainText('browser filters');
  await page.getByRole('button', { name: 'Library', exact: true }).last().click();
  await page.getByLabel('Year from', { exact: true }).fill('2030');
  await page.getByRole('button', { name: 'Apply filters', exact: true }).click();
  await expect(page.locator('.library-filters').getByRole('alert')).toContainText('Year from');
  await expect(page.getByLabel('Year from', { exact: true })).toHaveValue('2030');
  await expect(page.locator('.filter-summary')).toContainText('2025–2025');
  await expect(page.getByRole('button', { name: 'Play Signal edited in Vela', exact: true })).toBeVisible();
  await page.getByLabel('Year from', { exact: true }).fill('2025');
  await page.getByLabel('Genre', { exact: true }).fill('Missing browser genre');
  await page.getByRole('button', { name: 'Apply filters', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'No matching tracks', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Reset applied filters', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Play Beacon', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Play Signal edited in Vela', exact: true })).toBeVisible();
  await expect(page.getByLabel('Year from', { exact: true })).toHaveValue('');
  await page.getByLabel('Sort', { exact: true }).selectOption('newest');
  await page.getByRole('button', { name: 'Apply filters', exact: true }).click();
  await expect(page.locator('.filter-summary')).toHaveText('Recently added');
  await expect(page.locator('.track-row .track-title strong').first()).toHaveText('Beacon');
});

test('newer filter requests win and private suggestions clear across account changes', async ({ page }) => {
  await openAdminFixture(page, secondaryAdminCookies);
  await page.getByRole('button', { name: 'Filter and sort', exact: true }).click();
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  let seen;
  const held = new Promise(resolve => { seen = resolve; });
  await page.route('**/api/tracks?*', async route => {
    if (new URL(route.request().url()).searchParams.get('genre') === 'Older missing genre') {
      const response = await route.fetch();
      seen();
      await gate;
      await route.fulfill({ response });
    } else { await route.continue(); }
  });
  await page.getByLabel('Genre', { exact: true }).fill('Older missing genre');
  await page.getByRole('button', { name: 'Apply filters', exact: true }).click();
  await held;
  await page.getByLabel('Genre', { exact: true }).fill('Browser filters');
  await page.getByRole('button', { name: 'Apply filters', exact: true }).click();
  await expect(page.locator('.filter-summary')).toContainText('Browser filters');
  const older = page.waitForResponse(r => r.url().includes('/api/tracks?') && new URL(r.url()).searchParams.get('genre') === 'Older missing genre');
  release();
  await older;
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await expect(page.getByRole('button', { name: 'Play Signal edited in Vela', exact: true })).toBeVisible();
  await expect(page.locator('.filter-summary')).toContainText('Browser filters');
  let releaseFacets;
  const facetGate = new Promise(resolve => { releaseFacets = resolve; });
  let seenFacets;
  const facetsHeld = new Promise(resolve => { seenFacets = resolve; });
  let intercepted = false;
  await page.route('**/api/catalog/filters', async route => {
    if (!intercepted) {
      intercepted = true;
      const response = await route.fetch();
      seenFacets();
      await facetGate;
      await route.fulfill({ response });
    } else { await route.continue(); }
  });
  await page.getByRole('button', { name: 'Refresh suggestions', exact: true }).click();
  await facetsHeld;
  await page.getByRole('button', { name: /Settings/ }).first().click();
  await page.getByRole('button', { name: 'Sign out', exact: true }).click();
  await page.locator('#username').fill('listener');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await page.getByRole('button', { name: 'Library', exact: true }).first().click();
  await expect(page.locator('#library-search')).toHaveValue('');
  await page.getByRole('button', { name: 'Filter and sort', exact: true }).click();
  await expect(page.getByLabel('Genre', { exact: true })).toHaveValue('');
  const oldFacets = page.waitForResponse(r => r.url().endsWith('/api/catalog/filters'));
  releaseFacets();
  await oldFacets;
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await expect(page.locator('#library-genre-options option')).toHaveCount(0);
  await expect(page.getByLabel('Format', { exact: true })).toHaveValue('');
  await expect(page.locator('.filter-summary')).toHaveText('Library order');
});

test('real library pagination retains applied filters and search while drafts change', async ({ page }) => {
  test.setTimeout(120000);
  await openAdminFixture(page);
  await expect(page.locator('#library-search')).toBeVisible();
  const auth = await (await page.request.get('/api/me')).json();
  // Real, distinct PCM waveforms seed a private catalog through the production
  // upload endpoint. No catalog response is fabricated or SQL-inserted here.
  for (let at = 0; at < 52; at++) {
    const samples = 4410;
    const wave = Buffer.alloc(44 + samples * 2);
    wave.write('RIFF', 0); wave.writeUInt32LE(36 + samples * 2, 4);
    wave.write('WAVEfmt ', 8); wave.writeUInt32LE(16, 16);
    wave.writeUInt16LE(1, 20); wave.writeUInt16LE(1, 22);
    wave.writeUInt32LE(44100, 24); wave.writeUInt32LE(88200, 28);
    wave.writeUInt16LE(2, 32); wave.writeUInt16LE(16, 34);
    wave.write('data', 36); wave.writeUInt32LE(samples * 2, 40);
    for (let sample = 0; sample < samples; sample++) {
      wave.writeInt16LE(Math.round(2000 * Math.sin(2 * Math.PI * (220 + at) * sample / 44100)), 44 + sample * 2);
    }
    const response = await page.request.post('/api/upload', {
      headers: { 'X-CSRF-Token': auth.csrf_token },
      multipart: { file: { name: `Window ${String(at).padStart(3, '0')}.wav`, mimeType: 'audio/wav', buffer: wave } },
    });
    expect(response.ok()).toBeTruthy();
    expect((await response.json()).duplicate).toBeFalsy();
  }
  await page.getByRole('button', { name: 'Filter and sort', exact: true }).click();
  await page.getByLabel('Format', { exact: true }).selectOption('WAV audio');
  await page.getByLabel('Sort', { exact: true }).selectOption('title');
  await page.getByRole('button', { name: 'Apply filters', exact: true }).click();
  await expect(page.locator('.filter-summary')).toContainText('WAV audio');
  await page.locator('#library-search').fill('Window');
  await page.locator('#library-search').press('Enter');
  await expect(page.locator('.applied-search')).toContainText('Window');
  await expect(page.locator('.track-row')).toHaveCount(50);
  await expect(page.locator('.track-row .track-title strong').first()).toHaveText('Window 000');
  // These are drafts only; paging must continue the committed Window/WAV query.
  await page.getByLabel('Genre', { exact: true }).fill('Unsaved genre draft');
  await page.locator('#library-search').fill('Unsaved search draft');
  const next = page.waitForResponse(r => r.url().includes('/api/tracks?') && new URL(r.url()).searchParams.has('cursor'));
  await page.getByRole('button', { name: 'Next page →', exact: true }).click();
  const params = new URL((await next).url()).searchParams;
  expect(params.get('q')).toBe('Window');
  expect(params.get('codec')).toBe('WAV audio');
  expect(params.get('sort')).toBe('title');
  expect(params.has('genre')).toBeFalsy();
  await expect(page.locator('.track-row')).toHaveCount(2);
  await expect(page.locator('.track-row .track-title strong').first()).toHaveText('Window 050');
  await expect(page.locator('.track-row .track-title strong').last()).toHaveText('Window 051');
  await page.getByRole('button', { name: 'Back to first page', exact: true }).click();
  await expect(page.locator('.track-row')).toHaveCount(50);
  await expect(page.locator('#library-search')).toHaveValue('Unsaved search draft');
  await expect(page.locator('.applied-search')).toContainText('Window');
});
