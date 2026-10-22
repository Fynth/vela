/**
 * DR01 acceptance: one glass and motion vocabulary for the web layers.
 *
 * DR01 is the token layer, not the surfaces. Nothing in this package is
 * applied to a component: `backdrop-filter` and the new transitions belong
 * to DR02/DR03. The gate therefore proves four things and nothing more.
 *
 *  1. The vocabulary is declared once. The four themed glass colours are
 *     declared in the canonical dark root and re-pointed in the light root;
 *     the three blur radii, the eight durations, the two easing curves and
 *     the text alias are declared exactly once, in the canonical root. The
 *     stylesheet is read with node:fs and the declarations are counted, so
 *     a second root palette or a duplicated light block fails here instead
 *     of drifting later.
 *
 *  2. The roles resolve in the running application. Every role is read
 *     from the live document with getComputedStyle in the dark theme and in
 *     the light theme. The spec never launches a browser of its own: it
 *     uses the page fixture, so `npx playwright test --browser=chromium`
 *     measures Chromium and `--browser=firefox` measures Firefox, and the
 *     engine name is recorded in the artifact.
 *
 *  3. The reduced-motion query still zeroes the eight durations and the
 *     existing global animation/transition override is still in force. A
 *     zero token cannot stop a JS or Web Animations driver, so the media
 *     query itself is exercised rather than inferred from the token.
 *
 *  4. The composition did not move. The body size, the sidebar, topbar
 *     and player roles, the player band's height and the horizontal
 *     overflow are measured at 1284×856 and 390×844 in both themes, and
 *     the frames are kept.
 *
 * The spec owns its server, its account and its real uploaded tracks. It
 * shares no state with any other spec file and does not depend on the
 * order in which the other files run.
 *
 * Artifacts (the path is used exactly as given, relative to the repository
 * root):
 *   target/checks/dr01-tokens/<engine>/tokens.json    the measured roles
 *   target/checks/dr01-tokens/<engine>/geometry.json   the measured geometry
 *   target/checks/dr01-tokens/shots/<engine>/*.png    the rendered frames
 */
import { test, expect } from '@playwright/test';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { navigate, settle, setupWithTracks, startServer } from './dr04-density.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const output = resolve(root, process.env.VELA_DR01_EVIDENCE || 'target/checks/dr01-tokens');
const stylesheet = resolve(root, 'apps/web/style.css');

/** The eight duration roles, in the order the plan lists them. */
const DURATIONS = [
  ['--vela-motion-instant', '80ms'],
  ['--vela-motion-quick', '120ms'],
  ['--vela-motion-overlay', '140ms'],
  ['--vela-motion-exit', '100ms'],
  ['--vela-motion-panel', '180ms'],
  ['--vela-motion-skeleton', '1800ms'],
  ['--vela-motion-reorder', '150ms'],
  ['--vela-motion-artwork', '140ms'],
];

/** The three blur radii and the two easing curves. */
const BLURS = [
  ['--vela-glass-blur', '16px'],
  ['--vela-glass-blur-mobile', '12px'],
  ['--vela-glass-blur-topbar', '12px'],
];
const EASES = [
  ['--vela-ease-enter', 'ease-out'],
  ['--vela-ease-state', 'ease-in-out'],
];

/** The four themed glass colours: dark in the root, re-pointed in light. */
const GLASS_COLOURS = [
  ['--vela-glass-bg', '#131513d9', '#f4f7f3e6'],
  ['--vela-glass-border', '#ffffff14', '#14271c14'],
  ['--vela-glass-highlight', '#ffffff0f', '#ffffffb3'],
  ['--vela-glass-shadow', '0 18px 50px #00000059', '0 18px 50px #21372924'],
];

/** The text alias: one declaration, and it points at the existing role. */
const GLASS_TEXT = '--vela-glass-text';
/** The role the alias points at. It is read for the comparison only. */
const TEXT_ROLE = '--vela-text';

const ALL_ROLES = [
  ...GLASS_COLOURS.map(([role]) => role),
  ...BLURS.map(([role]) => role),
  GLASS_TEXT,
  ...DURATIONS.map(([role]) => role),
  ...EASES.map(([role]) => role),
];

/**
 * Reads the roles from the live document. The values are the computed
 * custom properties, so `var()` references are already substituted.
 */
const READ_ROLES = (roles) => {
  const style = getComputedStyle(document.documentElement);
  const values = {};
  for (const role of roles) values[role] = style.getPropertyValue(role).trim();
  return values;
};

/**
 * Resolves a role to a real colour through a throwaway probe element, so
 * the comparison does not depend on how an engine reports a custom
 * property that holds a `var()` reference. The probe is removed again and
 * never takes part in the layout.
 */
const RESOLVE_COLOUR = (role) => {
  const probe = document.createElement('span');
  probe.style.position = 'fixed';
  probe.style.left = '-9999px';
  probe.style.top = '0';
  probe.style.width = '0';
  probe.style.height = '0';
  probe.style.color = 'var(' + role + ')';
  document.body.appendChild(probe);
  const colour = getComputedStyle(probe).color;
  probe.remove();
  return colour;
};

/** The alpha of an eight-digit hex colour, as the fraction it declares. */
const alphaOf = (hex) => {
  const digits = hex.trim().replace('#', '');
  expect(digits.length, 'the glass colour is an eight-digit hex').toBe(8);
  return parseInt(digits.slice(6, 8), 16) / 255;
};

/**
 * Counts the declarations of a role in a stylesheet fragment.
 *
 * The count is anchored to the `role:` token, not to the start of a line, so
 * a compact declaration written on one line (`:root { --vela-glass-bg: red; }`)
 * cannot slip past the uniqueness check. Comments are stripped first, because
 * the DR01 comments name the roles without declaring them. The `:` must
 * follow the role directly, which keeps `--vela-glass-blur` from matching
 * `--vela-glass-blur-mobile`.
 */
const stripComments = (text) => text.replace(/\/\*[\s\S]*?\*\//g, '');
const countDeclarations = (text, role) =>
  (stripComments(text).match(new RegExp(role.replace(/-/g, '\\-') + '\\s*:', 'g')) || []).length;

/** Extracts a brace-balanced block that starts at the given marker. */
const blockAt = (source, marker) => {
  const start = source.indexOf(marker);
  if (start < 0) return null;
  const open = source.indexOf('{', start);
  let depth = 0;
  for (let at = open; at < source.length; at += 1) {
    if (source[at] === '{') depth += 1;
    else if (source[at] === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(start, at + 1);
    }
  }
  return null;
};

test('DR01 the stylesheet declares one glass and motion vocabulary', async () => {
  const source = await readFile(stylesheet, 'utf8');

  // The counting rule: a role is declared once in the canonical layer and,
  // for the eight durations, once more inside the reduce block, where it is
  // zeroed. The canonical layer is everything before the DR01 reduce block,
  // so the two declarations of a duration are counted separately and the
  // second one is proved to be the reduced-motion override.
  const reduceMarker = '/* DR01: reduce zeroes the eight duration roles';
  const cut = source.indexOf(reduceMarker);
  expect(cut, 'the DR01 reduce block is present').toBeGreaterThan(0);
  const canonical = source.slice(0, cut);
  const reduce = source.slice(cut);
  expect(reduce, 'the reduce block closes its media query').toContain('@media (prefers-reduced-motion: reduce)');

  // The four themed colours: exactly two declarations, the dark root and
  // the light root. A third one would be a second palette.
  for (const [role] of GLASS_COLOURS) {
    expect(countDeclarations(canonical, role), `${role} is declared in dark and light`).toBe(2);
  }

  // The blur radii, the easing curves and the text alias: exactly one
  // declaration each. The light theme inherits them.
  for (const [role] of [...BLURS, ...EASES]) {
    expect(countDeclarations(canonical, role), `${role} is declared once`).toBe(1);
  }
  expect(countDeclarations(canonical, GLASS_TEXT), `${GLASS_TEXT} is declared once`).toBe(1);

  // The eight durations: one declaration in the canonical layer and one
  // inside the reduce block, and nowhere else.
  for (const [role] of DURATIONS) {
    expect(countDeclarations(canonical, role), `${role} is declared once in the canonical layer`).toBe(1);
    expect(countDeclarations(reduce, role), `${role} is zeroed once in the reduce block`).toBe(1);
  }
  expect(countDeclarations(source, '--vela-ease-enter'), 'the easing curves are never re-declared').toBe(1);
  expect(countDeclarations(source, '--vela-ease-state'), 'the easing curves are never re-declared').toBe(1);
  expect(countDeclarations(source, GLASS_TEXT), 'the text alias is never re-declared').toBe(1);
  for (const [role] of BLURS) {
    expect(countDeclarations(source, role), `${role} is never re-declared`).toBe(1);
  }

  // The light root re-points the four colours and nothing else: no blur,
  // no duration and no easing curve is repeated there.
  const light = blockAt(canonical, ':root[data-theme="light"] {');
  expect(light, 'the light root is present').not.toBeNull();
  for (const [role] of GLASS_COLOURS) {
    expect(countDeclarations(light, role), `light re-points ${role}`).toBe(1);
  }
  for (const [role] of [...BLURS, ...DURATIONS, ...EASES, [GLASS_TEXT]]) {
    expect(countDeclarations(light, role), `light inherits ${role}`).toBe(0);
  }

  // The reduce block declares the eight durations and no other role.
  for (const [role] of DURATIONS) {
    expect(reduce, `${role} is zeroed`).toMatch(new RegExp(role + '\\s*:\\s*0ms'));
  }
  for (const [role] of [...GLASS_COLOURS, ...BLURS, ...EASES, [GLASS_TEXT]]) {
    expect(countDeclarations(reduce, role), `the reduce block does not touch ${role}`).toBe(0);
  }

  // The existing global override is still there and was not weakened.
  expect(source, 'the global reduced-motion override survives').toContain(
    '@media(prefers-reduced-motion:reduce){*,*:before,*:after{animation:none!important;transition:none!important;scroll-behavior:auto!important}}',
  );

  // The reduce block is a real media query whose body is brace-balanced, and
  // it sits after the light root: a duration declared anywhere else would be
  // the canonical value, not the reduced one.
  const reduceBlock = blockAt(reduce, '@media (prefers-reduced-motion: reduce)');
  expect(reduceBlock, 'the DR01 reduce block is one balanced media query').not.toBeNull();
  expect(reduceBlock, 'the reduce block contains all eight zero durations')
    .toMatch(/--vela-motion-artwork\s*:\s*0ms/);
  expect(source.indexOf(reduceMarker), 'the reduce block comes after the light root')
    .toBeGreaterThan(source.indexOf(':root[data-theme="light"]'));

  // The counting rule itself is asserted, not trusted: a compact third
  // declaration on one line must be counted, because a check that only sees
  // declarations at the start of a line would let a second palette through.
  const compact = ':root { --vela-glass-bg: #131513d9; }\n  --vela-glass-bg: red;';
  expect(countDeclarations(compact, '--vela-glass-bg'), 'a compact declaration is counted').toBe(2);
  expect(countDeclarations('/* --vela-glass-bg: red; */', '--vela-glass-bg'), 'a comment is not a declaration').toBe(0);
  expect(countDeclarations('--vela-glass-blur-mobile: 12px;', '--vela-glass-blur'), 'a longer role name is not a match').toBe(0);
});

test('DR01 the roles resolve in dark and light, and light re-points four colours', async ({ page }) => {
  test.setTimeout(240000);
  const engine = page.context().browser().browserType().name();
  const engineOutput = join(output, engine);
  await mkdir(engineOutput, { recursive: true });
  const runtime = await startServer(engineOutput);
  const evidence = { engine, roles: ALL_ROLES, dark: null, light: null };
  try {
    await setupWithTracks(page, runtime, 3);
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible' });

    const setTheme = async (theme) => {
      const current = await page.locator('html').getAttribute('data-theme');
      if (current !== theme) {
        await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
        await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
      }
      await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    };

    // Dark: the canonical layer.
    await setTheme('dark');
    const dark = await page.evaluate(READ_ROLES, [...ALL_ROLES, TEXT_ROLE]);
    const darkText = await page.evaluate(RESOLVE_COLOUR, TEXT_ROLE);
    const darkGlassText = await page.evaluate(RESOLVE_COLOUR, GLASS_TEXT);
    evidence.dark = { tokens: dark, text: darkText, glassText: darkGlassText };

    for (const role of ALL_ROLES) {
      expect(dark[role], `dark resolves ${role}`).not.toBe('');
    }
    expect(dark['--vela-glass-bg'], 'the dark glass background').toBe('#131513d9');
    for (const [role, value] of [...BLURS, ...DURATIONS, ...EASES]) {
      expect(dark[role], `dark ${role}`).toBe(value);
    }
    expect(dark[GLASS_TEXT], 'the glass text alias resolves to the text role')
      .toBe(dark[TEXT_ROLE]);
    expect(darkGlassText, 'the glass text alias paints the text colour').toBe(darkText);

    // Light: the four themed colours are re-pointed, everything else is
    // inherited and must be byte-for-byte the same string.
    await setTheme('light');
    const light = await page.evaluate(READ_ROLES, [...ALL_ROLES, TEXT_ROLE]);
    const lightText = await page.evaluate(RESOLVE_COLOUR, TEXT_ROLE);
    const lightGlassText = await page.evaluate(RESOLVE_COLOUR, GLASS_TEXT);
    evidence.light = { tokens: light, text: lightText, glassText: lightGlassText };

    for (const role of ALL_ROLES) {
      expect(light[role], `light resolves ${role}`).not.toBe('');
    }
    for (const [role, darkValue, lightValue] of GLASS_COLOURS) {
      expect(dark[role], `dark ${role}`).toBe(darkValue);
      expect(light[role], `light ${role}`).toBe(lightValue);
      expect(light[role], `light ${role} differs from dark`).not.toBe(dark[role]);
    }
    for (const [role] of [...BLURS, ...DURATIONS, ...EASES]) {
      expect(light[role], `light inherits ${role} unchanged`).toBe(dark[role]);
    }
    expect(light[GLASS_TEXT], 'light inherits the text alias').toBe(light[TEXT_ROLE]);
    expect(lightGlassText, 'the light glass text paints the light text colour').toBe(lightText);

    // The alpha of the declared background. This is arithmetic on the
    // declaration; it is not a contrast measurement over any artwork, and
    // DR02 owns that question.
    const darkAlpha = alphaOf(dark['--vela-glass-bg']);
    const lightAlpha = alphaOf(light['--vela-glass-bg']);
    expect(darkAlpha, 'the dark glass alpha').toBeCloseTo(217 / 255, 4);
    expect(lightAlpha, 'the light glass alpha').toBeCloseTo(230 / 255, 4);
    expect(darkAlpha, 'the dark glass alpha is at least 0.82').toBeGreaterThanOrEqual(0.82);
    expect(lightAlpha, 'the light glass alpha is at least 0.82').toBeGreaterThanOrEqual(0.82);
    evidence.alpha = { dark: darkAlpha, light: lightAlpha };

    await writeFile(join(engineOutput, 'tokens.json'), JSON.stringify(evidence, null, 2));
  } finally {
    await runtime.close();
  }
});

test('DR01 reduced motion zeroes the durations and keeps the global override', async ({ page }) => {
  test.setTimeout(240000);
  const engine = page.context().browser().browserType().name();
  const engineOutput = join(output, engine);
  await mkdir(engineOutput, { recursive: true });
  const runtime = await startServer(engineOutput);
  try {
    await setupWithTracks(page, runtime, 2);
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible' });

    const readTransitions = () => page.evaluate(() => {
      const pick = (selector) => {
        const element = document.querySelector(selector);
        if (!element) return null;
        return getComputedStyle(element).transitionDuration;
      };
      return { trackRow: pick('.track-row'), button: pick('.button') };
    });

    // Without the media query the existing transitions are real, so the
    // zeroed durations below are the override's work and not the absence
    // of a transition.
    const before = await readTransitions();
    expect(before.trackRow, 'the track row has a transition').not.toBeNull();
    expect(before.button, 'the button has a transition').not.toBeNull();
    expect(parseFloat(before.trackRow), 'the track row transition is a real duration').toBeGreaterThan(0);
    expect(parseFloat(before.button), 'the button transition is a real duration').toBeGreaterThan(0);

    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.waitForTimeout(200);

    const reduced = await page.evaluate(READ_ROLES, ALL_ROLES);
    for (const [role] of DURATIONS) {
      expect(reduced[role], `reduce zeroes ${role}`).toBe('0ms');
    }
    // The easing curves are not durations: they stay as they are.
    for (const [role, value] of EASES) {
      expect(reduced[role], `reduce keeps ${role}`).toBe(value);
    }

    const after = await readTransitions();
    for (const [name, value] of Object.entries(after)) {
      expect(value, `${name} has a transition duration`).not.toBeNull();
      for (const part of value.split(',')) {
        expect(parseFloat(part), `${name} transition duration is zero`).toBe(0);
      }
    }
  } finally {
    await runtime.close();
  }
});

test('DR01 the composition did not move: the same geometry, the same frames', async ({ page }) => {
  test.setTimeout(240000);
  const engine = page.context().browser().browserType().name();
  const engineOutput = join(output, engine);
  const shotOutput = join(output, 'shots', engine);
  await mkdir(engineOutput, { recursive: true });
  await mkdir(shotOutput, { recursive: true });
  const runtime = await startServer(engineOutput);
  const evidence = { engine, frames: [] };
  try {
    await setupWithTracks(page, runtime, 3);
    await page.goto(runtime.origin);
    await settle(page);
    await navigate(page, 'Library');
    await page.locator('.track-row').first().waitFor({ state: 'visible' });

    const measure = () => page.evaluate(() => {
      const style = getComputedStyle(document.documentElement);
      const bar = document.querySelector('.player-bar');
      return {
        bodyFontSize: getComputedStyle(document.body).fontSize,
        sidebarWidth: style.getPropertyValue('--vela-sidebar-width').trim(),
        topbarHeight: style.getPropertyValue('--vela-topbar-height').trim(),
        playerHeight: style.getPropertyValue('--vela-player-height').trim(),
        mobilePlayerHeight: style.getPropertyValue('--vela-mobile-player-height').trim(),
        mobileNavHeight: style.getPropertyValue('--vela-mobile-nav-height').trim(),
        playerBarHeight: bar ? +bar.getBoundingClientRect().height.toFixed(2) : null,
        overflow: document.documentElement.scrollWidth - window.innerWidth,
      };
    });

    const setTheme = async (theme) => {
      const current = await page.locator('html').getAttribute('data-theme');
      if (current !== theme) {
        await page.getByRole('button', { name: 'Change color theme', exact: true }).click();
        await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
        await page.waitForTimeout(400);
      }
      await page.waitForTimeout(200);
    };

    for (const [width, height, name] of [[1284, 856, 'library-1284x856'], [390, 844, 'library-390x844']]) {
      await page.setViewportSize({ width, height });
      await page.waitForTimeout(300);
      for (const theme of ['dark', 'light']) {
        await setTheme(theme);
        const frame = await measure();
        evidence.frames.push({ name, theme, ...frame });
        await page.screenshot({ path: join(shotOutput, `${name}-${theme}.png`), animations: 'disabled' });

        expect(frame.overflow, `${width} ${theme} has no horizontal overflow`).toBeLessThanOrEqual(0);
        expect(frame.bodyFontSize, `${width} ${theme} keeps the 14px body`).toBe('14px');
        if (width > 620) {
          expect(frame.sidebarWidth, `${width} ${theme} keeps the 216px sidebar`).toBe('216px');
          expect(frame.topbarHeight, `${width} ${theme} keeps the 75px topbar`).toBe('75px');
          expect(frame.playerHeight, `${width} ${theme} keeps the 88px player`).toBe('88px');
          expect(Math.abs(frame.playerBarHeight - 88), `${width} ${theme} the player band is 88px`)
            .toBeLessThanOrEqual(1);
        } else {
          expect(frame.mobilePlayerHeight, `${width} ${theme} keeps the 120px mini-player`).toBe('120px');
          expect(frame.mobileNavHeight, `${width} ${theme} keeps the 70px navigation band`).toBe('70px');
          expect(frame.playerHeight, `${width} ${theme} the mobile player reserve is the mini-player`)
            .toBe('120px');
        }
      }
    }

    // Home in the light theme: the DR05 control screen.
    await page.setViewportSize({ width: 1284, height: 856 });
    await page.waitForTimeout(300);
    await navigate(page, 'Home');
    await page.waitForTimeout(500);
    await setTheme('light');
    const home = await measure();
    evidence.frames.push({ name: 'home-1284x856', theme: 'light', ...home });
    await page.screenshot({ path: join(shotOutput, 'home-1284x856-light.png'), animations: 'disabled' });
    expect(home.overflow, 'home has no horizontal overflow').toBeLessThanOrEqual(0);
    expect(home.bodyFontSize, 'home keeps the 14px body').toBe('14px');
    expect(Math.abs(home.playerBarHeight - 88), 'the home player band is 88px').toBeLessThanOrEqual(1);

    await writeFile(join(engineOutput, 'geometry.json'), JSON.stringify(evidence, null, 2));
  } finally {
    await runtime.close();
  }
});
