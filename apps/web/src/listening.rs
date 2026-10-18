//! Account-bound opt-in collector. Audio remains independent of history availability.
use crate::{
    audible::{self, AudibleClock, Outbox, Sample},
    client::{self, RequestScope},
    experience::Actions,
    session::{ContextKind, PlaybackContext},
    storage_get, storage_set, token,
};
use dioxus::prelude::*;
use gloo_timers::callback::{Interval, Timeout};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use vela_domain::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlAudioElement;

#[derive(Clone, Default, PartialEq)]
pub struct ListeningView {
    pub preferences: Option<ListeningPreferences>,
    pub loading: bool,
    pub busy: bool,
    pub error: String,
    pub pending: usize,
    pub revision: u64,
    pub privacy_generation: u64,
}
struct Occurrence {
    progress: ListeningProgress,
    dirty: bool,
}
struct State {
    auth: Signal<Option<AuthResponse>>,
    view: Signal<ListeningView>,
    owner: String,
    session: String,
    device: String,
    generation: u64,
    privacy_generation: u64,
    preferences: Option<ListeningPreferences>,
    blocked: bool,
    current_track: Option<String>,
    occurrence: Option<Occurrence>,
    clock: AudibleClock,
    pending: Vec<ListeningProgress>,
    preferences_request: RequestScope,
    events_request: RequestScope,
    inflight: bool,
    last_flush: f64,
    failures: u8,
    timer: Option<Interval>,
}
#[derive(Clone)]
pub struct Collector(Rc<RefCell<State>>);
impl PartialEq for Collector {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
fn monotonic_ms() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map_or(0.0, |clock| clock.now())
}
fn remove_storage(key: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.remove_item(key);
    }
}
fn deadline(controller: Option<web_sys::AbortController>) -> (Timeout, Rc<Cell<bool>>) {
    let timed_out = Rc::new(Cell::new(false));
    let expired = timed_out.clone();
    let timer = Timeout::new(10000, move || {
        expired.set(true);
        if let Some(controller) = controller {
            controller.abort();
        }
    });
    (timer, timed_out)
}
impl State {
    fn enabled(&self) -> bool {
        !self.blocked
            && !self.session.is_empty()
            && token(self.auth) == self.session
            && self
                .preferences
                .as_ref()
                .is_some_and(|p| p.enabled && p.device_enabled)
    }
    fn key(&self) -> String {
        format!("vela.{}.listening.v1", self.owner)
    }
    fn persist(&self) {
        if self.pending.is_empty() || !self.enabled() {
            remove_storage(&self.key());
            return;
        }
        if let Some(preferences) = &self.preferences {
            let outbox = Outbox {
                version: 1,
                owner: self.owner.clone(),
                device: self.device.clone(),
                revision: preferences.revision,
                events: self.pending.clone(),
            };
            if let Ok(raw) = serde_json::to_string(&outbox)
                && raw.len() <= audible::MAX_OUTBOX_BYTES
            {
                storage_set(&self.key(), &raw);
            }
        }
    }
    fn publish(&mut self) {
        let mut view = self.view.peek().clone();
        view.preferences = self.preferences.clone();
        view.pending = self.pending.len();
        view.privacy_generation = self.privacy_generation;
        self.view.set(view);
    }
    fn matches(&self, generation: u64, session: &str) -> bool {
        self.generation == generation && self.session == session && token(self.auth) == session
    }
    fn begin_occurrence(&mut self) {
        self.clock.reset();
        self.occurrence = self
            .current_track
            .as_ref()
            .filter(|_| self.enabled())
            .map(|track| Occurrence {
                progress: ListeningProgress {
                    session_id: uuid::Uuid::new_v4().to_string(),
                    sequence: 0,
                    track_id: track.clone(),
                    listened_ms: 0,
                    position_ms: 0,
                    started_at: (js_sys::Date::now() / 1000.0).floor() as i64,
                    ended: false,
                },
                dirty: false,
            });
    }
    fn snapshot(&mut self, ended: bool) {
        let Some(current) = self.occurrence.as_mut() else {
            return;
        };
        current.progress.ended |= ended;
        if current.progress.listened_ms == 0 || (!current.dirty && !ended) {
            return;
        }
        if current.progress.sequence == i32::MAX as u32 {
            return;
        }
        current.progress.sequence += 1;
        current.dirty = false;
        audible::enqueue(&mut self.pending, current.progress.clone());
        self.persist();
        self.publish();
    }
    fn stop_collection(&mut self) {
        self.privacy_generation = self.privacy_generation.wrapping_add(1);
        self.blocked = true;
        self.clock.reset();
        self.occurrence = None;
        self.pending.clear();
        self.events_request.cancel();
        self.inflight = false;
        self.failures = 0;
        remove_storage(&self.key());
    }
}
impl Collector {
    pub fn new(auth: Signal<Option<AuthResponse>>, view: Signal<ListeningView>) -> Self {
        let collector = Self(Rc::new(RefCell::new(State {
            auth,
            view,
            owner: String::new(),
            session: String::new(),
            device: String::new(),
            generation: 0,
            privacy_generation: 0,
            preferences: None,
            blocked: true,
            current_track: None,
            occurrence: None,
            clock: AudibleClock::default(),
            pending: Vec::new(),
            preferences_request: RequestScope::default(),
            events_request: RequestScope::default(),
            inflight: false,
            last_flush: 0.0,
            failures: 0,
            timer: None,
        })));
        let weak = Rc::downgrade(&collector.0);
        collector.0.borrow_mut().timer = Some(Interval::new(5000, move || {
            if let Some(state) = weak.upgrade() {
                Self(state).flush(false);
            }
        }));
        collector
    }
    pub fn bind(&self, session: &str, owner: Option<&str>) {
        let mut state = self.0.borrow_mut();
        state.preferences_request.cancel();
        // Never replay the previous login's outbox into another account/session.
        state.stop_collection();
        state.generation = state.generation.wrapping_add(1);
        state.owner = owner.unwrap_or_default().to_string();
        state.session = session.to_string();
        state.current_track = None;
        state.preferences = None;
        state.device.clear();
        state.view.set(ListeningView::default());
        if state.owner.is_empty() || session.is_empty() {
            return;
        }
        let key = format!("vela.{}.listening-device.v1", state.owner);
        state.device = storage_get(&key)
            .filter(|value| audible::canonical_uuid(value))
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        storage_set(&key, &state.device);
        drop(state);
        self.reload();
    }
    pub fn destroy(&self) {
        self.bind("", None);
        self.0.borrow_mut().timer = None;
    }
    pub fn reload(&self) {
        let (generation, session, device, signal) = {
            let mut state = self.0.borrow_mut();
            if state.session.is_empty() {
                return;
            }
            let Ok(signal) = state.preferences_request.begin() else {
                return;
            };
            let mut view = state.view.peek().clone();
            view.loading = true;
            view.error.clear();
            state.view.set(view);
            (
                state.generation,
                state.session.clone(),
                state.device.clone(),
                signal,
            )
        };
        let controller = self.0.borrow().preferences_request.controller();
        let collector = self.clone();
        spawn_local(async move {
            let (timer, timed_out) = deadline(controller);
            let result = client::get::<ListeningPreferences>(
                &format!("/api/listening/preferences?device_id={device}"),
                &signal,
            )
            .await;
            drop(timer);
            let mut state = collector.0.borrow_mut();
            if !state.matches(generation, &session) || (signal.aborted() && !timed_out.get()) {
                return;
            }
            let mut view = state.view.peek().clone();
            view.loading = false;
            match result {
                Ok(preferences) if preferences.device_id == state.device => {
                    let changed = state
                        .preferences
                        .as_ref()
                        .is_none_or(|old| old.revision != preferences.revision);
                    let restart = changed || state.blocked;
                    let saved_outbox = storage_get(&state.key());
                    if changed {
                        state.stop_collection();
                    }
                    state.blocked = false;
                    state.preferences = Some(preferences);
                    if restart && state.enabled() {
                        if let Some(raw) = saved_outbox
                            && let Some(outbox) = Outbox::decode(
                                &raw,
                                &state.owner,
                                &state.device,
                                state.preferences.as_ref().unwrap().revision,
                            )
                        {
                            state.pending = outbox.events;
                        }
                        state.begin_occurrence();
                    }
                    if !state.enabled() {
                        state.stop_collection();
                    }
                    view.error.clear();
                }
                Ok(_) => {
                    view.error =
                        "The server returned another device's preferences. Refresh to retry.".into()
                }
                Err(error) => view.error = error,
            }
            state.view.set(view);
            state.publish();
        });
    }
    /// An explicit selection creates an occurrence even for the same track URL.
    pub fn select(&self, track: &Track) {
        self.finish();
        let mut state = self.0.borrow_mut();
        state.current_track = Some(track.id.clone());
        state.begin_occurrence();
    }
    /// Restored/recovered sources retain the same occurrence across source generations.
    pub fn resume(&self, track: &Track) {
        let mut state = self.0.borrow_mut();
        if state.current_track.as_deref() != Some(&track.id) || state.occurrence.is_none() {
            state.current_track = Some(track.id.clone());
            state.begin_occurrence();
        }
    }
    pub fn discontinuity(&self) {
        self.0.borrow_mut().clock.reset();
    }
    pub fn observe(&self, element: &HtmlAudioElement, force: bool) {
        {
            let mut state = self.0.borrow_mut();
            if !state.enabled() {
                state.clock.reset();
                return;
            }
            let delta = state.clock.observe(Sample {
                wall_ms: monotonic_ms(),
                position: element.current_time(),
                rate: element.playback_rate(),
                audible: !element.paused()
                    && !element.ended()
                    && !element.seeking()
                    && !element.muted()
                    && element.volume() > 0.0
                    && element.ready_state() >= 2,
            });
            if let Some(current) = state.occurrence.as_mut() {
                current.progress.position_ms = (element.current_time().max(0.0) * 1000.0)
                    .round()
                    .min(604_800_000.0) as i64;
                if delta > 0 {
                    current.progress.listened_ms =
                        (current.progress.listened_ms + delta).min(86_400_000);
                    current.dirty = true;
                }
            }
        }
        self.flush(force);
    }
    pub fn finish(&self) {
        self.0.borrow_mut().snapshot(true);
        self.0.borrow_mut().occurrence = None;
        self.0.borrow_mut().clock.reset();
        self.flush(true);
    }
    pub fn flush(&self, force: bool) {
        let (generation, session, batch, signal) = {
            let mut state = self.0.borrow_mut();
            if !state.enabled() || state.inflight {
                return;
            }
            let interval = 5000.0 * f64::from(1_u32 << state.failures.min(4));
            let now = monotonic_ms();
            if !force && now - state.last_flush < interval {
                return;
            }
            state.snapshot(false);
            if state.pending.is_empty() {
                return;
            }
            let Ok(signal) = state.events_request.begin() else {
                return;
            };
            state.last_flush = now;
            state.inflight = true;
            let batch = ListeningBatch {
                device_id: state.device.clone(),
                revision: state.preferences.as_ref().unwrap().revision,
                events: state.pending.iter().take(20).cloned().collect(),
            };
            (state.generation, state.session.clone(), batch, signal)
        };
        let deadline_controller = self.0.borrow().events_request.controller();
        let collector = self.clone();
        spawn_local(async move {
            let deadline_signal = signal.clone();
            let deadline = Timeout::new(10000, move || {
                if !deadline_signal.aborted()
                    && let Some(controller) = deadline_controller
                {
                    controller.abort();
                }
            });
            let result = async {
                let response = gloo_net::http::Request::post("/api/listening/events")
                    .header("X-CSRF-Token", &session)
                    .abort_signal(Some(&signal))
                    .json(&batch)
                    .map_err(|_| ())?
                    .send()
                    .await
                    .map_err(|_| ())?;
                let status = response.status();
                let receipt = if response.ok() {
                    response.json::<ListeningReceipt>().await.ok()
                } else {
                    None
                };
                Ok::<_, ()>((status, receipt))
            }
            .await;
            drop(deadline);
            let mut state = collector.0.borrow_mut();
            if !state.matches(generation, &session)
                || state
                    .preferences
                    .as_ref()
                    .is_none_or(|p| p.revision != batch.revision)
            {
                return;
            }
            state.inflight = false;
            let mut view = state.view.peek().clone();
            match result {
                Ok((200, Some(receipt))) if receipt.revision == batch.revision => {
                    state.pending.retain(|event| {
                        !batch.events.iter().any(|sent| {
                            sent.session_id == event.session_id && sent.sequence >= event.sequence
                        })
                    });
                    state.failures = 0;
                    view.error.clear();
                    view.revision = view.revision.wrapping_add(1);
                }
                Ok((401 | 403 | 409, _)) => {
                    state.stop_collection();
                    view.error = "Collection stopped because your listening permission changed. Refresh privacy settings to review it.".into();
                }
                Ok((400 | 404, _)) => {
                    state.pending.retain(|event| {
                        !batch
                            .events
                            .iter()
                            .any(|sent| sent.session_id == event.session_id)
                    });
                    view.error = "Some listening sessions could not be saved. Check your device clock, track availability and account history limits; export or clear history if it is full.".into();
                }
                _ => {
                    state.failures = state.failures.saturating_add(1);
                    view.error = "Listening history has not synced. Playback is unaffected. Refresh privacy settings to check consent, or retry when connected.".into();
                }
            }
            state.view.set(view);
            state.persist();
            state.publish();
        });
    }
    pub fn configure(
        &self,
        enabled: bool,
        device_enabled: bool,
        retention_days: u16,
        delete: bool,
    ) {
        let (generation, session, device, body, signal) = {
            let mut state = self.0.borrow_mut();
            let Some(preferences) = state.preferences.clone() else {
                return;
            };
            if state.view.peek().busy {
                return;
            }
            // Changes stop collection immediately, even if the network then fails.
            state.stop_collection();
            let Ok(signal) = state.preferences_request.begin() else {
                return;
            };
            let mut view = state.view.peek().clone();
            view.busy = true;
            view.error.clear();
            state.view.set(view);
            let body = UpdateListeningPreferences {
                revision: preferences.revision,
                enabled,
                retention_days,
                device_id: state.device.clone(),
                device_enabled,
            };
            (
                state.generation,
                state.session.clone(),
                state.device.clone(),
                body,
                signal,
            )
        };
        let controller = self.0.borrow().preferences_request.controller();
        let collector = self.clone();
        spawn_local(async move {
            let (timer, timed_out) = deadline(controller);
            let path = if delete {
                format!("/api/listening/history?device_id={device}")
            } else {
                "/api/listening/preferences".into()
            };
            let result = client::api_abortable::<ListeningPreferences>(
                if delete { "DELETE" } else { "PUT" },
                &path,
                if delete {
                    None
                } else {
                    serde_json::to_value(body).ok()
                },
                &session,
                Some(&signal),
            )
            .await;
            drop(timer);
            let mut state = collector.0.borrow_mut();
            if !state.matches(generation, &session) || (signal.aborted() && !timed_out.get()) {
                return;
            }
            let mut view = state.view.peek().clone();
            view.busy = false;
            match result {
                Ok(preferences) if preferences.device_id == state.device => {
                    state.preferences = Some(preferences);
                    state.blocked = false;
                    state.begin_occurrence();
                    view.error.clear();
                    view.revision = view.revision.wrapping_add(1);
                }
                Ok(_) => view.error = "Another device's preferences were returned. Collection stays paused; refresh privacy settings.".into(),
                Err(message) => {
                    view.error = format!(
                        "{message} Collection is paused on this device. Refresh privacy settings before trying again."
                    )
                }
            }
            state.view.set(view);
            state.publish();
        });
    }
}

fn download_export(export: &ListeningExport) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(export)
        .map_err(|_| "Could not prepare your export.".to_string())?;
    let parts = js_sys::Array::new();
    parts.push(&raw.into());
    let blob = web_sys::Blob::new_with_str_sequence(&parts)
        .map_err(|_| "Could not prepare your export.".to_string())?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|_| "Could not open your export.".to_string())?;
    let element = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.create_element("a").ok())
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok());
    let result = if let Some(element) = element {
        let _ = element.set_attribute("href", &url);
        let _ = element.set_attribute("download", "vela-private-listening-history.json");
        element.click();
        Ok(())
    } else {
        Err("Could not start the download. Try another browser.".into())
    };
    Timeout::new(1000, move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    })
    .forget();
    result
}

fn device_age_label(timestamp: i64) -> String {
    let now = (js_sys::Date::now() / 1000.0) as i64;
    let age = now.saturating_sub(timestamp);
    if age < 60 {
        "just now".into()
    } else if age < 3_600 {
        format!("{}m ago", age / 60)
    } else if age < 86_400 {
        format!("{}h ago", age / 3_600)
    } else {
        format!("{}d ago", age / 86_400)
    }
}

#[component]
pub fn PrivacySettings(
    auth: Signal<Option<AuthResponse>>,
    view: Signal<ListeningView>,
    collector: Collector,
    onhistory: EventHandler<()>,
) -> Element {
    let mut confirm = use_signal(|| false);
    let mut exporting = use_signal(|| false);
    let mut export_error = use_signal(String::new);
    let requests = use_hook(RequestScope::default);
    let privacy_requests = requests.clone();
    let mut export_context = use_signal(|| None::<(String, u64, bool)>);
    let mut devices = use_signal(Vec::<ListeningDevice>::new);
    let mut devices_loading = use_signal(|| false);
    let mut devices_error = use_signal(String::new);
    let revoke_busy = use_signal(String::new);
    let revoke_confirm = use_signal(String::new);
    let device_requests = use_hook(RequestScope::default);
    let device_requests_effect = device_requests.clone();
    use_effect(move || {
        let current = view.read();
        let _auth = auth.read();
        let context = (token(auth), current.privacy_generation, current.busy);
        if export_context.peek().as_ref() != Some(&context) {
            privacy_requests.cancel();
            exporting.set(false);
            export_error.set(String::new());
            export_context.set(Some(context));
        }
    });
    let cleanup = requests.clone();
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let dropped = alive.clone();
    let cleanup_devices = device_requests.clone();
    use_drop(move || {
        dropped.set(false);
        cleanup.cancel();
        cleanup_devices.cancel();
    });
    let devices_alive = alive.clone();
    use_effect(move || {
        let session = token(auth);
        let _auth = auth.read();
        let _revision = view.read().preferences.as_ref().map(|value| value.revision);
        if session.is_empty() {
            devices.set(Vec::new());
            return;
        }
        device_requests_effect.cancel();
        devices_loading.set(true);
        devices_error.set(String::new());
        let Ok(signal) = device_requests_effect.begin() else {
            devices_loading.set(false);
            return;
        };
        let alive = devices_alive.clone();
        spawn_local(async move {
            let result = client::get::<ListeningDevices>("/api/listening/devices", &signal).await;
            if !alive.get() || token(auth) != session {
                return;
            }
            devices_loading.set(false);
            match result {
                Ok(list) => devices.set(list.devices),
                Err(message) => devices_error.set(message),
            }
        });
    });
    let current = view.read().clone();
    let preferences = current.preferences.clone();
    let enabled = preferences.as_ref().is_some_and(|p| p.enabled);
    let device_enabled = preferences.as_ref().is_some_and(|p| p.device_enabled);
    let retention = preferences.as_ref().map_or(90, |p| p.retention_days);
    let blocked = current.busy || current.loading || preferences.is_none();
    let privacy_revision = preferences.as_ref().map(|preferences| preferences.revision);
    let privacy_generation = current.privacy_generation;
    let global = collector.clone();
    let device = collector.clone();
    let retention_collector = collector.clone();
    let refresh = collector.clone();
    rsx! { div { class: "panel listening-privacy", "aria-labelledby": "private-history-title",
        h2 { id: "private-history-title", "Private listening history" }
        p { "Keep a private record of music you actually play. Collection starts only after you enable both your account and this device. History is never published or shared with collaborators." }
        if current.loading { p { role: "status", "Loading privacy settings…" } }
        if !current.error.is_empty() { p { class: "alert error", role: "alert", "{current.error}" } }
        label { class: "checkbox-label", input { r#type: "checkbox", checked: enabled, disabled: blocked, "aria-label": "Enable private listening history", onchange: move |event| global.configure(event.checked(), device_enabled, retention, false) } span { "Enable private listening history for my account" } }
        label { class: "checkbox-label", input { r#type: "checkbox", checked: device_enabled, disabled: blocked || !enabled, "aria-label": "Collect listening history on this device", onchange: move |event| device.configure(enabled, event.checked(), retention, false) } span { "Collect on this device" } }
        label { r#for: "history-retention", "Keep history for" }
        select { id: "history-retention", value: "{retention}", disabled: blocked, onchange: move |event| { if let Ok(days) = event.value().parse() { retention_collector.configure(enabled, device_enabled, days, false); } },
            if ![7_u16, 30, 90, 180, 365].contains(&retention) { option { value: "{retention}", "{retention} days" } }
            for days in [7_u16, 30, 90, 180, 365] { option { value: "{days}", "{days} days" } }
        }
        small { "Exports include server-synced sessions. This browser keeps up to 100 pending summaries; when full, it drops the oldest. Signing out or turning collection off clears that local journal." }
        small { "Seeking, buffering, muted playback and rapid skips do not become completed listens. A browser can measure playback progress; it cannot verify whether speakers are audible." }
        if current.pending > 0 { p { role: "status", "{current.pending} listening sessions waiting to sync on this device." } }
        div { class: "listening-privacy-actions",
            button { class: "button secondary small", onclick: move |_| onhistory.call(()), "Open listening history" }
            button { class: "button secondary small", disabled: current.busy || current.loading, onclick: move |_| refresh.reload(), "Refresh privacy settings" }
            button { class: "button secondary small", disabled: exporting(), onclick: move |_| {
                let session = token(auth);
                let Ok(signal) = requests.begin() else { return; };
                let controller = requests.controller();
                let alive = alive.clone();
                exporting.set(true); export_error.set(String::new());
                spawn_local(async move {
                    let (timer, timed_out) = deadline(controller);
                    let result = client::get::<ListeningExport>("/api/listening/export", &signal).await;
                    drop(timer);
                    if !alive.get() || (signal.aborted() && !timed_out.get()) || token(auth) != session { return; }
                    if view.peek().busy || view.peek().privacy_generation != privacy_generation || view.peek().preferences.as_ref().map(|preferences| preferences.revision) != privacy_revision { return; }
                    exporting.set(false);
                    if let Err(message) = result.and_then(|value| download_export(&value)) { export_error.set(message); }
                });
            }, if exporting() { "Preparing export…" } else { "Export private history" } }
            button { class: "button danger small", disabled: blocked, onclick: move |_| confirm.set(true), "Delete history…" }
        }
        if !export_error().is_empty() { p { role: "alert", "{export_error}" } }
        div { class: "listening-devices", "aria-label": "Devices with listening history",
            h3 { "Listening devices" }
            p { "Each browser that consented to collection is listed here. Revoking a device deletes that device's saved sessions and invalidates any export prepared under the previous privacy revision." }
            if devices_loading() { p { role: "status", "Loading listening devices…" } }
            if !devices_error().is_empty() { p { class: "alert error", role: "alert", "{devices_error()}" } }
            if !devices_loading() && devices_error().is_empty() && devices().is_empty() { p { "No device has consented to collection yet." } }
            for device in devices() {
                {
                    let device_id = device.device_id.clone();
                    let current_device = preferences.as_ref().is_some_and(|value| value.device_id == device_id);
                    let revoke_id = device_id.clone();
                    let revoke_confirm_id = device_id.clone();
                    let revoke_ui = collector.clone();
                    let revision = privacy_revision;
                    let session = token(auth);
                    let alive = alive.clone();
                    let mut revoke_busy = revoke_busy;
                    let mut revoke_confirm = revoke_confirm;
                    let devices = devices;
                    let device_requests = device_requests.clone();
                    let busy = revoke_busy() == device_id;
                    let last_active = device.last_activity_at.map(device_age_label).unwrap_or_else(|| "never".into());
                    let collection = if device.enabled { "collection on" } else { "collection off" };
                    rsx! { article { class: "history-track", "data-listening-device": "{device_id}",
                        div { class: "history-play", div { strong { if current_device { "This browser" } else { "Listening device" } } small { "{device.session_count} sessions · {collection} · last active {last_active}" } } }
                        if revoke_confirm() == device_id {
                            button { class: "button danger small", disabled: busy, "aria-label": "Confirm revoke listening device {device_id}", onclick: move |_| {
                                let Some(revision) = revision else { return; };
                                let Ok(signal) = device_requests.begin() else { return; };
                                revoke_busy.set(revoke_id.clone());
                                devices_error.set(String::new());
                                let path = format!("/api/listening/devices/{}?revision={}", revoke_id, revision);
                                let session = session.clone();
                                let alive = alive.clone();
                                let mut devices = devices;
                                let revoke_ui = revoke_ui.clone();
                                let mut revoke_busy = revoke_busy;
                                let mut revoke_confirm = revoke_confirm;
                                spawn_local(async move {
                                    let result = client::api_abortable::<ListeningDevices>("DELETE", &path, None, &session, Some(&signal)).await;
                                    if !alive.get() || token(auth) != session { return; }
                                    revoke_busy.set(String::new());
                                    revoke_confirm.set(String::new());
                                    match result {
                                        Ok(list) => { devices.set(list.devices); revoke_ui.reload(); }
                                        Err(message) => devices_error.set(message),
                                    }
                                });
                            }, if busy { "Revoking…" } else { "Confirm revoke" } }
                            button { class: "button secondary small", onclick: move |_| revoke_confirm.set(String::new()), "Keep device" }
                        } else {
                            button { class: "button danger small", disabled: busy, "aria-label": "Revoke listening device {device_id}", onclick: move |_| revoke_confirm.set(revoke_confirm_id.clone()), "Revoke device…" }
                        }
                    } }
                }
            }
        }
        if confirm() { div { class: "history-delete-confirm", role: "group", "aria-label": "Confirm history deletion",
            p { "Delete all listening history for this account and turn off collection on every device? This cannot be undone. Existing music and playlists are preserved." }
            button { class: "button danger", disabled: current.busy, onclick: move |_| { collector.configure(false, false, retention, true); confirm.set(false); }, "Delete history and turn off collection" }
            button { class: "button secondary", onclick: move |_| confirm.set(false), "Keep my history" }
        } }
    } }
}

#[component]
pub fn History(
    auth: Signal<Option<AuthResponse>>,
    view: Signal<ListeningView>,
    actions: Actions,
    compact: bool,
) -> Element {
    let requests = use_hook(RequestScope::default);
    let cleanup = requests.clone();
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let dropped = alive.clone();
    use_drop(move || {
        dropped.set(false);
        cleanup.cancel();
    });
    let mut generation = use_signal(|| 0_u64);
    let mut scope = use_signal(|| (String::new(), None::<String>));
    let mut cursor = use_signal(|| None::<String>);
    let mut refresh = use_signal(|| 0_u64);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(String::new);
    let mut result = use_signal(|| None::<ListeningHistoryPage>);
    use_effect(move || {
        let session = token(auth);
        let _auth = auth.read();
        let _revision = view.read().revision;
        let _refresh = *refresh.read();
        let cursor = cursor.read().clone();
        requests.cancel();
        let next = generation.peek().wrapping_add(1);
        generation.set(next);
        let context = (session.clone(), cursor.clone());
        if *scope.peek() != context {
            scope.set(context);
            result.set(None);
        }
        error.set(String::new());
        if session.is_empty() {
            return;
        }
        let Ok(signal) = requests.begin() else {
            return;
        };
        let controller = requests.controller();
        let alive = alive.clone();
        loading.set(true);
        spawn_local(async move {
            let (timer, timed_out) = deadline(controller);
            let mut path = format!(
                "/api/listening/history?limit={}",
                if compact { 6 } else { 20 }
            );
            if let Some(cursor) = cursor {
                crate::filters::append_query(&mut path, "cursor", &cursor);
            }
            let response = client::get::<ListeningHistoryPage>(&path, &signal).await;
            drop(timer);
            if !alive.get()
                || (signal.aborted() && !timed_out.get())
                || token(auth) != session
                || *generation.peek() != next
            {
                return;
            }
            loading.set(false);
            match response {
                Ok(value) => result.set(Some(value)),
                Err(message) => error.set(message),
            }
        });
    });
    let snapshot = result.read().clone();
    let tracks: Vec<Track> = snapshot
        .as_ref()
        .map(|page| page.items.iter().map(|entry| entry.track.clone()).collect())
        .unwrap_or_default();
    let has_entries = !tracks.is_empty();
    rsx! { if !compact || has_entries { section { class: "home-section listening-history", "aria-label": "Private listening history",
        div { class: "section-toolbar", div { h2 { if compact { "Recently played" } else { "Your listening history" } } p { "Only your own listening. Private by default." } }
            if compact { button { class: "text-button", onclick: move |_| actions.navigate.call("history".into()), "View history" } }
            else { button { class: "button secondary small", disabled: loading(), onclick: move |_| { let next = refresh.peek().wrapping_add(1); refresh.set(next); }, "Refresh listening history" } }
        }
        if loading() { p { role: "status", "Loading listening history…" } }
        if !error().is_empty() { p { class: "alert error", role: "alert", "{error}" } }
        if let Some(page) = snapshot {
            if !compact { p { "{page.total_listens} meaningful listens · {page.total_listened_ms / 60000} minutes played within your retention period" } }
            if page.items.is_empty() && !loading() { div { class: "empty-state", h3 { "Your listening story starts with you" } p { "History collection is off until you choose to enable it. Play at least half a track or 30 seconds with collection enabled to see it here." } button { class: "button secondary", onclick: move |_| actions.navigate.call("settings".into()), "Manage private listening history" } } }
            div { class: "history-track-list", for (at, entry) in page.items.iter().enumerate() { { let context = tracks.clone(); let track = entry.track.clone(); rsx! {
                article { class: "history-track", key: "{entry.session_id}",
                    button { class: "history-play", "aria-label": "Play {track.title} from listening history", onclick: move |_| actions.play.call((context.clone(), at, PlaybackContext::new(ContextKind::History, "Private listening history"))), crate::Art { track: track.clone() } div { strong { "{track.title}" } small { "{track.artist}" } } crate::icons::Icon { name: "play" } }
                    small { "{entry.listened_ms / 1000}s played" }
                    button { class: "icon-button history-radio", title: "Find similar music", "aria-label": "Explore radio from {entry.track.title}", onclick: { let radio = entry.track.clone(); move |_| actions.radio.call(radio.clone()) }, crate::icons::Icon { name: "radio" } }
                }
            } } } }
            if !compact { div { class: "pagination", button { class: "button secondary small", disabled: loading(), onclick: move |_| cursor.set(None), "Latest listens" } if let Some(next) = page.next_cursor { button { class: "button secondary small", disabled: loading(), onclick: move |_| cursor.set(Some(next.clone())), "Older listens" } } } }
        }
    } } }
}
