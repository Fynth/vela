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
const near = (actual, expected, tolerance = 1) => Number.isFinite(actual) && Math.abs(actual - expected) <= tolerance;
const box = selector => el(selector)?.getBoundingClientRect();
const cssToken = (node, name) => getComputedStyle(node).getPropertyValue(name).trim();
const cssPx = (node, name) => parseFloat(getComputedStyle(node)[name]) || 0;
const fontFamilyOf = node => getComputedStyle(node).fontFamily.replace(/["']/g, '').replace(/\s+/g, ' ').trim();
const fontFaceList = () => {
  const faces = [];
  if (typeof document.fonts.forEach === 'function') document.fonts.forEach(face => faces.push(face));
  else for (const face of document.fonts) faces.push(face);
  return faces;
};
const compactDestinations = ['Library', 'Albums', 'Artists', 'Favorites', 'Playlists', 'Public', 'Queue', 'Settings'];
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
// TR10 typography evidence. This reads the real FontFaceSet, the packaged CSSOM and
// the actual renderer. It never installs a font, overrides the family or trusts the
// stylesheet's own claim: both subsets must be loaded faces and the Cyrillic sample
// must lay out with Manrope instead of the fallback metrics.
const fontEvidence = async () => {
  const probe = document.createElement('div');
  probe.style.cssText = 'position:fixed;left:-20000px;top:0;visibility:hidden;white-space:nowrap;font-size:16px;font-weight:600';
  const sample = document.createElement('span');
  sample.textContent = 'Привет, мир! Ёё № «Vela»';
  const manrope = document.createElement('span');
  manrope.style.fontFamily = 'Manrope,sans-serif';
  manrope.textContent = sample.textContent;
  const system = document.createElement('span');
  system.style.fontFamily = 'sans-serif';
  system.textContent = sample.textContent;
  probe.append(sample, manrope, system);
  el('.native-app').append(probe);
  try {
    const latin = await Promise.race([document.fonts.load('600 16px Manrope', 'Vela Continuity 0123456789'), sleep(5000)]);
    const cyrillic = await Promise.race([document.fonts.load('600 16px Manrope', sample.textContent), sleep(5000)]);
    const faces = fontFaceList().map(face => ({
      family: String(face.family || '').replace(/["']/g, '').trim(),
      weight: String(face.weight || ''), status: String(face.status || ''),
    }));
    const rules = [];
    for (const sheet of document.styleSheets) {
      let cssRules = null;
      try { cssRules = sheet.cssRules; } catch { continue; }
      if (!cssRules) continue;
      for (const rule of cssRules) {
        if (rule.type !== CSSRule.FONT_FACE_RULE) continue;
        rules.push({
          family: String(rule.style.getPropertyValue('font-family') || '').replace(/["']/g, '').trim(),
          weight: String(rule.style.getPropertyValue('font-weight') || '').trim(),
          display: String(rule.style.getPropertyValue('font-display') || '').trim(),
          sources: String(rule.style.getPropertyValue('src') || '').trim(),
          range: String(rule.style.getPropertyValue('unicode-range') || '').trim(),
          text: String(rule.cssText || ''),
        });
      }
    }
    return {
      rootFamily: fontFamilyOf(document.documentElement),
      appFamily: fontFamilyOf(el('.native-app')),
      sampleFamily: fontFamilyOf(sample),
      sampleText: sample.textContent,
      sampleWidth: sample.getBoundingClientRect().width,
      manropeWidth: manrope.getBoundingClientRect().width,
      systemWidth: system.getBoundingClientRect().width,
      latinLoaded: Array.isArray(latin) ? latin.length : -1,
      cyrillicLoaded: Array.isArray(cyrillic) ? cyrillic.length : -1,
      latinCheck: document.fonts.check('600 16px Manrope', 'Vela Continuity 0123456789'),
      cyrillicCheck: document.fonts.check('600 16px Manrope', sample.textContent),
      fontStatus: document.fonts.status,
      faces, rules,
      networkFonts: performance.getEntriesByType('resource').map(entry => entry.name).filter(name => /\.woff2?($|[?#])/i.test(name)),
    };
  } finally { probe.remove(); }
};
// TR10 single-engine evidence: ask the Rust driver (same process as the real window)
// to count the native output streams it owns on the QA's dedicated Pulse/PipeWire sink.
// One streaming client means one AudioEngine output authority; no DOM guess is used.
const engineStreams = async () => {
  dioxus.send({engineStreams:true});
  const reply = await dioxus.recv();
  return reply?.engineStreams || null;
};
const waitForEngineStreams = async () => {
  const end = performance.now() + 12000;
  let evidence = null;
  while (performance.now() < end) {
    evidence = await engineStreams();
    if (evidence && Number(evidence.sinkInputs) >= 1) return evidence;
    await sleep(150);
  }
  throw new Error(`The native AudioEngine opened no output stream (last ${JSON.stringify(evidence)})`);
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
    // AUD-05: a cold start whose private offline profile names the saved session's
    // origin attempts that session once by itself, so this phase may land on the
    // private account directly. The explicit Continue action stays available (and is
    // used) whenever the startup did not already resume it; both paths exercise the
    // same stored token and the same guarded resume routine.
    await wait(() => button('Continue saved session') || button('Sign out') || el('[data-testid="native-offline-library"]'), 'Saved-session startup decision');
    assert(!el('[data-testid="native-offline-library"]'), 'A reachable server resumes or offers the saved session instead of the offline library');
    const automatic = !button('Continue saved session');
    if (!automatic) await click('Continue saved session');
    await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Saved session resume');
    checks.push(automatic ? 'Saved session resumed from the private offline profile at startup' : 'Saved session resumed through the explicit Continue action');
    assert(button('Sign out'), 'Saved session resumes the private account without a password');
    if (config.testSession) {
      await wait(() => (el('[data-offline-track-id]')?.textContent || '').includes('Integrity check failed'), 'Offline startup integrity sweep');
      assert(el('[data-offline-track-id] button')?.disabled, 'Corrupt offline copy disables Play offline');
    }
    await click('Sign out'); await wait(() => button('Sign in'), 'Resume sign-out');
    assert(!el('.native-player') && !document.querySelector('[data-native-track-id]'), 'Resume sign-out clears private state');
    // AUD-05: an explicit sign-out withdraws the local offline hint immediately in the
    // UI and (off the UI thread) on this device; the fixture asserts the file too, so
    // this window gives the bounded local withdrawal a moment before it closes.
    assert(!button('Open offline library'), 'Explicit sign-out withdraws the local offline library entry');
    await sleep(700);
    dioxus.send({ok:true, checks, viewportEvidence});
  } catch(error) { dioxus.send({ok:false, checks, failure:String(error.message)}); }
} else if (config.sessionPhase === 'offline') {
  // AUD-05/REG-AUD-05 cold offline launch: the fixture server was stopped by the
  // previous run, and the private offline profile plus its real saved copies are on
  // disk. This window must open a truthful local library, play the saved original
  // through the one native AudioEngine, and never invent an account or a request.
  try {
    await wait(() => el('[data-testid="native-offline-library"]'), 'Cold-start offline library');
    assert(!el('.native-signin'), 'No reachable server: the sign-in form is not the landing screen');
    assert(button('Sign in to Vela'), 'The offline library offers a real sign-in instead of pretending to be signed in');
    const surface = el('[data-testid="native-offline-library"]');
    assert((surface.dataset.offlineOrigin || '') === config.origin + '/', `The offline library names the canonical origin it belongs to (actual "${surface.dataset.offlineOrigin}")`);
    const rows = [...document.querySelectorAll('[data-offline-track-id]')];
    assert(rows.length >= 1, 'The offline library lists the real saved copies on this device');
    assert((surface.dataset.offlineCount || '0') === String(rows.length), `The offline library discloses its exact copy count (${surface.dataset.offlineCount} vs ${rows.length})`);
    assert(document.querySelectorAll('[data-native-track-id]').length === 0, 'The offline library invents no server catalog rows');
    assert(document.querySelectorAll('img[src^="data:image/"]').length === 0, 'No authorized artwork data URL exists without a session');
    // Read/play only: nothing that needs a session is offered.
    assert(!el('#native-search'), 'The offline library exposes no library search');
    assert(!button('Refresh library') && !button('Sign out') && !button('Favorites') && !button('Playlists'),
      'The offline library exposes no account navigation or sign-out');
    assert(document.querySelectorAll('[aria-label^="Remove offline copy"]').length === 0, 'The offline library cannot remove a copy');
    assert(!el('[data-testid="native-player-favorite"]'), 'The offline library exposes no favorite mutation');
    assert(!document.querySelector('[data-testid="native-player-queue"]'), 'The offline library exposes no server queue view');
    const title = (rows[0].querySelector('.native-queue-select strong')?.textContent || '').trim();
    assert(title.length > 0, 'Offline rows carry their real saved title');
    assert(!button(`Play ${title}`), 'The saved copy is played only through the offline path, never as a server original');
    assert(button('Shuffle')?.disabled && !button('Repeat off')?.disabled, 'The offline screen exposes repeat one but disables the shuffle it cannot execute');
    await capture('offline-dark');
    await click(`Play offline ${title}`);
    await wait(() => phase() === 'Playing' && position() > 200, 'Cold-start offline playback advances the native clock');
    const source = generation();
    assert(position() > 0 && source !== null && source !== undefined, `Cold-start offline playback reports a real native source (generation ${source}, position ${position()}ms)`);
    const engine = await waitForEngineStreams();
    assert(document.querySelectorAll('.native-player').length === 1
      && document.querySelectorAll('[data-audio-generation]').length === 1
      && document.querySelectorAll('[data-audio-phase]').length === 1,
      `Exactly one native player/engine surface exists offline (players ${document.querySelectorAll('.native-player').length}, generation surfaces ${document.querySelectorAll('[data-audio-generation]').length})`);
    assert(document.querySelectorAll('audio,video,iframe').length === 0, 'The cold-start window renders no HTML media/iframe audio substitute');
    assert(Number(engine.sinkInputs) === 1,
      `Exactly one native output stream exists on the QA-owned sink (sink "${engine.sink}", streams ${engine.sinkInputs}, process id ${engine.pid})`);
    assert(engine.processStreams === null || Number(engine.processStreams) === 1,
      `Every stream on the QA-owned sink belongs to this window process (process-matched ${engine.processStreams}, sink total ${engine.sinkInputs})`);
    assert((el('.native-player-status small')?.textContent || '').includes(`Offline · ${title}`), 'The player states the offline source context');
    await click(`Pause offline ${title}`);
    await wait(() => phase() === 'Paused', 'Cold-start offline pause');
    const paused = position(); await sleep(300);
    assert(generation() === source && Math.abs(position() - paused) < 200, 'Pausing offline keeps the same native source and position');
    await click(`Play offline ${title}`);
    await wait(() => phase() === 'Playing' && position() > paused + 100, 'Cold-start offline resume');
    assert(generation() === source, 'Resuming offline retains the single native source');
    checks.push(`REG-AUD-05 cold offline launch: ${rows.length} real saved cop(ies) for ${surface.dataset.offlineOrigin}, no sign-in, no catalog rows, played "${title}" to ${position()}ms on one native source (${engine.sinkInputs} output stream, generation ${source})`);
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
  // TR10 typography: the approved Manrope subsets are compiled into the packaged
  // stylesheet as self-contained data URIs. This runs in the real WebKitGTK window:
  // no font is installed or fetched here, and the Cyrillic sample must actually lay
  // out with Manrope's metrics instead of the system fallback.
  await Promise.race([document.fonts.ready, sleep(5000)]);
  const fonts = await fontEvidence();
  assert(fonts.fontStatus === 'loaded', `Document fonts settle on loaded (actual ${fonts.fontStatus})`);
  assert(fonts.rootFamily.startsWith('Manrope') && /Inter/.test(fonts.rootFamily) && /ui-sans-serif/.test(fonts.rootFamily) && /sans-serif/.test(fonts.rootFamily),
    `Root UI font stack starts with Manrope and keeps the existing system fallbacks (actual "${fonts.rootFamily}")`);
  assert(fonts.appFamily === fonts.rootFamily, `The native app wrapper inherits the same Manrope stack (actual "${fonts.appFamily}")`);
  assert(fonts.faces.length === 2 && fonts.faces.every(face => face.family === 'Manrope' && face.status === 'loaded'),
    `Both Manrope subsets are real loaded FontFace entries (faces ${JSON.stringify(fonts.faces)})`);
  assert(fonts.latinLoaded >= 1 && fonts.cyrillicLoaded >= 1 && fonts.latinCheck && fonts.cyrillicCheck,
    `document.fonts.load/check report both the Latin and the Cyrillic subset loaded (latin ${fonts.latinLoaded}/${fonts.latinCheck}, cyrillic ${fonts.cyrillicLoaded}/${fonts.cyrillicCheck})`);
  assert(fonts.rules.length === 2 && fonts.rules.every(rule => rule.family === 'Manrope' && rule.weight === '200 800' && rule.display === 'swap'),
    `Exactly two embedded Manrope @font-face rules carry weight 200 800 and font-display swap (rules ${JSON.stringify(fonts.rules.map(rule => [rule.family, rule.weight, rule.display]))})`);
  assert(fonts.rules.every(rule => /^url\("data:font\/woff2;base64,[A-Za-z0-9+/=]+"\) format\("woff2"\)$/.test(rule.sources)),
    `Every Manrope source is one self-contained woff2 data URI, with no second font source (sources ${fonts.rules.map(rule => rule.sources.slice(0, 34)).join(' | ')})`);
  const latinFaces = fonts.rules.filter(rule => /U\+0-FF/i.test(rule.range));
  const cyrillicFaces = fonts.rules.filter(rule => /U\+400-45F/i.test(rule.range));
  assert(latinFaces.length === 1 && cyrillicFaces.length === 1 && latinFaces[0] !== cyrillicFaces[0],
    `The embedded faces keep distinct Latin and Cyrillic unicode ranges (${fonts.rules.map(rule => rule.range.slice(0, 42)).join(' | ')})`);
  assert(fonts.networkFonts.length === 0, `No font was requested over the network (requests ${JSON.stringify(fonts.networkFonts)})`);
  assert(fonts.sampleFamily.startsWith('Manrope') && fonts.sampleWidth > 0 && fonts.sampleWidth === fonts.manropeWidth,
    `The Cyrillic sample renders inside the inherited Manrope stack (family "${fonts.sampleFamily}", width ${fonts.sampleWidth.toFixed(2)})`);
  assert(Math.abs(fonts.manropeWidth - fonts.systemWidth) > 0.5,
    `The Cyrillic sample really uses Manrope metrics rather than the system fallback (Manrope ${fonts.manropeWidth.toFixed(2)}px vs sans-serif ${fonts.systemWidth.toFixed(2)}px)`);
  checks.push(`TR10 typography: root "${fonts.rootFamily}"; 2 loaded data-URI faces (weight 200 800, swap); Latin+Cyrillic fonts.check true; Cyrillic sample ${fonts.sampleText.length} chars "${fonts.sampleText}" in family "${fonts.sampleFamily}" at ${fonts.sampleWidth.toFixed(2)}px vs fallback ${fonts.systemWidth.toFixed(2)}px; network font requests ${fonts.networkFonts.length}`);
  await wait(() => button('Sign in') && !button('Sign in').disabled, 'Native output initialization');
  fill('#native-origin', config.origin); fill('#native-username', config.username); fill('#native-password', config.password);
  el('.signin-form form').requestSubmit();
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Three real private uploaded tracks');
  // TR10 window chrome: Linux renders no GTK titlebar, so the window actions and the
  // drag region are real in-app controls; there must be exactly one accessible group.
  const chrome = el('.native-window-controls');
  const chromeButtons = chrome ? [...chrome.querySelectorAll('button')] : [];
  const dragRegion = el('.native-window-drag');
  assert(document.querySelectorAll('.native-window-controls').length === 1 && chrome?.getAttribute('role') === 'group'
    && chrome?.getAttribute('aria-label') === 'Window controls' && chromeButtons.length === 3,
    `The frameless window keeps exactly one accessible window-control group (groups ${document.querySelectorAll('.native-window-controls').length}, buttons ${chromeButtons.length})`);
  assert(JSON.stringify(chromeButtons.map(node => node.getAttribute('aria-label'))) === JSON.stringify(['Minimize window', 'Maximize or restore window', 'Close window']),
    `Window controls expose the real minimize/maximize/close actions (${chromeButtons.map(node => node.getAttribute('aria-label')).join('/')})`);
  assert(chromeButtons.every(node => { const rect = node.getBoundingClientRect(); return rect.width >= 44 && rect.height >= 44 && rect.left >= 0 && rect.right <= innerWidth + 1; }),
    `Window controls keep 44px hit boxes inside the ${innerWidth}px window (${chromeButtons.map(node => `${Math.round(node.getBoundingClientRect().width)}x${Math.round(node.getBoundingClientRect().height)}`).join(',')})`);
  assert(dragRegion && dragRegion.getBoundingClientRect().width > 0 && getComputedStyle(dragRegion).cursor === 'grab',
    'The frameless topbar keeps one real drag region');
  checks.push(`TR10 window chrome: one frameless window-control group ${chromeButtons.map(node => node.getAttribute('aria-label')).join('/')}, drag region ${Math.round(dragRegion?.getBoundingClientRect().width || 0)}px, viewport ${innerWidth}x${innerHeight}, decorations=false`);
  // TR10 single AudioEngine: the window must own exactly one native output authority.
  // The DOM exposes one player/engine surface and no HTML media substitute, and the
  // QA-owned Pulse/PipeWire sink (the only sink this isolated session routes to) must
  // carry exactly one output stream, produced by this real window process.
  const engine = await waitForEngineStreams();
  assert(document.querySelectorAll('.native-player').length === 1
    && document.querySelectorAll('[data-audio-generation]').length === 1
    && document.querySelectorAll('[data-audio-phase]').length === 1,
    `Exactly one native player/engine surface exists (players ${document.querySelectorAll('.native-player').length}, generation surfaces ${document.querySelectorAll('[data-audio-generation]').length})`);
  assert(document.querySelectorAll('audio,video,iframe').length === 0, 'The native window renders no HTML media/iframe audio substitute');
  assert(Number(engine.sinkInputs) === 1,
    `Exactly one native output stream exists on the QA-owned sink (sink "${engine.sink}", streams ${engine.sinkInputs}, process id ${engine.pid})`);
  assert(engine.processStreams === null || Number(engine.processStreams) === 1,
    `Every stream on the QA-owned sink belongs to this window process (process-matched ${engine.processStreams}, sink total ${engine.sinkInputs})`);
  checks.push(`TR10 single AudioEngine: one player band, one audio-generation surface, zero HTML media elements; ${engine.sinkInputs} output stream on the QA-owned sink "${engine.sink}" (process id ${engine.pid}, process-matched ${engine.processStreams === null ? 'not exposed by the server' : engine.processStreams})`);
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
  // IN01: the pending private-library page renders bounded skeleton rows instead of
  // the old text-only indicator. The response is delayed relative to the DOM update,
  // so the pending skeleton state is observable before the settled rows replace it.
  const skeletons = () => [...document.querySelectorAll('[data-native-skeleton-row]')];
  const pendingSkeletons = [];
  const skeletonObserver = new MutationObserver(() => {
    const libraryView = el('.native-library');
    if (!libraryView || libraryView.dataset.loading !== 'true' || pendingSkeletons.length >= 24) return;
    const rows = skeletons();
    if (!rows.length) return;
    const table = rows[0].closest('[data-native-skeleton]');
    const row = rows[0].getBoundingClientRect();
    const art = rows[0].querySelector('.neutral-cover')?.getBoundingClientRect();
    pendingSkeletons.push({
      at: performance.now(), count: rows.length, declared: table?.dataset.nativeSkeleton || '',
      busy: (table?.getAttribute('aria-busy') || '') === 'true' && (libraryView.getAttribute('aria-busy') || '') === 'true',
      status: (el('.native-skeleton-status')?.textContent || '').trim(),
      text: rows.map(node => node.textContent.trim()).join(''),
      controls: rows.reduce((total, node) => total + node.querySelectorAll('button,input,a,img').length, 0),
      columns: getComputedStyle(rows[0]).gridTemplateColumns,
      row: {x: row.x, y: row.y, width: row.width, height: row.height},
      art: art ? {width: art.width, height: art.height} : null
    });
  });
  skeletonObserver.observe(document.body, {childList: true, subtree: true, attributes: true, attributeFilter: ['data-loading', 'aria-busy']});
  if (!el('[data-native-track-id]')) throw new Error('Skeleton fixture needs one settled library row first');
  fill('#native-search', 'Signal'); el('.native-search').requestSubmit();
  try { await wait(() => pendingSkeletons.length > 0, 'Skeleton rows during the pending library request'); }
  catch { throw new Error(`Skeleton rows during the pending library request failed rows=${skeletons().length} loading=${el('.native-library')?.dataset.loading} dataset=${JSON.stringify(el('.native-library')?.dataset)}`); }
  const pendingSkeleton = pendingSkeletons[0];
  const settledColumns = getComputedStyle(el('[data-native-track-id]')).gridTemplateColumns;
  assert(pendingSkeleton.count === 8 && pendingSkeleton.declared === '8', `Pending library request shows the bounded eight-row skeleton list (actual ${pendingSkeleton.count} rows, declared ${pendingSkeleton.declared || '-'})`);
  assert(pendingSkeleton.busy, 'Pending library region and skeleton table are marked aria-busy while the request is unanswered');
  assert(pendingSkeleton.status.includes('Loading your library'), `Skeleton keeps the honest library status label (actual "${pendingSkeleton.status}")`);
  assert(pendingSkeleton.text === '', 'Skeleton rows carry no placeholder title, artist, album, duration or percentage');
  assert(pendingSkeleton.controls === 0, `Skeleton rows render no controls and no artwork request slot (actual ${pendingSkeleton.controls})`);
  assert(pendingSkeleton.art && pendingSkeleton.art.width > 0 && Math.abs(pendingSkeleton.art.width - pendingSkeleton.art.height) < 1, `Skeleton artwork block is square (actual ${pendingSkeleton.art ? `${pendingSkeleton.art.width}x${pendingSkeleton.art.height}` : 'missing'})`);
  assert(pendingSkeleton.columns === settledColumns, `Skeleton rows use the settled track grid (skeleton ${pendingSkeleton.columns}, settled ${settledColumns})`);
  await wait(() => skeletons().length === 0 && document.querySelectorAll('[data-native-track-id]').length === 1 && button('Play Signal'), 'Settled rows replace the skeleton list');
  const settledRow = el('[data-native-track-id]').getBoundingClientRect();
  assert(Math.abs(settledRow.y - pendingSkeleton.row.y) <= 1 && Math.abs(settledRow.height - pendingSkeleton.row.height) <= 1 && Math.abs(settledRow.width - pendingSkeleton.row.width) <= 1, `Settled row replaces its skeleton without a layout jump (skeleton y=${pendingSkeleton.row.y.toFixed(1)} ${pendingSkeleton.row.width.toFixed(1)}x${pendingSkeleton.row.height.toFixed(1)}, settled y=${settledRow.y.toFixed(1)} ${settledRow.width.toFixed(1)}x${settledRow.height.toFixed(1)})`);
  const settledArt = el('[data-native-track-id] .neutral-cover')?.getBoundingClientRect();
  assert(settledArt && Math.abs(settledArt.width - pendingSkeleton.art.width) < 1 && Math.abs(settledArt.height - pendingSkeleton.art.height) < 1, 'Skeleton artwork block occupies the settled artwork box');
  assert(!el('.native-skeleton-status'), 'Settled library removes the pending status region');
  skeletonObserver.disconnect();
  checks.push(`IN01 skeleton evidence: ${pendingSkeletons.length} pending observation(s) over ${(performance.now() - pendingSkeleton.at).toFixed(0)}ms, ${pendingSkeleton.count} rows (declared ${pendingSkeleton.declared}), artwork ${pendingSkeleton.art.width}x${pendingSkeleton.art.height}, grid ${pendingSkeleton.columns}, skeleton y ${pendingSkeleton.row.y.toFixed(1)} vs settled y ${settledRow.y.toFixed(1)}, status "${pendingSkeleton.status}"`);
  fill('#native-search', ''); el('.native-search').requestSubmit();
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Library restored after the skeleton fixture');
  await click('Play Signal');
  await wait(() => (el('.player-track strong')?.textContent || '') === 'Signal' && phase() === 'Playing' && position() > 100, 'Signal playback for the player cover');
  // TR03: the band states the real artist and album of the tagged FLAC fixture.
  assert((el('.player-artist')?.textContent || '') === 'Vela Test' && (el('.player-album')?.textContent || '') === 'Real Fixtures'
    && cssPx(el('.player-artist'), 'fontSize') === 10,
    `TR03 band states the real artist and album of the tagged FLAC at the approved 10px role (actual "${(el('.player-artist')?.textContent || '')}" / "${(el('.player-album')?.textContent || '')}", ${cssPx(el('.player-artist'), 'fontSize')}px)`);
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
  // TR03: the persistent band carries the approved 48px authorized artwork box.
  assert(near(playerMetrics.boxWidth, 48) && near(playerMetrics.boxHeight, 48), `TR03 player artwork keeps the approved 48px box (actual ${playerMetrics.boxWidth}x${playerMetrics.boxHeight})`);
  await click('Pause Signal'); await wait(() => phase() === 'Paused', 'Artwork fixture track paused');
  await click('Queue');
  await wait(() => button('Clear queue') && !button('Clear queue').disabled, 'Queue clear control for the artwork fixture');
  await click('Clear queue'); await wait(() => phase() === 'Idle', 'Artwork fixture playback stopped');
  assert(!el('.native-player .player-source'), 'TR03 player shows no source line when nothing is selected');
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
  assert(!el('.native-scroll-inner>.alert.notice'), 'Notification can be dismissed without changing the library');
  // TR01/TR02: namespaced roles plus the one-offset, one-reserve desktop shell.
  const app = el('.native-app'), main = el('.native-main'), sidebar = el('.native-sidebar');
  const scroll = el('.native-scroll'), player = el('.native-player');
  const sidebarRect = box('.native-sidebar'), mainRect = box('.native-main');
  const topbarRect = box('.native-topbar'), scrollRect = box('.native-scroll'), playerRect = box('.native-player');
  assert(app?.dataset.theme === 'dark', 'Baseline shell geometry is the shipped dark theme');
  assert(sidebarRect && near(sidebarRect.width, 216), `Desktop sidebar is the approved 216px (actual ${sidebarRect?.width})`);
  assert(topbarRect && near(topbarRect.height, 75), `Desktop topbar is the approved 75px (actual ${topbarRect?.height})`);
  assert(playerRect && near(playerRect.height, 88), `Desktop player is the approved 88px (actual ${playerRect?.height})`);
  assert(playerRect && near(playerRect.left, 0) && near(playerRect.width, innerWidth), `Player spans the full window (${playerRect?.left}..${playerRect?.right} of ${innerWidth})`);
  assert(sidebarRect && mainRect && topbarRect && scrollRect
    && near(mainRect.left, sidebarRect.right) && near(topbarRect.left, mainRect.left) && near(scrollRect.left, mainRect.left),
    `Sidebar offset is applied exactly once (sidebar right ${sidebarRect?.right}, main/topbar/scroll left ${mainRect?.left}/${topbarRect?.left}/${scrollRect?.left})`);
  assert(scrollRect && playerRect
    && near(scrollRect.bottom, playerRect.top) && near(playerRect.top + playerRect.height, innerHeight),
    `The player reserve is one grid band (scroll bottom ${scrollRect?.bottom}, player top ${playerRect?.top}, viewport ${innerHeight})`);
  assert(getComputedStyle(scroll).overflowY === 'auto' && getComputedStyle(main).overflowY === 'hidden'
    && document.scrollingElement.scrollHeight <= innerHeight + 1,
    'The central content column is the only vertical scroller; the document itself does not scroll');
  assert(!el('.native-content') && document.querySelectorAll('.native-scroll').length === 1
    && !scroll.contains(el('.native-topbar')),
    'The old content offset is gone and the fixed topbar is outside the single scroll owner');
  const inner = el('.native-scroll-inner');
  const contentLeft = box('.library-intro')?.left;
  assert(inner && near(cssPx(inner, 'paddingLeft'), 33)
    && near(contentLeft, mainRect.left + 33),
    `Content gutter is applied once (inner padding ${inner ? cssPx(inner, 'paddingLeft') : '-'}, first content left ${contentLeft}, main left ${mainRect.left})`);
  assert(cssPx(inner, 'paddingBottom') < playerRect.height,
    `Content does not add a second player-height reserve (padding-bottom ${inner ? cssPx(inner, 'paddingBottom') : '-'}, player ${playerRect.height})`);
  assert(cssToken(app, '--vela-bg') === '#131513'
    && cssToken(app, '--vela-primary') === '#d3e9a5'
    && cssToken(app, '--vela-on-primary') === '#1b2511'
    && cssToken(app, '--vela-muted-text') === '#9aa296'
    && cssToken(app, '--vela-quiet-text') === '#798372'
    && cssToken(app, '--vela-sidebar-bg') === '#0e100f'
    && cssToken(app, '--vela-player-bg') === '#10130f'
    && cssToken(app, '--vela-player-border') === '#343b2e',
    'The approved dark role values are exposed under the namespaced --vela-* layer');
  assert(cssToken(app, '--vela-sidebar-width') === '216px'
    && cssToken(app, '--vela-topbar-height') === '75px'
    && cssToken(app, '--vela-player-height') === '88px'
    && cssToken(app, '--sidebar').includes('216')
    && cssToken(app, '--player').includes('88'),
    'Length roles stay separate from text roles and alias the legacy --sidebar/--player names');
  assert(getComputedStyle(app).backgroundColor === 'rgb(19, 21, 19)'
    && getComputedStyle(sidebar).backgroundColor === 'rgb(14, 16, 15)'
    && getComputedStyle(player).backgroundColor === 'rgb(16, 19, 15)'
    && getComputedStyle(player).borderTopColor === 'rgb(52, 59, 46)'
    && getComputedStyle(app).color === 'rgb(243, 243, 237)'
    && getComputedStyle(el('.native-sidebar .native-brand > .vela-icon')).color === 'rgb(211, 233, 165)',
    'Charcoal canvas, forest sidebar/player, warm-white text and pistachio mark render from those roles');
  checks.push(`TR01/TR02 desktop shell: ${Math.round(sidebarRect.width)}px sidebar, ${Math.round(topbarRect.height)}px topbar, ${Math.round(playerRect.height)}px player at ${innerWidth}x${innerHeight}, main left ${Math.round(mainRect.left)} = sidebar right ${Math.round(sidebarRect.right)}, content gutter ${cssPx(inner, 'paddingLeft')}px, scroll bottom ${Math.round(scrollRect.bottom)} = player top ${Math.round(playerRect.top)}`);
  await capture('library-dark');
  // TR10 typography screenshot: the same real library at the requested viewport, now
  // rendered from the embedded Manrope faces captured above.
  await capture('typography-dark');
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
  // TR03: reference-style persistent player band. These checks read the real rendered
  // band and dispatch the existing root transport actions; the presentation keeps no
  // second audio, queue or position authority.
  const bandNode = () => el('.native-player');
  const zoneRect = selector => el(selector)?.getBoundingClientRect();
  const overlapping = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 1
    && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 1;
  const bandControls = () => [...document.querySelectorAll('.native-player button,.native-player input')]
    .filter(node => node.getClientRects().length)
    .map(node => ({label: (node.getAttribute('aria-label') || node.id || node.textContent).trim(), rect: node.getBoundingClientRect()}));
  const bandRect = bandNode().getBoundingClientRect();
  const trackZone = zoneRect('.player-track'), centerZone = zoneRect('.player-controls'), extrasZone = zoneRect('.player-extras');
  assert(bandRect && near(bandRect.height, 88) && near(bandRect.left, 0) && near(bandRect.width, innerWidth),
    `TR03 player is one full-width 88px band (actual ${bandRect?.width}x${bandRect?.height} at ${bandRect?.left},${bandRect?.top})`);
  assert(cssPx(bandNode(), 'paddingTop') === 10 && cssPx(bandNode(), 'paddingBottom') === 10
    && cssPx(bandNode(), 'paddingLeft') === 26 && cssPx(bandNode(), 'paddingRight') === 26,
    `TR03 band keeps the approved 10px 26px padding (${cssPx(bandNode(), 'paddingTop')}px ${cssPx(bandNode(), 'paddingLeft')}px)`);
  assert(getComputedStyle(bandNode()).backgroundColor === 'rgb(16, 19, 15)'
    && getComputedStyle(bandNode()).borderTopColor === 'rgb(52, 59, 46)',
    'TR03 band renders the player surface and top border roles');
  assert(trackZone && centerZone && extrasZone && near(trackZone.left, bandRect.left + 26) && near(extrasZone.right, bandRect.right - 26),
    `TR03 three unboxed zones span the band exactly once (track ${trackZone?.left}, actions right ${extrasZone?.right}, band ${bandRect.left}..${bandRect.right})`);
  assert(trackZone.right <= centerZone.left + 1 && centerZone.right <= extrasZone.left + 1,
    `TR03 zones do not overlap (track right ${trackZone.right}, transport ${centerZone.left}..${centerZone.right}, actions left ${extrasZone.left})`);
  const transport = el('.player-controls .transport');
  const transportLabels = [...transport.querySelectorAll('button')].map(node => (node.getAttribute('aria-label') || '').trim());
  assert(JSON.stringify(transportLabels) === JSON.stringify(['Shuffle', 'Previous', 'Pause', 'Next', 'Repeat off']),
    `TR03 transport order is shuffle/previous/play-pause/next/repeat (actual ${transportLabels.join('/')})`);
  const transportRects = [...transport.querySelectorAll('button')].map(node => node.getBoundingClientRect());
  assert(transportRects.every(rect => rect.width >= 44 && rect.height >= 44),
    `TR03 each transport control keeps its own 44px hit box (${transportRects.map(rect => `${Math.round(rect.width)}x${Math.round(rect.height)}`).join(',')})`);
  assert(transportRects.every((rect, at) => at === 0 || rect.left >= transportRects[at - 1].right - 1), 'TR03 transport hit boxes do not overlap');
  const playCircle = el('[data-testid="native-player-play"] .play-circle').getBoundingClientRect();
  assert(near(playCircle.width, 32) && near(playCircle.height, 32),
    `TR03 play uses the approved 32px circle inside its 44px hit box (actual ${playCircle.width}x${playCircle.height})`);
  assert(getComputedStyle(el('.play-circle')).backgroundColor === 'rgb(228, 235, 217)'
    && getComputedStyle(el('.play-circle')).color === 'rgb(27, 37, 17)',
    'TR03 play circle renders the approved #e4ebd9 fill with the dark icon');
  assert(cssPx(el('.player-track strong'), 'fontSize') === 12,
    `TR03 player title uses the approved 12px role (actual ${cssPx(el('.player-track strong'), 'fontSize')}px)`);
  assert((el('.player-track strong')?.textContent || '') === 'Native Continuity' && !el('.player-sub'),
    `TR03 states only real metadata: the untagged WAV fixture shows its title without an invented artist or album (sub ${!!el('.player-sub')})`);
  const seekInput = el('#native-seek');
  assert(seekInput.type === 'range' && Number(seekInput.max) > 0 && (seekInput.getAttribute('aria-valuetext') || '').includes(' of '),
    `TR03 seek is a real native range with an honest time announcement (${seekInput.getAttribute('aria-valuetext')})`);
  assert(cssPx(seekInput, 'height') === 22 && parseFloat(cssToken(seekInput, '--vela-progress')) > 0,
    `TR03 seek keeps the compact row and a state-driven filled rail (height ${cssPx(seekInput, 'height')}px, progress ${cssToken(seekInput, '--vela-progress')})`);
  assert(cssPx(el('.player-time'), 'fontSize') === 10
    && getComputedStyle(el('.player-time')).fontVariantNumeric.includes('tabular'),
    `TR03 player times use the approved 10px tabular role (actual ${cssPx(el('.player-time'), 'fontSize')}px ${getComputedStyle(el('.player-time')).fontVariantNumeric})`);
  const volumeSlider = el('#native-volume');
  assert(volumeSlider.type === 'range' && near(cssPx(volumeSlider, 'width'), 65),
    `TR03 volume keeps the approved 65px rail (actual ${cssPx(volumeSlider, 'width')}px)`);
  const muteState = () => el('[data-testid="native-player-mute"]')?.getAttribute('aria-pressed');
  const volumeLevel = () => Number(el('#native-volume')?.value);
  assert(muteState() === 'true' && button('Unmute'), 'TR03 zero engine volume is an honest muted state with a real Unmute action');
  fill('#native-volume', '45');
  await wait(() => muteState() === 'false' && volumeLevel() === 45, 'Real engine volume acknowledged at 45%');
  const tr03Source = generation(), tr03Position = position();
  await click('Mute');
  await wait(() => muteState() === 'true' && volumeLevel() === 0, 'Mute drives the one engine volume to zero');
  assert(generation() === tr03Source && phase() === 'Playing' && position() > tr03Position,
    'Mute never replaces the source or interrupts the advancing native clock');
  await click('Unmute');
  await wait(() => muteState() === 'false' && volumeLevel() === 45, 'Unmute restores the remembered audible level');
  assert(generation() === tr03Source && button('Pause'), 'Unmute keeps the same native source and playing state');
  checks.push(`TR03 volume/mute: 45% → 0% → 45% on source ${tr03Source}, position ${tr03Position}→${position()}ms, no source replacement`);
  fill('#native-volume', '0'); await sleep(250);
  const queueEntry = el('[data-testid="native-player-queue"]');
  assert(queueEntry && queueEntry.getAttribute('aria-label') === 'Open queue', 'TR03 band exposes the real Queue entry');
  const queueSource = generation(), queuePosition = position();
  await click('Open queue');
  await wait(() => document.querySelectorAll('[data-native-queue-index]').length > 0, 'Band Queue entry opens the real playback queue');
  assert(generation() === queueSource && position() >= queuePosition && phase() === 'Playing',
    `TR03 opening the queue never resets the source or rewinds the position (source ${generation()}, ${position()}ms vs ${queuePosition}ms)`);
  await sleep(350);
  assert(position() > queuePosition, 'TR03 the native clock keeps advancing while the queue view is open');
  const queueOccurrences = document.querySelectorAll('[data-native-queue-index]').length;
  const queueCurrentAt = [...document.querySelectorAll('.native-queue-row')].findIndex(row => row.classList.contains('current'));
  assert(queueCurrentAt >= 0, 'TR03 queue view marks the current occurrence');
  await click('Library');
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Library returns after the band queue entry');
  await click('Open queue');
  await wait(() => document.querySelectorAll('[data-native-queue-index]').length === queueOccurrences, 'Queue reopens with the same occurrences');
  assert(document.querySelectorAll('[data-native-queue-index]').length === queueOccurrences
    && [...document.querySelectorAll('.native-queue-row')].findIndex(row => row.classList.contains('current')) === queueCurrentAt
    && generation() === queueSource,
    'TR03 navigation through the band never resets queue, current occurrence or source');
  checks.push(`TR03 queue entry: ${queueOccurrences} occurrences, current ${queueCurrentAt}, source ${queueSource} retained across Library↔Queue`);
  await click('Library');
  await wait(() => document.querySelectorAll('[data-native-track-id]').length === 3, 'Library restored after the band continuity check');
  assert([...document.querySelectorAll('.native-player button')].every(node => (node.getAttribute('aria-label') || '').trim().length > 0),
    'TR03 every band control keeps an accessible name');
  assert((document.querySelector('label[for="native-seek"]')?.textContent || '') === 'Playback position'
    && (document.querySelector('label[for="native-volume"]')?.textContent || '') === 'Volume',
    'TR03 both native sliders keep their keyboard labels');
  const bandTabbable = [...document.querySelectorAll('.native-player button,.native-player input')].filter(node => !node.disabled && node.tabIndex >= 0).length;
  assert(bandTabbable >= 9, `TR03 band stays keyboard reachable (${bandTabbable} tabbable controls)`);
  assert(el('[data-testid="native-player-announcement"]')?.getAttribute('aria-live') === 'polite'
    && (el('[data-testid="native-player-announcement"]')?.textContent || '').includes('Native Continuity'),
    'TR03 announces the current track once through a polite live region');
  assert(seekInput.disabled === false && el('#native-volume').disabled === false,
    'TR03 seek and volume stay operable while a source is loaded');
  const bandText = (bandNode().textContent || '').toLowerCase();
  assert(!/lossless|bit-?perfect|hi-?res|hires|studio master|kbps|khz|demo/.test(bandText),
    `TR03 band claims no invented quality, bitrate or licence state (text "${(bandNode().textContent || '').trim().slice(0, 120)}")`);
  assert((el('.player-source-value')?.textContent || '').trim() === 'Your library',
    `TR03 source line states the actual playback context (actual "${(el('.player-source-value')?.textContent || '').trim()}")`);
  assert(!el('[data-testid="native-expanded-player"]') && !button('Expand') && !button('Now playing'),
    'TR03 presents no expanded Now Playing surface; TR07 owns it and the band never fakes one');
  checks.push(`TR03 player band: ${Math.round(bandRect.width)}x${Math.round(bandRect.height)} at (${Math.round(bandRect.left)},${Math.round(bandRect.top)}), zones ${Math.round(trackZone.width)}/${Math.round(centerZone.width)}/${Math.round(extrasZone.width)}px, transport ${transportRects.map(rect => Math.round(rect.width)).join('/')}px, play circle ${Math.round(playCircle.width)}px, seek ${cssPx(seekInput, 'height')}px, volume ${Math.round(cssPx(volumeSlider, 'width'))}px, source "${(el('.player-source-value')?.textContent || '').trim()}"`);
  await capture('player-band-dark');
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
  // TR03: record the actual queue signature before the appearance changes below.
  const appearanceQueue = {count: document.querySelectorAll('[data-native-queue-index]').length, current: [...document.querySelectorAll('.native-queue-row')].findIndex(row => row.classList.contains('current'))};
  assert(appearanceQueue.count > 0 && appearanceQueue.current >= 0, 'Queue view exposes the current occurrence before the appearance checks');
  await click('Use the light theme');
  assert(el('.native-app').dataset.theme === 'light', 'Functional light theme');
  assert(generation() === gen && phase() === 'Paused', 'Theme change preserves the native source, queue and paused state');
  await capture('queue-light');
  // Theme/navigation/scale are presentation changes. None may replace the one
  // AudioEngine source or reset its paused position.
  const scaleSource = generation(), scalePosition = position();
  await click('Settings');
  await wait(() => el('[data-testid="native-app-scale-value"]'), 'Settings opens for the app-scale continuity check');
  assert(generation() === scaleSource && phase() === 'Paused', 'Navigation to Settings preserves the native source');
  const scaleSlider = el('#native-app-scale');
  const scaleValue = () => (el('[data-testid="native-app-scale-value"]')?.textContent || '').trim();
  const appScale = () => Number(el('.native-app').dataset.appScale);
  const stepScale = async (label, expected) => {
    await click(label);
    await wait(() => scaleValue() === `${expected}%`, `App size ${expected}%`);
    assert(appScale() === expected, `App size ${expected}% is applied on the single native-app wrapper`);
  };
  assert(scaleSlider && scaleSlider.type === 'range' && scaleSlider.min === '80' && scaleSlider.max === '150' && scaleSlider.step === '5'
    && scaleSlider.getAttribute('aria-valuemin') === '80' && scaleSlider.getAttribute('aria-valuemax') === '150'
    && scaleValue() === '100%' && appScale() === 100,
    `App size exposes the validated 80-150 range (min ${scaleSlider?.min}, max ${scaleSlider?.max}, step ${scaleSlider?.step}, value ${scaleValue()}, wrapper ${appScale()})`);
  assert((el('.app-scale-legend')?.textContent || '').replace(/\s+/g, '') === '80%100%150%', 'App size legend states the real 80/100/150 endpoints');
  await click('Increase app size');
  await wait(() => scaleValue() === '105%', 'App size stepper applies 105%');
  assert(generation() === scaleSource && phase() === 'Paused' && Math.abs(position() - scalePosition) < 150,
    'App scale preserves the source, selection and paused position');
  assert(appScale() === 105 && cssToken(app, '--app-zoom') === '1.05' && near(parseFloat(getComputedStyle(app).zoom) * 100, 105, 0.5),
    'App scale is applied once on the native-app wrapper');
  await click('Reset to 100%');
  await wait(() => scaleValue() === '100%', 'App size reset to 100%');
  assert(generation() === scaleSource && phase() === 'Paused', 'Scale reset keeps the same native source and paused state');
  // TR10: the documented device range is 80-150 on 5-point steps. Drive the real
  // stepper across both ends, verify the bounded controls stop there and that the
  // single native engine/source/position survives every step.
  for (let step = 0; step < 4; step++) await stepScale('Decrease app size', 100 - (step + 1) * 5);
  assert(scaleValue() === '80%' && appScale() === 80, 'App size reaches its documented 80% floor');
  assert(button('Decrease app size')?.disabled === true, 'The 80% floor disables the decrease control');
  button('Decrease app size').click(); await sleep(250);
  assert(scaleValue() === '80%' && appScale() === 80, 'The 80% floor cannot be undercut');
  assert(cssToken(app, '--app-zoom') === '0.80' && near(parseFloat(getComputedStyle(app).zoom) * 100, 80, 0.5)
    && getComputedStyle(app).transform === 'none',
    `The 80% size is one zoom value, never a stacked transform (zoom ${getComputedStyle(app).zoom}, --app-zoom ${cssToken(app, '--app-zoom')}, transform ${getComputedStyle(app).transform})`);
  assert(document.documentElement.scrollWidth <= innerWidth + 1, `App size 80% does not overflow the ${innerWidth}px viewport`);
  for (let step = 0; step < 14; step++) await stepScale('Increase app size', 80 + (step + 1) * 5);
  assert(scaleValue() === '150%' && appScale() === 150, 'App size reaches its documented 150% ceiling');
  assert(button('Increase app size')?.disabled === true, 'The 150% ceiling disables the increase control');
  button('Increase app size').click(); await sleep(250);
  assert(scaleValue() === '150%' && appScale() === 150, 'The 150% ceiling cannot be exceeded');
  assert(cssToken(app, '--app-zoom') === '1.50' && near(parseFloat(getComputedStyle(app).zoom) * 100, 150, 0.5)
    && getComputedStyle(app).transform === 'none',
    `The 150% size is one zoom value, never a stacked transform (zoom ${getComputedStyle(app).zoom}, --app-zoom ${cssToken(app, '--app-zoom')}, transform ${getComputedStyle(app).transform})`);
  assert(document.documentElement.scrollWidth <= innerWidth + 1, `App size 150% does not overflow the ${innerWidth}px viewport`);
  assert(generation() === scaleSource && phase() === 'Paused' && Math.abs(position() - scalePosition) < 150,
    'The complete 80-150 app-scale sweep preserves the native source, selection and paused position');
  const engineAtScale = await engineStreams();
  assert(engineAtScale && Number(engineAtScale.sinkInputs) === 1,
    `The complete app-scale sweep keeps exactly one native output stream (${JSON.stringify(engineAtScale)})`);
  const peakScale = appScale();
  await click('Reset to 100%');
  await wait(() => scaleValue() === '100%', 'App size reset after the 80-150 sweep');
  assert(generation() === scaleSource && phase() === 'Paused' && appScale() === 100,
    'Reset returns to 100% with the same native source and paused state');
  checks.push(`TR10 app scale: 100%→105%→100%→80%→150% (peak ${peakScale}%)→100% on source ${scaleSource}, one output stream on "${engineAtScale?.sink}"`);
  await click('Queue');
  await wait(() => document.querySelectorAll('[data-native-queue-index]').length > 0, 'Queue view after theme and app-scale changes');
  assert(document.querySelectorAll('[data-native-queue-index]').length === appearanceQueue.count
    && [...document.querySelectorAll('.native-queue-row')].findIndex(row => row.classList.contains('current')) === appearanceQueue.current
    && generation() === scaleSource && phase() === 'Paused',
    'TR03 theme and app-scale changes never reset the queue, its current occurrence, the source or the paused state');
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
  // TR02 narrow shell: the same single scroll owner and player band, with the
  // desktop sidebar offset removed rather than compounded.
  const compactApp = el('.native-app'), compactSidebar = el('.native-sidebar'), compactMain = el('.native-main');
  const compactTopbar = el('.native-topbar'), compactScroll = el('.native-scroll');
  const compactMainRect = compactMain.getBoundingClientRect(), compactTopbarRect = compactTopbar.getBoundingClientRect();
  const compactScrollRect = compactScroll.getBoundingClientRect();
  assert(getComputedStyle(compactSidebar).display === 'none' && cssToken(compactApp, '--vela-sidebar-width') === '0px',
    'Narrow shell removes the desktop sidebar and zeroes its layout role');
  assert(near(compactMainRect.left, 0) && near(compactTopbarRect.left, 0) && near(compactScrollRect.left, 0),
    `Narrow main/topbar/scroll start at 0, not an inherited sidebar offset (${compactMainRect.left}/${compactTopbarRect.left}/${compactScrollRect.left})`);
  assert(compactPlayer && near(compactPlayer.left, 0) && near(compactPlayer.width, innerWidth) && compactPlayer.height >= 88,
    `Narrow player stays full-width and at least the 88px baseline (${compactPlayer?.left}..${compactPlayer?.right}, ${compactPlayer?.height}px)`);
  assert(near(compactPlayer.top + compactPlayer.height, innerHeight) && near(compactScrollRect.bottom, compactPlayer.top),
    `Narrow player reserve is one band below the scroller (scroll bottom ${compactScrollRect.bottom}, player top ${compactPlayer?.top})`);
  assert(getComputedStyle(compactScroll).overflowY === 'auto'
    && document.scrollingElement.scrollHeight <= innerHeight + 1
    && document.querySelectorAll('.native-scroll').length === 1 && !el('.native-content'),
    'Narrow shell keeps exactly one central scroll owner and no document overflow');
  const compactNav = el('.native-compact-nav');
  const compactNavButtons = compactNav ? [...compactNav.querySelectorAll('button')] : [];
  assert(compactNav && getComputedStyle(compactNav).display === 'flex'
    && compactNavButtons.length === compactDestinations.length
    && compactDestinations.every(name => compactNavButtons.some(node => (node.getAttribute('aria-label') || node.textContent).trim() === name))
    && compactNavButtons.every(node => node.getBoundingClientRect().height >= 44)
    && getComputedStyle(compactNav).overflowX === 'auto',
    'All current native destinations remain reachable in the compact navigation');
  checks.push(`TR02 narrow shell: ${innerWidth}x${innerHeight}, main left ${Math.round(compactMainRect.left)}, topbar ${Math.round(compactTopbarRect.height)}px, player ${compactPlayer.height.toFixed(1)}px at y=${compactPlayer.top.toFixed(1)}, scroller ${Math.round(compactScrollRect.height)}px, destinations ${compactNavButtons.map(node => (node.getAttribute('aria-label') || node.textContent).trim()).join('/')}`);
  // TR03 at 640px: the same band keeps every real control reachable in two rows
  // with no overlap, no clipped control and no hidden transport.
  const narrowBandNode = el('.native-player'), narrowBand = narrowBandNode.getBoundingClientRect();
  const narrowTrack = zoneRect('.player-track'), narrowCenter = zoneRect('.player-controls'), narrowExtras = zoneRect('.player-extras');
  assert(narrowTrack && narrowCenter && narrowTrack.bottom <= narrowCenter.top + 1,
    `TR03 narrow band stacks the real title row above the transport (track bottom ${narrowTrack?.bottom}, transport top ${narrowCenter?.top})`);
  assert(narrowExtras && narrowExtras.top < narrowCenter.bottom && narrowExtras.bottom > narrowCenter.top,
    'TR03 narrow band keeps Queue, mute and volume beside the transport in the second row');
  assert(near(cssPx(el('.player-cover'), 'width'), 48), `TR03 narrow band keeps the 48px artwork box (actual ${cssPx(el('.player-cover'), 'width')}px)`);
  assert(getComputedStyle(el('.player-source')).display === 'none',
    'TR03 narrow band applies the approved 761–900px "hide source" step instead of clipping it');
  const narrowControls = bandControls();
  assert(narrowControls.length >= 9, `TR03 narrow band keeps every control (actual ${narrowControls.length})`);
  assert(narrowControls.every(control => control.rect.left >= narrowBand.left - 1 && control.rect.right <= narrowBand.right + 1
    && control.rect.top >= narrowBand.top - 1 && control.rect.bottom <= narrowBand.bottom + 1),
    `TR03 no narrow band control leaves the player (${narrowControls.map(control => control.label).join('/')})`);
  assert(narrowControls.every((control, at) => narrowControls.every((other, index) => at === index || !overlapping(control.rect, other.rect))),
    `TR03 narrow band controls never overlap (${narrowControls.map(control => `${control.label}:${Math.round(control.rect.width)}x${Math.round(control.rect.height)}`).join(',')})`);
  assert(narrowControls.every(control => control.rect.width >= 44 && control.rect.height >= 22),
    'TR03 narrow band keeps usable pointer boxes for every control');
  assert(narrowBandNode.scrollWidth <= narrowBandNode.clientWidth + 1,
    `TR03 narrow band has no clipped horizontal content (scroll ${narrowBandNode.scrollWidth} vs ${narrowBandNode.clientWidth})`);
  assert(button('Play') && !button('Pause') && muteState() === 'true' && button('Unmute'),
    'TR03 narrow band renders the real paused and muted states');
  checks.push(`TR03 narrow band: ${Math.round(narrowBand.width)}x${Math.round(narrowBand.height)} at 640px, track row bottom ${Math.round(narrowTrack.bottom)}, transport y ${Math.round(narrowCenter.top)}–${Math.round(narrowCenter.bottom)}, actions x ${Math.round(narrowExtras.left)}–${Math.round(narrowExtras.right)}, controls ${narrowControls.length}`);
  await capture('player-band-narrow-light');
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
    // IN01: with the fixture server stopped the library request cannot settle, so the
    // skeleton state now stops instead of pretending: the error surfaces, the pending
    // rows and busy region are gone, no stale row is kept and the retry stays enabled.
    const retry = button('Refresh library');
    if (!retry || retry.disabled) throw new Error(`Library retry control missing before the failure fixture (rows=${document.querySelectorAll('[data-native-track-id]').length})`);
    retry.click();
    await wait(() => (el('.native-library')?.dataset.error || '') !== '' && el('.native-library')?.dataset.loading === 'false', 'Failed library request reports an error');
    assert((el('.alert.error')?.textContent || '').includes('Refresh the library to try again'), 'Failed library request explains how to retry');
    assert(button('Refresh library') && !button('Refresh library').disabled, 'Failed library request keeps the explicit refresh retry enabled');
    assert(skeletons().length === 0, 'Failed library request leaves no pending skeleton rows');
    assert(!el('.native-skeleton-status'), 'Failed library request removes the pending status region');
    assert((el('.native-library')?.getAttribute('aria-busy') || '') === 'false', 'Failed library request clears the busy region');
    assert(document.querySelectorAll('[data-native-track-id]').length === 0, 'Failed library request renders no stale or fake track rows');
    checks.push(`IN01 failure evidence: pending skeletons stopped at ${skeletons().length} rows, busy=${el('.native-library')?.getAttribute('aria-busy')}, retry enabled=${!button('Refresh library')?.disabled}, error="${(el('.alert.error')?.textContent || '').trim()}"`);
    dioxus.send({ok:true, checks, viewportEvidence});
  }
} catch(error) { dioxus.send({ok:false, checks, failure:String(error.message)}); }
}
