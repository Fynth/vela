//! TR07a: the right modal queue drawer. A projection of the root order,
//! never a second player and never a second copy of the queue.
use crate::icons::Icon;
use crate::playback::Projection;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use vela_domain::Track;
use wasm_bindgen::JsCast;

/// The drawer is a native modal dialog: the browser owns the focus trap, the
/// top layer and the backdrop. This mirrors `now_playing::dialog`; the lean
/// Dioxus web build has no mounted-event support, so the element is looked up
/// after the renderer's DOM batch instead.
fn drawer() -> Option<web_sys::HtmlDialogElement> {
    web_sys::window()?
        .document()?
        .get_element_by_id("queue-drawer-dialog")?
        .dyn_into()
        .ok()
}

/// The root player owns the audible level. The drawer only mirrors the
/// control it was opened with so the slider never shows a level the root
/// does not have.
fn player_volume() -> Option<f64> {
    let input = web_sys::window()?
        .document()?
        .get_element_by_id("player-volume")?
        .dyn_into::<web_sys::HtmlInputElement>()
        .ok()?;
    input.value().parse::<f64>().ok()
}

/// Format a millisecond duration as `m:ss`.
fn format_duration(duration_ms: i64) -> String {
    let total_seconds = if duration_ms > 0 {
        duration_ms / 1000
    } else {
        0
    };
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
}

#[component]
pub fn QueueDrawer(
    queue: Vec<Track>,
    projection: Projection,
    current: Option<usize>,
    playing: bool,
    requested: bool,
    context_label: String,
    shuffle: bool,
    repeat: String,
    sleep_minutes: String,
    sleep_remaining: u64,
    onclose: EventHandler<()>,
    onselect: EventHandler<usize>,
    onremove: EventHandler<usize>,
    onmoveup: EventHandler<usize>,
    onmovedown: EventHandler<usize>,
    onclear: EventHandler<()>,
    onprevious: EventHandler<()>,
    onnext: EventHandler<()>,
    ontoggle: EventHandler<()>,
    onshuffle: EventHandler<()>,
    onrepeat: EventHandler<()>,
    onvolume: EventHandler<f64>,
    onsleep: EventHandler<String>,
    onchoose: EventHandler<()>,
) -> Element {
    let mut volume_value = use_signal(|| 1.0_f64);

    use_effect(move || {
        spawn(async move {
            TimeoutFuture::new(0).await;
            if let Some(dialog) = drawer() {
                let _ = dialog.show_modal();
            }
            if let Some(value) = player_volume() {
                volume_value.set(value);
            }
        });
    });

    let next_list = projection.next();
    let next_count = next_list.len();
    let next_occurrence = next_list.first().copied();
    let explicit_next = projection.explicit_next.clone();
    let current_index = current.unwrap_or(usize::MAX);
    let last_index = queue.len().saturating_sub(1);
    let current_track: Option<Track> = current.and_then(|at| queue.get(at).cloned());
    let has_current = current_track.is_some();
    let current_title = current_track
        .as_ref()
        .map(|track| track.title.clone())
        .unwrap_or_else(|| "Nothing playing".to_string());
    let current_artist = current_track
        .as_ref()
        .map(|track| track.artist.clone())
        .unwrap_or_default();
    // The real running order is the pending deck only when shuffle is off.
    // Under shuffle the deck is drawn at play time, so the drawer says so
    // instead of presenting the pending list as a guarantee.
    let shuffle_pending = shuffle && explicit_next.is_empty();
    // "Upcoming" means the occurrences after the playing one, exactly the set
    // the root's clear-upcoming edit removes.
    let cursor = current.unwrap_or(0);
    let has_upcoming = queue.len() > cursor.saturating_add(1);
    let state = if !has_current {
        "Nothing playing"
    } else if playing {
        "Playing"
    } else {
        "Paused"
    };
    // Dioxus 0.7 turns an `if` expression in a component property into a
    // `String`; the icon name is a `&'static str`, so it is resolved first.
    let toggle_icon = if playing || requested {
        "pause"
    } else {
        "play"
    };

    rsx! {
        dialog {
            id: "queue-drawer-dialog",
            class: "queue-drawer",
            role: "dialog",
            "aria-modal": "true",
            "aria-label": "Playback queue",
            oncancel: move |event| {
                event.prevent_default();
                onclose.call(());
            },
            div {
                class: "queue-panel",
                role: "complementary",
                "aria-label": "Playback queue",

                header { class: "queue-header",
                    div { class: "queue-heading",
                        span { class: "eyebrow", "СЕЙЧАС ИГРАЕТ" }
                        p { class: "queue-context", title: "{context_label}", "Playing from {context_label}" }
                    }
                    button {
                        class: "icon-button queue-close",
                        r#type: "button",
                        "aria-label": "Close playback queue",
                        onclick: move |_| onclose.call(()),
                        Icon { name: "close" }
                    }
                }

                section { class: "queue-current", "aria-label": "Current track",
                    div { class: "artwork queue-current-art",
                        if let Some(id) = current_track.as_ref().and_then(|track| track.artwork_id.clone()) {
                            img { src: "/api/artwork/{id}", alt: "", loading: "lazy" }
                        } else {
                            span { "♪" }
                        }
                    }
                    div { class: "queue-current-meta",
                        strong { class: "queue-current-title", "{current_title}" }
                        small { class: "queue-current-artist", "{current_artist}" }
                        span { class: "queue-current-state", "{state}" }
                    }
                    button {
                        class: "icon-button queue-toggle",
                        r#type: "button",
                        "aria-label": if playing || requested { "Pause playback" } else { "Resume playback" },
                        onclick: move |_| ontoggle.call(()),
                        Icon { name: toggle_icon }
                    }
                }

                div { class: "queue-transport",
                    button {
                        class: "icon-button",
                        r#type: "button",
                        "aria-label": "Previous in queue",
                        onclick: move |_| onprevious.call(()),
                        Icon { name: "previous" }
                    }
                    button {
                        class: "icon-button",
                        r#type: "button",
                        "aria-label": "Next in queue",
                        onclick: move |_| onnext.call(()),
                        Icon { name: "next" }
                    }
                    button {
                        class: if shuffle { "icon-button selected" } else { "icon-button" },
                        r#type: "button",
                        "aria-label": "Shuffle",
                        "aria-pressed": shuffle,
                        onclick: move |_| onshuffle.call(()),
                        Icon { name: "shuffle" }
                    }
                    button {
                        class: if repeat != "off" { "icon-button selected" } else { "icon-button" },
                        r#type: "button",
                        "aria-label": "Repeat in queue: {repeat}",
                        "aria-pressed": repeat != "off",
                        onclick: move |_| onrepeat.call(()),
                        Icon { name: "repeat" }
                    }
                }

                div { class: "queue-volume",
                    input {
                        id: "queue-volume",
                        r#type: "range",
                        min: "0",
                        max: "1",
                        step: "0.01",
                        value: "{volume_value}",
                        "aria-label": "Queue volume",
                        oninput: move |event| {
                            if let Ok(value) = event.value().parse::<f64>() {
                                onvolume.call(value);
                            }
                        },
                    }
                }

                hr { class: "queue-divider" }

                div { class: "queue-upcoming-heading",
                    h3 { "Дальше" }
                    span { class: "queue-count", "{next_count}" }
                }

                if shuffle_pending {
                    p { class: "queue-note", "Перемешивание: порядок определяется при воспроизведении." }
                }

                div { class: "queue-list",
                    for (at, track) in queue.iter().enumerate() {
                        {
                            let is_current = at == current_index;
                            let is_next = next_occurrence == Some(at);
                            let is_explicit = explicit_next.contains(&at);
                            let play_label = if is_current && requested { "Pause" } else { "Play" };
                            let title = track.title.clone();
                            let artist = track.artist.clone();
                            let artwork = track.artwork_id.clone();
                            let duration = format_duration(track.duration_ms);
                            let track_id = track.id.clone();
                            let row_class = if is_current && is_next {
                                "queue-row current queue-row-next"
                            } else if is_current {
                                "queue-row current"
                            } else if is_next {
                                "queue-row queue-row-next"
                            } else {
                                "queue-row"
                            };
                            rsx! {
                                div { class: row_class, key: "{at}-{track_id}",
                                    button {
                                        class: "queue-track",
                                        r#type: "button",
                                        "aria-label": "{play_label} queue item {at + 1}: {title}",
                                        onclick: move |_| onselect.call(at),
                                        div { class: "artwork",
                                            if let Some(id) = artwork.clone() {
                                                img { src: "/api/artwork/{id}", alt: "", loading: "lazy" }
                                            } else {
                                                span { "♪" }
                                            }
                                        }
                                        div { class: "queue-track-text",
                                            strong { "{title}" }
                                            small { "{artist}" }
                                        }
                                    }
                                    span { class: "queue-duration", "{duration}" }
                                    if is_next && is_explicit {
                                        span { class: "queue-row-badge", "По запросу" }
                                    }
                                    div { class: "queue-row-actions",
                                        button {
                                            class: "icon-button",
                                            r#type: "button",
                                            "aria-label": "Move track up",
                                            disabled: at == 0,
                                            onclick: move |_| onmoveup.call(at),
                                            Icon { name: "up" }
                                        }
                                        button {
                                            class: "icon-button",
                                            r#type: "button",
                                            "aria-label": "Move queue occurrence {at + 1} down",
                                            disabled: at == last_index,
                                            onclick: move |_| onmovedown.call(at),
                                            Icon { name: "down" }
                                        }
                                        button {
                                            class: "icon-button",
                                            r#type: "button",
                                            "aria-label": "Remove queued track",
                                            "aria-describedby": if is_current { "queue-current-remove-note" } else { "" },
                                            disabled: is_current,
                                            onclick: move |_| onremove.call(at),
                                            Icon { name: "close" }
                                        }
                                    }
                                    if is_current {
                                        span { id: "queue-current-remove-note", class: "sr-only",
                                            "The playing occurrence cannot be removed because that would stop the audio; remove later rows with Clear upcoming tracks instead."
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if queue.is_empty() {
                    div { class: "queue-empty",
                        p { "Здесь пока тихо. Добавьте треки из медиатеки." }
                        button {
                            class: "button secondary",
                            r#type: "button",
                            "aria-label": "Open the library",
                            onclick: move |_| onchoose.call(()),
                            "Открыть медиатеку"
                        }
                    }
                }

                button {
                    class: "button secondary queue-clear",
                    r#type: "button",
                    "aria-label": "Clear upcoming tracks",
                    disabled: !has_upcoming,
                    onclick: move |_| onclear.call(()),
                    "Clear upcoming tracks"
                }

                div { class: "inline-form sleep-timer",
                    label { r#for: "sleep-timer", "Sleep timer" }
                    select {
                        id: "sleep-timer",
                        value: "{sleep_minutes}",
                        onchange: move |event| onsleep.call(event.value()),
                        option { value: "0", selected: sleep_minutes == "0", "Off" }
                        option { value: "5", selected: sleep_minutes == "5", "5 minutes" }
                        option { value: "15", selected: sleep_minutes == "15", "15 minutes" }
                        option { value: "30", selected: sleep_minutes == "30", "30 minutes" }
                        option { value: "60", selected: sleep_minutes == "60", "60 minutes" }
                    }
                    if sleep_remaining > 0 {
                        small { role: "status", "Pauses in {format_duration((sleep_remaining * 1000) as i64)}" }
                        button {
                            class: "text-button",
                            r#type: "button",
                            "aria-label": "Cancel sleep timer",
                            onclick: move |_| onsleep.call("0".to_string()),
                            "Cancel sleep timer"
                        }
                    }
                }

            }
        }
    }
}
