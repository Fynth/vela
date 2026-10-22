import { test, expect } from '@playwright/test';
import { mkdir, mkdtemp, open, readFile, rm, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';

const root = fileURLToPath(new URL('../../', import.meta.url));

// DR07: the queue drawer must be usable with a keyboard alone. The focus
// ring is a real 2px/3px ring in the semantic accent role, both rails carry
// a visible name and announce a value a person can act on, and Escape hands
// the focus back to the control that opened the drawer. The sleep countdown
// ticks every second and must stay out of every live region, and the drawer
// must also close from the Now Playing sheet's "View full queue". This
// journey owns its server, data and sessions.
async function isolatedServer(output) {
  const directory = await mkdtemp(join(tmpdir(), 'vela-dr07-'));
  const listener = createServer();
  await new Promise(resolve => listener.listen(0, '127.0.0.1', resolve));
  const port = listener.address().port;
  await new Promise(resolve => listener.close(resolve));
  const log = await open(join(output, 'server.log'), 'w', 0o600);
  const server = spawn(resolve(root, process.env.VELA_TEST_BINARY || 'target/debug/vela-server'), [], {
    cwd: root, stdio: ['ignore', log.fd, log.fd],
    env: { ...process.env, VELA_BIND: '127.0.0.1:' + port, VELA_DATA_DIR: join(directory, 'data'),
      VELA_WEB_DIR: resolve(root, process.env.VELA_TEST_WEB_DIR || 'dist'), VELA_SCAN_ROOTS: '[]' },
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

async function navigate(page, name) {
  const mobile = page.getByRole('navigation', { name: 'Mobile navigation', exact: true });
  if (await mobile.isVisible()) {
    const tab = mobile.getByRole('button', { name, exact: true });
    if (await tab.count()) await tab.click();
    else {
      await mobile.getByRole('button', { name: 'More destinations', exact: true }).click();
      await page.getByRole('navigation', { name: 'More from Vela', exact: true })
        .getByRole('button', { name, exact: true }).click();
    }
  } else await page.getByRole('button', { name, exact: true }).first().click();
}

async function setTheme(page, theme) {
  if (await page.locator('html').getAttribute('data-theme') !== theme) {
    await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
  }
  await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

// The real trigger of the drawer. One of the two exists per layout.
async function queueTrigger(page) {
  const desktop = page.getByRole('button', { name: 'Show playback queue', exact: true });
  if (await desktop.isVisible()) return desktop;
  return page.getByRole('button', { name: 'Show playback queue and controls', exact: true });
}

async function openQueue(page) {
  // The drawer is a modal: while it is open the trigger behind it is
  // inert, so a click on it would never land. The state is asserted
  // first, so a journey that forgets to close the drawer fails at once
  // instead of waiting for a click that cannot happen.
  await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);
  const trigger = await queueTrigger(page);
  await trigger.click();
  await expect(page.locator('#queue-drawer-dialog')).toBeVisible();
  return trigger;
}

// A real Tab press owns the keyboard modality, so the focus that follows is
// a genuine :focus-visible ring in both engines: a programmatic focus after a
// pointer action may legally leave the ring off.
async function focusControl(page, selector) {
  await page.keyboard.press('Tab');
  await expect.poll(async () => {
    await page.locator(selector).first().focus();
    return page.evaluate(sel => document.activeElement === document.querySelector(sel), selector);
  }, { message: `${selector} receives focus` }).toBe(true);
}

// The real keyboard order of the drawer: one forward walk with real Tab
// presses, element by element, until the engine's own cycle repeats a
// control. Firefox does not cycle inside a modal dialog, so the walk is
// recorded instead of being forced.
const walkOrder = async (page, limit = 30) => {
  const order = [];
  const seen = new Set();
  for (let step = 0; step < limit; step++) {
    const current = await page.evaluate(() => {
      const element = document.activeElement;
      if (!element || element === document.body) return null;
      const path = [];
      for (let node = element; node && node !== document.body; node = node.parentElement) {
        const siblings = node.parentElement ? [...node.parentElement.children] : [];
        path.unshift(`${node.tagName}:${siblings.indexOf(node)}`);
      }
      return {
        path: path.join('/'), tag: element.tagName, id: element.id,
        cls: String(element.className || ''),
        label: (element.getAttribute('aria-label') || element.textContent || '').trim().slice(0, 40),
      };
    });
    if (!current || seen.has(current.path)) break;
    seen.add(current.path);
    order.push(current);
    await page.keyboard.press('Tab');
  }
  return order;
};

// The focus ring as the browser paints it, with the surface it is painted on.
const RING = () => {
  const parse = value => { const parts = value.match(/[\d.]+/g); return parts ? { r: +parts[0], g: +parts[1], b: +parts[2], a: parts[3] === undefined ? 1 : +parts[3] } : null; };
  const luminance = colour => [colour.r, colour.g, colour.b]
    .map(value => value / 255)
    .map(value => value <= .04045 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4)
    .reduce((total, value, at) => total + value * [.2126, .7152, .0722][at], 0);
  const surfaceOf = element => {
    for (let parent = element; parent; parent = parent.parentElement) {
      const colour = parse(getComputedStyle(parent).backgroundColor);
      if (colour && colour.a === 1) return colour;
    }
    return parse(getComputedStyle(document.body).backgroundColor);
  };
  const element = document.activeElement;
  if (!element || element === document.body) return null;
  const style = getComputedStyle(element);
  const rect = element.getBoundingClientRect();
  const outline = parse(style.outlineColor);
  const surface = surfaceOf(element);
  const a = luminance(outline), b = luminance(surface);
  const dialog = document.querySelector('#queue-drawer-dialog');
  const dialogRect = dialog ? dialog.getBoundingClientRect() : null;
  return {
    tag: element.tagName, id: element.id, className: String(element.className),
    label: (element.getAttribute('aria-label') || element.textContent || '').trim().slice(0, 48),
    focusVisible: element.matches(':focus-visible'),
    outlineStyle: style.outlineStyle,
    outlineWidth: Number.parseFloat(style.outlineWidth),
    outlineOffset: Number.parseFloat(style.outlineOffset),
    outlineColor: style.outlineColor,
    surface: `rgb(${surface.r}, ${surface.g}, ${surface.b})`,
    ratio: (Math.max(a, b) + .05) / (Math.min(a, b) + .05),
    rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height, right: rect.right, bottom: rect.bottom },
    dialogRect: dialogRect ? { x: dialogRect.x, y: dialogRect.y, right: dialogRect.right, bottom: dialogRect.bottom } : null,
  };
};

// The real rectangles of the two rails and the labels that name them.
const RAILS = () => {
  const measure = selector => {
    const input = document.querySelector(selector);
    if (!input) return null;
    const container = input.closest('.queue-volume, .queue-seek');
    const label = container ? container.querySelector('label.queue-field-label') : null;
    const rect = input.getBoundingClientRect();
    const style = getComputedStyle(input);
    const labelStyle = label ? getComputedStyle(label) : null;
    return {
      id: input.id, value: input.value, min: input.getAttribute('min'), max: input.getAttribute('max'),
      valueText: input.getAttribute('aria-valuetext'), ariaLabel: input.getAttribute('aria-label'),
      disabled: input.disabled,
      width: rect.width, height: rect.height,
      columnWidth: container ? container.clientWidth : null,
      style: { display: style.display, minWidth: style.minWidth, height: style.height, minHeight: style.minHeight, padding: style.padding, margin: style.margin },
      label: label ? {
        text: label.textContent.trim(), for: label.htmlFor, bound: label.htmlFor === input.id,
        visible: label.checkVisibility(), fontSize: labelStyle.fontSize, fontWeight: labelStyle.fontWeight,
        color: labelStyle.color,
      } : null,
    };
  };
  return { volume: measure('#queue-volume'), position: measure('#queue-position') };
};

const mmss = seconds => {
  const total = Number.isFinite(seconds) && seconds > 0 ? Math.floor(seconds) : 0;
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, '0')}`;
};

// The evidence directory. The engine is part of the default path, so a
// Chromium run and a Firefox run never overwrite each other's screenshots;
// VELA_DR07_EVIDENCE overrides the whole path when a run wants its own.
const evidenceDir = engine => resolve(root, process.env.VELA_DR07_EVIDENCE || `target/checks/dr07-a11y-${engine}`);

// The sleep countdown as the accessibility tree sees it: its own role, its
// own politeness, its name, its text and every live region above it.
const COUNTDOWN = () => {
  const element = document.querySelector('.sleep-timer small');
  if (!element) return null;
  const liveAbove = [];
  for (let node = element.parentElement; node; node = node.parentElement) {
    const role = node.getAttribute('role');
    const live = node.getAttribute('aria-live');
    const isLive = (live && live !== 'off') || (role && ['status', 'alert', 'log', 'marquee'].includes(role));
    if (isLive) liveAbove.push({ tag: node.tagName, role, live });
  }
  return {
    role: element.getAttribute('role'),
    live: element.getAttribute('aria-live'),
    name: element.getAttribute('aria-label'),
    text: element.textContent.trim(),
    liveAbove,
  };
};

// The countdown's own box inside the drawer: is it visible, and does it stay
// inside the panel at every width?
const COUNTDOWN_BOX = () => {
  const element = document.querySelector('.sleep-timer small');
  const panel = document.querySelector('#queue-drawer-dialog .queue-panel');
  const form = document.querySelector('.sleep-timer');
  if (!element || !panel || !form) return null;
  const rect = element.getBoundingClientRect();
  const panelRect = panel.getBoundingClientRect();
  return {
    text: element.textContent.trim(),
    visible: element.checkVisibility(),
    left: rect.left, right: rect.right, width: rect.width,
    panelLeft: panelRect.left, panelRight: panelRect.right,
    panelOverflowX: panel.scrollWidth - panel.clientWidth,
    formOverflowX: form.scrollWidth - form.clientWidth,
  };
};

// The drawer's own announcement surface: the one live region inside the
// modal, and every live region the page still carries with its text. The
// region is a permanent node: it is in the DOM before the first transition
// and only its text changes, so `present` is true from the start.
const STATUS = () => {
  const dialog = document.querySelector('#queue-drawer-dialog');
  const line = dialog ? dialog.querySelector('[role="status"]') : null;
  const live = [];
  for (const node of document.querySelectorAll('[role="status"],[role="alert"],[aria-live]:not([aria-live="off"])')) {
    const role = node.getAttribute('role');
    const politeness = node.getAttribute('aria-live');
    if (role !== 'status' && role !== 'alert' && !politeness) continue;
    live.push({
      role, politeness,
      text: (node.textContent || '').trim().slice(0, 90),
      inside: !!(dialog && dialog.contains(node)),
    });
  }
  return {
    present: !!line,
    line: line ? line.textContent.trim() : null,
    // The line carries exactly one text: the message of the last transition,
    // empty before the first transition.
    texts: line ? [...line.childNodes].filter(node => node.nodeType === 3).length : 0,
    elements: line ? line.children.length : 0,
    count: dialog ? dialog.querySelectorAll('[role="status"]').length : 0,
    live,
  };
};

// The announcement line and the space it takes: the empty line is in the
// DOM and in the accessibility tree, but it is not a line of the drawer, so
// the sleep row and the drawer keep the geometry they had without it.
const STATUS_BOX = () => {
  const line = document.querySelector('#queue-drawer-dialog .queue-sleep-status');
  const wrapper = document.querySelector('#queue-drawer-dialog .queue-sleep');
  const row = document.querySelector('#queue-drawer-dialog .sleep-timer');
  if (!line || !wrapper || !row) return null;
  const style = getComputedStyle(line);
  return {
    text: line.textContent.trim(),
    collapsed: line.classList.contains('is-empty'),
    height: line.getBoundingClientRect().height,
    marginTop: style.marginTop,
    wrapperHeight: wrapper.getBoundingClientRect().height,
    rowHeight: row.getBoundingClientRect().height,
  };
};

// Is the background behind the open modal really out of reach? A control
// behind a native modal dialog cannot take the focus, so the root's
// receipt is not in the accessible surface while the drawer is open.
const BACKGROUND_BLOCKED = () => {
  const dialog = document.querySelector('#queue-drawer-dialog');
  const trigger = document.querySelector('button[aria-label="Show playback queue"],button[aria-label="Show playback queue and controls"]');
  if (!dialog || !trigger) return { moved: null, inside: false };
  const before = document.activeElement;
  trigger.focus();
  const active = document.activeElement;
  return { moved: active !== before, inside: !!(dialog && dialog.contains(active)) };
};

// The announcement line as the browser paints it: its own colour against
// the surface of the drawer panel, with the real contrast ratio.
const STATUS_CONTRAST = () => {
  const line = document.querySelector('#queue-drawer-dialog .queue-sleep-status');
  const panel = document.querySelector('#queue-drawer-dialog .queue-panel');
  if (!line || !panel) return null;
  const parse = value => { const parts = value.match(/[\d.]+/g); return parts ? { r: +parts[0], g: +parts[1], b: +parts[2], a: parts[3] === undefined ? 1 : +parts[3] } : null; };
  const luminance = colour => [colour.r, colour.g, colour.b]
    .map(value => value / 255)
    .map(value => value <= .04045 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4)
    .reduce((total, value, at) => total + value * [.2126, .7152, .0722][at], 0);
  const style = getComputedStyle(line);
  const foreground = parse(style.color);
  let background = null;
  for (let node = panel; node; node = node.parentElement) {
    const colour = parse(getComputedStyle(node).backgroundColor);
    if (colour && colour.a === 1) { background = colour; break; }
  }
  if (!foreground || !background) return null;
  const a = luminance(foreground), b = luminance(background);
  return {
    text: line.textContent.trim(),
    color: style.color,
    surface: `rgb(${background.r}, ${background.g}, ${background.b})`,
    fontSize: style.fontSize,
    ratio: (Math.max(a, b) + .05) / (Math.min(a, b) + .05),
  };
};

// The two decorative roles of the queue rows as the browser resolves them
// in the current theme: the current row's tint and the next occurrence's
// outline, over the panel they sit on.
const ROW_ROLES = () => {
  const panel = document.querySelector('#queue-drawer-dialog .queue-panel');
  if (!panel) return null;
  const parse = value => { const parts = value.match(/[\d.]+/g); return parts ? { r: +parts[0], g: +parts[1], b: +parts[2], a: parts[3] === undefined ? 1 : +parts[3] } : null; };
  // The roles are authored as hex with an alpha byte: #rrggbbaa.
  const parseHex = value => {
    const hex = value.trim().replace('#', '');
    if (hex.length !== 8) return null;
    return {
      r: parseInt(hex.slice(0, 2), 16), g: parseInt(hex.slice(2, 4), 16),
      b: parseInt(hex.slice(4, 6), 16), a: parseInt(hex.slice(6, 8), 16) / 255,
    };
  };
  const luminance = colour => [colour.r, colour.g, colour.b]
    .map(value => value / 255)
    .map(value => value <= .04045 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4)
    .reduce((total, value, at) => total + value * [.2126, .7152, .0722][at], 0);
  const ratio = (top, bottom) => {
    const a = luminance(top), b = luminance(bottom);
    return (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
  };
  const over = (top, bottom) => ({
    r: top.r * top.a + bottom.r * (1 - top.a),
    g: top.g * top.a + bottom.g * (1 - top.a),
    b: top.b * top.a + bottom.b * (1 - top.a),
  });
  const surface = parse(getComputedStyle(panel).backgroundColor);
  const role = name => {
    const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    const colour = parseHex(value);
    return colour ? { value, ratio: ratio(over(colour, surface), surface) } : null;
  };
  return {
    surface: getComputedStyle(panel).backgroundColor,
    current: role('--vela-track-current'),
    next: role('--vela-track-next'),
    // The row that actually paints the role, when one is on screen.
    applied: (() => {
      const row = document.querySelector('#queue-drawer-dialog .queue-row.current');
      if (!row) return null;
      const painted = parse(getComputedStyle(row).backgroundColor);
      const roleColour = parseHex(getComputedStyle(document.documentElement).getPropertyValue('--vela-track-current').trim());
      if (!painted || !roleColour) return null;
      return {
        css: getComputedStyle(row).backgroundColor,
        sameRole: painted.r === roleColour.r && painted.g === roleColour.g
          && painted.b === roleColour.b && Math.abs(painted.a - roleColour.a) < 0.002,
      };
    })(),
  };
};

test('DR07 the queue drawer is keyboard readable: ring, named rails, focus return', async ({ page, browserName }) => {
  test.setTimeout(600000);
  const output = evidenceDir(browserName);
  await mkdir(output, { recursive: true });
  const runtime = await isolatedServer(output);
  const evidence = { themes: {}, viewports: [], escape: [], rails: {}, values: {} };
  try {
    await expect.poll(async () => {
      try { return (await page.request.get(runtime.origin + '/health', { timeout: 1000 })).ok(); }
      catch { return false; }
    }).toBeTruthy();
    const setup = await page.request.post(runtime.origin + '/api/setup', {
      headers: { Origin: runtime.origin }, data: { username: 'reader', password: 'private dr07 fixture password' },
    });
    expect(setup.ok()).toBeTruthy();
    const me = await setup.json();
    const headers = { 'X-CSRF-Token': me.csrf_token, Origin: runtime.origin };
    for (const [name, mime] of [['signal.flac', 'audio/flac'], ['beacon.mp3', 'audio/mpeg']]) {
      const upload = await page.request.post(runtime.origin + '/api/upload', {
        headers, multipart: { file: { name, mimeType: mime, buffer: await readFile(resolve(root, 'crates/media/tests/fixtures/' + name)) } },
      });
      expect(upload.ok(), name).toBeTruthy();
    }
    const catalog = await (await page.request.get(runtime.origin + '/api/tracks?sort=newest&limit=50')).json();
    const durations = catalog.items.map(track => track.duration_ms);
    expect(durations.length, 'the real catalog has real durations').toBe(2);
    for (const duration of durations) expect(duration, 'a real track has a real length').toBeGreaterThan(0);

    await page.setViewportSize({ width: 1284, height: 856 });
    await page.goto(runtime.origin);
    await expect(page.locator('.auth-card, .app-shell')).toBeVisible();
    if (await page.locator('.app-shell').count() === 0) {
      await page.locator('#username').fill('reader');
      await page.locator('#password').fill('private dr07 fixture password');
      await page.getByRole('button', { name: /sign in|create your music space/i }).first().click();
    }
    await expect(page.locator('.app-shell')).toBeVisible();

    // --- The empty queue: the position rail has no length yet and says so.
    await navigate(page, 'Library');
    await expect(page.locator('.track-row').first()).toBeVisible();
    await openQueue(page);
    const empty = await page.evaluate(RAILS);
    expect(empty.position.disabled, 'the empty queue has no position to seek').toBe(true);
    expect(empty.position.valueText, 'an unknown duration is announced as unknown').toBe('Duration unknown');
    expect(empty.position.max, 'an unknown duration is not a real length').toBe('1');
    expect(empty.volume.valueText, 'the volume announces a percentage').toMatch(/^\d+%$/);
    expect(empty.volume.label.text, 'the volume rail has a visible name').toBe('Volume');
    expect(empty.position.label.text, 'the position rail has a visible name').toBe('Playback position');
    evidence.emptyQueue = empty;
    await page.screenshot({ path: `${output}/queue-empty-1284x856.png`, animations: 'disabled' });
    // Escape closes the drawer and the focus goes back to the connected trigger.
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);
    await expect.poll(() => page.evaluate(() => document.activeElement?.getAttribute('aria-label') || '')).toMatch(/^Show playback queue/);
    const returned = await page.evaluate(() => {
      const element = document.activeElement;
      return { label: element?.getAttribute('aria-label'), connected: element?.isConnected === true, tag: element?.tagName };
    });
    expect(returned.connected, 'the trigger is still connected').toBe(true);
    expect(returned.label, 'the focus went back to the queue trigger').toMatch(/^Show playback queue/);
    evidence.escape.push({ scene: 'empty queue', active: returned });

    // --- The "Open library" exit leaves the surface for good. The dialog
    // leaves the top layer before it leaves the DOM, so the focus goes back
    // to the control that opened it and the navigation does not strand it
    // on the document body.
    await navigate(page, 'Home');
    await openQueue(page);
    const openLibrary = page.locator('#queue-drawer-dialog').getByRole('button', { name: 'Open library', exact: true });
    await expect(openLibrary).toBeVisible();
    await openLibrary.click();
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);
    await expect(page.locator('.track-row').first()).toBeVisible();
    await expect.poll(() => page.evaluate(() => {
      const element = document.activeElement;
      return { label: element?.getAttribute('aria-label') || '', connected: element?.isConnected === true, tag: element?.tagName };
    }), { message: 'the focus is handed back after the navigation' })
      .toMatchObject({ label: /^Show playback queue/, connected: true });
    const library = await page.evaluate(() => {
      const element = document.activeElement;
      return {
        tag: element?.tagName, label: element?.getAttribute('aria-label'),
        connected: element?.isConnected === true,
      };
    });
    expect(library.connected, 'the trigger is still connected').toBe(true);
    expect(library.tag, 'the focus is not on the document body').not.toBe('BODY');
    expect(library.label, 'the focus went back to the queue trigger').toMatch(/^Show playback queue/);
    evidence.escape.push({ scene: 'open library', active: library });
    await page.screenshot({ path: `${output}/queue-open-library-1284x856.png`, animations: 'disabled' });

    // --- A real queue of the real catalog: two occurrences, nothing played.
    const add = page.getByRole('button', { name: /^Add .* to playback queue$/ });
    await add.nth(0).click();
    await add.nth(1).click();
    await expect(page.locator('.feedback-stack .alert.success')).toBeVisible();

    for (const theme of ['light', 'dark']) {
      await setTheme(page, theme);
      await openQueue(page);
      await expect(page.locator('.queue-row')).toHaveCount(2);
      await expect(page.locator('#queue-position')).toBeEnabled();

      // The rails: a real 44px target, the full width of the column and a
      // visible name bound to the control.
      const rails = await page.evaluate(RAILS);
      for (const rail of [rails.volume, rails.position]) {
        expect(rail.height, `${rail.id} height`).toBeGreaterThanOrEqual(43.5);
        expect(rail.height, `${rail.id} height`).toBeLessThanOrEqual(44.5);
        expect(Math.abs(rail.width - rail.columnWidth), `${rail.id} width vs column`).toBeLessThanOrEqual(1);
        expect(rail.style.display, `${rail.id} display`).toBe('block');
        expect(rail.style.minWidth, `${rail.id} min width`).toBe('0px');
        expect(rail.label, `${rail.id} has a label`).not.toBeNull();
        expect(rail.label.visible, `${rail.id} label is visible`).toBe(true);
        expect(rail.label.bound, `${rail.id} label names the control`).toBe(true);
        expect(rail.label.text.length, `${rail.id} label has text`).toBeGreaterThan(0);
        expect(rail.label.fontSize, `${rail.id} label size`).toBe('12px');
        expect(rail.label.fontWeight, `${rail.id} label weight`).toBe('500');
      }
      // The real length of the real track, never the technical maximum. The
      // rail's own range is the server's real duration of the queued
      // occurrence, which is one of the catalog's real durations.
      const liveMax = Number(rails.position.max);
      expect(durations.some(duration => Math.abs(duration - liveMax * 1000) <= 2),
        `the rail spans a real server duration (${liveMax}s of ${durations.join('/')}ms)`).toBe(true);
      const total = mmss(liveMax);
      expect(rails.position.valueText, 'the position announces a real pair').toBe(`0:00 of ${total}`);
      expect(rails.position.valueText).not.toBe('Duration unknown');
      expect(rails.volume.valueText, 'the volume announces a percentage').toMatch(/^\d+%$/);
      evidence.rails[theme] = rails;
      // The two decorative roles of the rows are semantic roles with a value
      // per theme, not a hardcoded pistachio that only reads on the dark
      // panel.
      const roles = await page.evaluate(ROW_ROLES);
      expect(roles, `${theme} the row roles are resolved`).not.toBeNull();
      expect(roles.current, `${theme} the current row has a role`).not.toBeNull();
      expect(roles.next, `${theme} the next occurrence has a role`).not.toBeNull();
      expect(roles.current.value, `${theme} the current row is a role, not a literal`).toMatch(/^#|rgba/);
      evidence.rowRoles = evidence.rowRoles || {};
      evidence.rowRoles[theme] = roles;
      await page.screenshot({ path: `${output}/queue-open-1284x856-${theme}.png`, animations: 'disabled' });

      // The keyboard order of the drawer's own controls, walked with real
      // Tab presses. Every control the ring is measured on is in that order.
      const order = await walkOrder(page);
      evidence.order = evidence.order || {};
      evidence.order[theme] = order;
      expect(order.length, `${theme} the drawer is reachable with Tab`).toBeGreaterThan(8);
      expect(order[0].cls, `${theme} the drawer starts on its close control`).toContain('queue-close');
      for (const [selector, name] of [['#queue-volume', 'volume rail'], ['#queue-position', 'position rail']]) {
        const id = selector.slice(1);
        expect(order.some(entry => entry.id === id), `${theme} the ${name} is in the Tab order`).toBe(true);
      }
      expect(order.some(entry => entry.cls.includes('queue-track')), `${theme} a queue row is in the Tab order`).toBe(true);
      expect(order.some(entry => entry.label === 'Move track up'), `${theme} the move action is in the Tab order`).toBe(true);
      expect(order.some(entry => entry.label === 'Remove queued track'), `${theme} the remove action is in the Tab order`).toBe(true);

      // The ring: every control the keyboard reaches, measured as the browser
      // paints it.
      const targets = [
        ['.queue-close', 'close'],
        ['.queue-track', 'queue-track'],
        ['.queue-row:nth-child(2) .queue-row-actions button:nth-child(1)', 'move-up'],
        ['.queue-row:nth-child(2) .queue-row-actions button:nth-child(3)', 'remove'],
        ['#queue-volume', 'volume-rail'],
        ['#queue-position', 'position-rail'],
      ];
      const readings = [];
      for (const [selector, name] of targets) {
        await focusControl(page, selector);
        const ring = await page.evaluate(RING);
        expect(ring, `${theme} ${name} is focused`).not.toBeNull();
        expect(ring.focusVisible, `${theme} ${name} matches :focus-visible`).toBe(true);
        expect(ring.outlineStyle, `${theme} ${name} ring style`).toBe('solid');
        expect(ring.outlineWidth, `${theme} ${name} ring width`).toBe(2);
        expect(ring.outlineOffset, `${theme} ${name} ring offset`).toBe(3);
        expect(ring.ratio, `${theme} ${name} ring contrast on ${ring.surface}`).toBeGreaterThanOrEqual(3);
        const margin = ring.outlineOffset + ring.outlineWidth;
        expect(ring.rect.x - margin, `${theme} ${name} ring inside the drawer`).toBeGreaterThanOrEqual(ring.dialogRect.x - 0.5);
        expect(ring.rect.right + margin, `${theme} ${name} ring inside the drawer`).toBeLessThanOrEqual(ring.dialogRect.right + 0.5);
        readings.push({ name, selector, ...ring });
        await page.screenshot({ path: `${output}/queue-focus-${name}-1284x856-${theme}.png`, animations: 'disabled' });
      }
      evidence.themes[theme] = readings;

      // The values: real keypresses on the native rails.
      await focusControl(page, '#queue-volume');
      await page.keyboard.press('Home');
      await expect.poll(() => page.locator('#queue-volume').getAttribute('aria-valuetext')).toBe('0%');
      await page.keyboard.press('ArrowRight');
      await expect.poll(() => page.locator('#queue-volume').getAttribute('aria-valuetext')).toBe('1%');
      await page.keyboard.press('End');
      await expect.poll(() => page.locator('#queue-volume').getAttribute('aria-valuetext')).toBe('100%');
      await page.keyboard.press('Home');
      await expect.poll(() => page.locator('#queue-volume').getAttribute('aria-valuetext')).toBe('0%');
      const volumeValue = await page.locator('#queue-volume').inputValue();

      // The position rail announces the real position of the real track.
      await focusControl(page, '#queue-position');
      await page.keyboard.press('Home');
      await expect.poll(() => page.locator('#queue-position').getAttribute('aria-valuetext')).toBe(`0:00 of ${total}`);
      await page.keyboard.press('ArrowRight');
      await expect.poll(() => page.locator('#queue-position').inputValue()).toBe('0.1');
      await page.keyboard.press('End');
      await expect.poll(() => page.locator('#queue-position').getAttribute('aria-valuetext')).toBe(`${total} of ${total}`);
      await page.keyboard.press('Home');
      await expect.poll(() => page.locator('#queue-position').getAttribute('aria-valuetext')).toBe(`0:00 of ${total}`);
      const positionValue = await page.locator('#queue-position').inputValue();
      evidence.values[theme] = { total, volumeValue, positionValue, positionMax: await page.locator('#queue-position').getAttribute('max') };

      // Escape closes the drawer and hands the focus back to the trigger.
      await page.keyboard.press('Escape');
      await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);
      await expect.poll(() => page.evaluate(() => document.activeElement?.getAttribute('aria-label') || '')).toMatch(/^Show playback queue/);
      const active = await page.evaluate(() => {
        const element = document.activeElement;
        return {
          tag: element?.tagName, label: element?.getAttribute('aria-label'),
          connected: element?.isConnected === true, className: String(element?.className || ''),
        };
      });
      expect(active.connected, 'the trigger is still connected').toBe(true);
      expect(active.label, 'the focus went back to the queue trigger').toMatch(/^Show playback queue/);
      evidence.escape.push({ scene: theme, active });
      await page.screenshot({ path: `${output}/queue-focus-returned-1284x856-${theme}.png`, animations: 'disabled' });
    }

    // --- The sleep countdown ticks every second, so it must not sit in a
    // polite live region: a screen reader would read "4:58", "4:57", "4:56"
    // without a pause. It keeps a name that carries the remaining time.
    await openQueue(page);
    // The drawer starts without an announcement: the live region is a
    // permanent node and its text is empty before the first transition.
    const silent = await page.evaluate(STATUS);
    expect(silent.present, 'the live region is in the DOM before any transition').toBe(true);
    expect(silent.count, 'the drawer has exactly one live region').toBe(1);
    expect(silent.line, 'the drawer starts without an announcement').toBe('');
    expect(silent.texts, 'the live region holds exactly one text').toBe(1);
    expect(silent.elements, 'the live region holds no element children').toBe(0);
    // The empty line is silent and takes no space: the sleep row keeps the
    // geometry it had without the line.
    const emptyBox = await page.evaluate(STATUS_BOX);
    expect(emptyBox, 'the announcement line is in the drawer').not.toBeNull();
    expect(emptyBox.collapsed, 'the empty line is marked as empty').toBe(true);
    expect(emptyBox.height, 'the empty line takes no height').toBe(0);
    expect(emptyBox.marginTop, 'the empty line takes no margin').toBe('0px');
    expect(Math.abs(emptyBox.wrapperHeight - emptyBox.rowHeight),
      'the sleep row and the line share one wrapper and the empty line adds no space').toBeLessThanOrEqual(1);
    await page.locator('#sleep-timer').selectOption('5');
    await expect(page.locator('#queue-drawer-dialog [role="status"]')).toHaveText('Sleep timer set for 5 minutes.');
    const started = await page.evaluate(STATUS);
    expect(started.count, 'the drawer has exactly one announcement surface').toBe(1);
    expect(started.present, 'the live region is still the same permanent node').toBe(true);
    expect(started.line, 'the start is announced once').toBe('Sleep timer set for 5 minutes.');
    expect(started.texts, 'the line carries exactly one text').toBe(1);
    expect(started.elements, 'the line carries no element children').toBe(0);
    const startedBox = await page.evaluate(STATUS_BOX);
    expect(startedBox.collapsed, 'the line is not empty any more').toBe(false);
    expect(startedBox.height, 'the line is a line of the drawer').toBeGreaterThan(0);
    expect(startedBox.marginTop, 'the line keeps the drawer gap').toBe('14px');
    // The background behind the open modal is out of reach: the root's
    // receipt cannot announce the same message from there.
    const blocked = await page.evaluate(BACKGROUND_BLOCKED);
    expect(blocked.moved, 'the background behind the modal is inert').toBe(false);
    expect(blocked.inside, 'the focus stays inside the modal').toBe(true);
    await expect(page.locator('.sleep-timer small')).toBeVisible();
    const countdown = await page.evaluate(COUNTDOWN);
    expect(countdown.role, 'the ticking countdown is a timer, not a status').toBe('timer');
    expect(countdown.live, 'the ticking countdown is not announced').toBe('off');
    expect(countdown.name, 'the countdown has an accessible name').toMatch(/^Pauses in \d+:\d{2}$/);
    expect(countdown.text, 'the visible countdown carries the remaining time').toBe(countdown.name);
    expect(countdown.liveAbove, 'no live region wraps the ticking countdown').toEqual([]);
    await page.screenshot({ path: `${output}/queue-sleep-countdown-1284x856.png`, animations: 'disabled' });
    // The countdown really ticks: the value changes while it stays outside
    // every live region.
    await expect.poll(async () => (await page.evaluate(COUNTDOWN)).name, { timeout: 8000 })
      .not.toBe(countdown.name);
    const ticked = await page.evaluate(COUNTDOWN);
    expect(ticked.role, 'the ticking countdown is still a timer').toBe('timer');
    expect(ticked.live, 'the ticking countdown is still not announced').toBe('off');
    expect(ticked.liveAbove, 'the ticking countdown is still outside every live region').toEqual([]);
    // A tick is not an announcement: the one status line keeps its text.
    const afterTick = await page.evaluate(STATUS);
    expect(afterTick.line, 'a tick does not change the announcement').toBe('Sleep timer set for 5 minutes.');
    expect(afterTick.count, 'a tick did not add a second announcement').toBe(1);
    evidence.countdown = { first: countdown, ticked, announcement: afterTick };
    // The live region is a permanent node: it was in the DOM before the
    // first transition and only its text changed.
    evidence.countdown.statusAlwaysPresent = silent.present && silent.count === 1 && started.present;
    evidence.countdown.statusBefore = silent;
    evidence.countdown.statusBoxEmpty = emptyBox;
    evidence.countdown.statusBoxAfter = startedBox;
    // The cancel is one meaningful message on the same line.
    await page.getByRole('button', { name: 'Cancel sleep timer', exact: true }).click();
    await expect(page.locator('#queue-drawer-dialog [role="status"]')).toHaveText('Sleep timer canceled.');
    await expect(page.locator('.sleep-timer small')).toHaveCount(0);
    const canceled = await page.evaluate(STATUS);
    expect(canceled.count, 'the cancel reuses the same announcement surface').toBe(1);
    expect(canceled.line, 'the cancel is announced once').toBe('Sleep timer canceled.');
    evidence.countdown.canceled = canceled;
    // The drawer leaves the surface before the next journey opens it again.
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);

    // --- A drawer that re-renders under the keyboard still hands the focus
    // back. The countdown ticks every second and the drawer is re-rendered
    // while the focus is inside it.
    await openQueue(page);
    await page.locator('#sleep-timer').selectOption('5');
    await expect(page.locator('.sleep-timer small')).toBeVisible();
    await page.keyboard.press('Tab');
    const inside = await page.evaluate(() => {
      const dialog = document.querySelector('#queue-drawer-dialog');
      return !!(dialog && dialog.contains(document.activeElement));
    });
    expect(inside, 'the focus is inside the drawer').toBe(true);
    const before = await page.evaluate(COUNTDOWN);
    await expect.poll(async () => (await page.evaluate(COUNTDOWN)).name, { timeout: 8000 }).not.toBe(before.name);
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);
    await expect.poll(() => page.evaluate(() => document.activeElement?.getAttribute('aria-label') || '')).toMatch(/^Show playback queue/);
    const rerendered = await page.evaluate(() => {
      const element = document.activeElement;
      return { label: element?.getAttribute('aria-label'), connected: element?.isConnected === true, tag: element?.tagName };
    });
    expect(rerendered.connected, 'the trigger is still connected').toBe(true);
    expect(rerendered.tag, 'the focus is not on the document body').not.toBe('BODY');
    expect(rerendered.label, 'the focus went back to the queue trigger').toMatch(/^Show playback queue/);
    evidence.escape.push({ scene: 're-render inside the drawer', active: rerendered });

    // --- The expiry needs the real countdown to reach zero. The page clock
    // is the test's own lever: the app's deadline and its one-second tick
    // both read the page's clock, so the timer really expires.
    await openQueue(page);
    await page.clock.install();
    await page.locator('#sleep-timer').selectOption('5');
    await expect(page.locator('#queue-drawer-dialog [role="status"]')).toHaveText('Sleep timer set for 5 minutes.');
    await page.clock.fastForward(301000);
    await expect(page.locator('#queue-drawer-dialog [role="status"]')).toHaveText('Sleep timer reached zero and paused playback.');
    const expired = await page.evaluate(STATUS);
    expect(expired.count, 'the expiry reuses the same announcement surface').toBe(1);
    expect(expired.line, 'the expiry is announced once').toBe('Sleep timer reached zero and paused playback.');
    expect(expired.live.filter(entry => entry.inside), 'the expiry is announced from inside the drawer only').toHaveLength(1);
    evidence.countdown.expired = expired;
    // The clock flows again before the shot: a frozen page clock would
    // keep the screenshot waiting for animations that never end.
    await page.clock.resume();
    await page.screenshot({ path: `${output}/queue-sleep-expired-1284x856.png`, animations: 'disabled' });
    await page.locator('#sleep-timer').selectOption('0');
    await expect(page.locator('.sleep-timer small')).toHaveCount(0);
    // The drawer leaves the surface before the Now Playing journey.
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);

    // --- The same line in the light theme. The theme control is behind the
    // modal, so the drawer is closed before the theme changes and opened
    // again for the measurement.
    await setTheme(page, 'light');
    await openQueue(page);
    await page.locator('#sleep-timer').selectOption('5');
    await expect(page.locator('#queue-drawer-dialog [role="status"]')).toHaveText('Sleep timer set for 5 minutes.');
    const light = await page.evaluate(STATUS_CONTRAST);
    expect(light, 'the announcement line is painted in the light theme').not.toBeNull();
    expect(light.ratio, `the announcement line on ${light.surface}`).toBeGreaterThanOrEqual(4.5);
    expect(light.fontSize, 'the announcement line is small text').toBe('12px');
    evidence.countdown.light = light;
    await page.screenshot({ path: `${output}/queue-sleep-status-1284x856-light.png`, animations: 'disabled' });
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);
    await setTheme(page, 'dark');

    // --- Now Playing → "View full queue" → Escape: the sheet opens the
    // drawer, and Escape still closes the drawer and leaves the surface.
    await openQueue(page);
    await page.getByRole('button', { name: /^Play queue item 1:/ }).click();
    await expect(page.getByRole('button', { name: 'Open now playing', exact: true })).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);
    await page.getByRole('button', { name: 'Open now playing', exact: true }).click();
    await expect(page.locator('#now-playing-dialog')).toBeVisible();
    await page.getByRole('button', { name: 'View full queue', exact: true }).click();
    await expect(page.locator('#queue-drawer-dialog')).toBeVisible();
    await expect(page.locator('#now-playing-dialog')).toHaveCount(0);
    await page.screenshot({ path: `${output}/queue-from-now-playing-1284x856.png`, animations: 'disabled' });
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);
    await expect.poll(() => page.evaluate(() => {
      const element = document.activeElement;
      return element && element.tagName !== 'BODY' && element.isConnected === true;
    }), { message: 'the focus is handed back to a connected control' }).toBe(true);
    const fromSheet = await page.evaluate(() => {
      const element = document.activeElement;
      return {
        tag: element?.tagName, label: element?.getAttribute('aria-label'),
        connected: element?.isConnected === true, className: String(element?.className || ''),
      };
    });
    expect(fromSheet.connected, 'the focus is on a connected control').toBe(true);
    expect(fromSheet.tag, 'the focus is not on the document body').not.toBe('BODY');
    expect(fromSheet.label, 'the focus went back to the now-playing trigger').toBe('Open now playing');
    evidence.escape.push({ scene: 'now playing → view full queue', active: fromSheet });
    await page.screenshot({ path: `${output}/queue-focus-returned-from-now-playing-1284x856.png`, animations: 'disabled' });

    // --- The drawer freezes its own trigger. A row menu that is open
    // behind the drawer keeps its own frozen trigger, so Escape hands the
    // focus back to the control that opened the drawer and never to the
    // row's More button. At this width the row menu has no backdrop, so it
    // can stay open while the drawer opens.
    await page.setViewportSize({ width: 1284, height: 856 });
    await navigate(page, 'Library');
    await expect(page.locator('.track-row').first()).toBeVisible();
    const more = page.getByRole('button', { name: /^More actions for / }).first();
    await more.click({ timeout: 15000 });
    await expect(page.locator('.track-menu-panel')).toBeVisible();
    const drawerTrigger = await queueTrigger(page);
    await drawerTrigger.click({ timeout: 15000 });
    await expect(page.locator('#queue-drawer-dialog')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);
    await expect.poll(() => page.evaluate(() => document.activeElement?.getAttribute('aria-label') || ''))
      .toMatch(/^Show playback queue/);
    const afterMenu = await page.evaluate(() => {
      const element = document.activeElement;
      return {
        tag: element?.tagName, label: element?.getAttribute('aria-label'),
        connected: element?.isConnected === true,
        menuOpen: !!document.querySelector('.track-menu-panel'),
      };
    });
    expect(afterMenu.connected, 'the drawer trigger is still connected').toBe(true);
    expect(afterMenu.tag, 'the focus is not on the document body').not.toBe('BODY');
    expect(afterMenu.label, 'the focus went back to the drawer trigger').toMatch(/^Show playback queue/);
    expect(afterMenu.label, 'the focus did not go back to the row menu').not.toMatch(/^More actions for /);
    await page.screenshot({ path: `${output}/queue-focus-returned-after-menu-1284x856.png`, animations: 'disabled' });

    // --- The row menu keeps its own trigger: closing the menu hands the
    // focus back to the More button, and the picker that the menu opens
    // hands the focus back to the same control.
    await more.click({ timeout: 15000 });
    await expect(page.locator('.track-menu-panel')).toHaveCount(0);
    await expect.poll(() => page.evaluate(() => document.activeElement?.getAttribute('aria-label') || ''))
      .toMatch(/^More actions for /);
    const menuClosed = await page.evaluate(() => {
      const element = document.activeElement;
      return { label: element?.getAttribute('aria-label'), connected: element?.isConnected === true };
    });
    expect(menuClosed.connected, 'the More button is still connected').toBe(true);
    expect(menuClosed.label, 'the focus went back to the More button').toMatch(/^More actions for /);
    await more.click({ timeout: 15000 });
    await expect(page.locator('.track-menu-panel')).toBeVisible();
    // The row menu is a real menu: its items are menuitems, not buttons.
    await page.getByRole('menuitem', { name: 'Add to playlist…', exact: true }).click({ timeout: 15000 });
    await expect(page.locator('#playlist-picker')).toBeVisible();
    await page.keyboard.press('Escape');
    // The picker is mounted for the app's whole lifetime: it closes, it is
    // not removed.
    await expect(page.locator('#playlist-picker')).toBeHidden();
    await expect.poll(() => page.evaluate(() => document.activeElement?.getAttribute('aria-label') || ''))
      .toMatch(/^More actions for /);
    const pickerClosed = await page.evaluate(() => {
      const element = document.activeElement;
      return { label: element?.getAttribute('aria-label'), connected: element?.isConnected === true };
    });
    expect(pickerClosed.connected, 'the More button is still connected').toBe(true);
    expect(pickerClosed.label, 'the focus went back to the More button').toMatch(/^More actions for /);
    await page.screenshot({ path: `${output}/queue-focus-returned-to-more-1284x856.png`, animations: 'disabled' });
    evidence.focus = {
      ordinary: evidence.escape.find(entry => entry.scene === 'empty queue')?.active ?? null,
      nowPlaying: fromSheet,
      menuOpen: afterMenu,
      menuClosed,
      pickerClosed,
    };

    // --- Reflow: the rails keep their geometry at every width, and the
    // running countdown stays inside the drawer at every width too.
    await openQueue(page);
    await page.locator('#sleep-timer').selectOption('5');
    await expect(page.locator('.sleep-timer small')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);
    for (const [width, height] of [[1284, 856], [1440, 900], [1728, 1080], [390, 844], [360, 800], [320, 700]]) {
      await page.setViewportSize({ width, height });
      await page.waitForTimeout(400);
      await openQueue(page);
      // The empty live region takes no space at any width: the sleep row
      // and the line share one wrapper, and the empty line is collapsed.
      const emptyLine = await page.evaluate(STATUS_BOX);
      expect(emptyLine, `${width}px the announcement line is in the drawer`).not.toBeNull();
      expect(emptyLine.collapsed, `${width}px the empty line is collapsed`).toBe(true);
      expect(emptyLine.height, `${width}px the empty line takes no height`).toBe(0);
      expect(emptyLine.marginTop, `${width}px the empty line takes no margin`).toBe('0px');
      expect(Math.abs(emptyLine.wrapperHeight - emptyLine.rowHeight),
        `${width}px the empty line adds no space`).toBeLessThanOrEqual(1);
      const rails = await page.evaluate(RAILS);
      // The current row really paints the semantic role, not a hardcoded
      // pistachio.
      const painted = await page.evaluate(ROW_ROLES);
      expect(painted, `${width}px the row roles are resolved`).not.toBeNull();
      if (painted.applied) {
        expect(painted.applied.sameRole, `${width}px the current row paints the role`).toBe(true);
      }
      for (const rail of [rails.volume, rails.position]) {
        expect(rail.height, `${width}px ${rail.id} height`).toBeGreaterThanOrEqual(43.5);
        expect(Math.abs(rail.width - rail.columnWidth), `${width}px ${rail.id} width vs column`).toBeLessThanOrEqual(1);
        expect(rail.label.visible, `${width}px ${rail.id} label is visible`).toBe(true);
      }
      // The sleep row lives at the bottom of the drawer's own scroller, so
      // the countdown is brought into view before it is measured and shot.
      await page.evaluate(() => {
        const panel = document.querySelector('#queue-drawer-dialog .queue-panel');
        if (panel) panel.scrollTop = panel.scrollHeight;
      });
      const box = await page.evaluate(COUNTDOWN_BOX);
      expect(box, `${width}px the countdown is in the drawer`).not.toBeNull();
      expect(box.visible, `${width}px the countdown is visible`).toBe(true);
      expect(box.text, `${width}px the countdown shows the remaining time`).toMatch(/^Pauses in \d+:\d{2}$/);
      expect(box.width, `${width}px the countdown has a width`).toBeGreaterThan(0);
      expect(box.left, `${width}px the countdown stays inside the drawer`).toBeGreaterThanOrEqual(box.panelLeft - 0.5);
      expect(box.right, `${width}px the countdown stays inside the drawer`).toBeLessThanOrEqual(box.panelRight + 0.5);
      expect(box.panelOverflowX, `${width}px the drawer does not scroll sideways`).toBeLessThanOrEqual(1);
      expect(box.formOverflowX, `${width}px the sleep row does not scroll sideways`).toBeLessThanOrEqual(1);
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
      expect(overflow, `${width}px document overflow`).toBeLessThanOrEqual(3);
      evidence.viewports.push({ width, height, rails, overflow, countdown: box, status: emptyLine });
      await page.screenshot({ path: `${output}/queue-reflow-${width}x${height}.png`, animations: 'disabled' });
      await page.keyboard.press('Escape');
      await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);
    }
    // The timer is cancelled again, so the countdown does not outlive the run.
    await openQueue(page);
    await page.locator('#sleep-timer').selectOption('0');
    await expect(page.locator('.sleep-timer small')).toHaveCount(0);
    await page.keyboard.press('Escape');
    await expect(page.locator('#queue-drawer-dialog')).toHaveCount(0);

    await writeFile(`${output}/dr07-evidence.json`, JSON.stringify(evidence, null, 2));
  } finally { await runtime.close(); }
});
