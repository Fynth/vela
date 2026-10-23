import { test, expect } from '@playwright/test';
import { fileURLToPath } from 'node:url';
import { randomUUID } from 'node:crypto';
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

async function openAdminSession(page) {
  // Full serial runs reuse the first test's administrator session, keeping the
  // server's real per-IP login limiter untouched; targeted runs still sign in.
  if (adminCookies) {
    await page.context().addCookies(adminCookies);
    await page.goto('/');
    await expect(page.getByRole('button', { name: 'Library', exact: true }).first()).toBeVisible();
    return await (await page.request.get('/api/me')).json();
  }
  await page.goto('/');
  const setup = page.getByRole('button', { name: 'Create your music space →', exact: true });
  const signin = page.getByRole('button', { name: 'Sign in →', exact: true });
  // Boot is asynchronous: decide setup-vs-signin only after one of them exists.
  await expect(setup.or(signin)).toBeVisible({ timeout: 30000 });
  if (await setup.isVisible()) {
    await page.locator('#username').fill('admin');
    await page.locator('#password').fill(password);
    await setup.click();
    await expect(page.getByRole('heading', { name: 'Make room for your first listen', exact: true })).toBeVisible();
  } else {
    await page.locator('#username').fill('admin');
    await page.locator('#password').fill(password);
    await page.getByRole('button', { name: 'Sign in →', exact: true }).click();
  }
  await expect(page.getByRole('button', { name: 'Library', exact: true }).first()).toBeVisible();
  return await (await page.request.get('/api/me')).json();
}


// TR08: "More" opens Settings directly on the mobile composition; every other
// route is a real entry in the "More from Vela" list inside Settings.
async function navigate(page, name) {
  const mobile = page.getByRole('navigation', { name: 'Mobile navigation', exact: true });
  if (await mobile.isVisible()) {
    const tab = mobile.getByRole('button', { name, exact: true });
    if (await tab.count()) { await tab.click(); }
    else {
      await mobile.getByRole('button', { name: 'More destinations', exact: true }).click();
      if (name === 'Settings') return;
      await page.getByRole('navigation', { name: 'More from Vela', exact: true })
        .getByRole('button', { name, exact: true }).click();
    }
  } else { await page.getByRole('button', { name, exact: true }).first().click(); }
}

function pcmWave(duration, frequency) {
  const samples = 44100 * duration;
  const wave = Buffer.alloc(44 + samples * 2);
  wave.write('RIFF', 0); wave.writeUInt32LE(36 + samples * 2, 4);
  wave.write('WAVEfmt ', 8); wave.writeUInt32LE(16, 16);
  wave.writeUInt16LE(1, 20); wave.writeUInt16LE(1, 22);
  wave.writeUInt32LE(44100, 24); wave.writeUInt32LE(88200, 28);
  wave.writeUInt16LE(2, 32); wave.writeUInt16LE(16, 34);
  wave.write('data', 36); wave.writeUInt32LE(samples * 2, 40);
  for (let sample = 0; sample < samples; sample++) wave.writeInt16LE(Math.round(1500 * Math.sin(2 * Math.PI * frequency * sample / 44100)), 44 + sample * 2);
  return wave;
}

async function findTrack(page, title) {
  await navigate(page, 'Search');
  await page.locator('#global-search').fill(title);
  await page.locator('#global-search').press('Enter');
  await expect(page.getByRole('region', { name: 'Track results', exact: true })).toBeVisible();
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
  await expect(page.getByRole('heading', { name: 'Your library', exact: true })).toBeVisible();
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
  // The queue edits below assert that the cursor follows the occurrence it was
  // on. The tracked fixture is one second long, so a run that reaches natural
  // EOF between two edits advances the cursor for a reason that has nothing to
  // do with the edit (the same two assertions failed on origin/main). Playback
  // is paused here so the queue section measures the queue and not the clock;
  // the decode itself was already asserted above.
  await page.locator('#player-toggle').click();
  await expect(page.locator('#player-toggle')).toHaveAttribute('aria-label', 'Play');
  await page.getByRole('button', { name: 'Add Signal to playback queue', exact: true }).click();
  await page.getByRole('button', { name: 'Add Signal to playback queue', exact: true }).click();
  await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
  await expect(page.locator('.queue-row')).toHaveCount(3);
  await page.locator('.queue-row').nth(1).getByRole('button', { name: 'Move track up', exact: true }).click();
  await expect(page.locator('.queue-row').nth(1)).toHaveClass(/current/);
  await page.locator('.queue-row').nth(0).getByRole('button', { name: 'Move queue occurrence 1 down', exact: true }).click();
  await expect(page.locator('.queue-row').nth(0)).toHaveClass(/current/);
  await expect(page.locator('.queue-row').nth(0).getByRole('button', { name: 'Remove queued track', exact: true })).toBeDisabled();
  await expect(page.locator('#queue-current-remove-note')).toHaveCount(1);
  await page.getByRole('button', { name: 'Clear upcoming tracks', exact: true }).click();
  await expect(page.locator('.queue-row')).toHaveCount(1);
  await expect(page.locator('.queue-row').first()).toHaveClass(/current/);
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
  // TR07: a row's own menu opens the real searchable add-to-playlist picker; the
  // destination is chosen inside it and the receipt names that playlist.
  await page.getByRole('button', { name: 'More actions for Signal', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Add to playlist…', exact: true }).click();
  const playlistPicker = page.locator('#playlist-picker');
  await expect(playlistPicker).toBeVisible();
  await playlistPicker.getByRole('option', { name: /Evening signals/ }).click();
  await playlistPicker.getByRole('button', { name: 'Add to Evening signals', exact: true }).click();
  await expect(playlistPicker).toBeHidden();
  await expect(page.locator('.playlist-undo')).toContainText('Added “Signal” to Evening signals.');
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

test('playlist folders group, move, rename and delete without changing tracks', async ({page})=>{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await openAdminFixture(page);await navigate(page,'Playlists');
  await page.locator('#folder-name').fill('Folder QA');await page.getByRole('button',{name:'+ Folder',exact:true}).click();
  const folder=page.locator('[data-playlist-folder]').filter({hasText:'Folder QA (0)'});
  await expect(folder).toBeVisible();
  const folderId=await folder.getAttribute('data-playlist-folder');
  await page.locator('#playlist-name').fill('Folder list');await page.getByRole('button',{name:'+ Create',exact:true}).click();
  const card=page.locator('.playlist-card').filter({hasText:'Folder list'});await expect(card).toBeVisible();
  await card.click();
  await page.locator('#playlist-folder-target').selectOption(folderId);
  await page.getByRole('button',{name:'Move to folder',exact:true}).click();
  await expect(page.getByText('Playlist folder updated.',{exact:true})).toBeVisible();
  await page.getByRole('button',{name:/Folder QA \(1\)/}).click();
  await expect(page.locator('.playlist-card').filter({hasText:'Folder list'})).toBeVisible();
  await expect(page.locator('.playlist-card').filter({hasText:'Evening signals'})).toHaveCount(0);
  await page.locator('#folder-rename').fill('Folder QA renamed');
  await page.getByRole('button',{name:'Save folder name',exact:true}).click();
  await expect(page.locator('[data-playlist-folder]').filter({hasText:'Folder QA renamed'})).toBeVisible();
  await page.getByRole('button',{name:/Delete folder Folder QA renamed/}).click();
  await page.getByRole('button',{name:/Confirm delete folder Folder QA renamed/}).click();
  await expect(page.locator('[data-playlist-folder]')).toHaveCount(0);
  await expect(page.locator('.playlist-card').filter({hasText:'Folder list'})).toBeVisible();
  await page.setViewportSize({width:320,height:760});
  await expect(page.locator('.playlist-card').filter({hasText:'Folder list'})).toBeVisible();
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth+3)).toBeTruthy();
  await page.screenshot({path:'test-results/playlist-folders-phone.png',fullPage:true});
  expect(errors).toEqual([]);
});

test('owner renames and deletes a playlist without changing owned tracks', async ({ page }) => {
  await openAdminFixture(page);
  const before = await (await page.request.get('/api/tracks?limit=100')).json();
  await navigate(page, 'Playlists');
  await page.locator('#playlist-name').fill('Lifecycle list');
  await page.getByRole('button', { name: '+ Create', exact: true }).click();
  const created = page.locator('.playlist-card').filter({ hasText: 'Lifecycle list' });
  await expect(created).toBeVisible();
  await created.click();
  await expect(page.locator('.playlist-detail h2')).toHaveText('Lifecycle list');
  await expect(page.locator('#playlist-rename')).toHaveValue('Lifecycle list');

  await page.locator('#playlist-rename').fill('Lifecycle renamed');
  await page.getByRole('button', { name: 'Save name', exact: true }).click();
  await expect(page.getByText('Playlist renamed.', { exact: true })).toBeVisible();
  await expect(page.locator('.playlist-detail h2')).toHaveText('Lifecycle renamed');
  await expect(page.locator('.playlist-detail')).toContainText('Revision 1');
  await expect(created).toHaveCount(0);
  const renamedCard = page.locator('.playlist-card').filter({ hasText: 'Lifecycle renamed' });
  await expect(renamedCard).toBeVisible();

  const listed = await (await page.request.get('/api/playlists')).json();
  const target = listed.find(playlist => playlist.name === 'Lifecycle renamed');
  expect(target).toBeDefined();
  await page.getByRole('button', { name: 'Delete playlist', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Confirm delete', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.locator('.playlist-detail h2')).toHaveText('Lifecycle renamed');
  await page.getByRole('button', { name: 'Delete playlist', exact: true }).click();
  await page.getByRole('button', { name: 'Confirm delete', exact: true }).click();
  await expect(page.getByText('Playlist deleted. Your tracks and audio are unchanged.', { exact: true })).toBeVisible();
  await expect(page.locator('.playlist-detail')).toHaveCount(0);
  await expect(renamedCard).toHaveCount(0);
  expect((await page.request.get(`/api/playlists/${target.id}`)).status()).toBe(404);
  const after = await (await page.request.get('/api/tracks?limit=100')).json();
  expect(after.items.map(track => track.id)).toEqual(before.items.map(track => track.id));
  await page.screenshot({ path: 'test-results/playlist-lifecycle.png', fullPage: true });
});

test('sleep timer pauses the shared player and can be canceled', async ({ page }) => {
  await openAdminFixture(page);
  await page.getByRole('button', { name: '+ Add music', exact: true }).first().click();
  await page.locator('#audio-files').setInputFiles({ name: 'Sleep timer fixture.wav', mimeType: 'audio/wav', buffer: pcmWave(60, 330) });
  await page.getByRole('button', { name: 'Upload to my library', exact: true }).click();
  await expect(page.getByText('1 files processed.', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Close upload dialog' }).click();
  await page.getByRole('button', { name: 'Play Sleep timer fixture', exact: true }).click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => !audio.paused && audio.currentTime > 0)).toBeTruthy();
  await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
  await page.clock.install();
  await page.locator('#sleep-timer').selectOption('5');
  await expect(page.getByText(/Pauses in/)).toBeVisible();
  await page.screenshot({ path: 'test-results/sleep-timer.png' });
  await page.clock.runFor(4 * 60 * 1000);
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', false);
  await page.clock.runFor(61 * 1000);
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', true);
  await expect(page.getByText('Sleep timer reached zero and paused playback.', { exact: true })).toBeVisible();
  // The queue drawer is a modal surface, so the player bar behind it is
  // inert. The drawer carries its own real transport.
  const queueDrawer = page.getByRole('complementary', { name: 'Playback queue', exact: true });
  await queueDrawer.getByRole('button', { name: 'Resume playback', exact: true }).click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => !audio.paused)).toBeTruthy();
  await page.locator('#sleep-timer').selectOption('15');
  await page.getByRole('button', { name: 'Cancel sleep timer', exact: true }).click();
  await expect(page.getByText('Sleep timer canceled.', { exact: true })).toBeVisible();
  await page.clock.runFor(20 * 60 * 1000);
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', false);
  await page.getByRole('button', { name: 'Close playback queue', exact: true }).click();
});

test('malicious URL import rejected through UI and small-screen layout remains usable', async ({ page }) => {
  await openAdminFixture(page);
  await page.getByRole('button', { name: 'Add music', exact: true }).first().click();
  await page.locator('#import-url').fill('http://169.254.169.254/latest/meta-data/');
  await page.getByRole('button', { name: 'Preview download →', exact: true }).click();
  // The import error is the root's own inline alert. The query is scoped to
  // the content column: the root's feedback stack owns a permanent alert
  // region for the Undo failure, so a page-wide alert query is ambiguous.
  await expect(page.locator('.content-inner').getByRole('alert')).toBeVisible();
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
  await page.getByRole('button', { name: 'Add music', exact: true }).first().click();
  await page.locator('#import-url').fill('https://upload.wikimedia.org/wikipedia/commons/c/c8/Example.ogg');
  await page.getByRole('button', { name: 'Preview download →', exact: true }).click();
  await page.getByRole('button', { name: 'Import to my library', exact: true }).click();
  await expect(page.getByText('Your import is ready in the library.', { exact: true })).toBeVisible({ timeout: 90000 });
  await page.getByRole('button', { name: /Library/ }).first().click();
  expect(await page.locator('.track-row').count()).toBeGreaterThanOrEqual(2);
  await page.getByRole('button', { name: 'Play Example', exact: true }).click();
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
  // TR08: More is a destination that opens Settings itself — no intermediate
  // sheet — and Settings lists every remaining authorized route, including the
  // administrator-only Jobs view, as a real navigation control.
  await mobile.getByRole('button', { name: 'More destinations', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Settings', exact: true, level: 1 })).toBeVisible();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  const destinations = page.getByRole('navigation', { name: 'More from Vela', exact: true });
  for (const name of ['Albums', 'Artists', 'Favorites', 'Listening history', 'Add music', 'Network', 'Jobs']) {
    await expect(destinations.getByRole('button', { name, exact: true })).toBeVisible();
  }
  // The hub is the first Settings surface, so More cannot land below it.
  expect(await page.evaluate(() => {
    const hub = document.querySelector('[data-testid="settings-destinations"]');
    const appearance = document.querySelector('[data-testid="settings-appearance"]');
    return !!hub && !!appearance && Boolean(hub.compareDocumentPosition(appearance) & Node.DOCUMENT_POSITION_FOLLOWING);
  })).toBeTruthy();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBeTruthy();
  await destinations.getByRole('button', { name: 'Jobs', exact: true }).click();
  await expect(page.locator('.background-job').first()).toBeVisible();
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
  const wave = pcmWave(45, 330);
  const upload = await page.request.post('/api/upload', { headers: { 'X-CSRF-Token': auth.csrf_token }, multipart: { file: { name: 'Unbroken session.wav', mimeType: 'audio/wav', buffer: wave } } });
  expect(upload.ok()).toBeTruthy();
  const track = (await upload.json()).track;
  await navigate(page, 'Playlists');
  await page.locator('#playlist-name').fill('Vela listening');
  await page.getByRole('button', { name: '+ Create', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Vela listening', exact: true })).toBeVisible();
  await navigate(page, 'Home');
  await expect(page.getByRole('heading', { name: 'In your own rhythm', exact: true })).toBeVisible();
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
  // TR08: keyboard activation of the "More" destination opens Settings itself and
  // the replacement list is reachable the same way. No intermediate sheet, so no
  // dialog focus containment/Escape contract is asserted here any more.
  const more = page.getByRole('button', { name: 'More destinations', exact: true });
  await more.focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { name: 'Settings', exact: true, level: 1 })).toBeVisible();
  await expect(more).toHaveAttribute('aria-current', 'page');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.locator('.more-sheet, .more-backdrop')).toHaveCount(0);
  const destinations = page.getByRole('navigation', { name: 'More from Vela', exact: true });
  const albums = destinations.getByRole('button', { name: 'Albums', exact: true });
  await albums.focus();
  expect(await albums.evaluate(node => node === document.activeElement)).toBeTruthy();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { name: 'Albums', exact: true, level: 1 })).toBeVisible();
  await expect(more).toHaveAttribute('aria-current', 'true');
  await checkPlayer();
  await page.reload();
  await navigate(page, 'Library');
  await page.getByRole('button', { name: 'Show playback queue and controls', exact: true }).click();
  await expect(page.locator('.queue-row .queue-track strong')).toHaveText([track.title, 'Beacon', 'Beacon', 'Signal edited in Vela']);
  await expect(page.locator('.queue-row').first()).toHaveClass(/current/);
  expect(errors).toEqual([]);
});

test('IN01 Home and search skeletons keep real geometry without changing playback', async ({ page }) => {
  test.setTimeout(180000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const auth = await openAdminSession(page);
  // Long enough that genuine playback cannot end while the held requests resolve.
  const wave = pcmWave(60, 440);
  const upload = await page.request.post('/api/upload', { headers: { 'X-CSRF-Token': auth.csrf_token }, multipart: { file: { name: 'IN01 skeleton source.wav', mimeType: 'audio/wav', buffer: wave } } });
  expect(upload.ok()).toBeTruthy();
  const track = (await upload.json()).track;
  // Catalog-only artist metadata makes the real card a complete title/artist row,
  // the shape the skeleton reserves; the audio file itself is untouched.
  const metadata = await (await page.request.get(`/api/tracks/${track.id}/metadata`)).json();
  const cataloged = await page.request.patch(`/api/tracks/${track.id}/metadata`, { headers: { 'X-CSRF-Token': auth.csrf_token }, data: { revision: metadata.revision, title: track.title, artist: 'Vela Test', album: 'IN01 geometry', album_artist: 'Vela Test', genre: '', year: null, track_number: null, disc_number: null } });
  expect(cataloged.ok()).toBeTruthy();

  // Search issues a fresh authenticated request, so a just-uploaded track does
  // not depend on a stale Library list (a Firefox-observed race).
  await navigate(page, 'Search');
  await page.locator('#global-search').fill(track.title);
  await page.locator('#global-search').press('Enter');
  const play = page.getByRole('button', { name: `Play ${track.title}`, exact: true }).first();
  await expect(play).toBeVisible({ timeout: 15000 });
  await play.click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
  await page.evaluate(() => { window.__in01Audio = document.getElementById('vela-audio'); });
  const playback = await page.locator('#vela-audio').evaluate(audio => ({ src: audio.getAttribute('src'), generation: audio.getAttribute('data-source-generation') }));
  expect(playback.src).toBeTruthy();
  expect(playback.generation).toBeTruthy();
  const playbackUnchanged = async () => {
    const now = await page.locator('#vela-audio').evaluate(audio => ({ same: audio === window.__in01Audio, src: audio.getAttribute('src'), generation: audio.getAttribute('data-source-generation') }));
    expect(now.same).toBeTruthy();
    expect(now.src).toBe(playback.src);
    expect(now.generation).toBe(playback.generation);
  };
  const cardMetrics = selector => page.locator(selector).first().evaluate(card => {
    const rect = card.getBoundingClientRect();
    const style = getComputedStyle(card);
    const art = card.querySelector('.artwork').getBoundingClientRect();
    const grid = getComputedStyle(card.parentElement);
    return { height: rect.height, padding: style.padding, columns: grid.gridTemplateColumns, gap: grid.gap, artWidth: art.width, artHeight: art.height };
  });

  let homeHold = null;
  let homeFailure = false;
  await page.route('**/api/tracks?*', async route => {
    const params = new URL(route.request().url()).searchParams;
    const recent = params.get('sort') === 'newest' && !params.has('favorites');
    if (recent && homeFailure) {
      homeFailure = false;
      await route.fulfill({ status: 503, contentType: 'application/json', body: JSON.stringify({ error: 'fixture tracks outage' }) });
      return;
    }
    if (recent && homeHold) {
      const hold = homeHold;
      hold.seen();
      await hold.gate;
      try { const response = await route.fetch(); await route.fulfill({ response }); } catch {}
      hold.deliver();
      return;
    }
    await route.continue();
  });
  const holdHome = () => {
    let release, seen, delivered;
    const gate = new Promise(resolve => { release = resolve; });
    const held = new Promise(resolve => { seen = resolve; });
    const done = new Promise(resolve => { delivered = resolve; });
    homeHold = { gate, seen, deliver: delivered };
    return { held, done, release };
  };

  const firstHome = holdHome();
  await navigate(page, 'Home');
  await firstHome.held;
  const loadingHome = page.getByRole('status', { name: 'Loading Home', exact: true });
  await expect(loadingHome).toBeVisible();
  await expect(loadingHome).toHaveAttribute('aria-busy', 'true');
  await expect(page.locator('.track-skeleton-card')).toHaveCount(4);
  await expect(page.locator('.track-skeleton-card').first().locator('.track-skeleton-art')).toBeVisible();
  await expect(page.locator('.track-skeleton-card').first().locator('.track-skeleton-bar')).toHaveCount(2);
  const skeleton = await cardMetrics('.track-skeleton-card');
  expect(Math.abs(skeleton.artWidth - skeleton.artHeight)).toBeLessThanOrEqual(1);
  expect(await page.locator('.track-skeleton-card .track-skeleton-art').first().evaluate(node => getComputedStyle(node).animationName)).toBe('vela-track-loading');
  await playbackUnchanged();
  firstHome.release(); await firstHome.done;
  homeHold = null;
  const realCards = page.locator('.home-track-grid .listening-card:not(.track-skeleton-card)');
  await expect(realCards.first()).toBeVisible();
  await expect(page.locator('.track-skeleton-card')).toHaveCount(0);
  const real = await cardMetrics('.home-track-grid .listening-card:not(.track-skeleton-card)');
  expect(real.padding).toBe(skeleton.padding);
  const skeletonColumns = skeleton.columns.split(' ').map(width => parseFloat(width));
  const realColumns = real.columns.split(' ').map(width => parseFloat(width));
  expect(realColumns.length).toBe(skeletonColumns.length);
  realColumns.forEach((width, at) => expect(Math.abs(width - skeletonColumns[at])).toBeLessThanOrEqual(0.5));
  expect(real.gap).toBe(skeleton.gap);
  expect(Math.abs(real.height - skeleton.height)).toBeLessThanOrEqual(2);
  expect(Math.abs(real.artHeight - skeleton.artHeight)).toBeLessThanOrEqual(2);
  await playbackUnchanged();

  // Known rows stay while a same-view refresh is pending, and a failed refresh
  // keeps them instead of falling back to placeholders.
  const pendingHome = holdHome();
  await page.getByRole('button', { name: 'Refresh Home', exact: true }).click();
  await pendingHome.held;
  await expect(realCards.first()).toBeVisible();
  await expect(page.locator('.track-skeleton-card')).toHaveCount(0);
  await playbackUnchanged();
  pendingHome.release(); await pendingHome.done;
  homeHold = null;
  homeFailure = true;
  await page.getByRole('button', { name: 'Refresh Home', exact: true }).click();
  await expect(page.locator('.home-page .alert.error')).toBeVisible();
  await expect(page.locator('.track-skeleton-card')).toHaveCount(0);
  await expect(realCards.first()).toBeVisible();
  await playbackUnchanged();

  await page.emulateMedia({ reducedMotion: 'reduce' });
  let searchHold = null;
  let searchFailure = null;
  await page.route('**/api/search?*', async route => {
    const query = new URL(route.request().url()).searchParams.get('q');
    if (searchFailure === query) {
      searchFailure = null;
      await route.fulfill({ status: 503, contentType: 'application/json', body: JSON.stringify({ error: 'fixture search outage' }) });
      return;
    }
    if (searchHold && searchHold.query === query) {
      const hold = searchHold;
      hold.seen();
      await hold.gate;
      try { const response = await route.fetch(); await route.fulfill({ response }); } catch {}
      hold.delivered();
      return;
    }
    await route.continue();
  });
  const holdSearch = query => {
    let release, seen, delivered;
    const gate = new Promise(resolve => { release = resolve; });
    const held = new Promise(resolve => { seen = resolve; });
    const done = new Promise(resolve => { delivered = resolve; });
    searchHold = { query, gate, seen, delivered };
    return { held, done, release };
  };

  const firstSearch = holdSearch(track.title);
  await navigate(page, 'Search');
  await page.locator('#global-search').fill(track.title);
  await firstSearch.held;
  const loadingSearch = page.getByRole('status', { name: 'Searching your library', exact: true });
  await expect(loadingSearch).toBeVisible();
  await expect(loadingSearch).toHaveAttribute('aria-busy', 'true');
  await expect(page.locator('.track-skeleton-row')).toHaveCount(3);
  expect(await page.locator('.track-skeleton-row .track-skeleton-art').first().evaluate(node => getComputedStyle(node).animationName)).toBe('none');
  expect(await page.locator('.track-skeleton-row .track-skeleton-bar').first().evaluate(node => getComputedStyle(node).animationName)).toBe('none');
  await playbackUnchanged();
  firstSearch.release(); await firstSearch.done;
  searchHold = null;
  const realRows = page.locator('.search-track-row:not(.track-skeleton-row)');
  await expect(realRows.first()).toBeVisible();
  await expect(page.locator('.track-skeleton-row')).toHaveCount(0);
  await playbackUnchanged();

  const refreshSearch = holdSearch(track.title);
  await page.locator('#global-search').press('Enter');
  await refreshSearch.held;
  await expect(realRows.first()).toBeVisible();
  await expect(page.locator('.track-skeleton-row')).toHaveCount(0);
  await expect(page.getByRole('status', { name: 'Searching your library', exact: true })).toHaveCount(0);
  await playbackUnchanged();
  refreshSearch.release(); await refreshSearch.done;
  searchHold = null;

  searchFailure = 'IN01 missing title';
  await page.locator('#global-search').fill('IN01 missing title');
  await expect(page.locator('.search-page .alert.error')).toBeVisible();
  await expect(page.getByRole('status', { name: 'Searching your library', exact: true })).toHaveCount(0);
  await expect(page.locator('.track-skeleton-row')).toHaveCount(0);
  await playbackUnchanged();
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
  await findTrack(page, 'Signal');
  await page.getByRole('button', { name: 'Play Signal edited in Vela', exact: true }).click();
  await findTrack(page, 'Beacon');
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
  // Home's approved composition has no hero play-all; start the same real
  // session from the library rows instead.
  await page.getByRole('button', { name: 'Library', exact: true }).first().click();
  await page.locator('.track-row .track-play').first().click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
  await page.getByRole('button', { name: 'Shuffle', exact: true }).click();
  await page.getByRole('button', { name: 'Repeat: off', exact: true }).click();
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
  await expect(page.locator('.now-playing strong')).toHaveText('Nothing playing');
  await expect(page.getByRole('button', { name: 'Shuffle', exact: true })).toHaveAttribute('aria-pressed', 'false');
  await expect(page.getByRole('button', { name: 'Repeat: off', exact: true })).toBeVisible();
});

test('playlist replies cannot reopen abandoned views or put an old share link on another playlist', async ({ page }) => {
  await openAdminFixture(page);
  await navigate(page, 'Playlists');
  const lists = await (await page.request.get('/api/playlists')).json();
  const first = lists.find(list => list.name === 'Vela listening');
  const second = lists.find(list => list.name === 'Evening signals');
  expect(Boolean(first && second)).toBeTruthy();
  const hold = async (path, method) => {
    let release, seen;
    const gate = new Promise(resolve => { release = resolve; });
    const held = new Promise(resolve => { seen = resolve; });
    let once = false;
    await page.route(path, async route => {
      if (!once && route.request().method() === method) {
        once = true;
        const response = await route.fetch();
        expect(response.ok()).toBeTruthy();
        seen(); await gate; await route.fulfill({ response });
      } else { await route.continue(); }
    });
    return { held, release: async () => {
      const reply = page.waitForResponse(response => response.request().method() === method && new URL(response.url()).pathname === path.replace('**', ''));
      release(); await reply;
      await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
      await page.unroute(path);
    } };
  };
  const open = async name => {
    await page.locator('.playlist-card').filter({ hasText: name }).click();
    await expect(page.locator('.playlist-detail h2')).toHaveText(name);
  };
  const pending = await hold(`**/api/playlists/${first.id}`, 'GET');
  await page.locator('.playlist-card').filter({ hasText: first.name }).click();
  await pending.held;
  await navigate(page, 'Home');
  await navigate(page, 'Playlists');
  await pending.release();
  await expect(page.locator('.playlist-detail')).toHaveCount(0);

  await open(second.name);
  const pendingRefresh = await hold(`**/api/playlists/${second.id}`, 'GET');
  await page.getByRole('button', { name: 'Remove from playlist', exact: true }).first().click();
  await pendingRefresh.held;
  await open(first.name);
  await pendingRefresh.release();
  await expect(page.locator('.playlist-detail h2')).toHaveText(first.name);
  expect((await (await page.request.get(`/api/playlists/${second.id}`)).json()).items).toHaveLength(0);

  const pendingShare = await hold(`**/api/playlists/${first.id}/shares`, 'POST');
  await page.getByRole('button', { name: 'Create view link', exact: true }).click();
  await pendingShare.held;
  await open(second.name);
  await pendingShare.release();
  await expect(page.locator('.playlist-detail h2')).toHaveText(second.name);
  await expect(page.locator('#share-link')).toHaveCount(0);

  // An authenticated API edit advances the revision while an older refresh is held.
  const auth = await (await page.request.get('/api/me')).json();
  const tracks = await (await page.request.get('/api/tracks?q=Unbroken&limit=1')).json();
  const add = async () => {
    const current = await (await page.request.get(`/api/playlists/${first.id}`)).json();
    const result = await page.request.post(`/api/playlists/${first.id}/items`, {
      headers: { 'X-CSRF-Token': auth.csrf_token },
      data: { revision: current.playlist.revision, mutation_id: randomUUID(), track_id: tracks.items[0].id, item_ids: null },
    });
    expect(result.ok()).toBeTruthy();
  };
  await add(); await add();
  await open(first.name);
  const olderRevision = await hold(`**/api/playlists/${first.id}`, 'GET');
  await page.getByRole('button', { name: 'Remove from playlist', exact: true }).first().click();
  await olderRevision.held;
  await add();
  const latest = await (await page.request.get(`/api/playlists/${first.id}`)).json();
  // The stale UI mutation gets a genuine409, then fetches the newer snapshot.
  await page.getByRole('button', { name: 'Move track up in playlist', exact: true }).nth(1).click();
  await expect(page.locator('.playlist-detail')).toContainText(`Revision ${latest.playlist.revision}`);
  await olderRevision.release();
  await expect(page.locator('.playlist-detail')).toContainText(`Revision ${latest.playlist.revision}`);
  await expect(page.locator('.playlist-detail .playlist-track')).toHaveCount(latest.items.length);
});

test('shuffle visits each occurrence once, previous follows history and repeat uses actual audio endings', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await openAdminFixture(page);
  const auth = await (await page.request.get('/api/me')).json();
  for (const [name, frequency] of [['Drift', 440], ['Harbor', 550]]) {
    const response = await page.request.post('/api/upload', { headers: { 'X-CSRF-Token': auth.csrf_token }, multipart: { file: { name: `${name}.wav`, mimeType: 'audio/wav', buffer: pcmWave(30, frequency) } } });
    expect(response.ok()).toBeTruthy();
  }
  await findTrack(page, 'Unbroken session');
  await page.getByRole('button', { name: 'Play Unbroken session', exact: true }).click();
  for (const title of ['Drift', 'Harbor']) {
    await findTrack(page, title);
    await page.getByRole('button', { name: `Add ${title} to playback queue`, exact: true }).click();
  }
  await page.getByRole('button', { name: 'Shuffle', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Shuffle', exact: true })).toHaveAttribute('aria-pressed', 'true');
  const selected = async () => page.locator('#vela-audio').getAttribute('src');
  const heard = [await selected()];
  for (let step = 0; step < 2; step++) {
    const previous = await selected();
    await page.getByRole('button', { name: 'Next track', exact: true }).click();
    await expect.poll(selected).not.toBe(previous);
    await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', false);
    heard.push(await selected());
  }
  expect(new Set(heard).size).toBe(3);
  await page.getByRole('button', { name: 'Next track', exact: true }).click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', true);
  expect(await selected()).toBe(heard[2]);
  await page.getByRole('slider', { name: 'Playback position', exact: true }).evaluate(input => { input.value = '0'; input.dispatchEvent(new Event('input', { bubbles: true })); });
  await page.getByRole('button', { name: 'Previous track', exact: true }).click();
  await expect(page.locator('#vela-audio')).toHaveAttribute('src', heard[1]);
  await page.getByRole('button', { name: 'Next track', exact: true }).click();
  await expect(page.locator('#vela-audio')).toHaveAttribute('src', heard[2]);
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.readyState)).toBeGreaterThanOrEqual(2);
  await page.getByRole('slider', { name: 'Playback position', exact: true }).evaluate(input => { input.value = '5'; input.dispatchEvent(new Event('input', { bubbles: true })); });
  await page.getByRole('button', { name: 'Pause', exact: true }).click();
  await page.getByRole('button', { name: 'Previous track', exact: true }).click();
  expect(await selected()).toBe(heard[2]);
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', true);
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeLessThan(1);
  await page.getByRole('button', { name: 'Repeat: off', exact: true }).click();
  await page.getByRole('button', { name: 'Repeat: all', exact: true }).click();
  await page.getByRole('button', { name: 'Play', exact: true }).click();
  await page.locator('#vela-audio').evaluate(audio => { audio.currentTime = audio.duration - 0.15; });
  // Let the actual decoder reach EOF; do not dispatch an artificial ended event.
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeLessThan(2);
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', false);
  expect(await selected()).toBe(heard[2]);
  await page.getByRole('button', { name: 'Next track', exact: true }).click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', true);
  expect(await selected()).toBe(heard[2]);
  await page.getByRole('button', { name: 'Repeat: one', exact: true }).click();
  await page.getByRole('button', { name: 'Repeat: off', exact: true }).click();
  await page.getByRole('button', { name: 'Next track', exact: true }).click();
  await expect.poll(selected).not.toBe(heard[2]);
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', false);
  expect(errors).toEqual([]);
});

test('explicit next and duplicate occurrence history survive queue reorder and removal in shuffle', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await openAdminFixture(page);
  await findTrack(page, 'Unbroken session');
  await page.getByRole('button', { name: 'Play Unbroken session', exact: true }).click();
  for (const title of ['Drift', 'Harbor']) {
    await findTrack(page, title);
    await page.getByRole('button', { name: `Add ${title} to playback queue`, exact: true }).click();
  }
  await page.getByRole('button', { name: 'Shuffle', exact: true }).click();
  await findTrack(page, 'Drift');
  await page.getByRole('button', { name: 'Play Drift next', exact: true }).click();
  await page.getByRole('button', { name: 'Next track', exact: true }).click();
  await expect(page.locator('.now-playing strong')).toHaveText('Drift');
  await findTrack(page, 'Harbor');
  await page.getByRole('button', { name: 'Play Harbor next', exact: true }).click();
  const drift = await page.locator('#vela-audio').getAttribute('src');
  await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
  await expect(page.locator('.queue-row .queue-track strong')).toHaveText(['Unbroken session', 'Drift', 'Harbor', 'Drift', 'Harbor']);
  await page.locator('.queue-row').nth(2).getByRole('button', { name: 'Move track up', exact: true }).click();
  await expect(page.locator('.queue-row').nth(2)).toHaveClass(/current/);
  await page.locator('.queue-row').first().getByRole('button', { name: 'Remove queued track', exact: true }).click();
  await expect(page.locator('.queue-row .queue-track strong')).toHaveText(['Harbor', 'Drift', 'Drift', 'Harbor']);
  await expect(page.locator('.queue-row').nth(1)).toHaveClass(/current/);
  await expect(page.locator('#vela-audio')).toHaveAttribute('src', drift);
  // The queue drawer is a modal surface: the player bar behind it is inert,
  // so the drawer's own transport drives the same root actions.
  const drawer = page.getByRole('complementary', { name: 'Playback queue', exact: true });
  await drawer.getByRole('button', { name: 'Next in queue', exact: true }).click();
  await expect(page.locator('.queue-row').first()).toHaveClass(/current/);
  await expect(page.locator('.now-playing strong')).toHaveText('Harbor');
  await drawer.getByRole('slider', { name: 'Queue playback position', exact: true }).evaluate(input => { input.value = '0'; input.dispatchEvent(new Event('input', { bubbles: true })); });
  await drawer.getByRole('button', { name: 'Previous in queue', exact: true }).click();
  await expect(page.locator('.queue-row').nth(1)).toHaveClass(/current/);
  await expect(page.locator('.now-playing strong')).toHaveText('Drift');
  await drawer.getByRole('button', { name: 'Next in queue', exact: true }).click();
  await expect(page.locator('.queue-row').first()).toHaveClass(/current/);
  await page.reload();
  await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
  await expect(page.locator('.queue-row .queue-track strong')).toHaveText(['Harbor', 'Drift', 'Drift', 'Harbor']);
  await expect(page.locator('.queue-row').first()).toHaveClass(/current/);
  await expect(page.getByRole('button', { name: 'Shuffle', exact: true }).first()).toHaveAttribute('aria-pressed', 'true');
  expect(errors).toEqual([]);
});

// RES01–04: actual long PCM and the real account-scoped bookmark, no fake catalog.
test('RES01 restored duplicate occurrence keeps position context and preferences without autoplay', async ({ page }) => {
  const errors=[]; page.on('pageerror', e=>errors.push(e.message));
  await openAdminFixture(page);
  await findTrack(page,'Drift');
  await page.getByRole('button',{name:'Play Drift',exact:true}).click();
  await page.getByRole('button',{name:'Add Drift to playback queue',exact:true}).click();
  await findTrack(page,'Harbor');
  await page.getByRole('button',{name:'Add Harbor to playback queue',exact:true}).click();
  await page.getByRole('button',{name:'Show playback queue',exact:true}).click();
  const drawer = page.getByRole('complementary', { name: 'Playback queue', exact: true });
  await page.locator('.queue-row').nth(1).getByRole('button',{name:'Drift',exact:false}).first().click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.readyState)).toBeGreaterThanOrEqual(2);
  await drawer.getByRole('slider',{name:'Queue playback position',exact:true}).evaluate(e=>{e.value='11';e.dispatchEvent(new Event('input',{bubbles:true}));});
  await drawer.getByRole('button',{name:'Pause playback',exact:true}).click();
  await page.locator('.queue-row').nth(2).getByRole('button',{name:'Move track up',exact:true}).click();
  await page.locator('.queue-row').first().getByRole('button',{name:'Remove queued track',exact:true}).click();
  await expect(page.locator('.queue-row').nth(1)).toHaveClass(/current/);
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThanOrEqual(11);
  await drawer.getByRole('button',{name:'Shuffle',exact:true}).click();
  await drawer.getByRole('button',{name:'Repeat in queue: off',exact:true}).click();
  await drawer.getByRole('slider',{name:'Queue volume',exact:true}).evaluate(e=>{e.value='0.37';e.dispatchEvent(new Event('input',{bubbles:true}));});
  const account=await (await page.request.get('/api/me')).json();
  const key=`vela.${account.user.id}.playback.v1`;
  const bookmark=await page.evaluate(key=>JSON.parse(localStorage.getItem(key)),key);
  expect(bookmark.cursor).toBe(1);expect(bookmark.position).toBeGreaterThanOrEqual(11);expect(bookmark.context.kind).toBe('search');
  expect(bookmark.volume).toBe(0.37);expect(bookmark.shuffle).toBe(true);expect(bookmark.repeat).toBe('all');
  expect(JSON.stringify(bookmark)).not.toContain(account.csrf_token);
  let streams=0;page.on('request',r=>{if(/\/api\/tracks\/[^/]+\/stream$/.test(r.url())) streams++;});
  await page.reload();
  await expect(page.locator('.now-playing strong')).toHaveText('Drift');
  expect(Math.abs(Number(await page.getByRole('slider',{name:'Playback position',exact:true}).inputValue())-bookmark.position)).toBeLessThan(0.2);
  expect(streams).toBe(0);expect(await page.locator('#vela-audio').getAttribute('src')).toBeNull();
  await page.getByRole('button',{name:'Show playback queue',exact:true}).click();
  await expect(page.locator('.queue-row .queue-track strong')).toHaveText(['Harbor','Drift']);
  await expect(page.locator('.queue-row').nth(1)).toHaveClass(/current/);
  await expect(page.locator('.queue-context')).toHaveText('Playing from Search results');
  await expect(drawer.getByRole('button',{name:'Shuffle',exact:true})).toHaveAttribute('aria-pressed','true');
  await drawer.getByRole('button',{name:'Shuffle',exact:true}).click();
  // A paused restored bookmark has no src: sequential Previous must use its saved elapsed time.
  await drawer.getByRole('button',{name:'Previous in queue',exact:true}).click();
  await expect(page.locator('.queue-row').nth(1)).toHaveClass(/current/);
  expect(await page.locator('#vela-audio').getAttribute('src')).toBeNull();
  await expect(drawer.getByRole('slider',{name:'Queue playback position',exact:true})).toHaveValue('0');
  expect(streams).toBe(0);
  await drawer.getByRole('slider',{name:'Queue playback position',exact:true}).evaluate(e=>{e.value='11';e.dispatchEvent(new Event('input',{bubbles:true}));});

  await drawer.getByRole('button',{name:'Resume playback',exact:true}).click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThanOrEqual(11);
  expect(await page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeLessThan(15);
  await page.getByRole('button',{name:'Close playback queue',exact:true}).click();
  await navigate(page,'Home');await navigate(page,'Library');
  await expect(page.locator('.now-playing strong')).toHaveText('Drift');
  await findTrack(page,'Harbor');await page.getByRole('button',{name:'Play Harbor',exact:true}).click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  expect(await page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeLessThan(3);
  expect(errors).toEqual([]);
});

test('RES02 malformed foreign and oversized bookmarks do not fetch or expose queue data',async({page})=>{
  await openAdminFixture(page);
  const account=await (await page.request.get('/api/me')).json();
  await findTrack(page,'Drift');await page.getByRole('button',{name:'Play Drift',exact:true}).click();
  await page.getByRole('button',{name:'Pause',exact:true}).click();
  const key=`vela.${account.user.id}.playback.v1`;
  const raw=await page.evaluate(k=>localStorage.getItem(k),key);
  let streams=0;page.on('request',r=>{if(/\/stream$/.test(r.url()))streams++;});
  await page.addInitScript(()=>{
    const config=JSON.parse(localStorage.getItem('vela-fixture-bookmark-injection')||'null');
    if(config){localStorage.removeItem('vela-fixture-bookmark-injection');localStorage.setItem(config.key,config.invalid);localStorage.removeItem(`vela.${config.id}.queue`);localStorage.removeItem(`vela.${config.id}.queue_index`);}
  });
  for(const invalid of ['{', JSON.stringify({...JSON.parse(raw),owner_id:randomUUID()}),JSON.stringify({...JSON.parse(raw),cursor:1001}), ' '.repeat(4*1024*1024+1)]){
    await page.evaluate(config=>localStorage.setItem('vela-fixture-bookmark-injection',JSON.stringify(config)),{key,invalid,id:account.user.id});
    await page.reload();
    await expect(page.locator('.now-playing strong')).toHaveText('Nothing playing');
    await expect(page.getByRole('button',{name:'Play',exact:true})).toBeDisabled();
    await page.getByRole('button',{name:'Show playback queue',exact:true}).click();
    await expect(page.locator('.queue-row')).toHaveCount(0);
    await page.getByRole('button',{name:'Close playback queue',exact:true}).click();
  }
  expect(streams).toBe(0);
});

async function pausedBookmark(page,title='Drift',position=9){
  await openAdminFixture(page);await findTrack(page,title);
  await page.getByRole('button',{name:`Play ${title}`,exact:true}).click();
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.readyState)).toBeGreaterThanOrEqual(2);
  await page.getByRole('slider',{name:'Playback position',exact:true}).evaluate((e,p)=>{e.value=String(p);e.dispatchEvent(new Event('input',{bubbles:true}));},position);
  await page.getByRole('button',{name:'Pause',exact:true}).click();
  // Release this fixture's buffered resource before reload. Firefox can reuse
  // it without any network request, which would bypass the transport fault.
  await releaseTestMedia(page);await page.reload();
  await expect(page.locator('.now-playing strong')).toHaveText(title);
  const saved=Number(await page.getByRole('slider',{name:'Playback position',exact:true}).inputValue());
  expect(saved).toBeGreaterThanOrEqual(position);expect(saved).toBeLessThan(position+1);
}
async function releaseTestMedia(page){
  await page.locator('#vela-audio').evaluate(a=>{a.pause();a.removeAttribute('src');a.load();});
}

test('NET01 genuine failed transfer recovers via authorized HEAD at the saved seek',async({page})=>{
  const errors=[];page.on('pageerror',e=>errors.push(e.message));await pausedBookmark(page);
  let failures=0,heads=0,probeGeneration;
  await page.route('**/api/tracks/*/stream',async route=>{
    if(route.request().method()==='HEAD'){
      probeGeneration=await page.locator('#vela-audio').getAttribute('data-source-generation');
      heads++;await route.continue();
    }
    // Keep the genuine transport outage until Vela performs its authorized probe.
    else if(heads===0){failures++;await route.abort('connectionreset');}else{await route.continue();}
  });
  await page.getByRole('button',{name:'Play',exact:true}).click();
  await expect.poll(()=>heads,{timeout:15000}).toBe(1);expect(failures).toBeGreaterThan(0);
  // Attribute the successful reload to Vela, not a browser-only retry while
  // the authorized HEAD is still in flight.
  await expect.poll(()=>page.locator('#vela-audio').getAttribute('data-source-generation')).not.toBe(probeGeneration);
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThanOrEqual(9);
  expect(await page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeLessThan(14);
  expect(errors).toEqual([]);
});

test('NET02 pause and a newer duplicate occurrence cancel held recovery and retries are bounded',async({page})=>{
  const errors=[];page.on('pageerror',e=>errors.push(e.message));await pausedBookmark(page);
  let release;const gate=new Promise(resolve=>release=resolve);let seen;const held=new Promise(resolve=>seen=resolve);
  let gets=0,heads=0;
  await page.route('**/api/tracks/*/stream',async route=>{
    if(route.request().method()==='HEAD'){heads++;const response=await route.fetch();seen();await gate;try{await route.fulfill({response});}catch{ /* physically aborted */ }}
    else if(heads===0){gets++;await route.abort('connectionreset');}else{await route.continue();}
  });
  await page.getByRole('button',{name:'Play',exact:true}).click();await held;expect(gets).toBeGreaterThan(0);
  await page.getByRole('button',{name:'Pause',exact:true}).click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',true);
  // Select the *other* queued Drift occurrence. The current library row now
  // correctly resumes its paused occurrence instead of resetting it to zero.
  await findTrack(page,'Drift');
  await page.getByRole('button',{name:'Add Drift to playback queue',exact:true}).click();
  await page.getByRole('button',{name:'Show playback queue',exact:true}).click();
  await page.locator('.queue-row').last().locator('.queue-track').click();
  await page.getByRole('button',{name:'Close playback queue',exact:true}).click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  expect(await page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeLessThan(3);
  const generation=await page.locator('#vela-audio').getAttribute('data-source-generation');
  await page.getByRole('slider',{name:'Playback position',exact:true}).evaluate(e=>{e.value='6';e.dispatchEvent(new Event('input',{bubbles:true}));});
  release();
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  expect(await page.locator('#vela-audio').getAttribute('data-source-generation')).toBe(generation);
  expect(await page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThanOrEqual(6);
  expect(heads).toBe(1);
  await page.getByRole('button',{name:'Pause',exact:true}).click();await releaseTestMedia(page);await page.reload();
  await page.unroute('**/api/tracks/*/stream');
  let probes=0;
  await page.route('**/api/tracks/*/stream',async route=>{if(route.request().method()==='HEAD')probes++;await route.abort('connectionreset');});
  await page.getByRole('button',{name:'Play',exact:true}).click();
  await expect(page.getByRole('status').filter({hasText:'Press Play to retry from your saved position'})).toBeVisible({timeout:15000});
  expect(probes).toBe(3);
  await page.screenshot({path:'test-results/resume-recovery-desktop.png'});
  await page.setViewportSize({width:320,height:780});await page.emulateMedia({reducedMotion:'reduce'});
  for(const theme of ['dark','light']){
    if(await page.locator('html').getAttribute('data-theme')!==theme)await page.getByRole('button',{name:'Change color theme',exact:true}).click();
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBeTruthy();
    await expect(page.getByRole('button',{name:'Retry playback',exact:true})).toBeInViewport();
    expect(await page.getByRole('button',{name:'Retry playback',exact:true}).evaluate(e=>e.getBoundingClientRect().height)).toBeGreaterThanOrEqual(44);
    await page.screenshot({path:`test-results/resume-recovery-phone-${theme}.png`});
  }
  const dialog=await openNowPlaying(page);
  await expect(dialog.getByRole('status')).toContainText('Press Play to retry from your saved position');
  const retry=dialog.getByRole('button',{name:'Retry playback',exact:true});
  await retry.scrollIntoViewIfNeeded();await expect(retry).toBeInViewport();
  await page.unroute('**/api/tracks/*/stream');
  await retry.click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThanOrEqual(6);
  await dialog.getByRole('button',{name:'Close now playing',exact:true}).click();
  expect(errors).toEqual([]);
});

test('RES03 stalled metadata recovers and an obsolete transfer cannot replace the resumed occurrence',async({page})=>{
  const errors=[];page.on('pageerror',e=>errors.push(e.message));await pausedBookmark(page);
  let release;const gate=new Promise(resolve=>release=resolve);let seen;const held=new Promise(resolve=>seen=resolve);
  let gets=0,heads=0;
  await page.route('**/api/tracks/*/stream',async route=>{
    if(route.request().method()==='HEAD'){heads++;await route.continue();}
    else if(gets++===0){const response=await route.fetch();seen();await gate;try{await route.fulfill({response});}catch{ /* cancelled old source */ }}
    else{await route.continue();}
  });
  await page.getByRole('button',{name:'Play',exact:true}).click();await held;
  await expect.poll(()=>heads,{timeout:15000}).toBe(1);
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThanOrEqual(9);
  const generation=await page.locator('#vela-audio').getAttribute('data-source-generation');release();
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  expect(await page.locator('#vela-audio').getAttribute('data-source-generation')).toBe(generation);
  expect(await page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThanOrEqual(9);
  expect(errors).toEqual([]);
});

test('RES04 playlist row plays its correct duplicate occurrence in the full authorized context',async({page})=>{
  await openAdminFixture(page);const account=await (await page.request.get('/api/me')).json();
  const response=await page.request.post('/api/playlists',{headers:{'X-CSRF-Token':account.csrf_token},data:{name:'Resume route'}});
  expect(response.ok()).toBeTruthy();const playlist=await response.json();
  for(const title of ['Drift','Drift','Harbor']){
    const found=await (await page.request.get(`/api/search?q=${title}`)).json();
    const detail=await (await page.request.get(`/api/playlists/${playlist.id}`)).json();
    const added=await page.request.post(`/api/playlists/${playlist.id}/items`,{headers:{'X-CSRF-Token':account.csrf_token},data:{track_id:found.tracks.find(t=>t.title===title).id,revision:detail.playlist.revision,mutation_id:randomUUID()}});expect(added.ok()).toBeTruthy();
  }
  await navigate(page,'Playlists');await page.locator('.playlist-card').filter({hasText:'Resume route'}).click();
  await page.locator('.playlist-track').nth(1).getByRole('button',{name:'Play Drift',exact:true}).click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  await page.getByRole('button',{name:'Show playback queue',exact:true}).click();
  await expect(page.locator('.queue-row .queue-track strong')).toHaveText(['Drift','Drift','Harbor']);
  await expect(page.locator('.queue-row').nth(1)).toHaveClass(/current/);
  await expect(page.locator('.queue-context')).toHaveText('Playing from Resume route');
  await page.reload();await expect(page.locator('.now-playing strong')).toHaveText('Drift');
  await page.getByRole('button',{name:'Show playback queue',exact:true}).click();
  await expect(page.locator('.queue-row').nth(1)).toHaveClass(/current/);
  await expect(page.locator('.queue-context')).toHaveText('Playing from Resume route');
  await page.getByRole('complementary',{name:'Playback queue',exact:true}).getByRole('button',{name:'Next in queue',exact:true}).click();
  await expect(page.locator('.now-playing strong')).toHaveText('Harbor');
});

async function openNowPlaying(page){
  await page.getByRole('button',{name:'Open now playing',exact:true}).click();
  const dialog=page.getByRole('dialog',{name:'Now playing',exact:true});
  await expect(dialog).toBeVisible();await expect.poll(()=>dialog.evaluate(d=>d.matches(':modal'))).toBeTruthy();return dialog;
}

test('NP01 full player shares genuine audio transport queue and native modal focus',async({page})=>{
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(()=>{
    if('mediaSession' in navigator){
      const original=navigator.mediaSession.setActionHandler.bind(navigator.mediaSession);
      window.__velaTestMediaHandlers={};
      navigator.mediaSession.setActionHandler=(action,handler)=>{window.__velaTestMediaHandlers[action]=handler;return original(action,handler);};
    }
  });
  await openAdminFixture(page);await findTrack(page,'Harbor');
  await page.getByRole('button',{name:'Add Harbor to playback queue',exact:true}).click();
  await expect(page.locator('.player-bar input[aria-label="Playback position"]')).toHaveAttribute('max','30');
  const inspected=await openNowPlaying(page);
  await expect(inspected.locator('.np-times')).toContainText('0:30');
  await expect(inspected.getByRole('slider',{name:'Playback position',exact:true})).toHaveAttribute('max','30');
  expect(await page.locator('#vela-audio').getAttribute('src')).toBeNull();
  await inspected.getByRole('slider',{name:'Playback position',exact:true}).evaluate(e=>{e.value='11';e.dispatchEvent(new Event('input',{bubbles:true}));});
  await expect(page.locator('.player-bar input[aria-label="Playback position"]')).toHaveValue('11');
  expect(await page.locator('#vela-audio').getAttribute('src')).toBeNull();
  await inspected.getByRole('button',{name:'Close now playing',exact:true}).click();
  await page.getByRole('button',{name:'Play',exact:true}).click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThanOrEqual(11);
  expect(await page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeLessThan(14);
  await findTrack(page,'Drift');
  await page.getByRole('button',{name:'Play Drift',exact:true}).click();
  await page.getByRole('button',{name:'Add Drift to playback queue',exact:true}).click();
  await findTrack(page,'Harbor');await page.getByRole('button',{name:'Add Harbor to playback queue',exact:true}).click();
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.readyState)).toBe(4);
  await page.evaluate(()=>{window.__npAudio=document.querySelector('#vela-audio');window.__npSrc=window.__npAudio.src;window.__npGen=window.__npAudio.dataset.sourceGeneration;});
  const before=await page.locator('#vela-audio').evaluate(a=>a.currentTime);
  let streams=0;page.on('request',r=>{if(r.url().endsWith('/stream'))streams++;});
  const opener=page.getByRole('button',{name:'Open now playing',exact:true});await opener.focus();await page.keyboard.press('Enter');
  const dialog=page.getByRole('dialog',{name:'Now playing',exact:true});await expect(dialog).toBeVisible();
  expect(await dialog.evaluate(d=>d.matches(':modal'))).toBeTruthy();
  await expect(dialog.getByRole('heading',{name:'Drift',exact:true})).toBeVisible();
  await expect(dialog.locator('.np-context')).toHaveText('Playing from Search results');
  expect(await page.evaluate(()=>document.querySelectorAll('audio').length===1&&window.__npAudio===document.querySelector('#vela-audio')&&window.__npAudio.src===window.__npSrc&&window.__npAudio.dataset.sourceGeneration===window.__npGen)).toBeTruthy();
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThan(before);
  expect(streams).toBe(0);
  await page.locator('button[data-destination="library"]').first().evaluate(button=>button.focus());
  expect(await dialog.evaluate(d=>d.contains(document.activeElement))).toBeTruthy();
  await dialog.getByRole('button',{name:'Close now playing',exact:true}).focus();await page.keyboard.press('Shift+Tab');
  expect(await dialog.evaluate(d=>d.contains(document.activeElement))).toBeTruthy();await page.keyboard.press('Tab');
  expect(await dialog.evaluate(d=>d.contains(document.activeElement))).toBeTruthy();
  await dialog.getByRole('button',{name:'Pause',exact:true}).click();await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',true);
  await expect(page.locator('#player-toggle')).toHaveAttribute('aria-label','Play');
  await dialog.getByRole('slider',{name:'Playback position',exact:true}).evaluate(e=>{e.value='8';e.dispatchEvent(new Event('input',{bubbles:true}));});
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThanOrEqual(8);
  await dialog.getByRole('button',{name:'Previous track',exact:true}).click();
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeLessThan(1);await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',true);
  await dialog.getByRole('button',{name:'Play',exact:true}).click();await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  await dialog.getByRole('slider',{name:'Volume',exact:true}).evaluate(e=>{e.value='0.21';e.dispatchEvent(new Event('input',{bubbles:true}));});
  await expect(page.locator('#vela-audio')).toHaveJSProperty('volume',0.21);
  await expect(page.locator('.player-right input[aria-label="Volume"]')).toHaveValue('0.21');
  await page.evaluate(()=>window.__npDialog=document.querySelector('#now-playing-dialog'));
  // Duplicate occurrences use the same URL, but Next must update the root cursor
  // without replacing the dialog or audio element, even through native inertness.
  await page.evaluate(()=>window.__velaTestMediaHandlers.nexttrack());
  await expect(dialog.getByText('Track 2 of 3',{exact:true})).toBeVisible();
  expect(await page.evaluate(()=>window.__npDialog===document.querySelector('#now-playing-dialog')&&window.__npAudio===document.querySelector('#vela-audio')&&window.__npAudio.dataset.sourceGeneration!==window.__npGen)).toBeTruthy();
  await dialog.getByRole('button',{name:'Shuffle',exact:true}).click();await expect(dialog.getByRole('button',{name:'Shuffle',exact:true})).toHaveAttribute('aria-pressed','true');
  await dialog.getByRole('button',{name:'Repeat: off',exact:true}).click();await expect(dialog.getByRole('button',{name:'Repeat: all',exact:true})).toHaveAttribute('aria-pressed','true');
  // Real Media Session bridge calls the mounted bottom controls through native inertness.
  await page.evaluate(()=>window.__velaTestMediaHandlers.pause());await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',true);
  await page.evaluate(()=>window.__velaTestMediaHandlers.play());await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  await dialog.getByRole('button',{name:'Play Harbor from queue',exact:true}).click();
  await expect(dialog.getByRole('heading',{name:'Harbor',exact:true})).toBeVisible();
  expect(await page.evaluate(()=>window.__npDialog===document.querySelector('#now-playing-dialog')&&window.__npAudio===document.querySelector('#vela-audio'))).toBeTruthy();
  await expect(dialog.locator('.np-details')).toContainText('44100 Hz');
  const favorite=dialog.getByRole('button',{name:/current track.*favorites|Favorite current track/});
  const wasLiked=await favorite.getAttribute('aria-pressed');await favorite.click();await expect(favorite).toHaveAttribute('aria-pressed',String(wasLiked!=='true'));
  await expect(favorite).toBeEnabled();
  await dialog.getByRole('button',{name:'View full queue',exact:true}).click();await expect(dialog).toHaveCount(0);
  await expect(page.getByRole('complementary',{name:'Playback queue',exact:true})).toBeVisible();await expect(page.locator('.queue-row').nth(2)).toHaveClass(/current/);
  await page.getByRole('button',{name:'Close playback queue',exact:true}).click();await openNowPlaying(page);
  await page.keyboard.press('Escape');await expect(dialog).toHaveCount(0);await expect(opener).toBeFocused();
  const source=await page.locator('#vela-audio').getAttribute('src');await navigate(page,'Home');await navigate(page,'Library');
  await expect(page.locator('#vela-audio')).toHaveAttribute('src',source);expect(await page.evaluate(()=>window.__npAudio===document.querySelector('#vela-audio'))).toBeTruthy();
  await openNowPlaying(page);await dialog.evaluate(d=>d.close());await expect(dialog).toHaveCount(0);await expect(opener).toBeFocused();
  expect(errors).toEqual([]);
});

test('NP02 held genuine favorite replies cannot change a newer track or reopened player feedback',async({page})=>{
  const errors=[];page.on('pageerror',e=>errors.push(e.message));await openAdminFixture(page);await findTrack(page,'Drift');
  await page.getByRole('button',{name:'Play Drift',exact:true}).click();await findTrack(page,'Harbor');await page.getByRole('button',{name:'Add Harbor to playback queue',exact:true}).click();
  await page.getByRole('button',{name:'Pause',exact:true}).click();const dialog=await openNowPlaying(page);
  const waits=[];let calls=0;
  await page.route('**/api/tracks/*/favorite',async route=>{
    const at=calls++;let release;const gate=new Promise(resolve=>release=resolve);
    const headers={...route.request().headers()};if(at%2===0)headers['x-csrf-token']='invalid-fixture-token';
    const response=await route.fetch({headers});expect(response.ok()).toBe(at%2===1);
    waits[at]={release,url:route.request().url()};await gate;await route.fulfill({response});
  });
  const favorite=()=>dialog.getByRole('button',{name:/current track.*favorites|Favorite current track/});
  const pending=async at=>expect.poll(()=>Boolean(waits[at])).toBeTruthy();
  const finish=async at=>{const response=page.waitForResponse(r=>r.url()===waits[at].url&&r.request().method()==='POST');waits[at].release();await response;await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));};
  await favorite().click();await pending(0);await expect(favorite()).toBeDisabled();
  await dialog.getByRole('button',{name:'Next track',exact:true}).click();await expect(dialog.getByRole('heading',{name:'Harbor',exact:true})).toBeVisible();
  await favorite().click();await pending(1);await finish(0);await expect(favorite()).toBeDisabled();await expect(dialog.getByRole('alert')).toHaveCount(0);
  await finish(1);await expect(favorite()).toBeEnabled();
  await favorite().click();await pending(2);await dialog.getByRole('button',{name:'Close now playing',exact:true}).click();await openNowPlaying(page);
  await favorite().click();await pending(3);await finish(2);await expect(favorite()).toBeDisabled();await expect(dialog.getByRole('alert')).toHaveCount(0);
  await finish(3);await expect(favorite()).toBeEnabled();await expect(dialog.getByRole('alert')).toHaveCount(0);
  expect(errors).toEqual([]);
});

test('NP03 queued artwork metadata and favorites stay consistent across screens and responsive themes',async({page})=>{
  const errors=[];page.on('pageerror',e=>errors.push(e.message));await openAdminFixture(page);await findTrack(page,'Signal edited in Vela');
  const found=await (await page.request.get('/api/search?q=Signal%20edited%20in%20Vela')).json();const track=found.tracks.find(t=>t.title==='Signal edited in Vela');expect(track).toBeDefined();
  await page.getByRole('button',{name:`Add ${track.title} to playback queue`,exact:true}).click();
  expect(await page.locator('#vela-audio').getAttribute('src')).toBeNull();
  await navigate(page,'Albums');await page.locator('.catalog-card').filter({hasText:track.album}).first().click();
  const row=page.locator('.group-track-table .track-row').filter({hasText:track.title});
  const rowFavorite=row.locator('.favorite');const original=await rowFavorite.getAttribute('aria-pressed');await rowFavorite.click();await expect(rowFavorite).toHaveAttribute('aria-pressed',String(original!=='true'));
  let dialog=await openNowPlaying(page);const favorite=()=>dialog.getByRole('button',{name:/current track.*favorites|Favorite current track/});
  await expect(favorite()).toHaveAttribute('aria-pressed',String(original!=='true'));await favorite().click();await expect(favorite()).toHaveAttribute('aria-pressed',original);
  await dialog.getByRole('button',{name:'Close now playing',exact:true}).click();await expect(rowFavorite).toHaveAttribute('aria-pressed',original);
  await navigate(page,'Library');await page.locator('#library-search').fill(track.title);await page.locator('.search-form').evaluate(f=>f.requestSubmit());
  const libraryFavorite=page.locator('.track-row').filter({hasText:track.title}).locator('.favorite');await expect(libraryFavorite).toHaveAttribute('aria-pressed',original);
  await libraryFavorite.click();dialog=await openNowPlaying(page);await expect(favorite()).toHaveAttribute('aria-pressed',String(original!=='true'));await favorite().click();await expect(favorite()).toHaveAttribute('aria-pressed',original);await dialog.getByRole('button',{name:'Close now playing',exact:true}).click();
  await expect(libraryFavorite).toHaveAttribute('aria-pressed',original);
  await page.emulateMedia({reducedMotion:'reduce'});
  for(const width of [320,768,1440,1920]){
    await page.setViewportSize({width,height:900});
    for(const theme of ['dark','light']){
      if(await page.locator('html').getAttribute('data-theme')!==theme)await page.getByRole('button',{name:'Change color theme',exact:true}).click();
      dialog=await openNowPlaying(page);await expect(dialog.getByRole('heading',{name:track.title,exact:true})).toBeVisible();await expect(dialog.locator('.np-artist')).toHaveText(track.artist);
      await expect.poll(()=>dialog.locator('.np-artwork img').evaluate(i=>i.complete&&i.naturalWidth>0)).toBeTruthy();
      expect(await dialog.evaluate(d=>d.scrollWidth<=d.clientWidth+1)).toBeTruthy();
      await expect(dialog.getByRole('button',{name:'Play',exact:true})).toBeInViewport();
      if(width>=768)await expect(dialog.getByRole('slider',{name:'Volume',exact:true})).toBeInViewport();
      await page.screenshot({path:`test-results/now-playing-${width}-${theme}.png`});
      await dialog.getByRole('button',{name:'Close now playing',exact:true}).click();
    }
  }
  await page.setViewportSize({width:1440,height:720});dialog=await openNowPlaying(page);
  await expect(dialog.getByRole('button',{name:'Play',exact:true})).toBeInViewport();
  await expect(dialog.getByRole('slider',{name:'Volume',exact:true})).toBeInViewport();
  await expect(dialog.locator('.np-details').getByText(`${(track.size_bytes/1024).toFixed(1)} KiB`,{exact:true})).toBeVisible();
  await page.screenshot({path:'test-results/now-playing-desktop-short.png'});
  await dialog.getByRole('button',{name:'Close now playing',exact:true}).click();
  await page.setViewportSize({width:320,height:780});await page.evaluate(()=>document.documentElement.style.fontSize='24px');dialog=await openNowPlaying(page);
  expect(await dialog.evaluate(d=>d.scrollWidth<=d.clientWidth+1)).toBeTruthy();await dialog.getByRole('button',{name:'Play',exact:true}).scrollIntoViewIfNeeded();await expect(dialog.getByRole('button',{name:'Play',exact:true})).toBeInViewport();
  await page.screenshot({path:'test-results/now-playing-phone-large-text.png'});
  expect(await page.locator('#vela-audio').getAttribute('src')).toBeNull();expect(errors).toEqual([]);
});

test('G08 G09 actual background DSP creates explained private radio without replacing audio', async ({page}) => {
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await openAdminFixture(page);await findTrack(page,'Drift');
  await page.getByRole('button',{name:'Play Drift',exact:true}).click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  await page.locator('#vela-audio').evaluate(a=>{window.radioAudioNode=a;});
  const before=await page.locator('#vela-audio').evaluate(a=>({src:a.src,time:a.currentTime}));
  await page.getByLabel('Audio details for Drift',{exact:true}).click();
  await page.getByRole('button',{name:'Start track radio for Drift',exact:true}).click();
  await expect(page.getByRole('heading',{name:'Drift radio',exact:true})).toBeVisible();
  await expect(page.getByRole('button',{name:'Play radio',exact:true})).toBeVisible();
  expect(await page.locator('#vela-audio').evaluate(a=>a===window.radioAudioNode&&a.src)).toBe(before.src);
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThan(before.time);
  const summary=await (await page.request.get('/api/audio-analysis')).json();expect(summary.complete).toBe(0);
  await page.getByRole('button',{name:'Analyze seed',exact:true}).click();
  await expect(page.locator('.analysis-seed-status')).toHaveText('Seed: Analysis complete',{timeout:20000});
  await page.getByText('Measured audio details',{exact:true}).click();
  await expect(page.locator('.radio-measurements')).toContainText('44100 Hz');
  await expect(page.locator('.radio-measurements')).toContainText('vela-pcm-spectral-4');
  await expect(page.locator('.radio-measurements')).toContainText('not LUFS');
  await expect(page.locator('.radio-measurements')).toContainText('Estimated tempo');
  await expect(page.locator('.radio-measurements')).toContainText('Chroma pitch classes');
  await expect(page.locator('.radio-measurements')).toContainText('Estimated key');
  await page.getByRole('button',{name:'Analyze up to 50 library tracks',exact:true}).click();
  await expect.poll(async()=>{const s=await(await page.request.get('/api/audio-analysis')).json();return s.complete>=2;},{timeout:20000}).toBeTruthy();
  await expect(page.locator('.radio-reason').filter({hasText:'Similar measured spectrum and signal level'}).first()).toBeVisible({timeout:20000});
  const radio=await(await page.request.get('/api/search?q=Drift')).json();
  const derived=await(await page.request.get(`/api/tracks/${radio.tracks[0].id}/radio?limit=20`)).json();
  expect(derived.uses_audio_features).toBeTruthy();expect(derived.items.length).toBeGreaterThan(0);
  expect(derived.items.every(item=>item.track.id!==derived.seed.id&&item.reason.length>0)).toBeTruthy();
  await page.getByRole('button',{name:'Play radio',exact:true}).click();
  await page.getByRole('button',{name:'Show playback queue',exact:true}).click();
  await expect(page.locator('.queue-row')).toHaveCount(derived.items.length+1);
  await page.getByRole('button',{name:'Close playback queue',exact:true}).click();
  await page.getByRole('button',{name:'Pause',exact:true}).click();
  for(const [width,theme] of [320,768,1440,1920].flatMap(width=>['dark','light'].map(theme=>[width,theme]))) {
    await page.setViewportSize({width,height:900});
    await page.locator('html').evaluate((el,value)=>el.setAttribute('data-theme',value),theme);
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth)).toBeTruthy();
    const art=await page.locator('.radio-track-play .artwork').first().boundingBox();
    expect(Math.abs(art.width-art.height)).toBeLessThan(1);
    await page.screenshot({path:`test-results/radio-${width}-${theme}.png`,fullPage:true});
  }
  await navigate(page,'Home');await expect(page.getByRole('heading',{name:'Drift radio',exact:true})).toHaveCount(0);
  await page.setViewportSize({width:1440,height:900});
  await page.getByRole('button',{name:'Track radio',exact:true}).click();
  await expect(page.getByRole('heading',{name:'Drift radio',exact:true})).toBeVisible();
  expect(errors).toEqual([]);
});

test('G07 private history measures actual playback only after account and device consent, exports and erases', async ({page}) => {
  const errors=[]; page.on('pageerror',error=>errors.push(error.message));
  await openAdminFixture(page);
  const me=await (await page.request.get('/api/me')).json();
  const audio=pcmWave(8,660);
  const upload=await page.request.post('/api/upload',{headers:{'X-CSRF-Token':me.csrf_token},multipart:{file:{name:'History Tide.wav',mimeType:'audio/wav',buffer:audio}}});
  expect(upload.ok()).toBeTruthy();
  const uploaded=await upload.json();
  const exportHistory=async()=>await (await page.request.get('/api/listening/export')).json();
  await findTrack(page,'History Tide');
  await page.getByRole('button',{name:'Play History Tide',exact:true}).click();
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThan(1);
  await page.getByRole('button',{name:'Pause',exact:true}).click();
  expect((await exportHistory()).events).toEqual([]);
  await navigate(page,'Settings');
  const global=page.getByRole('checkbox',{name:'Enable private listening history',exact:true});
  const device=page.getByRole('checkbox',{name:'Collect listening history on this device',exact:true});
  await expect(global).not.toBeChecked();await expect(device).not.toBeChecked();
  await global.check();await expect(global).toBeChecked();await expect(device).toBeEnabled();
  expect((await exportHistory()).events).toEqual([]);
  await device.check();await expect(device).toBeChecked();await expect(global).toBeEnabled();
  const secondDevice=randomUUID();
  const currentPreferences=await (await page.request.get(`/api/listening/preferences?device_id=${secondDevice}`)).json();
  const secondGrant=await page.request.put('/api/listening/preferences',{headers:{'X-CSRF-Token':me.csrf_token},data:{revision:currentPreferences.revision,enabled:true,retention_days:90,device_id:secondDevice,device_enabled:true}});
  expect(secondGrant.ok()).toBeTruthy();
  await page.getByRole('button',{name:'Refresh privacy settings',exact:true}).click();
  await expect(page.locator('[data-listening-device]')).toHaveCount(2);
  await expect(page.locator(`[data-listening-device="${secondDevice}"]`)).toContainText('Listening device');
  await page.getByRole('button',{name:`Revoke listening device ${secondDevice}`,exact:true}).click();
  await page.getByRole('button',{name:`Confirm revoke listening device ${secondDevice}`,exact:true}).click();
  await expect(page.locator('[data-listening-device]')).toHaveCount(1);
  await expect(page.locator(`[data-listening-device="${secondDevice}"]`)).toHaveCount(0);
  await findTrack(page,'History Tide');
  await page.getByRole('button',{name:'Play History Tide',exact:true}).click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  await page.locator('#vela-audio').evaluate(a=>{a.muted=true;});
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThan(1.1);
  await page.locator('#vela-audio').evaluate(a=>{a.muted=false;});
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThan(6.8);
  await page.getByRole('button',{name:'Pause',exact:true}).click();
  await expect.poll(async()=> (await exportHistory()).events.filter(e=>e.progress.track_id===uploaded.track.id).length).toBe(1);
  await expect.poll(async()=> (await exportHistory()).events.find(e=>e.progress.track_id===uploaded.track.id)?.qualified).toBeTruthy();
  const first=(await exportHistory()).events[0];
  expect(first.progress.listened_ms).toBeGreaterThanOrEqual(4000);
  expect(first.progress.listened_ms).toBeLessThan(6800);
  expect(first.progress.listened_ms).toBeLessThan(first.progress.position_ms);
  // A paused resume keeps its occurrence; natural repeat-one starts a fresh one.
  await page.getByRole('button',{name:'Repeat: off',exact:true}).click();
  await page.getByRole('button',{name:'Repeat: all',exact:true}).click();
  await page.getByRole('slider',{name:'Playback position',exact:true}).fill('7.5');
  await page.getByRole('button',{name:'Play',exact:true}).click();
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>!a.paused&&a.currentTime<3)).toBeTruthy();
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThan(4.8);
  await page.getByRole('button',{name:'Pause',exact:true}).click();
  await expect.poll(async()=>(await exportHistory()).events.filter(e=>e.progress.track_id===uploaded.track.id&&e.qualified).length).toBe(2);
  const repeated=(await exportHistory()).events.filter(e=>e.progress.track_id===uploaded.track.id);
  expect(new Set(repeated.map(e=>e.progress.session_id)).size).toBe(2);
  expect(repeated.some(e=>e.progress.ended)).toBeTruthy();
  await page.getByRole('button',{name:'Repeat: one',exact:true}).click();
  await navigate(page,'Listening history');
  await expect(page.getByRole('button',{name:'Play History Tide from listening history',exact:true})).toHaveCount(2);
  await page.screenshot({path:'test-results/history-desktop.png',fullPage:true});
  await navigate(page,'Home');
  await expect(page.getByRole('heading',{name:'Recently played',exact:true})).toBeVisible();
  const rotation=page.getByRole('region',{name:'Your rotation',exact:true});
  await expect(rotation.getByRole('button',{name:'Play History Tide from rotation',exact:true})).toBeVisible();
  await expect(rotation).toContainText('2 qualified listens');
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',true);
  await page.getByRole('button',{name:'Explore radio from History Tide',exact:true}).first().click();
  await expect(page.getByRole('heading',{name:'History Tide radio',exact:true})).toBeVisible();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',true);
  await navigate(page,'Home');
  await expect(page.getByRole('button',{name:'Refresh personal mixes',exact:true})).toBeEnabled();
  let releaseMix;const mixGate=new Promise(resolve=>releaseMix=resolve);let mixHeld=false;
  await page.route('**/api/recommendations?*',async route=>{
    const response=await route.fetch({timeout:3000});expect(response.ok()).toBeTruthy();
    expect((await response.json()).rotation.some(item=>item.track.id===uploaded.track.id)).toBeTruthy();
    mixHeld=true;await mixGate;try{await route.fulfill({response});}catch{/* Erasure workflow invalidates the abandoned read. */}
  });
  await page.getByRole('button',{name:'Refresh personal mixes',exact:true}).click();
  await expect.poll(()=>mixHeld,{timeout:3000}).toBeTruthy();
  await navigate(page,'Settings');
  const downloadPromise=page.waitForEvent('download');
  await page.getByRole('button',{name:'Export private history',exact:true}).click();
  const download=await downloadPromise;
  expect(download.suggestedFilename()).toBe('vela-private-listening-history.json');
  const stream=await download.createReadStream();const chunks=[];for await(const chunk of stream) chunks.push(chunk);
  const exported=JSON.parse(Buffer.concat(chunks).toString());
  expect(exported.events.some(e=>e.progress.session_id===first.progress.session_id)).toBeTruthy();
  await page.setViewportSize({width:320,height:740});
  await page.locator('.listening-privacy').screenshot({path:'test-results/history-privacy-phone.png'});
  // Firefox reports a 3px scrollWidth rounding residue here; no element exceeds
  // the viewport and the320px screenshot is unclipped.
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth+3)).toBeTruthy();
  let releaseExport;const exportGate=new Promise(resolve=>releaseExport=resolve);
  let exportSeen;const exportHeld=new Promise(resolve=>exportSeen=resolve);
  await page.route('**/api/listening/export',async route=>{const response=await route.fetch();exportSeen();await exportGate;try{await route.fulfill({response});}catch{/* Request was physically cancelled by erasure. */}});
  const staleDownloads=[];page.on('download',download=>staleDownloads.push(download.suggestedFilename()));
  await page.getByRole('button',{name:'Export private history',exact:true}).click();await exportHeld;
  await page.getByRole('button',{name:'Delete history…',exact:true}).click();
  await page.getByRole('button',{name:'Delete history and turn off collection',exact:true}).click();
  await expect(global).not.toBeChecked();await expect(device).not.toBeChecked();
  releaseExport();await expect(page.getByRole('button',{name:'Export private history',exact:true})).toBeEnabled();
  await page.unroute('**/api/listening/export');
  await expect.poll(async()=>(await exportHistory()).events.length).toBe(0);
  expect(staleDownloads).toEqual([]);
  releaseMix();await page.unroute('**/api/recommendations?*');
  await navigate(page,'Home');
  await expect(page.getByRole('region',{name:'Your rotation',exact:true})).toContainText('Your listening history is off');
  await expect(page.getByRole('button',{name:'Play History Tide from rotation',exact:true})).toHaveCount(0);
  await findTrack(page,'History Tide');await page.getByRole('button',{name:'Play History Tide',exact:true}).click();
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>a.currentTime)).toBeGreaterThan(1);
  await page.getByRole('button',{name:'Pause',exact:true}).click();
  expect((await exportHistory()).events).toEqual([]);
  expect(errors).toEqual([]);
});

test('PC01 personal mixes play real owned music and hidden choices support undo restore and export',async({page})=>{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await openAdminFixture(page);
  const me=await(await page.request.get('/api/me')).json();
  const tracks=(await(await page.request.get('/api/tracks?limit=100')).json()).items;
  const seed=tracks.find(track=>track.duration_ms>=20000)||tracks[0];
  expect(tracks.length).toBeGreaterThan(2);
  const favorite=await page.request.post(`/api/tracks/${seed.id}/favorite`,{headers:{'X-CSRF-Token':me.csrf_token},data:{favorite:true}});
  expect(favorite.ok()).toBeTruthy();
  await page.locator('#vela-audio').evaluate(a=>{window.personalAudioNode=a;});
  await navigate(page,'Home');
  const region=page.getByRole('region',{name:'Personal recommendations',exact:true});
  const explore=region.getByRole('region',{name:'Explore your library',exact:true});
  await expect(explore.locator('.personal-mix-card').first()).toBeVisible();
  const mix=await(await page.request.get('/api/recommendations?limit=8')).json();
  expect(mix.history_enabled).toBe(false);expect(mix.rotation).toEqual([]);expect(mix.anchor_count).toBeGreaterThan(0);
  expect(mix.discovery.length).toBeGreaterThan(0);
  const first=mix.discovery[0];
  expect(tracks.some(track=>track.id===first.track.id)).toBeTruthy();
  await expect(explore.locator(`[data-track-id="${first.track.id}"] .personal-mix-reason`)).toHaveText(first.reason);
  await region.getByRole('button',{name:'Play exploration mix',exact:true}).click();
  await expect.poll(()=>page.locator('#vela-audio').evaluate(a=>!a.paused&&a.currentTime>0)).toBeTruthy();
  expect(await page.locator('#vela-audio').evaluate(a=>a===window.personalAudioNode)).toBeTruthy();
  await expect(page.locator('#vela-audio')).toHaveAttribute('src',new RegExp(`/api/tracks/${first.track.id}/stream`));
  await page.getByRole('button',{name:'Pause',exact:true}).click();
  const source=await page.locator('#vela-audio').getAttribute('src');
  let releaseHide;const hideGate=new Promise(resolve=>releaseHide=resolve);
  let hideHeld=false,hideResponded=false;
  const hideHandler=async route=>{
    if(route.request().method()!=='PUT'){await route.continue();return;}
    const response=await route.fetch({timeout:3000});expect(response.ok()).toBeTruthy();hideHeld=true;
    await hideGate;await route.fulfill({response});hideResponded=true;
  };
  await page.route('**/api/recommendations/feedback',hideHandler);
  // Finish well inside the product's 10s mutation deadline, even when a locator fails.
  const raceDeadline=Date.now()+7000;const raceTimeout=()=>Math.max(1,raceDeadline-Date.now());
  try{
    await explore.getByRole('button',{name:`Hide ${first.track.title} from personal mixes`,exact:true}).click({timeout:raceTimeout()});
    await expect.poll(()=>hideHeld,{timeout:raceTimeout()}).toBeTruthy();
    // The root current-track favorite lives in Now Playing, including while Home refreshes.
    await page.getByRole('button',{name:'Open now playing',exact:true}).click({timeout:raceTimeout()});
    const dialog=page.getByRole('dialog',{name:'Now playing',exact:true});
    const currentFavorite=dialog.getByRole('button',{name:/current track.*favorites|Favorite current track/});
    await expect(currentFavorite).toBeEnabled({timeout:raceTimeout()});
    const original=await currentFavorite.getAttribute('aria-pressed');expect(['true','false']).toContain(original);
    await currentFavorite.click({timeout:raceTimeout()});
    await expect(currentFavorite).toHaveAttribute('aria-pressed',String(original!=='true'),{timeout:raceTimeout()});
    await expect(currentFavorite).toBeEnabled({timeout:raceTimeout()});
    await currentFavorite.click({timeout:raceTimeout()});
    await expect(currentFavorite).toHaveAttribute('aria-pressed',original,{timeout:raceTimeout()});
    await expect(currentFavorite).toBeEnabled({timeout:raceTimeout()});
    await dialog.getByRole('button',{name:'Close now playing',exact:true}).click({timeout:raceTimeout()});
    expect(Date.now()).toBeLessThan(raceDeadline);
  }finally{
    releaseHide();
    await expect.poll(()=>hideResponded,{timeout:3000}).toBeTruthy();
    await page.unroute('**/api/recommendations/feedback',hideHandler);
  }
  await expect(region.getByRole('button',{name:'Undo last recommendation hide',exact:true})).toBeEnabled();
  await expect(explore.locator(`[data-track-id="${first.track.id}"]`)).toHaveCount(0);
  let exclusions=await(await page.request.get('/api/recommendations/feedback')).json();
  expect(exclusions.track_ids).toContain(first.track.id);
  expect(await page.locator('#vela-audio').getAttribute('src')).toBe(source);
  await region.getByRole('button',{name:'Undo last recommendation hide',exact:true}).click();
  await expect(explore.locator(`[data-track-id="${first.track.id}"]`)).toBeVisible();
  await explore.getByRole('button',{name:`Hide ${first.track.title} from personal mixes`,exact:true}).click();
  await expect(region.getByRole('button',{name:'Restore hidden recommendations…',exact:true})).toBeEnabled();
  const downloadPromise=page.waitForEvent('download');
  await region.getByRole('button',{name:'Export hidden recommendations',exact:true}).click();
  const download=await downloadPromise;const stream=await download.createReadStream();const chunks=[];for await(const chunk of stream)chunks.push(chunk);
  expect(JSON.parse(Buffer.concat(chunks).toString()).track_ids).toContain(first.track.id);
  await region.getByRole('button',{name:'Restore hidden recommendations…',exact:true}).click();
  await region.getByRole('button',{name:'Keep hidden tracks',exact:true}).click();
  expect((await(await page.request.get('/api/recommendations/feedback')).json()).track_ids).toContain(first.track.id);
  await region.getByRole('button',{name:'Restore hidden recommendations…',exact:true}).click();
  await region.getByRole('button',{name:'Restore all hidden recommendations',exact:true}).click();
  await expect(explore.locator(`[data-track-id="${first.track.id}"]`)).toBeVisible();
  expect((await(await page.request.get('/api/recommendations/feedback')).json()).track_ids).toEqual([]);
  for(const [width,theme] of [320,768,1440,1920].flatMap(width=>['dark','light'].map(theme=>[width,theme]))) {
    await page.setViewportSize({width,height:900});
    await page.locator('html').evaluate((el,value)=>el.setAttribute('data-theme',value),theme);
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth)).toBeTruthy();
    const art=await explore.locator('.artwork').first().boundingBox();
    expect(Math.abs(art.width-art.height)).toBeLessThan(1);
    await explore.scrollIntoViewIfNeeded();
    await page.screenshot({path:`test-results/personal-${width}-${theme}.png`,animations:'disabled'});
  }
  expect(errors).toEqual([]);
});

test('PC02 held genuine personal recommendations cannot return after navigation or logout',async({page})=>{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  // Signing out must revoke only this scenario's session, not the shared fixture cookie.
  const session=await page.request.post('/api/login',{data:{username:'admin',password}});
  expect(session.ok()).toBeTruthy();
  await page.goto('/');
  await expect(page.getByRole('button',{name:'Library',exact:true}).first()).toBeVisible();
  await navigate(page,'Library');
  const leaveWithHeldRecommendations=async leave=>{
    let release;const gate=new Promise(resolve=>release=resolve);
    let held=false,active=0;
    const handler=async route=>{
      active++;
      try{
        const response=await route.fetch({timeout:3000});expect(response.ok()).toBeTruthy();held=true;
        await gate;
        try{await route.fulfill({response});}catch{/* Navigation/session teardown physically aborts obsolete reads. */}
      }finally{active--;}
    };
    await page.route('**/api/recommendations?*',handler);
    try{
      await navigate(page,'Home');
      await expect.poll(()=>held,{timeout:3000}).toBeTruthy();
      await leave();
      release();
      // Observe every held callback finish before checking the destination for stale content.
      await expect.poll(()=>active,{timeout:3000}).toBe(0);
    }finally{
      release();await page.unroute('**/api/recommendations?*',handler);
    }
  };
  await leaveWithHeldRecommendations(()=>navigate(page,'Search'));
  await expect(page.getByRole('region',{name:'Personal recommendations',exact:true})).toHaveCount(0);
  await expect(page.locator('#global-search')).toBeVisible();
  await leaveWithHeldRecommendations(async()=>{
    await navigate(page,'Settings');
    await page.getByRole('button',{name:'Sign out',exact:true}).click();
    await expect(page.locator('#username')).toBeVisible();
  });
  await expect(page.locator('#username')).toBeVisible();
  await expect(page.getByRole('region',{name:'Personal recommendations',exact:true})).toHaveCount(0);
  expect(errors).toEqual([]);
});

// Device app size (EX261 / SCALE01–SCALE02, SCALE07–SCALE09 subset). The preference
// is presentation-only, device-scoped and validated: malformed stores never reach
// the renderer, and no scale change may remount the one real media element.
test('SCALE01 device app size applies live, persists, recovers from bad stores and keeps playback identity', async ({ page }) => {
  test.setTimeout(180000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const deviceKey = 'vela.device.presentation.v1';
  const zoom = async () => Number(await page.evaluate(() => getComputedStyle(document.documentElement).zoom));
  const setSlider = async value => {
    await page.getByRole('slider', { name: 'App size', exact: true })
      .evaluate((input, next) => { input.value = String(next); input.dispatchEvent(new Event('input', { bubbles: true })); input.dispatchEvent(new Event('change', { bubbles: true })); }, value);
  };
  // A reload re-mounts the SPA, so every navigation waits for the signed-in shell.
  const reloadApp = async () => {
    await page.reload();
    await expect(page.locator('main.main-content')).toBeVisible();
  };
  // Full-suite runs reuse the suite's own session; a targeted `-g` run starts from
  // the isolated server.sh fixture and therefore creates its own account and track.
  let title = 'Drift';
  if (adminCookies) {
    await openAdminFixture(page);
  } else {
    await page.goto('/');
    await page.locator('#username').fill('admin');
    await page.locator('#password').fill(password);
    await page.getByRole('button', { name: /Create your music space|Sign in →/ }).click();
    await expect(page.getByRole('button', { name: 'Library', exact: true }).first()).toBeVisible();
    const auth = await (await page.request.get('/api/me')).json();
    const upload = await page.request.post('/api/upload', { headers: { 'X-CSRF-Token': auth.csrf_token }, multipart: { file: { name: 'Appearance fixture.wav', mimeType: 'audio/wav', buffer: pcmWave(20, 440) } } });
    expect(upload.ok()).toBeTruthy();
    title = (await upload.json()).track.title;
  }
  await findTrack(page, title);
  await page.getByRole('button', { name: `Play ${title}`, exact: true }).click();
  await page.getByRole('button', { name: `Add ${title} to playback queue`, exact: true }).click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
  const source = await page.locator('#vela-audio').getAttribute('src');
  await page.evaluate(() => { window.__scaleAudio = document.getElementById('vela-audio'); });
  const assertSamePlayback = async () => {
    expect(await page.evaluate(() => document.getElementById('vela-audio') === window.__scaleAudio)).toBeTruthy();
    await expect(page.locator('#vela-audio')).toHaveAttribute('src', source);
    await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', false);
  };
  await navigate(page, 'Settings');
  const appearance = page.locator('[data-testid="settings-appearance"]');
  await expect(appearance).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Appearance', exact: true })).toBeVisible();
  await expect(appearance).toContainText('Changes this device only');
  await expect(appearance).toContainText('stored in this browser only');
  await expect(page.locator('[data-testid="app-scale-value"]')).toHaveText('100%');
  expect(await zoom()).toBeCloseTo(1, 3);
  // Keyboard-only: reach the slider, change the value with an arrow key, keep focus.
  const slider = page.getByRole('slider', { name: 'App size', exact: true });
  await slider.focus();
  await page.keyboard.press('ArrowRight');
  await expect(page.locator('[data-testid="app-scale-value"]')).toHaveText('105%');
  expect(await zoom()).toBeCloseTo(1.05, 3);
  expect(await slider.evaluate(input => input === document.activeElement)).toBeTruthy();
  expect(await slider.evaluate(input => getComputedStyle(input).outlineStyle)).not.toBe('none');
  await expect(page.locator('[data-testid="appearance-save-state"]')).toHaveText('Saved on this device.');
  // Stepper and drag both apply exactly once and never stack.
  await page.getByRole('button', { name: 'Decrease app size', exact: true }).click();
  await expect(page.locator('[data-testid="app-scale-value"]')).toHaveText('100%');
  await setSlider(125);
  await expect(page.locator('[data-testid="app-scale-value"]')).toHaveText('125%');
  expect(await zoom()).toBeCloseTo(1.25, 3);
  expect(await page.evaluate(() => getComputedStyle(document.documentElement).zoom)).not.toMatch(/2\./);
  await assertSamePlayback();
  // Queue, cursor and position survive the scale change.
  await page.getByRole('button', { name: 'Show playback queue', exact: true }).click();
  await expect(page.locator('.queue-row')).toHaveCount(2);
  await expect(page.locator('.queue-row').first()).toHaveClass(/current/);
  await expect(page.locator('.queue-row .queue-track strong').first()).toHaveText(title);
  await page.getByRole('button', { name: 'Close playback queue', exact: true }).click();
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
  await assertSamePlayback();
  // The versioned device document holds presentation fields only.
  const document = await page.evaluate(key => localStorage.getItem(key), deviceKey);
  expect(JSON.parse(document)).toEqual({ app_scale_percent: 125, theme: 'dark', version: 1 });
  const account = await (await page.request.get('/api/me')).json();
  expect(document).not.toContain(account.csrf_token);
  expect(document.toLowerCase()).not.toMatch(/csrf|token|cookie|password/);
  expect(await page.evaluate(() => localStorage.getItem('vela.theme'))).toBeNull();
  // Responsive evidence at 1280 and 320, both themes, reduced motion honoured.
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.screenshot({ path: 'test-results/scale-settings-1280-dark-125.png' });
  await page.getByRole('radio', { name: 'Light', exact: true }).check();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.screenshot({ path: 'test-results/scale-settings-1280-light-125.png' });
  await page.getByRole('radio', { name: 'System', exact: true }).check();
  expect(await page.locator('html').getAttribute('data-theme')).toBe(await page.evaluate(() => matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'));
  await page.getByRole('radio', { name: 'Dark', exact: true }).check();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  expect(JSON.parse(await page.evaluate(key => localStorage.getItem(key), deviceKey)).theme).toBe('dark');
  await page.setViewportSize({ width: 320, height: 780 });
  await setSlider(150);
  await expect(page.locator('[data-testid="app-scale-value"]')).toHaveText('150%');
  expect(await zoom()).toBeCloseTo(1.5, 3);
  await assertSamePlayback();
  // High scale must not strand the reset action outside the visible width.
  const reset = page.getByRole('button', { name: 'Reset to 100%', exact: true });
  await reset.scrollIntoViewIfNeeded();
  const box = await reset.boundingBox();
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual((await page.evaluate(() => innerWidth)) + 1);
  await page.screenshot({ path: 'test-results/scale-settings-320-dark-150.png' });
  // Reload: the saved size is applied before the app's own private requests.
  await page.addInitScript(() => {
    window.__scaleOrder = { zoomAt: null, firstApiAt: null };
    const setAttribute = Element.prototype.setAttribute;
    Element.prototype.setAttribute = function (name, value) {
      if (name === 'style' && String(value).includes('--app-zoom') && window.__scaleOrder.zoomAt === null) window.__scaleOrder.zoomAt = performance.now();
      return setAttribute.call(this, name, value);
    };
    const fetch = window.fetch;
    window.fetch = function (input, init) {
      const url = typeof input === 'string' ? input : input && input.url;
      if (window.__scaleOrder.firstApiAt === null && String(url).includes('/api/')) window.__scaleOrder.firstApiAt = performance.now();
      return fetch.call(this, input, init);
    };
  });
  let streams = 0;
  page.on('request', request => { if (/\/stream$/.test(request.url())) streams++; });
  await reloadApp();
  await expect(page.locator('[data-testid="app-scale-value"]')).toHaveText('150%');
  expect(await zoom()).toBeCloseTo(1.5, 3);
  const order = await page.evaluate(() => window.__scaleOrder);
  expect(order.zoomAt).not.toBeNull();
  expect(order.firstApiAt === null || order.zoomAt <= order.firstApiAt).toBeTruthy();
  expect(streams).toBe(0);
  await expect(page.locator('.now-playing strong')).toHaveText(title);
  await navigate(page, 'Settings');
  // Reset and its own persistence.
  await page.getByRole('button', { name: 'Reset to 100%', exact: true }).click();
  await expect(page.locator('[data-testid="app-scale-value"]')).toHaveText('100%');
  expect(await zoom()).toBeCloseTo(1, 3);
  expect(JSON.parse(await page.evaluate(key => localStorage.getItem(key), deviceKey)).app_scale_percent).toBe(100);
  await reloadApp();
  await expect(page.locator('[data-testid="app-scale-value"]')).toHaveText('100%');
  expect(await zoom()).toBeCloseTo(1, 3);
  // A malformed store never reaches the renderer and is explained, not deleted.
  await page.evaluate(key => localStorage.setItem(key, '{not json'), deviceKey);
  await reloadApp();
  await navigate(page, 'Settings');
  await expect(page.locator('[data-testid="app-scale-value"]')).toHaveText('100%');
  expect(await zoom()).toBeCloseTo(1, 3);
  await expect(page.locator('[data-testid="appearance-recovery"]')).toBeVisible();
  expect(await page.evaluate(key => localStorage.getItem(key), deviceKey)).toBe('{not json');
  // Out-of-range and out-of-step values are rejected before the renderer.
  for (const bad of ['{"version":1,"app_scale_percent":200,"theme":"dark"}', '{"version":1,"app_scale_percent":83,"theme":"dark"}', '{"version":9,"app_scale_percent":125,"theme":"dark"}']) {
    await page.evaluate(([key, value]) => localStorage.setItem(key, value), [deviceKey, bad]);
    await reloadApp();
    await navigate(page, 'Settings');
    await expect(page.locator('[data-testid="app-scale-value"]')).toHaveText('100%');
    expect(await zoom()).toBeCloseTo(1, 3);
    await expect(page.locator('[data-testid="appearance-recovery"]')).toBeVisible();
  }
  // Storage denied: the value applies for this session and says so; the next start
  // recovers the last durable value instead of claiming it was saved.
  await page.evaluate(([key, value]) => localStorage.setItem(key, value), [deviceKey, '{"app_scale_percent":100,"theme":"dark","version":1}']);
  await reloadApp();
  await navigate(page, 'Settings');
  // Resume playback so a denied write is measured against a listening user. The
  // restored bookmark may sit at the fixture's end, so restart inside the file.
  await page.getByRole('slider', { name: 'Playback position', exact: true }).evaluate(input => { input.value = '1'; input.dispatchEvent(new Event('input', { bubbles: true })); });
  await page.locator('#player-toggle').click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', false);
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(0);
  const playingSource = await page.locator('#vela-audio').getAttribute('src');
  await page.evaluate(() => {
    window.__restoreSetItem = Storage.prototype.setItem;
    Storage.prototype.setItem = function () { throw new DOMException('denied', 'QuotaExceededError'); };
  });
  await page.getByRole('button', { name: 'Increase app size', exact: true }).click();
  await expect(page.locator('[data-testid="app-scale-value"]')).toHaveText('105%');
  expect(await zoom()).toBeCloseTo(1.05, 3);
  await expect(page.locator('[data-testid="appearance-save-state"]')).toHaveText(/Applied for this session/);
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused', false);
  await expect(page.locator('#vela-audio')).toHaveAttribute('src', playingSource);
  const before = await page.locator('#vela-audio').evaluate(audio => audio.currentTime);
  await expect.poll(() => page.locator('#vela-audio').evaluate(audio => audio.currentTime)).toBeGreaterThan(before);
  // The denied value was never durable: the next start recovers the saved 100%.
  await reloadApp();
  await expect(page.locator('[data-testid="app-scale-value"]')).toHaveText('100%');
  expect(await zoom()).toBeCloseTo(1, 3);
  expect(errors).toEqual([]);
});

test('AUTH01 real revoked session stops the source and clears private playback UI',async({page})=>{
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  // A dedicated real session prevents revoking another scenario's fixture cookie.
  const response=await page.request.post('/api/login',{data:{username:'admin',password}});
  expect(response.ok()).toBeTruthy();const account=await response.json();
  await page.goto('/');await navigate(page,'Search');
  await findTrack(page,'Drift');await page.getByRole('button',{name:'Play Drift',exact:true}).click();
  await expect(page.locator('#vela-audio')).toHaveJSProperty('paused',false);
  await findTrack(page,'Harbor');
  await page.getByRole('button',{name:'Add Harbor to playback queue',exact:true}).click();
  const revoked=await page.request.post('/api/logout',{headers:{'X-CSRF-Token':account.csrf_token}});expect(revoked.ok()).toBeTruthy();
  const dialog=await openNowPlaying(page);
  await dialog.getByRole('button',{name:'Play Harbor from queue',exact:true}).click();
  await expect(page.locator('#username')).toBeVisible();
  await expect(page.getByRole('alert').filter({hasText:'Your session expired'})).toBeVisible();
  await expect(page.locator('#vela-audio')).toHaveCount(0);
  await expect(dialog).toHaveCount(0);
  expect(errors).toEqual([]);
});

test('PN01 explicit publication separates private playback, metadata, streaming and revocation', async ({page,browser}) => {
  test.setTimeout(90000);
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await openAdminFixture(page);
  // The suite owns this disposable loopback server. Never enable a real origin.
  expect(new URL(page.url()).origin).toBe('http://127.0.0.1:3217');
  const me=await(await page.request.get('/api/me')).json();
  const headers={'X-CSRF-Token':me.csrf_token};
  const original=await(await page.request.get('/api/network/settings')).json();
  expect(original.enabled).toBe(false);expect(original.origin).toBeNull();
  expect((await(await page.request.get('/api/publications?limit=20')).json()).items).toEqual([]);
  const library=(await(await page.request.get('/api/tracks?limit=100')).json()).items;
  // Long WAVs in this suite are generated PCM, never the optional licensed import.
  const track=library.find(item=>item.codec==='WAV audio'&&item.duration_ms>=20000);
  expect(track).toBeDefined();
  const privateBefore=JSON.stringify(track);
  await page.locator('#vela-audio').evaluate(audio=>{window.publicationAudioNode=audio;});
  const anonymous=await browser.newContext();
  const publicUrl=path=>new URL(path,page.url()).href;
  const anonymousGet=(path,options)=>anonymous.request.get(publicUrl(path),options);
  const created=[];
  try {
    expect((await anonymousGet('/api/public/catalog')).status()).toBe(404);
    expect((await anonymousGet('/.well-known/vela')).status()).toBe(404);
    await navigate(page,'Network');
    const network=page.getByRole('region',{name:'Network',exact:true});
    await expect(network.getByTestId('network-enabled-state')).toHaveText('Public catalog disabled');
    await expect(network.getByLabel('Enable public catalog',{exact:true})).not.toBeChecked();
    await expect(network.getByLabel('Confirm publication rights',{exact:true})).not.toBeChecked();
    await expect(network.getByLabel('Allow public streaming',{exact:true})).not.toBeChecked();
    await expect(network.getByLabel('Allow public downloads',{exact:true})).not.toBeChecked();
    await expect(network.getByLabel('Allow public downloads',{exact:true})).toBeDisabled();
    await expect(network.getByRole('button',{name:'Create publication',exact:true})).toBeDisabled();
    await network.getByLabel('Public server name',{exact:true}).fill('Vela isolated browser rights fixture');
    await network.getByLabel('Public HTTPS origin',{exact:true}).fill('https://music.example.org');
    await network.getByLabel('Enable public catalog',{exact:true}).check();
    const saved=page.waitForResponse(response=>new URL(response.url()).pathname==='/api/network/settings'&&response.request().method()==='PUT');
    await network.getByRole('button',{name:'Save public catalog settings',exact:true}).click();
    const enabled=await saved;expect(enabled.ok()).toBeTruthy();
    expect((await enabled.json()).revision).toBe(original.revision+1);
    await expect(network.getByTestId('network-enabled-state')).toHaveText('Public catalog enabled');
    const emptyCatalog=await(await anonymousGet('/api/public/catalog')).json();
    expect(emptyCatalog.origin).toBe('https://music.example.org');expect(emptyCatalog.items).toEqual([]);
    expect((await(await page.request.get('/api/publications?limit=20')).json()).items).toEqual([]);

    const selectTrack=async()=>{
      await network.getByLabel('Search your tracks',{exact:true}).fill(track.title);
      await network.getByRole('button',{name:'Find my tracks',exact:true}).click();
      const choice=network.locator(`.network-track-choice[data-track-id="${track.id}"]`);
      await expect(choice).toBeVisible();await choice.click();
      await expect(network.getByTestId('publication-selected-track')).toContainText(track.title);
    };
    const fillDeclaration=async()=>{
      await network.getByLabel('Declared public license',{exact:true}).selectOption('CC-BY-4.0');
      await network.getByLabel('Public attribution',{exact:true}).fill(`${track.title} — generated PCM by the Vela project, browser rights fixture`);
      await network.getByLabel('Rights evidence HTTPS URL',{exact:true}).fill('https://example.org/vela-generated-pcm-license');
      await expect(network.getByLabel('Confirm publication rights',{exact:true})).not.toBeChecked();
      await expect(network.getByRole('button',{name:'Create publication',exact:true})).toBeDisabled();
    };
    await selectTrack();await fillDeclaration();
    await network.getByLabel('Allow public streaming',{exact:true}).check();
    await expect(network.getByLabel('Allow public downloads',{exact:true})).toBeEnabled();
    await network.getByLabel('Allow public downloads',{exact:true}).check();
    await network.getByLabel('Allow public streaming',{exact:true}).uncheck();
    await expect(network.getByLabel('Allow public downloads',{exact:true})).not.toBeChecked();
    await expect(network.getByLabel('Allow public downloads',{exact:true})).toBeDisabled();
    await network.getByRole('button',{name:`Play my original ${track.title}`,exact:true}).click();
    await expect.poll(()=>page.locator('#vela-audio').evaluate(audio=>!audio.paused&&audio.currentTime>0&&audio.error===null)).toBeTruthy();
    expect(await page.locator('#vela-audio').evaluate(audio=>audio===window.publicationAudioNode)).toBeTruthy();
    await expect(page.locator('audio')).toHaveCount(1);
    await expect(page.locator('#vela-audio')).toHaveAttribute('src',new RegExp(`/api/tracks/${track.id}/stream`));
    await page.getByRole('button',{name:'Pause',exact:true}).click();
    const source=await page.locator('#vela-audio').getAttribute('src');
    await network.getByLabel('Confirm publication rights',{exact:true}).check();
    const metadataResponse=page.waitForResponse(response=>new URL(response.url()).pathname==='/api/publications'&&response.request().method()==='POST');
    await network.getByRole('button',{name:'Create publication',exact:true}).click();
    const metadataReply=await metadataResponse;expect(metadataReply.ok()).toBeTruthy();
    const metadata=await metadataReply.json();created.push(metadata.id);
    expect(metadata.track.id).toBe(track.id);expect(metadata.id).not.toBe(track.id);
    expect(metadata.allow_stream).toBe(false);expect(metadata.allow_download).toBe(false);
    const record=network.locator(`[data-publication-id="${metadata.id}"]`);
    await expect(record).toBeVisible();await expect(record).toContainText('Metadata only');
    await expect(network.getByLabel('Confirm publication rights',{exact:true})).not.toBeChecked();
    const publicMetadata=await(await anonymousGet(`/api/public/catalog/${metadata.id}`)).json();
    expect(publicMetadata.title).toBe(track.title);expect(publicMetadata.license).toBe('CC-BY-4.0');
    expect(publicMetadata.can_stream).toBe(false);expect(publicMetadata.can_download).toBe(false);
    const publicText=JSON.stringify(publicMetadata);
    expect(publicText).not.toContain(track.id);expect(publicText).not.toContain('rights_evidence');expect(publicText).not.toContain('asset_id');expect(publicText).not.toContain('owner_id');
    expect((await anonymousGet(`/api/public/media/${metadata.id}/stream`)).status()).toBe(404);
    expect((await anonymousGet(`/api/public/media/${metadata.id}/download`)).status()).toBe(404);
    expect((await anonymousGet(`/api/tracks/${track.id}/stream`)).status()).toBe(401);
    expect(await page.locator('#vela-audio').getAttribute('src')).toBe(source);

    await record.getByRole('button',{name:`Revoke publication ${track.title}`,exact:true}).click();
    const confirmation=network.getByRole('group',{name:'Confirm publication revocation',exact:true});
    await confirmation.getByRole('button',{name:'Keep publication',exact:true}).click();
    expect((await anonymousGet(`/api/public/catalog/${metadata.id}`)).ok()).toBeTruthy();
    await record.getByRole('button',{name:`Revoke publication ${track.title}`,exact:true}).click();
    const revokedMetadata=page.waitForResponse(response=>new URL(response.url()).pathname===`/api/publications/${metadata.id}/revoke`);
    await confirmation.getByRole('button',{name:'Confirm revoke publication',exact:true}).click();
    expect((await revokedMetadata).ok()).toBeTruthy();
    await expect(network.locator(`[data-publication-id="${metadata.id}"] .network-status`)).toHaveText('Revoked');
    expect((await anonymousGet(`/api/public/catalog/${metadata.id}`)).status()).toBe(404);

    await selectTrack();await fillDeclaration();
    await network.getByLabel('Confirm publication rights',{exact:true}).check();
    await network.getByLabel('Allow public streaming',{exact:true}).check();
    await expect(network.getByLabel('Confirm publication rights',{exact:true})).not.toBeChecked();
    await expect(network.getByLabel('Allow public downloads',{exact:true})).not.toBeChecked();
    await network.getByLabel('Confirm publication rights',{exact:true}).check();
    // Hold an actual committed POST while a root theme update rerenders the page.
    let release;const gate=new Promise(resolve=>release=resolve);let held=false,delivered=false;
    const handler=async route=>{
      if(route.request().method()!=='POST'){await route.continue();return;}
      const response=await route.fetch({timeout:3000});expect(response.ok()).toBeTruthy();held=true;
      await gate;await route.fulfill({response});delivered=true;
    };
    await page.route('**/api/publications',handler);
    let streamed;
    try{
      const streamedReply=page.waitForResponse(response=>new URL(response.url()).pathname==='/api/publications'&&response.request().method()==='POST');
      await network.getByRole('button',{name:'Create publication',exact:true}).click();
      await expect.poll(()=>held,{timeout:3000}).toBeTruthy();
      await expect(network.getByRole('button',{name:'Refresh publications',exact:true})).toBeDisabled();
      await page.getByRole('button',{name:'Change color theme',exact:true}).click();
      release();const response=await streamedReply;expect(response.ok()).toBeTruthy();
      streamed=await response.json();created.push(streamed.id);
      await expect.poll(()=>delivered,{timeout:3000}).toBeTruthy();
    }finally{release();await page.unroute('**/api/publications',handler);}
    expect(streamed.id).not.toBe(metadata.id);expect(streamed.allow_stream).toBe(true);expect(streamed.allow_download).toBe(false);
    const streamingRecord=network.locator(`[data-publication-id="${streamed.id}"]`);
    await expect(streamingRecord).toContainText('Streaming only');
    const range=await anonymousGet(`/api/public/media/${streamed.id}/stream`,{headers:{Range:'bytes=0-31'}});
    expect(range.status()).toBe(206);expect((await range.body()).length).toBe(32);
    expect((await anonymousGet(`/api/public/media/${streamed.id}/download`)).status()).toBe(404);
    const catalog=await(await anonymousGet('/api/public/catalog')).json();
    expect(catalog.items.map(item=>item.id)).toEqual([streamed.id]);
    expect(catalog.items[0].can_stream).toBe(true);expect(catalog.items[0].can_download).toBe(false);
    await streamingRecord.getByRole('button',{name:`Revoke publication ${track.title}`,exact:true}).click();
    const revokedStream=page.waitForResponse(response=>new URL(response.url()).pathname===`/api/publications/${streamed.id}/revoke`);
    await confirmation.getByRole('button',{name:'Confirm revoke publication',exact:true}).click();
    expect((await revokedStream).ok()).toBeTruthy();
    await expect(network.locator(`[data-publication-id="${streamed.id}"] .network-status`)).toHaveText('Revoked');
    for(const suffix of ['catalog/'+streamed.id,'media/'+streamed.id+'/stream','media/'+streamed.id+'/download']) {
      expect((await anonymousGet('/api/public/'+suffix)).status()).toBe(404);
    }
    expect((await(await anonymousGet('/api/public/catalog')).json()).items).toEqual([]);
    const latest=(await(await page.request.get('/api/tracks?limit=100')).json()).items.find(item=>item.id===track.id);
    expect(JSON.stringify(latest)).toBe(privateBefore);
    expect(await page.locator('#vela-audio').evaluate(audio=>audio===window.publicationAudioNode)).toBeTruthy();
    expect(await page.locator('#vela-audio').getAttribute('src')).toBe(source);
    await network.getByLabel('Enable public catalog',{exact:true}).uncheck();
    const disabled=page.waitForResponse(response=>new URL(response.url()).pathname==='/api/network/settings'&&response.request().method()==='PUT');
    await network.getByRole('button',{name:'Save public catalog settings',exact:true}).click();
    expect((await disabled).ok()).toBeTruthy();
    await expect(network.getByTestId('network-enabled-state')).toHaveText('Public catalog disabled');
    expect((await anonymousGet('/api/public/catalog')).status()).toBe(404);
    expect(errors).toEqual([]);
  }finally{
    // Restore only the disposable fixture's server policy, retaining audit records.
    const owned=await(await page.request.get('/api/publications?limit=100')).json();
    for(const record of owned.items.filter(item=>(created.includes(item.id)||item.rights_evidence==='https://example.org/vela-generated-pcm-license')&&item.revoked_at===null)) {
      expect((await page.request.post(`/api/publications/${record.id}/revoke`,{headers,data:{revision:record.revision}})).ok()).toBeTruthy();
    }
    const current=await(await page.request.get('/api/network/settings')).json();
    expect((await page.request.put('/api/network/settings',{headers,data:{revision:current.revision,enabled:false,origin:original.origin||'',name:original.name}})).ok()).toBeTruthy();
    await anonymous.close();
  }
});

test('PN02 public preview rejects private origins and obsolete publication reads stay cancelled across responsive navigation',async({page})=>{
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await openAdminFixture(page);
  expect(new URL(page.url()).origin).toBe('http://127.0.0.1:3217');
  await navigate(page,'Network');
  const network=page.getByRole('region',{name:'Network',exact:true});
  await expect(network.getByTestId('network-enabled-state')).toHaveText('Public catalog disabled');
  const preview=network.getByRole('region',{name:'Preview a public server',exact:true});
  await preview.getByLabel('Public server HTTPS origin',{exact:true}).fill('https://127.0.0.1');
  const denied=page.waitForResponse(response=>new URL(response.url()).pathname==='/api/network/preview'&&response.request().method()==='POST');
  await preview.getByRole('button',{name:'Preview public server',exact:true}).click();
  expect((await denied).status()).toBe(400);
  await expect(preview.getByRole('alert')).toBeVisible();
  await expect(preview.getByTestId('network-peer-result')).toHaveCount(0);
  await expect(preview.getByRole('link',{name:/Open public stream|Open public download/})).toHaveCount(0);
  await preview.getByLabel('Public server HTTPS origin',{exact:true}).fill('http://127.0.0.1:3217');
  await preview.getByRole('button',{name:'Preview public server',exact:true}).click();
  await expect(preview.getByRole('alert')).toContainText('public HTTPS origin');
  // Take a real owned-record read, hold its delivery, then physically leave its page.
  let release;const gate=new Promise(resolve=>release=resolve);let held=false,active=0;
  const handler=async route=>{
    active++;
    try{
      const response=await route.fetch({timeout:3000});expect(response.ok()).toBeTruthy();held=true;
      await gate;try{await route.fulfill({response});}catch{/* Teardown aborts the real obsolete read. */}
    }finally{active--;}
  };
  await page.route('**/api/publications?*',handler);
  try{
    await network.getByRole('button',{name:'Refresh publications',exact:true}).click();
    await expect.poll(()=>held,{timeout:3000}).toBeTruthy();
    const aborted=page.waitForEvent('requestfailed',{predicate:request=>new URL(request.url()).pathname==='/api/publications'});
    await navigate(page,'Search');await aborted;
    release();await expect.poll(()=>active,{timeout:3000}).toBe(0);
    await expect(page.getByRole('region',{name:'Network',exact:true})).toHaveCount(0);
    await expect(page.locator('#global-search')).toBeVisible();
  }finally{release();await page.unroute('**/api/publications?*',handler);}
  await navigate(page,'Network');
  await expect(network.getByTestId('network-enabled-state')).toHaveText('Public catalog disabled');
  const owned=await(await page.request.get('/api/publications?limit=20')).json();
  expect(owned.items.length).toBeGreaterThanOrEqual(2);
  expect(owned.items.every(record=>record.revoked_at!==null)).toBeTruthy();
  await expect(network.locator('[data-publication-id]')).toHaveCount(owned.items.length);
  await page.addStyleTag({content:'*,*::before,*::after{animation:none!important;transition:none!important;scroll-behavior:auto!important}'});
  for(const [width,theme] of [320,1440].flatMap(width=>['dark','light'].map(theme=>[width,theme]))) {
    await page.setViewportSize({width,height:900});
    await page.locator('html').evaluate((element,value)=>element.setAttribute('data-theme',value),theme);
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth)).toBeTruthy();
    await network.locator('.network-hero').scrollIntoViewIfNeeded();
    await page.screenshot({path:`test-results/network-${width}-${theme}.png`,animations:'disabled'});
    if(width===320){
      await navigate(page,'Settings');await navigate(page,'Network');
      await expect(network.getByTestId('network-enabled-state')).toHaveText('Public catalog disabled');
      await network.locator('.network-publication-form').scrollIntoViewIfNeeded();
      expect(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth)).toBeTruthy();
      await page.screenshot({path:`test-results/network-publication-${width}-${theme}.png`,animations:'disabled'});
    }
  }
  expect(errors).toEqual([]);
});

test('private job replies cannot reappear after logout and another account signs in', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await openAdminFixture(page);
  // Hold a genuine denied player mutation through the same real account switch.
  await findTrack(page,'Drift');await page.getByRole('button',{name:'Play Drift',exact:true}).click();
  await page.getByRole('button',{name:'Pause',exact:true}).click();
  let releaseFavorite;const favoriteGate=new Promise(resolve=>releaseFavorite=resolve);let favoriteSeen;const favoriteHeld=new Promise(resolve=>favoriteSeen=resolve);
  await page.route('**/api/tracks/*/favorite',async route=>{
    const response=await route.fetch({headers:{...route.request().headers(),'x-csrf-token':'invalid-fixture-token'}});expect(response.ok()).toBeFalsy();favoriteSeen();await favoriteGate;await route.fulfill({response});
  });
  const player=await openNowPlaying(page);await player.getByRole('button',{name:/current track.*favorites|Favorite current track/}).click();await favoriteHeld;
  await player.getByRole('button',{name:'Close now playing',exact:true}).click();
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
  const favoriteReply=page.waitForResponse(response=>response.url().endsWith('/favorite')&&response.request().method()==='POST');releaseFavorite();await favoriteReply;
  await expect(page.getByRole('dialog',{name:'Now playing',exact:true})).toHaveCount(0);
  await expect(page.getByRole('button',{name:'Open now playing',exact:true})).toHaveCount(0);
  // The root's own error banner is scoped to the content column: the
  // feedback stack owns a permanent alert region for the Undo failure, so a
  // page-wide alert query would match that empty region too.
  await expect(page.locator('.content-inner').getByRole('alert')).toHaveCount(0);
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
  // TR08: Settings remains the hub of the remaining routes for this account, and
  // Jobs is not offered where this listener cannot open it.
  await navigate(page, 'Settings');
  const settingsRoutes = page.getByRole('navigation', { name: 'More from Vela', exact: true });
  for (const name of ['Albums', 'Artists', 'Favorites', 'Listening history', 'Add music', 'Network']) {
    await expect(settingsRoutes.getByRole('button', { name, exact: true })).toBeVisible();
  }
  await expect(settingsRoutes.getByRole('button', { name: 'Jobs', exact: true })).toHaveCount(0);
  await expect(settingsRoutes.getByRole('button', { name: 'Track radio', exact: true })).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
  expect(errors).toEqual([]);
});
