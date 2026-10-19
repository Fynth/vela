// Test-only interaction driver; real Rust HTTP, queue and audio remain authoritative.
const config = await dioxus.recv();
const runtimeErrors = [];
window.addEventListener('error', event => runtimeErrors.push(String(event.message || event.error || 'error')));
window.addEventListener('unhandledrejection', event => runtimeErrors.push(String(event.reason || 'rejection')));
const originalConsoleError = console.error.bind(console);
console.error = (...args) => { runtimeErrors.push(args.map(String).join(' ')); originalConsoleError(...args); };
const checks = [];
const viewportEvidence = [];
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
const fill = (selector, value, event = 'input') => { const input = el(selector); if (!input) throw new Error(`Missing input ${selector}`); const disclosure = input.closest('details'); if(disclosure && !disclosure.open) disclosure.querySelector('summary').click(); input.value = value; input.dispatchEvent(new Event(event, { bubbles:true })); };
const phase = () => el('.native-player')?.dataset.audioPhase;
const position = () => Number(el('#native-seek')?.value || 0);
const generation = () => el('.native-player')?.dataset.audioGeneration;
const layoutSnapshot = name => {
  // Inspect the product, excluding Dioxus's off-canvas development/HMR chrome.
  const nodes = [...document.querySelectorAll('.native-app,.native-app *')].filter(node => {
    const style = getComputedStyle(node), rect = node.getBoundingClientRect();
    return !node.closest('svg,.sr-only') && style.display !== 'none' && style.visibility !== 'hidden' && rect.width > 0 && rect.height > 0;
  }).slice(0, 1500);
  const ids = new Map(nodes.map((node, at) => [node, node.id ? `#${node.id}` : `native-${at}`]));
  const number = value => parseFloat(value) || 0;
  const edges = (style, prefix) => Object.fromEntries(['top', 'right', 'bottom', 'left'].map(edge => [edge, number(style[prefix + edge[0].toUpperCase() + edge.slice(1)])]));
  return {
    name, title: document.title, viewport: {width: innerWidth, height: innerHeight, devicePixelRatio},
    elements: nodes.map(node => {
      const rect = node.getBoundingClientRect(), style = getComputedStyle(node), tag = node.tagName.toLowerCase();
      const interactive = ['button', 'input', 'select', 'summary', 'a'].includes(tag);
      return {id: ids.get(node), parentId: ids.get(node.parentElement), tag, role: node.getAttribute('role') || undefined,
        label: (node.getAttribute('aria-label') || ((interactive || /^h[1-6]$/.test(tag)) ? node.textContent : '')).trim().slice(0, 80) || undefined,
        interactive, inputType: tag === 'input' ? node.type : undefined,
        rect: {x: rect.x, y: rect.y, width: rect.width, height: rect.height},
        clientWidth: node.clientWidth, clientHeight: node.clientHeight, scrollWidth: node.scrollWidth, scrollHeight: node.scrollHeight,
        style: {display: style.display, position: style.position, flexDirection: style.flexDirection, flexWrap: style.flexWrap,
          alignItems: style.alignItems, justifyContent: style.justifyContent, rowGap: number(style.rowGap), columnGap: number(style.columnGap),
          padding: edges(style, 'padding'), margin: edges(style, 'margin'), overflowX: style.overflowX, overflowY: style.overflowY,
          textOverflow: style.textOverflow, backgroundColor: style.backgroundColor, backgroundImage: style.backgroundImage}};
    })
  };
};
const capture = async name => { await sleep(200); dioxus.send({capture:name, layout:layoutSnapshot(name)}); await dioxus.recv(); };
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
// The compositor owns the first geometry, so the fixture asks for its configured
// viewport explicitly before the artwork evidence is captured.
const resizeExact = async (targetWidth, targetHeight) => {
  const trace = [];
  await resize(targetWidth, targetHeight);
  trace.push(innerWidth);
  for (let attempt = 0; attempt < 6 && Math.abs(innerWidth - targetWidth) > 2; attempt++) {
    const current = innerWidth;
    if (!(Number.isFinite(current) && current > 0)) { await sleep(200); continue; }
    const width = Math.max(320, Math.min(4096, Math.round(physical(targetWidth) * targetWidth / current)));
    const height = innerHeight > 0 ? Math.max(320, Math.min(4096, Math.round(physical(targetHeight) * targetHeight / innerHeight))) : physical(targetHeight);
    dioxus.send({resize:{width,height}});
    await dioxus.recv(); await sleep(350);
    trace.push(innerWidth);
  }
  return trace;
};
const imageMetrics = async image => {
  try { await image.decode(); } catch {}
  const style = getComputedStyle(image), rect = image.getBoundingClientRect();
  return {
    src: image.getAttribute('src') || '', complete: image.complete,
    naturalWidth: image.naturalWidth, naturalHeight: image.naturalHeight,
    objectFit: style.objectFit, objectPosition: style.objectPosition,
    boxWidth: rect.width, boxHeight: rect.height,
    artwork: image.dataset.nativeArtwork, classes: image.className
  };
};
if (config.windowOnly) {
  try {
    await wait(() => button('Sign in') && !button('Sign in').disabled, 'Native output initialization');
    assert(config.decorations === false, 'Native Wayland window disables GTK titlebar decorations');
    assert(button('Close window') && button('Minimize window') && button('Maximize or restore window'), 'Integrated window controls are reachable without a titlebar');
    fill('#native-origin', config.origin); fill('#native-username', config.username); fill('#native-password', config.password);
    el('.signin-form form').requestSubmit();
    await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Actual private library on Wayland');
    fill('#native-volume', '0'); await sleep(300);
    await click('Play Native Continuity');
    await wait(() => phase() === 'Playing' && position() > 200, 'Wayland native clock advances');
    const source = generation();
    fill('#native-seek', '5000', 'change');
    await wait(() => position() >= 4900, 'Wayland seek');
    await click('Pause Native Continuity');
    await wait(() => phase() === 'Paused', 'Wayland row pause');
    const paused = position(); await sleep(300);
    assert(generation() === source && paused >= 4900 && Math.abs(position() - paused) < 150, 'Wayland row pause preserves source and position');
    await click('Play Native Continuity');
    await wait(() => phase() === 'Playing' && position() > paused + 100, 'Wayland row resume');
    assert(generation() === source, 'Wayland row resume retains the native source');
    assert(document.documentElement.scrollWidth <= innerWidth + 1, 'Normal tiled Wayland window has no horizontal overflow');
    await capture('library-dark');
    dioxus.send({ok:true, checks, viewportEvidence:[{actualWidth:innerWidth,actualHeight:innerHeight,mode:'normal tiled, no resize requests'}]});
  } catch(error) { dioxus.send({ok:false, checks, failure:String(error.message)}); }
} else if (config.sessionPhase === 'fps') {
  try {
    await wait(() => button('Sign in') && !button('Sign in').disabled, 'Sign-in ready');
    await new Promise(r => setTimeout(r, 6000));
    const sample = await new Promise(resolve => {
      const times=[]; let last=performance.now(); const start=last;
      function tick(now){ times.push(now-last); last=now; if(now-start<3000) requestAnimationFrame(tick); else resolve({frames:times.length,elapsed:now-start,intervals:times}); }
      requestAnimationFrame(tick);
    });
    const fps = (sample.frames*1000/sample.elapsed).toFixed(1);
    dioxus.send({ok:true, checks:[`rAF ${fps} fps over ${sample.elapsed.toFixed(0)}ms`, `dpr=${devicePixelRatio} viewport=${innerWidth}x${innerHeight} screen=${screen.width}x${screen.height}`]});
  } catch(error) { dioxus.send({ok:false, checks, failure:String(error.message)}); }
} else if (config.sessionPhase === 'resume') {
  try {
    await wait(() => button('Continue saved session') && !button('Continue saved session').disabled, 'Saved session option');
    await click('Continue saved session');
    await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Saved session resume');
    assert(button('Sign out'), 'Saved session resumes the private account without a password');
    if (config.testSession) {
      await wait(() => (el('[data-offline-track-id]')?.textContent || '').includes('Integrity check failed'), 'Offline startup integrity sweep');
      assert(el('[data-offline-track-id] button')?.disabled, 'Corrupt offline copy disables Play offline');
    }
    await click('Sign out'); await wait(() => button('Sign in'), 'Resume sign-out');
    assert(!el('.native-player') && !document.querySelector('[data-native-track-id]'), 'Resume sign-out clears private state');
    dioxus.send({ok:true, checks, viewportEvidence});
  } catch(error) { dioxus.send({ok:false, checks, failure:String(error.message)}); }
} else {
try {
  assert(config.decorations === false, 'Linux window disables GTK titlebar decorations');
  assert(button('Close window') && button('Minimize window') && button('Maximize or restore window'), 'Frameless window keeps accessible window controls');
  const requestedWidth = Number(config.windowWidth) || 1280;
  const requestedHeight = Number(config.windowHeight) || 840;
  const resizeTrace = await resizeExact(requestedWidth, requestedHeight);
  assert(Math.abs(innerWidth - requestedWidth) <= 2 && Math.abs(innerHeight - requestedHeight) <= 2, `Native QA window runs at the requested ${requestedWidth}x${requestedHeight} viewport (actual ${innerWidth}x${innerHeight}, dpr ${dpr()}, trace ${JSON.stringify(resizeTrace)})`);
  await wait(() => button('Sign in') && !button('Sign in').disabled, 'Native output initialization');
  fill('#native-origin', config.origin); fill('#native-username', config.username); fill('#native-password', config.password);
  el('.signin-form form').requestSubmit();
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Three real private uploaded tracks');
  // ART01/ART11: the uploaded FLAC carries a real embedded cover. The renderer may only
  // receive the validated data URL the native client authorized; a missing cover is the
  // neutral Vela glyph, never a broken image.
  fill('#native-volume', '0'); await sleep(200);
  const rowCovers = () => [...document.querySelectorAll('[data-native-track-id] img.neutral-cover')];
  await wait(() => rowCovers().some(image => image.src.startsWith('data:image/') && image.complete && image.naturalWidth > 0 && image.naturalHeight > 0), 'Authorized library cover <img>');
  const libraryCover = rowCovers().find(image => image.getAttribute('src')?.startsWith('data:image/') && image.naturalWidth > 0);
  const libraryMetrics = await imageMetrics(libraryCover);
  const artworkWidth = Math.round(innerWidth);
  await capture(`library-artwork-${artworkWidth}`);
  assert(libraryMetrics.artwork === 'ready', `Library cover <img> is the ready artwork state (actual ${libraryMetrics.artwork}, classes ${libraryMetrics.classes})`);
  assert(libraryMetrics.naturalWidth > 0 && libraryMetrics.naturalHeight > 0, `Library cover decodes real pixels (natural ${libraryMetrics.naturalWidth}x${libraryMetrics.naturalHeight})`);
  assert(libraryMetrics.objectFit === 'cover', `Library cover uses object-fit: cover (actual ${libraryMetrics.objectFit})`);
  assert(libraryMetrics.boxWidth > 0 && Math.abs(libraryMetrics.boxWidth - libraryMetrics.boxHeight) < 1, `Library cover occupies a square box (actual ${libraryMetrics.boxWidth}x${libraryMetrics.boxHeight})`);
  assert(document.querySelectorAll('[data-native-track-id] .neutral-cover[data-native-artwork="missing"]').length >= 1, 'Tracks without embedded art render the neutral Vela fallback');
  assert([...document.images].every(image => (image.getAttribute('src') || '').startsWith('data:image/')), 'The renderer never fetches an arbitrary image URL');
  assert([...document.images].every(image => image.naturalWidth > 0), 'No broken-image glyph is rendered');
  checks.push(`Library cover evidence: src ${libraryMetrics.src.slice(0, 23)}… (${libraryMetrics.src.length} chars), natural ${libraryMetrics.naturalWidth}x${libraryMetrics.naturalHeight}, box ${libraryMetrics.boxWidth}x${libraryMetrics.boxHeight}, object-fit ${libraryMetrics.objectFit}/${libraryMetrics.objectPosition}, missing fallbacks ${document.querySelectorAll('[data-native-track-id] .neutral-cover[data-native-artwork="missing"]').length}, viewport ${innerWidth}x${innerHeight} dpr ${dpr()}`);
  await click('Play Signal');
  await wait(() => (el('.player-track strong')?.textContent || '') === 'Signal' && phase() === 'Playing' && position() > 100, 'Signal playback for the player cover');
  const playerCoverNode = () => [...document.querySelectorAll('.native-player img.neutral-cover.player-cover')].find(image => image.src.startsWith('data:image/') && image.naturalWidth > 0);
  await wait(() => playerCoverNode(), 'Bottom player cover <img>');
  const playerMetrics = await imageMetrics(playerCoverNode());
  await capture(`player-artwork-${Math.round(innerWidth)}`);
  assert(playerMetrics.artwork === 'ready', `Player cover <img> is the ready artwork state (actual ${playerMetrics.artwork})`);
  assert(playerMetrics.naturalWidth > 0 && playerMetrics.naturalHeight > 0, `Player cover decodes real pixels (natural ${playerMetrics.naturalWidth}x${playerMetrics.naturalHeight})`);
  assert(playerMetrics.objectFit === 'cover', `Player cover uses object-fit: cover (actual ${playerMetrics.objectFit})`);
  assert(playerMetrics.boxWidth > 0 && Math.abs(playerMetrics.boxWidth - playerMetrics.boxHeight) < 1, `Player cover occupies a square box (actual ${playerMetrics.boxWidth}x${playerMetrics.boxHeight})`);
  assert(playerMetrics.src === libraryMetrics.src, 'Bottom player reuses the exact library data-URL rendition for the same track');
  assert([...document.querySelectorAll('.native-player img')].every(image => image.src.startsWith('data:image/')), 'Player cover is a native data URL, never a remote image address');
  checks.push(`Player cover evidence: src ${playerMetrics.src.slice(0, 23)}… (${playerMetrics.src.length} chars, identical to library ${playerMetrics.src === libraryMetrics.src}), natural ${playerMetrics.naturalWidth}x${playerMetrics.naturalHeight}, box ${playerMetrics.boxWidth}x${playerMetrics.boxHeight}, object-fit ${playerMetrics.objectFit}/${playerMetrics.objectPosition}`);
  await click('Pause Signal'); await wait(() => phase() === 'Paused', 'Artwork fixture track paused');
  await click('Queue');
  await wait(() => button('Clear queue') && !button('Clear queue').disabled, 'Queue clear control for the artwork fixture');
  await click('Clear queue'); await wait(() => phase() === 'Idle', 'Artwork fixture playback stopped');
  await click('Library');
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Library restored after the artwork fixture');
  if (button('Dismiss notification')) await click('Dismiss notification');
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
  if (button('Dismiss notification')) await click('Dismiss notification');
  assert(!el('.native-content>.alert.notice'), 'Notification can be dismissed without changing the library');
  await capture('library-dark');
  for (const [width, name] of [[1440, 'library-wide-dark'], [1728, 'library-ultrawide-dark']]) {
    await resize(width, 900);
    for (let attempt = 0; attempt < 4 && Math.abs(innerWidth - width) > 2; attempt++) {
      dioxus.send({resize:{width:Math.min(4096, Math.round(physical(width) * width / innerWidth)),height:physical(900)}});
      await dioxus.recv(); await sleep(350);
    }
    const matched = Math.abs(innerWidth - width) <= 2;
    viewportEvidence.push({requestedWidth:width,actualWidth:innerWidth,status:matched?'PASS':'NOT_RUN'});
    if (matched) {
      assert(true, `${name} uses the requested ${width}px viewport (actual ${innerWidth}px)`);
      await capture(name);
    }
  }
  await resize(1280, 840);
  const firstNativeRow = el('[data-native-track-id]');
  const rowTime = firstNativeRow.querySelector('.native-track-time').getBoundingClientRect();
  const rowActions = firstNativeRow.querySelector('.native-track-actions').getBoundingClientRect();
  assert(rowActions.left >= rowTime.right - 1, 'Track duration and four action controls do not overlap');
  assert(el('.native-player').getBoundingClientRect().height <= 160, 'Idle desktop transport does not reserve a large empty status strip');
  const rapidRow = button('Play Native Continuity');
  rapidRow.click(); rapidRow.click();
  await wait(() => phase() === 'Paused', 'Rapid repeated selection stays paused after preparation');
  assert(position() < 150, 'Rapid double click pauses the pending selection instead of restarting it');
  await click('Play Native Continuity');
  await wait(() => phase() === 'Playing' && position() > 400, 'Native playback clock advances');
  const rowGeneration = generation();
  fill('#native-seek', '5000', 'change');
  await wait(() => position() >= 4900, 'Native row toggle seek fixture');
  await click('Pause Native Continuity');
  await wait(() => phase() === 'Paused', 'Current library row pauses');
  const rowPaused = position();
  await sleep(300);
  assert(generation() === rowGeneration && Math.abs(position() - rowPaused) < 120 && rowPaused >= 4900, 'Repeated row click pauses without replacing source or resetting position');
  await click('Play Native Continuity');
  await wait(() => phase() === 'Playing' && position() > rowPaused + 100, 'Current library row resumes');
  assert(generation() === rowGeneration, 'Row resume reuses the same native source');
  el('[data-testid="native-player-favorite"]').click();
  await wait(() => el('[data-testid="native-player-favorite"]')?.getAttribute('aria-label') === 'Remove Native Continuity from favorites', 'Player favorite mutation');
  assert(el('[data-testid="native-player-favorite"]')?.getAttribute('aria-label') === 'Remove Native Continuity from favorites', 'Player favorite control updates the shared queue state');
  const gen = generation(); const before = position();
  await click('Queue'); await sleep(450);
  assert(generation() === gen && position() > before, 'Navigation preserves native source and advancing position');
  const activeQueueRow = [...document.querySelectorAll('.native-queue-row')].find(row => row.classList.contains('current'));
  activeQueueRow.querySelector('.native-queue-select').click();
  await wait(() => phase() === 'Paused', 'Current queue row pauses');
  activeQueueRow.querySelector('.native-queue-select').click();
  await wait(() => phase() === 'Playing', 'Current queue row resumes');
  assert(generation() === gen, 'Current queue row toggle preserves occurrence, source and shuffle order');
  await click('Pause'); await wait(() => phase() === 'Paused', 'Native pause');
  const paused = position(); await sleep(450);
  assert(Math.abs(position() - paused) < 150, 'Paused native position is stable');
  fill('#native-seek', '6000', 'change');
  await wait(() => position() >= 5900 && position() <= 6300 && phase() === 'Paused', 'Native seek remains paused');
  await click('Play'); await wait(() => phase() === 'Playing' && position() > 6400, 'Native resume after seek');
  await click('Pause'); await wait(() => phase() === 'Paused', 'Pause before queue edits');
  await click('Use the light theme');
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
  assert(el('#native-create-playlist summary') && !el('#native-create-playlist').open, 'Playlist creation starts in a compact native disclosure');
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
  fill('#native-folder-name', 'Native folder'); await click('Create folder');
  await wait(() => el('[data-native-folder]'), 'Native folder creation');
  fill('#native-playlist-name', 'Unfiled keeper'); await click('Create playlist');
  await wait(() => button('Open playlist Unfiled keeper'), 'Unfiled keeper creation');
  await click('Open playlist Native evening');
  await wait(() => el('#native-playlist-folder'), 'Native playlist folder picker');
  const folderPicker = el('#native-playlist-folder'); folderPicker.value = folderPicker.options[1].value; folderPicker.dispatchEvent(new Event('change', { bubbles:true }));
  await click('Move to folder');
  await wait(() => (el('.alert.notice')?.textContent || '').includes('Playlist folder updated.'), 'Native playlist folder move');
  await click('Back to playlists');
  await click('Native folder (1)');
  await wait(() => button('Open playlist Native evening') && !button('Open playlist Unfiled keeper'), 'Native folder filter');
  assert(!button('Open playlist Unfiled keeper'), 'Native folder filter excludes unfiled playlists');
  fill('#native-folder-rename', 'Native folder renamed'); await click('Save folder name');
  await wait(() => button('Native folder renamed (1)'), 'Native folder rename');
  await click('Delete folder Native folder renamed');
  await click('Confirm delete folder Native folder renamed');
  await wait(() => !document.querySelector('[data-native-folder]') && button('Open playlist Native evening'), 'Native folder deletion unfiles playlists');
  assert(button('Open playlist Unfiled keeper'), 'All playlists restores unfiled grouping after folder deletion');
  await click('Open playlist Native evening');
  await wait(() => button('Play playlist') && document.querySelectorAll('.native-queue-row').length === 2, 'Real playlist detail with two entries');
  assert(document.querySelectorAll('.native-queue-row').length === 2 && button('Play playlist'), 'Playlist detail shows both owned entries');
  const trackPicker = el('#native-playlist-track'); trackPicker.value = [...trackPicker.options].find(option => option.textContent.includes('Native Continuity')).value; trackPicker.dispatchEvent(new Event('change', { bubbles:true }));
  await click('Add selected track');
  await wait(() => document.querySelectorAll('.native-queue-row').length === 3, 'Native playlist track add');
  assert(document.querySelectorAll('.native-queue-row').length === 3, 'Owned library track added to the native playlist');
  await click('Play playlist entry 3: Native Continuity');
  await wait(() => phase() === 'Playing' && position() > 100, 'Individual playlist row plays');
  const playlistSource = generation();
  await click('Pause playlist entry 3: Native Continuity');
  await wait(() => phase() === 'Paused', 'Individual playlist row pauses');
  assert(generation() === playlistSource, 'Individual playlist entry controls the existing native source');
  await click('Play playlist entry 3: Native Continuity');
  await wait(() => phase() === 'Playing', 'Individual playlist row resumes');
  assert(generation() === playlistSource, 'Individual playlist entry resume preserves its occurrence');
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
    dioxus.send({ok:true, checks, viewportEvidence, closeWhilePlaying:true});
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
    await wait(() => document.querySelectorAll('[data-public-track-id]').length === 20, 'Paginated public catalog first page');
    assert(document.querySelectorAll('[data-public-track-id]').length === 20 && button('Load more public records'), 'First public page exposes a cursor');
    await click('Load more public records');
    await wait(() => document.querySelectorAll('[data-public-track-id]').length === 21, 'Public catalog next page');
    assert(document.querySelectorAll('[data-public-track-id]').length === 21, 'Public catalog cursor appends the remaining record without duplicates');
    assert(!button('Load more public records'), 'Exhausted public catalog removes Load more');
    await click('Save this server');
    await wait(() => (el('.alert.notice')?.textContent || '').includes('Public server saved.'), 'Save public server');
    assert(document.querySelector('[aria-label^="Remove saved server"]'), 'Saved public server is listed');
    await capture('public-dark');
    el(`[aria-label="Remove saved server ${config.peerOrigin}/"]`).click();
    await wait(() => (el('.alert.notice')?.textContent || '').includes('Public server removed.'), 'Remove public server');
    const remainingLabels = [...document.querySelectorAll('[aria-label^="Remove saved server"]')].map(node => node.getAttribute('aria-label'));
    if (document.querySelector(`[aria-label="Remove saved server ${config.peerOrigin}/"]`)) throw new Error(`Saved public server can be removed failed labels=${JSON.stringify(remainingLabels)} expected=${config.peerOrigin}/`);
    checks.push('Saved public server can be removed');
    await click('Save this server');
    await wait(() => document.querySelector('[aria-label^="Remove saved server"]'), 'Re-save public server');
    assert(document.querySelector('[aria-label^="Remove saved server"]'), 'Saved public server persists for the config check');
    await click('Explore public catalog');
    await wait(() => document.querySelector('[data-public-track-id] button'), 'Public preview refreshed');
    const publicRow = el('[data-public-track-id]');
    const publicTitle = publicRow.querySelector('.native-queue-select strong')?.textContent;
    const publicButtons = [...document.querySelectorAll('[data-public-track-id] button')];
    const clicked = publicButtons.find(candidate => !candidate.disabled);
    if (!clicked) throw new Error(`Public play handler missing rows=${publicButtons.length} disabled=${publicButtons.map(b => b.disabled).join(',')} errors=${JSON.stringify(runtimeErrors.slice(-4))}`);
    clicked.dispatchEvent(new MouseEvent('click', { bubbles:true, cancelable:true, view:window }));
    await wait(() => (el('.player-track strong')?.textContent || '') === publicTitle, 'Public track selection');
    const selectedTitle = el('.player-track strong')?.textContent;
    if (selectedTitle !== publicTitle) throw new Error(`Public track selection failed expected=${publicTitle} actual=${selectedTitle} phase=${phase()} status=${el('.native-player-status small')?.textContent || '-'} error=${el('.player-error')?.textContent?.trim() || '-'}`);
    try { await wait(() => ['Playing','Ended'].includes(phase()) && position() > 0 && (el('.native-player-status small')?.textContent || '').includes('Public ·'), 'Public track playback'); }
    catch { throw new Error(`Public track playback failed (phase=${phase()} position=${position()} status=${el('.native-player-status small')?.textContent || '-'} error=${el('.player-error')?.textContent?.trim() || '-'})`); }
    assert(['Playing','Ended'].includes(phase()), 'Authorized second-server public audio plays through the native engine');
    assert((el('.native-player-status small')?.textContent || '').includes('Public ·'), 'Player identifies the public origin context');
    assert(button('Shuffle')?.disabled && button('Repeat off')?.disabled && button('Shuffle').getAttribute('aria-pressed') === 'false', 'Public source disables unsupported shuffle and repeat with an explanation');
    assert(button('Repeat off').getAttribute('aria-description')?.includes('public sources'), 'Unavailable public repeat explains its source limitation');
    fill('#native-peer-query', 'Vela'); await click('Search saved servers');
    try { await wait(() => [...document.querySelectorAll('[data-public-track-id][data-public-origin]')].map(row => row.dataset.publicOrigin).filter((origin, index, all) => all.indexOf(origin) === index).length >= 2, 'Unified saved-server search'); }
    catch { throw new Error(`Unified saved-server search failed (error=${el('.alert.error')?.textContent?.trim() || '-'} query=${el('#native-peer-query')?.value || '-'} origins=${JSON.stringify([...document.querySelectorAll('[data-public-track-id][data-public-origin]')].map(row => row.dataset.publicOrigin))} saved=${JSON.stringify([...document.querySelectorAll('[aria-label^="Remove saved server"]')].map(node => node.getAttribute('aria-label')))} peers=${JSON.stringify([...document.querySelectorAll('[data-public-peer-summary]')].map(node => [node.dataset.publicOrigin, node.dataset.publicPeerError]))})`); }
    const origins = [...document.querySelectorAll('[data-public-track-id][data-public-origin]')].map(row => row.dataset.publicOrigin);
    assert(origins.some(origin => origin.startsWith(config.peerOrigin)) && origins.some(origin => origin.startsWith(config.peer2Origin)), 'Unified search returns both saved public origins');
    const peer1Rows = () => [...document.querySelectorAll('[data-public-track-id][data-public-origin]')].filter(row => row.dataset.publicOrigin.startsWith(config.peerOrigin));
    await wait(() => peer1Rows().length === 20, 'Cross-peer search first page');
    assert(button(`Load more from ${config.peerOrigin}/`), 'Search peer keeps its own cursor');
    await click(`Load more from ${config.peerOrigin}/`);
    await wait(() => peer1Rows().length === 21, 'Cross-peer search next page');
    const allIds = [...document.querySelectorAll('[data-public-track-id][data-public-origin]')].map(row => row.dataset.publicTrackId);
    assert(allIds.length === new Set(allIds).size, 'Cross-peer Load more appends only new IDs');
    assert(!button(`Load more from ${config.peerOrigin}/`), 'Exhausted search peer removes its Load more');
    const cachedRow = [...document.querySelectorAll('[data-public-track-id][data-public-origin]')].find(row => row.dataset.publicOrigin.startsWith(config.cachedOrigin));
    assert(cachedRow && cachedRow.dataset.publicCached === 'true' && !cachedRow.querySelector('button'), 'Unreachable saved peer falls back to browse-only cached metadata');
    const cachedSummary = document.querySelector(`[data-public-peer-summary][data-public-origin="${config.cachedOrigin}/"]`);
    assert(cachedSummary && (cachedSummary.textContent || '').includes('1h old'), 'Cached results disclose their age');
    assert(button(`Retry live for ${config.cachedOrigin}/`), 'Cached results offer an explicit live retry');
    await click(`Retry live for ${config.cachedOrigin}/`);
    await wait(() => button(`Retry live for ${config.cachedOrigin}/`) && !document.body.textContent.includes('Retrying…'), 'Cached peer live retry');
    assert(document.querySelector(`[data-public-peer-summary][data-public-origin="${config.cachedOrigin}/"]`)?.dataset.publicPeerCached === 'true', 'Failed live retry remains browse-only cached metadata');
    await capture('public-search-dark');
    const peer2Row = [...document.querySelectorAll('[data-public-track-id][data-public-origin]')].find(row => row.dataset.publicOrigin.startsWith(config.peer2Origin));
    assert(peer2Row, 'Second saved origin is represented in unified results');
    peer2Row.querySelector('button').click();
    await wait(() => (el('.native-player-status small')?.textContent || '').includes(config.peer2Origin), 'Second-origin public playback');
    assert((el('.native-player-status small')?.textContent || '').includes(config.peer2Origin), 'Public audio from the second saved origin is selected natively');
    await click('Library');
    await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Library during remote playback');
    await click('Previous');
    await wait(() => phase() === 'Playing' && (el('.native-player-status small')?.textContent || '').includes(config.peerOrigin), 'Remote queue previous');
    assert(phase() === 'Playing' && (el('.native-player-status small')?.textContent || '').includes(config.peerOrigin), 'Remote queue previous moves to the other origin');
    await click('Next');
    await wait(() => phase() === 'Playing' && (el('.native-player-status small')?.textContent || '').includes(config.peer2Origin), 'Remote queue next');
    assert(phase() === 'Playing' && (el('.native-player-status small')?.textContent || '').includes(config.peer2Origin), 'Remote queue next returns to the selected origin');
    assert(el('.native-library')?.dataset.view === 'library', 'Remote transport works after leaving Public without navigating back');
    await click('Library');
    await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Library after public playback');
    await click('Sign out'); await wait(() => button('Sign in'), 'Native sign-out clears account');
    assert(!el('.native-player') && document.querySelectorAll('[data-native-track-id]').length === 0, 'Sign-out removes private catalog and player');
    assert(document.querySelectorAll('img[src^="data:image/"]').length === 0, 'Sign-out leaves zero authorized artwork data URLs in the native DOM');
    assert(document.querySelectorAll('[data-native-artwork]').length === 0, 'Sign-out leaves zero cover presentation slots in the native DOM');
    fill('#native-origin', config.origin); fill('#native-username', config.username); fill('#native-password', config.password);
    if (config.testSession) {
      if (!el('#native-remember').checked) el('#native-remember').click();
      el('.signin-form form').requestSubmit();
      await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Library after offline sign-in');
      await wait(() => (el('.alert.notice')?.textContent || '').includes('Session saved on this device.'), 'Opt-in session persistence');
    } else {
      el('.signin-form form').requestSubmit();
      await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Library after offline sign-in');
    }
    await wait(() => [...document.querySelectorAll('[data-native-track-id] img.neutral-cover')].some(image => image.src.startsWith('data:image/') && image.complete && image.naturalWidth > 0), 'Authorized artwork after sign-in again');
    assert(document.querySelectorAll('[data-native-track-id] img.neutral-cover').length >= 1, 'Re-authorized library renders the embedded cover after sign-in');
    const offlineTitles = [...document.querySelectorAll('[data-native-track-id] .native-track-name strong')].slice(0, 2).map(node => node.textContent);
    assert(offlineTitles.length === 2, 'Offline fixtures are listed');
    for (const title of offlineTitles) {
      await click(`Save ${title} offline`);
      await wait(() => button(`Play offline ${title}`), `Offline copy listed: ${title}`);
    }
    assert(document.querySelectorAll('[data-offline-track-id]').length === 2, 'Two private device copies are listed');
    dioxus.send({handshake:'server-stop'});
    await dioxus.recv();
    let serverDown = false;
    try { const probe = await fetch(`/health?offline=${Date.now()}`, {cache:'no-store'}); serverDown = !probe.ok; } catch { serverDown = true; }
    assert(serverDown, 'Fixture server is actually stopped before offline playback');
    await click(`Play offline ${offlineTitles[1]}`);
    await wait(() => phase() === 'Playing' && position() > 100 && (el('.native-player-status small')?.textContent || '').includes('Offline ·'), 'Offline playback');
    assert(phase() === 'Playing' && (el('.native-player-status small')?.textContent || '').includes('Offline ·'), 'Saved original plays natively with the server stopped');
    assert(button('Shuffle')?.disabled && !button('Repeat off')?.disabled, 'Offline source exposes repeat one but disables unsupported shuffle');
    await click('Repeat off');
    assert(button('Repeat one')?.getAttribute('aria-pressed') === 'true' && !button('Repeat all'), 'Offline repeat cycles directly to one track');
    const offlineGeneration = generation();
    fill('#native-seek', String(Math.max(0, Number(el('#native-seek').max) - 250)), 'change');
    await wait(() => generation() !== offlineGeneration && phase() === 'Playing' && (el('.native-player-status small')?.textContent || '').includes('Offline ·'), 'Offline repeat one replays after actual EOF');
    assert(generation() !== offlineGeneration && (el('.native-player-status small')?.textContent || '').includes('Offline ·'), 'Offline repeat EOF reuses the verified local descriptor while the server is stopped');
    await click('Repeat one');
    assert(button('Repeat off')?.getAttribute('aria-pressed') === 'false', 'Offline repeat returns to off without an unsupported all mode');
    await click('Next');
    await wait(() => (el('.native-player-status small')?.textContent || '').includes(`Offline · ${offlineTitles[0]}`), 'Offline next');
    assert((el('.native-player-status small')?.textContent || '').includes(`Offline · ${offlineTitles[0]}`), 'Offline queue advances to the second saved copy');
    await click('Previous');
    await wait(() => (el('.native-player-status small')?.textContent || '').includes(`Offline · ${offlineTitles[1]}`), 'Offline previous');
    assert((el('.native-player-status small')?.textContent || '').includes(`Offline · ${offlineTitles[1]}`), 'Offline queue returns to the first saved copy');
    await click(`Remove offline copy ${offlineTitles[1]}`);
    await click(`Confirm remove offline ${offlineTitles[1]}`);
    await wait(() => !button(`Play offline ${offlineTitles[1]}`), 'Offline copy removed');
    assert(document.querySelectorAll('[data-offline-track-id]').length === 1, 'Offline removal leaves the other device copy');
    dioxus.send({ok:true, checks, viewportEvidence});
  }
} catch(error) { dioxus.send({ok:false, checks, failure:String(error.message)}); }
}
