# ADR0023 — Contrast and listening layout

Date: 2026-10-19. Accepted for current-source web and Linux desktop UI.
Source inspection baseline: `0170ec5`; workspace version remains0.12.6.

## Problem

Vela's subdued sage palette, very small secondary text and uneven layout made daily
listening unnecessarily difficult. Native track actions occupied more space than
their grid column, long-lived notices displaced the page, and compact controls had
35–40px targets. The web Library began with a large decorative banner rather than
the actual collection. Enlarging navigation without reviewing constrained heights
also exposed nested sidebar clipping.

## Decision

Keep the existing original Vela identity and Dioxus architecture. Use bright mint
actions against near-black surfaces, with a separate readable light palette. Reuse
the existing theme toggle/persistence; introduce no second design-system runtime,
framework, external font, proprietary artwork or bitmap dependency.

Use semantic surface/text/control tokens, readable14–15px body/track labels,
12–13px secondary labels, visible control borders and44px main interaction targets.
Theme colors switch atomically; short transform/opacity feedback remains and
existing reduced-motion rules still apply. Disabled controls have explicit colors.

Native uses one shell grid with a sidebar column, scrolling content and the existing
player as its bottom row. The sidebar width is counted once. Track grids explicitly
reserve176px for four44px actions. Playlist creation is a native details/summary
disclosure; successful notices are dismissible overlays. Web navigation scrolls as
one region rather than letting its bottom group clip the library. Web Library puts
its existing Play action next to its title and shows real tracks immediately.

## Boundaries and tradeoffs

- Root browser/native playback owners, authenticated sources, cancellation guards,
  queue semantics, private sessions and server authorization remain authoritative.
- Larger mobile targets make track rows taller. Long sidebars scroll vertically;
  they do not shrink text or discard destinations to fit short windows.
- Standard desktop page inset is24px; phone inset is16px. Some inherited component
  measurements remain outside the generic Better Design spacing scale; record them
  as moderate findings rather than claiming a perfect spacing score.
- Native cover placeholders remain. Authenticated native artwork needs a separate
  bounded fetch/cache/ownership contract; CSS must not invent album covers.
- This is a UI increment, not progressive streaming, prefetch, device handoff,
  native mobile delivery or Spotify feature parity.

## Verification

Use [the dated review](../DESIGN_REVIEW_2026-10-19.md) for exact commands, evidence,
failure corrections and platform limits. DESIGN01 runs a private isolated real
server, uploads legitimate fixture audio, verifies actual playback through navigation,
measures selected rendered text/control contrast, checks touch targets/overflow and
captures both themes. The native QA driver exercises the actual WebKitGTK/Rodio
app against isolated servers and an owned null audio sink. Screenshot dimensions
must match the reported viewport; an off-screen cropped window is invalid evidence.
