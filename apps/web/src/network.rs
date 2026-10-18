//! Explicit, owner-controlled publication and bounded public catalog preview.
//! Owned originals use the root player; remote links never enter its private queue.
use crate::{
    client::{self, RequestScope},
    experience::Actions,
    filters::append_query,
    icons::Icon,
    session::{ContextKind, PlaybackContext},
    token,
};
use dioxus::prelude::*;
use gloo_timers::callback::Timeout;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{cell::Cell, rc::Rc};
use vela_domain::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{AbortController, AbortSignal};

#[derive(Clone, Default)]
struct Gate {
    requests: RequestScope,
    generation: Rc<Cell<u64>>,
}
impl Gate {
    fn cancel(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        self.requests.cancel();
    }
    fn begin(&self, session: String) -> Result<Ticket, String> {
        self.cancel();
        let signal = self.requests.begin()?;
        Ok(Ticket {
            session,
            generation: self.generation.get(),
            gate: self.clone(),
            signal,
            // The deadline owns this exact controller, never a newer request.
            controller: self.requests.controller(),
        })
    }
}

struct Ticket {
    session: String,
    generation: u64,
    gate: Gate,
    signal: AbortSignal,
    controller: Option<AbortController>,
}
impl Ticket {
    fn current(&self, ui: &Ui) -> bool {
        ui.alive.get()
            && !self.session.is_empty()
            && token(ui.auth) == self.session
            && *ui.session.peek() == self.session
            && self.gate.generation.get() == self.generation
    }
}

async fn request<T: DeserializeOwned>(
    ticket: &Ticket,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> Result<T, String> {
    let expired = Rc::new(Cell::new(false));
    let timed_out = expired.clone();
    let controller = ticket.controller.clone();
    let timer = Timeout::new(10_000, move || {
        timed_out.set(true);
        if let Some(controller) = controller {
            controller.abort();
        }
    });
    let result = client::api_abortable(
        method,
        path,
        body,
        if method == "GET" { "" } else { &ticket.session },
        Some(&ticket.signal),
    )
    .await;
    drop(timer);
    if expired.get() {
        if method == "GET" || path == "/api/network/preview" {
            Err("The request timed out. Check the server and try again.".into())
        } else {
            Err("The request timed out and its result is unknown. Refresh the server settings or your publications before trying the change again.".into())
        }
    } else {
        result
    }
}

#[derive(Clone, Default)]
struct ServerDraft {
    name: String,
    origin: String,
    enabled: bool,
}
impl From<&NetworkSettings> for ServerDraft {
    fn from(value: &NetworkSettings) -> Self {
        Self {
            name: value.name.clone(),
            origin: value.origin.clone().unwrap_or_default(),
            enabled: value.enabled,
        }
    }
}

#[derive(Clone, Default)]
struct PublicationDraft {
    selected: Option<Track>,
    license: String,
    attribution: String,
    evidence: String,
    stream: bool,
    download: bool,
    confirmed: bool,
}
impl PublicationDraft {
    fn license(&self) -> Option<PublicLicense> {
        match self.license.as_str() {
            "CC0-1.0" => Some(PublicLicense::Cc0),
            "CC-BY-4.0" => Some(PublicLicense::Attribution),
            "CC-BY-SA-4.0" => Some(PublicLicense::ShareAlike),
            _ => None,
        }
    }
    fn ready(&self) -> bool {
        self.selected.is_some()
            && self.license().is_some()
            && !self.attribution.trim().is_empty()
            && !self.evidence.trim().is_empty()
            && self.confirmed
            && (!self.download || self.stream)
    }
}

#[derive(Clone)]
struct RecordSnapshot {
    cursor: Option<String>,
    page: PublicationPage,
}

#[derive(Clone)]
struct PeerSnapshot {
    query: String,
    data: PeerPreview,
    origin: String,
}

#[derive(Clone)]
struct Ui {
    auth: Signal<Option<AuthResponse>>,
    session: Signal<String>,
    alive: Rc<Cell<bool>>,
    settings_gate: Gate,
    records_gate: Gate,
    search_gate: Gate,
    peer_gate: Gate,
    write_gate: Gate,
    settings: Signal<Option<NetworkSettings>>,
    server_draft: Signal<ServerDraft>,
    settings_loading: Signal<bool>,
    settings_error: Signal<String>,
    records: Signal<Option<RecordSnapshot>>,
    cursor: Signal<Option<String>>,
    records_loading: Signal<bool>,
    records_error: Signal<String>,
    search: Signal<String>,
    searched: Signal<Option<String>>,
    tracks: Signal<Option<TrackPage>>,
    search_loading: Signal<bool>,
    search_error: Signal<String>,
    draft: Signal<PublicationDraft>,
    peer_query: Signal<String>,
    peer: Signal<Option<PeerSnapshot>>,
    peer_loading: Signal<bool>,
    peer_error: Signal<String>,
    busy: Signal<bool>,
    write_error: Signal<String>,
    notice: Signal<String>,
    revoke: Signal<Option<Publication>>,
}
impl Ui {
    fn current(&self) -> bool {
        self.alive.get()
            && !self.session.peek().is_empty()
            && token(self.auth) == *self.session.peek()
    }
    fn editable(&self) -> bool {
        self.current() && !*self.busy.peek()
    }
    fn enabled(&self) -> bool {
        self.settings.peek().as_ref().is_some_and(|s| s.enabled)
            && !*self.settings_loading.peek()
            && self.settings_error.peek().is_empty()
    }
    fn record_actionable(&self, id: &str, revision: i64) -> bool {
        self.editable()
            && !*self.records_loading.peek()
            && self.records_error.peek().is_empty()
            && self.records.peek().as_ref().is_some_and(|snapshot| {
                snapshot.cursor == *self.cursor.peek()
                    && snapshot
                        .page
                        .items
                        .iter()
                        .any(|p| p.id == id && p.revision == revision)
            })
    }
    fn cancel(&self) {
        self.settings_gate.cancel();
        self.records_gate.cancel();
        self.search_gate.cancel();
        self.peer_gate.cancel();
        self.write_gate.cancel();
    }
    fn reset(&mut self, session: String) {
        self.cancel();
        self.session.set(session);
        self.settings.set(None);
        self.server_draft.set(ServerDraft::default());
        self.settings_loading.set(false);
        self.settings_error.set(String::new());
        self.records.set(None);
        self.cursor.set(None);
        self.records_loading.set(false);
        self.records_error.set(String::new());
        self.search.set(String::new());
        self.searched.set(None);
        self.tracks.set(None);
        self.search_loading.set(false);
        self.search_error.set(String::new());
        self.draft.set(PublicationDraft::default());
        self.peer_query.set(String::new());
        self.peer.set(None);
        self.peer_loading.set(false);
        self.peer_error.set(String::new());
        self.busy.set(false);
        self.write_error.set(String::new());
        self.notice.set(String::new());
        self.revoke.set(None);
    }
}

fn load_settings(mut ui: Ui) {
    if !ui.editable() {
        return;
    }
    ui.settings_error.set(String::new());
    let ticket = match ui.settings_gate.begin(token(ui.auth)) {
        Ok(ticket) => ticket,
        Err(error) => {
            ui.settings_error.set(error);
            return;
        }
    };
    ui.settings_loading.set(true);
    spawn_local(async move {
        if !ticket.current(&ui) {
            return;
        }
        let result =
            request::<NetworkSettings>(&ticket, "GET", "/api/network/settings", None).await;
        if !ticket.current(&ui) {
            return;
        }
        ui.settings_loading.set(false);
        match result {
            Ok(settings) => {
                ui.server_draft.set(ServerDraft::from(&settings));
                ui.settings.set(Some(settings));
            }
            Err(error) => ui.settings_error.set(error),
        }
    });
}

fn load_records(mut ui: Ui, cursor: Option<String>) {
    if !ui.editable() {
        return;
    }
    ui.cursor.set(cursor.clone());
    ui.records.set(None);
    ui.revoke.set(None);
    ui.records_error.set(String::new());
    let ticket = match ui.records_gate.begin(token(ui.auth)) {
        Ok(ticket) => ticket,
        Err(error) => {
            ui.records_error.set(error);
            return;
        }
    };
    ui.records_loading.set(true);
    spawn_local(async move {
        if !ticket.current(&ui) || *ui.cursor.peek() != cursor {
            return;
        }
        let mut path = "/api/publications?limit=20".to_string();
        if let Some(cursor) = &cursor {
            append_query(&mut path, "cursor", cursor);
        }
        let result = request::<PublicationPage>(&ticket, "GET", &path, None).await;
        if !ticket.current(&ui) || *ui.cursor.peek() != cursor {
            return;
        }
        ui.records_loading.set(false);
        match result {
            Ok(page) => ui.records.set(Some(RecordSnapshot { cursor, page })),
            Err(error) => ui.records_error.set(error),
        }
    });
}

fn search_tracks(mut ui: Ui) {
    if !ui.editable() {
        return;
    }
    let query = ui.search.peek().trim().to_string();
    ui.search_gate.cancel();
    ui.tracks.set(None);
    ui.search_error.set(String::new());
    ui.search_loading.set(false);
    if query.len() > 256 || query.chars().any(char::is_control) {
        ui.search_error
            .set("Use at most 256 bytes without control characters.".into());
        return;
    }
    ui.searched.set(Some(query.clone()));
    let ticket = match ui.search_gate.begin(token(ui.auth)) {
        Ok(ticket) => ticket,
        Err(error) => {
            ui.search_error.set(error);
            return;
        }
    };
    ui.search_loading.set(true);
    spawn_local(async move {
        if !ticket.current(&ui) || ui.search.peek().trim() != query {
            return;
        }
        let mut path = "/api/tracks?limit=20".to_string();
        if !query.is_empty() {
            append_query(&mut path, "q", &query);
        }
        let result = request::<TrackPage>(&ticket, "GET", &path, None).await;
        if !ticket.current(&ui)
            || ui.search.peek().trim() != query
            || ui.searched.peek().as_ref() != Some(&query)
        {
            return;
        }
        ui.search_loading.set(false);
        match result {
            Ok(page) => ui.tracks.set(Some(page)),
            Err(error) => ui.search_error.set(error),
        }
    });
}

fn valid_id(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.hyphenated().to_string() == value)
}

/// Remote metadata can never supply a complete audio URL or another protocol.
/// Construct links from a checked HTTPS root and a canonical publication UUID.
fn safe_origin(value: &str) -> Option<String> {
    if value
        .chars()
        .any(|ch| ch.is_control() || ch.is_whitespace())
    {
        return None;
    }
    let url = web_sys::Url::new(value).ok()?;
    let origin = url.origin();
    (url.protocol() == "https:"
        && url.username().is_empty()
        && url.password().is_empty()
        && url.port().is_empty()
        && url.pathname() == "/"
        && url.search().is_empty()
        && url.hash().is_empty()
        && value.trim_end_matches('/') == origin)
        .then_some(origin)
}

fn preview_origin(data: &PeerPreview) -> Option<String> {
    let origin = safe_origin(&data.capabilities.origin)?;
    (data.capabilities.protocol_version == 1
        && data.catalog.origin == data.capabilities.origin
        && valid_id(&data.capabilities.server_id)
        && data.catalog.server_id == data.capabilities.server_id
        && data.catalog.items.len() <= 20
        && data
            .catalog
            .items
            .iter()
            .all(|item| valid_id(&item.id) && (!item.can_download || item.can_stream)))
    .then_some(origin)
}

fn preview_peer(mut ui: Ui) {
    if !ui.current() || *ui.peer_loading.peek() {
        return;
    }
    let query = ui.peer_query.peek().trim().to_string();
    ui.peer.set(None);
    ui.peer_error.set(String::new());
    if query.len() > 2048 || safe_origin(&query).is_none() {
        ui.peer_error.set("Enter a public HTTPS origin, for example https://music.example.org, without a path, credentials, query or fragment.".into());
        return;
    }
    let ticket = match ui.peer_gate.begin(token(ui.auth)) {
        Ok(ticket) => ticket,
        Err(error) => {
            ui.peer_error.set(error);
            return;
        }
    };
    ui.peer_loading.set(true);
    spawn_local(async move {
        if !ticket.current(&ui) || ui.peer_query.peek().trim() != query {
            return;
        }
        let body = serde_json::to_value(PreviewPeer {
            origin: query.clone(),
        })
        .ok();
        let result = request::<PeerPreview>(&ticket, "POST", "/api/network/preview", body).await;
        if !ticket.current(&ui) || ui.peer_query.peek().trim() != query {
            return;
        }
        ui.peer_loading.set(false);
        match result {
            Ok(data) => match preview_origin(&data) {
                Some(origin) => ui.peer.set(Some(PeerSnapshot { query, data, origin })),
                None => ui.peer_error.set("The server returned an unsupported or inconsistent public catalog. No audio links were opened.".into()),
            },
            Err(error) => ui.peer_error.set(error),
        }
    });
}

#[derive(Clone)]
enum Change {
    Settings(UpdateNetworkSettings),
    Publish(CreatePublication),
    Revoke { id: String, revision: i64 },
}

fn change(mut ui: Ui, change: Change) {
    if !ui.editable() {
        return;
    }
    match &change {
        Change::Settings(input) => {
            if !ui
                .auth
                .peek()
                .as_ref()
                .is_some_and(|auth| auth.user.is_admin)
                || *ui.settings_loading.peek()
                || !ui.settings_error.peek().is_empty()
                || !ui
                    .settings
                    .peek()
                    .as_ref()
                    .is_some_and(|s| s.revision == input.revision)
            {
                return;
            }
        }
        Change::Publish(_) if !ui.enabled() || !ui.draft.peek().ready() => return,
        Change::Revoke { id, revision } if !ui.record_actionable(id, *revision) => return,
        _ => {}
    }
    let ticket = match ui.write_gate.begin(token(ui.auth)) {
        Ok(ticket) => ticket,
        Err(error) => {
            ui.write_error.set(error);
            return;
        }
    };
    // Ordinary read refreshes never cancel a write or discard its receipt.
    // Only account change, a replacement mutation, or teardown can invalidate it.
    ui.busy.set(true);
    ui.write_error.set(String::new());
    ui.notice.set(String::new());
    spawn_local(async move {
        if !ticket.current(&ui) {
            return;
        }
        let result: Result<Option<NetworkSettings>, String> = match &change {
            Change::Settings(input) => request::<NetworkSettings>(
                &ticket,
                "PUT",
                "/api/network/settings",
                serde_json::to_value(input).ok(),
            )
            .await
            .map(Some),
            Change::Publish(input) => request::<Publication>(
                &ticket,
                "POST",
                "/api/publications",
                serde_json::to_value(input).ok(),
            )
            .await
            .map(|_| None),
            Change::Revoke { id, revision } => request::<Publication>(
                &ticket,
                "POST",
                &format!("/api/publications/{id}/revoke"),
                serde_json::to_value(RevokePublication {
                    revision: *revision,
                })
                .ok(),
            )
            .await
            .map(|_| None),
        };
        if !ticket.current(&ui) {
            return;
        }
        ui.busy.set(false);
        match result {
            Ok(settings) => {
                if let Some(settings) = settings {
                    ui.server_draft.set(ServerDraft::from(&settings));
                    ui.settings.set(Some(settings));
                }
                match change {
                    Change::Settings(input) => {
                        ui.notice.set(if input.enabled {
                            "Public catalog settings saved. Only your explicitly published records are public; enabling the server does not publish any track.".into()
                        } else {
                            "Public catalog disabled. Existing publication records are retained, and new public requests are blocked.".into()
                        });
                    }
                    Change::Publish(_) => {
                        ui.draft.set(PublicationDraft::default());
                        ui.notice.set("Publication created with your declared license and selected audio permissions.".into());
                    }
                    Change::Revoke { .. } => {
                        ui.notice.set("Publication revoked. New public metadata and audio requests are blocked. Previously delivered copies cannot be recalled.".into());
                    }
                }
                ui.revoke.set(None);
                load_records(ui.clone(), None);
            }
            Err(error) => {
                ui.write_error.set(error);
                // Refresh is an explicit recovery action, never a silent retry.
            }
        }
    });
}

fn save_settings(ui: Ui) {
    if !ui.editable() {
        return;
    }
    let Some(settings) = ui.settings.peek().clone() else {
        return;
    };
    let draft = ui.server_draft.peek().clone();
    change(
        ui,
        Change::Settings(UpdateNetworkSettings {
            revision: settings.revision,
            enabled: draft.enabled,
            origin: draft.origin.trim().to_string(),
            name: draft.name.trim().to_string(),
        }),
    );
}

fn publish(mut ui: Ui) {
    if !ui.editable() {
        return;
    }
    let draft = ui.draft.peek().clone();
    let (Some(track), Some(license)) = (draft.selected.clone(), draft.license()) else {
        return;
    };
    let attribution = draft.attribution.trim();
    if attribution.is_empty()
        || attribution.len() > 2000
        || attribution.chars().any(|ch| ch.is_control() && ch != '\n')
    {
        ui.write_error.set("Use a public attribution of at most 2,000 bytes, without control characters other than line breaks.".into());
        return;
    }
    let evidence = draft.evidence.trim();
    let valid_evidence = web_sys::Url::new(evidence).is_ok_and(|url| {
        url.protocol() == "https:"
            && url.username().is_empty()
            && url.password().is_empty()
            && url.search().is_empty()
            && url.hash().is_empty()
    });
    if evidence.len() > 2048 || evidence.chars().any(char::is_control) || !valid_evidence {
        ui.write_error.set("Use an HTTPS rights evidence URL of at most 2,048 bytes, without credentials, query or fragment.".into());
        return;
    }
    change(
        ui,
        Change::Publish(CreatePublication {
            track_id: track.id,
            license,
            attribution: attribution.to_string(),
            rights_evidence: evidence.to_string(),
            allow_stream: draft.stream,
            allow_download: draft.download,
            rights_confirmed: draft.confirmed,
        }),
    );
}

fn settings_panel(ui: Ui, admin: bool) -> Element {
    let settings = ui.settings.read().clone();
    let draft = ui.server_draft.read().clone();
    let loading = *ui.settings_loading.read();
    let error = ui.settings_error.read().clone();
    let disabled = !ui.editable() || loading || !error.is_empty() || settings.is_none();
    let mut form = ui.server_draft;
    let save_ui = ui.clone();
    let refresh_ui = ui.clone();
    rsx! { section { class: "panel network-settings", "aria-label": "Server publication settings",
        div { class: "section-toolbar", div { span { class: "eyebrow", "THIS SERVER" } h2 { "Public catalog settings" } }
            button { class: "text-button", disabled: !ui.editable() || loading, onclick: move |_| load_settings(refresh_ui.clone()), "Refresh server settings" }
        }
        if loading { p { role: "status", "Loading server settings…" } }
        if !error.is_empty() { div { class: "alert error", role: "alert", "{error}" } }
        if let Some(settings) = settings {
            p { class: "network-server-state", "data-testid": "network-enabled-state", strong { if settings.enabled { "Public catalog enabled" } else { "Public catalog disabled" } }
                if loading || !error.is_empty() { small { "Last confirmed state; refresh to verify the current server policy." } }
            }
            p { "Enabling the server publishes no music. Every track needs its own explicit publication and license declaration." }
            if admin {
                form { onsubmit: move |event| { event.prevent_default(); save_settings(save_ui.clone()); },
                    label { r#for: "network-name", "Public server name" }
                    input { id: "network-name", required: true, maxlength: 80, value: "{draft.name}", disabled,
                        oninput: move |event| form.write().name = event.value() }
                    label { r#for: "network-origin", "Public HTTPS origin" }
                    input { id: "network-origin", r#type: "url", maxlength: 2048, placeholder: "https://music.example.org", value: "{draft.origin}", required: draft.enabled, disabled,
                        oninput: move |event| form.write().origin = event.value() }
                    small { "Use the public HTTPS root address, without a path, port, query or credentials. Changing it affects every active publication." }
                    label { class: "checkbox-label", input { r#type: "checkbox", checked: draft.enabled, disabled, "aria-label": "Enable public catalog",
                        onchange: move |event| form.write().enabled = event.checked() }
                        span { "Enable this server’s public catalog. Active publications and their selected audio grants become accessible without signing in." }
                    }
                    button { class: "button secondary", r#type: "submit", disabled, "Save public catalog settings" }
                }
            } else {
                p { "An administrator controls this server’s public catalog. Your library stays private until you explicitly publish individual tracks." }
                if let Some(origin) = settings.origin { p { class: "network-origin", "{origin}" } }
            }
        }
    } }
}

fn publish_panel(ui: Ui, actions: Actions) -> Element {
    let draft = ui.draft.read().clone();
    let query = ui.search.read().clone();
    let tracks = ui.tracks.read().clone();
    let search_loading = *ui.search_loading.read();
    let search_error = ui.search_error.read().clone();
    let blocked = !ui.editable();
    let can_publish = ui.enabled() && draft.ready() && !blocked;
    let mut form = ui.draft;
    let search_ui = ui.clone();
    let input_ui = ui.clone();
    let retry_ui = ui.clone();
    let publish_ui = ui.clone();
    let selected_ui = ui.clone();
    let clear_ui = ui.clone();
    rsx! { section { class: "panel network-publish", "aria-label": "Publish a track",
        span { class: "eyebrow", "A DELIBERATE CHOICE" } h2 { "Publish a track" }
        p { "Choose music you are authorized to share. Public metadata includes the title, artist, album, declared license and attribution. Audio permissions are separate." }
        if !ui.enabled() { p { class: "network-disabled-note", role: "status", "Publication is unavailable while this server’s public catalog is disabled or its settings are unconfirmed. You can still review your records below." } }
        form { class: "network-search", "aria-label": "Find an owned track to publish", onsubmit: move |event| { event.prevent_default(); search_tracks(search_ui.clone()); },
            label { r#for: "publication-search", "Search your tracks" }
            div { class: "network-inline-form", input { id: "publication-search", value: "{query}", maxlength: 256, placeholder: "Title, artist or album", disabled: blocked,
                oninput: move |event| {
                    let mut ui = input_ui.clone();
                    if ui.editable() {
                        ui.search_gate.cancel();
                        ui.search.set(event.value());
                        ui.searched.set(None);
                        ui.tracks.set(None);
                        ui.search_loading.set(false);
                        ui.search_error.set(String::new());
                    }
                } }
                button { class: "button secondary", r#type: "submit", disabled: blocked || search_loading, "Find my tracks" }
            }
        }
        if search_loading { p { role: "status", "Searching your owned tracks…" } }
        if !search_error.is_empty() { div { class: "alert error", role: "alert", "{search_error}" button { class: "text-button", disabled: blocked || search_loading, onclick: move |_| search_tracks(retry_ui.clone()), "Retry track search" } } }
        if let Some(tracks) = tracks {
            if tracks.items.is_empty() { p { "No owned tracks match. Try another search or add music to your library." } }
            else { div { class: "network-track-results", role: "group", "aria-label": "Owned track results",
                for track in tracks.items { {
                    let chosen = draft.selected.as_ref().is_some_and(|selected| selected.id == track.id);
                    let pick = track.clone();
                    let pick_ui = ui.clone();
                    rsx! { button { class: if chosen { "network-track-choice selected" } else { "network-track-choice" }, r#type: "button", disabled: blocked,
                        "aria-label": "Select {track.title} for publication", "aria-pressed": chosen, "data-track-id": "{track.id}",
                        onclick: move |_| {
                            let mut ui = pick_ui.clone();
                            if ui.editable() && ui.tracks.peek().as_ref().is_some_and(|page| page.items.iter().any(|t| t.id == pick.id)) {
                                ui.draft.write().selected = Some(pick.clone());
                                ui.draft.write().confirmed = false;
                            }
                        },
                        crate::Art { track: track.clone() } span { strong { "{track.title}" } small { "{track.artist} · {track.album}" } }
                        if chosen { Icon { name: "check" } }
                    } }
                } }
            } }
            if tracks.next_cursor.is_some() { small { "Showing the first 20 matching owned tracks. Narrow your search to find another track." } }
        }
        if let Some(track) = draft.selected.clone() {
            div { class: "network-selection", "data-testid": "publication-selected-track", strong { "Selected: {track.title}" } small { "{track.artist}" }
                div { class: "network-actions",
                    button { class: "text-button", r#type: "button", disabled: blocked, "aria-label": "Play my original {track.title}", onclick: move |_| {
                        if selected_ui.current() && selected_ui.draft.peek().selected.as_ref().is_some_and(|selected| selected.id == track.id) {
                            actions.play.call((vec![track.clone()], 0, PlaybackContext::new(ContextKind::Home, "My original · publication preview")));
                        }
                    }, Icon { name: "play" } "Play my original" }
                    button { class: "text-button", r#type: "button", disabled: blocked, onclick: move |_| {
                        let mut ui = clear_ui.clone();
                        if ui.editable() { ui.draft.write().selected = None; ui.draft.write().confirmed = false; }
                    }, "Clear selected track" }
                }
                small { "This plays your private original through your existing player. It does not test or grant public access." }
            }
        }
        form { class: "network-publication-form", onsubmit: move |event| { event.prevent_default(); publish(publish_ui.clone()); },
            label { r#for: "publication-license", "Declared public license" }
            select { id: "publication-license", required: true, value: "{draft.license}", disabled: blocked,
                onchange: move |event| { let mut draft = form.write(); draft.license = event.value(); draft.confirmed = false; },
                option { value: "", disabled: true, "Choose a license" }
                option { value: "CC0-1.0", "CC0 1.0 — Public domain dedication" }
                option { value: "CC-BY-4.0", "CC BY 4.0 — Attribution" }
                option { value: "CC-BY-SA-4.0", "CC BY-SA 4.0 — Attribution, share alike" }
            }
            small { "You declare the license; Vela does not verify copyright ownership. Creative Commons grants already received cannot be recalled by revoking a publication." }
            label { r#for: "publication-attribution", "Public attribution" }
            textarea { id: "publication-attribution", rows: 3, required: true, maxlength: 2000, value: "{draft.attribution}", placeholder: "Creator, title and required credit", disabled: blocked,
                oninput: move |event| { let mut draft = form.write(); draft.attribution = event.value(); draft.confirmed = false; } }
            label { r#for: "publication-evidence", "Rights evidence HTTPS URL" }
            input { id: "publication-evidence", r#type: "url", pattern: "https://.*", required: true, maxlength: 2048, value: "{draft.evidence}", placeholder: "https://example.org/license-evidence", disabled: blocked,
                oninput: move |event| { let mut draft = form.write(); draft.evidence = event.value(); draft.confirmed = false; } }
            small { "An owner-only source or license evidence URL, without credentials, query or fragment. Vela stores it for your record and never fetches or publishes it." }
            fieldset { class: "network-grants", disabled: blocked,
                legend { "Public audio permissions" }
                label { class: "checkbox-label", input { r#type: "checkbox", checked: draft.stream, "aria-label": "Allow public streaming",
                    onchange: move |event| { let mut draft = form.write(); draft.stream = event.checked(); if !draft.stream { draft.download = false; } draft.confirmed = false; } }
                    span { "Allow public streaming" }
                }
                label { class: "checkbox-label", input { r#type: "checkbox", checked: draft.download, disabled: !draft.stream, "aria-label": "Allow public downloads",
                    onchange: move |event| { let mut draft = form.write(); draft.download = event.checked() && draft.stream; draft.confirmed = false; } }
                    span { "Allow public downloads (requires streaming permission)" }
                }
                small { if draft.stream { "Streaming exposes audio publicly and listeners can retain delivered audio. The download grant controls the separate download endpoint." } else { "Metadata only by default. This publication will grant no public audio access." } }
            }
            label { class: "checkbox-label network-rights-confirm", input { r#type: "checkbox", checked: draft.confirmed, disabled: blocked, "aria-label": "Confirm publication rights",
                onchange: move |event| form.write().confirmed = event.checked() }
                span { "I have the rights to publish this track under the selected license, attribution and audio permissions." }
            }
            button { class: "button primary", r#type: "submit", disabled: !can_publish, "Create publication" }
        }
    } }
}

fn records_panel(ui: Ui, actions: Actions) -> Element {
    let loading = *ui.records_loading.read();
    let error = ui.records_error.read().clone();
    let records = ui
        .records
        .read()
        .clone()
        .filter(|snapshot| snapshot.cursor == *ui.cursor.peek());
    let confirm = ui.revoke.read().clone();
    let blocked = !ui.editable() || loading;
    let enabled = if *ui.settings_loading.peek() || !ui.settings_error.peek().is_empty() {
        None
    } else {
        ui.settings.peek().as_ref().map(|settings| settings.enabled)
    };
    let refresh_ui = ui.clone();
    let first_ui = ui.clone();
    let next_ui = ui.clone();
    let keep_ui = ui.clone();
    let revoke_ui = ui.clone();
    rsx! { section { class: "panel network-publications", "aria-label": "My publications",
        div { class: "section-toolbar", div { span { class: "eyebrow", "YOUR PUBLICATION RECORDS" } h2 { "My publications" } }
            button { class: "text-button", disabled: blocked, onclick: move |_| { let cursor = refresh_ui.cursor.peek().clone(); load_records(refresh_ui.clone(), cursor); }, "Refresh publications" }
        }
        p { "Only your records appear here, including revoked publications. Your private library and playlist sharing stay separate." }
        if loading { p { role: "status", "Loading your publications…" } }
        if !error.is_empty() { div { class: "alert error", role: "alert", "{error} Use Refresh publications to try again." } }
        if let Some(snapshot) = records {
            if snapshot.page.items.is_empty() { div { class: "network-empty", h3 { "No publication records on this page" } p { "Tracks stay private until you deliberately create a publication. Nothing is published automatically." } } }
            else { div { class: "network-records", for record in snapshot.page.items { {
                let play_ui = ui.clone();
                let confirm_ui = ui.clone();
                let play = record.track.clone();
                let id = record.id.clone();
                let revision = record.revision;
                let revoke = record.clone();
                let status = if record.revoked_at.is_some() { "Revoked" } else { match enabled {
                    Some(true) => "Published", Some(false) => "Hidden while public catalog is disabled", None => "Public status unconfirmed",
                } };
                rsx! { article { class: "network-record", key: "{record.id}", "data-publication-id": "{record.id}", "data-revision": "{record.revision}",
                    div { class: "network-record-heading", crate::Art { track: record.track.clone() }
                        div { h3 { "{record.track.title}" } p { "{record.track.artist} · {record.track.album}" } }
                        span { class: "network-status", "{status}" }
                    }
                    dl { dt { "Declared license" } dd { a { href: record.license.url(), target: "_blank", rel: "noopener noreferrer", "{record.license.id()}" } }
                        dt { "Audio grants" } dd { if record.allow_download { "Streaming and downloads" } else if record.allow_stream { "Streaming only" } else { "Metadata only" } }
                        dt { "Attribution" } dd { class: "network-attribution", "{record.attribution}" }
                    }
                    details { class: "network-record-details", summary { "Publication details" }
                        dl { dt { "Publication ID" } dd { "{record.id}" } dt { "Revision" } dd { "{record.revision}" }
                            dt { "Rights evidence (owner only)" } dd { "{record.rights_evidence}" }
                            dt { "Created" } dd { "{created_label(record.created_at)}" }
                            if let Some(revoked_at) = record.revoked_at { dt { "Revoked" } dd { "{created_label(revoked_at)}" } }
                        }
                    }
                    div { class: "network-actions",
                        button { class: "text-button", disabled: blocked || !error.is_empty(), "aria-label": "Play my original {record.track.title}", onclick: move |_| {
                            if play_ui.record_actionable(&id, revision) {
                                actions.play.call((vec![play.clone()], 0, PlaybackContext::new(ContextKind::Home, "My original · publication record")));
                            }
                        }, Icon { name: "play" } "Play my original" }
                        if record.revoked_at.is_none() { button { class: "text-button network-revoke", disabled: blocked || !error.is_empty(), "aria-label": "Revoke publication {record.track.title}", onclick: move |_| {
                            let mut ui = confirm_ui.clone();
                            if ui.record_actionable(&revoke.id, revoke.revision) { ui.revoke.set(Some(revoke.clone())); }
                        }, "Revoke…" } }
                    }
                } }
            } } } }
            div { class: "network-pagination",
                button { class: "button secondary small", disabled: blocked || ui.cursor.peek().is_none(), onclick: move |_| load_records(first_ui.clone(), None), "First publication page" }
                if let Some(cursor) = snapshot.page.next_cursor { button { class: "button secondary small", disabled: blocked, onclick: move |_| load_records(next_ui.clone(), Some(cursor.clone())), "Next publication page" } }
                small { "Up to 20 records per page" }
            }
        }
        if let Some(record) = confirm {
            if ui.record_actionable(&record.id, record.revision) || *ui.busy.peek() { div { class: "network-revoke-confirm", role: "group", "aria-label": "Confirm publication revocation",
                h3 { "Revoke {record.track.title}?" }
                p { "Block new public metadata, streaming and download requests for this publication. Your private original stays available. Previously delivered copies and license grants cannot be recalled." }
                div { class: "network-actions",
                    button { class: "button danger", disabled: blocked, onclick: move |_| change(revoke_ui.clone(), Change::Revoke { id: record.id.clone(), revision: record.revision }), "Confirm revoke publication" }
                    button { class: "text-button", disabled: !ui.editable(), onclick: move |_| { let mut ui = keep_ui.clone(); if ui.editable() { ui.revoke.set(None); } }, "Keep publication" }
                }
            } }
        }
    } }
}

fn created_label(timestamp: i64) -> String {
    js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(timestamp as f64 * 1000.0))
        .to_locale_string("en-GB", &js_sys::Object::new())
        .as_string()
        .unwrap_or_else(|| "Date unavailable".into())
}

fn peer_panel(ui: Ui) -> Element {
    let query = ui.peer_query.read().clone();
    let loading = *ui.peer_loading.read();
    let error = ui.peer_error.read().clone();
    let peer = ui
        .peer
        .read()
        .clone()
        .filter(|snapshot| snapshot.query == query.trim());
    let input_ui = ui.clone();
    let preview_ui = ui.clone();
    let retry_ui = ui.clone();
    rsx! { section { class: "panel network-peers", "aria-label": "Preview a public server",
        span { class: "eyebrow", "A WINDOW INTO ANOTHER CATALOG" } h2 { "Preview a public server" }
        p { "Enter a compatible Vela public HTTPS origin to request its declared public metadata. Previewing does not import music or send your private library or listening history." }
        form { onsubmit: move |event| { event.prevent_default(); preview_peer(preview_ui.clone()); },
            label { r#for: "network-peer-origin", "Public server HTTPS origin" }
            div { class: "network-inline-form", input { id: "network-peer-origin", r#type: "url", required: true, maxlength: 2048, placeholder: "https://music.example.org", value: "{query}", disabled: !ui.current(),
                oninput: move |event| {
                    let mut ui = input_ui.clone();
                    if ui.current() {
                        ui.peer_gate.cancel();
                        ui.peer_query.set(event.value());
                        ui.peer.set(None);
                        ui.peer_loading.set(false);
                        ui.peer_error.set(String::new());
                    }
                } }
                button { class: "button secondary", r#type: "submit", disabled: !ui.current() || loading, "Preview public server" }
            }
        }
        if loading { p { role: "status", "Requesting this server’s public catalog…" } }
        if !error.is_empty() { div { class: "alert error", role: "alert", "{error}" button { class: "text-button", disabled: !ui.current() || loading, onclick: move |_| preview_peer(retry_ui.clone()), "Retry public server preview" } } }
        if let Some(peer) = peer {
            div { class: "network-peer-result", "data-testid": "network-peer-result",
                h3 { "{peer.data.capabilities.name}" } p { class: "network-origin", "{peer.origin}" }
                small { "Read-only Vela catalog profile, version 1 · Up to 20 public records" }
                if peer.data.catalog.items.is_empty() { p { "This server returned no public tracks. Private, revoked or unavailable publications are not listed." } }
                else { div { class: "network-peer-tracks", for track in peer.data.catalog.items { {
                    let stream = format!("{}/api/public/media/{}/stream", peer.origin, track.id);
                    let download = format!("{}/api/public/media/{}/download", peer.origin, track.id);
                    rsx! { article { class: "network-peer-track", "data-public-track-id": "{track.id}",
                        h4 { "{track.title}" } p { "{track.artist} · {track.album}" }
                        a { class: "network-license", href: track.license.url(), target: "_blank", rel: "noopener noreferrer", "{track.license.id()}" }
                        p { class: "network-attribution", "{track.attribution}" }
                        div { class: "network-actions",
                            if track.can_stream { a { class: "button secondary small", href: stream, target: "_blank", rel: "noopener noreferrer", "referrerpolicy": "no-referrer", "aria-label": "Open public stream for {track.title}", "Open public stream ↗" } }
                            if track.can_download && track.can_stream { a { class: "button secondary small", href: download, target: "_blank", rel: "noopener noreferrer", "referrerpolicy": "no-referrer", "aria-label": "Open public download for {track.title}", "Open download ↗" } }
                            if !track.can_stream { small { "Metadata only · No public audio grant" } }
                        }
                    } }
                } } } }
                if peer.data.catalog.next_cursor.is_some() { p { "This preview shows the first 20 public records; it does not crawl or cache the remote catalog." } }
                p { class: "network-peer-note", "Audio opens directly on the public server in another tab. Its permissions are rechecked there and may have changed. Remote tracks are not added to your private playback queue." }
            }
        }
    } }
}

#[component]
pub fn Network(auth: Signal<Option<AuthResponse>>, actions: Actions) -> Element {
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let ui = Ui {
        auth,
        session: use_signal(String::new),
        alive,
        settings_gate: use_hook(Gate::default),
        records_gate: use_hook(Gate::default),
        search_gate: use_hook(Gate::default),
        peer_gate: use_hook(Gate::default),
        write_gate: use_hook(Gate::default),
        settings: use_signal(|| None),
        server_draft: use_signal(ServerDraft::default),
        settings_loading: use_signal(|| false),
        settings_error: use_signal(String::new),
        records: use_signal(|| None),
        cursor: use_signal(|| None),
        records_loading: use_signal(|| false),
        records_error: use_signal(String::new),
        search: use_signal(String::new),
        searched: use_signal(|| None),
        tracks: use_signal(|| None),
        search_loading: use_signal(|| false),
        search_error: use_signal(String::new),
        draft: use_signal(PublicationDraft::default),
        peer_query: use_signal(String::new),
        peer: use_signal(|| None),
        peer_loading: use_signal(|| false),
        peer_error: use_signal(String::new),
        busy: use_signal(|| false),
        write_error: use_signal(String::new),
        notice: use_signal(String::new),
        revoke: use_signal(|| None),
    };
    let cleanup = ui.clone();
    use_drop(move || {
        cleanup.alive.set(false);
        cleanup.cancel();
    });
    let effect = ui.clone();
    use_effect(move || {
        let _auth = auth.read();
        let session = token(auth);
        let mut ui = effect.clone();
        if *ui.session.peek() == session {
            return;
        }
        ui.reset(session.clone());
        if !session.is_empty() {
            load_settings(ui.clone());
            load_records(ui.clone(), None);
            search_tracks(ui);
        }
    });
    let admin = auth
        .read()
        .as_ref()
        .is_some_and(|value| value.user.is_admin);
    let _session = ui.session.read();
    let busy = *ui.busy.read();
    let write_error = ui.write_error.read().clone();
    let notice = ui.notice.read().clone();
    rsx! { section { class: "network-page", "aria-label": "Network",
        div { class: "network-hero", Icon { name: "network" } div { span { class: "eyebrow", "PRIVATE BY DEFAULT · SHARED BY CHOICE" } h2 { "Music you choose to share" }
            p { "Publish individual tracks with clear rights, or preview another server’s public catalog. Your account, library and listening history remain private." }
        } }
        if ui.current() {
            if busy { p { class: "network-saving", role: "status", "Saving your publication change…" } }
            if !write_error.is_empty() { div { class: "alert error", role: "alert", "{write_error}" } }
            if !notice.is_empty() { div { class: "alert success", role: "status", "{notice}" } }
            {settings_panel(ui.clone(), admin)}
            {publish_panel(ui.clone(), actions)}
            {records_panel(ui.clone(), actions)}
            {peer_panel(ui.clone())}
            p { class: "network-scope", "Publications and read-only catalog preview are available here. Signed federation and native network playback are not implemented." }
        } else { p { role: "status", "Loading this account’s network controls…" } }
    } }
}
