# Settings experience: one coherent, searchable system

All behavior in this file is proposed. Preserve the existing audio/root/session
contracts and original Vela design. EP01–EP04 in the roadmap deliver the foundation.

## Entry points and navigation

Add a real Settings destination to native desktop and retain the web destination.
Desktop: profile menu, sidebar lower area and Ctrl/Cmd+, where host shortcuts permit.
Mobile: Library/profile menu; do not consume a permanent bottom tab just for settings.
Provide Appearance and accessibility before sign-in, without contacting a server.

Use these listener categories in this order:

1. Appearance: theme, application scale, text, density, artwork, panels.
2. Playback: queue behavior, resume, sound, transitions, sleep.
3. Downloads and storage: offline copies, cache, transfers, actual usage.
4. Library and discovery: organization, recommendations, exclusions, metadata.
5. Devices and connections: server connections, outputs, presence and handoff.
6. Privacy and sharing: history consent, visibility, invitations, blocks, exports.
7. Notifications: in-app/system destinations and quiet hours.
8. Accessibility and shortcuts: keyboard map, motion, announcements, text direction.
9. Account and security: current server identity, password/session controls, optional identity link.
10. About and help: version/build, capabilities, diagnostics, recovery instructions.

Administration is a separate administrator-only destination: Users, Libraries,
Imports, Background work, Storage, Network policy, Backups, Updates and Diagnostics.
Its search results must not appear for ordinary users. Client Settings must not show
a raw diagnostics dashboard as its first screen.

Desktop layouts: existing primary sidebar, settings category rail inside the content
canvas, readable setting rows in the main column, persistent bottom player. At narrow
widths or high scale, collapse the inner rail to a category list/back stack. Never
leave two mandatory fixed sidebars that squeeze the settings form off-screen.

## Setting row composition

Each row has a short label, current effective value/control, optional one-sentence
help and, when relevant, a scope or policy explanation. Use separators and typography,
not a separate rounded card for each row. Related subcontrols expand directly under
their parent. Reserve cards for previews, connected devices and actionable summaries.

Examples of actual proposed copy:

| Location | Label | Help/control |
| --- | --- | --- |
| Appearance | App size | “Changes this device only.” Minus, percentage, plus, Reset |
| Appearance | Theme | System / Dark / Light with one live preview |
| Downloads | Playback cache | Actual used bytes, limit, “Clear cache”; saved downloads shown separately |
| Playback | Continue after this queue | “Adds available tracks based on this listening session.” Off by default until implemented |
| Privacy | Listening history | Existing account consent and this-device consent remain separate |
| Devices | Transfer playback | Destination, capability check, confirmation/acknowledgement; never forced takeover |

Never show an active switch that merely stores a value for an absent engine feature.
For a useful unavailable capability, use a concise reason and a real resolution
action; otherwise omit it. An unsupported equalizer is not a checkbox waiting for code.

## Search contract

Search labels, short explanations, category names and curated English/Russian aliases.
Examples: scale/zoom/UI size/масштаб; normalize/loudness/ReplayGain; cache/storage/
download; pause/headphones; private/history/scrobble. Match local typed metadata,
not hidden admin values, credential values, user file paths or private diagnostics.

Rank exact label, prefix, category then alias; return at most20 hits. Input changes
cancel any optional remote capability request and increment a generation. Most
settings search should require no network. Enter opens/highlights the row, Escape
clears search before leaving Settings, arrows move within suggestions. Preserve
category/scroll/focus on Back. Use stable setting keys for internal deep links.
Do not persist settings search history by default.

## Applying changes

Classify every setting before exposing it:

| Mode | Examples | Interaction and error behavior |
| --- | --- | --- |
| Immediate local | Theme, scale, density, panel visibility | Apply live without remount; persist atomically; on persistence failure keep session value and explicitly say it is temporary |
| Immediate account | A safe recommendation preference | Optimistic only when reversible; one owner/revision-bound write, rollback on definite failure; reconcile uncertain timeout before retry |
| Explicit Apply | Several dependent download/network preferences | Draft summary, Validate, Apply; no partial visual success if the transaction fails |
| Next track | Quality preset, replay-gain mode | Show “Applies to the next track”; preserve current source unless explicit restart is chosen |
| Output reconfigure | Device/sample/output mode | Prepare and acknowledge safe switch; rollback on failure; no silent stop or volume jump |
| Restart required | Renderer backend or startup-only server policy | Show active versus pending value and an explicit restart action; do not kill playback automatically |
| Destructive/sensitive action | Erase history, remove downloads, revoke sessions | Preview exact scope, explicit confirmation, independently authorized mutation; no generic preference toggle |

A saved indicator must distinguish Applying, Saved, Saved for next track,
Restart required, Not saved and Temporarily applied. Never display Saved before an
acknowledged server receipt. Coalesce repeated slider movement but persist the final
value. Stale writes cannot overwrite a newer account/session/device generation.

For navigation with drafts, offer Apply / Discard / Stay. Do not show this prompt for
already saved immediate settings. An initial release can avoid multisection drafts;
do not invent a giant Save Everything transaction spanning local files and servers.

## Scope, precedence and policy

Display “This device”, “Your account on [server]”, or “Server policy” where confusion
is likely. Device presentation settings are available while offline/signed out.
Account preferences are scoped by canonical origin plus local user ID, not username.
Do not sync a large desktop scale to a phone by default.

An effective value may be forced by administrator policy, limited by hardware, or
pending restart. Show the chosen value, effective result and a concise reason when
they differ. A locked value must explain who controls it. Examples: “Downloads are
disabled by this server”, “This browser cannot select an audio output”, “This output
does not support exclusive mode”. Hide private operator paths/credentials from reasons.

## Profiles without configuration overload

Start with sensible defaults. Later introduce explicit profiles such as Balanced,
Low data, Album listening and Large text. A profile is a previewed set of existing
settings, not a hidden mode with its own state machine. Show changed fields before
applying; preserve manually excluded fields; offer Restore previous values.

Do not include history consent, public visibility, identity linking or credential
changes in one-click profiles. Never opt a user into collection while installing,
importing a preset or upgrading. Avoid automatic location-based profiles.

## Reset, export and import

- Reset a row or category with a preview. Reset Appearance must not delete downloads.
- “Reset all preferences” excludes accounts, media, history and server policy; list
  exclusions before confirmation. Default privacy remains private.
- Export versioned typed preferences with platform compatibility metadata. Exclude
  passwords, tokens, cookies, share links, absolute private paths and listening data.
- Import into a draft; reject oversized/unknown-dangerous values; show accepted,
  unsupported, policy-locked and skipped-sensitive fields. Do not silently activate
  privacy/network sharing, external connections or output volume.
- Invalid/corrupt preferences fail to safe defaults with recovery UI, not an
  unstartable app. Preserve the bad file privately for diagnostics when appropriate.

## Accessibility, visual hierarchy and motion

Keep original bright mint tokens, system fonts and SVG family. Theme previews are
small truthful previews of real components, not unrelated art boards. Use readable
rows, restrained separators, meaningful color and a single primary Apply action
only for the current draft. Keep actions reachable at high scale and320px-equivalent
content widths. Visible focus and screen-reader names must survive every setting.

Do not animate font/zoom changes through hundreds of layout frames. Apply size
changes promptly and retain the focused row/scroll anchor. Respect reduced motion
and system text settings. Do not bind global Space to Play while operating a switch,
range input, text field, dialog or screen-reader interaction.

## Done means

Every exposed row has a registry key or independently authorized action contract,
real persistence, effective-state projection, reset behavior, capability condition,
tests and a named implementation owner. Keyboard-only users can find scale, change
it and recover. The same audio occurrence/time survives theme, scale and navigation.
No general settings screen or admin form is considered complete from copy alone.
