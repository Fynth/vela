# Original Vela design system

Use the existing sage/forest/cream identity, Vela wordmark and locally generated fixture artwork. Dark background#101716, surfaces#17201e/#1d2825, accent#9ddbc4, text#f0efe8. Light uses ivory#f4f2eb, surface#fffcf5, accent#21775b and text#182c25. Reuse CSS variables for all components; no proprietary assets or new bitmap dependencies.

Use the system sans-serif stack,4/8px spacing rhythm,12–16px readable controls/body,24–40px headings,9–16px radii and restrained borders. New UI uses original inline SVG with consistent24px geometry/stroke; decorative SVG is hidden from assistive technology and buttons carry accessible names. Artwork is square, lazy-loaded where possible, with native vector fallback rather than fabricated covers. Existing technical format labels describe actual reported properties, not assumed losslessness.

Desktop: constrained central canvas with library shortcuts and bottom transport. Mobile: five44px-minimum navigation targets, mini-player above navigation, safe-area padding and a focus-contained More sheet; avoid a compressed ten-item desktop menu. Cards reflow at320px, tablets and desktop/ultrawide without hiding essential actions. Keep the single root audio element mounted through all these states.

Motion is short opacity/transform feedback, disabled under prefers-reduced-motion. Focus rings must remain visible, dialog focus returns to its trigger, errors cannot rely only on color, and the playing state has text/icon evidence. Skeletons reflect a real pending request; empty states offer actual Add music/search/library actions. Do not promise WCAG/screen-reader certification from a screenshot.

Design references studied: [Spotify's library/canvas/playing layout rationale](https://newsroom.spotify.com/2023-06-20/spotify-desktop-experience-redesign-your-library-now-playing-views-customize/) and [Apple's explicit next/queue interactions](https://support.apple.com/en-ae/guide/iphone/ipha4521ef7d/ios). These inform independent information architecture, not copied assets. [Dioxus resource lifecycle](https://dioxuslabs.com/learn/0.7/essentials/basics/resources/) informs cancellation; the current exact-session/generation pattern is retained.

Baseline screenshots are preserved under ignored target/checks/experience-baseline-screens. Final review must include actual fixture music at320px, tablet,desktop and ultrawide, dark/light, navigation/player continuity and keyboard More dismissal. Record artifact paths and findings in PROGRESS; no visual/platform verification is inferred from a compile.
