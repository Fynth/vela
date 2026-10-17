//! Listener pages use bounded typed requests and the root-owned playback actions.
use crate::session::{ContextKind, PlaybackContext};
use crate::{
    Art, CatalogCover,
    client::{RequestScope, get},
    filters::append_query,
    icons::Icon,
    seconds, spawn_authorized, token,
};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use std::{cell::Cell, rc::Rc};
use vela_domain::*;

#[derive(Clone, Copy, PartialEq)]
pub struct Actions {
    pub play: EventHandler<(Vec<Track>, usize, PlaybackContext)>,
    pub queue: EventHandler<(Track, bool)>,
    pub favorite: EventHandler<Track>,
    pub radio: EventHandler<Track>,
    pub album: EventHandler<AlbumSummary>,
    pub artist: EventHandler<ArtistSummary>,
    pub playlist: EventHandler<String>,
    pub navigate: EventHandler<String>,
    pub add: EventHandler<()>,
}

#[component]
fn TrackActions(track: Track, actions: Actions) -> Element {
    let next = track.clone();
    let last = track.clone();
    let favorite = track.clone();
    let radio = track.clone();
    rsx! { div { class: "listening-actions",
        button { class: "icon-button", "aria-label": "Play {track.title} next", title: "Play next", onclick: move |_| actions.queue.call((next.clone(), true)), Icon { name: "next-add" } }
        button { class: "icon-button", "aria-label": "Add {track.title} to playback queue", title: "Add to queue", onclick: move |_| actions.queue.call((last.clone(), false)), Icon { name: "plus" } }
        button { class: if track.favorite { "icon-button selected" } else { "icon-button" }, "aria-label": if track.favorite { format!("Remove {} from favorites", track.title) } else { format!("Favorite {}", track.title) }, "aria-pressed": track.favorite, title: "Favorite", onclick: move |_| actions.favorite.call(favorite.clone()), Icon { name: "heart" } }
        details { class: "track-info-menu",
            summary { "aria-label": "Audio details for {track.title}", title: "Audio details", Icon { name: "more" } }
            div { class: "track-info-content", strong { "Original audio" } dl {
                dt { "Format" } dd { "{track.codec}" }
                if let Some(rate) = track.sample_rate { dt { "Sample rate" } dd { "{rate} Hz" } }
                if let Some(depth) = track.bit_depth { dt { "Bit depth" } dd { "{depth} bits" } }
                if let Some(rate) = track.bitrate { dt { "Bitrate" } dd { "{rate} kbps" } }
                dt { "Duration" } dd { "{seconds(track.duration_ms as f64 / 1000.0)}" }
            } button { class: "button secondary small", "aria-label": "Start track radio for {track.title}", onclick: move |_| actions.radio.call(radio.clone()), Icon { name: "radio" } "Start track radio" } }
        }
    } }
}

#[component]
fn TrackCards(tracks: Vec<Track>, origin: PlaybackContext, actions: Actions) -> Element {
    rsx! { div { class: "home-track-grid", for (at, track) in tracks.iter().cloned().enumerate() {
        { let origin=origin.clone(); let context = tracks.clone(); let play = track.clone(); rsx! {
            article { class: "listening-card", key: "{track.id}",
                button { class: "listening-card-play", "aria-label": "Play {track.title}", onclick: move |_| actions.play.call((context.clone(), at, origin.clone())),
                    div { class: "listening-cover", Art { track: play } span { class: "cover-play", Icon { name: "play" } } }
                    strong { "{track.title}" } small { "{track.artist}" }
                }
                TrackActions { track:track.clone(), actions }
            }
        } }
    } } }
}

#[derive(Clone, Default)]
struct HomeData {
    recent: Vec<Track>,
    favorites: Vec<Track>,
    albums: Vec<AlbumSummary>,
    playlists: Vec<PlaylistSummary>,
}

#[component]
pub fn Home(
    auth: Signal<Option<AuthResponse>>,
    revision: Signal<u64>,
    actions: Actions,
) -> Element {
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let dropped = alive.clone();
    let requests = use_hook(RequestScope::default);
    let cleanup = requests.clone();
    use_drop(move || {
        dropped.set(false);
        cleanup.cancel();
    });
    let mut generation = use_signal(|| 0_u64);
    let mut refresh = use_signal(|| 0_u64);
    let mut data = use_signal(HomeData::default);
    let mut owner = use_signal(String::new);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(String::new);
    use_effect(move || {
        requests.cancel();
        let session = token(auth);
        let _auth = auth.read();
        let _revision = *revision.read();
        let _refresh = *refresh.read();
        let request = generation.peek().wrapping_add(1);
        generation.set(request);
        if *owner.peek() != session {
            data.set(HomeData::default());
            owner.set(session.clone());
        }
        if session.is_empty() {
            data.set(HomeData::default());
            return;
        }
        loading.set(true);
        let signal = match requests.begin() {
            Ok(signal) => signal,
            Err(message) => {
                error.set(message);
                loading.set(false);
                return;
            }
        };
        let alive = alive.clone();
        spawn_authorized(auth, async move {
            let recent = get::<TrackPage>("/api/tracks?sort=newest&limit=8", &signal).await;
            let favorites =
                get::<TrackPage>("/api/tracks?sort=newest&favorites=true&limit=8", &signal).await;
            let albums = get::<AlbumPage>("/api/albums?limit=6", &signal).await;
            let playlists = get::<Vec<PlaylistSummary>>("/api/playlists", &signal).await;
            if !alive.get() || token(auth) != session || *generation.peek() != request {
                return;
            }
            let failed =
                recent.is_err() || favorites.is_err() || albums.is_err() || playlists.is_err();
            data.set(HomeData {
                recent: recent.map(|p| p.items).unwrap_or_default(),
                favorites: favorites.map(|p| p.items).unwrap_or_default(),
                albums: albums.map(|p| p.items).unwrap_or_default(),
                playlists: playlists.unwrap_or_default().into_iter().take(6).collect(),
            });
            error.set(if failed {
                "Some sections could not be loaded. Check your connection and refresh Home.".into()
            } else {
                String::new()
            });
            loading.set(false);
        });
    });
    let snapshot = data.read().clone();
    let name = auth
        .read()
        .as_ref()
        .map(|a| a.user.username.clone())
        .unwrap_or_default();
    let playable = snapshot.recent.clone();
    rsx! { section { class: "home-page", "aria-label": "Home",
        div { class: "home-hero",
            div { span { class: "eyebrow", "YOURS TO DISCOVER" } h1 { "Hello, {name}." } p { "A little closer to the music you love." }
                if !playable.is_empty() { button { class: "button cream", onclick: move |_| actions.play.call((playable.clone(), 0, PlaybackContext::new(ContextKind::Home,"New in your library"))), Icon { name: "play" } "Play new additions" } }
                else if !loading() { button { class: "button cream", onclick: move |_| actions.add.call(()), Icon { name: "plus" } "Add your first music" } }
            }
            div { class: "home-artwork", "aria-hidden": "true", if let Some(track) = snapshot.recent.first() { Art { track: track.clone() } } else { div { class: "home-vinyl", span { "v" } } } }
        }
        div { class: "section-toolbar home-toolbar", p { "Your music. A fresh place to start." } button { class: "button secondary small", disabled: loading(), onclick: move |_| { let next = refresh.peek().wrapping_add(1); refresh.set(next); }, "Refresh Home" } }
        if !error().is_empty() { div { class: "alert error", role: "alert", "{error}" } }
        if loading() && snapshot.recent.is_empty() { div { class: "loading-skeletons", role: "status", "aria-label": "Loading Home", for _ in 0..4 { div { class: "skeleton-card", "aria-hidden": "true" } } } }
        if !snapshot.recent.is_empty() {
            section { class: "home-section", "aria-labelledby": "home-additions", div { class: "section-toolbar", h2 { id: "home-additions", "New in your library" } button { class: "text-button", onclick: move |_| actions.navigate.call("library".into()), "Open library" } } TrackCards { tracks: snapshot.recent, origin:PlaybackContext::new(ContextKind::Home,"New in your library"), actions } }
        } else if !loading() && error().is_empty() { div { class: "empty-state", Icon { name: "music" } h2 { "Make room for your first listen" } p { "Upload music you own or import an authorized audio download. Your library stays private." } button { class: "button primary", onclick: move |_| actions.add.call(()), "Add music" } button { class: "text-button", onclick: move |_| actions.navigate.call("imports".into()), "Import from a URL" } } }
        if !snapshot.favorites.is_empty() { section { class: "home-section", "aria-labelledby": "home-favorites", div { class: "section-toolbar", h2 { id: "home-favorites", "Your favorites" } button { class: "text-button", onclick: move |_| actions.navigate.call("favorites".into()), "All favorites" } } TrackCards { tracks: snapshot.favorites, origin:PlaybackContext::new(ContextKind::Favorites,"Your favorites"), actions } } }
        if !snapshot.albums.is_empty() { section { class: "home-section", "aria-labelledby": "home-albums", h2 { id: "home-albums", "From your albums" } div { class: "home-album-grid", for album in snapshot.albums {
            { let open = album.clone(); rsx! { button { class: "listening-card collection-card", onclick: move |_| actions.album.call(open.clone()), "aria-label": "Open album {album.title} by {album.artist}", CatalogCover { artwork_id: album.artwork_id } strong { "{album.title}" } small { "{album.artist}" } } } }
        } } } }
        if !snapshot.playlists.is_empty() { section { class: "home-section", "aria-labelledby": "home-playlists", h2 { id: "home-playlists", "Your playlists" } div { class: "home-playlist-grid", for list in snapshot.playlists {
            { let id = list.id.clone(); let shared = auth.read().as_ref().is_some_and(|u| u.user.id != list.owner_id); rsx! { button { class: "home-playlist-card", onclick: move |_| actions.playlist.call(id.clone()), Icon { name: "playlists" } div { strong { "{list.name}" } small { "{list.item_count} tracks" if shared { " · Shared metadata" } } } } } }
        } } } }
    } }
}

#[component]
pub fn Search(
    auth: Signal<Option<AuthResponse>>,
    revision: Signal<u64>,
    actions: Actions,
) -> Element {
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let dropped = alive.clone();
    let requests = use_hook(RequestScope::default);
    let cleanup = requests.clone();
    use_drop(move || {
        dropped.set(false);
        cleanup.cancel();
    });
    let mut query = use_signal(String::new);
    let mut category = use_signal(|| "all");
    let mut generation = use_signal(|| 0_u64);
    let mut submitted = use_signal(|| 0_u64);
    let mut last_submit = use_signal(|| 0_u64);
    let mut results = use_signal(|| None::<SearchResults>);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(String::new);
    use_effect(move || {
        requests.cancel();
        let text = query.read().clone();
        let _auth = auth.read();
        let _revision = *revision.read();
        let submit = *submitted.read();
        let immediate = submit != *last_submit.peek();
        last_submit.set(submit);
        let request = generation.peek().wrapping_add(1);
        generation.set(request);
        results.set(None);
        error.set(String::new());
        let session = token(auth);
        if text.trim().is_empty() || session.is_empty() {
            loading.set(false);
            return;
        }
        if text.len() > 256 || text.chars().any(char::is_control) {
            loading.set(false);
            error.set("Use at most 256 bytes without control characters.".into());
            return;
        }
        loading.set(true);
        let signal = match requests.begin() {
            Ok(signal) => signal,
            Err(message) => {
                error.set(message);
                loading.set(false);
                return;
            }
        };
        let alive = alive.clone();
        spawn_authorized(auth, async move {
            if !immediate {
                TimeoutFuture::new(300).await;
            }
            if !alive.get() || token(auth) != session || *generation.peek() != request {
                return;
            }
            let mut url = "/api/search".to_string();
            append_query(&mut url, "q", text.trim());
            let response = get::<SearchResults>(&url, &signal).await;
            if !alive.get()
                || token(auth) != session
                || *generation.peek() != request
                || *query.peek() != text
            {
                return;
            }
            match response {
                Ok(value) => results.set(Some(value)),
                Err(message) => error.set(message),
            }
            loading.set(false);
        });
    });
    let found = results.read().clone();
    let selected_category = category();
    let count = found
        .as_ref()
        .map(|r| match selected_category {
            "tracks" => r.tracks.len(),
            "albums" => r.albums.len(),
            "artists" => r.artists.len(),
            "playlists" => r.playlists.len(),
            _ => r.tracks.len() + r.albums.len() + r.artists.len() + r.playlists.len(),
        })
        .unwrap_or(0);
    rsx! { section { class: "search-page", "aria-label": "Search music",
        form { class: "global-search-form", onsubmit: move |e| { e.prevent_default(); let next = submitted.peek().wrapping_add(1); submitted.set(next); },
            Icon { name: "search" } label { class: "sr-only", r#for: "global-search", "Search tracks, albums, artists and playlists" }
            input { id: "global-search", value: "{query}", maxlength: 256, placeholder: "What do you want to hear?", autocomplete: "off", autofocus: true, oninput: move |e| query.set(e.value()), onkeydown: move |e| { if e.key() == Key::Escape { query.set(String::new()); } } }
            if !query().is_empty() { button { class: "icon-button", r#type: "button", "aria-label": "Clear global search", onclick: move |_| query.set(String::new()), Icon { name: "close" } } }
            button { class: "button primary", r#type: "submit", "Search" }
        }
        div { class: "search-categories", "aria-label": "Search categories", for (id,label) in [("all","All"),("tracks","Tracks"),("albums","Albums"),("artists","Artists"),("playlists","Playlists")] {
            button { class: if selected_category == id { "filter-chip active" } else { "filter-chip" }, "aria-pressed": selected_category == id, onclick: move |_| { category.set(id); }, "{label}" }
        } }
        p { class: "search-scope", "Your library and explicitly shared playlist titles · Up to 8 results per category" }
        if loading() { div { class: "loading-skeletons", role: "status", "aria-label": "Searching your library", for _ in 0..3 { div { class: "skeleton-row", "aria-hidden": "true" } } } }
        if !error().is_empty() { div { class: "alert error", role: "alert", span { "{error}" } button { class: "button secondary small", onclick: move |_| { let next=submitted.peek().wrapping_add(1); submitted.set(next); }, "Retry search" } } }
        if query().trim().is_empty() { div { class: "empty-state search-empty", Icon { name: "search" } h2 { "Find your next listen" } p { "Search your tracks, albums, artists and playlist names. Only music you can access appears here." } } }
        if let Some(found) = found {
            if count == 0 { div { class: "empty-state", h2 { "No search results" } p { "Try another title or name, or choose a different category." } } }
            if matches!(selected_category,"all"|"tracks") && !found.tracks.is_empty() { section { class: "search-result-section", "aria-label": "Track results", h2 { "Tracks" } for (at,track) in found.tracks.iter().cloned().enumerate() {
                { let context = found.tracks.clone(); let row = track.clone(); rsx! { article { class: "search-track-row", key: "{track.id}", button { class: "search-track-main", "aria-label": "Play {track.title}", onclick: move |_| actions.play.call((context.clone(),at,PlaybackContext::new(ContextKind::Search,"Search results"))), Art { track: row } div { strong { "{track.title}" } small { "{track.artist} · {track.album}" } } Icon { name: "play" } } TrackActions { track:track.clone(), actions } } } }
            } } }
            if matches!(selected_category,"all"|"albums") && !found.albums.is_empty() { section { class: "search-result-section", "aria-label": "Album results", h2 { "Albums" } div { class: "home-album-grid", for album in found.albums {
                { let open=album.clone(); rsx! { button { class: "listening-card collection-card", "aria-label": "Open album {album.title} by {album.artist}", onclick: move |_| actions.album.call(open.clone()), CatalogCover { artwork_id: album.artwork_id } strong { "{album.title}" } small { "{album.artist}" } } } }
            } } } }
            if matches!(selected_category,"all"|"artists") && !found.artists.is_empty() { section { class: "search-result-section", "aria-label": "Artist results", h2 { "Artists" } div { class: "home-album-grid", for artist in found.artists {
                { let open=artist.clone(); rsx! { button { class: "listening-card collection-card", "aria-label": "Open artist {artist.name}", onclick: move |_| actions.artist.call(open.clone()), CatalogCover { artwork_id: artist.artwork_id, artist:true } strong { "{artist.name}" } small { "{artist.track_count} tracks in your library" } } } }
            } } } }
            if matches!(selected_category,"all"|"playlists") && !found.playlists.is_empty() { section { class: "search-result-section", "aria-label": "Playlist results", h2 { "Playlists" } div { class: "home-playlist-grid", for list in found.playlists {
                { let id=list.id.clone(); let shared=auth.read().as_ref().is_some_and(|u| u.user.id != list.owner_id); rsx! { button { class: "home-playlist-card", onclick: move |_| actions.playlist.call(id.clone()), Icon { name:"playlists" } div { strong { "{list.name}" } small { "{list.item_count} tracks" if shared { " · Shared metadata · Audio access checked separately" } } } } } }
            } } } }
        }
    } }
}
