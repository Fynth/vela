# Vela Music: complete listening product and settings expansion

Planning date: 2026-10-19. Language: English. Repository anchor: `6fa2346`.
Status: implementation specification only. No application code is changed by this package.

Latest supplement: [Interaction Playbook](INTERACTION_PLAYBOOK/README.md), with 96
granular briefs, complete loading/context-menu specifications, 16 interaction recipes,
11 delivery packages and a copyable agent prompt. Read its current-state reconciliation
before treating older gaps below as absent: native authorized artwork and web/native
80–150% appearance Settings now have their own dated implementation evidence.

## Assignment

Turn the existing Vela application into an enjoyable everyday music product, with
deep configuration inside the application, dependable playback, meaningful discovery,
and optional authorized social/network experiences. Application scale is the first
explicit new feature. The goal is preference over competing players in selected user
journeys, not an unsupported claim of universal superiority or access to their catalogs.

The plan contains 320 numbered feature opportunities, a settings registry, concrete
architecture decisions, implementation recipes, sequenced work packages and acceptance
tests. A feature opportunity is not automatically a release commitment. Ship coherent
vertical slices; never expose 320 unfinished controls at once.

## Read in this order

1. [Repository facts, UX gaps and preservation boundary](01_CURRENT_STATE_AND_GAPS.md).
2. [Primary-source research and product decisions](02_RESEARCH_AND_REFERENCES.md).
3. [Settings information architecture and interaction contract](03_SETTINGS_EXPERIENCE.md).
4. [Application scale, HiDPI and accessibility specification](04_APPLICATION_SCALE.md).
5. [Typed settings registry and policy rules](05_SETTINGS_REGISTRY.md).
6. [Complete screen and interaction design](06_MUSIC_FIRST_UX.md).
7. [320 feature implementation briefs](07_FEATURE_CATALOG_320.md).
8. [Data, API, preference and capability architecture](08_ARCHITECTURE_AND_DATA.md).
9. [Implementation recipes for major subsystems](09_IMPLEMENTATION_RECIPES.md).
10. [Ordered delivery packages and ownership](10_DELIVERY_ROADMAP.md).
11. [Test matrix, measurements and release gates](11_ACCEPTANCE_AND_QA.md).
12. [Copyable implementation-agent mandate](12_AGENT_HANDOFF.md).
13. [Everyday scenarios and completion criteria](13_PRODUCT_SCENARIOS.md).
14. [Native desktop/phone master plan and copyable prompt](14_NATIVE_CROSS_PLATFORM_MASTER_PLAN.md).
15. [Interactive product refinement and action-menu handoff](INTERACTION_PLAYBOOK/README.md).

## Authority and relationship to earlier plans

This package supplements the existing [product mandate](../../../../VELA_MUSIC_OMEGA_AUTONOMOUS_ENGINEERING_SPEC.md),
[contracts](../../../CONTRACTS.md), [current-state index](../../../CURRENT_STATE.md),
[Player Reboot](../PLAYER_REBOOT/README.md), and [earlier audit](../README.md).
It does not erase their evidence or silently replace privacy/media authorization.
The older F001–F240 catalog is not a second delivery backlog: map useful entries to
the EX001–EX320 opportunities here when accepting a delivery package. Deduplicate
by user outcome, not by title. Existing fixes remain existing fixes.

Current user instructions take precedence over autonomous implementation mandates:
this assignment is documentation only. The receiving agent must receive a new
implementation instruction before acting on the handoff's executable work packages.

## First implementation outcome

The original artwork/Settings priorities have since received source slices:
`0f9f12b` authorized native artwork, `1618e63` web appearance and `eb36048` native
appearance. Artwork visual QA in `9c1ba4f`/`8c339e3` is limited to its named subset;
it does not verify every art surface/platform. Reconcile ART01–ART14 and SCALE/SET
against current QA_EVIDENCE before accepting remaining work. The standalone native
handoff remains in the final section of file14.

A listener can open Settings in web and native desktop, search for “scale”, adjust
the application size, restore 100%, choose System/Dark/Light appearance, and understand
which preferences belong to this device. Playback, source generation and position
survive the entire flow. No server database migration is needed for device-only
scale. Settings beyond actual backend/platform capabilities stay out of primary UI.

Next interaction outcomes are truthful geometry-matched loading, root-coordinated
right-click/More/keyboard actions, a searchable playlist picker and safe receipts/
Undo. Continue durable native queue restore, remaining artwork/Settings validation,
cache/download controls, true next-source preparation and private discovery under
their existing contracts. Devices, mobile audio and social federation follow their
prerequisites rather than being simulated in the interface.

## Evidence language

- SOURCE_OBSERVED: inspected in the working tree; no execution claim.
- HISTORICALLY_VERIFIED: recorded in an earlier dated executed run, not rerun here.
- PROPOSED: an implementation decision or feature in this plan.
- CONDITIONAL: requires a platform, provider, device, credential or rights gate.
- VERIFIED: reserved for a receiving agent's executed acceptance evidence.

All 320 entries are proposals or extensions, even when their foundations already
exist. No recommendation, performance, native-device or visual result is newly
certified by this documentation pass. Only document integrity and preservation
checks are executed for this deliverable.

## Definition of product quality

Judge task success, recoverability, accessibility, latency, trust and visual clarity.
Do not reward screen count, toggle count, decorative animations, or a feature list
that conceals unsupported paths. Keep the private SQLite modular-monolith install
simple. Optional advanced features must not require a central cloud, GPU or paid AI.

## Documentation delivery validation

PASS for15 new Markdown files,320 unique ordered EX feature rows,120 unique settings
rows,27 defined/referenced EP packages,30 journey rows,14 SCALE,16 SET and14 ART
acceptance rows; relative links and table-column consistency checked. Two scoped
Better Design checks on proposed App size/cache copy PASS. These are document/copy
checks only: no layout, native device, FPS, audio or application feature is certified.

Preservation check:443 tracked non-Markdown files have identical working-tree SHA256
values before and after this task. The initial dirty native changes were independently
committed as7113fd5 while documentation was being prepared; no source bytes changed
between those observations and that commit is not authored by this documentation work.
The initial inspection anchor6fa2346 remains valid with its stated dirty context.
The unrelated prompt.txt was not read/staged/changed. No application tests were rerun.

Every new file is a plan/proposed contract. Do not interpret the counts above as
implemented features or add them to the earlier240 entries as unique functionality.
