# Typed settings registry and policy catalog

The 120 entries below are proposed contracts, not120 implemented switches. Expose
an entry only when its implementation package and capability tests are complete.
For every key the implementation records label/help translation keys, value type,
default, validator, scope, category, apply mode, capability predicate, redaction/export
policy, reset behavior, revision and test IDs. Avoid an unbounded key/value API.

## Scope and apply legend

- D: device/installation presentation or hardware preference; never auto-sync to another device.
- A: account on a canonical server origin; server-authoritative, owner-only.
- S: server administrator policy, never an ordinary user write.
- Immediate: live reversible update; Next: next source/track; Apply: validated draft;
  Output: acknowledged output switch; Restart: show active/pending states.
- Defaults below are Vela decisions. Existing stored values migrate without being
  overwritten. System means a documented OS/browser signal, not heuristic inference.
- Privacy consent remains in its existing dedicated revisioned service. Registry
  rows describe the UI contract, not permission to bypass consent mutations.

## Appearance — EP01–EP04

| ID | Key | Scope; proposed default/type | Validation and apply/capability |
| --- | --- | --- | --- |
| S001 | appearance.theme | D; System enum | System/Dark/Light; Immediate; migrate valid vela.theme first |
| S002 | appearance.app_scale_percent | D;100 integer |75..200 step5; Immediate; platform adapter and SCALE01–14 |
| S003 | appearance.text_percent | D;100 integer |100..200 step5; Immediate; preserve system text enlargement |
| S004 | appearance.density | D; Comfortable enum | Comfortable/Compact/Touch; Immediate; target-size floors override compactness |
| S005 | appearance.motion | D; System enum | System/Reduced/Full; reduced OS setting constrains decorative motion |
| S006 | appearance.artwork_accent | D; Subtle enum | Off/Subtle/Vivid; Immediate; contrast-tested foregrounds never inherit arbitrary cover color |
| S007 | appearance.cover_size | D; Medium enum | Small/Medium/Large semantic sizes; reflow, not fixed columns |
| S008 | appearance.library_panel_width | D;248 integer |180..400 host logical units; clamp to available window width |
| S009 | appearance.right_panel | D; Hidden enum | Hidden/Now playing/Queue; available space and implemented views gate choices |
| S010 | appearance.queue_order_view | D; Saved enum | Saved/Actual next; Actual next requires side-effect-free traversal preview |
| S011 | appearance.language | D; System locale | Only shipped complete/fallback locales; preserve text direction and plural forms |
| S012 | appearance.time_format | D; System enum | System/12h/24h; user-facing dates only, no wire timestamp changes |

## Accessibility and input — EP03/EP24

| ID | Key | Scope; proposed default/type | Validation and apply/capability |
| --- | --- | --- | --- |
| S013 | accessibility.high_contrast | D; System enum | System/On/Off; honor forced-colors constraints |
| S014 | accessibility.target_size | D; Standard enum | Standard44/Large52 logical units; never lower platform minimum |
| S015 | accessibility.announcements | D; Essential enum | Essential/Detailed; retain errors and primary state changes |
| S016 | accessibility.marquee | D; Off boolean | Long titles wrap/disclose when off; reduced motion always disables movement |
| S017 | accessibility.lyric_motion | D; Reduced enum | Reduced/Follow; no forced scrolling while reading selected lines |
| S018 | accessibility.waveform | D; Off enum | Off/Static/Interactive; measured waveform required; text seek fallback |
| S019 | input.shortcuts_enabled | D; True boolean | Disables app shortcuts only, never host/browser accessibility shortcuts |
| S020 | input.shortcut_map | D; Default bounded map | Max64 allowlisted commands; reserved-key/conflict validation; export safe |
| S021 | input.seek_seconds | D;10 integer |1..60; active finite-duration sources only |
| S022 | input.previous_restart_seconds | D;3 integer |0..10; shared playback command semantics, tested paused behavior |
| S023 | input.row_activation | D; Select enum | Select/Play; explicit play control always toggles current occurrence |
| S024 | input.double_click_play | D; True boolean | Pointer desktop only; never required for keyboard/touch playback |

## Transport and queue — EP05/EP07

| ID | Key | Scope; proposed default/type | Validation and apply/capability |
| --- | --- | --- | --- |
| S025 | playback.restore_session | D; Paused enum | Off/Paused; no auto-start after cold launch |
| S026 | playback.autoplay_after_context | A; False boolean | Only accessible candidates; Next; separate current-source state |
| S027 | playback.default_repeat | D; Off enum | Off/All/One; source-specific effective mode remains authoritative |
| S028 | playback.default_shuffle | D; False boolean | Fair occurrence model; unavailable adapters show effective false |
| S029 | playback.unavailable_action | D; Ask enum | Ask/Skip/Stop; bounded skip count; no infinite queue loop |
| S030 | playback.remove_played | D; False boolean | New queue-journal semantics required; Previous history retained separately |
| S031 | playback.confirm_replace_queue | D; True boolean | Preview replacement; explicit recoverable undo snapshot |
| S032 | playback.resume_position | D; True boolean | Known durations clamp; music/podcast semantics separate |
| S033 | playback.sleep_action | D; Pause enum | Pause/End current/End album; actual eligible context required |
| S034 | playback.pause_on_disconnect | D; True boolean | Native route-change capability; no browser guarantee without evidence |
| S035 | playback.resume_after_interruption | D; Ask enum | Never/Ask/When safe; no surprise output at full volume |
| S036 | playback.speed | D;1.0 number |0.5..2.0; category/engine support; default music1.0; Next |

## Audio and quality — EP08/EP20–EP23

| ID | Key | Scope; proposed default/type | Validation and apply/capability |
| --- | --- | --- | --- |
| S037 | audio.output_device_ref | D; System opaque ref | Platform device registry; Output; missing device falls back with notice |
| S038 | audio.start_volume | D; Restore capped enum | Keep current existing volume unless chosen; no reset-induced loudness jump |
| S039 | audio.streaming_quality | D; Original enum | Original/Compatible/Low data; negotiated provider/server policy; Next |
| S040 | audio.download_quality | D; Original enum | Changing affects future downloads; existing encodes labeled accurately |
| S041 | audio.normalization | D; Off enum | Off/Track/Album; verified ReplayGain/R128 implementation only; Next |
| S042 | audio.preamp_db | D;0 finite number |−12..+6dB; limiter/headroom policy; disabled until DSP exists |
| S043 | audio.limiter | D; On when DSP boolean | Mandatory clipping guard for positive-gain presets; does not prove hearing safety |
| S044 | audio.gapless | D; When supported enum | Actual engine/codec validated capability; no universal promise |
| S045 | audio.crossfade_seconds | D;0 finite number |0..12; supported mixer only; album continuity and gapless precedence explicit |
| S046 | audio.eq_profile_ref | D; None opaque ref | Bounded validated bands/gain; bypass available; no arbitrary executable filter |
| S047 | audio.exclusive_output | D; False boolean | Hardware/OS capability; Output; mutually exclusive DSP/bit-perfect claims evaluated |
| S048 | audio.channel_mode | D; Original enum | Original/Mono; implemented engine conversion only, no lossy format relabeling |

## Cache, downloads and transfers — EP06/EP07/EP09

| ID | Key | Scope; proposed default/type | Validation and apply/capability |
| --- | --- | --- | --- |
| S049 | storage.cache_budget_mib | D;2048 integer |0..32768 bounded by actual free space;0 disables new cache, not active leases |
| S050 | storage.cache_entry_limit | D;200 integer |0..5000 plus byte ceiling; effective limit shown; current file protected |
| S051 | storage.download_budget_mib | D;2048 integer |128..262144 and available space; lowering never silently deletes pinned copies |
| S052 | storage.free_reserve_mib | D;512 integer |128..8192; low-space stop before publication; disk failures still handled |
| S053 | transfers.prefetch | D; Off enum | Off/Next track; requires safe actual-next worker; cache alone is not prefetch |
| S054 | transfers.prefetch_budget_mib | D;128 integer |8..512 within cache/global limits; no extra entitlement |
| S055 | transfers.download_concurrency | D;2 integer |1..4 subject to server cap; bounded queue/cancellation |
| S056 | transfers.network_policy | D; Unmetered enum | Unmetered/Any/Ask; unknown network conservatively disables speculation |
| S057 | transfers.roaming_allowed | D; False boolean | Only when reliable OS classification exists; otherwise Ask |
| S058 | downloads.keep_playlist_updated | A; False boolean | Per-playlist override; diff revisions and preview removals; do not auto-delete saved manual tracks |
| S059 | downloads.background | D; When supported enum | Platform permission/power gate; no promise for a suspended web tab |
| S060 | downloads.integrity_policy | D; Verify enum | Safe verification cannot be turned off; exposes scheduling/status, not a bypass |

## Library and search — EP10–EP13

| ID | Key | Scope; proposed default/type | Validation and apply/capability |
| --- | --- | --- | --- |
| S061 | library.default_view | D; Last used enum | List/Grid/Last used; pages render real data with bounded loading |
| S062 | library.sort | A; Newest enum | Only indexed validated orders; context-specific persistence |
| S063 | library.visible_columns | D; Default bounded list | Known keys, title/action always accessible; narrow layout adapts |
| S064 | library.pinned_refs | A; Empty bounded list | Max100 authorized stable refs, no secret share tokens |
| S065 | library.compilation_grouping | A; Album artist enum | Choice affects presentation; no destructive re-identification |
| S066 | library.preferred_asset | A; Best compatible enum | Lossless/Smallest/Best compatible; use actual encoding properties |
| S067 | library.missing_track_visibility | A; Show enum | Show/Hide; hidden metadata is not deleted from playlists |
| S068 | metadata.provider_lookup | A; Off boolean | Explicit external egress consent; provider identity/rate/policy |
| S069 | metadata.review_before_apply | A; True boolean | User edits retain precedence; background guesses never silently overwrite |
| S070 | search.default_scope | D; This server enum | This server/Connected/Downloaded; implemented adapters only |
| S071 | search.history_enabled | A; False boolean | Bounded user-clearable retention; separate from listening consent |
| S072 | search.normalization | A; Accent-aware enum | Original/Accent-aware; indexed normalization preserves display names |

## Discovery and private listening — EP14/EP15

| ID | Key | Scope; proposed default/type | Validation and apply/capability |
| --- | --- | --- | --- |
| S073 | discovery.enabled | A; True boolean | Content-only local discovery can work without history; disclose signals |
| S074 | discovery.history_influence | A; Only with consent enum | Current account AND device collection contracts; no retained-disabled-history reads |
| S075 | discovery.novelty | A; Balanced enum | Familiar/Balanced/Explore; ranking parameter, not a fake global catalog |
| S076 | discovery.artist_spacing | A;4 integer |1..20, sparse library fallback explained |
| S077 | discovery.album_spacing | A;3 integer |1..20; album-mode continuity exemption explicit |
| S078 | discovery.repeat_cooldown_days | A;7 integer |0..180; history permission gate; exclusions separate |
| S079 | discovery.source_scope | A; Owned enum | Owned/Explicit shared/Connected public; media rights checked after ranking |
| S080 | discovery.explicit_content | A; Allow enum | Allow/Exclude tagged; missing tags are Unknown, not a guarantee |
| S081 | discovery.analysis_enrollment | A; Off enum | Off/Selected/New eligible; explicit consent, quotas/versioned jobs |
| S082 | privacy.history_account | A; False boolean | Existing revisioned consent API only; upgrade/import never opts in |
| S083 | privacy.history_this_device | A device grant; False | Existing device+privacy revision; revocation invalidates late outboxes |
| S084 | privacy.history_retention_days | A; No new time pruning; optional integer | None/30/90/180/365 days; keep existing record cap; preview and reviewed prune transaction, never erase old data by upgrade |

## Devices, connections and synchronization — EP16–EP19

| ID | Key | Scope; proposed default/type | Validation and apply/capability |
| --- | --- | --- | --- |
| S085 | device.display_name | D; Local device name |1..80 control-free chars; publish only with presence consent |
| S086 | device.presence | A device grant; False | Expiring heartbeat; explicitly scoped observers; history consent does not imply presence |
| S087 | device.remote_control | A device grant; False | Owner-only commands, sequence/idempotency, revocation |
| S088 | device.handoff_default | D; Ask enum | Ask only initially; no automatic takeover on sign-in |
| S089 | sync.queue_bookmarks | A; False boolean | Explicit opt-in, occurrence IDs/revisions; preserve independent live players |
| S090 | sync.library_preferences | A; True boolean | Existing account preferences only; no device scale or output refs |
| S091 | connection.default_ref | D; Last active opaque ref | Canonical saved connection; credentials in secure store only |
| S092 | connection.allow_offline_start | D; True boolean | Reviewed local account unlock/session path; cached bytes are not new grants |
| S093 | connection.retry_profile | D; Balanced enum | Bounded reviewed profiles; no disable-TLS or unlimited retry option |
| S094 | connection.background_refresh | D; Visible only enum | Visible only/Allowed native; deadlines/power/network caps |
| S095 | casting.discovery | D; False boolean | Explicit LAN permission; provider support; separate from URL-import egress |
| S096 | casting.default_target_ref | D; None opaque ref | Select actual capable output; never auto-transfer on launch |

## Notifications — EP17/EP24

| ID | Key | Scope; proposed default/type | Validation and apply/capability |
| --- | --- | --- | --- |
| S097 | notifications.in_app | A; Important enum | Important/All; errors/action receipts remain reachable |
| S098 | notifications.system | D; False boolean | OS permission requested only after explicit action |
| S099 | notifications.imports | A; Failures enum | Off/Failures/All; durable real job events, no invented completion |
| S100 | notifications.invites | A; True boolean | Only authorized invitations; block/rate policies applied |
| S101 | notifications.sound | D; False boolean | No sound without explicit user choice; visible equivalents always |
| S102 | notifications.quiet_hours | D; Disabled bounded schedule | Local timezone/DST semantics; critical errors still in inbox |
| S103 | notifications.lock_screen_details | D; Hidden enum | Hidden/Track title; OS constraints disclosed |
| S104 | notifications.digest | A; Off enum | Off/Daily/Weekly; opt-in actual events, no engagement spam |

## Account, social and identity — EP17–EP19

| ID | Key | Scope; proposed default/type | Validation and apply/capability |
| --- | --- | --- | --- |
| S105 | profile.display_name | A; Current name string |1..80 control-free; untrusted remote rendering escaped |
| S106 | profile.discoverability | A; Private enum | Private/Named/Public; explicit confirmation and server policy |
| S107 | profile.listening_visibility | A; Private enum | Private/Approved followers; separate opt-in, never inferred from collection |
| S108 | sharing.default_playlist_visibility | A; Private enum | Existing/new permission model; no retroactive publication |
| S109 | sharing.follow_requests | A; Approval enum | Approval/Public eligible; block/mute enforce on every path |
| S110 | rooms.allow_invites | A; Known only enum | None/Known/Anyone permitted; rate limits, no unsolicited audio |
| S111 | scrobble.enabled | A integration; False | Explicit provider consent/token reference; outbox and deletion caveat |
| S112 | identity.sync_presentation | A; False boolean | Only selected export-safe fields; app scale excluded by default |

## Administrator policies — EP04/EP25

| ID | Key | Scope; proposed default/type | Validation and apply/capability |
| --- | --- | --- | --- |
| S113 | server.default_upload_limit_bytes | S; Existing effective value | Positive bounded integer; cannot exceed operator hard ceiling; Apply |
| S114 | server.owner_storage_budget_bytes | S; Existing effective value | Budget cannot force deletion; over-budget blocks new writes; Apply |
| S115 | server.import_concurrency | S;1 integer |1..4 and profile ceiling; changes affect new leases |
| S116 | server.analysis_concurrency | S;1 integer | Preserve supervised decoder/memory budget; do not spawn unbounded workers |
| S117 | server.download_policy | S; Existing rights policy enum | Enforces new downloads per permission, never equates stream denial with byte recall |
| S118 | server.public_discovery | S; False boolean | Dedicated network-settings CAS; does not publish libraries/users |
| S119 | server.backup_schedule | S; Disabled bounded schedule | Valid destination capability, quota/encryption/restore checks; no arbitrary shell |
| S120 | server.update_channel | S; Stable enum | Check-only by default; explicit authenticated install/restart and backup gate |

## Actions are not ordinary preferences

These need separate independently authorized commands, previews and receipts:
Clear playback cache; remove selected downloads; verify/repair downloads; export
preferences; import preferences; reset category; reset all presentation; erase
history-and-disable; revoke a device; revoke a session; change password; link/unlink
an identity provider; connect/disconnect a server; transfer playback; leave/end a
room; export personal data; create/restore backup; apply server restart; rotate
credentials; add a trusted storage capability; revoke a publication.

Never encode a destructive action as a boolean in a generic PATCH. Cache clearing
excludes pinned downloads and active decoder leases; removal of downloads does not
remove original server music. A failed/uncertain receipt must be reconciled before
displaying success. Most destructive actions cannot promise reliable Undo.

## Operator-controlled bootstrap configuration

Bind address, container volume mapping, initial data directory, OS permissions,
TLS trust policy, allowed scan roots and hard resource/security ceilings remain
operator capabilities. Show safe effective status and validated in-app workflows
where possible. Do not pretend a web UI can grant filesystem mounts, change a
reverse proxy, bypass SSRF, disable auth or install a privileged media service.

After bootstrap, routine music organization/import/download/privacy/device tasks
should be possible in-app. For startup-only changes expose pending state, a scoped
permission explanation and guided handover; never write shell configuration from
untrusted fields. See architecture and QA for precedence, secrets and migration.
