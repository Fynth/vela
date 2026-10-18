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
  await resize(1280, 840);
  assert(document.querySelectorAll('audio,iframe').length === 0, 'No HTML media or iframe playback substitute');
  fill('#native-volume', '0'); await sleep(400);
  assert(Number(el('#native-volume').value) === 0, 'Native volume acknowledged zero before playback');
  await capture('library-dark');
  await click('Play Native Continuity');
  await wait(() => phase() === 'Playing' && position() > 400, 'Native playback clock advances');
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
  await wait(() => button('Play Beacon'), 'MP3 real search');
  await click('Play Beacon'); await wait(() => phase() === 'Playing' && position() > 100, 'Real MP3 native decoder advances');
  if (config.closeWhilePlaying) {
    await click('Pause');
    fill('#native-search', 'Native Continuity'); el('.native-search').requestSubmit();
    await wait(() => button('Play Native Continuity'), 'Long track for close during playback');
    await click('Play Native Continuity');
    await wait(() => phase() === 'Playing' && position() > 300, 'Close while real audio is advancing');
    dioxus.send({ok:true, checks, closeWhilePlaying:true});
  } else {
    await click('Sign out'); await wait(() => button('Sign in'), 'Native sign-out clears account');
    assert(!el('.native-player') && document.querySelectorAll('[data-native-track-id]').length === 0, 'Sign-out removes private catalog and player');
    dioxus.send({ok:true, checks});
  }
} catch(error) { dioxus.send({ok:false, checks, failure:String(error.message)}); }
