//! Listener pages use bounded typed requests and the root-owned playback actions.
use crate::home_editorial::HomeFeatures;
use crate::session::{ContextKind, PlaybackContext};
use crate::{
    Art, CatalogCover, bytes,
    client::{RequestScope, get},
    filters::append_query,
    icons::Icon,
    seconds, spawn_authorized, token,
};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use std::{cell::Cell, rc::Rc};
use vela_domain::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, PartialEq)]
pub struct Actions {
    pub active_track: Memo<Option<String>>,
    pub requested: Signal<bool>,
    pub start: EventHandler<(Vec<Track>, usize, PlaybackContext)>,
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

impl Actions {
    pub fn is_playing(self, id: &str) -> bool {
        let requested = *self.requested.read();
        self.active_track.read().as_deref() == Some(id) && requested
    }
    pub fn play_label(self, id: &str, title: &str) -> String {
        format!(
            "{} {title}",
            if self.is_playing(id) { "Pause" } else { "Play" }
        )
    }
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
                button { class: "listening-card-play", "aria-label": actions.play_label(&track.id, &track.title), onclick: move |_| actions.play.call((context.clone(), at, origin.clone())),
                    div { class: "listening-cover", Art { track: play } span { class: "cover-play", if actions.is_playing(&track.id) {Icon {name:"pause"}} else {Icon {name:"play"}} } }
                    strong { "{track.title}" } small { "{track.artist}" }
                }
                TrackActions { track:track.clone(), actions }
            }
        } }
    } } }
}

/// IN01: bounded placeholders reuse the real card and row geometry so a pending
/// request never shifts the page. They invent no track names, percentages or
/// play actions, and unmount as soon as a request settles or fails.
const HOME_SKELETON_CARDS: usize = 4;
const SEARCH_SKELETON_ROWS: usize = 3;

#[component]
fn TrackSkeletonCards() -> Element {
    rsx! { div { class: "home-track-grid track-skeletons", role: "status", "aria-busy": "true", "aria-label": "Loading Home",
        for index in 0..HOME_SKELETON_CARDS {
            article { key: "{index}", class: "listening-card track-skeleton-card", "aria-hidden": "true",
                div { class: "track-skeleton-play",
                    div { class: "listening-cover", span { class: "artwork track-skeleton-art" } }
                    strong { span { class: "track-skeleton-bar" } }
                    small { span { class: "track-skeleton-bar artist" } }
                }
                div { class: "listening-actions track-skeleton-actions",
                    span { class: "icon-button track-skeleton-action" }
                    span { class: "icon-button track-skeleton-action" }
                    span { class: "icon-button track-skeleton-action" }
                }
            }
        }
    } }
}

#[component]
fn TrackSkeletonRows() -> Element {
    rsx! { div { class: "search-skeletons track-skeletons", role: "status", "aria-busy": "true", "aria-label": "Searching your library",
        for index in 0..SEARCH_SKELETON_ROWS {
            div { key: "{index}", class: "search-track-row track-skeleton-row", "aria-hidden": "true",
                div { class: "search-track-main",
                    span { class: "artwork track-skeleton-art" }
                    div { strong { span { class: "track-skeleton-bar" } } small { span { class: "track-skeleton-bar artist" } } }
                }
                div { class: "listening-actions track-skeleton-actions",
                    span { class: "icon-button track-skeleton-action" }
                    span { class: "icon-button track-skeleton-action" }
                    span { class: "icon-button track-skeleton-action" }
                }
            }
        }
    } }
}

#[derive(Clone, Default)]
struct HomeData {
    recent: Vec<Track>,
    albums: Vec<AlbumSummary>,
    playlists: Vec<PlaylistSummary>,
}

impl HomeData {
    fn is_empty(&self) -> bool {
        self.recent.is_empty() && self.albums.is_empty() && self.playlists.is_empty()
    }
}

/// TR04a: the bounded Home filter. Each view recomposes the DTOs Home already
/// requested; no view asks the server for the whole library and none of them
/// touches the root player, queue or source.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HomeView {
    ForYou,
    Albums,
    Playlists,
}

impl HomeView {
    const ALL: [Self; 3] = [Self::ForYou, Self::Albums, Self::Playlists];

    fn key(self) -> &'static str {
        match self {
            Self::ForYou => "for-you",
            Self::Albums => "albums",
            Self::Playlists => "playlists",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::ForYou => "For you",
            Self::Albums => "Albums",
            Self::Playlists => "Playlists",
        }
    }
    fn tab_id(self) -> String {
        format!("home-tab-{}", self.key())
    }
}

/// Roving tabindex: DOM focus has to follow the selection, otherwise the next
/// Tab press leaves the tab list from the previously focused tab.
fn select_home_view(mut view: Signal<HomeView>, next: HomeView) {
    view.set(next);
    if let Some(tab) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(&next.tab_id()))
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = tab.focus();
    }
}

fn home_additions(tracks: Vec<Track>, actions: Actions) -> Element {
    let playable = tracks.clone();
    rsx! { section { class: "home-section", "aria-labelledby": "home-additions",
        div { class: "section-toolbar",
            h2 { id: "home-additions", "New in your library" }
            div { class: "home-section-actions",
                button { class: "button primary small", onclick: move |_| actions.start.call((playable.clone(), 0, PlaybackContext::new(ContextKind::Home, "New in your library"))), Icon { name: "play" } "Play new additions" }
                button { class: "text-button", onclick: move |_| actions.navigate.call("library".into()), "Open library" }
            }
        }
        TrackCards { tracks, origin: PlaybackContext::new(ContextKind::Home, "New in your library"), actions }
    } }
}

/// At most six real album cards: borderless art, title and artist from the
/// owner-scoped AlbumSummary. The owned neutral fallback stands in only when
/// that album has no authorized cover.
fn home_album_shelf(
    title: &'static str,
    subtitle: &'static str,
    albums: Vec<AlbumSummary>,
    actions: Actions,
) -> Element {
    rsx! { section { class: "home-section home-albums", "aria-labelledby": "home-albums",
        div { class: "section-toolbar",
            div { h2 { id: "home-albums", "{title}" } p { "{subtitle}" } }
            button { class: "text-button", onclick: move |_| actions.navigate.call("albums".into()), "All albums" }
        }
        div { class: "home-album-shelf",
            for album in albums.into_iter().take(6) {
                { let open = album.clone(); rsx! {
                    button { key: "{album.id}", class: "home-album-card", "data-album-id": "{album.id}",
                        "aria-label": "Open album {album.title} by {album.artist}",
                        onclick: move |_| actions.album.call(open.clone()),
                        CatalogCover { artwork_id: album.artwork_id.clone() }
                        strong { "{album.title}" }
                        small { "{album.artist}" }
                    }
                } }
            }
        }
    } }
}

fn home_playlist_cards(
    playlists: Vec<PlaylistSummary>,
    owner_id: &str,
    actions: Actions,
) -> Element {
    rsx! { div { class: "home-playlist-grid",
        for list in playlists {
            { let id = list.id.clone(); let shared = list.owner_id != owner_id; rsx! {
                button { key: "{list.id}", class: "home-playlist-card", "data-playlist-id": "{list.id}",
                    "aria-label": "Open playlist {list.name}",
                    onclick: move |_| actions.playlist.call(id.clone()),
                    CatalogCover { artwork_id: list.artwork_id.clone() }
                    div { strong { "{list.name}" } small { "{list.item_count} tracks" if shared { " · Shared metadata" } } }
                }
            } }
        }
    } }
}

fn home_playlist_preview(
    playlists: Vec<PlaylistSummary>,
    owner_id: &str,
    actions: Actions,
) -> Element {
    rsx! { section { class: "home-section", "aria-labelledby": "home-playlists",
        div { class: "section-toolbar",
            div { h2 { id: "home-playlists", "Your playlists" } p { "Playlists you own or can open from your library." } }
            button { class: "text-button", onclick: move |_| actions.navigate.call("playlists".into()), "Open playlists" }
        }
        { home_playlist_cards(playlists.into_iter().take(3).collect(), owner_id, actions) }
    } }
}

/// The playlist view keeps the real create/open path: the create control opens
/// the existing Playlists creation form, and every card opens the real,
/// reauthorized playlist detail. Home never grows a second playlist writer.
fn home_playlists_view(
    playlists: Vec<PlaylistSummary>,
    owner_id: &str,
    settled: bool,
    actions: Actions,
) -> Element {
    rsx! { section { class: "home-section home-playlists-view", "aria-labelledby": "home-playlists",
        div { class: "section-toolbar",
            div { h2 { id: "home-playlists", "For every mood" } p { "Your private playlists, in one place." } }
            button { class: "button primary small", "aria-label": "Create a playlist", onclick: move |_| actions.navigate.call("playlists".into()), "+ Create a playlist" }
        }
        if playlists.is_empty() {
            if settled { div { class: "empty-state home-empty", span { class: "empty-icon", Icon { name: "queue" } }
                h3 { "Make room for a mood" }
                p { "You have no playlists yet. Create one on the Playlists page, then add tracks from your library." }
            } }
        } else { { home_playlist_cards(playlists, owner_id, actions) } }
    } }
}

/// The empty library state.
///
/// It states plainly what is missing, offers the single real action that fixes
/// it, and reports an import only when the server says one is really running.
/// It never promises mixes, recommendations, track counts or percentages that
/// the server did not report.
#[component]
fn HomeEmptyState(auth: Signal<Option<AuthResponse>>, actions: Actions) -> Element {
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let dropped = alive.clone();
    let requests = use_hook(RequestScope::default);
    let cleanup = requests.clone();
    use_drop(move || {
        dropped.set(false);
        cleanup.cancel();
    });
    let mut activity = use_signal(Vec::<crate::jobs::ImportActivity>::new);
    let mut loading = use_signal(|| true);
    // The last read failure, kept apart from the activity itself: an unknown
    // import state is not the same as an idle account.
    let mut error = use_signal(String::new);
    use_effect(move || {
        requests.cancel();
        let session = token(auth);
        let _auth = auth.read();
        if session.is_empty() {
            activity.set(Vec::new());
            loading.set(false);
            return;
        }
        loading.set(true);
        let alive = alive.clone();
        let requests = requests.clone();
        spawn_authorized(auth, async move {
            // One owned read loop. It asks again only while the server still
            // reports a running import, and stops as soon as none is left.
            loop {
                let signal = match requests.begin() {
                    Ok(signal) => signal,
                    Err(_) => break,
                };
                let result = crate::jobs::read_import_activity(auth, Some(&signal)).await;
                if !alive.get() || token(auth) != session {
                    return;
                }
                // A failed read is not "no imports". Keep the last confirmed
                // activity and surface the failure, so a network blip never
                // silently erases a running import.
                let items = match result {
                    Ok(items) => items,
                    Err(message) => {
                        error.set(message);
                        loading.set(false);
                        return;
                    }
                };
                error.set(String::new());
                activity.set(items.clone());
                loading.set(false);
                if items.is_empty() {
                    return;
                }
                TimeoutFuture::new(2_000).await;
                if !alive.get() || token(auth) != session {
                    return;
                }
            }
        });
    });
    let items = activity.read().clone();
    let running = items.len();
    let failure = error.read().clone();
    rsx! { div { class: "empty-state home-empty",
        span { class: "empty-icon", Icon { name: "music" } }
        h2 { "Make room for your first listen" }
        p { "Add music you own. Your library stays private to your account." }
        button { class: "button primary home-empty-action", r#type: "button",
            "aria-label": "Add music to your library",
            onclick: move |_| actions.add.call(()), "Add music" }
        if !failure.is_empty() {
            p { class: "home-empty-error", role: "status",
                "Import status is unavailable right now: {failure}" }
        }
        if running > 0 {
            div { class: "home-empty-progress", role: "status",
                p { class: "home-empty-progress-title",
                    if running == 1 { "1 import is running" } else { "{running} imports are running" } }
                for item in items.iter() {
                    div { class: "home-empty-progress-row",
                        span { class: "home-empty-progress-status", "{item.status}" }
                        span { class: "home-empty-progress-bytes",
                            "{bytes(item.downloaded_bytes)}"
                            if let Some(total) = item.total_bytes { " of {bytes(total)}" } }
                        if let Some(total) = item.total_bytes {
                            progress { class: "home-empty-progress-bar", max: total.to_string(),
                                value: item.downloaded_bytes.clamp(0, total).to_string(),
                                "aria-label": "Import download progress" }
                        }
                    }
                }
            }
        } else if (loading)() {
            p { class: "home-empty-progress", role: "status", "Checking for running imports…" }
        }
    } }
}

/// The honest first-listen hint for a library that has music but no albums.
///
/// It stays out of the way: it explains the album view without inventing
/// records.
fn home_first_listen(actions: Actions) -> Element {
    rsx! { div { class: "empty-state",
        h2 { "Make room for your first listen" }
        p { "Upload music you own or import an authorized audio download. Your library stays private." }
        button { class: "button primary", onclick: move |_| actions.add.call(()), "Add music" }
        button { class: "text-button", onclick: move |_| actions.navigate.call("imports".into()), "Import from a URL" }
    } }
}

fn home_albums_note(actions: Actions) -> Element {
    rsx! { div { class: "home-note", Icon { name: "albums" }
        div { strong { "No albums yet" }
            p { "Albums appear here when your files carry album tags. Untagged music still plays from your library." }
        }
        button { class: "text-button", onclick: move |_| actions.add.call(()), "Add tagged music" }
    } }
}

#[component]
pub fn Home(
    auth: Signal<Option<AuthResponse>>,
    revision: Signal<u64>,
    actions: Actions,
    library_state: Memo<Option<bool>>,
    children: Element,
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
    let view = use_signal(|| HomeView::ForYou);
    use_effect(move || {
        requests.cancel();
        let session = token(auth);
        let _auth = auth.read();
        let _revision = *revision.read();
        let _refresh = *refresh.read();
        let request = generation.peek().wrapping_add(1);
        generation.set(request);
        error.set(String::new());
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
            let albums = get::<AlbumPage>("/api/albums?limit=6", &signal).await;
            let playlists = get::<Vec<PlaylistSummary>>("/api/playlists", &signal).await;
            if !alive.get() || token(auth) != session || *generation.peek() != request {
                return;
            }
            let failed = recent.is_err() || albums.is_err() || playlists.is_err();
            // Known rows stay during a refresh: a failed section keeps its last
            // confirmed music instead of blanking what the listener already sees.
            let HomeData {
                recent: previous_recent,
                albums: previous_albums,
                playlists: previous_playlists,
            } = data.peek().clone();
            data.set(HomeData {
                recent: recent.map(|page| page.items).unwrap_or(previous_recent),
                albums: albums.map(|page| page.items).unwrap_or(previous_albums),
                playlists: playlists
                    .map(|items| items.into_iter().take(6).collect())
                    .unwrap_or(previous_playlists),
            });
            error.set(if failed {
                "Some sections could not be loaded. Check your connection and refresh Home.".into()
            } else {
                String::new()
            });
            loading.set(false);
        });
    });
    // Render only data that belongs to the current account: a session change
    // must never paint the previous account's rows, albums or playlists, even
    // for the frame before the refetch effect runs.
    let session = token(auth);
    let known_owner = !session.is_empty() && *owner.read() == session;
    let snapshot = if known_owner {
        data.read().clone()
    } else {
        HomeData::default()
    };
    let pending = *loading.read() || !known_owner;
    let error_text = error.read().clone();
    // Empty states settle only after a real request finished: a pending Home
    // shows bounded placeholders, never a no-results view.
    let settled = !pending && error_text.is_empty();
    let current = *view.read();
    let owner_id = auth
        .read()
        .as_ref()
        .map(|value| value.user.id.clone())
        .unwrap_or_default();
    // The empty library state needs a real answer before it may be shown.
    // Home's own settled reads are the freshest evidence: when they found
    // something, the server's older count must not hide it. Only when Home
    // itself settled on nothing does the server's count confirm the emptiness;
    // an unknown state keeps the skeleton and never promises a collection that
    // may exist.
    let home_settled_empty = settled && snapshot.is_empty();
    let empty_library = match library_state() {
        Some(empty) => empty && home_settled_empty,
        None => home_settled_empty,
    };
    let body = if pending && snapshot.is_empty() && error_text.is_empty() {
        match current {
            HomeView::Playlists => {
                rsx! { div { class: "loading-skeletons", role: "status", "aria-busy": "true", "aria-label": "Loading Home",
                    for _ in 0..3 { div { class: "skeleton-card", "aria-hidden": "true" } }
                } }
            }
            _ => rsx! { TrackSkeletonCards {} },
        }
    } else {
        match current {
            HomeView::ForYou => rsx! {
                // A fully empty library gets one honest state instead of the
                // editorial feature pair, which would promise mixes, picks and
                // discoveries that cannot exist yet.
                if !pending && empty_library {
                    HomeEmptyState { auth, actions }
                } else {
                    HomeFeatures { auth, revision, playlists: snapshot.playlists.clone(), owner_id: owner_id.clone(), actions }
                    if snapshot.recent.is_empty() {
                        if settled {
                            {home_first_listen(actions)}
                        }
                    } else {
                        {home_additions(snapshot.recent.clone(), actions)}
                    }
                    if !snapshot.albums.is_empty() {
                        {home_album_shelf("From your albums", "Familiar records from your own collection.", snapshot.albums.clone(), actions)}
                    }
                    if !snapshot.playlists.is_empty() {
                        {home_playlist_preview(snapshot.playlists.clone(), &owner_id, actions)}
                    }
                }
            },
            HomeView::Albums => rsx! {
                if !pending && empty_library {
                    HomeEmptyState { auth, actions }
                } else {
                    if snapshot.albums.is_empty() {
                        if settled && !snapshot.recent.is_empty() {
                            {home_albums_note(actions)}
                        }
                    } else {
                        {home_album_shelf("Your albums", "Every album in your library.", snapshot.albums.clone(), actions)}
                    }
                    if snapshot.recent.is_empty() {
                        if settled && snapshot.albums.is_empty() {
                            {home_first_listen(actions)}
                        }
                    } else {
                        {home_additions(snapshot.recent.clone(), actions)}
                    }
                }
            },
            HomeView::Playlists => {
                home_playlists_view(snapshot.playlists.clone(), &owner_id, settled, actions)
            }
        }
    };
    rsx! { section { class: "home-page", "aria-label": "Home",
        header { class: "home-header",
            div { class: "home-heading",
                span { class: "eyebrow", "MUSIC. ONLY YOURS." }
                h1 { "In your own rhythm" span { class: "home-title-dot", "aria-hidden": "true" } }
            }
            p { class: "home-private", Icon { name: "shield-check" } span { "Private space" } }
        }
        div { class: "home-filter-row",
            div { class: "home-filter-group", role: "tablist", "aria-label": "Home views",
                for option in HomeView::ALL {
                    { let selected = option == current; rsx! {
                        button { key: "{option.key()}", id: "{option.tab_id()}", r#type: "button", role: "tab",
                            class: if selected { "home-filter active" } else { "home-filter" },
                            "aria-selected": selected, "aria-controls": "home-view-panel",
                            tabindex: if selected { "0" } else { "-1" },
                            onclick: move |_| select_home_view(view, option),
                            onkeydown: move |event| {
                                let count = HomeView::ALL.len();
                                let index = HomeView::ALL.iter().position(|item| *item == option).unwrap_or(0);
                                let next = match event.key() {
                                    Key::ArrowRight | Key::ArrowDown => Some(HomeView::ALL[(index + 1) % count]),
                                    Key::ArrowLeft | Key::ArrowUp => Some(HomeView::ALL[(index + count - 1) % count]),
                                    Key::Home => Some(HomeView::ALL[0]),
                                    Key::End => Some(HomeView::ALL[count - 1]),
                                    _ => None,
                                };
                                if let Some(next) = next {
                                    event.prevent_default();
                                    select_home_view(view, next);
                                }
                            },
                            "{option.label()}"
                        }
                    } }
                }
            }
            div { class: "home-filter-meta",
                p { class: "home-filter-caption", "Familiar. Loved. Unexpected." }
                button { class: "text-button home-refresh", "aria-label": "Refresh Home", disabled: *loading.read(),
                    onclick: move |_| { let next = refresh.peek().wrapping_add(1); refresh.set(next); }, "Refresh" }
            }
        }
        if !error_text.is_empty() { div { class: "alert error", role: "alert", "{error_text}" } }
        div { class: "home-view", id: "home-view-panel", role: "tabpanel", "aria-labelledby": "{current.tab_id()}",
            {body}
        }
        div { class: "home-extra", hidden: current != HomeView::ForYou, {children} }
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
    // Which query and account the visible rows belong to, so a pending refresh of
    // the same search keeps known rows instead of replacing them with skeletons.
    let mut shown = use_signal(String::new);
    let mut shown_session = use_signal(String::new);
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
        let session = token(auth);
        if *shown.peek() != text || *shown_session.peek() != session {
            results.set(None);
        }
        error.set(String::new());
        if text.trim().is_empty() || session.is_empty() {
            loading.set(false);
            results.set(None);
            shown.set(String::new());
            shown_session.set(String::new());
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
                Ok(value) => {
                    results.set(Some(value));
                    shown.set(text.clone());
                    shown_session.set(session.clone());
                }
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
        if loading() && found.is_none() && error().is_empty() { TrackSkeletonRows {} }
        if !error().is_empty() { div { class: "alert error", role: "alert", span { "{error}" } button { class: "button secondary small", onclick: move |_| { let next=submitted.peek().wrapping_add(1); submitted.set(next); }, "Retry search" } } }
        if query().trim().is_empty() { div { class: "empty-state search-empty", Icon { name: "search" } h2 { "Find your next listen" } p { "Search your tracks, albums, artists and playlist names. Only music you can access appears here." } } }
        if let Some(found) = found {
            if count == 0 { div { class: "empty-state", h2 { "No search results" } p { "Try another title or name, or choose a different category." } } }
            if matches!(selected_category,"all"|"tracks") && !found.tracks.is_empty() { section { class: "search-result-section", "aria-label": "Track results", h2 { "Tracks" } for (at,track) in found.tracks.iter().cloned().enumerate() {
                { let context = found.tracks.clone(); let row = track.clone(); rsx! { article { class: "search-track-row", key: "{track.id}", button { class: "search-track-main", "aria-label": actions.play_label(&track.id, &track.title), onclick: move |_| actions.play.call((context.clone(),at,PlaybackContext::new(ContextKind::Search,"Search results"))), Art { track: row } div { strong { "{track.title}" } small { "{track.artist} · {track.album}" } } if actions.is_playing(&track.id) {Icon {name:"pause"}} else {Icon {name:"play"}} } TrackActions { track:track.clone(), actions } } } }
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
