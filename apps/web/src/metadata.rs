//! Catalog-only editor. Each mounted dialog owns its request futures and draft.
use crate::{api, spawn_authorized, token};
use dioxus::prelude::*;
use serde_json::json;
use vela_domain::{AuthResponse, Track, TrackMetadata, UpdateTrackMetadata};
use wasm_bindgen::JsCast;

#[derive(Clone)]
struct Draft {
    revision: i64,
    title: String,
    artist: String,
    album: String,
    album_artist: String,
    genre: String,
    year: String,
    track_number: String,
    disc_number: String,
}
impl From<TrackMetadata> for Draft {
    fn from(value: TrackMetadata) -> Self {
        let track = value.track;
        let number = |value: Option<i64>| value.map(|n| n.to_string()).unwrap_or_default();
        Self {
            revision: value.revision,
            title: track.title,
            artist: track.artist,
            album: track.album,
            album_artist: track.album_artist,
            genre: track.genre,
            year: number(track.year),
            track_number: number(track.track_number),
            disc_number: number(track.disc_number),
        }
    }
}
impl Draft {
    fn fields(&self) -> [(&'static str, &'static str, String, bool); 8] {
        [
            ("title", "Title", self.title.clone(), false),
            ("artist", "Artist", self.artist.clone(), false),
            ("album", "Album", self.album.clone(), false),
            (
                "album_artist",
                "Album artist",
                self.album_artist.clone(),
                false,
            ),
            ("genre", "Genre", self.genre.clone(), false),
            ("year", "Year", self.year.clone(), true),
            (
                "track_number",
                "Track number",
                self.track_number.clone(),
                true,
            ),
            ("disc_number", "Disc number", self.disc_number.clone(), true),
        ]
    }
    fn set(&mut self, field: &str, value: String) {
        match field {
            "title" => self.title = value,
            "artist" => self.artist = value,
            "album" => self.album = value,
            "album_artist" => self.album_artist = value,
            "genre" => self.genre = value,
            "year" => self.year = value,
            "track_number" => self.track_number = value,
            "disc_number" => self.disc_number = value,
            _ => {}
        }
    }
    fn update(&self) -> Result<UpdateTrackMetadata, String> {
        let number = |label: &str, value: &str| {
            if value.trim().is_empty() {
                return Ok(None);
            }
            value
                .parse::<i64>()
                .ok()
                .filter(|v| (1..=9999).contains(v))
                .map(Some)
                .ok_or_else(|| {
                    format!("{label} must be a whole number between 1 and 9999, or left blank.")
                })
        };
        Ok(UpdateTrackMetadata {
            revision: self.revision,
            title: self.title.clone(),
            artist: self.artist.clone(),
            album: self.album.clone(),
            album_artist: self.album_artist.clone(),
            genre: self.genre.clone(),
            year: number("Year", &self.year)?,
            track_number: number("Track number", &self.track_number)?,
            disc_number: number("Disc number", &self.disc_number)?,
        })
    }
}
#[derive(Clone, Copy)]
struct Editor {
    auth: Signal<Option<AuthResponse>>,
    draft: Signal<Option<Draft>>,
    loading: Signal<bool>,
    saving: Signal<bool>,
    error: Signal<String>,
    conflict: Signal<bool>,
    generation: Signal<u64>,
}
fn load(id: String, mut editor: Editor) {
    let generation = editor.generation.peek().wrapping_add(1);
    editor.generation.set(generation);
    editor.loading.set(true);
    editor.error.set(String::new());
    editor.conflict.set(false);
    editor.draft.set(None);
    spawn_authorized(editor.auth, async move {
        let result =
            api::<TrackMetadata>("GET", &format!("/api/tracks/{id}/metadata"), None, "").await;
        if *editor.generation.peek() != generation {
            return;
        }
        match result {
            Ok(value) => editor.draft.set(Some(value.into())),
            Err(message) => editor.error.set(message),
        }
        editor.loading.set(false);
    });
}
fn save(id: String, mut editor: Editor, onsaved: EventHandler<Track>) {
    if *editor.loading.peek() || *editor.saving.peek() {
        return;
    }
    let Some(draft) = editor.draft.peek().clone() else {
        return;
    };
    let update = match draft.update() {
        Ok(value) => value,
        Err(message) => {
            editor.error.set(message);
            return;
        }
    };
    let generation = editor.generation.peek().wrapping_add(1);
    editor.generation.set(generation);
    editor.saving.set(true);
    editor.error.set(String::new());
    editor.conflict.set(false);
    spawn_authorized(editor.auth, async move {
        let result = api::<TrackMetadata>(
            "PATCH",
            &format!("/api/tracks/{id}/metadata"),
            Some(json!(update)),
            &token(editor.auth),
        )
        .await;
        if *editor.generation.peek() != generation {
            return;
        }
        editor.saving.set(false);
        match result {
            Ok(value) => onsaved.call(value.track),
            Err(message) => {
                editor.conflict.set(message.contains("Metadata changed"));
                editor.error.set(message);
            }
        }
    });
}

#[component]
pub fn MetadataEditor(
    track_id: String,
    auth: Signal<Option<AuthResponse>>,
    onclose: EventHandler<()>,
    onsaved: EventHandler<Track>,
) -> Element {
    let mut editor = Editor {
        auth,
        draft: use_signal(|| None),
        loading: use_signal(|| true),
        saving: use_signal(|| false),
        error: use_signal(String::new),
        conflict: use_signal(|| false),
        generation: use_signal(|| 0),
    };
    let initial_id = track_id.clone();
    let save_id = track_id.clone();
    use_effect(move || {
        load(initial_id.clone(), editor);
        // Wait for the renderer's DOM batch before opening the native modal.
        spawn_authorized(auth, async move {
            gloo_timers::future::TimeoutFuture::new(0).await;
            if let Some(dialog) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("metadata-editor"))
                .and_then(|e| e.dyn_into::<web_sys::HtmlDialogElement>().ok())
            {
                let _ = dialog.show_modal();
            }
        });
    });
    rsx! { dialog { id:"metadata-editor",class:"metadata-dialog","aria-labelledby":"metadata-heading",
        oncancel:move |event| {event.prevent_default();if !*editor.saving.peek(){onclose.call(());}},
        div {class:"metadata-heading",span {class:"eyebrow","YOUR MUSIC, YOUR DETAILS"} h2 {id:"metadata-heading","Edit metadata"}
            p {"Changes are saved in your catalog. Your original audio file and embedded tags stay unchanged."}}
        if !editor.error.read().is_empty() {div {class:"alert error",role:"alert","{editor.error}"}}
        if *editor.conflict.read() {p {class:"metadata-hint","Your draft is still here. Reloading discards it and opens the latest saved details."}}
        if *editor.loading.read() {div {class:"empty-state compact",div {class:"spinner"} p {"Loading current metadata…"}}}
        form {onsubmit:move |event|{event.prevent_default();save(save_id.clone(),editor,onsaved);},
            if let Some(draft)=editor.draft.read().clone() {div {class:"metadata-fields",
                for (field,label,value,numeric) in draft.fields() {div {class:if numeric {"metadata-field number-field"}else{"metadata-field"},
                    label {r#for:"metadata-{field}","{label}"}
                    input {id:"metadata-{field}",r#type:if numeric {"number"}else{"text"},value,
                        required:field=="title",min:if numeric {"1"}else{""},max:if numeric {"9999"}else{""},
                        maxlength:if field=="title" {512}else{1024},disabled:*editor.saving.read(),
                        oninput:move |event|{if let Some(draft)=editor.draft.write().as_mut(){draft.set(field,event.value());}}}
                } }
                p {class:"metadata-hint","Leave optional details blank when unknown."}
            }}
            div {class:"metadata-footer",
                button {r#type:"button",class:"button secondary",disabled:*editor.saving.read(),onclick:move |_|onclose.call(()),"Cancel"}
                if *editor.conflict.read() || (!editor.error.read().is_empty() && editor.draft.read().is_none()) {
                    button {r#type:"button",class:"button secondary",disabled:*editor.loading.read()||*editor.saving.read(),onclick:move |_|load(track_id.clone(),editor),"Reload current metadata"}
                }
                button {r#type:"submit",class:"button primary",disabled:*editor.loading.read()||*editor.saving.read()||editor.draft.read().is_none(),if *editor.saving.read(){"Saving…"}else{"Save metadata"}}
            }
        }
    } }
}
