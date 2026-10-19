# Approved Vela prototype → existing Rust/Dioxus application

Planning handoff, English, 2026-10-19. Status: DOCUMENTATION ONLY.
Repository inspection anchor: `4f16a3235cd12f2ed689c68e82e48671024c1c50`.
Approved visual reference: [prototyp-opal.vercel.app](https://prototyp-opal.vercel.app/).

The user selected this design. This package specifies a faithful transfer of its
composition and visual language into the existing Vela product. It is not another
design exploration, a new React application, or permission to replace working audio,
data, authorization, settings or native window behavior.

## Give this to DeepSeek harness

Open [START_HERE_DEEPSEEK.md](START_HERE_DEEPSEEK.md). Its opening block is a ready
implementation prompt. Give the agent this entire directory and the current repository;
do not send only a palette or a screenshot. The instructions are plain Markdown and
do not require a particular model API, proprietary harness feature or skill installer.
If the agent has limited context, use [the checkpoint protocol](07_HARNESS_PROTOCOL.md).

The current task only produces the plan. A receiving agent needs an implementation
assignment before executing it. This file itself does not turn documentation work
into authorization to modify the application or publish anything externally.

## Reading order

1. [Actual reference audit, discrepancies and evidence limits](01_REFERENCE_AUDIT.md).
2. [Complete visual specification and responsive compositions](02_DESIGN_SPEC.md).
3. [Interaction, player, navigation and asynchronous-state contracts](03_INTERACTION_CONTRACTS.md).
4. [Repository mapping and safe Dioxus migration](04_DIOXUS_MIGRATION.md).
5. [Delivery packages, test matrix and acceptance](05_ROADMAP_AND_QA.md).
6. [Original image manifest, fonts, icons and Russian copy](06_ASSETS_AND_COPY.md).
7. [Execution packets, checkpoints and reporting for a small-context agent](07_HARNESS_PROTOCOL.md).

## What was actually inspected

- Actual Git state, AGENTS, progress/contracts/current-state/architecture/design docs,
  current workspace pins, web/native entry points, DTOs, assets and test infrastructure.
- Public prototype HTML, its linked stylesheet and eight linked JavaScript chunks,
  read as data, never executed in a local application.
- The three original PNG files, downloaded for inspection and opened as images.
- Better Design URL extraction, focused UI/UX guidance, review rules and scoped copy
  checks. Extraction output was compared with the actual dark CSS instead of blindly
  importing its incorrectly merged button/background roles.
- Numerical contrast calculations on eight opaque color pairs. These are calculations,
  not a rendered accessibility audit or evidence about text over arbitrary artwork.

Interactive browser discovery returned no connected browser. The normal web reader
also could not open this site; direct HTTPS inspection returned 200. Consequently no
new page screenshot, responsive render, click test, focus test or audible playback
test is claimed. Server-rendered markup and published handlers are source evidence,
not proof that every prototype interaction works. See the audit for precise scope.

## Authority when sources disagree

1. Active user instructions and repository data/security/audio invariants.
2. This user's approved composition, palette, compact type and Russian reference copy.
3. Verified source facts and the explicit conflict decisions in this package.
4. Older broad plans, only where compatible with this selected design.

The new visual direction supersedes earlier mint/Inter styling proposals for these
screens. The [Interaction Playbook](../PRODUCT_EXPANSION/INTERACTION_PLAYBOOK/README.md)
still defines useful skeleton/menu/Undo behavior; it does not authorize substituting
a different layout. The [native master plan](../PRODUCT_EXPANSION/14_NATIVE_CROSS_PLATFORM_MASTER_PLAN.md)
retains platform, cache, offline and audio-engine boundaries.

## Accepted design trade-off

The chosen compact desktop design shows more music with quieter chrome and large
artwork. Small captions are less readable for some listeners. Keep the selected
desktop scale, but preserve existing 80–150% app scaling, browser/text enlargement,
light/System appearance, keyboard access and roomy touch targets. Accessibility
corrections may change a local token or hit region; they are not permission to replace
the approved composition with oversized dashboard cards.

The prototype shows six concept covers and three editorial stories. Production shows
real authorized content, fewer items when necessary, useful honest empty states and
no demo badge. Visual fidelity does not mean fabricating a user's music collection.

## Delivery status

All TR packages and PT acceptance scenarios in this package are PROPOSED, not shipped
or newly tested. No application code, CSS, dependency, schema, media, configuration or
running service is modified by the planning task. Reference image URLs and SHA256
values are portable; the temporary inspection download is not a release asset bundle.
Final document/preservation validation is recorded in 01_REFERENCE_AUDIT.
