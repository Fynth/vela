//! TR04b: the Home editorial feature and the My Wave module.
//! Both bind to real owner data. Neither owns audio or a second request source.
use crate::client::{RequestScope, get};
use crate::experience::Actions;
use crate::icons::Icon;
use crate::session::{ContextKind, PlaybackContext};
use crate::{spawn_authorized, token};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use vela_domain::*;

/// At most three owned playlists carry an editorial story.
const EDITORIAL_TARGETS: usize = 3;
/// One bounded page of real recommendation candidates.
const WAVE_LIMIT: usize = 12;
/// Supporting copy only: it never presents itself as a saved collection.
const EDITORIAL_SUBTITLE: &str = "Music to slow down to. And to hear something more.";
const EDITORIAL_EYEBROW: &str = "EDITORIAL";
const WAVE_EYEBROW: &str = "PERSONAL MIX";
const WAVE_TITLE: &str = "Your wave";
const WAVE_SUBTITLE: &str = "Your music. New combinations.";

/// The wave arrow opens the real personal mixes section rendered on this page.
/// It is a scroll, not a second reader: the mixes keep their own request scope.
fn scroll_to_personal_mixes() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    // `personal_mixes` owns this class; the section is always mounted on Home.
    let Ok(Some(section)) = document.query_selector(".personal-mixes") else {
        return;
    };
    section.scroll_into_view();
}

/// The Home editorial/wave pair. The editorial card and the wave own separate
/// request scopes: refreshing the wave never aborts an in-flight editorial
/// read, and neither card may start playback after it left the page.
#[component]
pub fn HomeFeatures(
    auth: Signal<Option<AuthResponse>>,
    revision: Signal<u64>,
    playlists: Vec<PlaylistSummary>,
    owner_id: String,
    actions: Actions,
) -> Element {
    rsx! {
        section { class: "home-features", "aria-label": "Editorial and personal mix",
            EditorialCard { auth, playlists, owner_id, actions }
            WaveCard { auth, revision, actions }
        }
    }
}

/// The editorial feature carries real owned playlists. Its title is the real
/// playlist name; the subtitle is supporting copy and never a saved collection.
#[component]
fn EditorialCard(
    auth: Signal<Option<AuthResponse>>,
    playlists: Vec<PlaylistSummary>,
    owner_id: String,
    actions: Actions,
) -> Element {
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let dropped = alive.clone();
    let reads = use_hook(RequestScope::default);
    let cleanup = reads.clone();
    use_drop(move || {
        dropped.set(false);
        cleanup.cancel();
    });
    let mut generation = use_signal(|| 0_u64);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(String::new);
    let mut selected = use_signal(|| 0_usize);

    // Only playlists this account owns can carry an editorial story: shared
    // metadata never grants a listening target.
    let targets: Vec<PlaylistSummary> = playlists
        .into_iter()
        .filter(|list| list.owner_id == owner_id)
        .take(EDITORIAL_TARGETS)
        .collect();
    let index = (*selected.read()).min(targets.len().saturating_sub(1));
    let Some(current) = targets.get(index).cloned() else {
        // No owned playlist means no story. The card states that honestly and
        // offers the real playlist path instead of a dead Listen control.
        return rsx! {
            article { class: "home-feature home-feature-empty",
                div { class: "home-feature-copy",
                    span { class: "eyebrow home-eyebrow", "{EDITORIAL_EYEBROW}" }
                    h2 { class: "home-feature-title", "Choose music for this space" }
                    p { class: "home-feature-subtitle",
                        "Create a private playlist to choose what appears here." }
                    div { class: "home-feature-actions",
                        button { class: "button primary home-feature-open", r#type: "button",
                            "aria-label": "Open playlists",
                            onclick: move |_| actions.navigate.call("playlists".into()), "Open playlists" }
                    }
                }
            }
        };
    };

    let title = current.name.clone();
    let listen_title = title.clone();
    let listen_id = current.id.clone();
    let open_id = current.id.clone();
    let pending = *busy.read();
    let listen_label = if pending { "Loading…" } else { "Play" };
    let error_text = error.read().clone();

    rsx! {
        article { class: "home-feature",
            div { class: "home-feature-copy",
                span { class: "eyebrow home-eyebrow", "{EDITORIAL_EYEBROW}" }
                h2 { class: "home-feature-title", "{title}" }
                p { class: "home-feature-subtitle", "{EDITORIAL_SUBTITLE}" }
                div { class: "home-feature-actions",
                    button { class: "button primary home-feature-listen", r#type: "button",
                        disabled: pending,
                        "aria-label": "Play «{title}»",
                        onclick: move |_| {
                            let session = token(auth);
                            if session.is_empty() {
                                return;
                            }
                            let request = generation.peek().wrapping_add(1);
                            generation.set(request);
                            error.set(String::new());
                            busy.set(true);
                            let signal = match reads.begin() {
                                Ok(signal) => signal,
                                Err(message) => {
                                    error.set(message);
                                    busy.set(false);
                                    return;
                                }
                            };
                            let alive = alive.clone();
                            let playlist_id = listen_id.clone();
                            let playlist_name = listen_title.clone();
                            spawn_authorized(auth, async move {
                                let result = get::<PlaylistDetail>(
                                    &format!("/api/playlists/{playlist_id}"),
                                    &signal,
                                )
                                .await;
                                // A late answer must never start playback: the
                                // card may be gone, the account may have changed
                                // or a newer request may already own this read.
                                if !alive.get()
                                    || token(auth) != session
                                    || *generation.peek() != request
                                {
                                    return;
                                }
                                busy.set(false);
                                match result {
                                    Ok(detail) => {
                                        let tracks: Vec<Track> = detail
                                            .items
                                            .into_iter()
                                            .filter(|entry| entry.can_stream)
                                            .map(|entry| entry.track)
                                            .collect();
                                        if tracks.is_empty() {
                                            error.set(
                                                "This playlist has no tracks you can play yet."
                                                    .into(),
                                            );
                                            return;
                                        }
                                        actions.start.call((
                                            tracks,
                                            0,
                                            PlaybackContext::new(
                                                ContextKind::Playlist,
                                                &playlist_name,
                                            ),
                                        ));
                                    }
                                    Err(message) => error.set(message),
                                }
                            });
                        },
                        Icon { name: "play" }
                        "{listen_label}"
                    }
                    button { class: "home-feature-arrow", r#type: "button",
                        "aria-label": "Open «{title}»",
                        onclick: move |_| actions.playlist.call(open_id.clone()),
                        Icon { name: "up" }
                    }
                }
                if !error_text.is_empty() {
                    p { class: "home-feature-error", role: "alert", "{error_text}" }
                }
            }
            div { class: "home-feature-indicators", role: "group", "aria-label": "Other playlists",
                for (position, target) in targets.iter().enumerate() {
                    { let active = position == index; let name = target.name.clone(); rsx! {
                        button { key: "{target.id}",
                            class: if active { "home-indicator active" } else { "home-indicator" },
                            r#type: "button",
                            "aria-label": "Show {name}",
                            "aria-current": if active { "true" } else { "false" },
                            onclick: move |_| selected.set(position)
                        }
                    } }
                }
            }
        }
    }
}

/// My Wave reads the owner-scoped recommendation snapshot on demand. The read
/// never pretends to be a mix: the play control stays unavailable until real
/// candidates arrive, and a failed read keeps the last confirmed data.
#[component]
fn WaveCard(
    auth: Signal<Option<AuthResponse>>,
    revision: Signal<u64>,
    actions: Actions,
) -> Element {
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let dropped = alive.clone();
    let reads = use_hook(RequestScope::default);
    let cleanup = reads.clone();
    use_drop(move || {
        dropped.set(false);
        cleanup.cancel();
    });
    let mut generation = use_signal(|| 0_u64);
    let mut data = use_signal(|| None::<Recommendations>);
    let mut owner = use_signal(String::new);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(String::new);

    use_effect(move || {
        reads.cancel();
        let session = token(auth);
        let _auth = auth.read();
        let _revision = *revision.read();
        let request = generation.peek().wrapping_add(1);
        generation.set(request);
        error.set(String::new());
        // A new account never sees the previous account's recommendations.
        if *owner.peek() != session {
            data.set(None);
            owner.set(session.clone());
        }
        if session.is_empty() {
            loading.set(false);
            return;
        }
        loading.set(true);
        let signal = match reads.begin() {
            Ok(signal) => signal,
            Err(message) => {
                error.set(message);
                loading.set(false);
                return;
            }
        };
        let alive = alive.clone();
        spawn_authorized(auth, async move {
            let result = get::<Recommendations>(
                &format!("/api/recommendations?limit={WAVE_LIMIT}"),
                &signal,
            )
            .await;
            if !alive.get() || token(auth) != session || *generation.peek() != request {
                return;
            }
            loading.set(false);
            match result {
                Ok(value) => data.set(Some(value)),
                // The last confirmed mix stays on screen; only the message changes.
                Err(message) => error.set(message),
            }
        });
    });

    let session = token(auth);
    let known_owner = !session.is_empty() && *owner.read() == session;
    let snapshot = if known_owner {
        data.read().clone()
    } else {
        None
    };
    let pending = *loading.read();
    let error_text = error.read().clone();
    let candidates: Vec<Track> = snapshot
        .as_ref()
        .map(|value| {
            value
                .discovery
                .iter()
                .chain(value.rotation.iter())
                .map(|entry| entry.track.clone())
                .collect()
        })
        .unwrap_or_default();
    // The quiet tag is honest: it appears only when audio features really
    // shaped a non-empty mix.
    let tuned = snapshot.as_ref().is_some_and(|value| {
        value.uses_audio_features && (!value.discovery.is_empty() || !value.rotation.is_empty())
    });
    let wave_playing = candidates
        .first()
        .is_some_and(|track| actions.is_playing(&track.id));
    let play_label = if wave_playing {
        "Pause «Your wave»"
    } else {
        "Play «Your wave»"
    };
    let play_icon = if wave_playing { "pause" } else { "play" };

    rsx! {
        article { class: if candidates.is_empty() { "home-wave home-wave-empty" } else { "home-wave" },
            div { class: "home-wave-copy",
                div { class: "home-wave-top",
                    span { class: "eyebrow home-eyebrow", Icon { name: "jobs" } "{WAVE_EYEBROW}" }
                    button { class: "home-wave-open", r#type: "button",
                        "aria-label": "Show personal mixes",
                        onclick: move |_| scroll_to_personal_mixes(),
                        Icon { name: "up" }
                    }
                }
                div { class: "home-wave-bottom",
                    div { class: "home-wave-text",
                        h2 { class: "home-wave-title", "{WAVE_TITLE}" }
                        p { class: "home-wave-subtitle", "{WAVE_SUBTITLE}" }
                        if tuned {
                            span { class: "home-wave-tuned", "Tuned to you" }
                        }
                        if pending && candidates.is_empty() {
                            p { class: "home-wave-note", role: "status", "Loading your mix…" }
                        } else if !error_text.is_empty() {
                            p { class: "home-wave-note home-wave-error", role: "alert", "{error_text}" }
                        } else if candidates.is_empty() {
                            div { class: "home-wave-note",
                                p { "No suitable tracks yet" }
                                button { class: "button secondary small", r#type: "button",
                                    "aria-label": "Open library",
                                    onclick: move |_| actions.navigate.call("library".into()), "Open library" }
                            }
                        }
                    }
                    button { class: "home-wave-play", r#type: "button",
                        disabled: pending || candidates.is_empty(),
                        "aria-label": play_label,
                        "aria-pressed": wave_playing,
                        onclick: move |_| {
                            let tracks = candidates.clone();
                            if tracks.is_empty() {
                                return;
                            }
                            let context = PlaybackContext::new(ContextKind::Home, WAVE_TITLE);
                            // A fresh wave starts the queue. A wave that is
                            // already playing toggles its pause, so the pause
                            // face never promises a control it does not have.
                            if wave_playing {
                                actions.play.call((tracks, 0, context));
                            } else {
                                actions.start.call((tracks, 0, context));
                            }
                        },
                        Icon { name: play_icon }
                    }
                }
            }
        }
    }
}
