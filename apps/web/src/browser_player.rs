//! Root-owned browser adapter: a source generation is distinct from a track ID.
use crate::{audio, clear_media_session, session::*, storage_set, update_media_metadata};
use dioxus::prelude::*;
use gloo_timers::callback::Timeout;
use std::{cell::RefCell, rc::Rc};
use vela_domain::Track;
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{AbortController, HtmlAudioElement};

#[derive(Clone, Copy)]
pub struct View {
    pub auth: Signal<Option<vela_domain::AuthResponse>>,
    pub playing: Signal<bool>,
    pub position: Signal<f64>,
    pub duration: Signal<f64>,
    pub status: Signal<String>,
    pub expired: Signal<bool>,
}
struct State {
    view: View,
    session: String,
    saved: Option<SavedSession>,
    generation: u64,
    source: String,
    element: Option<HtmlAudioElement>,
    intent: bool,
    pending: Option<f64>,
    loaded: bool,
    position: f64,
    last_save: f64,
    progress_at: f64,
    stable_from: f64,
    stable: bool,
    retries: RetryBudget,
    retry: Option<Timeout>,
    stall: Option<Timeout>,
    probe: Option<AbortController>,
    lifecycle: Option<Closure<dyn FnMut(web_sys::Event)>>,
}
#[derive(Clone)]
pub struct BrowserPlayer(Rc<RefCell<State>>);
impl BrowserPlayer {
    pub fn new(view: View) -> Self {
        let player = Self(Rc::new(RefCell::new(State {
            view,
            session: String::new(),
            saved: None,
            generation: 0,
            source: String::new(),
            element: None,
            intent: false,
            pending: None,
            loaded: false,
            position: 0.0,
            last_save: 0.0,
            progress_at: 0.0,
            stable_from: 0.0,
            stable: false,
            retries: RetryBudget::default(),
            retry: None,
            stall: None,
            probe: None,
            lifecycle: None,
        })));
        let weak = Rc::downgrade(&player.0);
        let callback = Closure::wrap(Box::new(move |_: web_sys::Event| {
            if let Some(state) = weak.upgrade() {
                let player = Self(state);
                player.time(true);
                player.checkpoint(true);
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        if let Some(window) = web_sys::window() {
            let _ = window.add_event_listener_with_callback(
                "vela-checkpoint",
                callback.as_ref().unchecked_ref(),
            );
        }
        player.0.borrow_mut().lifecycle = Some(callback);
        player
    }
    pub fn destroy(&self) {
        self.bind(String::new(), None);
        if let Some(callback) = self.0.borrow_mut().lifecycle.take()
            && let Some(window) = web_sys::window()
        {
            let _ = window.remove_event_listener_with_callback(
                "vela-checkpoint",
                callback.as_ref().unchecked_ref(),
            );
        }
    }
    fn cancel(state: &mut State) {
        state.retry = None;
        state.stall = None;
        if let Some(probe) = state.probe.take() {
            probe.abort();
        }
    }
    pub fn bind(&self, session: String, saved: Option<SavedSession>) {
        self.checkpoint(true);
        {
            let mut s = self.0.borrow_mut();
            Self::cancel(&mut s);
            s.generation = s.generation.wrapping_add(1);
            s.session = session;
            s.source.clear();
            s.element = None;
            s.intent = false;
            s.loaded = false;
            s.pending = None;
            s.retries.reset();
            s.position = saved.as_ref().map_or(0.0, |v| v.position);
            s.saved = saved;
            s.view.playing.set(false);
            {
                let position = s.position;
                s.view.position.set(position);
            }
            s.view.status.set(String::new());
            let duration = s
                .saved
                .as_ref()
                .and_then(|v| v.queue.get(v.cursor))
                .map_or(0.0, |t| t.duration_ms as f64 / 1000.0);
            s.view.duration.set(duration);
        }
        if let Some(element) = audio() {
            let _ = element.remove_attribute("data-play-request");
            let _ = element.remove_attribute("data-source-generation");
            let _ = element.pause();
            let _ = element.remove_attribute("src");
            element.load();
        }
        clear_media_session();
    }
    /// Queue edits change occurrence indices without changing the active source.
    pub fn sync(&self, mut saved: SavedSession) {
        let mut s = self.0.borrow_mut();
        if s.session.is_empty()
            || s.saved
                .as_ref()
                .is_none_or(|old| old.owner_id != saved.owner_id)
        {
            return;
        }
        saved.position = if saved.queue.is_empty() {
            0.0
        } else {
            s.position
        };
        s.saved = Some(saved);
        drop(s);
        self.checkpoint(true);
    }
    pub fn position(&self) -> f64 {
        self.0.borrow().position
    }
    pub fn wants_play(&self) -> bool {
        self.0.borrow().intent
    }
    pub fn checkpoint(&self, force: bool) {
        let mut s = self.0.borrow_mut();
        let now = js_sys::Date::now();
        if !force && now - s.last_save < 5000.0 {
            return;
        }
        let position = s.position;
        if let Some(saved) = s.saved.as_mut() {
            saved.position = if saved.queue.is_empty() {
                0.0
            } else {
                position
            };
            if let Some(raw) = saved.encode() {
                storage_set(&format!("vela.{}.playback.v1", saved.owner_id), &raw);
            }
        }
        s.last_save = now;
    }
    fn valid(&self, generation: u64) -> Option<HtmlAudioElement> {
        let s = self.0.borrow();
        let element = audio()?;
        (!s.session.is_empty()
            && crate::token(s.view.auth) == s.session
            && s.element.as_ref() == Some(&element)
            && !s.source.is_empty()
            && s.generation == generation
            && element.src() == s.source
            && element.get_attribute("data-source-generation").as_deref()
                == Some(&generation.to_string()))
        .then_some(element)
    }
    pub fn select(&self, track: &Track) {
        self.0.borrow_mut().retries.reset();
        self.load(track, 0.0);
    }
    fn load(&self, track: &Track, position: f64) {
        let Some(element) = audio() else {
            return;
        };
        let (generation, volume) = {
            let mut s = self.0.borrow_mut();
            if s.session.is_empty() || crate::token(s.view.auth) != s.session {
                return;
            }
            Self::cancel(&mut s);
            s.generation = s.generation.wrapping_add(1);
            s.loaded = false;
            s.intent = true;
            s.position = clamp_position(position, track.duration_ms as f64 / 1000.0);
            s.pending = Some(s.position);
            s.progress_at = js_sys::Date::now();
            s.stable_from = s.position;
            s.stable = false;
            s.view.playing.set(false);
            {
                let position = s.position;
                s.view.position.set(position);
            }
            s.view.duration.set(track.duration_ms as f64 / 1000.0);
            s.view.status.set(String::new());
            (s.generation, s.saved.as_ref().map_or(0.8, |v| v.volume))
        };
        let _ = element.remove_attribute("data-play-request");
        let _ = element.set_attribute("data-source-generation", &generation.to_string());
        element.set_volume(volume);
        element.set_src(&format!("/api/tracks/{}/stream", track.id));
        {
            let mut s = self.0.borrow_mut();
            s.source = element.src();
            s.element = Some(element.clone());
        }
        element.load();
        update_media_metadata(track);
        self.checkpoint(true);
        // A new selection can start within the user gesture; a resume must seek first.
        if position == 0.0 {
            self.play(generation);
        }
        self.arm_stall();
    }
    pub fn resume(&self) {
        let (generation, source, track, position, loaded) = {
            let s = self.0.borrow();
            (
                s.generation,
                s.source.clone(),
                s.saved
                    .as_ref()
                    .and_then(|v| v.queue.get(v.cursor))
                    .cloned(),
                s.position,
                s.loaded,
            )
        };
        if let Some(track) = track {
            if source.is_empty() || !loaded || audio().is_some_and(|e| e.error().is_some()) {
                self.0.borrow_mut().retries.reset();
                self.load(&track, position);
            } else {
                self.0.borrow_mut().intent = true;
                self.0.borrow_mut().view.status.set(String::new());
                self.play(generation);
                self.arm_stall();
            }
        }
    }
    pub fn pause(&self) {
        {
            let mut s = self.0.borrow_mut();
            s.intent = false;
            Self::cancel(&mut s);
            s.generation = s.generation.wrapping_add(1);
            if let Some(e) = audio() {
                let _ = e.set_attribute("data-source-generation", &s.generation.to_string());
                let _ = e.remove_attribute("data-play-request");
                let _ = e.pause();
            }
            s.view.playing.set(false);
            s.view.status.set(String::new());
        }
        self.checkpoint(true);
    }
    pub fn seek(&self, position: f64) {
        let mut s = self.0.borrow_mut();
        let duration = s
            .saved
            .as_ref()
            .and_then(|v| v.queue.get(v.cursor))
            .map_or(0.0, |t| t.duration_ms as f64 / 1000.0);
        let position = if position.is_finite() && duration.is_finite() && duration > 0.0 {
            position.clamp(0.0, duration)
        } else {
            clamp_position(position, duration)
        };
        s.position = position;
        s.view.position.set(position);
        if s.loaded {
            if let Some(e) = audio() {
                e.set_current_time(position);
            }
        } else {
            s.pending = Some(position);
        }
        drop(s);
        self.checkpoint(true);
    }
    fn play(&self, generation: u64) {
        let Some(element) = self.valid(generation) else {
            return;
        };
        if !self.0.borrow().intent {
            return;
        }
        let request = uuid::Uuid::new_v4().to_string();
        let _ = element.set_attribute("data-play-request", &request);
        let promise = element.play();
        let player = self.clone();
        spawn_local(async move {
            let result = match promise {
                Ok(p) => JsFuture::from(p).await,
                Err(e) => Err(e),
            };
            if let Err(error) = result {
                if player.valid(generation).is_none()
                    || element.get_attribute("data-play-request").as_deref() != Some(&request)
                {
                    return;
                }
                let name = js_sys::Reflect::get(&error, &"name".into())
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                if name == "AbortError" {
                    return;
                }
                if name == "NotAllowedError" {
                    player.pause();
                    player
                        .0
                        .borrow_mut()
                        .view
                        .status
                        .set("Your browser needs a new Play gesture to start audio.".into());
                } else {
                    player.recover();
                }
            }
        });
    }
    pub fn metadata(&self) {
        let generation = self.0.borrow().generation;
        let Some(e) = self
            .valid(generation)
            .filter(|e| e.ready_state() >= 1 && e.current_src() == e.src())
        else {
            return;
        };
        let intent = {
            let mut s = self.0.borrow_mut();
            s.loaded = true;
            s.view.duration.set(if e.duration().is_finite() {
                e.duration()
            } else {
                0.0
            });
            if let Some(pending) = s.pending.take() {
                let p = clamp_position(pending, e.duration());
                e.set_current_time(p);
                s.position = p;
                s.view.position.set(p);
            }
            s.intent
        };
        if intent {
            self.play(generation);
        }
    }
    pub fn time(&self, force: bool) {
        let generation = self.0.borrow().generation;
        let Some(e) = self.valid(generation) else {
            return;
        };
        {
            let mut s = self.0.borrow_mut();
            if !s.loaded || s.pending.is_some() || !e.current_time().is_finite() {
                return;
            }
            if (e.current_time() - s.position).abs() > 0.01 {
                s.progress_at = js_sys::Date::now();
            }
            s.position = if e.ended() { 0.0 } else { e.current_time() };
            {
                let position = s.position;
                s.view.position.set(position);
            }
            if !s.stable && s.position >= s.stable_from + 3.0 {
                s.stable = true;
                s.retries.reset();
            }
        }
        self.checkpoint(force);
    }
    pub fn playing(&self, value: bool) {
        let generation = self.0.borrow().generation;
        if self.valid(generation).is_some() {
            self.0.borrow_mut().view.playing.set(value);
            if !value {
                self.time(true);
            }
        }
    }
    pub fn ended(&self) -> bool {
        let generation = self.0.borrow().generation;
        if !self.0.borrow().intent || !self.valid(generation).is_some_and(|e| e.ended()) {
            return false;
        }
        {
            let mut s = self.0.borrow_mut();
            s.position = 0.0;
            s.view.position.set(0.0);
            s.intent = false;
            Self::cancel(&mut s);
        }
        self.checkpoint(true);
        true
    }
    pub fn failed(&self) {
        let generation = self.0.borrow().generation;
        let Some(e) = self.valid(generation) else {
            return;
        };
        if e.error().is_none() {
            return;
        }
        if e.error().is_some_and(|e| e.code() == 3) {
            self.pause();
            self.0.borrow_mut().view.status.set(
                "The browser could not decode this file. Retry or choose another track.".into(),
            );
        } else {
            self.recover();
        }
    }
    fn recover(&self) {
        let (generation, delay) = {
            let mut s = self.0.borrow_mut();
            if !s.intent || s.retry.is_some() || s.probe.is_some() {
                return;
            }
            s.stall = None;
            let Some(delay) = s.retries.next_delay() else {
                s.intent = false;
                s.view.playing.set(false);
                s.view.status.set(
                    "Connection interrupted. Press Play to retry from your saved position.".into(),
                );
                return;
            };
            s.view
                .status
                .set("Connection interrupted. Reconnecting…".into());
            (s.generation, delay)
        };
        let weak = Rc::downgrade(&self.0);
        self.0.borrow_mut().retry = Some(Timeout::new(delay, move || {
            if let Some(s) = weak.upgrade() {
                let p = Self(s);
                p.0.borrow_mut().retry = None;
                p.probe(generation);
            }
        }));
    }
    fn probe(&self, generation: u64) {
        if self.valid(generation).is_none() || !self.0.borrow().intent {
            return;
        }
        let Ok(controller) = AbortController::new() else {
            self.recover();
            return;
        };
        let signal = controller.signal();
        let deadline_controller = controller.clone();
        let deadline = Timeout::new(10000, move || deadline_controller.abort());
        self.0.borrow_mut().probe = Some(controller);
        let source = self.0.borrow().source.clone();
        let player = self.clone();
        spawn_local(async move {
            let response = gloo_net::http::RequestBuilder::new(&source)
                .method(gloo_net::http::Method::HEAD)
                .abort_signal(Some(&signal))
                .send()
                .await;
            drop(deadline);
            if player.valid(generation).is_none() || !player.0.borrow().intent {
                return;
            }
            player.0.borrow_mut().probe = None;
            match response.map(|r| r.status()) {
                Ok(200 | 206) => {
                    let pair = {
                        let s = player.0.borrow();
                        s.saved
                            .as_ref()
                            .and_then(|v| v.queue.get(v.cursor))
                            .cloned()
                            .map(|t| (t, s.position))
                    };
                    if let Some((track, position)) = pair {
                        player.load(&track, position);
                    }
                }
                Ok(401) => {
                    player.pause();
                    let mut s = player.0.borrow_mut();
                    s.view.expired.set(true);
                }
                Ok(403 | 404) => {
                    player.pause();
                    player.0.borrow_mut().view.status.set("This track is no longer available to your account. Choose another track or retry.".into());
                }
                _ => player.recover(),
            }
        });
    }
    fn arm_stall(&self) {
        let mut s = self.0.borrow_mut();
        if !s.intent || s.stall.is_some() {
            return;
        }
        let generation = s.generation;
        let weak = Rc::downgrade(&self.0);
        s.stall = Some(Timeout::new(8000, move || {
            if let Some(state) = weak.upgrade() {
                let player = Self(state);
                player.0.borrow_mut().stall = None;
                if player.valid(generation).is_none() || !player.0.borrow().intent {
                    return;
                }
                if js_sys::Date::now() - player.0.borrow().progress_at >= 7900.0 {
                    player.recover();
                } else {
                    player.arm_stall();
                }
            }
        }));
    }
}
