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
  await page.getByRole('button', { name: 'Library', exact: true }).first().click();
}


async function navigate(page, name) {
  const mobile = page.getByRole('navigation', { name: 'Mobile navigation', exact: true });
  if (await mobile.isVisible()) {
    const tab = mobile.getByRole('button', { name, exact: true });
    if (await tab.count()) { await tab.click(); }
    else {
      await mobile.getByRole('button', { name: 'More destinations', exact: true }).click();
      await page.getByRole('dialog', { name: 'More from your library', exact: true }).getByRole('button', { name, exact: true }).click();
    }
  } else { await page.getByRole('button', { name, exact: true }).first().click(); }
}

test.describe.configure({ mode: 'serial' });
test('first-run setup, real FLAC upload, artwork, decode, favorites and safe sharing', async ({ page, browser }) => {
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.goto('/');
  await page.locator('#username').fill('admin');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Create your music space →', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Make room for your first listen', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Library', exact: true }).first().click();
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
  await listenerPage.getByRole('button', { name: 'Library', exact: true }).first().click();
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
  adminCookies = await page.context().cookies();
});

test('malicious URL import rejected through UI and small-screen layout remains usable', async ({ page }) => {
  await openAdminFixture(page);
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
  await openAdminFixture(page);
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
  await openAdminFixture(page);
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
  await page.getByRole('button', { name: 'Library', exact: true }).first().click();
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
  await page.getByRole('button', { name: 'Library', exact: true }).first().click();
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
  await openAdminFixture(page);
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
  await openAdminFixture(page);
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
  await page.getByRole('button', { name: 'Library', exact: true }).first().click();
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
  await other.getByRole('button', { name: 'Library', exact: true }).first().click();
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
  await navigate(page, 'Albums');
  await page.getByRole('button', { name: 'Open album Browser edits by Vela Test', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Browser edits', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Play Signal edited in Vela', exact: true }).click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
  await page.getByRole('button', { name: 'Edit metadata for Signal edited in Vela', exact: true }).click();
  await expect(dialog.getByLabel('Title', { exact: true })).toHaveValue('Signal edited in Vela');
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await page.reload();
  await navigate(page, 'Library');
  await expect(page.getByRole('button', { name: 'Play Signal edited in Vela', exact: true })).toBeVisible();
  await otherContext.close();
});

test('cancelled metadata responses stay closed across account changes', async ({ page }) => {
  await page.goto('/');
  await page.locator('#username').fill('admin');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await page.getByRole('button', { name: 'Library', exact: true }).first().click();
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
  await page.getByRole('button', { name: 'Library', exact: true }).first().click();
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
  await page.getByRole('button', { name: 'Library', exact: true }).first().click();
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
  await navigate(page, 'Favorites');
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

test('job center controls genuine scans and pages their preserved history on mobile', async ({ page }) => {
  test.setTimeout(120000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await openAdminFixture(page);
  await page.getByRole('button', { name: 'Jobs', exact: true }).first().click();
  await expect(page.getByRole('heading', { name: 'Background tasks', exact: true })).toBeVisible();
  if (process.env.VELA_LIVE_IMPORT_TEST) {
    await expect(page.locator('.background-job[data-job-kind="import"] .background-job-status')).toHaveText('Completed');
    await expect(page.locator('.background-job[data-job-kind="import"]').getByRole('button', { name: 'Open library', exact: true })).toBeVisible();
    await expect(page.locator('.background-job[data-job-kind="import"]')).not.toContainText('wikimedia.org');
  }
  await page.getByRole('button', { name: 'Music folder settings', exact: true }).click();
  await page.getByLabel('Folder', { exact: true }).selectOption('fixture');
  await page.getByLabel('Source name', { exact: true }).fill('Job center archive');
  await page.getByRole('button', { name: 'Add source', exact: true }).click();
  await page.getByRole('button', { name: 'Scan Job center archive', exact: true }).click();
  await page.getByRole('button', { name: 'Jobs', exact: true }).first().click();
  await page.getByLabel('Task type', { exact: true }).selectOption('scan');
  const rows = page.locator('.background-job');
  await expect(rows).toHaveCount(1);
  // A tiny real scan may finish before the first GET. Manual refresh also
  // covers that idle state without assuming a minimum worker duration.
  await expect.poll(async () => {
    if (await page.getByRole('button', { name: 'Refresh tasks', exact: true }).isEnabled()) {
      await page.getByRole('button', { name: 'Refresh tasks', exact: true }).click();
    }
    return rows.first().locator('.background-job-status').textContent();
  }).toBe('Completed');
  const original = await rows.first().getAttribute('data-job-id');
  await expect(rows.first().locator('.background-job-counters')).toContainText('Unchanged');
  const rescan = page.waitForResponse(response => /\/api\/scan-sources\/[^/]+\/scan$/.test(new URL(response.url()).pathname) && response.request().method() === 'POST');
  await rows.first().getByRole('button', { name: 'Scan again', exact: true }).click();
  const newJob = await (await rescan).json();
  expect(newJob.id).not.toBe(original);
  await expect(rows).toHaveCount(2);
  const auth = await (await page.request.get('/api/me')).json();
  const source = (await (await page.request.get('/api/scan-sources')).json()).find(item => item.name === 'Job center archive');
  async function waitForScan(id) {
    await expect.poll(async () => {
      const jobs = await (await page.request.get('/api/scans')).json();
      return jobs.find(job => job.id === id)?.status;
    }, { intervals: [20, 50, 100] }).toBe('completed');
  }
  await waitForScan(newJob.id);
  // Fifty-two actual passes over the isolated FLAC/MP3 folder produce real
  // durable history. No job response or database row is fabricated.
  for (let pass = 2; pass < 52; pass++) {
    const response = await page.request.post(`/api/scan-sources/${source.id}/scan`, { headers: { 'X-CSRF-Token': auth.csrf_token } });
    expect(response.ok()).toBeTruthy();
    await waitForScan((await response.json()).id);
  }
  await page.getByRole('button', { name: 'Refresh tasks', exact: true }).click();
  await expect(rows).toHaveCount(50);
  const firstIds = await rows.evaluateAll(items => items.map(item => item.dataset.jobId));
  await page.getByRole('button', { name: 'Older tasks', exact: true }).click();
  await expect(rows).toHaveCount(2);
  const olderIds = await rows.evaluateAll(items => items.map(item => item.dataset.jobId));
  expect(new Set([...firstIds, ...olderIds]).size).toBe(52);
  await page.getByRole('button', { name: 'Refresh tasks', exact: true }).click();
  await expect(rows).toHaveCount(2);
  expect(await rows.evaluateAll(items => items.map(item => item.dataset.jobId))).toEqual(olderIds);
  await page.getByRole('button', { name: 'Latest tasks', exact: true }).click();
  await expect(rows).toHaveCount(50);
  await page.setViewportSize({ width: 320, height: 780 });
  const mobile = page.getByRole('navigation', { name: 'Mobile navigation' });
  await expect(mobile.getByRole('button')).toHaveCount(5);
  await mobile.getByRole('button', { name: 'More destinations', exact: true }).click();
  const destinations = page.getByRole('dialog', { name: 'More from your library', exact: true });
  for (const name of ['Jobs', 'Settings', 'Add music']) {
    await expect(destinations.getByRole('button', { name, exact: true })).toBeInViewport();
  }
  await page.keyboard.press('Escape');
  await expect(destinations).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBeTruthy();
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: 'test-results/jobs-mobile.png' });
  await page.locator('.job-center-links').getByRole('button', { name: 'Add music', exact: true }).click();
  await expect(page.locator('#import-url')).toBeVisible();
  await navigate(page, 'Jobs');
  await page.locator('.job-center-links').getByRole('button', { name: 'Open library', exact: true }).click();
  await expect(page.locator('#library-search')).toBeVisible();
  expect(errors).toEqual([]);
});

test('job filters ignore late replies and failed refresh retains a confirmed page', async ({ page }) => {
  await openAdminFixture(page);
  await page.getByRole('button', { name: 'Jobs', exact: true }).first().click();
  await page.getByLabel('Task type', { exact: true }).selectOption('scan');
  const rows = page.locator('.background-job');
  await expect(rows).toHaveCount(50);
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  let seen;
  const held = new Promise(resolve => { seen = resolve; });
  let heldOnce = false;
  await page.route('**/api/jobs?*', async route => {
    if (!heldOnce && new URL(route.request().url()).searchParams.get('state') === 'failed') {
      heldOnce = true;
      const response = await route.fetch();
      expect((await response.json()).items).toHaveLength(0);
      seen();
      await gate;
      await route.fulfill({ response });
    } else { await route.continue(); }
  });
  await page.getByLabel('Task state', { exact: true }).selectOption('failed');
  await held;
  await page.getByLabel('Task state', { exact: true }).selectOption('completed');
  await expect(page.locator('.job-center-update')).toHaveCount(0);
  await expect(rows).toHaveCount(50);
  const older = page.waitForResponse(response => response.url().includes('/api/jobs?') && new URL(response.url()).searchParams.get('state') === 'failed');
  release();
  await older;
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await expect(rows).toHaveCount(50);
  await expect(page.getByLabel('Task state', { exact: true })).toHaveValue('completed');
  await page.getByRole('button', { name: 'Older tasks', exact: true }).click();
  await expect(rows).toHaveCount(2);
  await page.route('**/api/jobs?*', route => route.fulfill({ status: 503, contentType: 'application/json', body: JSON.stringify({ error: 'Temporary fixture outage' }) }), { times: 1 });
  await page.getByRole('button', { name: 'Refresh tasks', exact: true }).click();
  await expect(page.locator('.job-center').getByRole('alert')).toContainText('Temporary fixture outage');
  await expect(rows).toHaveCount(2);
  await expect(page.locator('.job-center-stale')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Older tasks', exact: true })).toBeDisabled();
  await page.getByRole('button', { name: 'Refresh tasks', exact: true }).click();
  await expect(page.locator('.job-center').getByRole('alert')).toHaveCount(0);
  await expect(page.locator('.job-center-stale')).toHaveCount(0);
  await expect(rows).toHaveCount(2);
  // Idle Jobs does not keep fetching the legacy imports list in parallel.
  let legacyRequests = 0;
  const observe = request => { if (new URL(request.url()).pathname === '/api/imports') legacyRequests++; };
  page.on('request', observe);
  await page.waitForTimeout(2300);
  page.off('request', observe);
  expect(legacyRequests).toBe(0);
});

test('Home and global search use real music while navigation preserves the player and next queue', async ({ page }) => {
  test.setTimeout(120000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await openAdminFixture(page);
  const auth = await (await page.request.get('/api/me')).json();
  const samples = 44100 * 45;
  const wave = Buffer.alloc(44 + samples * 2);
  wave.write('RIFF', 0); wave.writeUInt32LE(36 + samples * 2, 4);
  wave.write('WAVEfmt ', 8); wave.writeUInt32LE(16, 16);
  wave.writeUInt16LE(1, 20); wave.writeUInt16LE(1, 22);
  wave.writeUInt32LE(44100, 24); wave.writeUInt32LE(88200, 28);
  wave.writeUInt16LE(2, 32); wave.writeUInt16LE(16, 34);
  wave.write('data', 36); wave.writeUInt32LE(samples * 2, 40);
  for (let sample = 0; sample < samples; sample++) wave.writeInt16LE(Math.round(1500 * Math.sin(2 * Math.PI * 330 * sample / 44100)), 44 + sample * 2);
  const upload = await page.request.post('/api/upload', { headers: { 'X-CSRF-Token': auth.csrf_token }, multipart: { file: { name: 'Unbroken session.wav', mimeType: 'audio/wav', buffer: wave } } });
  expect(upload.ok()).toBeTruthy();
  const track = (await upload.json()).track;
  await navigate(page, 'Playlists');
  await page.locator('#playlist-name').fill('Vela listening');
  await page.getByRole('button', { name: '+ Create', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Vela listening', exact: true })).toBeVisible();
  await navigate(page, 'Home');
  await expect(page.getByRole('heading', { name: 'Hello, admin.', exact: true })).toBeVisible();
  const additions = page.locator('[aria-labelledby="home-additions"]');
  await additions.getByRole('button', { name: `Play ${track.title}`, exact: true }).click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
  await page.getByRole('slider', { name: 'Playback position', exact: true }).evaluate(input => { input.value = '5'; input.dispatchEvent(new Event('input', { bubbles: true })); });
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThanOrEqual(5);
  const source = await page.locator('#vela-audio').getAttribute('src');
  await page.evaluate(() => { window.__experienceAudio = document.getElementById('vela-audio'); });
  const checkPlayer = async () => {
    expect(await page.evaluate(() => document.getElementById('vela-audio') === window.__experienceAudio)).toBeTruthy();
    await expect(page.locator('#vela-audio')).toHaveAttribute('src', source);
    await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', false);
  };
  await navigate(page, 'Search');
  await page.locator('#global-search').fill('Vela');
  await page.locator('#global-search').press('Enter');
  for (const name of ['Track results', 'Album results', 'Artist results', 'Playlist results']) await expect(page.getByRole('region', { name, exact: true })).toBeVisible();
  await checkPlayer();
  await page.getByRole('region', { name: 'Album results', exact: true }).getByRole('button').first().click();
  await expect(page.locator('.catalog-detail')).toBeVisible();
  await checkPlayer();
  await navigate(page, 'Artists');
  await page.getByRole('button', { name: 'Open artist Vela Test', exact: true }).click();
  await expect(page.locator('.catalog-detail')).toBeVisible();
  await checkPlayer();
  await navigate(page, 'Playlists');
  await checkPlayer();
  await page.goBack();
  await expect(page).toHaveURL(/#artists$/);
  await checkPlayer();
  await page.keyboard.press('Control+k');
  await expect(page.locator('#global-search')).toBeFocused();
  await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
  await page.getByRole('button', { name: 'Clear upcoming tracks', exact: true }).click();
  await page.getByRole('button', { name: 'Close playback queue', exact: true }).click();
  const search = async text => {
    await page.locator('#global-search').fill(text);
    await page.locator('#global-search').press('Enter');
    await expect(page.getByRole('region', { name: 'Track results', exact: true })).toBeVisible();
  };
  await search('Beacon');
  await page.getByRole('button', { name: 'Play Beacon next', exact: true }).click();
  await search('Signal');
  await page.getByRole('button', { name: 'Add Signal edited in Vela to playback queue', exact: true }).click();
  await search('Beacon');
  await page.getByRole('button', { name: 'Play Beacon next', exact: true }).click();
  await checkPlayer();
  await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
  await expect(page.locator('.queue-row .queue-track strong')).toHaveText([track.title, 'Beacon', 'Beacon', 'Signal edited in Vela']);
  await expect(page.locator('.queue-row').first()).toHaveClass(/current/);
  await page.getByRole('button', { name: 'Close playback queue', exact: true }).click();
  await navigate(page, 'Home');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const width of [320, 768, 1440, 1920]) {
    await page.setViewportSize({ width, height: 900 });
    for (const theme of ['dark', 'light']) {
      const current = await page.locator('html').getAttribute('data-theme');
      if (current !== theme) await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
      await checkPlayer();
      await page.screenshot({ path: `test-results/experience-home-${width}-${theme}.png` });
    }
  }
  await page.setViewportSize({ width: 320, height: 780 });
  const more = page.getByRole('button', { name: 'More destinations', exact: true });
  await more.focus();
  await page.keyboard.press('Enter');
  const sheet = page.getByRole('dialog', { name: 'More from your library', exact: true });
  await expect(sheet).toBeVisible();
  await page.keyboard.press('Shift+Tab');
  expect(await sheet.evaluate(node => node.contains(document.activeElement))).toBeTruthy();
  await page.keyboard.press('Escape');
  await expect(sheet).toHaveCount(0);
  await expect(more).toBeFocused();
  await checkPlayer();
  await page.reload();
  await navigate(page, 'Library');
  await page.getByRole('button', { name: 'Show playback queue and controls', exact: true }).click();
  await expect(page.locator('.queue-row .queue-track strong')).toHaveText([track.title, 'Beacon', 'Beacon', 'Signal edited in Vela']);
  await expect(page.locator('.queue-row').first()).toHaveClass(/current/);
  expect(errors).toEqual([]);
});

test('global search debounces and discards obsolete queries including invalid drafts', async ({ page }) => {
  await openAdminFixture(page);
  await navigate(page, 'Search');
  let release, seen, delivered;
  const delivery = new Promise(resolve => { delivered = resolve; });
  const gate = new Promise(resolve => { release = resolve; });
  const held = new Promise(resolve => { seen = resolve; });
  let once = false;
  await page.route('**/api/search?*', async route => {
    if (!once && new URL(route.request().url()).searchParams.get('q') === 'Older missing result') {
      once = true;
      const response = await route.fetch();
      seen(); await gate; try { await route.fulfill({ response }); } finally { delivered(); }
    } else { await route.continue(); }
  });
  await page.locator('#global-search').fill('Older missing result');
  await page.locator('#global-search').press('Enter');
  await held;
  const aborted = page.waitForEvent('requestfailed', { predicate: request => new URL(request.url()).searchParams.get('q') === 'Older missing result' });
  await page.locator('#global-search').fill('Beacon');
  await aborted;
  // No Enter: production debounce must issue this newer request itself.
  await expect(page.getByRole('button', { name: 'Play Beacon', exact: true })).toBeVisible();
  release(); await delivery;
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await expect(page.getByRole('button', { name: 'Play Beacon', exact: true })).toBeVisible();
  await page.locator('.search-categories').getByRole('button', { name: 'Albums', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'No search results', exact: true })).toBeVisible();
  await page.locator('.search-categories').getByRole('button', { name: 'Tracks', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Play Beacon', exact: true })).toBeVisible();
  await page.locator('#global-search').fill('я'.repeat(129));
  await expect(page.locator('.search-page').getByRole('alert')).toContainText('256 bytes');
  await expect(page.locator('.search-track-row')).toHaveCount(0);
  await page.getByRole('button', { name: 'Clear global search', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Find your next listen', exact: true }).last()).toBeVisible();
});

test('late play rejections cannot overwrite a newer genuine playback selection', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await openAdminFixture(page);
  await page.evaluate(() => {
    const play = HTMLMediaElement.prototype.play;
    let hold = true;
    HTMLMediaElement.prototype.play = function () {
      const actual = play.call(this);
      if (!hold) return actual;
      hold = false;
      actual.catch(() => {});
      return new Promise((_, reject) => { window.__rejectOldPlay = reject; });
    };
  });
  await page.getByRole('button', { name: 'Play Signal edited in Vela', exact: true }).click();
  await page.getByRole('button', { name: 'Play Beacon', exact: true }).click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
  await page.evaluate(() => window.__rejectOldPlay(new DOMException('Fixture old play aborted', 'AbortError')));
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await expect(page.getByRole('alert').filter({ hasText: 'Playback could not start' })).toHaveCount(0);
  await expect(page.locator('.now-playing strong')).toHaveText('Beacon');
  expect(errors).toEqual([]);
});

test('private search replies cannot populate another account after logout', async ({ page }) => {
  await page.goto('/');
  await page.locator('#username').fill('admin');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await navigate(page, 'Search');
  let release, seen, delivered;
  const delivery = new Promise(resolve => { delivered = resolve; });
  const gate = new Promise(resolve => { release = resolve; });
  const held = new Promise(resolve => { seen = resolve; });
  let once = false;
  await page.route('**/api/search?*', async route => {
    if (!once) {
      once = true;
      const response = await route.fetch();
      expect((await response.json()).tracks.length).toBeGreaterThan(0);
      seen(); await gate; try { await route.fulfill({ response }); } finally { delivered(); }
    } else { await route.continue(); }
  });
  await page.locator('#global-search').fill('Unbroken');
  await page.locator('#global-search').press('Enter');
  await held;
  await navigate(page, 'Settings');
  await page.getByRole('button', { name: 'Sign out', exact: true }).click();
  await page.locator('#username').fill('listener');
  await page.locator('#password').fill(password);
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await navigate(page, 'Search');
  await expect(page.locator('#global-search')).toHaveValue('');
  await page.locator('#global-search').fill('Unbroken');
  await page.locator('#global-search').press('Enter');
  await expect(page.getByRole('heading', { name: 'No search results', exact: true })).toBeVisible();
  release(); await delivery;
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await expect(page.getByRole('heading', { name: 'No search results', exact: true })).toBeVisible();
  await expect(page.locator('.search-track-row')).toHaveCount(0);
  await expect(page.locator('.now-playing strong')).toHaveText('Ready when you are');
});

test('private job replies cannot reappear after logout and another account signs in', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await openAdminFixture(page);
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  let seen;
  const held = new Promise(resolve => { seen = resolve; });
  let heldOnce = false;
  await page.route('**/api/jobs?*', async route => {
    if (!heldOnce) {
      heldOnce = true;
      const response = await route.fetch();
      expect((await response.json()).items.some(job => job.title === 'Job center archive')).toBeTruthy();
      seen();
      await gate;
      await route.fulfill({ response });
    } else { await route.continue(); }
  });
  await page.getByRole('button', { name: 'Jobs', exact: true }).first().click();
  await held;
  await page.getByRole('button', { name: /Settings/ }).first().click();
  await page.getByRole('button', { name: 'Sign out', exact: true }).click();
  await page.locator('#username').fill('listener');
  await page.locator('#password').fill(password);
  const signedIn = page.waitForResponse(response => new URL(response.url()).pathname === '/api/login');
  await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  await page.getByRole('button', { name: 'Library', exact: true }).first().click();
  expect((await signedIn).ok()).toBeTruthy();
  const older = page.waitForResponse(response => response.url().includes('/api/jobs?'));
  release();
  await older;
  await expect(page.getByRole('button', { name: 'Jobs', exact: true })).toHaveCount(0);
  const privateJobs = await (await page.request.get('/api/jobs')).json();
  expect(privateJobs.items).toEqual([]);
  await navigate(page, 'Search');
  await expect(page.locator('.background-job')).toHaveCount(0);
  await expect(page.getByText('Job center archive', { exact: true })).toHaveCount(0);
  await expect(page.locator('#job-type option[value="scan"]')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Music folder settings', exact: true })).toHaveCount(0);
  expect(errors).toEqual([]);
});
