//! Presentation of the root playback session. This view never owns audio or HTTP.
use crate::{Art, bytes, icons::Icon, seconds, session::PlaybackContext};
use dioxus::prelude::*;
use vela_domain::Track;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, PartialEq)]
pub struct Actions {
    pub close: EventHandler<()>,
    pub failed: EventHandler<String>,
    pub toggle: EventHandler<()>,
    pub previous: EventHandler<()>,
    pub next: EventHandler<()>,
    pub shuffle: EventHandler<()>,
    pub repeat: EventHandler<()>,
    pub seek: EventHandler<f64>,
    pub volume: EventHandler<f64>,
    pub favorite: EventHandler<Track>,
    pub retry: EventHandler<()>,
    pub queue: EventHandler<()>,
    pub select: EventHandler<usize>,
}

pub fn dialog() -> Option<web_sys::HtmlDialogElement> {
    web_sys::window()?
        .document()?
        .get_element_by_id("now-playing-dialog")?
        .dyn_into()
        .ok()
}

#[component]
pub fn NowPlaying(
    track: Track,
    context: PlaybackContext,
    playing: bool,
    requested: bool,
    position: f64,
    duration: f64,
    volume: f64,
    shuffle: bool,
    repeat: String,
    status: String,
    error: String,
    favorite_busy: bool,
    queue: Signal<Vec<Track>>,
    cursor: usize,
    actions: Actions,
) -> Element {
    let favorite = track.clone();
    // An inspected queued item may not have loaded browser metadata yet.
    let duration = if duration.is_finite() && duration > 0.0 {
        duration
    } else {
        track.duration_ms.max(0) as f64 / 1000.0
    };
    let count = queue.read().len();
    let toggle_icon = if playing || requested {
        "pause"
    } else {
        "play"
    };
    let repeat_icon = if repeat == "one" {
        "repeat-one"
    } else {
        "repeat"
    };
    let preview = queue
        .read()
        .iter()
        .enumerate()
        .skip(cursor.saturating_add(1))
        .take(5)
        .map(|(at, track)| (at, track.clone()))
        .collect::<Vec<_>>();
    // The existing lean Dioxus web build does not enable mounted-event support.
    // Like MetadataEditor, wait for the renderer's DOM batch. The task belongs
    // to this component scope and is dropped if the view closes before then.
    use_effect(move || {
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(0).await;
            if dialog().is_none_or(|dialog| dialog.show_modal().is_err()) {
                actions.failed.call(
                    "The full player could not open. Use the playback bar, or reload and try again."
                        .into(),
                );
            }
        });
    });
    rsx! {
        dialog {id:"now-playing-dialog",class:"now-playing-dialog",role:"dialog","aria-modal":"true","aria-label":"Now playing",
            oncancel:move |event|{event.prevent_default();actions.close.call(());},

            div {class:"np-layout",
                header {class:"np-header",
                    div {class:"np-heading",span {class:"eyebrow","NOW PLAYING"} p {class:"np-context",title:context.label.clone(),"Playing from {context.label}"}}
                    button {class:"icon-button np-close","aria-label":"Close now playing","data-dialog-dismiss":"",onclick:move |_|actions.close.call(()),Icon{name:"close"}}
                }
                section {class:"np-main","aria-label":"Current track",
                    div {class:"np-artwork",Art {key:"{track.id}-{track.artwork_id:?}",track:track.clone(),large:true}}
                    div {class:"np-title-row",
                        div {class:"np-title",h1 {"{track.title}"} p {class:"np-artist","{track.artist}"} if !track.album.is_empty() {p {class:"np-album","{track.album}"}}}
                        button {class:if track.favorite {"icon-button selected"} else {"icon-button"},"aria-label":if track.favorite {"Remove current track from favorites"} else {"Favorite current track"},"aria-pressed":track.favorite,disabled:favorite_busy,onclick:move |_|actions.favorite.call(favorite.clone()),Icon{name:"heart"}}
                    }
                    if !error.is_empty(){div {class:"np-error",role:"alert","{error}"}}
                    div {class:"np-seek",
                        input {r#type:"range",min:"0",max:if duration.is_finite(){duration.max(1.0).to_string()}else{"1".into()},step:"0.1",value:"{position}","aria-label":"Playback position",oninput:move |event|if let Ok(value)=event.value().parse::<f64>() {actions.seek.call(value);}}
                        div {class:"np-times",span {"{seconds(position)}"} span {"{seconds(duration)}"}}
                    }
                    div {class:"np-transport",
                        button {class:if shuffle {"icon-button selected"}else{"icon-button"},"aria-label":"Shuffle","aria-pressed":shuffle,onclick:move |_|actions.shuffle.call(()),Icon{name:"shuffle"}}
                        button {class:"icon-button","aria-label":"Previous track",onclick:move |_|actions.previous.call(()),Icon{name:"previous"}}
                        button {class:"np-toggle","aria-label":if playing||requested {"Pause"}else{"Play"},onclick:move |_|actions.toggle.call(()),Icon{name:toggle_icon}}
                        button {class:"icon-button","aria-label":"Next track",onclick:move |_|actions.next.call(()),Icon{name:"next"}}
                        button {class:if repeat!="off" {"icon-button selected"}else{"icon-button"},"aria-label":"Repeat: {repeat}","aria-pressed":repeat!="off",onclick:move |_|actions.repeat.call(()),Icon{name:repeat_icon}}
                    }
                    div {class:"np-volume",Icon{name:"volume"} label {class:"sr-only",r#for:"now-playing-volume","Volume"} input {id:"now-playing-volume",r#type:"range",min:"0",max:"1",step:"0.01",value:"{volume}",oninput:move |event|if let Ok(value)=event.value().parse::<f64>(){actions.volume.call(value);}}}
                    if !status.is_empty(){div {class:"np-connection",role:"status",p {"{status}"} button {class:"button secondary",onclick:move |_|actions.retry.call(()),"Retry playback"}}}
                }
                aside {class:"np-secondary","aria-label":"Track details and queue",
                    section {class:"np-section","aria-labelledby":"np-details-title",
                        h2 {id:"np-details-title","Your original audio"}
                        p {if playing {"Playing"}else if requested {"Connecting"}else{"Paused"} " · Direct stream · Browser decoder"}
                        dl {class:"np-details",
                            div {dt {"Format"} dd {"{track.codec}"}}
                            if let Some(value)=track.sample_rate {div {dt {"Sample rate"} dd {"{value} Hz"}}}
                            if let Some(value)=track.bit_depth {div {dt {"Bit depth"} dd {"{value} bits"}}}
                            if let Some(value)=track.bitrate {div {dt {"Bitrate"} dd {"{value} kbps"}}}
                            if let Some(value)=track.channels {div {dt {"Channels"} dd {"{value}"}}}
                            div {dt {"File size"} dd {"{bytes(track.size_bytes)}"}}
                            if let Some(value)=track.year {div {dt {"Year"} dd {"{value}"}}}
                            if !track.genre.is_empty(){div {dt {"Genre"} dd {"{track.genre}"}}}
                            if !track.album_artist.is_empty(){div {dt {"Album artist"} dd {"{track.album_artist}"}}}
                            if let Some(value)=track.disc_number {div {dt {"Disc"} dd {"{value}"}}}
                            if let Some(value)=track.track_number {div {dt {"Track"} dd {"{value}"}}}
                        }
                    }
                    section {class:"np-section","aria-labelledby":"np-queue-title",
                        h2 {id:"np-queue-title","Your queue"} p {"Track {cursor+1} of {count}"}
                        if shuffle {p {"Shuffle is on. Saved order shown; Play next requests take priority."}}
                        if preview.is_empty(){p {"No later tracks in the saved order."}}else{
                            div {class:"np-queue-list",for (at,item) in preview {button {key:"{at}-{item.id}",class:"np-queue-item","aria-label":"Play {item.title} from queue",onclick:move |_|actions.select.call(at),Art {track:item.clone()} div {strong {"{item.title}"} small {"{item.artist}"}}}}}
                        }
                        div {class:"np-queue-footer",button {class:"button secondary",onclick:move |_|actions.queue.call(()),Icon{name:"queue"} "View full queue"}}
                    }
                }
            }
        }
    }
}
