//! TR07a: the right modal queue drawer. A projection of the root order,
//! never a second player and never a second copy of the queue.
use crate::icons::Icon;
use crate::playback::Projection;
use crate::queue_dialog;
use crate::seconds;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use vela_domain::Track;

/// The queue drawer: a projection of the root's order, a native modal
/// dialog and nothing else.
///
/// The drawer has no closer of its own. Every exit — Escape, the close
/// button and "Open library" — calls the root, and the root closes the
/// native dialog before the drawer leaves the DOM (a modal dialog that is
/// removed while it is open drops the focus on the document body), clears
/// the announcement line and hands the focus back to the connected control
/// that opened the drawer. The time format is the root's own `seconds()`
/// helper, the same one the Now Playing sheet uses, so there is one format
/// in the crate and not a copy per surface.
///
/// DR-03: the receipt of one successful move.
///
/// The order has already changed when the receipt is published; the receipt
/// only says so. `generation` is the edit generation of the root, `at` is
/// the row that has just moved and `direction` is the way it came
/// (`-1` up, `1` down). `focus` says that the move control owned the focus,
/// so the root can hand the focus to the same control of the destination
/// row after the renderer rebuilt it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct QueueMoveReceipt {
    pub generation: u64,
    pub at: usize,
    pub direction: i8,
    pub focus: bool,
}

#[component]
pub fn QueueDrawer(
    queue: Vec<Track>,
    projection: Projection,
    current: Option<usize>,
    playing: bool,
    requested: bool,
    status: String,
    context_label: String,
    shuffle: bool,
    repeat: String,
    position: f64,
    duration: f64,
    volume: f64,
    sleep_minutes: String,
    sleep_remaining: u64,
    sleep_status: String,
    receipt: Option<QueueMoveReceipt>,
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
    onseek: EventHandler<f64>,
    onvolume: EventHandler<f64>,
    onretry: EventHandler<()>,
    onsleep: EventHandler<String>,
    onchoose: EventHandler<()>,
    onsealed: EventHandler<u64>,
) -> Element {
    // The drawer is a native modal dialog: the browser owns the focus
    // trap, the top layer and the backdrop. The element is looked up
    // after the renderer's DOM batch, because the lean Dioxus web build
    // has no mounted-event support. The control that opened the drawer
    // is frozen by the root, not here: this effect only opens the
    // surface and never reads a signal, so no later render can replace
    // the frozen trigger.
    use_effect(move || {
        spawn(async move {
            TimeoutFuture::new(0).await;
            if let Some(dialog) = queue_dialog() {
                let _ = dialog.show_modal();
            }
        });
    });

    let next_occurrence = projection.scheduled_next;
    let next_title = next_occurrence
        .and_then(|at| queue.get(at))
        .map(|track| track.title.clone());
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
    let shuffle_pending = shuffle && next_occurrence.is_none() && !projection.next().is_empty();
    // "Upcoming" means the occurrences after the playing one, exactly the set
    // the root's clear-upcoming edit removes.
    let cursor = current.unwrap_or(0);
    let has_upcoming = queue.len() > cursor.saturating_add(1);
    let state = if !has_current {
        "Nothing playing"
    } else if !status.is_empty() {
        "Playback needs attention"
    } else if playing {
        "Playing"
    } else if requested {
        "Connecting…"
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
    // The seek rail uses the real known duration. An unknown duration stays a
    // zero-length range rather than an invented length.
    let seek_max = if duration.is_finite() && duration > 0.0 {
        duration
    } else {
        1.0
    };
    // DR-07: both rails announce a value a person can act on. The volume is a
    // rounded percentage of the real output level. The position is a real
    // "m:ss of m:ss" pair; an unknown duration is announced as unknown, the
    // technical one-second maximum is never read as a track length.
    let volume_text = format!("{}%", (volume.clamp(0.0, 1.0) * 100.0).round() as i64);
    let seek_text = if duration.is_finite() && duration > 0.0 {
        format!(
            "{} of {}",
            seconds(position.min(duration)),
            seconds(duration)
        )
    } else {
        "Duration unknown".to_string()
    };
    // DR-07: the sleep countdown ticks every second. A polite live region
    // would read "4:58", "4:57", "4:56" without a pause, so the countdown is
    // a `timer` with an explicit `aria-live="off"`. Its name carries the
    // remaining time, so a person who navigates to it still hears that the
    // timer runs and how long is left.
    let sleep_text = if sleep_remaining > 0 {
        seconds(sleep_remaining as f64)
    } else {
        String::new()
    };
    // DR-07: the sleep timer's own announcement. The countdown is a quiet
    // timer, so the start, the cancel and the expiry are one meaningful
    // message each. The root writes that message into the drawer's status
    // line; the line changes only on those transitions, never on a tick.
    //
    // Escape, the close button and "Open library" are the same command:
    // the root closes the native dialog before the drawer leaves the DOM
    // and hands the focus back to the control that opened it. The drawer
    // keeps no closer of its own, so there is one closing path and not a
    // copy of it per surface.
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
                        span { class: "eyebrow", "NOW PLAYING" }
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
                    label { class: "queue-field-label", r#for: "queue-volume", "Volume" }
                    input {
                        id: "queue-volume",
                        r#type: "range",
                        min: "0",
                        max: "1",
                        step: "0.01",
                        value: "{volume}",
                        "aria-label": "Queue volume",
                        "aria-valuetext": "{volume_text}",
                        oninput: move |event| {
                            if let Ok(value) = event.value().parse::<f64>() {
                                onvolume.call(value);
                            }
                        },
                    }
                }

                // The modal background is inert, so the drawer carries the seek
                // itself: transport must stay reachable while it is open.
                div { class: "queue-seek",
                    label { class: "queue-field-label", r#for: "queue-position", "Playback position" }
                    input {
                        id: "queue-position",
                        r#type: "range",
                        min: "0",
                        max: "{seek_max}",
                        step: "0.1",
                        value: "{position}",
                        "aria-label": "Queue playback position",
                        "aria-valuetext": "{seek_text}",
                        disabled: !has_current,
                        oninput: move |event| {
                            if let Ok(value) = event.value().parse::<f64>() {
                                onseek.call(value);
                            }
                        },
                    }
                }

                hr { class: "queue-divider" }

                div { class: "queue-upcoming-heading",
                    h3 { "Saved queue" }
                    span { class: "queue-count", "{queue.len()}" }
                }

                if let Some(title) = next_title {
                    p { class: "queue-note", "Next: {title}" }
                }
                if shuffle_pending {
                    p { class: "queue-note", "Shuffle is on. The order is decided when playback starts." }
                }
                if !status.is_empty() {
                    div { class: "queue-playback-error", role: "alert",
                        p { "{status}" }
                        button { class: "button secondary", onclick: move |_| onretry.call(()), "Retry playback" }
                    }
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
                            let duration = seconds(track.duration_ms as f64 / 1000.0);
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
                            // DR-03: the row that has just moved settles for
                            // a moment. The order changed before the effect
                            // started, and the effect only says so.
                            let settle = receipt.filter(|receipt| receipt.at == at);
                            let settle_class = if settle.is_some() {
                                format!("{row_class} queue-row-settle")
                            } else {
                                row_class.to_string()
                            };
                            let settle_from = settle
                                .map(|receipt| if receipt.direction < 0 { "-4px" } else { "4px" })
                                .unwrap_or_default();
                            let settle_generation = settle.map(|receipt| receipt.generation);
                            rsx! {
                                div { class: settle_class, key: "{at}-{track_id}",
                                    style: if settle_from.is_empty() { String::new() } else { format!("--queue-settle-from: {settle_from}") },
                                    onanimationend: {
                                        let onsealed = onsealed;
                                        move |event: Event<AnimationData>| {
                                            if event.animation_name() != "vela-queue-settle" { return; }
                                            if let Some(generation) = settle_generation {
                                                onsealed.call(generation);
                                            }
                                        }
                                    },
                                    button {
                                        class: "queue-track",
                                        id: "queue-track-{at}",
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
                                        span { class: "queue-row-badge", "On request" }
                                    }
                                    div { class: "queue-row-actions",
                                        button {
                                            id: "queue-move-up-{at}",
                                            class: "icon-button",
                                            r#type: "button",
                                            "aria-label": "Move track up",
                                            disabled: at == 0,
                                            onclick: move |_| onmoveup.call(at),
                                            Icon { name: "up" }
                                        }
                                        button {
                                            id: "queue-move-down-{at}",
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
                        p { "No tracks queued. Add tracks from your library." }
                        button {
                            class: "button secondary",
                            r#type: "button",
                            "aria-label": "Open library",
                            onclick: move |_| onchoose.call(()),
                            "Open library"
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

                // DR-07: one announcement surface inside the drawer, and it
                // is always there. The live region exists before the first
                // transition, so an announcement depends on a change of its
                // text and never on the insertion of the region itself. The
                // modal background is inert, so the root's receipt cannot say
                // anything while the drawer is open; this line is the only
                // place the sleep timer's start, cancel and expiry are spoken
                // from inside the drawer. The line and the sleep row share
                // one wrapper, so an empty line takes no space at all.
                div { class: "queue-sleep",
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
                            small {
                                role: "timer",
                                "aria-live": "off",
                                "aria-label": "Pauses in {sleep_text}",
                                "Pauses in {sleep_text}"
                            }
                            button {
                                class: "text-button",
                                r#type: "button",
                                "aria-label": "Cancel sleep timer",
                                onclick: move |_| onsleep.call("0".to_string()),
                                "Cancel sleep timer"
                            }
                        }
                    }
                    p {
                        class: if sleep_status.is_empty() { "queue-sleep-status is-empty" } else { "queue-sleep-status" },
                        role: "status",
                        "{sleep_status}"
                    }
                }

            }
        }
    }
}
