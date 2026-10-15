mod filters;
mod metadata;
mod scanner;
use dioxus::prelude::*;
use gloo_net::http::{Request, Response};
use gloo_timers::future::TimeoutFuture;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use vela_domain::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{FormData, HtmlAudioElement, HtmlInputElement};

fn main() {
    dioxus::launch(App);
}

async fn api<T: DeserializeOwned>(
    method: &str,
    path: &str,
    body: Option<Value>,
    csrf: &str,
) -> Result<T, String> {
    let request = match method {
        "POST" => Request::post(path),
        "PUT" => Request::put(path),
        "PATCH" => Request::patch(path),
        "DELETE" => Request::delete(path),
        _ => Request::get(path),
    };
    let request = if csrf.is_empty() {
        request
    } else {
        request.header("X-CSRF-Token", csrf)
    };
    let response = if let Some(body) = body {
        request
            .json(&body)
            .map_err(|_| "Could not prepare the request.".to_string())?
            .send()
            .await
    } else {
        request.send().await
    }
    .map_err(|_| "Connection interrupted. Check that your server is available.".to_string())?;
    decode(response).await
}

async fn decode<T: DeserializeOwned>(response: Response) -> Result<T, String> {
    if !response.ok() {
        let status = response.status();
        return Err(response
            .json::<ApiError>()
            .await
            .map(|e| e.error)
            .unwrap_or_else(|_| format!("The server could not complete the request ({status}).")));
    }
    response
        .json()
        .await
        .map_err(|_| "The server returned an unreadable response.".to_string())
}

async fn mutate(method: &str, path: &str, body: Option<Value>, csrf: &str) -> Result<(), String> {
    let request = match method {
        "DELETE" => Request::delete(path),
        "PUT" => Request::put(path),
        _ => Request::post(path),
    }
    .header("X-CSRF-Token", csrf);
    let response = if let Some(body) = body {
        request
            .json(&body)
            .map_err(|_| "Could not prepare the request.".to_string())?
            .send()
            .await
    } else {
        request.send().await
    }
    .map_err(|_| "Connection interrupted. Please try again.".to_string())?;
    if response.ok() {
        Ok(())
    } else {
        Err(response
            .json::<ApiError>()
            .await
            .map(|e| e.error)
            .unwrap_or_else(|_| "The request was refused.".to_string()))
    }
}

fn token(auth: Signal<Option<AuthResponse>>) -> String {
    auth.peek()
        .as_ref()
        .map(|a| a.csrf_token.clone())
        .unwrap_or_default()
}

#[derive(Clone, Default, PartialEq, Eq)]
struct LibraryConditions {
    query: String,
    filters: TrackFilters,
}

#[derive(Clone, Copy)]
struct LibraryUi {
    tracks: Signal<Vec<Track>>,
    cursor: Signal<Option<String>>,
    loading: Signal<bool>,
    error: Signal<String>,
    filter_error: Signal<String>,
    conditions: Signal<LibraryConditions>,
    generation: Signal<u64>,
    auth: Signal<Option<AuthResponse>>,
    page: Signal<String>,
}

fn invalidate_library(mut ui: LibraryUi) {
    let next = ui.generation.peek().wrapping_add(1);
    ui.generation.set(next);
}

async fn load_library(after: Option<String>, ui: LibraryUi) {
    let conditions = ui.conditions.peek().clone();
    request_library(conditions, after, ui).await;
}

async fn request_library(conditions: LibraryConditions, after: Option<String>, mut ui: LibraryUi) {
    let expected_page = ui.page.peek().clone();
    if !matches!(expected_page.as_str(), "library" | "favorites") {
        return;
    }
    invalidate_library(ui);
    let generation = *ui.generation.peek();
    let session = token(ui.auth);
    if session.is_empty() {
        return;
    }
    ui.loading.set(true);
    let favorite = expected_page == "favorites";
    let mut url = format!("/api/tracks?limit=50&favorites={favorite}");
    filters::append_query(&mut url, "q", &conditions.query);
    filters::append_filters(&mut url, &conditions.filters);
    if let Some(value) = after {
        filters::append_query(&mut url, "cursor", &value);
    }
    let response = api::<TrackPage>("GET", &url, None, "").await;
    if *ui.generation.peek() != generation
        || *ui.page.peek() != expected_page
        || token(ui.auth) != session
    {
        return;
    }
    match response {
        Ok(page) => {
            // Commit only accepted conditions. Failed validation or a stale
            // response cannot change the conditions used by the next page.
            ui.conditions.set(conditions);
            ui.tracks.set(page.items);
            ui.cursor.set(page.next_cursor);
            ui.error.set(String::new());
            ui.filter_error.set(String::new());
        }
        Err(message) => ui.error.set(message),
    }
    ui.loading.set(false);
}

fn search_library(query: String, mut ui: LibraryUi) {
    if query.len() > 256 {
        reject_library_conditions("Shorten your search to at most 256 bytes.".into(), ui);
        return;
    }
    let mut conditions = ui.conditions.peek().clone();
    conditions.query = query;
    ui.filter_error.set(String::new());
    spawn_authorized(ui.auth, request_library(conditions, None, ui));
}

fn reject_library_conditions(message: String, mut ui: LibraryUi) {
    invalidate_library(ui);
    ui.loading.set(false);
    ui.filter_error.set(message);
}

fn apply_library_filters(filters: TrackFilters, mut ui: LibraryUi) {
    let mut conditions = ui.conditions.peek().clone();
    conditions.filters = filters;
    ui.filter_error.set(String::new());
    spawn_authorized(ui.auth, request_library(conditions, None, ui));
}

fn catalog_changed(mut revision: Signal<u64>) {
    let next = revision.peek().wrapping_add(1);
    revision.set(next);
}

fn storage_get(key: &str) -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item(key)
        .ok()?
}
fn storage_set(key: &str, value: &str) {
    if let Some(store) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = store.set_item(key, value);
    }
}
fn audio() -> Option<HtmlAudioElement> {
    web_sys::window()?
        .document()?
        .get_element_by_id("vela-audio")?
        .dyn_into()
        .ok()
}
fn queue_key(auth: Signal<Option<AuthResponse>>, suffix: &str) -> String {
    format!(
        "vela.{}.{}",
        auth.peek()
            .as_ref()
            .map(|a| a.user.id.as_str())
            .unwrap_or("signed-out"),
        suffix
    )
}
fn persist_queue(queue: &[Track], auth: Signal<Option<AuthResponse>>) {
    if let Ok(value) = serde_json::to_string(queue) {
        storage_set(&queue_key(auth, "queue"), &value);
    }
}
fn start_audio(track: &Track, mut error: Signal<String>) {
    if let Some(player) = audio() {
        player.set_src(&format!("/api/tracks/{}/stream", track.id));
        player.set_current_time(0.0);
        if let Ok(promise) = player.play() {
            spawn(async move {
                if JsFuture::from(promise).await.is_err() {
                    error.set(
                        "Playback could not start. This browser may not support this audio format."
                            .into(),
                    );
                }
            });
        }
        update_media_metadata(track);
    }
}
fn update_media_metadata(track: &Track) {
    let info = json!({"title": track.title, "artist": track.artist, "album": track.album, "artwork": track.artwork_id.as_ref().map(|id| vec![json!({"src": format!("/api/artwork/{id}")})]).unwrap_or_default()});
    if let Ok(detail) = js_sys::JSON::parse(&info.to_string()) {
        let init = web_sys::CustomEventInit::new();
        init.set_detail(&detail);
        if let Ok(event) = web_sys::CustomEvent::new_with_event_init_dict("vela-track", &init)
            && let Some(document) = web_sys::window().and_then(|w| w.document())
        {
            let _ = document.dispatch_event(&event);
        }
    }
}
fn play_queue(
    new_queue: Vec<Track>,
    at: usize,
    mut queue: Signal<Vec<Track>>,
    mut index: Signal<usize>,
    auth: Signal<Option<AuthResponse>>,
    error: Signal<String>,
) {
    if let Some(track) = new_queue.get(at) {
        start_audio(track, error);
        index.set(at);
        storage_set(&queue_key(auth, "queue_index"), &at.to_string());
    }
    persist_queue(&new_queue, auth);
    queue.set(new_queue);
}
fn advance(
    direction: i32,
    queue: Signal<Vec<Track>>,
    mut index: Signal<usize>,
    shuffle: bool,
    repeat: &str,
    auth: Signal<Option<AuthResponse>>,
    error: Signal<String>,
) {
    let length = queue.peek().len();
    if length == 0 {
        return;
    }
    if direction < 0 && audio().is_some_and(|p| p.current_time() > 3.0) {
        if let Some(p) = audio() {
            p.set_current_time(0.0);
        }
        return;
    }
    let at = if shuffle && length > 1 {
        (index.peek().saturating_add(1) + (js_sys::Math::random() * (length - 1) as f64) as usize)
            % length
    } else if direction < 0 {
        index.peek().checked_sub(1).unwrap_or(length - 1)
    } else if *index.peek() + 1 < length {
        *index.peek() + 1
    } else if repeat == "all" {
        0
    } else {
        return;
    };
    if let Some(track) = queue.peek().get(at) {
        start_audio(track, error);
        index.set(at);
        storage_set(&queue_key(auth, "queue_index"), &at.to_string());
    }
}
fn seconds(value: f64) -> String {
    let value = if value.is_finite() {
        value.max(0.0) as u64
    } else {
        0
    };
    format!("{}:{:02}", value / 60, value % 60)
}
fn bytes(value: i64) -> String {
    if value > 1_073_741_824 {
        format!("{:.1} GB", value as f64 / 1_073_741_824.0)
    } else {
        format!("{:.1} MB", value.max(0) as f64 / 1_048_576.0)
    }
}

async fn upload_files(
    csrf: String,
    mut status: Signal<String>,
    mut error: Signal<String>,
) -> Result<usize, String> {
    let input = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("audio-files"))
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
        .ok_or_else(|| "The file picker is unavailable.".to_string())?;
    let files = input
        .files()
        .ok_or_else(|| "Select an audio file first.".to_string())?;
    let length = files.length();
    if length == 0 {
        return Err("Select one or more audio files first.".into());
    }
    let mut successful = 0;
    for at in 0..length {
        let Some(file) = files.item(at) else {
            continue;
        };
        status.set(format!("Uploading {} of {length}: {}", at + 1, file.name()));
        let data = FormData::new().map_err(|_| "Could not start the upload.".to_string())?;
        data.append_with_blob_and_filename("file", &file, &file.name())
            .map_err(|_| "Could not read the selected file.".to_string())?;
        let init = web_sys::RequestInit::new();
        init.set_method("POST");
        init.set_body(&data);
        init.set_credentials(web_sys::RequestCredentials::SameOrigin);
        let request = web_sys::Request::new_with_str_and_init("/api/upload", &init)
            .map_err(|_| "Could not start the upload.".to_string())?;
        request
            .headers()
            .set("X-CSRF-Token", &csrf)
            .map_err(|_| "Could not authorize the upload.".to_string())?;
        let window = web_sys::window().ok_or_else(|| "The browser is unavailable.".to_string())?;
        let response = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|_| "The upload connection was interrupted.".to_string())?
            .dyn_into::<web_sys::Response>()
            .map_err(|_| "Could not read the upload response.".to_string())?;
        let text = JsFuture::from(
            response
                .text()
                .map_err(|_| "Could not read the upload response.".to_string())?,
        )
        .await
        .map_err(|_| "Could not read the upload response.".to_string())?
        .as_string()
        .unwrap_or_default();
        if response.ok() {
            match serde_json::from_str::<UploadResult>(&text) {
                Ok(result) => {
                    successful += 1;
                    if result.duplicate {
                        status.set(format!("{} was already in your library.", file.name()));
                    }
                }
                Err(_) => error.set(
                    "The upload succeeded, but its result could not be read. Refresh your library."
                        .into(),
                ),
            }
        } else {
            error.set(
                serde_json::from_str::<ApiError>(&text)
                    .map(|e| format!("{}: {}", file.name(), e.error))
                    .unwrap_or_else(|_| format!("{} could not be uploaded.", file.name())),
            );
        }
    }
    input.set_value("");
    Ok(successful)
}

#[component]
fn Art(track: Track, #[props(default = false)] large: bool) -> Element {
    let mut failed = use_signal(|| false);
    let class = if large {
        "artwork artwork-large"
    } else {
        "artwork"
    };
    rsx! { div { class, if let Some(id) = track.artwork_id.filter(|_| !failed()) { img { src: "/api/artwork/{id}", alt: "", loading: "lazy", onerror: move |_| failed.set(true) } } else { span { "♫" } } } }
}

// Every asynchronous UI action is bound to the session that started it.
// Browser futures resume on one thread; checking before each poll prevents
// an old response from writing private state after logout/account switching.
fn clear_media_session() {
    if let Some(window) = web_sys::window()
        && let Ok(session) = js_sys::Reflect::get(&window.navigator(), &"mediaSession".into())
        && !session.is_undefined()
        && !session.is_null()
    {
        let _ = js_sys::Reflect::set(&session, &"metadata".into(), &wasm_bindgen::JsValue::NULL);
        let _ = js_sys::Reflect::set(&session, &"playbackState".into(), &"none".into());
    }
}

fn spawn_authorized(
    auth: Signal<Option<AuthResponse>>,
    future: impl std::future::Future<Output = ()> + 'static,
) {
    let authority = auth.peek().as_ref().map(|s| s.csrf_token.clone());
    let mut future = Box::pin(future);
    spawn(std::future::poll_fn(move |context| {
        let current = auth.peek().as_ref().map(|s| s.csrf_token.clone());
        if current != authority {
            std::task::Poll::Ready(())
        } else {
            future.as_mut().poll(context)
        }
    }));
}

#[derive(Clone)]
enum CatalogSelection {
    Album(AlbumSummary),
    Artist(ArtistSummary),
}

impl CatalogSelection {
    fn id(&self) -> &str {
        match self {
            Self::Album(album) => &album.id,
            Self::Artist(artist) => &artist.id,
        }
    }
    fn view(&self) -> &str {
        match self {
            Self::Album(_) => "albums",
            Self::Artist(_) => "artists",
        }
    }
}

#[derive(Clone, Copy)]
struct CatalogUi {
    albums: Signal<Vec<AlbumSummary>>,
    artists: Signal<Vec<ArtistSummary>>,
    cursor: Signal<Option<String>>,
    loading: Signal<bool>,
    selected: Signal<Option<CatalogSelection>>,
    tracks: Signal<Vec<Track>>,
    track_cursor: Signal<Option<String>>,
    detail_loading: Signal<bool>,
    generation: Signal<u64>,
    auth: Signal<Option<AuthResponse>>,
    page: Signal<String>,
    error: Signal<String>,
}

fn invalidate_catalog(mut ui: CatalogUi) -> u64 {
    let generation = ui.generation.peek().wrapping_add(1);
    ui.generation.set(generation);
    generation
}

fn catalog_current(
    ui: CatalogUi,
    generation: u64,
    view: &str,
    authority: &str,
    selection: Option<&str>,
) -> bool {
    *ui.generation.peek() == generation
        && ui.page.peek().as_str() == view
        && ui
            .auth
            .peek()
            .as_ref()
            .is_some_and(|auth| auth.csrf_token == authority)
        && ui.selected.peek().as_ref().map(CatalogSelection::id) == selection
}

fn catalog_url(view: &str, id: Option<&str>, cursor: Option<String>) -> String {
    let mut url = id
        .map(|id| format!("/api/{view}/{id}/tracks?limit=50"))
        .unwrap_or_else(|| format!("/api/{view}?limit=50"));
    if let Some(cursor) = cursor {
        url.push_str(&format!(
            "&cursor={}",
            js_sys::encode_uri_component(&cursor)
                .as_string()
                .unwrap_or_default()
        ));
    }
    url
}

fn catalog_page(cursor: Option<String>, mut ui: CatalogUi) {
    let view = ui.page.peek().clone();
    if !matches!(view.as_str(), "albums" | "artists") {
        return;
    }
    let authority = token(ui.auth);
    if authority.is_empty() {
        return;
    }
    let generation = invalidate_catalog(ui);
    ui.selected.set(None);
    ui.tracks.set(Vec::new());
    ui.track_cursor.set(None);
    ui.detail_loading.set(false);
    ui.loading.set(true);
    let url = catalog_url(&view, None, cursor);
    spawn_authorized(ui.auth, async move {
        if view == "albums" {
            let response = api::<AlbumPage>("GET", &url, None, "").await;
            if !catalog_current(ui, generation, &view, &authority, None) {
                return;
            }
            match response {
                Ok(result) => {
                    ui.albums.set(result.items);
                    ui.cursor.set(result.next_cursor);
                }
                Err(message) => ui.error.set(message),
            }
        } else {
            let response = api::<ArtistPage>("GET", &url, None, "").await;
            if !catalog_current(ui, generation, &view, &authority, None) {
                return;
            }
            match response {
                Ok(result) => {
                    ui.artists.set(result.items);
                    ui.cursor.set(result.next_cursor);
                }
                Err(message) => ui.error.set(message),
            }
        }
        ui.loading.set(false);
    });
}

fn catalog_tracks(cursor: Option<String>, mut ui: CatalogUi) {
    let Some(selection) = ui.selected.peek().clone() else {
        return;
    };
    let view = selection.view().to_string();
    if ui.page.peek().as_str() != view {
        return;
    }
    let authority = token(ui.auth);
    if authority.is_empty() {
        return;
    }
    let generation = invalidate_catalog(ui);
    ui.tracks.set(Vec::new());
    ui.track_cursor.set(None);
    ui.detail_loading.set(true);
    let url = catalog_url(&view, Some(selection.id()), cursor);
    spawn_authorized(ui.auth, async move {
        let response = api::<TrackPage>("GET", &url, None, "").await;
        if !catalog_current(ui, generation, &view, &authority, Some(selection.id())) {
            return;
        }
        match response {
            Ok(result) => {
                ui.tracks.set(result.items);
                ui.track_cursor.set(result.next_cursor);
            }
            Err(message) => ui.error.set(message),
        }
        ui.detail_loading.set(false);
    });
}

fn open_catalog(selection: CatalogSelection, mut ui: CatalogUi) {
    ui.selected.set(Some(selection));
    ui.loading.set(false);
    catalog_tracks(None, ui);
}

fn clear_catalog(mut ui: CatalogUi) {
    invalidate_catalog(ui);
    ui.albums.set(Vec::new());
    ui.artists.set(Vec::new());
    ui.cursor.set(None);
    ui.selected.set(None);
    ui.tracks.set(Vec::new());
    ui.track_cursor.set(None);
    ui.loading.set(false);
    ui.detail_loading.set(false);
}

#[component]
fn CatalogCover(artwork_id: Option<String>, #[props(default = false)] artist: bool) -> Element {
    let mut failed = use_signal(|| false);
    rsx! { div { class: if artist { "catalog-cover artist-cover" } else { "catalog-cover" }, if let Some(id) = artwork_id.filter(|_| !failed()) { img { src: "/api/artwork/{id}", alt: "", loading: "lazy", onerror: move |_| failed.set(true) } } else { span { if artist { "◌" } else { "♫" } } } } }
}

#[component]
fn GroupTracks(
    tracks: Vec<Track>,
    current_id: Option<String>,
    playing: bool,
    onplay: EventHandler<usize>,
    onfavorite: EventHandler<Track>,
    onqueue: EventHandler<Track>,
    onedit: EventHandler<Track>,
) -> Element {
    rsx! { div { class: "track-table group-track-table", div { class: "track-header", span { "#" } span { "TITLE" } span { "ALBUM" } span { "QUALITY" } span { "TIME" } span { "" } }
        for (at, track) in tracks.iter().cloned().enumerate() {
            { let favorite = track.clone(); let queued = track.clone(); let edited = track.clone();
            rsx! { div { key: "{track.id}", class: if current_id.as_ref().is_some_and(|id| *id == track.id) { "track-row current" } else { "track-row" },
                button { class: "track-play", "aria-label": "Play {track.title}", onclick: move |_| onplay.call(at), if current_id.as_ref().is_some_and(|id| *id == track.id) && playing { "♫" } else { "▶" } }
                button { class: "track-title", onclick: move |_| onplay.call(at), Art { track: track.clone() } div { strong { "{track.title}" } small { "{track.artist}" } } }
                span { class: "track-album", "{track.album}" } span { class: "quality-tag", "{track.codec}" } span { class: "track-duration", "{seconds(track.duration_ms as f64 / 1000.0)}" }
                div { class: "track-actions", button { class: if track.favorite { "icon-button favorite selected" } else { "icon-button favorite" }, "aria-label": if track.favorite { "Remove favorite" } else { "Favorite track" }, "aria-pressed": track.favorite, onclick: move |_| onfavorite.call(favorite.clone()), if track.favorite { "♥" } else { "♡" } } button { class: "icon-button", "aria-label": "Add {track.title} to playback queue", onclick: move |_| onqueue.call(queued.clone()), "+" } button { class: "icon-button metadata-edit", "aria-label": "Edit metadata for {track.title}", title: "Edit metadata", onclick: move |_| onedit.call(edited.clone()), "✎" } }
            } } }
        }
    } }
}

#[allow(non_snake_case)]
fn App() -> Element {
    let mut auth = use_signal(|| None::<AuthResponse>);
    let mut booting = use_signal(|| true);
    let mut needs_setup = use_signal(|| false);
    let mut version = use_signal(String::new);
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(String::new);
    let mut notice = use_signal(String::new);
    let mut page = use_signal(|| "library".to_string());
    let mut query = use_signal(String::new);
    let mut tracks = use_signal(Vec::<Track>::new);
    let cursor = use_signal(|| None::<String>);
    let loading = use_signal(|| false);
    let request_generation = use_signal(|| 0_u64);
    let mut library_conditions = use_signal(LibraryConditions::default);
    let mut filter_draft = use_signal(filters::FilterDraft::default);
    let mut filter_open = use_signal(|| false);
    let mut filter_error = use_signal(String::new);
    let facets_revision = use_signal(|| 0_u64);
    let library = LibraryUi {
        tracks,
        cursor,
        loading,
        error,
        generation: request_generation,
        conditions: library_conditions,
        filter_error,
        auth,
        page,
    };
    let catalog = CatalogUi {
        albums: use_signal(Vec::<AlbumSummary>::new),
        artists: use_signal(Vec::<ArtistSummary>::new),
        cursor: use_signal(|| None::<String>),
        loading: use_signal(|| false),
        selected: use_signal(|| None::<CatalogSelection>),
        tracks: use_signal(Vec::<Track>::new),
        track_cursor: use_signal(|| None::<String>),
        detail_loading: use_signal(|| false),
        generation: use_signal(|| 0_u64),
        auth,
        page,
        error,
    };
    let mut stats = use_signal(|| None::<LibraryStats>);
    let mut add_music = use_signal(|| false);
    let mut editing_track = use_signal(|| None::<Track>);
    let mut uploading = use_signal(|| false);
    let mut upload_status = use_signal(String::new);
    let mut import_url = use_signal(String::new);
    let mut preview = use_signal(|| None::<ImportPreview>);
    let mut import_busy = use_signal(|| false);
    let mut jobs = use_signal(Vec::<ImportJob>::new);
    let mut playlists = use_signal(Vec::<PlaylistSummary>::new);
    let mut selected_playlist = use_signal(|| None::<PlaylistDetail>);
    let mut playlist_name = use_signal(String::new);
    let mut collaborator = use_signal(String::new);
    let mut collaborator_list = use_signal(Vec::<PlaylistCollaborator>::new);
    let mut collaborator_revision = use_signal(|| 0_u64);
    let mut share_url = use_signal(String::new);
    let mut public_share = use_signal(|| None::<PlaylistDetail>);
    let mut share_days = use_signal(|| "7".to_string());
    let mut collaborator_edit = use_signal(|| false);
    let mut selected_add_playlist = use_signal(String::new);
    let mut new_user = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut queue = use_signal(Vec::<Track>::new);
    let mut index = use_signal(|| 0);
    let mut playing = use_signal(|| false);
    let mut position = use_signal(|| 0.0);
    let mut duration = use_signal(|| 0.0);
    let mut volume = use_signal(|| 0.8);
    let mut shuffle = use_signal(|| false);
    let mut repeat = use_signal(|| "off".to_string());
    let mut show_queue = use_signal(|| false);
    let mut theme = use_signal(|| storage_get("vela.theme").unwrap_or_else(|| "dark".into()));

    use_effect(move || {
        spawn_authorized(auth, async move {
            let share_token = web_sys::window()
                .and_then(|w| w.location().hash().ok())
                .and_then(|search| {
                    search
                        .trim_start_matches('#')
                        .split('&')
                        .find_map(|part| part.strip_prefix("share=").map(str::to_string))
                });
            if let Some(share_token) = share_token.filter(|t| {
                t.len() <= 256
                    && t.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            }) {
                match api::<PlaylistDetail>("GET", &format!("/api/shares/{share_token}"), None, "")
                    .await
                {
                    Ok(value) => {
                        public_share.set(Some(value.clone()));
                        selected_playlist.set(Some(value));
                        page.set("playlists".into());
                    }
                    Err(message) => error.set(message),
                }
            }
            match api::<SetupStatus>("GET", "/api/setup", None, "").await {
                Ok(status) => {
                    needs_setup.set(status.needs_setup);
                    version.set(status.version);
                    if !status.needs_setup
                        && let Ok(session) = api::<AuthResponse>("GET", "/api/me", None, "").await
                    {
                        auth.set(Some(session));
                    }
                }
                Err(message) => error.set(message),
            }
            booting.set(false);
        });
    });
    use_effect(move || {
        let selected = selected_playlist
            .read()
            .as_ref()
            .map(|d| (d.playlist.id.clone(), d.playlist.owner_id.clone()));
        let user = auth.read().as_ref().map(|a| a.user.id.clone());
        let _revision = collaborator_revision();
        collaborator_list.set(Vec::new());
        if let (Some((id, owner)), Some(user)) = (selected, user)
            && owner == user
        {
            spawn_authorized(auth, async move {
                if let Ok(list) = api::<Vec<PlaylistCollaborator>>(
                    "GET",
                    &format!("/api/playlists/{id}/collaborators"),
                    None,
                    "",
                )
                .await
                    && auth.peek().as_ref().is_some_and(|a| a.user.id == user)
                    && selected_playlist
                        .peek()
                        .as_ref()
                        .is_some_and(|p| p.playlist.id == id)
                {
                    collaborator_list.set(list);
                }
            });
        }
    });
    use_effect(move || {
        let _session = auth.read().as_ref().map(|s| s.csrf_token.clone());
        editing_track.set(None);
        invalidate_library(library);
        tracks.set(Vec::new());
        let mut cursor = cursor;
        cursor.set(None);
        let mut loading = loading;
        loading.set(false);
        library_conditions.set(LibraryConditions::default());
        query.set(String::new());
        filter_draft.set(filters::FilterDraft::default());
        filter_error.set(String::new());
        filter_open.set(false);
        clear_catalog(catalog);
        busy.set(false);
        import_busy.set(false);
        uploading.set(false);
        add_music.set(false);
        preview.set(None);
        import_url.set(String::new());
        upload_status.set(String::new());
        new_user.set(String::new());
        new_password.set(String::new());
        collaborator.set(String::new());
        selected_add_playlist.set(String::new());
        show_queue.set(false);
        if auth.peek().is_none() {
            return;
        }
        let restored = storage_get(&queue_key(auth, "queue"))
            .and_then(|value| serde_json::from_str::<Vec<Track>>(&value).ok())
            .unwrap_or_default();
        let at = storage_get(&queue_key(auth, "queue_index"))
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|at| *at < restored.len())
            .unwrap_or(0);
        queue.set(restored);
        index.set(at);
        playing.set(false);
        position.set(0.0);
        duration.set(0.0);
    });
    use_effect(move || {
        let Some(session) = auth.read().clone() else {
            return;
        };
        let active = page.read().clone();
        invalidate_library(library);
        invalidate_catalog(catalog);
        if active == "albums" || active == "artists" {
            catalog_page(None, catalog);
        }
        if active == "library" || active == "favorites" {
            spawn_authorized(auth, load_library(None, library));
        }
        spawn_authorized(auth, async move {
            if let Ok(value) = api::<LibraryStats>("GET", "/api/stats", None, "").await {
                if auth.peek().as_ref().map(|a| &a.user.id) != Some(&session.user.id) {
                    return;
                }
                stats.set(Some(value));
            }
            if let Ok(value) = api::<Vec<PlaylistSummary>>("GET", "/api/playlists", None, "").await
            {
                if auth.peek().as_ref().map(|a| &a.user.id) != Some(&session.user.id) {
                    return;
                }
                playlists.set(value);
            }
            if active == "imports"
                && let Ok(value) = api::<Vec<ImportJob>>("GET", "/api/imports", None, "").await
            {
                if auth.peek().as_ref().map(|a| &a.user.id) != Some(&session.user.id) {
                    return;
                }
                jobs.set(value);
            }
        });
    });
    use_effect(move || {
        let Some(session) = auth.read().clone() else {
            return;
        };
        spawn_authorized(auth, async move {
            if let Ok(value) = api::<Vec<ImportJob>>("GET", "/api/imports", None, "").await {
                if auth.peek().as_ref().map(|a| &a.user.id) != Some(&session.user.id) {
                    return;
                }
                jobs.set(value);
            }
            loop {
                TimeoutFuture::new(2_000).await;
                if auth.peek().as_ref().map(|a| &a.user.id) != Some(&session.user.id) {
                    break;
                }
                if jobs.peek().iter().any(|j| {
                    matches!(
                        j.status.as_str(),
                        "queued" | "running" | "downloading" | "processing"
                    )
                }) && let Ok(value) =
                    api::<Vec<ImportJob>>("GET", "/api/imports", None, "").await
                {
                    if auth.peek().as_ref().map(|a| &a.user.id) != Some(&session.user.id) {
                        break;
                    }
                    let completed = value.iter().any(|j| {
                        j.status == "completed"
                            && jobs
                                .peek()
                                .iter()
                                .any(|old| old.id == j.id && old.status != "completed")
                    });
                    jobs.set(value);
                    if completed {
                        catalog_changed(facets_revision);
                        notice.set("Your import is ready in the library.".into());
                        if matches!(page.peek().as_str(), "albums" | "artists") {
                            catalog_page(None, catalog);
                        }
                        if let Ok(value) = api::<LibraryStats>("GET", "/api/stats", None, "").await
                        {
                            stats.set(Some(value));
                        }
                        if matches!(page.peek().as_str(), "library" | "favorites") {
                            load_library(None, library).await;
                        }
                    }
                }
            }
        });
    });
    use_effect(move || {
        if auth.read().is_some() {
            let at = *index.read();
            storage_set(&queue_key(auth, "queue_index"), &at.to_string());
        }
    });
    use_effect(move || {
        let value = theme.read().clone();
        storage_set("vela.theme", &value);
        if let Some(document) = web_sys::window().and_then(|w| w.document())
            && let Some(root) = document.document_element()
        {
            let _ = root.set_attribute("data-theme", &value);
        }
    });

    let current = queue.read().get(*index.read()).cloned();
    let heading = match page.read().as_str() {
        "albums" => "Let the whole record play",
        "artists" => "The voices in your collection",
        "favorites" => "Made for repeat",
        "imports" => "Bring your music home",
        "playlists" => "Every mood, a collection",
        "settings" => "Your listening space",
        _ => "Your music. Your space.",
    };

    rsx! {
        if booting() { div { class: "boot-screen", div { class: "brand-mark", "V" } p { "Opening your music space…" } } }
        else if auth.read().is_none() {
            main { class: "auth-shell",
                section { class: "auth-story", div { class: "brand", img { src: "/vela.svg", alt: "" } "vela" }
                    span { class: "eyebrow", "A LITTLE CLOSER TO THE MUSIC" }
                    h1 { "A home for" br {} "your sound." }
                    p { "Keep the music you love. Listen on your terms. Make room for the next discovery." }
                    div { class: "vinyl", div { class: "vinyl-label", span { "vela" } small { "SIDE A · YOUR COLLECTION" } } }
                    div { class: "auth-footer", "Private by default · Yours by design" }
                }
                section { class: "auth-card",
                    if let Some(shared) = public_share.read().clone() { div { class: "public-share", span { class: "eyebrow", "A PLAYLIST SHARED WITH YOU" } h2 { "{shared.playlist.name}" } p { "{shared.items.len()} tracks · View-only link" }
                            div { class: "public-share-tracks", for item in shared.items { div { class: "playlist-track", Art { track: item.track.clone() } div { strong { "{item.track.title}" } small { "{item.track.artist}" } } } } }
                            p { "Sign in to listen to tracks available in your own library. This link shares the playlist’s titles, not its audio files." }
                        } }
                    span { class: "eyebrow", if needs_setup() { "WELCOME TO VELA" } else { "GOOD TO HAVE YOU BACK" } }
                    h2 { if needs_setup() { "Make yourself at home" } else { "Your next listen awaits" } }
                    p { if needs_setup() { "Create the administrator account for this server. Your library starts private." } else { "Sign in to your private music library." } }
                    if !error().is_empty() { div { class: "alert error", role: "alert", "{error}" } }
                    form { onsubmit: move |event| { event.prevent_default(); if busy() { return; } busy.set(true); error.set(String::new()); spawn_authorized(auth, async move {
                        let path = if needs_setup() { "/api/setup" } else { "/api/login" };
                        match api::<AuthResponse>("POST", path, Some(json!({"username": username(), "password": password()})), "").await {
                            Ok(session) => { auth.set(Some(session)); needs_setup.set(false); password.set(String::new()); }
                            Err(message) => error.set(message),
                        }
                        busy.set(false);
                    }); },
                        label { r#for: "username", "Username" }
                        input { id: "username", value: "{username}", autocomplete: "username", required: true, maxlength: 64, autofocus: true, oninput: move |e| username.set(e.value()) }
                        label { r#for: "password", "Password" }
                        input { id: "password", r#type: "password", value: "{password}", autocomplete: if needs_setup() { "new-password" } else { "current-password" }, required: true, minlength: if needs_setup() { 12 } else { 1 }, oninput: move |e| password.set(e.value()) }
                        if needs_setup() { small { "Use at least 12 characters. Store this password somewhere safe." } }
                        button { class: "button primary", r#type: "submit", disabled: busy(), if busy() { "Opening your space…" } else if needs_setup() { "Create your music space →" } else { "Sign in →" } }
                    }
                    small { class: "version", "Vela {version} · Self-hosted music, beautifully simple." }
                }
            }
        } else {
            div { class: "app-shell",
                aside { class: "sidebar",
                    a { class: "brand", href: "#", onclick: move |e| { e.prevent_default(); page.set("library".into()); }, img { src: "/vela.svg", alt: "" } "vela" }
                    span { class: "nav-label", "YOUR SPACE" }
                    nav { "aria-label": "Main navigation",
                        for (id, symbol, name) in [("library", "◈", "Library"), ("albums", "▣", "Albums"), ("artists", "◌", "Artists"), ("favorites", "♡", "Favorites"), ("playlists", "≡", "Playlists"), ("imports", "↧", "Add music")] {
                            button { class: if page() == id { "nav-item active" } else { "nav-item" }, "aria-label": name, "aria-current": if page() == id { "page" } else { "false" }, onclick: move |_| { page.set(id.into()); selected_playlist.set(None); }, span { class: "nav-icon", "{symbol}" } "{name}" }
                        }
                    }
                    div { class: "sidebar-collection", span { class: "nav-label", "YOUR COLLECTION" }
                        if let Some(value) = stats.read().as_ref() { strong { "{value.tracks}" } span { "tracks, all yours" } }
                        p { "Good music deserves a place to stay." }
                    }
                    div { class: "sidebar-bottom",
                        button { class: if page() == "settings" { "nav-item active" } else { "nav-item" }, onclick: move |_| page.set("settings".into()), span { class: "nav-icon", "⚙" } "Settings" }
                        div { class: "account", div { class: "avatar", "{auth.read().as_ref().map(|a| a.user.username.chars().next().unwrap_or('V').to_ascii_uppercase()).unwrap_or('V')}" }
                            div { strong { "{auth.read().as_ref().map(|a| a.user.username.clone()).unwrap_or_default()}" } small { "Private library" } }
                        }
                    }
                }
                main { class: "main-content",
                    header { class: "topbar", div { class: "breadcrumb", "YOUR PERSONAL MUSIC CLOUD" }
                        div { class: "topbar-actions", button { class: "icon-button theme-button", title: "Change color theme", "aria-label": "Change color theme", onclick: move |_| { theme.set(if theme() == "dark" { "light".into() } else { "dark".into() }); }, if theme() == "dark" { "☀" } else { "☾" } }
                            button { class: "button primary small", onclick: move |_| { add_music.set(true); }, "+ Add music" }
                        }
                    }
                    if !error().is_empty() { div { class: "alert error", role: "alert", span { "{error}" } button { "aria-label": "Dismiss error", onclick: move |_| error.set(String::new()), "×" } } }
                    if !notice().is_empty() { div { class: "alert success", role: "status", span { "{notice}" } button { "aria-label": "Dismiss notification", onclick: move |_| notice.set(String::new()), "×" } } }
                    div { class: "page-heading", span { class: "eyebrow", "LISTEN A LITTLE LONGER" } h1 { "{heading}" } }

                    if page() == "library" || page() == "favorites" {
                        if page() == "library" {
                            section { class: "library-banner", div { span { class: "eyebrow", "BUILT AROUND YOU" } h2 { "A collection worth" br {} "getting lost in." } p { "Every album, every favorite. No noise, just your music." }
                                if !tracks.read().is_empty() { button { class: "button cream", onclick: move |_| play_queue(tracks.peek().clone(), 0, queue, index, auth, error), "▶ Play these tracks" } }
                                else { button { class: "button cream", onclick: move |_| add_music.set(true), "+ Add your first album" } }
                            } div { class: "banner-record", div { "v" } }
                            div { class: "banner-caption", "OWN YOUR SOUND" }
                            }
                        }
                        section { class: "library-section",
                            div { class: "section-toolbar", div { h2 { if page() == "favorites" { "Your favorites" } else { "In your collection" } }
                                    if let Some(value) = stats.read().as_ref() { p { "{value.tracks} tracks · {value.albums} albums · {value.artists} artists" } }
                                }
                                form { class: "search-form", onsubmit: move |e| { e.prevent_default(); search_library(query(), library); },
                                    label { class: "sr-only", r#for: "library-search", "Search your library" }
                                    input { id: "library-search", value: "{query}", maxlength: 256, placeholder: "Search your music", oninput: move |e| query.set(e.value()) }
                                    button { r#type: "submit", "aria-label": "Search", "⌕" }
                                }
                            }
                            filters::LibraryFilters {
                                auth, open: filter_open, draft: filter_draft,
                                applied: library_conditions.read().filters.clone(),
                                error: filter_error, loading, revision: facets_revision,
                                onapply: move |filters| apply_library_filters(filters, library),
                                oninvalid: move |message| reject_library_conditions(message, library),
                                onreset: move |_| {
                                    filter_draft.set(filters::FilterDraft::default());
                                    apply_library_filters(TrackFilters::default(), library);
                                },
                            }
                            if !library_conditions.read().query.is_empty() {
                                p { class: "applied-search", role: "status", "Results for “{library_conditions.read().query}”"
                                    button { class: "button secondary small", onclick: move |_| { query.set(String::new()); search_library(String::new(), library); }, "Clear search" }
                                }
                            }
                            if !playlists.read().is_empty() { div { class: "playlist-target", label { r#for: "playlist-target", "Add tracks to" } select { id: "playlist-target", value: "{selected_add_playlist}", onchange: move |e| selected_add_playlist.set(e.value()), option { value: "", "Choose a playlist" } for list in playlists.read().iter().filter(|p| p.can_edit) { option { value: "{list.id}", "{list.name}" } } } } }
                            if loading() && tracks.read().is_empty() { div { class: "empty-state", div { class: "spinner" } h3 { "Finding your music…" } } }
                            else if tracks.read().is_empty() { div { class: "empty-state", span { class: "empty-icon", if page() == "favorites" { "♡" } else { "♫" } }
                                    h3 { if !library_conditions.read().query.is_empty() || filters::has_criteria(&library_conditions.read().filters) { "No matching tracks" } else if page() == "favorites" { "Keep the good ones close" } else { "Your collection starts here" } }
                                    p { if filters::has_criteria(&library_conditions.read().filters) { "No tracks match your applied filters. Broaden the filters or reset them to see more of your music." } else if !library_conditions.read().query.is_empty() { "Try another title, artist, or album, or clear your search." } else if page() == "favorites" { "Tap the heart beside a track to save it here." } else { "Upload your audio files or import a downloadable music link." } }
                                    if library_conditions.read().filters != TrackFilters::default() { button { class: "button secondary", onclick: move |_| { filter_draft.set(filters::FilterDraft::default()); apply_library_filters(TrackFilters::default(), library); }, "Reset applied filters" } }
                                    if page() == "library" && library_conditions.read().query.is_empty() && library_conditions.read().filters == TrackFilters::default() { button { class: "button primary", onclick: move |_| add_music.set(true), "+ Add music" } }
                                } }
                            else { div { class: "track-table", div { class: "track-header", span { "#" } span { "TITLE" } span { "ALBUM" } span { "QUALITY" } span { "TIME" } span { "" } }
                                    for (at, track) in tracks.read().iter().cloned().enumerate() {
                                        { let row_track = track.clone(); let favorite_track = track.clone(); let add_track = track.clone(); let edit_track = track.clone();
                                        rsx! { div { key: "{track.id}", class: if current.as_ref().is_some_and(|t| t.id == track.id) { "track-row current" } else { "track-row" },
                                            button { class: "track-play", "aria-label": "Play {track.title}", onclick: move |_| play_queue(tracks.peek().clone(), at, queue, index, auth, error), if current.as_ref().is_some_and(|t| t.id == track.id) && playing() { "♫" } else { "▶" } }
                                            button { class: "track-title", onclick: move |_| play_queue(tracks.peek().clone(), at, queue, index, auth, error), Art { track: row_track } div { strong { "{track.title}" } small { "{track.artist}" } } }
                                            span { class: "track-album", "{track.album}" }
                                            span { class: "quality-tag", "{track.codec}" }
                                            span { class: "track-duration", "{seconds(track.duration_ms as f64 / 1000.0)}" }
                                            div { class: "track-actions", button { class: if track.favorite { "icon-button favorite selected" } else { "icon-button favorite" }, title: if track.favorite { "Remove favorite" } else { "Favorite track" }, "aria-label": if track.favorite { "Remove favorite" } else { "Favorite track" }, "aria-pressed": track.favorite, onclick: move |_| { let track = favorite_track.clone(); spawn_authorized(auth, async move { match mutate("POST", &format!("/api/tracks/{}/favorite", track.id), Some(json!({"favorite": !track.favorite})), &token(auth)).await { Ok(()) => { if let Some(value) = tracks.write().iter_mut().find(|t| t.id == track.id) { value.favorite = !track.favorite; } } Err(message) => error.set(message) } }); }, if track.favorite { "♥" } else { "♡" } }
                                                button { class: "icon-button", title: "Add to playback queue", "aria-label": "Add {track.title} to playback queue", onclick: move |_| { queue.write().push(track.clone()); persist_queue(&queue.peek(), auth); notice.set("Added to your playback queue.".into()); }, "+" }
                                                button { class: "icon-button metadata-edit", "aria-label": "Edit metadata for {edit_track.title}", title: "Edit metadata", onclick: move |_| editing_track.set(Some(edit_track.clone())), "✎" }
                                                if !selected_add_playlist().is_empty() { button { class: "icon-button", title: "Add to selected playlist", "aria-label": "Add to selected playlist", onclick: move |_| { let track = add_track.clone(); let id = selected_add_playlist(); spawn_authorized(auth, async move {
                                                    match api::<PlaylistDetail>("GET", &format!("/api/playlists/{id}"), None, "").await {
                                                        Ok(detail) => { match mutate("POST", &format!("/api/playlists/{id}/items"), Some(json!({"revision": detail.playlist.revision, "mutation_id": uuid::Uuid::new_v4().to_string(), "track_id": track.id})), &token(auth)).await { Ok(()) => notice.set("Track added to your playlist.".into()), Err(message) => error.set(message) } }
                                                        Err(message) => error.set(message)
                                                    }
                                                }); }, "≡" } }
                                            }
                                        } } }
                                    }
                                } }
                            if !tracks.read().is_empty() { div { class: "pagination", button { class: "button secondary small", disabled: loading(), onclick: move |_| { spawn_authorized(auth, load_library(None, library)); }, "Back to first page" } span { "Up to 50 tracks per page" } if cursor.read().is_some() { button { class: "button secondary small", disabled: loading(), onclick: move |_| { spawn_authorized(auth, load_library(cursor(), library)); }, if loading() { "Loading…" } else { "Next page →" } } } } }
                        }
                    }

                    if page() == "albums" || page() == "artists" {
                        if let Some(selection) = catalog.selected.read().clone() {
                            section { class: "catalog-detail",
                                button { class: "button secondary small catalog-back", onclick: move |_| catalog_page(None, catalog), if page() == "albums" { "All albums" } else { "All artists" } }
                                div { class: "catalog-detail-header",
                                    { match selection {
                                        CatalogSelection::Album(album) => rsx! {
                                            CatalogCover { artwork_id: album.artwork_id.clone() }
                                            div { class: "catalog-detail-info", span { class: "eyebrow", "ALBUM" } h2 { "{album.title}" } p { class: "catalog-credit", "{album.artist}" }
                                                div { class: "catalog-facts", span { "{album.track_count} tracks" } if let Some(year) = album.year { span { "{year}" } } span { "{seconds(album.duration_ms as f64 / 1000.0)} total" } }
                                                button { class: "button primary", disabled: *catalog.detail_loading.read() || catalog.tracks.read().is_empty(), onclick: move |_| play_queue(catalog.tracks.peek().clone(), 0, queue, index, auth, error), "▶ Play these tracks" }
                                            }
                                        },
                                        CatalogSelection::Artist(artist) => rsx! {
                                            CatalogCover { artwork_id: artist.artwork_id.clone(), artist: true }
                                            div { class: "catalog-detail-info", span { class: "eyebrow", "ARTIST" } h2 { "{artist.name}" } p { class: "catalog-credit", "In your private collection" }
                                                div { class: "catalog-facts", span { "{artist.track_count} tracks" } span { "{artist.album_count} albums" } }
                                                button { class: "button primary", disabled: *catalog.detail_loading.read() || catalog.tracks.read().is_empty(), onclick: move |_| play_queue(catalog.tracks.peek().clone(), 0, queue, index, auth, error), "▶ Play these tracks" }
                                            }
                                        }
                                    } }
                                }
                                div { class: "section-toolbar", div { h2 { "Tracks" } p { "Listen to the tracks on this page, or add your favorites to the queue." } } }
                                if *catalog.detail_loading.read() { div { class: "empty-state", div { class: "spinner" } h3 { "Opening the collection…" } } }
                                else if catalog.tracks.read().is_empty() { div { class: "empty-state compact", h3 { "No tracks on this page" } p { "Refresh the collection to check what is available." } button { class: "button secondary", onclick: move |_| catalog_tracks(None, catalog), "Refresh tracks" } } }
                                else {
                                    GroupTracks {
                                        tracks: catalog.tracks.read().clone(), current_id: current.as_ref().map(|track| track.id.clone()), playing: playing(),
                                        onplay: move |at| play_queue(catalog.tracks.peek().clone(), at, queue, index, auth, error),
                                        onfavorite: move |track: Track| {
                                            let mut catalog = catalog;
                                            spawn_authorized(auth, async move {
                                                match mutate("POST", &format!("/api/tracks/{}/favorite", track.id), Some(json!({"favorite": !track.favorite})), &token(auth)).await {
                                                    Ok(()) => {
                                                        if let Some(value) = catalog.tracks.write().iter_mut().find(|value| value.id == track.id) { value.favorite = !track.favorite; }
                                                        if let Some(value) = tracks.write().iter_mut().find(|value| value.id == track.id) { value.favorite = !track.favorite; }
                                                    }
                                                    Err(message) => error.set(message),
                                                }
                                            });
                                        },
                                        onqueue: move |track: Track| { queue.write().push(track); persist_queue(&queue.peek(), auth); notice.set("Added to your playback queue.".into()); },
                                        onedit: move |track: Track| editing_track.set(Some(track)),
                                    }
                                    div { class: "pagination", button { class: "button secondary small", onclick: move |_| catalog_tracks(None, catalog), "Back to first page" } span { "Up to 50 tracks per page" } if catalog.track_cursor.read().is_some() { button { class: "button secondary small", onclick: move |_| catalog_tracks(catalog.track_cursor.peek().clone(), catalog), "Next track page →" } } }
                                }
                            }
                        } else {
                            section { class: "catalog-section",
                                div { class: "section-toolbar", div { h2 { if page() == "albums" { "Albums in your collection" } else { "Artists in your collection" } } p { if page() == "albums" { "Every record, from the opening track to the last." } else { "Find another side of an artist you love." } } } button { class: "button secondary small", disabled: *catalog.loading.read(), onclick: move |_| catalog_page(None, catalog), "Refresh" } }
                                if *catalog.loading.read() { div { class: "empty-state", div { class: "spinner" } h3 { "Finding your collection…" } } }
                                else if page() == "albums" && catalog.albums.read().is_empty() { div { class: "empty-state", span { class: "empty-icon", "▣" } h3 { "A place for every record" } p { "Albums appear when your audio files contain album tags. All your tracks are available in Library." } button { class: "button primary", onclick: move |_| add_music.set(true), "+ Add music" } } }
                                else if page() == "artists" && catalog.artists.read().is_empty() { div { class: "empty-state", span { class: "empty-icon", "◌" } h3 { "Get to know your collection" } p { "Artists appear from the credits in your music. Add audio files to start exploring." } button { class: "button primary", onclick: move |_| add_music.set(true), "+ Add music" } } }
                                else {
                                    div { class: if page() == "artists" { "catalog-grid artists-grid" } else { "catalog-grid" },
                                        if page() == "albums" {
                                            for album in catalog.albums.read().iter().cloned() {
                                                { let selection = album.clone(); rsx! { button { class: "catalog-card", key: "{album.id}", "aria-label": "Open album {album.title} by {album.artist}", onclick: move |_| open_catalog(CatalogSelection::Album(selection.clone()), catalog),
                                                    CatalogCover { artwork_id: album.artwork_id.clone() } h3 { "{album.title}" } p { "{album.artist}" } div { class: "catalog-card-meta", span { "{album.track_count} tracks" } if let Some(year) = album.year { span { "{year}" } } }
                                                } } }
                                            }
                                        } else {
                                            for artist in catalog.artists.read().iter().cloned() {
                                                { let selection = artist.clone(); rsx! { button { class: "catalog-card artist-card", key: "{artist.id}", "aria-label": "Open artist {artist.name}", onclick: move |_| open_catalog(CatalogSelection::Artist(selection.clone()), catalog),
                                                    CatalogCover { artwork_id: artist.artwork_id.clone(), artist: true } h3 { "{artist.name}" } p { "{artist.track_count} tracks · {artist.album_count} albums" }
                                                } } }
                                            }
                                        }
                                    }
                                    div { class: "pagination", button { class: "button secondary small", onclick: move |_| catalog_page(None, catalog), "Back to first page" } span { if page() == "albums" { "Up to 50 albums per page" } else { "Up to 50 artists per page" } } if catalog.cursor.read().is_some() { button { class: "button secondary small", onclick: move |_| catalog_page(catalog.cursor.peek().clone(), catalog), "Next page →" } } }
                                }
                            }
                        }
                    }

                    if page() == "imports" {
                        section { class: "import-layout", div { class: "panel import-card", span { class: "eyebrow", "FROM A DOWNLOADABLE LINK" } h2 { "One link. A new discovery." } p { "Import audio from a direct HTTP or HTTPS download. Your server checks the file and adds it to your private library." }
                            form { onsubmit: move |e| { e.prevent_default(); if import_busy() { return; } import_busy.set(true); preview.set(None); error.set(String::new()); spawn_authorized(auth, async move { match api::<ImportPreview>("POST", "/api/imports/preview", Some(json!({"url": import_url()})), &token(auth)).await { Ok(value) => preview.set(Some(value)), Err(message) => error.set(message) } import_busy.set(false); }); },
                                label { r#for: "import-url", "Audio download URL" } input { id: "import-url", r#type: "url", required: true, placeholder: "https://example.org/music/track.flac", value: "{import_url}", oninput: move |e| { import_url.set(e.value()); preview.set(None); } }
                                button { class: "button primary", r#type: "submit", disabled: import_busy(), if import_busy() { "Checking the link…" } else { "Preview download →" } }
                            }
                            if let Some(value) = preview.read().clone() { div { class: "import-preview", span { class: "preview-icon", "↧" } h3 { "{value.filename}" } p { "{value.source}" } div { class: "metadata-pills", span { {value.content_type.as_deref().unwrap_or("Audio file")} } span { {value.size_bytes.map(bytes).unwrap_or_else(|| "Size unknown".into())} } } small { "Destination: your private library" }
                                    button { class: "button primary", disabled: import_busy(), onclick: move |_| { let url = value.url.clone(); import_busy.set(true); spawn_authorized(auth, async move { match api::<ImportJob>("POST", "/api/imports", Some(json!({"url": url})), &token(auth)).await { Ok(job) => { jobs.write().insert(0, job); preview.set(None); import_url.set(String::new()); notice.set("Import queued. You can keep listening while it runs.".into()); } Err(message) => error.set(message) } import_busy.set(false); }); }, "Import to my library" }
                                } }
                            small { class: "rights-note", "Use files you own or have permission to download. Commercial streaming links and protected content are not supported." }
                        }
                        div { class: "panel upload-card", span { class: "eyebrow", "FROM YOUR DEVICE" } h2 { "Your files, right here." } p { "Upload FLAC, MP3, AAC, Opus, Vorbis and other supported audio files. Embedded tags and artwork come along." } button { class: "button secondary", onclick: move |_| add_music.set(true), "Choose audio files" } small { "Multiple files are processed one at a time." } }
                        }
                        section { class: "jobs-section", div { class: "section-toolbar", div { h2 { "Import queue" } p { "Follow every download from link to library." } } button { class: "button secondary small", onclick: move |_| { spawn_authorized(auth, async move { match api::<Vec<ImportJob>>("GET", "/api/imports", None, "").await { Ok(value) => jobs.set(value), Err(message) => error.set(message) } }); }, "Refresh" } }
                            if jobs.read().is_empty() { div { class: "empty-state compact", h3 { "Nothing in the queue" } p { "Your downloads will appear here." } } }
                            for job in jobs.read().iter().cloned() {
                                { let cancel_id = job.id.clone(); let retry_id = job.id.clone(); let label = match job.status.as_str() { "queued" => "Queued", "running" | "downloading" => "Downloading", "processing" => "Reading metadata", "completed" => "Ready to listen", "cancelled" => "Cancelled", "failed" => "Needs attention", _ => "Waiting" }; let percent = job.total_bytes.filter(|v| *v > 0).map(|v| (job.downloaded_bytes as f64 * 100.0 / v as f64).clamp(0.0, 100.0)).unwrap_or(0.0);
                                rsx! { article { class: "job-card", key: "{job.id}", div { class: "job-icon", if job.status == "completed" { "✓" } else { "↧" } } div { class: "job-info", strong { {job.url.split('/').next_back().filter(|s| !s.is_empty()).unwrap_or("Audio download")} } span { class: "job-status", "{label} · {bytes(job.downloaded_bytes)}" } if matches!(job.status.as_str(), "queued" | "running" | "downloading" | "processing") { progress { max: "100", value: "{percent}", "aria-label": "Download progress" } } if let Some(message) = job.error { p { class: "job-error", "{message}" } } }
                                    if matches!(job.status.as_str(), "queued" | "running" | "downloading") { button { class: "button secondary small", onclick: move |_| { let id = cancel_id.clone(); spawn_authorized(auth, async move { match mutate("POST", &format!("/api/imports/{id}/cancel"), None, &token(auth)).await { Ok(()) => { if let Ok(value) = api::<Vec<ImportJob>>("GET", "/api/imports", None, "").await { jobs.set(value); } } Err(message) => error.set(message) } }); }, "Cancel" } }
                                    if job.status == "failed" || job.status == "cancelled" { button { class: "button secondary small", onclick: move |_| { let id = retry_id.clone(); spawn_authorized(auth, async move { match mutate("POST", &format!("/api/imports/{id}/retry"), None, &token(auth)).await { Ok(()) => { if let Ok(value) = api::<Vec<ImportJob>>("GET", "/api/imports", None, "").await { jobs.set(value); } } Err(message) => error.set(message) } }); }, "Retry" } }
                                } } }
                            }
                        }
                    }

                    if page() == "playlists" {
                        div { class: "section-toolbar", div { h2 { "Your playlists" } p { "A soundtrack for whatever comes next." } }
                            form { class: "inline-form", onsubmit: move |e| { e.prevent_default(); let name = playlist_name().trim().to_string(); if name.is_empty() { return; } spawn_authorized(auth, async move { match api::<PlaylistSummary>("POST", "/api/playlists", Some(json!({"name": name})), &token(auth)).await { Ok(value) => { playlists.write().insert(0, value); playlist_name.set(String::new()); } Err(message) => error.set(message) } }); }, label { class: "sr-only", r#for: "playlist-name", "New playlist name" } input { id: "playlist-name", required: true, maxlength: 150, placeholder: "New playlist name", value: "{playlist_name}", oninput: move |e| playlist_name.set(e.value()) } button { class: "button primary small", r#type: "submit", "+ Create" } }
                        }
                        if playlists.read().is_empty() { div { class: "empty-state", span { class: "empty-icon", "≡" } h3 { "Make room for a mood" } p { "Create a playlist, then add tracks from your library." } } }
                        div { class: "playlist-grid", for list in playlists.read().iter().cloned() { button { class: "playlist-card", key: "{list.id}", onclick: move |_| { let id = list.id.clone(); share_url.set(String::new()); spawn_authorized(auth, async move { match api::<PlaylistDetail>("GET", &format!("/api/playlists/{id}"), None, "").await { Ok(value) => selected_playlist.set(Some(value)), Err(message) => error.set(message) } }); }, div { class: "playlist-art", "≡" } h3 { "{list.name}" } p { "{list.item_count} tracks" if !list.can_edit { " · Shared with you" } } } } }
                        if let Some(detail) = selected_playlist.read().clone() { section { class: "panel playlist-detail", div { class: "section-toolbar", div { span { class: "eyebrow", "PLAYLIST" } h2 { "{detail.playlist.name}" } p { "{detail.items.len()} tracks · Revision {detail.playlist.revision}" } } if detail.items.iter().any(|i| i.can_stream) { button { class: "button primary", onclick: move |_| { if let Some(value) = selected_playlist.peek().as_ref() { play_queue(value.items.iter().filter(|i| i.can_stream).map(|i| i.track.clone()).collect(), 0, queue, index, auth, error); } }, "▶ Play playlist" } } }
                                if detail.playlist.owner_id != auth.read().as_ref().map(|a|a.user.id.clone()).unwrap_or_default()
                                    && playlists.read().iter().any(|p|p.id==detail.playlist.id) {
                                    button { class: "button secondary small", onclick: move |_| {
                                        let id=selected_playlist.peek().as_ref().map(|d|d.playlist.id.clone()).unwrap_or_default();
                                        let user=auth.peek().as_ref().map(|a|a.user.id.clone()).unwrap_or_default();
                                        spawn_authorized(auth, async move { match mutate("DELETE",&format!("/api/playlists/{id}/collaborators/{user}"),None,&token(auth)).await {
                                            Ok(())=>{selected_playlist.set(None); if let Ok(value)=api::<Vec<PlaylistSummary>>("GET","/api/playlists",None,"").await {playlists.set(value);} notice.set("You left the playlist.".into());}
                                            Err(message)=>error.set(message),
                                        } });
                                    }, "Leave playlist" }
                                }
                                for (at, entry) in detail.items.iter().cloned().enumerate() { { let track = entry.track.clone(); let item_id = entry.id.clone(); let revision = detail.playlist.revision; let list_id = detail.playlist.id.clone(); rsx! { div { class: "playlist-track", key: "{entry.id}", Art { track: track.clone() } div { strong { "{track.title}" } small { "{track.artist}" } }
                                            if entry.can_stream { button { class: "icon-button", "aria-label": "Play {track.title}", onclick: move |_| play_queue(vec![track.clone()], 0, queue, index, auth, error), "▶" } } else { span { class: "quality-tag", "Metadata only" } }
                                            if detail.playlist.can_edit { button { class: "icon-button", "aria-label": "Move track up in playlist", disabled: at == 0, onclick: move |_| {
                                                let Some(current) = selected_playlist.peek().clone() else { return; };
                                                let mut ids = current.items.iter().map(|entry| entry.id.clone()).collect::<Vec<_>>();
                                                if at == 0 || at >= ids.len() { return; }
                                                ids.swap(at, at - 1);
                                                spawn_authorized(auth, async move {
                                                    let id = current.playlist.id;
                                                    match mutate("PUT", &format!("/api/playlists/{id}/order"), Some(json!({"revision": current.playlist.revision, "mutation_id": uuid::Uuid::new_v4().to_string(), "item_ids": ids})), &token(auth)).await {
                                                        Ok(()) => notice.set("Playlist order updated.".into()),
                                                        Err(message) => error.set(message),
                                                    }
                                                    if let Ok(value) = api::<PlaylistDetail>("GET", &format!("/api/playlists/{id}"), None, "").await { selected_playlist.set(Some(value)); }
                                                });
                                            }, "↑" } }
                                            if detail.playlist.can_edit { button { class: "icon-button", "aria-label": "Remove from playlist", onclick: move |_| { let id = list_id.clone(); let item = item_id.clone(); spawn_authorized(auth, async move { let path = format!("/api/playlists/{id}/items/{item}?revision={revision}&mutation_id={}", uuid::Uuid::new_v4()); match mutate("DELETE", &path, None, &token(auth)).await { Ok(()) => { if let Ok(value) = api::<PlaylistDetail>("GET", &format!("/api/playlists/{id}"), None, "").await { selected_playlist.set(Some(value)); } } Err(message) => error.set(message) } }); }, "×" } }
                                        } } } }
                                if detail.playlist.owner_id == auth.read().as_ref().map(|a| a.user.id.clone()).unwrap_or_default() {
                                    div { class: "sharing-controls", h3 { "Share the playlist, keep control" } p { "Named collaborators can view playlist metadata. Link views include titles from your own library; contributors’ private tracks stay hidden. Audio access remains separate." }
                                        if !collaborator_list.read().is_empty() {
                                            div { class: "collaborator-list", "aria-label": "Playlist collaborators",
                                                for person in collaborator_list.read().iter().cloned() {
                                                    div { class: "section-toolbar", key: "{person.user_id}",
                                                        span { "{person.username}" small { if person.can_edit { " · Editor" } else { " · Viewer" } } }
                                                        button { class: "button danger small", "aria-label": "Revoke {person.username}'s playlist access", onclick: move |_| {
                                                            let id=selected_playlist.peek().as_ref().map(|p|p.playlist.id.clone()).unwrap_or_default();
                                                            let target=person.user_id.clone();
                                                            spawn_authorized(auth, async move { match mutate("DELETE",&format!("/api/playlists/{id}/collaborators/{target}"),None,&token(auth)).await {
                                                                Ok(())=>{collaborator_revision.set(collaborator_revision()+1); notice.set("Collaborator access revoked.".into());}
                                                                Err(message)=>error.set(message),
                                                            } });
                                                        }, "Revoke access" }
                                                    }
                                                }
                                            }
                                        }
                                        form { class: "inline-form", onsubmit: move |e| { e.prevent_default(); let username = collaborator(); let can_edit = collaborator_edit(); let id = selected_playlist.peek().as_ref().map(|v| v.playlist.id.clone()).unwrap_or_default(); spawn_authorized(auth, async move { match mutate("POST", &format!("/api/playlists/{id}/collaborators"), Some(json!({"username": username, "can_edit": can_edit})), &token(auth)).await { Ok(()) => { collaborator.set(String::new()); notice.set("Playlist access updated.".into()); collaborator_revision.set(collaborator_revision()+1); } Err(message) => error.set(message) } }); }, label { class: "sr-only", r#for: "collaborator", "Collaborator username" } input { id: "collaborator", placeholder: "Username on this server", required: true, value: "{collaborator}", oninput: move |e| collaborator.set(e.value()) } label { class: "checkbox-label", input { r#type: "checkbox", checked: collaborator_edit(), onchange: move |e| collaborator_edit.set(e.checked()) } "Can edit" } button { class: "button secondary small", r#type: "submit", "Grant access" } }
                                        div { class: "inline-form share-form", label { r#for: "share-expiry", "Link expires in" } select { id: "share-expiry", value: "{share_days}", onchange: move |e| share_days.set(e.value()), option { value: "1", "1 day" } option { value: "7", "7 days" } option { value: "30", "30 days" } }
                                            button { class: "button secondary small", onclick: move |_| { let id = selected_playlist.peek().as_ref().map(|v| v.playlist.id.clone()).unwrap_or_default(); let days = share_days().parse::<i64>().unwrap_or(7); spawn_authorized(auth, async move { match api::<ShareResponse>("POST", &format!("/api/playlists/{id}/shares"), Some(json!({"expires_in_seconds": days * 86400})), &token(auth)).await { Ok(value) => { let origin = web_sys::window().and_then(|w| w.location().origin().ok()).unwrap_or_default(); share_url.set(format!("{origin}/#share={}", value.token)); } Err(message) => error.set(message) } }); }, "Create view link" }
                                            button { class: "button danger small", onclick: move |_| { let id = selected_playlist.peek().as_ref().map(|v| v.playlist.id.clone()).unwrap_or_default(); spawn_authorized(auth, async move { match mutate("DELETE", &format!("/api/playlists/{id}/shares"), None, &token(auth)).await { Ok(()) => { share_url.set(String::new()); notice.set("All share links for this playlist were revoked.".into()); } Err(message) => error.set(message) } }); }, "Revoke all links" }
                                        }
                                        if !share_url().is_empty() { label { r#for: "share-link", "Copy this view-only link" } input { id: "share-link", readonly: true, value: "{share_url}", onclick: move |_| { if let Some(input) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id("share-link")).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) { input.select(); } } } }
                                    }
                                }
                            } }
                    }

                    if page() == "settings" {
                        section { class: "settings-grid", div { class: "panel", h2 { "A space that is yours" } p { "Your library is private. Music files stay on this server, and your session credentials stay in a secure browser cookie." }
                                dl { dt { "Server version" } dd { "{version}" } dt { "Collection storage" } dd { {stats.read().as_ref().map(|s| bytes(s.size_bytes)).unwrap_or_else(|| "—".into())} } dt { "Playback" } dd { "Original audio · Browser native" } dt { "Federation" } dd { "Disabled" } }
                                button { class: "button secondary", onclick: move |_| { spawn_authorized(auth, async move { match mutate("POST", "/api/logout", None, &token(auth)).await { Ok(()) => { if let Some(player) = audio() { let _ = player.pause(); player.set_src(""); } clear_media_session(); auth.set(None); invalidate_library(library); clear_catalog(catalog); tracks.set(Vec::new()); queue.set(Vec::new()); playlists.set(Vec::new()); selected_playlist.set(None); stats.set(None); jobs.set(Vec::new()); share_url.set(String::new()); page.set("library".into());  error.set(String::new()); username.set(String::new()); } Err(message) => error.set(message) } }); }, "Sign out" }
                            }
                            div { class: "panel", h2 { "Listening controls" } p { "Press Space to play or pause, and the arrow keys to seek 10 seconds. Use your device’s media controls when your browser supports them." } button { class: "button secondary", onclick: move |_| theme.set(if theme() == "dark" { "light".into() } else { "dark".into() }), if theme() == "dark" { "Switch to light theme" } else { "Switch to dark theme" } } small { "Codec support depends on your browser. Files are streamed directly without transcoding." } }
                            if auth.read().as_ref().is_some_and(|a| a.user.is_admin) { div { class: "panel", h2 { "Invite someone in" } p { "Create an account with its own private library. Share playlists explicitly after they sign in." }
                                    form { onsubmit: move |e| { e.prevent_default(); spawn_authorized(auth, async move { match api::<User>("POST", "/api/users", Some(json!({"username": new_user(), "password": new_password()})), &token(auth)).await { Ok(value) => { notice.set(format!("Account {} created. Share the password privately.", value.username)); new_user.set(String::new()); new_password.set(String::new()); } Err(message) => error.set(message) } }); }, label { r#for: "new-user", "New username" } input { id: "new-user", required: true, maxlength: 64, value: "{new_user}", autocomplete: "off", oninput: move |e| new_user.set(e.value()) } label { r#for: "new-password", "Initial password" } input { id: "new-password", r#type: "password", required: true, minlength: 12, value: "{new_password}", autocomplete: "new-password", oninput: move |e| new_password.set(e.value()) } button { class: "button primary", r#type: "submit", "Create account" } }
                                } }
                            if auth.read().as_ref().is_some_and(|a| a.user.is_admin) {
                                scanner::ScannerPanel { auth, onchanged: move |_| {
                                    catalog_changed(facets_revision);
                                    invalidate_library(library);
                                    invalidate_catalog(catalog);
                                    spawn_authorized(auth, async move {
                                        if let Ok(value) = api::<LibraryStats>("GET", "/api/stats", None, "").await {
                                            stats.set(Some(value));
                                        }
                                    });
                                } }
                            }
                        }
                    }
                    footer { class: "content-footer", "YOUR MUSIC, WITHOUT THE NOISE." span { "Vela Music" } }
                }

                audio { id: "vela-audio", preload: "metadata", onplay: move |_| playing.set(true), onpause: move |_| playing.set(false), ontimeupdate: move |_| { if let Some(p) = audio() { position.set(p.current_time()); } }, onloadedmetadata: move |_| { if let Some(p) = audio() { duration.set(p.duration()); p.set_volume(volume()); } }, onerror: move |_| { if audio().is_some_and(|p| !p.src().is_empty()) { error.set("This track could not be played. Check your connection and the browser’s codec support.".into()); } }, onended: move |_| { if repeat() == "one" { if let Some(p) = audio() { p.set_current_time(0.0); let _ = p.play(); } } else { advance(1, queue, index, shuffle(), &repeat(), auth, error); } } }
                div { class: "player-bar", "aria-label": "Music player",
                    div { class: "now-playing", if let Some(track) = current.clone() { Art { track: track.clone() } div { strong { "{track.title}" } small { "{track.artist}" } } } else { div { class: "artwork", "♫" } div { strong { "Ready when you are" } small { "Choose something you love" } } } }
                    div { class: "player-center", div { class: "player-buttons", button { class: if shuffle() { "icon-button selected" } else { "icon-button" }, "aria-label": "Shuffle", "aria-pressed": shuffle(), disabled: current.is_none(), onclick: move |_| { shuffle.set(!shuffle()); }, "⤨" }
                            button { id: "player-previous", class: "icon-button", "aria-label": "Previous track", disabled: current.is_none(), onclick: move |_| advance(-1, queue, index, shuffle(), &repeat(), auth, error), "◀◀" }
                            button { id: "player-toggle", class: "play-toggle", "aria-label": if playing() { "Pause" } else { "Play" }, disabled: current.is_none(), onclick: move |_| { if let Some(p) = audio() { if playing() { let _ = p.pause(); } else if p.src().is_empty() { if let Some(track) = queue.peek().get(*index.peek()) { start_audio(track, error); } } else if let Ok(promise) = p.play() { spawn_authorized(auth, async move { if JsFuture::from(promise).await.is_err() { error.set("Playback could not start. Try selecting the track again.".into()); } }); } } }, if playing() { "Ⅱ" } else { "▶" } }
                            button { id: "player-next", class: "icon-button", "aria-label": "Next track", disabled: current.is_none(), onclick: move |_| advance(1, queue, index, shuffle(), &repeat(), auth, error), "▶▶" }
                            button { class: "icon-button mobile-queue", "aria-label": "Show playback queue and controls", onclick: move |_| show_queue.set(!show_queue()), "≡" }
                            button { class: if repeat() != "off" { "icon-button selected" } else { "icon-button" }, "aria-label": "Repeat: {repeat}", disabled: current.is_none(), onclick: move |_| { repeat.set(match repeat().as_str() { "off" => "all", "all" => "one", _ => "off" }.into()); }, if repeat() == "one" { "↻₁" } else { "↻" } }
                        }
                        div { class: "seek-row", span { "{seconds(position())}" } input { r#type: "range", min: "0", max: if duration().is_finite() { duration().max(1.0).to_string() } else { "1".into() }, step: "0.1", value: "{position}", "aria-label": "Playback position", disabled: current.is_none(), oninput: move |e| { if let Ok(value) = e.value().parse::<f64>() && let Some(p) = audio() { p.set_current_time(value); position.set(value); } } } span { "{seconds(duration())}" } }
                    }
                    div { class: "player-right", button { class: if show_queue() { "icon-button selected" } else { "icon-button" }, "aria-label": "Show playback queue", "aria-expanded": show_queue(), onclick: move |_| { show_queue.set(!show_queue()); }, "≡" } span { class: "volume-icon", "◖" } input { r#type: "range", min: "0", max: "1", step: "0.01", value: "{volume}", "aria-label": "Volume", oninput: move |e| { if let Ok(value) = e.value().parse::<f64>() { volume.set(value); if let Some(p) = audio() { p.set_volume(value); } } } } }
                }
                nav { class: "mobile-nav", "aria-label": "Mobile navigation", for (id, symbol, label) in [("library", "◈", "Library"), ("albums", "▣", "Albums"), ("artists", "◌", "Artists"), ("favorites", "♡", "Favorites"), ("playlists", "≡", "Playlists"), ("imports", "↧", "Add music"), ("settings", "⚙", "Settings")] { button { class: if page() == id { "active" } else { "" }, "aria-label": label, onclick: move |_| page.set(id.into()), span { "{symbol}" } small { "{label}" } } } }
                if show_queue() { aside { class: "queue-panel", "aria-label": "Playback queue", div { class: "section-toolbar", div { span { class: "eyebrow", "UP NEXT" } h2 { "Your listening queue" } } button { class: "icon-button", "aria-label": "Close playback queue", onclick: move |_| show_queue.set(false), "×" } }
                        div { class: "queue-playback-controls", button { class: "button secondary small", disabled: current.is_none(), onclick: move |_| advance(-1, queue, index, shuffle(), &repeat(), auth, error), "Previous" } button { class: if shuffle() { "button primary small" } else { "button secondary small" }, "aria-pressed": shuffle(), onclick: move |_| shuffle.set(!shuffle()), "Shuffle" } button { class: "button secondary small", onclick: move |_| repeat.set(match repeat().as_str() { "off" => "all", "all" => "one", _ => "off" }.into()), "Repeat: {repeat}" }
                            label { r#for: "queue-volume", "Volume" } input { id: "queue-volume", r#type: "range", min: "0", max: "1", step: "0.01", value: "{volume}", oninput: move |e| { if let Ok(value) = e.value().parse::<f64>() { volume.set(value); if let Some(p) = audio() { p.set_volume(value); } } } }
                        }
                        if queue.read().is_empty() { p { class: "muted", "Choose a track to start your queue." } }
                        for (at, track) in queue.read().iter().cloned().enumerate() { div { class: if at == index() { "queue-row current" } else { "queue-row" }, key: "{at}-{track.id}", button { class: "queue-track", onclick: move |_| { if let Some(value) = queue.peek().get(at) { start_audio(value, error); index.set(at); } }, Art { track: track.clone() } div { strong { "{track.title}" } small { "{track.artist}" } } }
                                button { class: "icon-button", "aria-label": "Move track up", disabled: at == 0, onclick: move |_| { if at==0 || at>=queue.peek().len() {return;} let current=index(); queue.write().swap(at, at-1); if current==at {index.set(at-1);} else if current==at-1 {index.set(at);} persist_queue(&queue.peek(), auth); }, "↑" }
                                button { class: "icon-button", "aria-label": "Remove queued track", disabled: at == index(), onclick: move |_| { if at>=queue.peek().len() || at==index() {return;} queue.write().remove(at); if at < index() { index.set(index() - 1); } persist_queue(&queue.peek(), auth); }, "×" }
                            } }
                        if !queue.read().is_empty() { button { class: "button secondary", onclick: move |_| { let retained = queue.peek().get(*index.peek()).cloned(); queue.set(retained.into_iter().collect()); index.set(0); persist_queue(&queue.peek(), auth); }, "Clear upcoming tracks" } }
                    } }
                if let Some(editing) = editing_track.read().clone() {
                    metadata::MetadataEditor { key: "{editing.id}", track_id: editing.id.clone(), auth,
                        onclose: move |_| editing_track.set(None),
                        onsaved: move |updated: Track| {
                            catalog_changed(facets_revision);
                            let mut catalog = catalog;
                            for track in tracks.write().iter_mut().filter(|t| t.id == updated.id) { *track = updated.clone(); }
                            for track in catalog.tracks.write().iter_mut().filter(|t| t.id == updated.id) { *track = updated.clone(); }
                            for track in queue.write().iter_mut().filter(|t| t.id == updated.id) { *track = updated.clone(); }
                            persist_queue(&queue.peek(), auth);
                            if queue.peek().get(index()).is_some_and(|track| track.id == updated.id) { update_media_metadata(&updated); }
                            editing_track.set(None);
                            notice.set("Metadata saved. Your original audio file is unchanged.".into());
                            if matches!(page.peek().as_str(), "albums" | "artists") { catalog_page(None, catalog); }
                            if matches!(page.peek().as_str(), "library" | "favorites") {
                                spawn_authorized(auth, load_library(None, library));
                            }
                            spawn_authorized(auth, async move { if let Ok(value) = api::<LibraryStats>("GET", "/api/stats", None, "").await { stats.set(Some(value)); } });
                        }
                    }
                }
                if add_music() { div { class: "modal-backdrop", div { class: "modal", role: "dialog", "aria-modal": "true", "aria-labelledby": "upload-title", div { class: "section-toolbar", span { class: "eyebrow", "MAKE YOURSELF AT HOME" } button { class: "icon-button", "aria-label": "Close upload dialog", disabled: uploading(), onclick: move |_| add_music.set(false), "×" } }
                            if !error().is_empty() { div { class: "alert error", role: "alert", "{error}" } } h2 { id: "upload-title", "Bring your music along" } p { "Choose audio files from your device. Vela reads the tags, saves the artwork, and keeps your originals intact." }
                            label { class: "upload-zone", r#for: "audio-files", span { "↥" } strong { "Choose audio files" } small { "FLAC · MP3 · AAC · Opus · Vorbis · WAV" } input { id: "audio-files", r#type: "file", multiple: true, accept: "audio/*,.flac,.mp3,.m4a,.aac,.ogg,.opus,.wav,.aiff,.aif,.ape,.wv", disabled: uploading() } }
                            if !upload_status().is_empty() { p { class: "upload-status", role: "status", "{upload_status}" } }
                            button { class: "button primary", disabled: uploading(), onclick: move |_| { if uploading() { return; } uploading.set(true); error.set(String::new()); spawn_authorized(auth, async move { match upload_files(token(auth), upload_status, error).await { Ok(count) => { upload_status.set(format!("{count} files processed.")); if count > 0 { catalog_changed(facets_revision); notice.set(format!("{count} audio files are ready to listen.")); if matches!(page.peek().as_str(), "albums" | "artists") { catalog_page(None, catalog); } load_library(None, library).await; if let Ok(value) = api::<LibraryStats>("GET", "/api/stats", None, "").await { stats.set(Some(value)); } } } Err(message) => error.set(message) } uploading.set(false); }); }, if uploading() { "Uploading and reading tags…" } else { "Upload to my library" } }
                            div { class: "modal-divider", "or" } button { class: "button secondary", disabled: uploading(), onclick: move |_| { add_music.set(false); page.set("imports".into()); }, "Import from a download link →" } small { "Files are processed one at a time. Keep this tab open while uploading." }
                        } } }
            }
        }
    }
}
