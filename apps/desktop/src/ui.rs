//! Native presentation. HTTP credentials and the one audio output stay in Rust.
use crate::{
    AudioPhase, AudioSnapshot, DownloadProgress, PreparedAudio,
    audio::AudioEngine,
    client::VelaClient,
    mpris::{self, Mpris, RepeatKind as SystemRepeat},
    playback::{Command, MAX_QUEUE, PlaybackOrder, Repeat, Transition},
};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use vela_domain::{AlbumSummary, ArtistSummary, PublicTrack, Track, TrackPage, User};

#[path = "../../web/src/icons.rs"]
mod icons;
use icons::Icon;

#[derive(Clone)]
struct Lifetime(Arc<AtomicBool>);
impl Lifetime {
    fn get(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    fn set(&self, value: bool) {
        self.0.store(value, Ordering::Release);
    }
}

#[derive(Clone, Default)]
struct Gate {
    generation: Rc<Cell<u64>>,
    cancel: Rc<std::cell::RefCell<Option<CancellationToken>>>,
}
impl Gate {
    fn cancel(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        if let Some(cancel) = self.cancel.borrow_mut().take() {
            cancel.cancel();
        }
    }
    fn begin(&self, epoch: u64) -> Ticket {
        self.cancel();
        let cancel = CancellationToken::new();
        *self.cancel.borrow_mut() = Some(cancel.clone());
        Ticket {
            epoch,
            generation: self.generation.get(),
            gate: self.clone(),
            cancel,
        }
    }
}
#[derive(Clone)]
struct Ticket {
    epoch: u64,
    generation: u64,
    gate: Gate,
    cancel: CancellationToken,
}
impl Ticket {
    fn current(&self, ui: &Ui) -> bool {
        ui.alive.get()
            && !self.cancel.is_cancelled()
            && *ui.epoch.peek() == self.epoch
            && self.gate.generation.get() == self.generation
    }
}

#[derive(Clone)]
struct Account {
    epoch: u64,
    client: VelaClient,
    user: User,
    origin: String,
}
#[derive(Clone)]
struct Catalog {
    epoch: u64,
    query: String,
    cursor: Option<String>,
    favorites: bool,
    page: Arc<TrackPage>,
}

#[derive(Clone)]
struct Ui {
    alive: Lifetime,
    audio: Signal<Option<AudioEngine>>,
    audio_error: Signal<String>,
    engine_cleanup: Rc<RefCell<Option<AudioEngine>>>,
    login_gate: Gate,
    catalog_gate: Gate,
    feedback_gate: Gate,
    playlist_gate: Gate,
    playlist_write_gate: Gate,
    folder_gate: Gate,
    browse_gate: Gate,
    remote_gate: Gate,
    offline_gate: Gate,
    source_gate: Gate,
    logout_gate: Gate,
    epoch: Signal<u64>,
    selection: Signal<u64>,
    generation_clock: Rc<Cell<u64>>,
    account: Signal<Option<Account>>,
    origin: Signal<String>,
    username: Signal<String>,
    password: Signal<String>,
    connecting: Signal<bool>,
    resuming: Signal<bool>,
    remember: Signal<bool>,
    saved_session: Signal<Option<crate::session_store::SavedSession>>,
    login_error: Signal<String>,
    view: Signal<&'static str>,
    theme: Signal<&'static str>,
    query_draft: Signal<String>,
    query: Signal<String>,
    cursor: Signal<Option<String>>,
    catalog: Signal<Option<Catalog>>,
    catalog_loading: Signal<bool>,
    catalog_error: Signal<String>,
    favorite_busy: Signal<String>,
    playlists: Signal<Vec<vela_domain::PlaylistSummary>>,
    playlist: Signal<Option<vela_domain::PlaylistDetail>>,
    playlist_id: Signal<Option<String>>,
    playlist_loading: Signal<bool>,
    playlist_error: Signal<String>,
    playlist_name_draft: Signal<String>,
    folders: Signal<Vec<vela_domain::PlaylistFolder>>,
    folder_filter: Signal<Option<String>>,
    folder_name_draft: Signal<String>,
    folder_rename_draft: Signal<String>,
    folder_busy: Signal<bool>,
    folder_confirm: Signal<String>,
    playlist_folder_target: Signal<String>,
    playlist_candidates: Signal<Vec<Track>>,
    playlist_candidate: Signal<String>,
    playlist_busy: Signal<bool>,
    playlist_delete_confirm: Signal<bool>,
    browse_kind: Signal<&'static str>,
    albums: Signal<Vec<AlbumSummary>>,
    artists: Signal<Vec<ArtistSummary>>,
    browse_tracks: Signal<Vec<Track>>,
    browse_id: Signal<Option<String>>,
    browse_title: Signal<String>,
    browse_loading: Signal<bool>,
    browse_error: Signal<String>,
    remote_origin: Signal<String>,
    remote_preview: Signal<Option<crate::remote::RemotePreview>>,
    remote_loading: Signal<bool>,
    remote_error: Signal<String>,
    remote_now: Signal<Option<crate::remote::RemoteNow>>,
    remote_queue: Signal<Vec<crate::remote::RemoteItem>>,
    remote_index: Signal<usize>,
    remote_query: Signal<String>,
    remote_results: Signal<Vec<crate::remote::PeerSearch>>,
    remote_searching: Signal<bool>,
    remote_search_error: Signal<String>,
    remote_more_busy: Signal<bool>,
    remote_more_error: Signal<String>,
    remote_peer_more_busy: Signal<String>,
    remote_peer_more_error: Signal<String>,
    remote_retry_busy: Signal<String>,
    remote_retry_error: Signal<String>,
    offline: Signal<Vec<crate::offline::OfflineTrack>>,
    offline_busy: Signal<String>,
    offline_error: Signal<String>,
    offline_corrupt: Signal<Vec<String>>,
    offline_active: Signal<bool>,
    offline_confirm: Signal<String>,
    saved_peers: Signal<Vec<String>>,
    queue: Signal<Vec<Track>>,
    order: Signal<PlaybackOrder>,
    shuffle: Signal<bool>,
    repeat: Signal<Repeat>,
    context: Signal<String>,
    snapshot: Signal<AudioSnapshot>,
    preparing: Signal<bool>,
    progress: Signal<DownloadProgress>,
    prepared: Signal<Option<Arc<PreparedAudio>>>,
    source_error: Signal<String>,
    notice: Signal<String>,
    ended: Rc<Cell<Option<u64>>>,
    mpris: Mpris,
    mpris_note: Signal<&'static str>,
    mpris_seek_serial: Signal<u64>,
}
impl Ui {
    fn signed_in(&self) -> bool {
        self.alive.get()
            && self
                .account
                .peek()
                .as_ref()
                .is_some_and(|account| account.epoch == *self.epoch.peek())
    }
    fn audio_alive(&self) -> bool {
        self.audio.read().as_ref().is_some_and(AudioEngine::alive)
    }
    fn audio_ready(&self) -> bool {
        self.audio_alive()
    }
    fn current_track(&self) -> Option<Track> {
        if !self.signed_in() {
            return None;
        }
        self.order
            .peek()
            .current()
            .and_then(|at| self.queue.peek().get(at).cloned())
    }
    fn source_current(&self, ticket: &Ticket, selection: u64, id: &str) -> bool {
        ticket.current(self)
            && self.signed_in()
            && *self.selection.peek() == selection
            && self.current_track().is_some_and(|track| track.id == id)
    }
    fn cancel_all(&self) {
        self.login_gate.cancel();
        self.catalog_gate.cancel();
        self.feedback_gate.cancel();
        self.playlist_gate.cancel();
        self.playlist_write_gate.cancel();
        self.browse_gate.cancel();
        self.remote_gate.cancel();
        self.source_gate.cancel();
        self.logout_gate.cancel();
    }
    fn new_selection(&mut self) -> u64 {
        self.source_gate.cancel();
        let selection = self.generation_clock.get().wrapping_add(1);
        self.generation_clock.set(selection);
        self.selection.set(selection);
        self.ended.set(None);
        self.preparing.set(false);
        self.progress.set(DownloadProgress::default());
        self.source_error.set(String::new());
        let volume = self.snapshot.peek().volume;
        self.snapshot.set(AudioSnapshot {
            generation: selection,
            volume,
            ..AudioSnapshot::default()
        });
        if let Some(audio) = self.audio.peek().clone()
            && let Err(error) = audio.stop(selection)
        {
            self.source_error.set(error);
        }
        selection
    }
    fn clear_account(&mut self) {
        self.cancel_all();
        let epoch = self.epoch.peek().wrapping_add(1);
        self.epoch.set(epoch);
        self.new_selection();
        self.account.set(None);
        // The in-memory copy of the saved session ends with the account. The
        // keyring item is deleted separately and never on this thread (AUD-02).
        self.saved_session.set(None);
        self.password.set(String::new());
        self.connecting.set(false);
        self.catalog.set(None);
        self.query.set(String::new());
        self.query_draft.set(String::new());
        self.cursor.set(None);
        self.catalog_loading.set(false);
        self.catalog_error.set(String::new());
        self.favorite_busy.set(String::new());
        self.playlists.set(Vec::new());
        self.playlist.set(None);
        self.playlist_id.set(None);
        self.playlist_loading.set(false);
        self.playlist_error.set(String::new());
        self.playlist_name_draft.set(String::new());
        self.folders.set(Vec::new());
        self.folder_filter.set(None);
        self.folder_name_draft.set(String::new());
        self.folder_rename_draft.set(String::new());
        self.folder_busy.set(false);
        self.folder_confirm.set(String::new());
        self.playlist_folder_target.set(String::new());
        self.playlist_candidates.set(Vec::new());
        self.playlist_candidate.set(String::new());
        self.playlist_busy.set(false);
        self.playlist_delete_confirm.set(false);
        self.albums.set(Vec::new());
        self.artists.set(Vec::new());
        self.browse_tracks.set(Vec::new());
        self.browse_id.set(None);
        self.browse_title.set(String::new());
        self.browse_loading.set(false);
        self.browse_error.set(String::new());
        self.remote_preview.set(None);
        self.remote_loading.set(false);
        self.remote_error.set(String::new());
        self.remote_now.set(None);
        self.remote_queue.set(Vec::new());
        self.remote_index.set(0);
        self.remote_query.set(String::new());
        self.remote_results.set(Vec::new());
        self.remote_searching.set(false);
        self.remote_search_error.set(String::new());
        self.remote_more_busy.set(false);
        self.remote_more_error.set(String::new());
        self.remote_peer_more_busy.set(String::new());
        self.remote_peer_more_error.set(String::new());
        self.remote_retry_busy.set(String::new());
        self.remote_retry_error.set(String::new());
        self.offline.set(Vec::new());
        self.offline_busy.set(String::new());
        self.offline_error.set(String::new());
        self.offline_corrupt.set(Vec::new());
        self.offline_active.set(false);
        self.offline_confirm.set(String::new());
        self.queue.set(Vec::new());
        self.order.set(PlaybackOrder::default());
        self.shuffle.set(false);
        self.repeat.set(Repeat::Off);
        self.context.set(String::new());
        self.prepared.set(None);
        self.source_error.set(String::new());
        self.notice.set(String::new());
        self.view.set("library");
    }
    /// Publish a bounded, credential-free snapshot to the OS media session.
    /// Ranking, HTTP and audio stay owned by this UI; MPRIS only observes.
    fn publish_system_controls(&self) {
        if self.remote_now.peek().is_some() {
            self.mpris.publish(mpris::Update {
                status: mpris::Status::Stopped,
                track: None,
                position_ms: 0,
                volume: self.snapshot.peek().volume as f64,
                shuffle: *self.shuffle.peek(),
                repeat: SystemRepeat::Off,
                can_play: false,
                can_pause: false,
                can_seek: false,
                can_go_next: false,
                can_go_previous: false,
                seek_serial: *self.mpris_seek_serial.peek(),
            });
            return;
        }
        let track = self.current_track();
        let snapshot = self.snapshot.peek().clone();
        let preparing = *self.preparing.peek();
        let repeat = match *self.repeat.peek() {
            Repeat::Off => SystemRepeat::Off,
            Repeat::All => SystemRepeat::All,
            Repeat::One => SystemRepeat::One,
        };
        let status = if track.is_none() {
            mpris::Status::Stopped
        } else if snapshot.phase == AudioPhase::Playing {
            mpris::Status::Playing
        } else if preparing || matches!(snapshot.phase, AudioPhase::Paused | AudioPhase::Loading) {
            mpris::Status::Paused
        } else {
            mpris::Status::Stopped
        };
        let can_play = track.is_some() && self.audio_alive();
        let can_seek = can_play
            && self.prepared.peek().is_some()
            && snapshot.duration_ms > 0
            && matches!(
                snapshot.phase,
                AudioPhase::Playing | AudioPhase::Paused | AudioPhase::Ended
            );
        let queue_len = self.queue.peek().len();
        self.mpris.publish(mpris::Update {
            status,
            track: track.map(|track| mpris::Track {
                id: track.id,
                title: track.title,
                artist: track.artist,
                album: track.album,
                duration_ms: track.duration_ms.max(0) as u64,
            }),
            position_ms: if can_play { snapshot.position_ms } else { 0 },
            volume: snapshot.volume as f64,
            shuffle: *self.shuffle.peek(),
            repeat,
            can_play,
            can_pause: can_play
                && matches!(snapshot.phase, AudioPhase::Playing | AudioPhase::Paused),
            can_seek,
            can_go_next: queue_len > 1 || repeat != SystemRepeat::Off,
            can_go_previous: queue_len > 1 || repeat == SystemRepeat::All,
            seek_serial: *self.mpris_seek_serial.peek(),
        });
    }
    fn catalog_current(&self, catalog: &Catalog) -> bool {
        self.signed_in()
            && catalog.epoch == *self.epoch.peek()
            && catalog.query == *self.query.peek()
            && catalog.cursor == *self.cursor.peek()
            && catalog.favorites == (*self.view.peek() == "favorites")
            && !*self.catalog_loading.peek()
            && self.catalog_error.peek().is_empty()
    }
}

fn default_origin() -> String {
    let fallback = "http://127.0.0.1:3210";
    let value = std::env::var("VELA_DESKTOP_SERVER_URL").unwrap_or_else(|_| fallback.into());
    // Use the same canonical-origin policy as the HTTP client so a malformed
    // environment value can never prefill a form that the client would reject.
    if value.len() <= 2048 && crate::client::canonical_origin(&value).is_ok() {
        value
    } else {
        fallback.into()
    }
}

fn login(mut ui: Ui) {
    if !ui.alive.get() || ui.signed_in() || *ui.connecting.peek() {
        return;
    }
    if ui.audio.peek().is_none() && ui.audio_error.peek().is_empty() {
        return;
    }
    // A previous server sign-out cannot replace feedback for this new attempt.
    ui.logout_gate.cancel();
    let origin = ui.origin.peek().trim().to_string();
    let username = ui.username.peek().trim().to_string();
    let password = ui.password.peek().clone();
    let remember = *ui.remember.peek();
    if origin.is_empty() || username.is_empty() || password.is_empty() {
        ui.login_error
            .set("Enter your server origin, username and password.".into());
        return;
    }
    let ticket = ui.login_gate.begin(*ui.epoch.peek());
    // AUD-01: this attempt claims the saved item. An older attempt may no longer
    // write it, even while its helper process is still running.
    let attempt = crate::session_store::begin_attempt();
    ui.password.set(String::new());
    ui.connecting.set(true);
    ui.login_error.set(String::new());
    ui.notice.set(String::new());
    spawn(async move {
        if !ticket.current(&ui) {
            return;
        }
        let result =
            VelaClient::connect(&origin, username.clone(), password, ticket.cancel.clone()).await;
        if !login_still_current(&ui, &ticket, &origin, &username) {
            return;
        }
        ui.connecting.set(false);
        match result {
            Ok((client, auth)) => {
                let token = client.session_token();
                if remember {
                    let save_origin = origin.clone();
                    let stored = tokio::task::spawn_blocking(move || {
                        crate::session_store::save(attempt, &save_origin, &token)
                    })
                    .await;
                    // AUD-01: the keyring may outlive this wait. A canceled or
                    // replaced login publishes no feedback and adopts nothing.
                    if !login_still_current(&ui, &ticket, &origin, &username) {
                        return;
                    }
                    match stored {
                        Ok(Ok(crate::session_store::StoreStatus::Stored)) => {
                            ui.notice.set("Session saved on this device.".into());
                        }
                        // A newer attempt owns the item; it reports for itself.
                        Ok(Ok(crate::session_store::StoreStatus::Superseded)) => {}
                        Ok(Err(error)) => ui.notice.set(error.message().to_owned()),
                        Err(_) => ui
                            .notice
                            .set("The session could not be saved on this device.".into()),
                    }
                }
                // AUD-01: adoption and every later UI mutation is guarded again.
                if !login_still_current(&ui, &ticket, &origin, &username) {
                    return;
                }
                adopt_account(ui, client, auth, origin, ticket.epoch);
            }
            Err(error) => ui.login_error.set(error),
        }
    });
}

/// AUD-01: the account may only be adopted while this exact login attempt still
/// owns the UI: same ticket/epoch and the same origin and username the attempt
/// was started with.
fn login_still_current(ui: &Ui, ticket: &Ticket, origin: &str, username: &str) -> bool {
    ticket.current(ui) && ui.origin.peek().trim() == origin && ui.username.peek().trim() == username
}

fn adopt_account(
    mut ui: Ui,
    client: crate::client::VelaClient,
    auth: vela_domain::AuthResponse,
    origin: String,
    epoch: u64,
) {
    let account_id = auth.user.id.clone();
    ui.account.set(Some(Account {
        epoch,
        client,
        user: auth.user,
        origin,
    }));
    ui.offline.set(Vec::new());
    let mut offline_ui = ui.clone();
    spawn(async move {
        let loaded = tokio::task::spawn_blocking({
            let account_id = account_id.clone();
            move || crate::offline::load(&account_id)
        })
        .await;
        if offline_ui
            .account
            .peek()
            .as_ref()
            .map(|account| account.user.id.as_str())
            != Some(account_id.as_str())
        {
            return;
        }
        match loaded {
            Ok(Ok(tracks)) => offline_ui.offline.set(tracks),
            Ok(Err(error)) => offline_ui.offline_error.set(error),
            Err(_) => offline_ui
                .offline_error
                .set("Saved offline music could not be read.".into()),
        }
        if let Ok(Ok(report)) = tokio::task::spawn_blocking({
            let account_id = offline_ui
                .account
                .peek()
                .as_ref()
                .map(|account| account.user.id.clone())
                .unwrap_or_default();
            move || crate::offline::verify(&account_id)
        })
        .await
            && offline_ui
                .account
                .peek()
                .as_ref()
                .map(|account| account.user.id.as_str())
                == Some(account_id.as_str())
        {
            if !report.corrupt.is_empty() {
                offline_ui.notice.set(format!(
                    "{} offline copy(ies) failed their integrity check. Save them again.",
                    report.corrupt.len()
                ));
            }
            offline_ui.offline_corrupt.set(report.corrupt);
        }
    });
    load_catalog(ui, String::new(), None, false);
}

fn resume_saved(mut ui: Ui) {
    if !ui.alive.get()
        || ui.signed_in()
        || *ui.connecting.peek()
        || *ui.resuming.peek()
        || (ui.audio.peek().is_none() && ui.audio_error.peek().is_empty())
    {
        return;
    }
    let Some(saved) = ui.saved_session.peek().clone() else {
        return;
    };
    let ticket = ui.login_gate.begin(*ui.epoch.peek());
    // AUD-01: this resume claims the saved item; a stale attempt may not delete
    // an item that a newer login already wrote.
    let attempt = crate::session_store::begin_attempt();
    ui.resuming.set(true);
    ui.login_error.set(String::new());
    ui.notice.set(String::new());
    spawn(async move {
        let result =
            crate::client::VelaClient::resume(&saved.origin, &saved.token, ticket.cancel.clone())
                .await;
        // AUD-01: a canceled or superseded resume neither reports nor adopts.
        if !ticket.current(&ui) || ui.signed_in() {
            return;
        }
        ui.resuming.set(false);
        match result {
            Ok((client, auth)) => {
                // AUD-01: re-checked once more before the account is adopted.
                if !ticket.current(&ui) || ui.signed_in() {
                    return;
                }
                adopt_account(ui, client, auth, saved.origin, ticket.epoch);
            }
            Err(error) => {
                let failure = saved_session_failure(&error);
                if !failure.deletes_saved_item() {
                    // AUD-04: a timeout, a network/DNS failure, a 5xx or a
                    // malformed record must keep the saved item so the listener
                    // can retry instead of losing a valid login.
                    if !matches!(failure, SavedSessionFailure::Canceled) {
                        ui.login_error.set(failure.message(&error));
                    }
                    return;
                }
                // Genuine rejection (HTTP 401 / expired session): the stored
                // token can never authenticate again. Drop the in-memory copy
                // first so the button cannot start another doomed attempt, then
                // delete the item off the UI thread with a bounded deadline.
                let message = failure.message(&error);
                ui.saved_session.set(None);
                let deletion =
                    delete_saved_session(move || crate::session_store::clear_attempt(attempt))
                        .await;
                // AUD-01: the deletion may outlive the UI state that ordered it.
                if !ticket.current(&ui) {
                    return;
                }
                ui.login_error.set(match deletion {
                    Ok(()) => message,
                    Err(problem) => format!("{message} {problem}"),
                });
            }
        }
    });
}

/// AUD-04: why a saved-session resume failed. Only a genuine authentication
/// rejection may delete the stored item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SavedSessionFailure {
    /// HTTP 401 / expired session: the stored token can never work again.
    Rejected,
    /// The saved record itself is unusable, but no server rejected the session.
    Unusable,
    /// Network, DNS, timeout or 5xx: keep the item and retry.
    Transient,
    /// The server refused, redirected or answered malformed data: keep the item.
    Refused,
    /// The attempt was canceled or superseded.
    Canceled,
}

impl SavedSessionFailure {
    fn deletes_saved_item(self) -> bool {
        matches!(self, Self::Rejected)
    }

    fn message(self, error: &str) -> String {
        match self {
            Self::Rejected | Self::Canceled => error.to_owned(),
            Self::Transient => format!(
                "{error} The saved session was kept on this device; retry when the server is reachable."
            ),
            Self::Unusable => format!(
                "{error} The saved session was kept on this device; sign in again to replace it."
            ),
            Self::Refused => format!("{error} The saved session was kept on this device."),
        }
    }
}

/// These are the native client's fixed user-facing error strings
/// (`client.rs`), matched exactly on purpose: a substring match on "Sign in
/// again" would classify the 403 message as a rejected session and delete a
/// valid saved login. An unrecognized message falls back to `Refused`, which
/// preserves the item.
const SAVED_SESSION_EXPIRED: &str = "The saved session expired. Sign in again.";
const SESSION_SIGNED_OUT: &str = "Sign in again to continue.";
const SAVED_SESSION_INVALID: &str = "The saved session is invalid. Sign in again.";
const REQUEST_CANCELED: &str = "Request canceled.";
const NETWORK_UNREACHABLE: &str =
    "Unable to reach the Vela server. Check the connection and retry.";
const REQUEST_TIMED_OUT: &str = "The Vela request timed out. Retry when the server is available.";
const REQUEST_FAILED: &str = "The Vela request failed. Check the server and retry.";
const SERVER_BUSY: &str = "The server is busy. Wait before retrying.";

fn saved_session_failure(error: &str) -> SavedSessionFailure {
    match error {
        REQUEST_CANCELED => SavedSessionFailure::Canceled,
        SAVED_SESSION_EXPIRED | SESSION_SIGNED_OUT => SavedSessionFailure::Rejected,
        SAVED_SESSION_INVALID => SavedSessionFailure::Unusable,
        NETWORK_UNREACHABLE | REQUEST_TIMED_OUT | REQUEST_FAILED | SERVER_BUSY => {
            SavedSessionFailure::Transient
        }
        _ => SavedSessionFailure::Refused,
    }
}

/// Run one keyring deletion off the UI thread with a bounded wait (AUD-02). The
/// helper process itself is deadline-bounded and killed inside `session_store`;
/// this outer bound only covers a worker that never returns from the runtime.
/// Neither the local logout nor the audio engine ever waits for it.
async fn delete_saved_session(
    delete: impl FnOnce() -> Result<(), crate::session_store::KeyringError> + Send + 'static,
) -> Result<(), String> {
    let task = tokio::task::spawn_blocking(delete);
    match tokio::time::timeout(crate::session_store::worst_case_deadline(), task).await {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(error))) => Err(error.message().to_owned()),
        Ok(Err(_)) => Err(crate::session_store::KeyringError::Unavailable
            .message()
            .to_owned()),
        Err(_) => Err(
            "The system keyring did not answer in time, so the saved session may still exist."
                .to_owned(),
        ),
    }
}

fn sign_out(mut ui: Ui) {
    let old = ui.account.peek().clone();
    // AUD-02: local access, the queue and audio end immediately. The keyring
    // deletion is started after that and is never awaited on the UI thread, so
    // a hung Secret Service cannot delay stopping playback or the local logout.
    ui.clear_account();
    ui.login_error.set(String::new());
    ui.notice
        .set("Signed out on this device. Your queue and temporary audio have been cleared.".into());
    let epoch = *ui.epoch.peek();
    let login_generation = ui.login_gate.generation.get();
    let mut keyring_ui = ui.clone();
    spawn(async move {
        let deletion = delete_saved_session(crate::session_store::clear).await;
        // AUD-01/AUD-02: only the still-current sign-out may publish this error.
        if !keyring_ui.alive.get()
            || *keyring_ui.epoch.peek() != epoch
            || keyring_ui.signed_in()
            || keyring_ui.login_gate.generation.get() != login_generation
        {
            return;
        }
        if let Err(problem) = deletion {
            keyring_ui.login_error.set(format!(
                "Local sign-out is complete, but the saved session was not confirmed as removed: {problem}"
            ));
        }
    });
    let Some(account) = old else {
        return;
    };
    let ticket = ui.logout_gate.begin(epoch);
    spawn(async move {
        let result = account.client.logout(ticket.cancel.clone()).await;
        if ticket.current(&ui)
            && !ui.signed_in()
            && let Err(error) = result
        {
            ui.login_error.set(format!(
                "Local sign-out is complete. Server sign-out was not confirmed: {error}"
            ));
        }
    });
}

fn expired(mut ui: Ui, error: &str) -> bool {
    if error != "Sign in again to continue." {
        return false;
    }
    ui.clear_account();
    ui.login_error
        .set("Your session ended. Sign in again to continue.".into());
    true
}

fn load_catalog(mut ui: Ui, query: String, cursor: Option<String>, favorites: bool) {
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    ui.catalog_gate.cancel();
    ui.catalog.set(None);
    ui.catalog_loading.set(false);
    ui.catalog_error.set(String::new());
    if query.len() > 256 || query.chars().any(char::is_control) {
        ui.catalog_error
            .set("Use a search of at most 256 bytes without control characters.".into());
        return;
    }
    ui.query.set(query.clone());
    ui.cursor.set(cursor.clone());
    ui.view.set(if favorites { "favorites" } else { "library" });
    let ticket = ui.catalog_gate.begin(account.epoch);
    ui.catalog_loading.set(true);
    spawn(async move {
        if !ticket.current(&ui)
            || *ui.query.peek() != query
            || *ui.cursor.peek() != cursor
            || (*ui.view.peek() == "favorites") != favorites
        {
            return;
        }
        let result = account
            .client
            .tracks(
                (!query.is_empty()).then_some(query.as_str()),
                cursor.as_deref(),
                favorites,
                ticket.cancel.clone(),
            )
            .await;
        if !ticket.current(&ui)
            || *ui.query.peek() != query
            || *ui.cursor.peek() != cursor
            || (*ui.view.peek() == "favorites") != favorites
        {
            return;
        }
        ui.catalog_loading.set(false);
        match result {
            Ok(page) => {
                ui.catalog_error.set(String::new());
                ui.catalog.set(Some(Catalog {
                    epoch: ticket.epoch,
                    query,
                    cursor,
                    favorites,
                    page: Arc::new(page),
                }));
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.catalog_error.set(error);
                }
            }
        }
    });
}

fn start_source(mut ui: Ui, reuse: bool, autoplay: bool, position_ms: u64) {
    ui.offline_active.set(false);
    ui.remote_now.set(None);
    ui.remote_queue.set(Vec::new());
    ui.remote_index.set(0);
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let Some(track) = ui.current_track() else {
        return;
    };
    let Some(audio) = ui.audio.peek().clone() else {
        return;
    };
    let prepared = if reuse {
        ui.prepared.peek().clone()
    } else {
        None
    };
    let selection = ui.new_selection();
    ui.prepared.set(None);
    if !ui.source_error.peek().is_empty() {
        return;
    }
    if let Some(prepared) = prepared {
        match audio.load(selection, prepared.clone(), autoplay, position_ms) {
            Ok(()) => ui.prepared.set(Some(prepared)),
            Err(error) => ui.source_error.set(error),
        }
        return;
    }
    let ticket = ui.source_gate.begin(account.epoch);
    let (progress, mut updates) = watch::channel(DownloadProgress::default());
    ui.preparing.set(true);
    let mut progress_ui = ui.clone();
    let progress_ticket = ticket.clone();
    let progress_id = track.id.clone();
    spawn(async move {
        loop {
            tokio::select! {
                _ = progress_ticket.cancel.cancelled() => return,
                result = updates.changed() => {
                    if result.is_err() || !progress_ui.source_current(&progress_ticket, selection, &progress_id) { return; }
                    progress_ui.progress.set(*updates.borrow_and_update());
                }
            }
        }
    });
    spawn(async move {
        if !ui.source_current(&ticket, selection, &track.id) {
            return;
        }
        let result = account
            .client
            .prepare(&track, ticket.cancel.clone(), progress)
            .await;
        if !ui.source_current(&ticket, selection, &track.id) {
            return;
        }
        ui.preparing.set(false);
        match result {
            Ok(prepared) => match audio.load(selection, prepared.clone(), autoplay, position_ms) {
                Ok(()) => ui.prepared.set(Some(prepared)),
                Err(error) => ui.source_error.set(error),
            },
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.source_error.set(error);
                }
            }
        }
    });
}

fn load_playlists(mut ui: Ui) {
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    ui.playlist_gate.cancel();
    ui.playlist.set(None);
    ui.playlist_id.set(None);
    ui.playlist_loading.set(true);
    ui.playlist_error.set(String::new());
    ui.view.set("playlists");
    let ticket = ui.playlist_gate.begin(account.epoch);
    spawn(async move {
        let result = account.client.playlists(ticket.cancel.clone()).await;
        if !ticket.current(&ui) || *ui.view.peek() != "playlists" {
            return;
        }
        ui.playlist_loading.set(false);
        match result {
            Ok(playlists) => {
                ui.playlist_error.set(String::new());
                ui.playlists.set(playlists);
                if let Ok(folders) = account.client.playlist_folders(ticket.cancel.clone()).await
                    && ticket.current(&ui)
                    && *ui.view.peek() == "playlists"
                {
                    ui.folders.set(folders);
                }
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn open_playlist(mut ui: Ui, id: String) {
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    if !uuid::Uuid::parse_str(&id).is_ok_and(|parsed| parsed.to_string() == id) {
        ui.playlist_error.set("Select a valid playlist.".into());
        return;
    }
    ui.playlist_gate.cancel();
    ui.playlist_id.set(Some(id.clone()));
    ui.playlist.set(None);
    ui.playlist_loading.set(true);
    ui.playlist_error.set(String::new());
    let ticket = ui.playlist_gate.begin(account.epoch);
    spawn(async move {
        let result = account.client.playlist(&id, ticket.cancel.clone()).await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "playlists"
            || ui.playlist_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.playlist_loading.set(false);
        match result {
            Ok(detail) => {
                ui.playlist_error.set(String::new());
                ui.playlist_name_draft.set(detail.playlist.name.clone());
                ui.playlist_delete_confirm.set(false);
                ui.playlist.set(Some(detail));
                let candidates = account
                    .client
                    .tracks(None, None, false, ticket.cancel.clone())
                    .await;
                if ticket.current(&ui)
                    && *ui.view.peek() == "playlists"
                    && ui.playlist_id.peek().as_deref() == Some(id.as_str())
                {
                    match candidates {
                        Ok(page) => ui.playlist_candidates.set(page.items),
                        Err(_) => ui.playlist_candidates.set(Vec::new()),
                    }
                }
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn create_playlist_ui(mut ui: Ui) {
    let name = ui.playlist_name_draft.peek().trim().to_string();
    if name.is_empty() || *ui.playlist_busy.peek() {
        return;
    }
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let ticket = ui.playlist_write_gate.begin(account.epoch);
    ui.playlist_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .create_playlist(&name, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui) || *ui.view.peek() != "playlists" {
            return;
        }
        ui.playlist_busy.set(false);
        match result {
            Ok(created) => {
                ui.playlist_error.set(String::new());
                ui.playlist_name_draft.set(String::new());
                ui.playlists.write().insert(0, created);
                ui.notice.set("Playlist created.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn rename_selected_playlist(mut ui: Ui) {
    let Some(detail) = ui.playlist.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let name = ui.playlist_name_draft.peek().trim().to_string();
    if name.is_empty() || *ui.playlist_busy.peek() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let id = detail.playlist.id;
    let revision = detail.playlist.revision;
    let ticket = ui.playlist_write_gate.begin(account.epoch);
    ui.playlist_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .rename_playlist(&id, revision, &name, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "playlists"
            || ui.playlist_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.playlist_busy.set(false);
        match result {
            Ok(updated) => {
                ui.playlist_error.set(String::new());
                ui.playlist_name_draft.set(updated.name.clone());
                if let Some(detail) = ui.playlist.write().as_mut() {
                    detail.playlist = updated.clone();
                }
                if let Some(entry) = ui.playlists.write().iter_mut().find(|list| list.id == id) {
                    *entry = updated;
                }
                ui.notice.set("Playlist renamed.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn delete_selected_playlist(mut ui: Ui) {
    let Some(detail) = ui.playlist.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    if *ui.playlist_busy.peek() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let id = detail.playlist.id;
    let revision = detail.playlist.revision;
    let ticket = ui.playlist_write_gate.begin(account.epoch);
    ui.playlist_busy.set(true);
    ui.playlist_delete_confirm.set(false);
    spawn(async move {
        let result = account
            .client
            .delete_playlist(&id, revision, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "playlists"
            || ui.playlist_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.playlist_busy.set(false);
        match result {
            Ok(()) => {
                ui.playlist_error.set(String::new());
                ui.playlist.set(None);
                ui.playlist_id.set(None);
                ui.playlist_name_draft.set(String::new());
                ui.playlists.write().retain(|list| list.id != id);
                ui.notice
                    .set("Playlist deleted. Your tracks and audio are unchanged.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn move_playlist_entry(mut ui: Ui, at: usize, direction: i32) {
    let Some(detail) = ui.playlist.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    if *ui.playlist_busy.peek() {
        return;
    }
    let target = at as i32 + direction;
    if target < 0 || target as usize >= detail.items.len() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let mut ids = detail
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    ids.swap(at, target as usize);
    let id = detail.playlist.id;
    let revision = detail.playlist.revision;
    let ticket = ui.playlist_write_gate.begin(account.epoch);
    ui.playlist_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .reorder_playlist(&id, revision, &ids, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "playlists"
            || ui.playlist_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.playlist_busy.set(false);
        match result {
            Ok(updated) => {
                ui.playlist_error.set(String::new());
                if let Some(entry) = ui.playlists.write().iter_mut().find(|list| list.id == id) {
                    entry.revision = updated.playlist.revision;
                    entry.item_count = updated.items.len() as i64;
                }
                ui.playlist.set(Some(updated));
                ui.notice.set("Playlist order updated.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn remove_playlist_entry(mut ui: Ui, item_id: String) {
    let Some(detail) = ui.playlist.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    if *ui.playlist_busy.peek() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let id = detail.playlist.id;
    let revision = detail.playlist.revision;
    let ticket = ui.playlist_write_gate.begin(account.epoch);
    ui.playlist_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .remove_playlist_item(&id, &item_id, revision, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "playlists"
            || ui.playlist_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.playlist_busy.set(false);
        match result {
            Ok(updated) => {
                ui.playlist_error.set(String::new());
                if let Some(entry) = ui.playlists.write().iter_mut().find(|list| list.id == id) {
                    entry.revision = updated.playlist.revision;
                    entry.item_count = updated.items.len() as i64;
                }
                ui.playlist.set(Some(updated));
                ui.notice.set("Playlist entry removed.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn create_folder(mut ui: Ui) {
    let name = ui.folder_name_draft.peek().trim().to_string();
    if name.is_empty() || *ui.folder_busy.peek() || !ui.signed_in() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let ticket = ui.folder_gate.begin(account.epoch);
    ui.folder_busy.set(true);
    ui.playlist_error.set(String::new());
    spawn(async move {
        let result = account
            .client
            .create_playlist_folder(&name, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui) || *ui.view.peek() != "playlists" {
            return;
        }
        ui.folder_busy.set(false);
        match result {
            Ok(folder) => {
                ui.folder_name_draft.set(String::new());
                ui.folders.write().push(folder);
                ui.folders.write().sort_by(|left, right| {
                    left.name
                        .to_lowercase()
                        .cmp(&right.name.to_lowercase())
                        .then_with(|| left.id.cmp(&right.id))
                });
                ui.notice.set("Playlist folder created.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn select_folder(mut ui: Ui, id: Option<String>) {
    ui.folder_confirm.set(String::new());
    ui.folder_rename_draft.set(
        id.as_deref()
            .and_then(|id| {
                ui.folders
                    .peek()
                    .iter()
                    .find(|folder| folder.id == id)
                    .map(|folder| folder.name.clone())
            })
            .unwrap_or_default(),
    );
    ui.folder_filter.set(id);
}

fn rename_folder(mut ui: Ui) {
    let Some(id) = ui.folder_filter.peek().clone() else {
        return;
    };
    let name = ui.folder_rename_draft.peek().trim().to_string();
    if name.is_empty() || *ui.folder_busy.peek() {
        return;
    }
    let Some(folder) = ui
        .folders
        .peek()
        .iter()
        .find(|folder| folder.id == id)
        .cloned()
    else {
        return;
    };
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let ticket = ui.folder_gate.begin(account.epoch);
    ui.folder_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .rename_playlist_folder(&id, folder.revision, &name, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui) || *ui.view.peek() != "playlists" {
            return;
        }
        ui.folder_busy.set(false);
        match result {
            Ok(updated) => {
                ui.folder_rename_draft.set(updated.name.clone());
                if let Some(entry) = ui.folders.write().iter_mut().find(|folder| folder.id == id) {
                    *entry = updated;
                }
                ui.notice.set("Folder renamed.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn delete_folder(mut ui: Ui, id: String) {
    let Some(folder) = ui
        .folders
        .peek()
        .iter()
        .find(|folder| folder.id == id)
        .cloned()
    else {
        return;
    };
    if *ui.folder_busy.peek() || !ui.signed_in() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let ticket = ui.folder_gate.begin(account.epoch);
    ui.folder_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .delete_playlist_folder(&id, folder.revision, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui) || *ui.view.peek() != "playlists" {
            return;
        }
        ui.folder_busy.set(false);
        match result {
            Ok(()) => {
                ui.folders.write().retain(|folder| folder.id != id);
                if ui.folder_filter.peek().as_deref() == Some(id.as_str()) {
                    ui.folder_filter.set(None);
                    ui.folder_rename_draft.set(String::new());
                }
                ui.folder_confirm.set(String::new());
                if let Ok(playlists) = account.client.playlists(ticket.cancel.clone()).await
                    && ticket.current(&ui)
                    && *ui.view.peek() == "playlists"
                {
                    ui.playlists.set(playlists);
                }
                ui.notice
                    .set("Folder deleted. Its playlists are now unfiled.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn move_playlist_folder(mut ui: Ui) {
    let Some(detail) = ui.playlist.peek().clone() else {
        return;
    };
    let target = ui.playlist_folder_target.peek().clone();
    let folder_id = (!target.is_empty()).then_some(target);
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let id = detail.playlist.id;
    let ticket = ui.folder_gate.begin(account.epoch);
    ui.folder_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .move_playlist(
                &id,
                detail.playlist.revision,
                folder_id.as_deref(),
                ticket.cancel.clone(),
            )
            .await;
        if !ticket.current(&ui) || *ui.view.peek() != "playlists" {
            return;
        }
        ui.folder_busy.set(false);
        match result {
            Ok(updated) => {
                if let Some(entry) = ui.playlists.write().iter_mut().find(|list| list.id == id) {
                    *entry = updated.clone();
                }
                if let Ok(detail) = account.client.playlist(&id, ticket.cancel.clone()).await
                    && ticket.current(&ui)
                    && *ui.view.peek() == "playlists"
                {
                    ui.playlist.set(Some(detail));
                }
                if let Ok(folders) = account.client.playlist_folders(ticket.cancel.clone()).await
                    && ticket.current(&ui)
                    && *ui.view.peek() == "playlists"
                {
                    ui.folders.set(folders);
                }
                ui.notice.set("Playlist folder updated.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn add_selected_playlist_track(mut ui: Ui) {
    let Some(detail) = ui.playlist.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let track_id = ui.playlist_candidate.peek().clone();
    if track_id.is_empty() || *ui.playlist_busy.peek() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let id = detail.playlist.id;
    let revision = detail.playlist.revision;
    let ticket = ui.playlist_write_gate.begin(account.epoch);
    ui.playlist_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .add_playlist_item(&id, revision, &track_id, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "playlists"
            || ui.playlist_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.playlist_busy.set(false);
        match result {
            Ok(updated) => {
                ui.playlist_error.set(String::new());
                ui.playlist_candidate.set(String::new());
                if let Some(entry) = ui.playlists.write().iter_mut().find(|list| list.id == id) {
                    entry.revision = updated.playlist.revision;
                    entry.item_count = updated.items.len() as i64;
                }
                ui.playlist.set(Some(updated));
                ui.notice.set("Track added to playlist.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn play_playlist(mut ui: Ui) {
    let Some(detail) = ui.playlist.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    if *ui.view.peek() != "playlists" || ui.audio.peek().is_none() {
        return;
    }
    let tracks = detail
        .items
        .iter()
        .filter(|item| item.can_stream)
        .map(|item| item.track.clone())
        .collect::<Vec<_>>();
    if tracks.is_empty() {
        ui.playlist_error
            .set("This playlist has no tracks your account may stream.".into());
        return;
    }
    ui.order.write().reset(tracks.len(), 0);
    ui.queue.set(tracks);
    ui.context
        .set(format!("Playlist · {}", detail.playlist.name));
    start_source(ui, false, true, 0);
}

fn toggle_favorite(mut ui: Ui, catalog: Catalog, track: Track) {
    if !ui.catalog_current(&catalog) || !ui.favorite_busy.peek().is_empty() {
        return;
    }
    let Some(account) = ui
        .account
        .peek()
        .clone()
        .filter(|account| ui.signed_in() && account.epoch == catalog.epoch)
    else {
        return;
    };
    let desired = !track.favorite;
    let ticket = ui.feedback_gate.begin(catalog.epoch);
    ui.favorite_busy.set(track.id.clone());
    spawn(async move {
        let result = account
            .client
            .favorite(&track.id, desired, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui) {
            return;
        }
        if *ui.favorite_busy.peek() == track.id {
            ui.favorite_busy.set(String::new());
        }
        match result {
            Ok(()) => {
                if catalog.favorites && !desired {
                    // Re-read the filtered page instead of leaving a stale row.
                    load_catalog(ui.clone(), catalog.query.clone(), None, true);
                } else {
                    let mut page = (*catalog.page).clone();
                    for item in &mut page.items {
                        if item.id == track.id {
                            item.favorite = desired;
                        }
                    }
                    ui.catalog.set(Some(Catalog {
                        page: Arc::new(page),
                        ..catalog.clone()
                    }));
                }
                for item in ui.queue.write().iter_mut() {
                    if item.id == track.id {
                        item.favorite = desired;
                    }
                }
                ui.notice.set(
                    if desired {
                        "Added to favorites."
                    } else {
                        "Removed from favorites."
                    }
                    .into(),
                );
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.catalog_error.set(error);
                }
            }
        }
    });
}

fn toggle_library_favorite(mut ui: Ui, track: Track) {
    if !ui.signed_in() || !ui.favorite_busy.peek().is_empty() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let desired = !track.favorite;
    let ticket = ui.feedback_gate.begin(account.epoch);
    ui.favorite_busy.set(track.id.clone());
    spawn(async move {
        let result = account
            .client
            .favorite(&track.id, desired, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui) {
            return;
        }
        if *ui.favorite_busy.peek() == track.id {
            ui.favorite_busy.set(String::new());
        }
        match result {
            Ok(()) => {
                for item in ui.queue.write().iter_mut() {
                    if item.id == track.id {
                        item.favorite = desired;
                    }
                }
                let current_catalog = ui.catalog.peek().clone();
                if let Some(catalog) = current_catalog {
                    let mut page = (*catalog.page).clone();
                    let mut changed = false;
                    for item in &mut page.items {
                        if item.id == track.id {
                            item.favorite = desired;
                            changed = true;
                        }
                    }
                    if changed {
                        ui.catalog.set(Some(Catalog {
                            page: Arc::new(page),
                            ..catalog
                        }));
                    }
                }
                ui.notice.set(
                    if desired {
                        "Added to favorites."
                    } else {
                        "Removed from favorites."
                    }
                    .into(),
                );
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.source_error.set(error);
                }
            }
        }
    });
}

fn play_catalog(mut ui: Ui, catalog: Catalog, at: usize) {
    if !ui.catalog_current(&catalog) {
        ui.catalog_error
            .set("This page changed. Refresh the library before playing.".into());
        return;
    }
    if at >= catalog.page.items.len() || ui.audio.peek().is_none() {
        return;
    }
    let mut tracks = catalog.page.items.clone();
    tracks.truncate(MAX_QUEUE);
    ui.order.write().reset(tracks.len(), at);
    ui.queue.set(tracks);
    ui.context.set(if catalog.query.is_empty() {
        "Your library".into()
    } else {
        "Search results".into()
    });
    start_source(ui, false, true, 0);
}

fn enqueue(mut ui: Ui, track: Track, next: bool) {
    if !ui.signed_in() {
        return;
    }
    if ui.queue.peek().len() >= MAX_QUEUE {
        ui.notice
            .set("Your queue is full (1,000 tracks). Remove a queued track first.".into());
        return;
    }
    let at = if next {
        ui.order
            .peek()
            .current()
            .map_or(0, |at| at + 1)
            .min(ui.queue.peek().len())
    } else {
        ui.queue.peek().len()
    };
    if !ui.order.write().insert(at, next) {
        return;
    }
    ui.queue.write().insert(at, track);
    if ui.context.peek().is_empty() {
        ui.context.set("Your queue".into());
    }
    ui.notice.set(
        if next {
            "Added next in your queue."
        } else {
            "Added to your queue."
        }
        .into(),
    );
}

fn select_queue(mut ui: Ui, at: usize) {
    if !ui.signed_in() || at >= ui.queue.peek().len() || ui.audio.peek().is_none() {
        return;
    }
    ui.order.write().reset(ui.queue.peek().len(), at);
    start_source(ui, false, true, 0);
}

fn advance(mut ui: Ui, command: Command) {
    if !ui.signed_in() || ui.current_track().is_none() {
        return;
    }
    let elapsed = ui.snapshot.peek().position_ms as f64 / 1000.0;
    let random = uuid::Uuid::new_v4();
    let bytes = random.as_bytes();
    let entropy = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let transition = ui.order.write().transition(command, elapsed, entropy);
    match transition {
        Transition::Play(_) => start_source(ui, false, true, 0),
        Transition::SeekStart { resume: true } => start_source(ui, true, true, 0),
        Transition::SeekStart { resume: false } => {
            if *ui.preparing.peek() {
                return;
            }
            if let Some(audio) = ui.audio.peek().clone()
                && let Err(error) = audio.seek(*ui.selection.peek(), 0)
            {
                ui.source_error.set(error);
            }
        }
        Transition::Stop => {
            if *ui.preparing.peek() {
                ui.new_selection();
                ui.prepared.set(None);
            } else if command != Command::Ended
                && let Some(audio) = ui.audio.peek().clone()
                && let Err(error) = audio.pause(*ui.selection.peek())
            {
                ui.source_error.set(error);
            }
        }
        Transition::None => {}
    }
}

fn reconnect_audio(mut ui: Ui) {
    if ui.audio.peek().as_ref().is_some_and(AudioEngine::alive) {
        return;
    }
    let Some(account) = ui
        .account
        .peek()
        .clone()
        .filter(|account| ui.signed_in() && account.epoch == *ui.epoch.peek())
    else {
        return;
    };
    ui.audio.set(None);
    ui.audio_error.set(String::new());
    let mut installing = ui.clone();
    spawn(async move {
        let result = tokio::task::spawn_blocking(AudioEngine::new).await;
        if !installing.alive.get() || *installing.epoch.peek() != account.epoch {
            return;
        }
        match result {
            Ok(Ok(engine)) => {
                *installing.engine_cleanup.borrow_mut() = Some(engine.clone());
                installing.audio.set(Some(engine));
            }
            Ok(Err(error)) => installing.audio_error.set(error),
            Err(_) => installing.audio_error.set(
                "Native audio output could not be reconnected. Restart Vela to try again.".into(),
            ),
        }
    });
}
fn toggle(mut ui: Ui) {
    if !ui.signed_in() || *ui.preparing.peek() {
        return;
    }
    let Some(audio) = ui.audio.peek().clone() else {
        return;
    };
    if ui.current_track().is_none() && ui.remote_now.peek().is_none() {
        return;
    }
    let snapshot = ui.snapshot.peek().clone();
    if ui.remote_now.peek().is_some() && snapshot.phase == AudioPhase::Ended {
        start_remote(ui);
        return;
    }
    let result = match snapshot.phase {
        AudioPhase::Playing => audio.pause(*ui.selection.peek()),
        AudioPhase::Paused if ui.prepared.peek().is_some() => audio.resume(*ui.selection.peek()),
        AudioPhase::Loading => return,
        _ => {
            start_source(ui, true, true, 0);
            return;
        }
    };
    if let Err(error) = result {
        ui.source_error.set(error);
    }
}

fn seek_to(mut ui: Ui, position: u64) {
    if !ui.signed_in() || *ui.preparing.peek() || ui.prepared.peek().is_none() {
        return;
    }
    let maximum = ui.snapshot.peek().duration_ms;
    let target = position.min(maximum);
    if let Some(audio) = ui.audio.peek().clone() {
        match audio.seek(*ui.selection.peek(), target) {
            Ok(()) => {
                let serial = ui.mpris_seek_serial.peek().wrapping_add(1);
                ui.mpris_seek_serial.set(serial);
            }
            Err(error) => ui.source_error.set(error),
        }
    }
}

fn seek(ui: Ui, position: String) {
    let Ok(position) = position.parse::<u64>() else {
        return;
    };
    seek_to(ui, position);
}

fn system_controls_note(state: mpris::State) -> &'static str {
    match state {
        mpris::State::Starting => "System media controls starting…",
        mpris::State::Ready => "System media controls active",
        mpris::State::Failed => "System media controls unavailable in this session",
        mpris::State::Unsupported => "System media controls are not integrated here yet",
    }
}

fn play_system(mut ui: Ui) {
    if !ui.signed_in()
        || ui.current_track().is_none()
        || !ui.source_error.peek().is_empty()
        || *ui.preparing.peek()
    {
        return;
    }
    let phase = ui.snapshot.peek().phase;
    if phase == AudioPhase::Playing {
        return;
    }
    if phase == AudioPhase::Paused
        && ui.prepared.peek().is_some()
        && let Some(audio) = ui.audio.peek().clone()
        && let Err(error) = audio.resume(*ui.selection.peek())
    {
        ui.source_error.set(error);
        return;
    }
    if phase != AudioPhase::Paused || ui.prepared.peek().is_none() {
        start_source(ui, true, true, 0);
    }
}

fn pause_system(mut ui: Ui) {
    if !ui.signed_in() || ui.snapshot.peek().phase != AudioPhase::Playing {
        return;
    }
    if let Some(audio) = ui.audio.peek().clone()
        && let Err(error) = audio.pause(*ui.selection.peek())
    {
        ui.source_error.set(error);
    }
}

fn stop_system(mut ui: Ui) {
    if !ui.signed_in() {
        return;
    }
    ui.new_selection();
    ui.prepared.set(None);
}

/// Apply one external media command on the UI thread. Returns true when the
/// command closed this window. Anything that needs account state is ignored
/// while signed out; only Quit remains valid at the sign-in screen.
fn apply_system_control(mut ui: Ui, command: mpris::Command) -> bool {
    match command {
        mpris::Command::Quit => {
            dioxus::desktop::window().close();
            return true;
        }
        mpris::Command::Play => play_system(ui),
        mpris::Command::Pause => pause_system(ui),
        mpris::Command::PlayPause => toggle(ui),
        mpris::Command::Stop => stop_system(ui),
        mpris::Command::Next => advance(ui, Command::Next),
        mpris::Command::Previous => advance(ui, Command::Previous),
        mpris::Command::SeekRelative(offset) => {
            if ui.signed_in() && ui.prepared.peek().is_some() {
                let position = ui.snapshot.peek().position_ms;
                let duration = ui.snapshot.peek().duration_ms;
                let target = (i128::from(position) + i128::from(offset) / 1000)
                    .clamp(0, i128::from(duration)) as u64;
                seek_to(ui, target);
            }
        }
        mpris::Command::SetPosition {
            track_id,
            position_ms,
        } => {
            if ui
                .current_track()
                .is_some_and(|track| mpris::track_path(&track.id) == track_id)
            {
                seek_to(ui, position_ms);
            }
        }
        mpris::Command::SetVolume(value) => {
            if value.is_finite()
                && ui.signed_in()
                && let Some(audio) = ui.audio.peek().clone()
            {
                let volume = (value.clamp(0.0, 1.0)) as f32;
                if let Err(error) = audio.set_volume(*ui.selection.peek(), volume) {
                    ui.source_error.set(error);
                }
            }
        }
        mpris::Command::SetShuffle(shuffle) => {
            if ui.signed_in() && *ui.shuffle.peek() != shuffle {
                ui.shuffle.set(shuffle);
                let repeat = *ui.repeat.peek();
                ui.order.write().configure(shuffle, repeat);
            }
        }
        mpris::Command::SetRepeat(repeat) => {
            if !ui.signed_in() {
                return false;
            }
            let repeat = match repeat {
                SystemRepeat::Off => Repeat::Off,
                SystemRepeat::All => Repeat::All,
                SystemRepeat::One => Repeat::One,
            };
            if *ui.repeat.peek() != repeat {
                ui.repeat.set(repeat);
                let shuffle = *ui.shuffle.peek();
                ui.order.write().configure(shuffle, repeat);
            }
        }
    }
    false
}

fn volume(mut ui: Ui, value: String) {
    if !ui.signed_in() {
        return;
    }
    let Ok(value) = value.parse::<f32>() else {
        return;
    };
    let Some(audio) = ui.audio.peek().clone() else {
        return;
    };
    if let Err(error) = audio.set_volume(*ui.selection.peek(), (value / 100.0).clamp(0.0, 1.0)) {
        ui.source_error.set(error);
    }
}

fn queue_edit(mut ui: Ui, at: usize, remove: bool) {
    if !ui.signed_in() {
        return;
    }
    if remove {
        if !ui.order.write().remove(at) {
            return;
        }
        ui.queue.write().remove(at);
    } else {
        if at == 0 || !ui.order.write().swap(at, at - 1) {
            return;
        }
        ui.queue.write().swap(at, at - 1);
    }
}

fn queue_move_down(mut ui: Ui, at: usize) {
    if !ui.signed_in() || at + 1 >= ui.queue.peek().len() {
        return;
    }
    if ui.order.write().swap(at, at + 1) {
        ui.queue.write().swap(at, at + 1);
        ui.notice.set("Queue order updated.".into());
    }
}

fn queue_clear_upcoming(mut ui: Ui) {
    if !ui.signed_in() {
        return;
    }
    let Some(current) = ui.order.peek().current() else {
        return;
    };
    let keep = current + 1;
    while ui.queue.peek().len() > keep {
        let last = ui.queue.peek().len() - 1;
        if !ui.order.write().remove(last) {
            break;
        }
        ui.queue.write().remove(last);
    }
}

fn clear_queue(mut ui: Ui) {
    if !ui.signed_in() {
        return;
    }
    ui.new_selection();
    ui.prepared.set(None);
    ui.queue.set(Vec::new());
    ui.order.write().reset(0, 0);
    ui.context.set(String::new());
    ui.notice
        .set("Queue cleared. Temporary audio released.".into());
}

fn cached_age_label(fetched_at: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(fetched_at);
    let age = now.saturating_sub(fetched_at);
    if age < 60 {
        "just now".into()
    } else if age < 3_600 {
        format!("{}m old", age / 60)
    } else if age < 86_400 {
        format!("{}h old", age / 3_600)
    } else {
        format!("{}d old", age / 86_400)
    }
}

fn time_label(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
fn byte_label(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn sign_in(ui: Ui) -> Element {
    let origin = ui.origin.read().clone();
    let username = ui.username.read().clone();
    let password = ui.password.read().clone();
    let connecting = *ui.connecting.read();
    let output_error = ui.audio_error.read().clone();
    let output_pending = !ui.audio_ready() && output_error.is_empty();
    let error = ui.login_error.read().clone();
    let notice = ui.notice.read().clone();
    let saved_session = ui.saved_session.read().clone();
    let resuming = *ui.resuming.read();
    let mut remember = ui.remember;
    let resume = ui.clone();
    let mut origin_signal = ui.origin;
    let mut username_signal = ui.username;
    let mut password_signal = ui.password;
    let submit = ui.clone();
    let cancel = ui.clone();
    rsx! { main { class: "native-signin", "aria-label": "Sign in to Vela",
        section { class: "signin-story", div { class: "native-brand", Icon {name:"music"} span {"vela"} small {"NATIVE"} }
            div {class:"signin-copy", span {class:"eyebrow","YOUR MUSIC, A LITTLE CLOSER"} h1 {"A space for" br {} "your own sound."} p {"Your private Vela library, played through this device’s native audio output."} }
            div {class:"signin-record","aria-hidden":"true",span {"v"}}
            small {class:"signin-footnote","Your account stays in memory. Sign out to clear the queue and temporary audio."}
        }
        section {class:"signin-form",span {class:"eyebrow","WELCOME TO YOUR LISTENING SPACE"} h2 {"Connect to your Vela"} p {"Use an existing account. Your administrator creates accounts on your Vela server."}
            if !error.is_empty() {div {class:"alert error",role:"alert","{error}"}}
            if !notice.is_empty() {p {class:"signin-notice",role:"status","{notice}"}}
            if let Some(saved) = saved_session {
                div { class: "signin-notice", role: "group", "aria-label": "Saved session",
                    p { "Saved session for {saved.origin}" }
                    button { class: "button secondary", disabled: connecting || resuming || output_pending, onclick: move |_| resume_saved(resume.clone()), if resuming { "Resuming…" } else { "Continue saved session" } }
                    small { "The keyring stores only this session token, not your password or CSRF token." }
                }
            }
            if output_pending {p {class:"signin-notice",role:"status","data-testid":"native-output-status","Preparing native audio output…"}}
            if !output_error.is_empty() {div {class:"alert error",role:"alert","{output_error} You can browse your library; playback needs an available audio output."}}
            form {onsubmit:move |event|{event.prevent_default();login(submit.clone());},
                label {r#for:"native-origin","Server origin"}
                input {id:"native-origin",r#type:"url",required:true,maxlength:2048,value:"{origin}",disabled:connecting,autocomplete:"url",placeholder:"https://music.example.org",oninput:move |event|origin_signal.set(event.value())}
                small {"HTTPS for remote servers; numeric loopback HTTP is available for local development."}
                label {r#for:"native-username","Username"}
                input {id:"native-username",required:true,maxlength:32,value:"{username}",disabled:connecting,autocomplete:"username",oninput:move |event|username_signal.set(event.value())}
                label {r#for:"native-password","Password"}
                input {id:"native-password",r#type:"password",required:true,value:"{password}",disabled:connecting,autocomplete:"off",oninput:move |event|password_signal.set(event.value())}
                label { class: "checkbox-label", r#for: "native-remember", input { id: "native-remember", r#type: "checkbox", checked: remember(), disabled: connecting, onchange: move |event| remember.set(event.checked()) } span { "Remember this session on this device" } }
                button {class:"button primary",r#type:"submit",disabled:connecting||output_pending,if connecting {"Connecting…"} else if output_pending {"Preparing native output…"} else {"Sign in"}}
                if connecting {button {class:"button secondary",r#type:"button",onclick:move |_|{let mut ui=cancel.clone();ui.login_gate.cancel();ui.connecting.set(false);ui.login_error.set("Connection canceled. Enter your password to try again.".into());},"Cancel connection"}}
            }
            small {class:"signin-limit","Opt-in session saving uses the system Secret Service and stores only the opaque session token; the password and CSRF token are never stored. Signing out deletes the saved session. No listening history is sent."}
        }
    } }
}

fn download_offline(mut ui: Ui, track: Track) {
    if !ui.offline_busy.peek().is_empty()
        || ui
            .offline
            .peek()
            .iter()
            .any(|saved| saved.track.id == track.id)
    {
        return;
    }
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let account_id = account.user.id.clone();
    let track_id = track.id.clone();
    let ticket = ui.offline_gate.begin(account.epoch);
    ui.offline_busy.set(track_id.clone());
    ui.offline_error.set(String::new());
    spawn(async move {
        let (progress, _updates) = tokio::sync::watch::channel(DownloadProgress::default());
        let result = account
            .client
            .prepare(&track, ticket.cancel.clone(), progress)
            .await;
        if !ticket.current(&ui)
            || ui
                .account
                .peek()
                .as_ref()
                .is_none_or(|account| account.user.id != account_id)
        {
            return;
        }
        let prepared = match result {
            Ok(prepared) => prepared,
            Err(error) => {
                ui.offline_busy.set(String::new());
                if !expired(ui.clone(), &error) {
                    ui.offline_error.set(error);
                }
                return;
            }
        };
        let save = tokio::task::spawn_blocking({
            let account_id = account_id.clone();
            let track = track.clone();
            move || crate::offline::save(&account_id, &track, &prepared)
        })
        .await;
        if !ticket.current(&ui) {
            return;
        }
        ui.offline_busy.set(String::new());
        match save {
            Ok(Ok(())) => {
                let loaded = tokio::task::spawn_blocking({
                    let account_id = account_id.clone();
                    move || crate::offline::load(&account_id)
                })
                .await;
                match loaded {
                    Ok(Ok(tracks)) => ui.offline.set(tracks),
                    Ok(Err(error)) => ui.offline_error.set(error),
                    Err(_) => ui
                        .offline_error
                        .set("Saved offline music could not be read.".into()),
                }
                ui.offline_corrupt
                    .write()
                    .retain(|track| track != &track_id);
                ui.notice.set("Saved for offline listening.".into());
            }
            Ok(Err(error)) => ui.offline_error.set(error),
            Err(_) => ui
                .offline_error
                .set("The offline copy could not be written.".into()),
        }
    });
}

fn play_offline(ui: Ui, saved: crate::offline::OfflineTrack) {
    let Some(index) = ui
        .offline
        .peek()
        .iter()
        .position(|entry| entry.track.id == saved.track.id)
    else {
        return;
    };
    start_offline_at(ui, index);
}

fn start_offline_at(mut ui: Ui, index: usize) {
    if !ui.signed_in() || (*ui.view.peek() != "library" && *ui.view.peek() != "favorites") {
        return;
    }
    let Some(saved) = ui.offline.peek().get(index).cloned() else {
        return;
    };
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let Some(audio) = ui.audio.peek().clone() else {
        return;
    };
    let prepared = match crate::offline::prepare(&account.user.id, &saved.track.id) {
        Ok(prepared) => prepared,
        Err(error) => {
            ui.offline_error.set(error);
            return;
        }
    };
    let selection = ui.new_selection();
    ui.source_error.set(String::new());
    ui.offline_active.set(true);
    ui.remote_now.set(None);
    ui.remote_queue.set(Vec::new());
    ui.remote_index.set(0);
    let queue = ui
        .offline
        .peek()
        .iter()
        .map(|entry| entry.track.clone())
        .collect::<Vec<_>>();
    let length = queue.len();
    ui.queue.set(queue);
    ui.order.write().reset(length, index);
    ui.context.set(format!("Offline · {}", saved.track.title));
    ui.prepared.set(None);
    match audio.load(selection, prepared.clone(), true, 0) {
        Ok(()) => ui.prepared.set(Some(prepared)),
        Err(error) => {
            ui.offline_active.set(false);
            ui.source_error.set(error);
        }
    }
}

fn offline_step(ui: Ui, delta: i32) {
    let current = ui.order.read().current();
    let Some(current) = current else {
        return;
    };
    let Some(next) = crate::remote::queue_target(current, ui.queue.peek().len(), delta) else {
        return;
    };
    start_offline_at(ui, next);
}

fn remove_offline(mut ui: Ui, track_id: String) {
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let account_id = account.user.id.clone();
    ui.offline_error.set(String::new());
    spawn(async move {
        let removed = tokio::task::spawn_blocking({
            let account_id = account_id.clone();
            let track_id = track_id.clone();
            move || crate::offline::remove(&account_id, &track_id)
        })
        .await;
        if ui
            .account
            .peek()
            .as_ref()
            .is_none_or(|account| account.user.id != account_id)
        {
            return;
        }
        match removed {
            Ok(Ok(())) => {
                if *ui.offline_active.peek() {
                    let selection = ui.new_selection();
                    if let Some(audio) = ui.audio.peek().clone() {
                        let _ = audio.stop(selection);
                    }
                    ui.prepared.set(None);
                    ui.offline_active.set(false);
                    ui.queue.set(Vec::new());
                    ui.order.write().reset(0, 0);
                    ui.offline_confirm.set(String::new());
                }
                let loaded = tokio::task::spawn_blocking({
                    let account_id = account_id.clone();
                    move || crate::offline::load(&account_id)
                })
                .await;
                match loaded {
                    Ok(Ok(tracks)) => ui.offline.set(tracks),
                    Ok(Err(error)) => ui.offline_error.set(error),
                    Err(_) => ui
                        .offline_error
                        .set("Saved offline music could not be read.".into()),
                }
                ui.offline_corrupt
                    .write()
                    .retain(|track| track != &track_id);
                ui.notice.set("Offline copy removed.".into());
            }
            Ok(Err(error)) => ui.offline_error.set(error),
            Err(_) => ui
                .offline_error
                .set("The offline copy could not be removed.".into()),
        }
    });
}

fn library(ui: Ui) -> Element {
    let loading = *ui.catalog_loading.read();
    let error = ui.catalog_error.read().clone();
    let draft = ui.query_draft.read().clone();
    let query = ui.query.read().clone();
    let favorites = *ui.view.read() == "favorites";
    let catalog = ui
        .catalog
        .read()
        .clone()
        .filter(|catalog| ui.catalog_current(catalog));
    let catalog_items = catalog
        .as_ref()
        .map_or(0, |catalog| catalog.page.items.len());
    let mut search = ui.query_draft;
    let submit = ui.clone();
    let refresh = ui.clone();
    let first = ui.clone();
    let next = ui.clone();
    let offline = ui.offline.read().clone();
    let offline_busy = ui.offline_busy.read().clone();
    let offline_error = ui.offline_error.read().clone();
    let offline_confirm = ui.offline_confirm.read().clone();
    let offline_corrupt = ui
        .offline_corrupt
        .read()
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let offline_ids = offline
        .iter()
        .map(|saved| saved.track.id.clone())
        .collect::<std::collections::HashSet<_>>();
    rsx! {section {class:"native-library","aria-label":if favorites {"Favorite tracks"} else {"Private library"},"data-view":if favorites {"favorites"} else {"library"},"data-query":"{query}","data-loading":"{loading}","data-error":"{error}","data-catalog-items":"{catalog_items}",
        div {class:"library-intro",div {span {class:"eyebrow",if favorites {"THE ONES YOU KEEP CLOSE"} else {"YOUR COLLECTION, YOUR ORIGINALS"}} h1 {if favorites {"Favorites, kept close"} else {"A library that feels like home"}} p {if favorites {"Only tracks you explicitly favorited, using the same owner-only catalog and audio rights."} else {"Choose a track. Keep the rest of your day in the background."}}}
            button {class:"button secondary",disabled:loading,onclick:move |_|{let query=refresh.query.peek().clone();let cursor=refresh.cursor.peek().clone();let favorites=*refresh.view.peek()=="favorites";load_catalog(refresh.clone(),query,cursor,favorites);},Icon {name:"history"} if favorites {"Refresh favorites"} else {"Refresh library"}}
        }
        form {class:"native-search",onsubmit:move |event|{event.prevent_default();let query=submit.query_draft.peek().trim().to_string();let favorites=*submit.view.peek()=="favorites";load_catalog(submit.clone(),query,None,favorites);},
            Icon {name:"search"} label {class:"sr-only",r#for:"native-search",if favorites {"Search your favorites"} else {"Search your library"}}
            input {id:"native-search",value:"{draft}",maxlength:256,placeholder:"Search titles, artists or albums",oninput:move |event|search.set(event.value())}
            button {class:"button primary",r#type:"submit","Search"}
        }
        if !offline.is_empty() {
            div {class:"catalog-caption",p {"Saved offline · {offline.len()} private device copies"} span {"NO SERVER NEEDED"}}
            div {class:"native-queue-list",for saved in offline.iter().cloned() {
                {
                    let play_handler = ui.clone();
                    let confirm_ui = ui.clone();
                    let cancel_ui = ui.clone();
                    let set_confirm_ui = ui.clone();
                    let confirm_id = saved.track.id.clone();
                    let offline_id = saved.track.id.clone();
                    let offline_damaged = offline_corrupt.contains(&offline_id);
                    let set_confirm_id = offline_id.clone();
                    let play_saved = saved.clone();
                    let title = saved.track.title.clone();
                    rsx! { article {class:"native-queue-row",key:"offline-{offline_id.clone()}","data-offline-track-id":"{offline_id.clone()}",
                        div {class:"neutral-cover","aria-hidden":"true",Icon {name:"down"}}
                        div {class:"native-queue-select",div {strong {"{saved.track.title}"} small {"{saved.track.artist} · {saved.length_bytes / 1024} KiB on this device"} small {if offline_damaged {"Integrity check failed · save it again"} else {"Private offline copy"}}}}
                        button {class:"button primary small",disabled:offline_damaged,"aria-label":"Play offline {title}",onclick:move |_|play_offline(play_handler.clone(),play_saved.clone()),Icon {name:"play"} "Play offline"}
                        if offline_confirm == offline_id {
                            button {class:"button danger small","aria-label":"Confirm remove offline {title}",onclick:move |_|remove_offline(confirm_ui.clone(),confirm_id.clone()),"Confirm remove"}
                            button {class:"button secondary small","aria-label":"Cancel remove offline {title}",onclick:move |_|{let mut ui=cancel_ui.clone();ui.offline_confirm.set(String::new());},"Cancel"}
                        } else {
                            button {class:"icon-button","aria-label":"Remove offline copy {title}",title:"Remove only this device copy; the original and server library stay.",onclick:move |_|{let mut ui=set_confirm_ui.clone();ui.offline_confirm.set(set_confirm_id.clone());},Icon {name:"close"}}
                        }
                    } }
                }
            } }
        }
        if !offline_error.is_empty() {div {class:"alert error",role:"alert","{offline_error}"}}
        div {class:"catalog-caption",p {if favorites {if query.is_empty() {"Your favorites · Up to 50 per page"} else {"Favorite results for “{query}” · Up to 50 per page"}} else if query.is_empty() {"Your private tracks · Up to 50 per page"} else {"Results for “{query}” · Up to 50 per page"}} span {"ORIGINAL AUDIO"}}
        if loading {div {class:"native-loading",role:"status",span {class:"loading-mark","aria-hidden":"true"} if favorites {"Loading your favorites…"} else {"Loading your library…"}}}
        if !error.is_empty() {div {class:"alert error",role:"alert","{error} Refresh the library to try again."}}
        if let Some(catalog) = catalog {
            if catalog.page.items.is_empty() {div {class:"native-empty",Icon {name:"heart"} h2 {if favorites {if query.is_empty() {"No favorites yet"} else {"No matching favorites"}} else if query.is_empty() {"Make room for your first listen"} else {"No matching tracks"}} p {if favorites {if query.is_empty() {"Favorite a track from your library to find it here. Favorites are private to your account."} else {"Try a different title, artist or album. Only your favorite tracks are searched."}} else if query.is_empty() {"Add music in Vela’s web app, then refresh this library. Your tracks stay private."} else {"Try a different title, artist or album. Search uses only your own library."}}}}
            else {div {class:"native-track-table",div {class:"native-track-header","aria-hidden":"true",span {""} span {"TITLE / ARTIST"} span {"ALBUM"} span {"TIME"} span {"QUEUE"}}
                for (at,track) in catalog.page.items.iter().cloned().enumerate() { {
                    let play_ui=ui.clone();let played_catalog=catalog.clone();let next_ui=ui.clone();let next_catalog=catalog.clone();let last_ui=ui.clone();let last_catalog=catalog.clone();let next_track=track.clone();let last_track=track.clone();let favorite_ui=ui.clone();let favorite_catalog=catalog.clone();let favorite_track=track.clone();let offline_ui=ui.clone();let offline_track=track.clone();let offline_saved=offline_ids.contains(&track.id);let offline_working=offline_busy==track.id;let offline_title=track.title.clone();
                    let current=ui.current_track().is_some_and(|current|current.id==track.id);
                    let favorite_disabled=ui.favorite_busy.read().as_str()==track.id;
                    rsx! {article {class:if current {"native-track current"} else {"native-track"},key:"{track.id}","data-native-track-id":"{track.id}",
                        button {class:"icon-button track-start",disabled:!ui.audio_ready(),title:if !ui.audio_ready() {"Native audio output is unavailable"} else {"Play original"},"aria-label":"Play {track.title}",onclick:move |_|play_catalog(play_ui.clone(),played_catalog.clone(),at),Icon {name:"play"}}
                        div {class:"native-track-name",div {class:"neutral-cover","aria-hidden":"true",Icon {name:"music"}} div {strong {"{track.title}"} small {"{track.artist}"}}}
                        span {class:"native-track-album","{track.album}"}
                        span {class:"native-track-time","{time_label(track.duration_ms.max(0) as u64)}"}
                        div {class:"native-track-actions",
                            button {class:if track.favorite {"icon-button selected"} else {"icon-button"},"aria-pressed":track.favorite,disabled:favorite_disabled,"aria-label":if track.favorite {format!("Remove {} from favorites",track.title)} else {format!("Favorite {}",track.title)},title:if favorite_disabled {"Updating this favorite…"} else if track.favorite {"Remove from favorites"} else {"Add to favorites"},onclick:move |_|toggle_favorite(favorite_ui.clone(),favorite_catalog.clone(),favorite_track.clone()),Icon {name:"heart"}}
                            button {class:"icon-button","aria-label":"Play {track.title} next",title:"Play next",onclick:move |_|{if next_ui.catalog_current(&next_catalog) {enqueue(next_ui.clone(),next_track.clone(),true);}},Icon {name:"next-add"}}
                            button {class:"icon-button","aria-label":"Add {track.title} to queue",title:"Add to queue",onclick:move |_|{if last_ui.catalog_current(&last_catalog){enqueue(last_ui.clone(),last_track.clone(),false);}},Icon {name:"plus"}}
                            button {class:"icon-button",disabled:offline_saved||offline_working,"aria-label":if offline_saved {format!("Already saved offline: {offline_title}")} else if offline_working {format!("Saving offline: {offline_title}")} else {format!("Save {offline_title} offline")},title:if offline_saved {"Saved on this device"} else if offline_working {"Downloading a private copy…"} else {"Save a private copy for offline listening"},onclick:move |_|download_offline(offline_ui.clone(),offline_track.clone()),Icon {name:"down"}}
                        }
                    } }
                } }
            } }
            div {class:"native-pagination",button {class:"button secondary",disabled:ui.cursor.peek().is_none(),onclick:move |_|{let query=first.query.peek().clone();let favorites=first.catalog.peek().as_ref().is_some_and(|c|c.favorites);load_catalog(first.clone(),query,None,favorites);},"First page"}
                if let Some(cursor)=catalog.page.next_cursor.clone() {button {class:"button secondary",onclick:move |_|{let query=next.query.peek().clone();let favorites=next.catalog.peek().as_ref().is_some_and(|c|c.favorites);load_catalog(next.clone(),query,Some(cursor.clone()),favorites);},"Next page"}}
                small {"Only this page is queued when you play a result."}
            }
        }
    } }
}

fn load_groups(mut ui: Ui, kind: &'static str, cursor: Option<String>) {
    if !matches!(kind, "albums" | "artists") {
        return;
    }
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    ui.browse_gate.cancel();
    ui.view.set(kind);
    ui.browse_kind.set(kind);
    ui.browse_id.set(None);
    ui.browse_title.set(String::new());
    ui.browse_tracks.set(Vec::new());
    ui.browse_loading.set(true);
    ui.browse_error.set(String::new());
    let ticket = ui.browse_gate.begin(account.epoch);
    spawn(async move {
        if kind == "albums" {
            let result = account
                .client
                .albums(cursor.as_deref(), ticket.cancel.clone())
                .await;
            if !ticket.current(&ui) || *ui.view.peek() != "albums" {
                return;
            }
            ui.browse_loading.set(false);
            match result {
                Ok(page) => {
                    ui.browse_error.set(String::new());
                    ui.albums.set(page.items);
                }
                Err(error) => {
                    if !expired(ui.clone(), &error) {
                        ui.browse_error.set(error);
                    }
                }
            }
        } else {
            let result = account
                .client
                .artists(cursor.as_deref(), ticket.cancel.clone())
                .await;
            if !ticket.current(&ui) || *ui.view.peek() != "artists" {
                return;
            }
            ui.browse_loading.set(false);
            match result {
                Ok(page) => {
                    ui.browse_error.set(String::new());
                    ui.artists.set(page.items);
                }
                Err(error) => {
                    if !expired(ui.clone(), &error) {
                        ui.browse_error.set(error);
                    }
                }
            }
        }
    });
}

fn open_group(mut ui: Ui, kind: &'static str, id: String) {
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    if !uuid::Uuid::parse_str(&id).is_ok_and(|parsed| parsed.to_string() == id) {
        ui.browse_error.set("Select a valid group.".into());
        return;
    }
    ui.browse_gate.cancel();
    ui.browse_kind.set(kind);
    ui.browse_id.set(Some(id.clone()));
    ui.browse_title.set(String::new());
    ui.browse_tracks.set(Vec::new());
    ui.browse_loading.set(true);
    ui.browse_error.set(String::new());
    let ticket = ui.browse_gate.begin(account.epoch);
    spawn(async move {
        let result = if kind == "albums" {
            match account.client.album(&id, ticket.cancel.clone()).await {
                Ok(album) => account
                    .client
                    .album_tracks(&id, None, ticket.cancel.clone())
                    .await
                    .map(|page| (format!("{} · {}", album.title, album.artist), page.items)),
                Err(error) => Err(error),
            }
        } else {
            match account.client.artist(&id, ticket.cancel.clone()).await {
                Ok(artist) => account
                    .client
                    .artist_tracks(&id, None, ticket.cancel.clone())
                    .await
                    .map(|page| (artist.name, page.items)),
                Err(error) => Err(error),
            }
        };
        if !ticket.current(&ui)
            || *ui.view.peek() != kind
            || ui.browse_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.browse_loading.set(false);
        match result {
            Ok((title, tracks)) => {
                ui.browse_error.set(String::new());
                ui.browse_title.set(title);
                ui.browse_tracks.set(tracks);
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.browse_error.set(error);
                }
            }
        }
    });
}

fn play_group(mut ui: Ui) {
    if !ui.signed_in() || ui.audio.peek().is_none() || ui.browse_id.peek().is_none() {
        return;
    }
    let tracks = ui.browse_tracks.peek().clone();
    if tracks.is_empty() {
        return;
    }
    let title = ui.browse_title.peek().clone();
    ui.order.write().reset(tracks.len(), 0);
    ui.queue.set(tracks);
    ui.context.set(title);
    start_source(ui, false, true, 0);
}

fn playlists_view(ui: Ui) -> Element {
    let loading = *ui.playlist_loading.read();
    let error = ui.playlist_error.read().clone();
    let lists = ui.playlists.read().clone();
    let folders = ui.folders.read().clone();
    let folder_filter_value = ui.folder_filter.read().clone();
    let filtered_lists = lists
        .iter()
        .filter(|list| {
            folder_filter_value
                .as_deref()
                .is_none_or(|id| list.folder_id.as_deref() == Some(id))
        })
        .cloned()
        .collect::<Vec<_>>();
    let folder_busy = *ui.folder_busy.read();
    let folder_confirm = ui.folder_confirm.read().clone();
    let playlist_folder_target_value = ui.playlist_folder_target.read().clone();
    let mut playlist_folder_target = ui.playlist_folder_target;
    let folder_create = ui.clone();
    let folder_rename_ui = ui.clone();
    let mut folder_name = ui.folder_name_draft;
    let mut folder_rename = ui.folder_rename_draft;
    let detail = ui.playlist.read().clone();
    let refresh = ui.clone();
    let back = ui.clone();
    let play = ui.clone();
    let create = ui.clone();
    let rename = ui.clone();
    let delete = ui.clone();
    let busy = ui.playlist_busy;
    let candidates = ui.playlist_candidates.read().clone();
    let mut candidate = ui.playlist_candidate;
    let add = ui.clone();
    let move_folder = ui.clone();
    let select_all = ui.clone();
    let mut draft = ui.playlist_name_draft;
    let mut confirm = ui.playlist_delete_confirm;
    let is_owner = detail.as_ref().is_some_and(|detail| {
        ui.account
            .peek()
            .as_ref()
            .is_some_and(|account| account.user.id == detail.playlist.owner_id)
    });
    rsx! {section {class:"native-library","aria-label":"Playlists",
        div {class:"library-intro",div {span {class:"eyebrow","YOUR COLLECTIONS"} h1 {"Playlists, ready to play"} p {"Owner-authorized playlist metadata. Audio still needs the same per-track stream rights as the library."}}
            button {class:"button secondary",disabled:loading,onclick:move |_|{load_playlists(refresh.clone());},Icon {name:"history"} "Refresh playlists"}
        }
        if loading {div {class:"native-loading",role:"status",span {class:"loading-mark","aria-hidden":"true"} "Loading playlists…"}}
        if !error.is_empty() {div {class:"alert error",role:"alert","{error}"}}
        if let Some(detail) = detail {
            div { class: "library-intro",
                div {
                    span { class: "eyebrow", "PLAYLIST" }
                    h1 { "data-testid": "native-playlist-name", "{detail.playlist.name}" }
                    p { "{detail.items.len()} tracks · Revision {detail.playlist.revision}" if detail.playlist.can_edit { " · editable" } else { " · shared" } }
                }
            }
            div { class: "native-track-actions",
                button { class: "button secondary small", onclick: move |_| { let mut ui = back.clone(); ui.playlist.set(None); ui.playlist_id.set(None); }, "Back to playlists" }
                button { class: "button primary small", disabled: !ui.audio_ready() || detail.items.iter().all(|item| !item.can_stream), onclick: move |_| play_playlist(play.clone()), Icon { name: "play" } "Play playlist" }
            }
            if is_owner {
                div { class: "inline-form",
                    label { class: "sr-only", r#for: "native-playlist-rename", "Playlist name" }
                    input { id: "native-playlist-rename", maxlength: 120, value: "{draft}", disabled: *busy.read(), oninput: move |event| draft.set(event.value()) }
                    button { class: "button secondary small", disabled: *busy.read() || draft.read().trim().is_empty(), onclick: move |_| rename_selected_playlist(rename.clone()), if *busy.read() { "Saving…" } else { "Save name" } }
                    if *confirm.read() {
                        button { class: "button danger small", disabled: *busy.read(), onclick: move |_| delete_selected_playlist(delete.clone()), "Confirm delete" }
                        button { class: "button secondary small", onclick: move |_| confirm.set(false), "Cancel" }
                    } else {
                        button { class: "button danger small", disabled: *busy.read(), onclick: move |_| confirm.set(true), "Delete playlist" }
                    }
                }
                if *confirm.read() { p { class: "muted", role: "status", "Deleting removes this playlist and its share links. Your original tracks and audio stay." } }
                div { class: "inline-form",
                    label { class: "sr-only", r#for: "native-playlist-track", "Library track" }
                    select { id: "native-playlist-track", value: "{candidate}", disabled: *busy.read(), onchange: move |event| candidate.set(event.value()),
                        option { value: "", "Choose a library track" }
                        for track in candidates.iter().cloned() {
                            option { value: "{track.id}", "{track.title} · {track.artist}" }
                        }
                    }
                    button { class: "button secondary small", disabled: *busy.read() || candidate.read().is_empty(), onclick: move |_| add_selected_playlist_track(add.clone()), "Add selected track" }
                }
                div { class: "inline-form",
                    label { class: "sr-only", r#for: "native-playlist-folder", "Playlist folder" }
                    select { id: "native-playlist-folder", value: "{playlist_folder_target_value}", disabled: *busy.read() || folder_busy, onchange: move |event| playlist_folder_target.set(event.value()),
                        option { value: "", "Unfiled" }
                        for folder in folders.iter().cloned() { option { value: "{folder.id}", "{folder.name}" } }
                    }
                    button { class: "button secondary small", disabled: *busy.read() || folder_busy, onclick: move |_| move_playlist_folder(move_folder.clone()), "Move to folder" }
                }
            }
            if detail.items.is_empty() {
                div { class: "native-empty", Icon { name: "queue" } h2 { "This playlist is empty" } p { "Add an owned track above, or manage it from the web app." } }
            } else {
                div { class: "native-queue-list",
                    for (at, entry) in detail.items.iter().cloned().enumerate() {
                        {
                            let up = ui.clone();
                            let down = ui.clone();
                            let remove = ui.clone();
                            let item_id = entry.id.clone();
                            const LAST_HINT: &str = "Already the last playlist entry.";
                            rsx! { article { class: "native-queue-row", key: "{entry.id}",
                                div { class: "neutral-cover", "aria-hidden": "true", Icon { name: "music" } }
                                div { class: "native-queue-select", div { strong { "{entry.track.title}" } small { "{entry.track.artist}" } } }
                                if entry.can_stream { span { class: "queue-current", "Playable" } } else { small { "Metadata only" } }
                                if detail.playlist.can_edit {
                                    button { class: "icon-button", disabled: at == 0 || *busy.read(), "aria-label": "Move playlist entry {at + 1} up", title: if at == 0 { "Already the first playlist entry." } else { "Move this entry one place earlier." }, onclick: move |_| move_playlist_entry(up.clone(), at, -1), Icon { name: "up" } }
                                    button { class: "icon-button", disabled: at + 1 >= detail.items.len() || *busy.read(), "aria-label": "Move playlist entry {at + 1} down", title: if at + 1 >= detail.items.len() { LAST_HINT } else { "Move this entry one place later." }, onclick: move |_| move_playlist_entry(down.clone(), at, 1), Icon { name: "down" } }
                                    button { class: "icon-button", disabled: *busy.read(), "aria-label": "Remove playlist entry {at + 1}", title: "Remove this entry from the playlist; the original track is unchanged.", onclick: move |_| remove_playlist_entry(remove.clone(), item_id.clone()), Icon { name: "close" } }
                                }
                            } }
                        }
                    }
                }
            }
        } else {
            div { class: "catalog-caption", p { "Playlist folders" } span { "PRIVATE GROUPING" } }
            form { class: "inline-form", onsubmit: move |event| { event.prevent_default(); create_folder(folder_create.clone()); },
                label { class: "sr-only", r#for: "native-folder-name", "New folder name" }
                input { id: "native-folder-name", maxlength: 120, placeholder: "New folder name", value: "{folder_name}", disabled: folder_busy, oninput: move |event| folder_name.set(event.value()) }
                button { class: "button primary small", r#type: "submit", disabled: folder_busy || folder_name.read().trim().is_empty(), if folder_busy { "Creating…" } else { "Create folder" } }
            }
            div { class: "inline-form", role: "group", "aria-label": "Folder filters",
                button { class: if folder_filter_value.is_none() { "button secondary small" } else { "button small" }, "aria-pressed": folder_filter_value.is_none(), onclick: move |_| select_folder(select_all.clone(), None), "All playlists" }
                for folder in folders.iter().cloned() {
                    {
                        let filter = folder.id.clone();
                        let select_ui = ui.clone();
                        let delete_ui = ui.clone();
                        let set_confirm_ui = ui.clone();
                        let folder_id = folder.id.clone();
                        let confirm_id = folder.id.clone();
                        let name = folder.name.clone();
                        let selected = folder_filter_value.as_deref() == Some(folder.id.as_str());
                        rsx! { div { class: "inline-form", "data-native-folder": "{folder.id}",
                            button { class: if selected { "button secondary small" } else { "button small" }, "aria-pressed": selected, onclick: move |_| select_folder(select_ui.clone(), Some(filter.clone())), "{name} ({folder.playlist_count})" }
                            if folder_confirm == folder_id {
                                button { class: "button danger small", "aria-label": "Confirm delete folder {folder.name}", onclick: move |_| delete_folder(delete_ui.clone(), confirm_id.clone()), "Confirm folder delete" }
                                button { class: "button secondary small", onclick: move |_| set_confirm_ui.clone().folder_confirm.set(String::new()), "Keep folder" }
                            } else {
                                button { class: "icon-button", "aria-label": "Delete folder {folder.name}", onclick: move |_| set_confirm_ui.clone().folder_confirm.set(folder_id.clone()), Icon { name: "close" } }
                            }
                        } }
                    }
                }
            }
            if let Some(folder) = folders.iter().find(|folder| folder_filter_value.as_deref() == Some(folder.id.as_str())).cloned() {
                form { class: "inline-form", onsubmit: move |event| { event.prevent_default(); rename_folder(folder_rename_ui.clone()); },
                    label { class: "sr-only", r#for: "native-folder-rename", "Folder name" }
                    input { id: "native-folder-rename", maxlength: 120, value: "{folder_rename}", disabled: folder_busy, oninput: move |event| folder_rename.set(event.value()) }
                    button { class: "button secondary small", r#type: "submit", disabled: folder_busy || folder_rename.read().trim().is_empty(), "Save folder name" }
                }
                small { "Deleting this folder unfiles its playlists; their tracks and shares stay." }
                if folder_confirm == folder.id { "" }
            }
            form { class: "inline-form", onsubmit: move |event| { event.prevent_default(); create_playlist_ui(create.clone()); },
                label { class: "sr-only", r#for: "native-playlist-name", "New playlist name" }
                input { id: "native-playlist-name", maxlength: 120, placeholder: "New playlist name", value: "{draft}", disabled: *busy.read(), oninput: move |event| draft.set(event.value()) }
                button { class: "button primary small", r#type: "submit", disabled: *busy.read() || draft.read().trim().is_empty(), if *busy.read() { "Creating…" } else { "Create playlist" } }
            }
            if !loading && lists.is_empty() && error.is_empty() {
                div { class: "native-empty", Icon { name: "queue" } h2 { "No playlists yet" } p { "Create one above with your own account, or create it in the web app." } }
            }
            div { class: "native-queue-list",
                for list in filtered_lists {
                    {
                        let open = ui.clone();
                        let list_id = list.id.clone();
                        rsx! { article { class: "native-queue-row", key: "{list.id}",
                            button { class: "native-queue-select", "aria-label": "Open playlist {list.name}", onclick: move |_| open_playlist(open.clone(), list_id.clone()),
                                div { class: "neutral-cover", "aria-hidden": "true", Icon { name: "queue" } }
                                div { strong { "{list.name}" } small { "{list.item_count} tracks" if !list.can_edit { " · shared" } } }
                            }
                        } }
                    }
                }
            }
        }
    } }
}

fn explore_public(mut ui: Ui) {
    let origin = ui.remote_origin.peek().trim().to_string();
    if origin.is_empty() {
        ui.remote_error.set("Enter a public server origin.".into());
        return;
    }
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    ui.remote_gate.cancel();
    ui.view.set("public");
    ui.remote_loading.set(true);
    ui.remote_error.set(String::new());
    ui.remote_preview.set(None);
    let ticket = ui.remote_gate.begin(account.epoch);
    spawn(async move {
        let result = crate::remote::preview(&origin, ticket.cancel.clone()).await;
        if !ticket.current(&ui) || *ui.view.peek() != "public" {
            return;
        }
        ui.remote_loading.set(false);
        match result {
            Ok(preview) => {
                ui.remote_error.set(String::new());
                ui.remote_preview.set(Some(preview));
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.remote_error.set(error);
                }
            }
        }
    });
}

fn play_public(mut ui: Ui, track: PublicTrack, origin: String) {
    if !track.can_stream || !ui.signed_in() {
        return;
    }
    ui.notice.set("Opening public track…".into());
    ui.remote_queue
        .set(vec![crate::remote::RemoteItem { origin, track }]);
    ui.remote_index.set(0);
    start_remote(ui);
}

fn play_public_result(mut ui: Ui, origin: String, track_id: String) {
    let mut items = Vec::new();
    for peer in ui.remote_results.peek().clone() {
        for track in peer.items {
            if track.can_stream {
                items.push(crate::remote::RemoteItem {
                    origin: peer.origin.clone(),
                    track,
                });
            }
        }
    }
    let Some(index) = items
        .iter()
        .position(|item| item.origin == origin && item.track.id == track_id)
    else {
        return;
    };
    ui.notice.set("Opening public track…".into());
    ui.remote_queue.set(items);
    ui.remote_index.set(index);
    start_remote(ui);
}

fn start_remote(mut ui: Ui) {
    if !ui.signed_in() || *ui.view.peek() != "public" {
        return;
    }
    ui.offline_active.set(false);
    let Some(item) = ui.remote_queue.peek().get(*ui.remote_index.peek()).cloned() else {
        return;
    };
    if !item.track.can_stream {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let Some(audio) = ui.audio.peek().clone() else {
        return;
    };
    let origin = item.origin.clone();
    let track = item.track.clone();
    let selection = ui.new_selection();
    // Switching to a public item must not leave a local occurrence current:
    // an in-flight local EOF/advance must not replace the public selection.
    ui.order.write().reset(0, 0);
    let _ = audio.stop(selection);
    ui.prepared.set(None);
    ui.remote_now.set(Some(crate::remote::RemoteNow {
        id: track.id.clone(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        origin: origin.clone(),
    }));
    ui.context.set(format!("Public · {origin}"));
    ui.preparing.set(true);
    let ticket = ui.source_gate.begin(account.epoch);
    spawn(async move {
        let result = crate::remote::prepare(&origin, &track, ticket.cancel.clone()).await;
        if !ticket.current(&ui)
            || *ui.selection.peek() != selection
            || ui
                .remote_now
                .peek()
                .as_ref()
                .map(|remote| remote.id.as_str())
                != Some(track.id.as_str())
            || ui
                .remote_queue
                .peek()
                .get(*ui.remote_index.peek())
                .map(|item| item.track.id.as_str())
                != Some(track.id.as_str())
        {
            return;
        }
        ui.preparing.set(false);
        match result {
            Ok(prepared) => match audio.load(selection, prepared.clone(), true, 0) {
                Ok(()) => ui.prepared.set(Some(prepared)),
                Err(error) => {
                    ui.source_error.set(error);
                    ui.remote_now.set(None);
                }
            },
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.source_error.set(error);
                }
                ui.remote_now.set(None);
            }
        }
    });
}

fn remote_step(mut ui: Ui, delta: i32) {
    if ui.remote_now.peek().is_none() {
        return;
    }
    let Some(next) =
        crate::remote::queue_target(*ui.remote_index.peek(), ui.remote_queue.peek().len(), delta)
    else {
        return;
    };
    ui.remote_index.set(next);
    start_remote(ui);
}

fn player_previous(ui: Ui) {
    if *ui.offline_active.peek() {
        offline_step(ui, -1);
        return;
    }
    if ui.remote_now.peek().is_some() {
        remote_step(ui, -1);
    } else {
        advance(ui, Command::Previous);
    }
}

fn player_next(ui: Ui) {
    if *ui.offline_active.peek() {
        offline_step(ui, 1);
        return;
    }
    if ui.remote_now.peek().is_some() {
        remote_step(ui, 1);
    } else {
        advance(ui, Command::Next);
    }
}

fn load_more_public(mut ui: Ui) {
    let Some(preview) = ui.remote_preview.peek().clone() else {
        return;
    };
    let Some(cursor) = preview.next_cursor.clone() else {
        return;
    };
    if *ui.remote_more_busy.peek() || !ui.signed_in() || *ui.view.peek() != "public" {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let origin = preview.origin;
    let ticket = ui.remote_gate.begin(account.epoch);
    ui.remote_more_busy.set(true);
    ui.remote_more_error.set(String::new());
    spawn(async move {
        let result = crate::remote::more(&origin, &cursor, ticket.cancel.clone()).await;
        if !ticket.current(&ui) || *ui.view.peek() != "public" {
            return;
        }
        let current = ui.remote_preview.peek().clone();
        if current.as_ref().is_none_or(|preview| {
            preview.origin != origin || preview.next_cursor.as_deref() != Some(cursor.as_str())
        }) {
            return;
        }
        ui.remote_more_busy.set(false);
        match result {
            Ok(page) if page.origin == origin => {
                let mut current = current.expect("preview checked above");
                let mut seen = current
                    .items
                    .iter()
                    .map(|track| track.id.clone())
                    .collect::<std::collections::HashSet<_>>();
                for track in page.items {
                    if seen.insert(track.id.clone()) {
                        current.items.push(track);
                    }
                }
                current.next_cursor = page.next_cursor;
                ui.remote_preview.set(Some(current));
            }
            Ok(_) => ui
                .remote_more_error
                .set("The public server changed origin; the page was ignored.".into()),
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.remote_more_error.set(error);
                }
            }
        }
    });
}

fn search_saved_peers(mut ui: Ui) {
    let query = ui.remote_query.peek().trim().to_string();
    if query.is_empty() {
        ui.remote_search_error.set("Enter a public search.".into());
        return;
    }
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let origins = ui.saved_peers.peek().clone();
    if origins.is_empty() {
        ui.remote_search_error
            .set("Save at least one public server first.".into());
        return;
    }
    ui.remote_gate.cancel();
    ui.remote_searching.set(true);
    ui.remote_search_error.set(String::new());
    ui.remote_results.set(Vec::new());
    let ticket = ui.remote_gate.begin(account.epoch);
    spawn(async move {
        let result = crate::remote::search_peers(&origins, &query, ticket.cancel.clone()).await;
        if !ticket.current(&ui) || *ui.view.peek() != "public" {
            return;
        }
        ui.remote_searching.set(false);
        match result {
            Ok(results) => ui.remote_results.set(results),
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.remote_search_error.set(error);
                }
            }
        }
    });
}

fn load_more_peer(mut ui: Ui, origin: String) {
    if !ui.remote_peer_more_busy.peek().is_empty() || *ui.view.peek() != "public" {
        return;
    }
    let Some(peer) = ui
        .remote_results
        .peek()
        .iter()
        .find(|peer| peer.origin == origin)
        .cloned()
    else {
        return;
    };
    let Some(cursor) = peer.next_cursor.clone() else {
        return;
    };
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let ticket = ui.remote_gate.begin(account.epoch);
    ui.remote_peer_more_busy.set(origin.clone());
    ui.remote_peer_more_error.set(String::new());
    spawn(async move {
        let result = crate::remote::more(&origin, &cursor, ticket.cancel.clone()).await;
        if !ticket.current(&ui) || *ui.view.peek() != "public" {
            return;
        }
        let mut results = ui.remote_results.peek().clone();
        let Some(index) = results.iter().position(|peer| {
            peer.origin == origin && peer.next_cursor.as_deref() == Some(cursor.as_str())
        }) else {
            return;
        };
        ui.remote_peer_more_busy.set(String::new());
        match result {
            Ok(page) if page.origin == origin => {
                let mut seen = results[index]
                    .items
                    .iter()
                    .map(|track| track.id.clone())
                    .collect::<std::collections::HashSet<_>>();
                for track in page.items {
                    if seen.insert(track.id.clone()) {
                        results[index].items.push(track);
                    }
                }
                results[index].next_cursor = page.next_cursor;
                ui.remote_results.set(results);
            }
            Ok(_) => ui
                .remote_peer_more_error
                .set("The public server changed origin; the page was ignored.".into()),
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.remote_peer_more_error.set(error);
                }
            }
        }
    });
}

fn retry_peer(mut ui: Ui, origin: String) {
    if !ui.remote_retry_busy.peek().is_empty() || *ui.view.peek() != "public" {
        return;
    }
    let query = ui.remote_query.peek().trim().to_string();
    if query.is_empty() {
        return;
    }
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let ticket = ui.remote_gate.begin(account.epoch);
    ui.remote_retry_busy.set(origin.clone());
    ui.remote_retry_error.set(String::new());
    spawn(async move {
        let result = crate::remote::search_peers(
            std::slice::from_ref(&origin),
            &query,
            ticket.cancel.clone(),
        )
        .await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "public"
            || ui.remote_query.peek().trim() != query
        {
            return;
        }
        let mut results = ui.remote_results.peek().clone();
        let Some(index) = results.iter().position(|peer| peer.origin == origin) else {
            return;
        };
        ui.remote_retry_busy.set(String::new());
        match result {
            Ok(peers) if peers.len() == 1 && peers[0].origin == origin => {
                results[index] = peers[0].clone();
                ui.remote_results.set(results);
            }
            Ok(_) => ui
                .remote_retry_error
                .set("The public server returned an unexpected result.".into()),
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.remote_retry_error.set(error);
                }
            }
        }
    });
}

fn save_public_peer(mut ui: Ui) {
    let Some(preview) = ui.remote_preview.peek().clone() else {
        return;
    };
    let mut peers = ui.saved_peers.peek().clone();
    if !peers.contains(&preview.origin) {
        peers.push(preview.origin);
        peers.sort();
    }
    match crate::peers::save(&peers) {
        Ok(()) => {
            ui.saved_peers.set(peers);
            ui.remote_error.set(String::new());
            ui.notice.set("Public server saved.".into());
        }
        Err(error) => ui.remote_error.set(error),
    }
}

fn remove_public_peer(mut ui: Ui, origin: String) {
    let mut peers = ui.saved_peers.peek().clone();
    peers.retain(|peer| peer != &origin);
    match crate::peers::save(&peers) {
        Ok(()) => {
            ui.saved_peers.set(peers);
            ui.notice.set("Public server removed.".into());
        }
        Err(error) => ui.remote_error.set(error),
    }
}

fn open_saved_peer(mut ui: Ui, origin: String) {
    ui.remote_origin.set(origin);
    explore_public(ui);
}

fn public_view(ui: Ui) -> Element {
    let loading = *ui.remote_loading.read();
    let error = ui.remote_error.read().clone();
    let preview = ui.remote_preview.read().clone();
    let saved = ui.saved_peers.read().clone();
    let searching = *ui.remote_searching.read();
    let search_error = ui.remote_search_error.read().clone();
    let results = ui.remote_results.read().clone();
    let explore = ui.clone();
    let save_current = ui.clone();
    let search = ui.clone();
    let more = ui.clone();
    let more_busy = *ui.remote_more_busy.read();
    let more_error = ui.remote_more_error.read().clone();
    let peer_more_busy = ui.remote_peer_more_busy.read().clone();
    let peer_more_error = ui.remote_peer_more_error.read().clone();
    let retry_busy = ui.remote_retry_busy.read().clone();
    let retry_error = ui.remote_retry_error.read().clone();
    let mut query = ui.remote_query;
    let mut origin = ui.remote_origin;
    rsx! {section {class:"native-library","aria-label":"Public catalogs",
        div {class:"library-intro",div {span {class:"eyebrow","OPT-IN PUBLIC NETWORK"} h1 {"Public catalogs"} p {"Fetch one explicitly published catalog over its advertised origin. Private libraries, history and credentials are never sent."}}}
        form {class:"native-search",onsubmit:move |event|{event.prevent_default();explore_public(explore.clone());},
            Icon {name:"network"} label {class:"sr-only",r#for:"native-peer-origin","Public server origin"}
            input {id:"native-peer-origin",r#type:"url",maxlength:2048,value:"{origin}",placeholder:"https://music.example.org",oninput:move |event|origin.set(event.value())}
            button {class:"button primary",r#type:"submit",disabled:loading,if loading {"Exploring…"} else {"Explore public catalog"}}
        }
        if !saved.is_empty() {
            form { class: "native-search", onsubmit: move |event| { event.prevent_default(); search_saved_peers(search.clone()); },
                Icon { name: "search" } label { class: "sr-only", r#for: "native-peer-query", "Search saved public servers" }
                input { id: "native-peer-query", maxlength: 128, value: "{query}", placeholder: "Search all saved public servers", oninput: move |event| query.set(event.value()) }
                button { class: "button primary", r#type: "submit", disabled: searching, if searching { "Searching…" } else { "Search saved servers" } }
            }
            if !search_error.is_empty() { div { class: "alert error", role: "alert", "{search_error}" } }
            if !results.is_empty() {
                div { class: "catalog-caption", p { "Unified saved-server results" } span { "{results.len()} SERVERS" } }
                for peer in results.iter().cloned() {
                    {
                        let peer_origin = peer.origin.clone();
                        let peer_label = peer.origin.clone();
                        let peer_error = peer.error.clone();
                        let peer_error_text = peer.error.clone().unwrap_or_default();
                        let peer_cached = peer.cached;
                        let peer_has_more = peer.next_cursor.is_some();
                        let peer_more_origin = peer.origin.clone();
                        let peer_cached_age = peer.cached_at.map(cached_age_label).unwrap_or_default();
                        let retry_origin = peer.origin.clone();
                        let retry_ui = ui.clone();
                        let more_ui = ui.clone();
                        rsx! {
                            article { class: "native-queue-row", key: "search-{peer.origin}", "data-public-origin": "{peer.origin}", "data-public-peer-summary": "true", "data-public-peer-error": "{peer_error_text}", "data-public-peer-cached": "{peer.cached}",
                                div { class: "neutral-cover", "aria-hidden": "true", Icon { name: "network" } }
                                div { class: "native-queue-select",
                                    div { strong { "Public origin" } small { "{peer_label}" } }
                                }
                                if peer_cached { small { "Cached metadata · {peer_cached_age} · stream unverified" } button { class: "button secondary small", "aria-label": "Retry live for {peer.origin}", disabled: retry_busy == peer.origin, onclick: move |_| retry_peer(retry_ui.clone(), retry_origin.clone()), if retry_busy == retry_origin { "Retrying…" } else { "Retry live" } } } else if peer_error.is_some() { small { "Unavailable" } } else { span { class: "queue-current", "{peer.items.len()} results" } }
                            }
                            if let Some(error) = peer_error { p { class: "muted", "{error}" } }
                            div { class: "native-queue-list",
                                for track in peer.items.iter().cloned() {
                                    {
                                        let play_ui = ui.clone();
                                        let play_track = track.clone();
                                        let play_origin = peer_origin.clone();
                                        let track_title = track.title.clone();
                                        rsx! { article { class: "native-queue-row", key: "{peer.origin}-{track.id}", "data-public-track-id": "{track.id}", "data-public-origin": "{peer.origin}", "data-public-cached": "{peer_cached}",
                                            div { class: "neutral-cover", "aria-hidden": "true", Icon { name: "music" } }
                                            div { class: "native-queue-select", div { strong { "{track.title}" } small { "{track.artist} · {track.album}" } small { "{peer.origin}" } } }
                                            span { class: "native-track-time", "{time_label(track.duration_ms.max(0) as u64)}" }
                                            if track.can_stream && !peer_cached {
                                                button { class: "button primary small", disabled: !ui.audio_ready(), "aria-label": "Play public track {track_title}", onclick: move |_| play_public_result(play_ui.clone(), play_origin.clone(), play_track.id.clone()), Icon { name: "play" } "Play public track" }
                                            } else if peer_cached { small { "Cached metadata only" } } else { small { "Metadata only" } }
                                        } }
                                    }
                                }
                            }
                            if peer_has_more {
                                button { class: "button secondary small", "aria-label": "Load more from {peer_more_origin}", disabled: peer_more_busy == peer_more_origin, onclick: move |_| load_more_peer(more_ui.clone(), peer_more_origin.clone()), if peer_more_busy == peer_more_origin { "Loading more…" } else { "Load more from this server" } }
                            }
                        }
                    }
                }
            }
            if !peer_more_error.is_empty() { p { class: "alert error", role: "alert", "{peer_more_error}" } }
            if !retry_error.is_empty() { p { class: "alert error", role: "alert", "{retry_error}" } }
            div { class: "catalog-caption", p { "Saved public servers" } span { "NO SECRETS" } }
            div { class: "native-queue-list",
                for peer in saved.iter().cloned() {
                    {
                        let open = ui.clone();
                        let remove = ui.clone();
                        let peer_label = peer.clone();
                        let peer_open = peer_label.clone();
                        rsx! { article { class: "native-queue-row", key: "{peer}",
                            button { class: "native-queue-select", "aria-label": "Open saved server {peer_label}", onclick: move |_| open_saved_peer(open.clone(), peer_open.clone()),
                                div { class: "neutral-cover", "aria-hidden": "true", Icon { name: "network" } }
                                div { strong { "Saved public server" } small { "{peer_label}" } }
                            }
                            button { class: "icon-button", "aria-label": "Remove saved server {peer_label}", title: "Remove this saved origin; no server data is deleted.", onclick: move |_| remove_public_peer(remove.clone(), peer_label.clone()), Icon { name: "close" } }
                        } }
                    }
                }
            }
        }
        if loading {div {class:"native-loading",role:"status",span {class:"loading-mark","aria-hidden":"true"} "Reading the public catalog…"}}
        if !error.is_empty() {div {class:"alert error",role:"alert","{error}"}}
        if let Some(preview) = preview {
            div {class:"catalog-caption",p {"{preview.items.len()} explicit public records · origin {preview.origin}"} if !saved.contains(&preview.origin) { button { class: "button secondary small", onclick: move |_| save_public_peer(save_current.clone()), "Save this server" } } else { span { "SAVED" } }}
            if preview.items.is_empty() {
                div { class: "native-empty", Icon { name: "network" } h2 { "This public catalog is empty" } p { "The server exposes metadata only when its owner explicitly publishes a licensed track." } }
            } else {
                div { class: "native-queue-list",
                    for track in preview.items.iter().cloned() {
                        {
                            let play_ui = ui.clone();
                            let play_track = track.clone();
                            let track_title = track.title.clone();
                            let origin_label = preview.origin.clone();
                            let play_origin = preview.origin.clone();
                            rsx! { article { class: "native-queue-row", key: "{track.id}", "data-public-track-id": "{track.id}",
                                div { class: "neutral-cover", "aria-hidden": "true", Icon { name: "music" } }
                                div { class: "native-queue-select",
                                    div {
                                        strong { "{track.title}" }
                                        small { "{track.artist} · {track.album}" }
                                        small { "Public origin · {origin_label}" }
                                    }
                                }
                                span { class: "native-track-time", "{time_label(track.duration_ms.max(0) as u64)}" }
                                if track.can_stream {
                                    button { class: "button primary small", disabled: !ui.audio_ready(), "aria-label": "Play public track {track_title}", onclick: move |_| play_public(play_ui.clone(), play_track.clone(), play_origin.clone()), Icon { name: "play" } "Play public track" }
                                } else {
                                    small { "Metadata only" }
                                }
                            } }
                        }
                    }
                }
            }
            if preview.next_cursor.is_some() {
                div { class: "inline-form",
                    button { class: "button secondary", disabled: more_busy, onclick: move |_| load_more_public(more.clone()), if more_busy { "Loading more…" } else { "Load more public records" } }
                }
                if !more_error.is_empty() { p { class: "alert error", role: "alert", "{more_error}" } }
            }
        }
    } }
}

fn browse_view(ui: Ui) -> Element {
    let kind = *ui.browse_kind.read();
    let albums = ui.albums.read().clone();
    let artists = ui.artists.read().clone();
    let tracks = ui.browse_tracks.read().clone();
    let title = ui.browse_title.read().clone();
    let selected = ui.browse_id.read().is_some();
    let loading = *ui.browse_loading.read();
    let error = ui.browse_error.read().clone();
    let refresh = ui.clone();
    let back = ui.clone();
    let play = ui.clone();
    let group_label = if kind == "albums" { "album" } else { "artist" };
    let heading = if kind == "albums" {
        "Albums in your library"
    } else {
        "Artists in your library"
    };
    let refresh_label = format!("Refresh {group_label}s");
    rsx! {section {class:"native-library","aria-label":if kind=="albums" {"Albums"} else {"Artists"},
        div {class:"library-intro",div {span {class:"eyebrow","BROWSE YOUR OWN CATALOG"} h1 { "data-testid": "native-group-title", if selected {"{title}"} else {"{heading}"} } p {"Grouping is derived from the private catalog; playback still requires the owner-authorized original."}}
            button {class:"button secondary",disabled:loading,onclick:move |_|{let kind=*refresh.browse_kind.peek();load_groups(refresh.clone(),kind,None);},Icon {name:"history"} "{refresh_label}"}
        }
        if loading {div {class:"native-loading",role:"status",span {class:"loading-mark","aria-hidden":"true"} "Loading {group_label}s…"}}
        if !error.is_empty() {div {class:"alert error",role:"alert","{error}"}}
        if selected {
            div {class:"native-track-actions",button {class:"button secondary small",onclick:move |_|{let mut ui=back.clone();ui.browse_id.set(None);ui.browse_title.set(String::new());ui.browse_tracks.set(Vec::new());},"Back to {group_label}s"} button {class:"button primary small",disabled:!ui.audio_ready()||tracks.is_empty(),onclick:move |_|play_group(play.clone()),Icon {name:"play"} "Play {group_label}"}}
            if !tracks.is_empty() {div {class:"native-queue-list",for (at, track) in tracks.iter().cloned().enumerate() { article {class:"native-queue-row",key:"{at}-{track.id}",
                span {class:"queue-number","{at+1}"} div {class:"neutral-cover","aria-hidden":"true",Icon {name:"music"}} div {class:"native-queue-select",div {strong {"{track.title}"} small {"{track.artist}"}}}
                span {class:"native-track-time","{time_label(track.duration_ms.max(0) as u64)}"}
            } } } }
        } else {
            if kind == "albums" {
                if albums.is_empty() && !loading && error.is_empty() {div {class:"native-empty",Icon {name:"albums"} h2 {"No album groups yet"} p {"Uploaded or scanned tracks with album metadata will appear here."}}}
                div {class:"native-queue-list",for album in albums { { let open=ui.clone(); let id=album.id.clone(); rsx! { article {class:"native-queue-row",key:"{album.id}",
                    button {class:"native-queue-select","aria-label":"Open album {album.title} by {album.artist}",onclick:move |_|open_group(open.clone(),"albums",id.clone()),div {class:"neutral-cover","aria-hidden":"true",Icon {name:"albums"}} div {strong {"{album.title}"} small {"{album.artist} · {album.track_count} tracks"} }}
                } } } } }
            } else {
                if artists.is_empty() && !loading && error.is_empty() {div {class:"native-empty",Icon {name:"artists"} h2 {"No artist groups yet"} p {"Uploaded or scanned tracks with artist metadata will appear here."}}}
                div {class:"native-queue-list",for artist in artists { { let open=ui.clone(); let id=artist.id.clone(); rsx! { article {class:"native-queue-row",key:"{artist.id}",
                    button {class:"native-queue-select","aria-label":"Open artist {artist.name}",onclick:move |_|open_group(open.clone(),"artists",id.clone()),div {class:"neutral-cover","aria-hidden":"true",Icon {name:"artists"}} div {strong {"{artist.name}"} small {"{artist.track_count} tracks · {artist.album_count} albums"} }}
                } } } } }
            }
        }
    } }
}

fn queue_view(ui: Ui) -> Element {
    let queue = ui.queue.read().clone();
    let queue_empty = queue.is_empty();
    let queue_len = queue.len();
    let current = ui.order.read().current();
    let clear = ui.clone();
    let clear_upcoming = ui.clone();
    let has_upcoming = current.is_some_and(|current| current + 1 < queue_len);
    rsx! {section {class:"native-queue","aria-label":"Playback queue",
        div {class:"library-intro",div {span {class:"eyebrow","ROOM FOR YOUR NEXT LISTEN"} h1 {"Your listening queue"} p {"{queue_len} of 1,000 places · Duplicate tracks keep their own place."}}
            div {class:"inline-form",button {class:"button secondary",disabled:!has_upcoming,title:if has_upcoming {"Remove only the queued occurrences after the current one."} else {"Nothing is queued after the current occurrence."},onclick:move |_|queue_clear_upcoming(clear_upcoming.clone()),"Clear upcoming"} button {class:"button secondary",disabled:queue_empty,onclick:move |_|clear_queue(clear.clone()),"Clear queue"}}
        }
        if queue_empty {div {class:"native-empty",Icon {name:"queue"} h2 {"A little room for what comes next"} p {"Play a library result, use Play next, or add tracks to your queue."}}}
        else {div {class:"native-queue-list",for (at,track) in queue.into_iter().enumerate() { {
            let select=ui.clone();let up=ui.clone();let down=ui.clone();let remove=ui.clone();
            rsx! {article {class:if current==Some(at){"native-queue-row current"}else{"native-queue-row"},key:"{at}-{track.id}","data-native-queue-index":"{at}",
                button {class:"native-queue-select",disabled:!ui.audio_ready(),"aria-label":"Play queue occurrence {at}: {track.title}",onclick:move |_|select_queue(select.clone(),at),span {class:"queue-number","{at+1}"} div {class:"neutral-cover","aria-hidden":"true",Icon {name:"music"}} div {strong {"{track.title}"} small {"{track.artist}"}} if current==Some(at){span {class:"queue-current","Current"}}}
                button {class:"icon-button",disabled:at==0,"aria-label":"Move queue occurrence {at} up",title:"Move up",onclick:move |_|queue_edit(up.clone(),at,false),Icon {name:"up"}}
                button {class:"icon-button",disabled:at+1>=queue_len,"aria-label":"Move queue occurrence {at} down",title:if at+1>=queue_len {"Already the last queued occurrence."} else {"Move down"},onclick:move |_|queue_move_down(down.clone(),at),Icon {name:"down"}}
                button {class:"icon-button",disabled:current==Some(at),"aria-label":"Remove queue occurrence {at}",title:if current==Some(at){"Current occurrence cannot be removed; clear the queue to stop it"}else{"Remove from queue"},onclick:move |_|queue_edit(remove.clone(),at,true),Icon {name:"close"}}
            } }
        } } } }
        p {class:"queue-explanation","Shuffle follows a temporary fair deck and playback history. The visible list is saved queue order, not a prediction of shuffled playback."}
    } }
}

fn player(ui: Ui) -> Element {
    let track = ui.current_track();
    let remote = ui.remote_now.read().clone();
    let offline_active = *ui.offline_active.read();
    let offline_current = ui.order.read().current();
    let offline_len = ui.queue.read().len();
    let snapshot = ui.snapshot.read().clone();
    let preparing = *ui.preparing.read();
    let progress = *ui.progress.read();
    let error = ui.source_error.read().clone();
    let shuffle = *ui.shuffle.read();
    let repeat = *ui.repeat.read();
    let context = ui.context.read().clone();
    let toggle_ui = ui.clone();
    let previous = ui.clone();
    let next = ui.clone();
    let reconnect = ui.clone();
    let audio_alive = ui.audio_alive();
    let seek_ui = ui.clone();
    let volume_ui = ui.clone();
    let cancel = ui.clone();
    let shuffle_ui = ui.clone();
    let repeat_ui = ui.clone();
    let blocked = (track.is_none() && remote.is_none()) || !ui.audio_ready();
    let playing = snapshot.phase == AudioPhase::Playing;
    let can_seek = !blocked
        && !preparing
        && ui.prepared.peek().is_some()
        && snapshot.duration_ms > 0
        && matches!(
            snapshot.phase,
            AudioPhase::Playing | AudioPhase::Paused | AudioPhase::Ended
        );
    let status = if preparing {
        "Preparing original audio"
    } else {
        match snapshot.phase {
            AudioPhase::Idle => "Ready when you are",
            AudioPhase::Loading => "Opening native audio",
            AudioPhase::Playing => "Playing through native audio",
            AudioPhase::Paused => "Paused",
            AudioPhase::Ended => "Playback finished",
            AudioPhase::Error => "Playback needs attention",
        }
    };
    let repeat_label = match repeat {
        Repeat::Off => "Repeat off",
        Repeat::All => "Repeat all",
        Repeat::One => "Repeat one",
    };
    let favorite_disabled = track
        .as_ref()
        .is_some_and(|track| ui.favorite_busy.read().as_str() == track.id);
    rsx! {footer {class:"native-player","aria-label":"Native music player","data-audio-phase":"{snapshot.phase:?}","data-audio-generation":"{snapshot.generation}",
        div {class:"player-track",
            div {class:"neutral-cover player-cover","aria-hidden":"true",Icon {name:"music"}}
            div {if let Some(remote)=remote.clone() {strong {"{remote.title}"} small {"{remote.artist} · public"} } else if let Some(track)=track.clone() {strong {"{track.title}"} small {"{track.artist}"}}else{strong {"Ready when you are"} small {"Choose something you love"}}}
            if remote.is_none() && let Some(favorite_track)=track.clone() {
                {
                    let favorite_ui=ui.clone();
                    rsx! { button {class:if favorite_track.favorite {"icon-button selected"}else{"icon-button"},"data-testid":"native-player-favorite","aria-pressed":favorite_track.favorite,disabled:favorite_disabled,"aria-label":if favorite_track.favorite {format!("Remove {} from favorites",favorite_track.title)} else {format!("Favorite {}",favorite_track.title)},title:if favorite_disabled {"Updating this favorite…"} else if favorite_track.favorite {"Remove from favorites"} else {"Add to favorites"},onclick:move |_|toggle_library_favorite(favorite_ui.clone(),favorite_track.clone()),Icon {name:"heart"}} }
                }
            }
        }
        div {class:"player-controls",div {class:"transport",
            button {class:if shuffle{"icon-button selected"}else{"icon-button"},"aria-label":"Shuffle","aria-pressed":shuffle,disabled:blocked,onclick:move |_|{let mut ui=shuffle_ui.clone();let next=!*ui.shuffle.peek();ui.shuffle.set(next);let repeat=*ui.repeat.peek();ui.order.write().configure(next,repeat);},Icon {name:"shuffle"}}
            button {class:"icon-button","aria-label":"Previous",disabled:blocked||(offline_active&&offline_current.is_none_or(|current|current==0))||(remote.is_some()&&*ui.remote_index.read()==0),onclick:move |_|player_previous(previous.clone()),Icon {name:"previous"}}
            button {class:"native-play-toggle","aria-label":if playing{"Pause"}else{"Play"},disabled:blocked||preparing||snapshot.phase==AudioPhase::Loading,onclick:move |_|toggle(toggle_ui.clone()),if playing{Icon {name:"pause"}}else{Icon {name:"play"}}}
            button {class:"icon-button","aria-label":"Next",disabled:blocked||(offline_active&&offline_current.is_none_or(|current|current+1>=offline_len))||(remote.is_some()&&*ui.remote_index.read()+1>=ui.remote_queue.read().len()),onclick:move |_|player_next(next.clone()),Icon {name:"next"}}
            button {class:if repeat!=Repeat::Off{"icon-button selected"}else{"icon-button"},"aria-label":repeat_label,"aria-pressed":repeat!=Repeat::Off,disabled:blocked,onclick:move |_|{let mut ui=repeat_ui.clone();let next=match *ui.repeat.peek(){Repeat::Off=>Repeat::All,Repeat::All=>Repeat::One,Repeat::One=>Repeat::Off};ui.repeat.set(next);let shuffle=*ui.shuffle.peek();ui.order.write().configure(shuffle,next);},if repeat==Repeat::One{Icon {name:"repeat-one"}}else{Icon {name:"repeat"}}}
        }
            div {class:"native-seek",span {"{time_label(snapshot.position_ms)}"} label {class:"sr-only",r#for:"native-seek","Playback position"} input {id:"native-seek",r#type:"range",min:0,max:snapshot.duration_ms.max(1),step:100,value:snapshot.position_ms,disabled:!can_seek,onchange:move |event|seek(seek_ui.clone(),event.value())} span {"{time_label(snapshot.duration_ms)}"}}
        }
        div {class:"player-volume",Icon {name:"volume"} label {class:"sr-only",r#for:"native-volume","Volume"} input {id:"native-volume",r#type:"range",min:0,max:100,step:1,value:(snapshot.volume*100.0).round(),disabled:!ui.audio_ready(),oninput:move |event|volume(volume_ui.clone(),event.value())}}
        div {class:"native-player-status",role:"status","data-testid":"native-playback-status",span {"{status}"} if !context.is_empty(){small {"Playing from {context}"}}
            if preparing {div {class:"preparation-progress",if progress.total_bytes>0 {progress {max:progress.total_bytes,value:progress.received_bytes.min(progress.total_bytes)} small {"{byte_label(progress.received_bytes)} / {byte_label(progress.total_bytes)}"}}else{progress {} small {"Waiting for audio bytes…"}} button {class:"text-button",onclick:move |_|{let mut ui=cancel.clone();ui.new_selection();ui.prepared.set(None);ui.source_error.set("Preparation canceled. Press Play to try again.".into());},"Cancel preparation"}}}
        }
        if !ui.audio_error.read().is_empty(){div {class:"player-error",role:"alert","{ui.audio_error}"}}
        if !error.is_empty(){div {class:"player-error",role:"alert","{error} Press Play to retry."}}
        if let Some(error)=snapshot.error{div {class:"player-error",role:"alert","{error}"}}
        if ui.audio.peek().is_some() && !audio_alive {div {class:"player-error",role:"alert","Native audio output stopped. " button {class:"text-button",onclick:move |_|reconnect_audio(reconnect.clone()),"Reconnect audio"}}}
    } }
}

#[allow(non_snake_case)]
pub fn App() -> Element {
    let ui = Ui {
        alive: use_hook(|| Lifetime(Arc::new(AtomicBool::new(true)))),
        audio: use_signal(|| None),
        audio_error: use_signal(String::new),
        engine_cleanup: use_hook(|| Rc::new(RefCell::new(None))),
        login_gate: use_hook(Gate::default),
        catalog_gate: use_hook(Gate::default),
        feedback_gate: use_hook(Gate::default),
        playlist_gate: use_hook(Gate::default),
        playlist_write_gate: use_hook(Gate::default),
        folder_gate: use_hook(Gate::default),
        browse_gate: use_hook(Gate::default),
        remote_gate: use_hook(Gate::default),
        offline_gate: use_hook(Gate::default),
        source_gate: use_hook(Gate::default),
        logout_gate: use_hook(Gate::default),
        epoch: use_signal(|| 1),
        selection: use_signal(|| 0),
        generation_clock: use_hook(|| Rc::new(Cell::new(0))),
        account: use_signal(|| None),
        origin: use_signal(default_origin),
        username: use_signal(String::new),
        password: use_signal(String::new),
        connecting: use_signal(|| false),
        resuming: use_signal(|| false),
        remember: use_signal(|| false),
        saved_session: use_signal(|| None),
        login_error: use_signal(String::new),
        view: use_signal(|| "library"),
        theme: use_signal(|| "dark"),
        query_draft: use_signal(String::new),
        query: use_signal(String::new),
        cursor: use_signal(|| None),
        catalog: use_signal(|| None),
        catalog_loading: use_signal(|| false),
        catalog_error: use_signal(String::new),
        favorite_busy: use_signal(String::new),
        playlists: use_signal(Vec::new),
        playlist: use_signal(|| None),
        playlist_id: use_signal(|| None),
        playlist_loading: use_signal(|| false),
        playlist_error: use_signal(String::new),
        playlist_name_draft: use_signal(String::new),
        folders: use_signal(Vec::new),
        folder_filter: use_signal(|| None),
        folder_name_draft: use_signal(String::new),
        folder_rename_draft: use_signal(String::new),
        folder_busy: use_signal(|| false),
        folder_confirm: use_signal(String::new),
        playlist_folder_target: use_signal(String::new),
        playlist_candidates: use_signal(Vec::new),
        playlist_candidate: use_signal(String::new),
        playlist_busy: use_signal(|| false),
        playlist_delete_confirm: use_signal(|| false),
        browse_kind: use_signal(|| "albums"),
        albums: use_signal(Vec::new),
        artists: use_signal(Vec::new),
        browse_tracks: use_signal(Vec::new),
        browse_id: use_signal(|| None),
        browse_title: use_signal(String::new),
        browse_loading: use_signal(|| false),
        browse_error: use_signal(String::new),
        remote_origin: use_signal(|| {
            std::env::var("VELA_NATIVE_QA_PEER_ORIGIN").unwrap_or_default()
        }),
        remote_preview: use_signal(|| None),
        remote_loading: use_signal(|| false),
        remote_error: use_signal(String::new),
        remote_now: use_signal(|| None),
        remote_queue: use_signal(Vec::new),
        remote_index: use_signal(|| 0),
        remote_query: use_signal(String::new),
        remote_results: use_signal(Vec::new),
        remote_searching: use_signal(|| false),
        remote_search_error: use_signal(String::new),
        remote_more_busy: use_signal(|| false),
        remote_more_error: use_signal(String::new),
        remote_peer_more_busy: use_signal(String::new),
        remote_peer_more_error: use_signal(String::new),
        remote_retry_busy: use_signal(String::new),
        remote_retry_error: use_signal(String::new),
        offline: use_signal(Vec::new),
        offline_busy: use_signal(String::new),
        offline_error: use_signal(String::new),
        offline_corrupt: use_signal(Vec::new),
        offline_active: use_signal(|| false),
        offline_confirm: use_signal(String::new),
        saved_peers: use_signal(|| crate::peers::load().unwrap_or_default()),
        queue: use_signal(Vec::new),
        order: use_signal(PlaybackOrder::default),
        shuffle: use_signal(|| false),
        repeat: use_signal(Repeat::default),
        context: use_signal(String::new),
        snapshot: use_signal(AudioSnapshot::default),
        preparing: use_signal(|| false),
        progress: use_signal(DownloadProgress::default),
        prepared: use_signal(|| None),
        source_error: use_signal(String::new),
        notice: use_signal(String::new),
        ended: use_hook(|| Rc::new(Cell::new(None))),
        mpris: use_hook(mpris::spawn),
        mpris_note: use_signal(|| "System media controls starting…"),
        mpris_seek_serial: use_signal(|| 0),
    };
    let cleanup = ui.clone();
    use_drop(move || {
        cleanup.alive.set(false);
        cleanup.cancel_all();
        if let Some(audio) = cleanup.engine_cleanup.borrow_mut().take() {
            let _ = audio.stop(cleanup.generation_clock.get().wrapping_add(1));
        }
    });
    let initialize_ui = ui.clone();
    use_future(move || {
        let mut initializing = initialize_ui.clone();
        async move {
            let lifetime = initializing.alive.clone();
            let result = tokio::task::spawn_blocking(move || {
                let result = AudioEngine::new();
                // A detached blocking constructor still drops its late output on close.
                if !lifetime.get() {
                    drop(result);
                    None
                } else {
                    Some(result)
                }
            })
            .await;
            if !initializing.alive.get() {
                return;
            }
            match result {
                Ok(Some(Ok(engine))) => {
                    *initializing.engine_cleanup.borrow_mut() = Some(engine.clone());
                    initializing.audio.set(Some(engine));
                }
                Ok(Some(Err(error))) => initializing.audio_error.set(error),
                Err(_) => initializing.audio_error.set(
                    "Native audio output could not be initialized. Restart Vela to try again."
                        .into(),
                ),
                Ok(None) => {}
            }
        }
    });
    let observe_ui = ui.clone();
    use_future(move || {
        let mut observer = observe_ui.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(200)).await;
                if !observer.alive.get() {
                    return;
                }
                // External OS controls are applied on this UI thread so every
                // account/session/lifetime check remains authoritative.
                while let Some(command) = observer.mpris.try_command() {
                    if apply_system_control(observer.clone(), command) {
                        return;
                    }
                }
                let note = system_controls_note(observer.mpris.state());
                if *observer.mpris_note.peek() != note {
                    observer.mpris_note.set(note);
                }
                if !observer.signed_in() {
                    observer.publish_system_controls();
                    continue;
                }
                let Some(audio) = observer.audio.peek().clone() else {
                    observer.publish_system_controls();
                    continue;
                };
                let snapshot = audio.snapshot();
                let generation = *observer.selection.peek();
                if snapshot.generation != generation {
                    observer.publish_system_controls();
                    continue;
                }
                let ended = snapshot.phase == AudioPhase::Ended;
                observer.snapshot.set(snapshot);
                observer.publish_system_controls();
                if ended && observer.ended.get() != Some(generation) {
                    observer.ended.set(Some(generation));
                    if observer.remote_now.peek().is_some() {
                        // Keep the public item/queue visible after natural EOF so
                        // the listener can replay or move through the remote queue.
                        observer.prepared.set(None);
                    } else {
                        advance(observer.clone(), Command::Ended);
                    }
                } else if !ended {
                    observer.ended.set(None);
                }
            }
        }
    });
    let _account = ui.account.read();
    let theme = *ui.theme.read();
    let view = *ui.view.read();
    let notice = ui.notice.read().clone();
    let mut theme_signal = ui.theme;
    let mut view_signal = ui.view;
    let logout = ui.clone();
    let nav_library = ui.clone();
    let nav_favorites = ui.clone();
    let nav_albums = ui.clone();
    let nav_artists = ui.clone();
    let nav_playlists = ui.clone();
    let nav_public = ui.clone();
    let compact_library = ui.clone();
    let compact_favorites = ui.clone();
    let compact_albums = ui.clone();
    let compact_artists = ui.clone();
    let compact_playlists = ui.clone();
    let compact_public = ui.clone();
    let css = include_str!("../style.css");
    let saved_loader = ui.clone();
    use_future(move || {
        let mut ui = saved_loader.clone();
        async move {
            let generation = ui.login_gate.generation.get();
            // Claim the item for this read: a cleanup may only delete the record
            // that was actually read, never one a newer login already saved.
            let attempt = crate::session_store::begin_attempt();
            let loaded = tokio::task::spawn_blocking(crate::session_store::load).await;
            // AUD-01: a saved-item read may not overwrite a login that started
            // while the keyring was being read.
            if !ui.alive.get() || ui.signed_in() || ui.login_gate.generation.get() != generation {
                return;
            }
            match loaded {
                Ok(Ok(Some(saved))) => ui.saved_session.set(Some(saved)),
                Ok(Ok(None)) => {}
                Ok(Err(error)) => {
                    ui.login_error.set(error.message().to_owned());
                    // AUD-04/AUD-11: only a malformed record is cleaned up, and
                    // only after the user-visible error is published. A timeout
                    // or an unavailable helper must keep the stored item.
                    if error.is_corrupt_record() {
                        let _ = delete_saved_session(move || {
                            crate::session_store::clear_attempt(attempt)
                        })
                        .await;
                    }
                }
                Err(_) => ui.login_error.set(
                    crate::session_store::KeyringError::Unavailable
                        .message()
                        .to_owned(),
                ),
            }
        }
    });
    rsx! {
        document::Style {"{css}"}
        div {class:"native-app","data-theme":theme,
            if ui.signed_in(){
                aside {class:"native-sidebar",div {class:"native-brand",Icon {name:"music"} span {"vela"} small {"NATIVE"}}
                    nav {"aria-label":"Main navigation",button {class:if view=="library"{"native-nav active"}else{"native-nav"},"aria-label":"Library",onclick:move |_|{view_signal.set("library");load_catalog(nav_library.clone(),String::new(),None,false);},Icon {name:"library"}"Library"} button {class:if view=="albums"{"native-nav active"}else{"native-nav"},"aria-label":"Albums",onclick:move |_|{load_groups(nav_albums.clone(),"albums",None);},Icon {name:"albums"}"Albums"} button {class:if view=="artists"{"native-nav active"}else{"native-nav"},"aria-label":"Artists",onclick:move |_|{load_groups(nav_artists.clone(),"artists",None);},Icon {name:"artists"}"Artists"} button {class:if view=="favorites"{"native-nav active"}else{"native-nav"},"aria-label":"Favorites",onclick:move |_|{view_signal.set("favorites");load_catalog(nav_favorites.clone(),String::new(),None,true);},Icon {name:"heart"}"Favorites"} button {class:if view=="playlists"{"native-nav active"}else{"native-nav"},"aria-label":"Playlists",onclick:move |_|{load_playlists(nav_playlists.clone());},Icon {name:"playlists"}"Playlists"} button {class:if view=="public"{"native-nav active"}else{"native-nav"},"aria-label":"Public",onclick:move |_|{view_signal.set("public");explore_public(nav_public.clone());},Icon {name:"network"}"Public"} button {class:if view=="queue"{"native-nav active"}else{"native-nav"},"aria-label":"Queue",onclick:move |_|view_signal.set("queue"),Icon {name:"queue"}"Queue"}}
                    div {class:"native-private-note",Icon {name:"check"} strong {"Private by default"} p {"Your account and queue stay in this running app. Temporary audio is released on replacement or sign-out."} p {class:"native-mpris-note",role:"status","data-testid":"native-mpris-status","{ui.mpris_note}"}}
                }
                main {class:"native-content",header {class:"native-topbar",div {span {class:"eyebrow","YOUR PERSONAL MUSIC SPACE"} if let Some(account)=ui.account.read().as_ref(){strong {"{account.user.username}"} small {"{account.origin}"}}}
                    div {class:"topbar-actions",button {class:"icon-button","aria-label":"Change color theme",onclick:move |_|theme_signal.set(if theme=="dark"{"light"}else{"dark"}),if theme=="dark"{Icon {name:"sun"}}else{Icon {name:"moon"}}} button {class:"button secondary small",onclick:move |_|sign_out(logout.clone()),"Sign out"}}
                }
                    nav {class:"native-compact-nav","aria-label":"Compact navigation",button {class:if view=="library"{"active"}else{""},onclick:move |_|{view_signal.set("library");load_catalog(compact_library.clone(),String::new(),None,false);},"Library"} button {class:if view=="albums"{"active"}else{""},onclick:move |_|{load_groups(compact_albums.clone(),"albums",None);},"Albums"} button {class:if view=="artists"{"active"}else{""},onclick:move |_|{load_groups(compact_artists.clone(),"artists",None);},"Artists"} button {class:if view=="favorites"{"active"}else{""},onclick:move |_|{view_signal.set("favorites");load_catalog(compact_favorites.clone(),String::new(),None,true);},"Favorites"} button {class:if view=="playlists"{"active"}else{""},onclick:move |_|{load_playlists(compact_playlists.clone());},"Playlists"} button {class:if view=="public"{"active"}else{""},onclick:move |_|{view_signal.set("public");explore_public(compact_public.clone());},"Public"} button {class:if view=="queue"{"active"}else{""},onclick:move |_|view_signal.set("queue"),"Queue"}}
                    if !notice.is_empty(){div {class:"alert notice",role:"status","{notice}"}}
                    if view=="queue"{{queue_view(ui.clone())}}else if view=="playlists"{{playlists_view(ui.clone())}}else if view=="albums"||view=="artists"{{browse_view(ui.clone())}}else if view=="public"{{public_view(ui.clone())}}else{{library(ui.clone())}}
                    p {class:"native-preparation-note","Tracks are prepared before playing. This device keeps no offline library; sign in again after closing Vela."}
                }
                {player(ui.clone())}
            }else{{sign_in(ui.clone())}}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_session_failures_are_classified_by_exact_client_message() {
        // HTTP 401 from `/api/me` and the generic expired-session error are the
        // only messages that may delete the stored item.
        assert_eq!(
            saved_session_failure(SAVED_SESSION_EXPIRED),
            SavedSessionFailure::Rejected
        );
        assert_eq!(
            saved_session_failure(SESSION_SIGNED_OUT),
            SavedSessionFailure::Rejected
        );
        // Transient transport and server conditions must preserve the item.
        for error in [
            NETWORK_UNREACHABLE,
            REQUEST_TIMED_OUT,
            REQUEST_FAILED,
            "The server is busy. Wait before retrying.",
        ] {
            assert_eq!(
                saved_session_failure(error),
                SavedSessionFailure::Transient,
                "{error}"
            );
        }
        // A malformed stored record is unusable but was not rejected by a server.
        assert_eq!(
            saved_session_failure(SAVED_SESSION_INVALID),
            SavedSessionFailure::Unusable
        );
        // A 403 talks about signing in but is not a session rejection.
        assert_eq!(
            saved_session_failure(
                "The server denied this action. Sign in again or check your permissions."
            ),
            SavedSessionFailure::Refused
        );
        assert_eq!(
            saved_session_failure(
                "The server redirected a Vela request. Use its direct HTTPS origin."
            ),
            SavedSessionFailure::Refused
        );
        assert_eq!(
            saved_session_failure("The server returned an invalid Vela response."),
            SavedSessionFailure::Refused
        );
        assert_eq!(
            saved_session_failure(REQUEST_CANCELED),
            SavedSessionFailure::Canceled
        );
        assert_eq!(
            saved_session_failure("something new"),
            SavedSessionFailure::Refused
        );
    }

    #[test]
    fn only_a_real_session_rejection_deletes_the_saved_item() {
        for failure in [
            SavedSessionFailure::Unusable,
            SavedSessionFailure::Transient,
            SavedSessionFailure::Refused,
            SavedSessionFailure::Canceled,
        ] {
            assert!(!failure.deletes_saved_item(), "{failure:?}");
        }
        assert!(SavedSessionFailure::Rejected.deletes_saved_item());
    }

    #[test]
    fn non_rejection_messages_say_the_item_was_kept() {
        let error = NETWORK_UNREACHABLE;
        assert!(
            SavedSessionFailure::Transient
                .message(error)
                .contains("kept")
        );
        assert!(
            SavedSessionFailure::Unusable
                .message(error)
                .contains("kept")
        );
        assert!(SavedSessionFailure::Refused.message(error).contains("kept"));
        assert_eq!(SavedSessionFailure::Rejected.message(error), error);
        // The rejected message never claims the item survived.
        assert!(
            !SavedSessionFailure::Rejected
                .message(error)
                .contains("kept")
        );
    }
}
