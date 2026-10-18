// Test-only interaction driver; real Rust HTTP, queue and audio remain authoritative.
const config = await dioxus.recv();
const checks = [];
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const wait = async (test, name, timeout = 18000) => {
  const end = performance.now() + timeout;
  while (performance.now() < end) { if (test()) return; await sleep(60); }
  throw new Error(name);
};
const assert = (condition, name) => { if (!condition) throw new Error(name); checks.push(name); };
const el = selector => document.querySelector(selector);
const button = name => [...document.querySelectorAll('button')].find(node => (node.getAttribute('aria-label') || node.textContent).trim() === name && node.getClientRects().length);
const click = async name => { await wait(() => button(name) && !button(name).disabled, `button: ${name}`); button(name).click(); await sleep(100); };
const pageText = selector => [...document.querySelectorAll(selector)].map(node => node.textContent.trim());
const fill = (selector, value, event = 'input') => { const input = el(selector); if (!input) throw new Error(`Missing input ${selector}`); input.value = value; input.dispatchEvent(new Event(event, { bubbles:true })); };
const phase = () => el('.native-player')?.dataset.audioPhase;
const position = () => Number(el('#native-seek')?.value || 0);
const generation = () => el('.native-player')?.dataset.audioGeneration;
const capture = async name => { await sleep(200); dioxus.send({capture:name}); await dioxus.recv(); };
const dpr = () => (Number.isFinite(devicePixelRatio) && devicePixelRatio > 0) ? devicePixelRatio : 1;
const physical = value => Math.max(320, Math.min(4096, Math.round(value * dpr())));
const resize = async (width, height) => {
  dioxus.send({resize:{width:physical(width),height:physical(height)}});
  await dioxus.recv(); await sleep(300);
};
const resizeCompact = async () => {
  const trace = [];
  await resize(640, 700);
  trace.push(innerWidth);
  for (let attempt = 0; attempt < 5 && !(innerWidth >= 620 && innerWidth <= 660); attempt++) {
    const current = innerWidth;
    if (!(Number.isFinite(current) && current > 0)) { await sleep(200); continue; }
    const target = Math.max(320, Math.min(4096, Math.round(physical(640) * (640 / current))));
    dioxus.send({resize:{width:target,height:physical(700)}});
    await dioxus.recv(); await sleep(350);
    trace.push(innerWidth);
  }
  window.__velaResizeTrace = trace;
};
try {
  await wait(() => button('Sign in') && !button('Sign in').disabled, 'Native output initialization');
  fill('#native-origin', config.origin); fill('#native-username', config.username); fill('#native-password', config.password);
  el('.signin-form form').requestSubmit();
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Three real private uploaded tracks');
  await click('Favorite Signal');
  await wait(() => button('Remove Signal from favorites'), 'Real favorite mutation acknowledged');
  assert(button('Remove Signal from favorites'), 'Favorite mutation acknowledged by the API');
  await click('Favorites');
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 1 && button('Play Signal'), 'Favorites API view shows exactly the favorite');
  assert(document.querySelectorAll('[data-native-track-id]').length === 1 && button('Play Signal'), 'Favorites view shows only the favorite track');
  await capture('favorites-dark');
  await click('Remove Signal from favorites');
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 0 && button('Refresh favorites'), 'Unfavorite re-reads the filtered page');
  assert(document.querySelectorAll('[data-native-track-id]').length === 0, 'Unfavorite removes the row from the filtered view');
  await click('Library');
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Library restores three tracks');
  assert(document.querySelectorAll('[data-native-track-id]').length === 3, 'Library navigation restores the full owned catalog');
  await resize(1280, 840);
  assert(document.querySelectorAll('audio,iframe').length === 0, 'No HTML media or iframe playback substitute');
  fill('#native-volume', '0'); await sleep(400);
  assert(Number(el('#native-volume').value) === 0, 'Native volume acknowledged zero before playback');
  await capture('library-dark');
  await click('Play Native Continuity');
  await wait(() => phase() === 'Playing' && position() > 400, 'Native playback clock advances');
  el('[data-testid="native-player-favorite"]').click();
  await wait(() => el('[data-testid="native-player-favorite"]')?.getAttribute('aria-label') === 'Remove Native Continuity from favorites', 'Player favorite mutation');
  assert(el('[data-testid="native-player-favorite"]')?.getAttribute('aria-label') === 'Remove Native Continuity from favorites', 'Player favorite control updates the shared queue state');
  const gen = generation(); const before = position();
  await click('Queue'); await sleep(450);
  assert(generation() === gen && position() > before, 'Navigation preserves native source and advancing position');
  await click('Pause'); await wait(() => phase() === 'Paused', 'Native pause');
  const paused = position(); await sleep(450);
  assert(Math.abs(position() - paused) < 150, 'Paused native position is stable');
  fill('#native-seek', '6000', 'change');
  await wait(() => position() >= 5900 && position() <= 6300 && phase() === 'Paused', 'Native seek remains paused');
  await click('Play'); await wait(() => phase() === 'Playing' && position() > 6400, 'Native resume after seek');
  await click('Pause'); await wait(() => phase() === 'Paused', 'Pause before queue edits');
  await click('Change color theme');
  assert(el('.native-app').dataset.theme === 'light', 'Functional light theme');
  await capture('queue-light');
  await click('Library');
  fill('#native-search', 'Signal'); el('.native-search').requestSubmit();
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 1 && button('Play Signal'), 'Real API search');
  assert(generation() === gen && phase() === 'Paused', 'Search preserves native playback');
  await click('Play Signal next'); await click('Add Signal to queue');
  await click('Queue');
  assert(document.querySelectorAll('[data-native-queue-index]').length === 5, 'Duplicate queue occurrences retained');
  await click('Move queue occurrence 4 up'); await click('Remove queue occurrence 4');
  assert(document.querySelectorAll('[data-native-queue-index]').length === 4, 'Queue reorder and removal');
  const queueCurrent = () => [...document.querySelectorAll('.native-queue-row')].findIndex(row => row.classList.contains('current'));
  const beforeMove = queueCurrent();
  assert(beforeMove >= 0 && beforeMove + 1 < document.querySelectorAll('.native-queue-row').length, 'Queue has a movable current occurrence');
  await click(`Move queue occurrence ${beforeMove} down`);
  await wait(() => (el('.alert.notice')?.textContent || '').includes('Queue order updated.'), 'Queue move-down feedback');
  try { await wait(() => queueCurrent() === beforeMove + 1, 'Native queue move-down'); }
  catch { throw new Error(`Native queue move-down failed rows=${JSON.stringify([...document.querySelectorAll('.native-queue-row')].map(row => [row.className, row.textContent.trim()]))}`); }
  assert(queueCurrent() === beforeMove + 1, 'Queue move-down follows the current occurrence');
  await click(`Move queue occurrence ${beforeMove + 1} up`);
  await wait(() => queueCurrent() === beforeMove, 'Native queue move-up restore');
  await click('Clear upcoming');
  await wait(() => document.querySelectorAll('[data-native-queue-index]').length === beforeMove + 1, 'Native clear upcoming');
  assert(document.querySelectorAll('[data-native-queue-index]').length === beforeMove + 1, 'Clear upcoming keeps the current occurrence and earlier rows');
  await resizeCompact();
  assert(innerWidth >= 620 && innerWidth <= 660, `Compact native viewport is 640px (actual ${innerWidth}px, dpr ${dpr()}, trace ${JSON.stringify(window.__velaResizeTrace || [])})`);
  const queueBounds = [...document.querySelectorAll('[data-native-queue-index]')].map(node => node.getBoundingClientRect());
  assert(queueBounds.every(rect => rect.left >= -1 && rect.right <= innerWidth + 1), `Compact queue rows stay inside ${innerWidth}px (right edges ${queueBounds.map(rect => Math.round(rect.right)).join(',')})`);
  const queueButtons = [...document.querySelectorAll('[data-native-queue-index] .icon-button')].map(node => node.getBoundingClientRect());
  assert(queueButtons.every(rect => rect.left >= -1 && rect.right <= innerWidth + 1 && rect.width >= 34), `Compact queue controls remain reachable (${queueButtons.map(rect => Math.round(rect.right)).join(',')})`);
  const compactPlayer = el('.native-player')?.getBoundingClientRect();
  assert(compactPlayer && compactPlayer.right <= innerWidth + 1 && compactPlayer.left >= -1, `Compact player stays inside ${innerWidth}px`);
  await capture('compact-light');
  assert(document.documentElement.scrollWidth <= innerWidth + 1, 'Compact native window has no horizontal document overflow');
  await click('Clear queue'); await wait(() => phase() === 'Idle', 'Clear queue stops native audio');
  assert(document.querySelectorAll('[data-native-queue-index]').length === 0, 'Queue is cleared');
  await click('Library'); fill('#native-search', 'Beacon'); el('.native-search').requestSubmit();
  await wait(() => el('.native-library')?.dataset.query === 'Beacon' && Number(el('.native-library')?.dataset.catalogItems) === 1 && button('Play Beacon'), 'MP3 real search');
  await click('Play Beacon');
  try { await wait(() => phase() === 'Playing' && position() > 100, 'Real MP3 native decoder advances'); }
  catch { const library = el('.native-library'); throw new Error(`Real MP3 native decoder advances (phase=${phase()} position=${position()} preparing=${!!el('.preparation-progress')} error=${el('.player-error')?.textContent?.trim() || '-'} library=${library ? JSON.stringify(library.dataset) : 'none'} search=${el('#native-search')?.value || '-'} rows=${document.querySelectorAll('[data-native-track-id]').length} api=${JSON.stringify(performance.getEntriesByType('resource').map(e=>e.name).filter(n=>n.includes('/api/tracks')).slice(-3))})`); }
  await click('Playlists');
  await wait(() => button('Open playlist Native evening') && button('Create playlist'), 'Real playlist list');
  assert(button('Open playlist Native evening'), 'Playlist list shows the real fixture playlist');
  fill('#native-playlist-name', 'Native draft'); await click('Create playlist');
  await wait(() => button('Open playlist Native draft'), 'Native playlist creation');
  assert(button('Open playlist Native draft'), 'Native playlist creation acknowledged by the API');
  await click('Open playlist Native draft');
  await wait(() => el('#native-playlist-rename')?.value === 'Native draft' && button('Save name'), 'Native playlist detail');
  fill('#native-playlist-rename', 'Native renamed'); await click('Save name');
  try { await wait(() => el('[data-testid=\"native-playlist-name\"]')?.textContent === 'Native renamed', 'Native playlist rename'); }
  catch { throw new Error(`Native playlist rename failed (heading=${el('[data-testid=\"native-playlist-name\"]')?.textContent || '-'} error=${el('.alert.error')?.textContent?.trim() || '-'} input=${el('#native-playlist-rename')?.value || '-'})`); }
  assert(el('[data-testid=\"native-playlist-name\"]')?.textContent === 'Native renamed', 'Native playlist rename acknowledged by the API');
  await click('Delete playlist'); await wait(() => button('Confirm delete'), 'Delete confirmation');
  await click('Cancel');
  assert(el('[data-testid=\"native-playlist-name\"]')?.textContent === 'Native renamed', 'Cancel preserves the selected playlist');
  await click('Delete playlist'); await wait(() => button('Confirm delete'), 'Delete confirmation again');
  await click('Confirm delete');
  await wait(() => !button('Open playlist Native renamed') && button('Create playlist') && !el('#native-playlist-rename'), 'Native playlist deletion');
  assert(!button('Open playlist Native renamed'), 'Native playlist deletion removes the card and detail');
  await wait(() => button('Open playlist Native evening'), 'Fixture playlist survives native deletion');
  await click('Open playlist Native evening');
  await wait(() => button('Play playlist') && document.querySelectorAll('.native-queue-row').length === 2, 'Real playlist detail with two entries');
  assert(document.querySelectorAll('.native-queue-row').length === 2 && button('Play playlist'), 'Playlist detail shows both owned entries');
  const trackPicker = el('#native-playlist-track'); trackPicker.value = trackPicker.options[3].value; trackPicker.dispatchEvent(new Event('change', { bubbles:true }));
  await click('Add selected track');
  await wait(() => document.querySelectorAll('.native-queue-row').length === 3, 'Native playlist track add');
  assert(document.querySelectorAll('.native-queue-row').length === 3, 'Owned library track added to the native playlist');
  await capture('playlists-dark');
  await click('Play playlist');
  await wait(() => phase() === 'Playing' && position() > 200, 'Playlist playback starts through native audio');
  assert(phase() === 'Playing', 'Playlist playback starts through native audio');
  await click('Queue');
  await wait(() => document.querySelectorAll('[data-native-queue-index]').length === 3, 'Playlist queue populated');
  assert(document.querySelectorAll('[data-native-queue-index]').length === 3, 'Playlist playback populates the shared native queue');
  await click('Library');
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Library after playlist playback');
  await click('Albums');
  await wait(() => button('Open album Real Fixtures by Vela Test'), 'Real album groups');
  await click('Open album Real Fixtures by Vela Test');
  await wait(() => el('[data-testid="native-group-title"]')?.textContent === 'Real Fixtures · Vela Test' && button('Play album'), 'Real album detail');
  assert(button('Play album'), 'Album detail exposes real grouped tracks');
  await capture('albums-dark');
  await click('Play album');
  await wait(() => (el('.native-player-status small')?.textContent || '').includes('Real Fixtures') && phase() === 'Playing' && position() > 0, 'Album playback context');
  assert(phase() === 'Playing', 'Album playback starts through native audio');
  await click('Library');
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Library after album playback');
  await click('Artists');
  await wait(() => button('Open artist Vela Test'), 'Real artist groups');
  await click('Open artist Vela Test');
  await wait(() => el('[data-testid="native-group-title"]')?.textContent === 'Vela Test' && button('Play artist'), 'Real artist detail');
  assert(button('Play artist'), 'Artist detail exposes real grouped tracks');
  await click('Play artist');
  await wait(() => (el('.native-player-status small')?.textContent || '').includes('Vela Test') && phase() === 'Playing' && position() > 0, 'Artist playback context');
  assert(phase() === 'Playing', 'Artist playback starts through native audio');
  await click('Library');
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Library after artist playback');
  if (config.closeWhilePlaying) {
    await click('Pause');
    fill('#native-search', 'Native Continuity'); el('.native-search').requestSubmit();
    await wait(() => button('Play Native Continuity'), 'Long track for close during playback');
    await click('Play Native Continuity');
    await wait(() => phase() === 'Playing' && position() > 300, 'Close while real audio is advancing');
    dioxus.send({ok:true, checks, closeWhilePlaying:true});
  } else {
    await click('Playlists');
    await wait(() => button('Open playlist Native evening'), 'Playlist reopen for entry editing');
    await click('Open playlist Native evening');
    await wait(() => document.querySelectorAll('.native-queue-row').length === 3 && button('Move playlist entry 2 up'), 'Playlist entry editing controls');
    const orderBefore = pageText('.native-queue-row .native-queue-select strong');
    await click('Move playlist entry 2 up');
    await wait(() => JSON.stringify(pageText('.native-queue-row .native-queue-select strong')) === JSON.stringify([orderBefore[1], orderBefore[0], ...orderBefore.slice(2)]), 'Playlist reorder applied');
    assert(true, 'Playlist entry reorder acknowledged by the API');
    await click('Move playlist entry 1 down');
    await wait(() => JSON.stringify(pageText('.native-queue-row .native-queue-select strong')) === JSON.stringify(orderBefore), 'Playlist order restored');
    assert(true, 'Playlist entry order can be restored');
    await click('Remove playlist entry 2');
    await wait(() => document.querySelectorAll('.native-queue-row').length === 2, 'Playlist entry removal applied');
    assert(document.querySelectorAll('.native-queue-row').length === 2, 'Playlist entry removal acknowledged by the API');
    await click('Public');
    await wait(() => el('#native-peer-origin'), 'Public catalog view');
    fill('#native-peer-origin', config.peerOrigin);
    await click('Explore public catalog');
    try { await wait(() => document.querySelectorAll('[data-public-track-id]').length >= 1, 'Second-server public catalog'); }
    catch { throw new Error(`Second-server public catalog failed (error=${el('.alert.error')?.textContent?.trim() || '-'} input=${el('#native-peer-origin')?.value || '-'} rows=${document.querySelectorAll('[data-public-track-id]').length})`); }
    assert(document.querySelectorAll('[data-public-track-id]').length >= 1, 'Second-server public catalog exposes an explicitly published track');
    await click('Save this server');
    await wait(() => (el('.alert.notice')?.textContent || '').includes('Public server saved.'), 'Save public server');
    assert(document.querySelector('[aria-label^="Remove saved server"]'), 'Saved public server is listed');
    await capture('public-dark');
    el('[aria-label^="Remove saved server"]').click();
    await wait(() => (el('.alert.notice')?.textContent || '').includes('Public server removed.'), 'Remove public server');
    const remainingLabels = [...document.querySelectorAll('[aria-label^="Remove saved server"]')].map(node => node.getAttribute('aria-label'));
    if (document.querySelector(`[aria-label="Remove saved server ${config.peerOrigin}/"]`)) throw new Error(`Saved public server can be removed failed labels=${JSON.stringify(remainingLabels)} expected=${config.peerOrigin}/`);
    checks.push('Saved public server can be removed');
    await click('Save this server');
    await wait(() => document.querySelector('[aria-label^="Remove saved server"]'), 'Re-save public server');
    assert(document.querySelector('[aria-label^="Remove saved server"]'), 'Saved public server persists for the config check');
    const publicRow = el('[data-public-track-id]');
    publicRow.querySelector('button').click();
    try { await wait(() => phase() === 'Playing' && position() > 100 && (el('.native-player-status small')?.textContent || '').includes('Public ·'), 'Public track playback'); }
    catch { throw new Error(`Public track playback failed (phase=${phase()} position=${position()} status=${el('.native-player-status small')?.textContent || '-'} error=${el('.player-error')?.textContent?.trim() || '-'})`); }
    assert(phase() === 'Playing', 'Authorized second-server public audio plays through the native engine');
    assert((el('.native-player-status small')?.textContent || '').includes('Public ·'), 'Player identifies the public origin context');
    fill('#native-peer-query', 'Vela'); await click('Search saved servers');
    try { await wait(() => [...document.querySelectorAll('[data-public-track-id][data-public-origin]')].map(row => row.dataset.publicOrigin).filter((origin, index, all) => all.indexOf(origin) === index).length >= 2, 'Unified saved-server search'); }
    catch { throw new Error(`Unified saved-server search failed (error=${el('.alert.error')?.textContent?.trim() || '-'} query=${el('#native-peer-query')?.value || '-'} origins=${JSON.stringify([...document.querySelectorAll('[data-public-track-id][data-public-origin]')].map(row => row.dataset.publicOrigin))} saved=${JSON.stringify([...document.querySelectorAll('[aria-label^="Remove saved server"]')].map(node => node.getAttribute('aria-label')))} peers=${JSON.stringify([...document.querySelectorAll('[data-public-peer-summary]')].map(node => [node.dataset.publicOrigin, node.dataset.publicPeerError]))})`); }
    const origins = [...document.querySelectorAll('[data-public-track-id][data-public-origin]')].map(row => row.dataset.publicOrigin);
    assert(origins.some(origin => origin.startsWith(config.peerOrigin)) && origins.some(origin => origin.startsWith(config.peer2Origin)), 'Unified search returns both saved public origins');
    await capture('public-search-dark');
    const peer2Row = [...document.querySelectorAll('[data-public-track-id][data-public-origin]')].find(row => row.dataset.publicOrigin.startsWith(config.peer2Origin));
    assert(peer2Row, 'Second saved origin is represented in unified results');
    peer2Row.querySelector('button').click();
    await wait(() => phase() === 'Playing' && (el('.native-player-status small')?.textContent || '').includes(config.peer2Origin), 'Second-origin public playback');
    assert(phase() === 'Playing', 'Public audio from the second saved origin plays natively');
    await click('Library');
    await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Library after public playback');
    await click('Sign out'); await wait(() => button('Sign in'), 'Native sign-out clears account');
    assert(!el('.native-player') && document.querySelectorAll('[data-native-track-id]').length === 0, 'Sign-out removes private catalog and player');
    dioxus.send({ok:true, checks});
  }
} catch(error) { dioxus.send({ok:false, checks, failure:String(error.message)}); }
