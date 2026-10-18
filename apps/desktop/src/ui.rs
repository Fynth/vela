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
use vela_domain::{Track, TrackPage, User};

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
    login_error: Signal<String>,
    view: Signal<&'static str>,
    theme: Signal<&'static str>,
    query_draft: Signal<String>,
    query: Signal<String>,
    cursor: Signal<Option<String>>,
    catalog: Signal<Option<Catalog>>,
    catalog_loading: Signal<bool>,
    catalog_error: Signal<String>,
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
        self.password.set(String::new());
        self.connecting.set(false);
        self.catalog.set(None);
        self.query.set(String::new());
        self.query_draft.set(String::new());
        self.cursor.set(None);
        self.catalog_loading.set(false);
        self.catalog_error.set(String::new());
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
    if origin.is_empty() || username.is_empty() || password.is_empty() {
        ui.login_error
            .set("Enter your server origin, username and password.".into());
        return;
    }
    let ticket = ui.login_gate.begin(*ui.epoch.peek());
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
        if !ticket.current(&ui)
            || ui.origin.peek().trim() != origin
            || ui.username.peek().trim() != username
        {
            return;
        }
        ui.connecting.set(false);
        match result {
            Ok((client, auth)) => {
                ui.account.set(Some(Account {
                    epoch: ticket.epoch,
                    client,
                    user: auth.user,
                    origin,
                }));
                load_catalog(ui, String::new(), None);
            }
            Err(error) => ui.login_error.set(error),
        }
    });
}

fn sign_out(mut ui: Ui) {
    let old = ui.account.peek().clone();
    ui.clear_account();
    ui.login_error.set(String::new());
    ui.notice
        .set("Signed out on this device. Your queue and temporary audio have been cleared.".into());
    let Some(account) = old else {
        return;
    };
    let ticket = ui.logout_gate.begin(*ui.epoch.peek());
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

fn load_catalog(mut ui: Ui, query: String, cursor: Option<String>) {
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
    let ticket = ui.catalog_gate.begin(account.epoch);
    ui.catalog_loading.set(true);
    spawn(async move {
        if !ticket.current(&ui) || *ui.query.peek() != query || *ui.cursor.peek() != cursor {
            return;
        }
        let result = account
            .client
            .tracks(
                (!query.is_empty()).then_some(query.as_str()),
                cursor.as_deref(),
                ticket.cancel.clone(),
            )
            .await;
        if !ticket.current(&ui) || *ui.query.peek() != query || *ui.cursor.peek() != cursor {
            return;
        }
        ui.catalog_loading.set(false);
        match result {
            Ok(page) => ui.catalog.set(Some(Catalog {
                epoch: ticket.epoch,
                query,
                cursor,
                page: Arc::new(page),
            })),
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.catalog_error.set(error);
                }
            }
        }
    });
}

fn start_source(mut ui: Ui, reuse: bool, autoplay: bool, position_ms: u64) {
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

fn play_catalog(mut ui: Ui, catalog: Catalog, at: usize) {
    if !ui.catalog_current(&catalog) || at >= catalog.page.items.len() || ui.audio.peek().is_none()
    {
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
    if ui.current_track().is_none() {
        return;
    }
    let snapshot = ui.snapshot.peek().clone();
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
                button {class:"button primary",r#type:"submit",disabled:connecting||output_pending,if connecting {"Connecting…"} else if output_pending {"Preparing native output…"} else {"Sign in"}}
                if connecting {button {class:"button secondary",r#type:"button",onclick:move |_|{let mut ui=cancel.clone();ui.login_gate.cancel();ui.connecting.set(false);ui.login_error.set("Connection canceled. Enter your password to try again.".into());},"Cancel connection"}}
            }
            small {class:"signin-limit","Audio is prepared in a private temporary file before playback. This client does not keep an offline library or send listening history."}
        }
    } }
}

fn library(ui: Ui) -> Element {
    let loading = *ui.catalog_loading.read();
    let error = ui.catalog_error.read().clone();
    let draft = ui.query_draft.read().clone();
    let query = ui.query.read().clone();
    let catalog = ui
        .catalog
        .read()
        .clone()
        .filter(|catalog| ui.catalog_current(catalog));
    let mut search = ui.query_draft;
    let submit = ui.clone();
    let refresh = ui.clone();
    let first = ui.clone();
    let next = ui.clone();
    rsx! {section {class:"native-library","aria-label":"Private library",
        div {class:"library-intro",div {span {class:"eyebrow","YOUR COLLECTION, YOUR ORIGINALS"} h1 {"A library that feels like home"} p {"Choose a track. Keep the rest of your day in the background."}}
            button {class:"button secondary",disabled:loading,onclick:move |_|{let query=refresh.query.peek().clone();let cursor=refresh.cursor.peek().clone();load_catalog(refresh.clone(),query,cursor);},Icon {name:"history"} "Refresh library"}
        }
        form {class:"native-search",onsubmit:move |event|{event.prevent_default();let query=submit.query_draft.peek().trim().to_string();load_catalog(submit.clone(),query,None);},
            Icon {name:"search"} label {class:"sr-only",r#for:"native-search","Search your library"}
            input {id:"native-search",value:"{draft}",maxlength:256,placeholder:"Search titles, artists or albums",oninput:move |event|search.set(event.value())}
            button {class:"button primary",r#type:"submit","Search"}
        }
        div {class:"catalog-caption",p {if query.is_empty() {"Your private tracks · Up to 50 per page"} else {"Results for “{query}” · Up to 50 per page"}} span {"ORIGINAL AUDIO"}}
        if loading {div {class:"native-loading",role:"status",span {class:"loading-mark","aria-hidden":"true"} "Loading your library…"}}
        if !error.is_empty() {div {class:"alert error",role:"alert","{error} Refresh the library to try again."}}
        if let Some(catalog) = catalog {
            if catalog.page.items.is_empty() {div {class:"native-empty",Icon {name:"library"} h2 {if query.is_empty() {"Make room for your first listen"} else {"No matching tracks"}} p {if query.is_empty() {"Add music in Vela’s web app, then refresh this library. Your tracks stay private."} else {"Try a different title, artist or album. Search uses only your own library."}}}}
            else {div {class:"native-track-table",div {class:"native-track-header","aria-hidden":"true",span {""} span {"TITLE / ARTIST"} span {"ALBUM"} span {"TIME"} span {"QUEUE"}}
                for (at,track) in catalog.page.items.iter().cloned().enumerate() { {
                    let play_ui=ui.clone();let played_catalog=catalog.clone();let next_ui=ui.clone();let next_catalog=catalog.clone();let last_ui=ui.clone();let last_catalog=catalog.clone();let next_track=track.clone();let last_track=track.clone();
                    let current=ui.current_track().is_some_and(|current|current.id==track.id);
                    rsx! {article {class:if current {"native-track current"} else {"native-track"},key:"{track.id}","data-native-track-id":"{track.id}",
                        button {class:"icon-button track-start",disabled:!ui.audio_ready(),title:if !ui.audio_ready() {"Native audio output is unavailable"} else {"Play original"},"aria-label":"Play {track.title}",onclick:move |_|play_catalog(play_ui.clone(),played_catalog.clone(),at),Icon {name:"play"}}
                        div {class:"native-track-name",div {class:"neutral-cover","aria-hidden":"true",Icon {name:"music"}} div {strong {"{track.title}"} small {"{track.artist}"}}}
                        span {class:"native-track-album","{track.album}"}
                        span {class:"native-track-time","{time_label(track.duration_ms.max(0) as u64)}"}
                        div {class:"native-track-actions",
                            button {class:"icon-button","aria-label":"Play {track.title} next",title:"Play next",onclick:move |_|{if next_ui.catalog_current(&next_catalog) {enqueue(next_ui.clone(),next_track.clone(),true);}},Icon {name:"next-add"}}
                            button {class:"icon-button","aria-label":"Add {track.title} to queue",title:"Add to queue",onclick:move |_|{if last_ui.catalog_current(&last_catalog){enqueue(last_ui.clone(),last_track.clone(),false);}},Icon {name:"plus"}}
                        }
                    } }
                } }
            } }
            div {class:"native-pagination",button {class:"button secondary",disabled:ui.cursor.peek().is_none(),onclick:move |_|{let query=first.query.peek().clone();load_catalog(first.clone(),query,None);},"First page"}
                if let Some(cursor)=catalog.page.next_cursor.clone() {button {class:"button secondary",onclick:move |_|{let query=next.query.peek().clone();load_catalog(next.clone(),query,Some(cursor.clone()));},"Next page"}}
                small {"Only this page is queued when you play a result."}
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
    rsx! {section {class:"native-queue","aria-label":"Playback queue",
        div {class:"library-intro",div {span {class:"eyebrow","ROOM FOR YOUR NEXT LISTEN"} h1 {"Your listening queue"} p {"{queue_len} of 1,000 places · Duplicate tracks keep their own place."}}
            button {class:"button secondary",disabled:queue_empty,onclick:move |_|clear_queue(clear.clone()),"Clear queue"}
        }
        if queue_empty {div {class:"native-empty",Icon {name:"queue"} h2 {"A little room for what comes next"} p {"Play a library result, use Play next, or add tracks to your queue."}}}
        else {div {class:"native-queue-list",for (at,track) in queue.into_iter().enumerate() { {
            let select=ui.clone();let up=ui.clone();let remove=ui.clone();
            rsx! {article {class:if current==Some(at){"native-queue-row current"}else{"native-queue-row"},key:"{at}-{track.id}","data-native-queue-index":"{at}",
                button {class:"native-queue-select",disabled:!ui.audio_ready(),"aria-label":"Play queue occurrence {at}: {track.title}",onclick:move |_|select_queue(select.clone(),at),span {class:"queue-number","{at+1}"} div {class:"neutral-cover","aria-hidden":"true",Icon {name:"music"}} div {strong {"{track.title}"} small {"{track.artist}"}} if current==Some(at){span {class:"queue-current","Current"}}}
                button {class:"icon-button",disabled:at==0,"aria-label":"Move queue occurrence {at} up",title:"Move up",onclick:move |_|queue_edit(up.clone(),at,false),Icon {name:"up"}}
                button {class:"icon-button",disabled:current==Some(at),"aria-label":"Remove queue occurrence {at}",title:if current==Some(at){"Current occurrence cannot be removed; clear the queue to stop it"}else{"Remove from queue"},onclick:move |_|queue_edit(remove.clone(),at,true),Icon {name:"close"}}
            } }
        } } } }
        p {class:"queue-explanation","Shuffle follows a temporary fair deck and playback history. The visible list is saved queue order, not a prediction of shuffled playback."}
    } }
}

fn player(ui: Ui) -> Element {
    let track = ui.current_track();
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
    let blocked = track.is_none() || !ui.audio_ready();
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
    rsx! {footer {class:"native-player","aria-label":"Native music player","data-audio-phase":"{snapshot.phase:?}","data-audio-generation":"{snapshot.generation}",
        div {class:"player-track",div {class:"neutral-cover player-cover","aria-hidden":"true",Icon {name:"music"}} div {if let Some(track)=track {strong {"{track.title}"} small {"{track.artist}"}}else{strong {"Ready when you are"} small {"Choose something you love"}}}}
        div {class:"player-controls",div {class:"transport",
            button {class:if shuffle{"icon-button selected"}else{"icon-button"},"aria-label":"Shuffle","aria-pressed":shuffle,disabled:blocked,onclick:move |_|{let mut ui=shuffle_ui.clone();let next=!*ui.shuffle.peek();ui.shuffle.set(next);let repeat=*ui.repeat.peek();ui.order.write().configure(next,repeat);},Icon {name:"shuffle"}}
            button {class:"icon-button","aria-label":"Previous",disabled:blocked,onclick:move |_|advance(previous.clone(),Command::Previous),Icon {name:"previous"}}
            button {class:"native-play-toggle","aria-label":if playing{"Pause"}else{"Play"},disabled:blocked||preparing||snapshot.phase==AudioPhase::Loading,onclick:move |_|toggle(toggle_ui.clone()),if playing{Icon {name:"pause"}}else{Icon {name:"play"}}}
            button {class:"icon-button","aria-label":"Next",disabled:blocked,onclick:move |_|advance(next.clone(),Command::Next),Icon {name:"next"}}
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
        login_error: use_signal(String::new),
        view: use_signal(|| "library"),
        theme: use_signal(|| "dark"),
        query_draft: use_signal(String::new),
        query: use_signal(String::new),
        cursor: use_signal(|| None),
        catalog: use_signal(|| None),
        catalog_loading: use_signal(|| false),
        catalog_error: use_signal(String::new),
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
                    advance(observer.clone(), Command::Ended);
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
    let css = include_str!("../style.css");
    rsx! {
        document::Style {"{css}"}
        div {class:"native-app","data-theme":theme,
            if ui.signed_in(){
                aside {class:"native-sidebar",div {class:"native-brand",Icon {name:"music"} span {"vela"} small {"NATIVE"}}
                    nav {"aria-label":"Main navigation",button {class:if view=="library"{"native-nav active"}else{"native-nav"},"aria-label":"Library",onclick:move |_|view_signal.set("library"),Icon {name:"library"}"Library"} button {class:if view=="queue"{"native-nav active"}else{"native-nav"},"aria-label":"Queue",onclick:move |_|view_signal.set("queue"),Icon {name:"queue"}"Queue"}}
                    div {class:"native-private-note",Icon {name:"check"} strong {"Private by default"} p {"Your account and queue stay in this running app. Temporary audio is released on replacement or sign-out."} p {class:"native-mpris-note",role:"status","data-testid":"native-mpris-status","{ui.mpris_note}"}}
                }
                main {class:"native-content",header {class:"native-topbar",div {span {class:"eyebrow","YOUR PERSONAL MUSIC SPACE"} if let Some(account)=ui.account.read().as_ref(){strong {"{account.user.username}"} small {"{account.origin}"}}}
                    div {class:"topbar-actions",button {class:"icon-button","aria-label":"Change color theme",onclick:move |_|theme_signal.set(if theme=="dark"{"light"}else{"dark"}),if theme=="dark"{Icon {name:"sun"}}else{Icon {name:"moon"}}} button {class:"button secondary small",onclick:move |_|sign_out(logout.clone()),"Sign out"}}
                }
                    nav {class:"native-compact-nav","aria-label":"Compact navigation",button {class:if view=="library"{"active"}else{""},onclick:move |_|view_signal.set("library"),"Library"} button {class:if view=="queue"{"active"}else{""},onclick:move |_|view_signal.set("queue"),"Queue"}}
                    if !notice.is_empty(){div {class:"alert notice",role:"status","{notice}"}}
                    if view=="queue"{{queue_view(ui.clone())}}else{{library(ui.clone())}}
                    p {class:"native-preparation-note","Tracks are prepared before playing. This device keeps no offline library; sign in again after closing Vela."}
                }
                {player(ui.clone())}
            }else{{sign_in(ui.clone())}}
        }
    }
}
