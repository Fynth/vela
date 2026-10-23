//! DR-03: the artwork crossfade of the root player and the Now Playing
//! sheet.
//!
//! Two layers at most, one generation per layer and no timer anywhere. The
//! outgoing layer stays until the incoming one is really there, the effect
//! ends when the browser says it ended, and a cancelled effect — a reader
//! who turns reduced motion on, a surface that changes — never waits for an
//! end event that will not arrive.
//!
//! Every callback of a layer carries the generation of that layer, not of the
//! surface: the late `load` of a picture that is already leaving is dropped
//! instead of being read as the arrival of the picture that replaced it. The
//! readiness itself belongs to the picture, not to the generation, so a track
//! that keeps the same artwork keeps the picture that is already loaded.
//!
//! The state belongs to this component. An account change or a sign-out
//! drops the component and the state with it, so a previous account's
//! picture can never reach the screen and nothing has to be remembered
//! outside the surface that shows it.

use crate::Art;
use dioxus::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use vela_domain::Track;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// The effect that fades a layer out. Only this one releases a layer, so an
/// effect of a child never does.
const COVER_OUT: &str = "vela-cover-out";

/// One number per surface, so the cancel of one surface's layer is never
/// read by another surface.
static COVER_FADE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

/// The identity of one picture: the track it belongs to and the artwork it
/// shows. The artwork owns the layer, so the same picture in another
/// occurrence is the same layer and never starts a fade.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverIdentity {
    pub track_id: String,
    pub artwork_id: Option<String>,
}

impl CoverIdentity {
    fn of(track: &Track) -> Self {
        Self {
            track_id: track.id.clone(),
            artwork_id: track.artwork_id.clone(),
        }
    }

    /// The picture this identity shows, without the track: two occurrences of
    /// the same artwork are the same picture.
    fn picture(&self) -> String {
        match &self.artwork_id {
            Some(artwork) => format!("art-{artwork}"),
            None => "placeholder".to_string(),
        }
    }
}

/// One layer of the crossfade: the picture on the screen, the track it
/// belongs to and the generation of this layer.
///
/// The generation is allocated when the layer is selected and never reused,
/// so a callback of a layer is read by the layer that asked for it and by
/// nobody else.
#[derive(Clone, Debug, PartialEq)]
pub struct CoverLayer {
    pub identity: CoverIdentity,
    pub track: Track,
    pub generation: u64,
}

/// The state of one crossfade surface.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CoverFadeModel {
    owner: String,
    lifetime: u64,
    current: Option<CoverLayer>,
    previous: Option<CoverLayer>,
    /// Allocates the generations of the layers.
    sequence: u64,
    /// The key of the picture the browser has already confirmed. Readiness
    /// belongs to the picture, not to the layer: a track change that keeps
    /// the same artwork keeps the picture that is on the screen.
    ready: Option<String>,
    leaving: bool,
}

impl CoverFadeModel {
    /// The stable key of a picture. It carries the session, so a picture of
    /// another account or another session is always another element with its
    /// own load — and the same picture of the same session is always the same
    /// element, which is never fetched twice.
    fn key_of(&self, identity: &CoverIdentity) -> String {
        format!("{}:{}:{}", self.owner, self.lifetime, identity.picture())
    }

    /// Take the picture the caller asks for.
    ///
    /// The change is applied before the markup is built, so the outgoing
    /// layer is already in the document when the incoming one appears. A
    /// picture that did not change, a picture that appears where there was
    /// none, a track without a picture and a new session are not a
    /// crossfade: the picture changes at once and nothing of the previous
    /// session stays on the screen.
    pub fn select(&mut self, track: Track, owner: &str, lifetime: u64, reduce: bool) {
        let identity = CoverIdentity::of(&track);
        let session_changed = self.owner != owner || self.lifetime != lifetime;
        self.owner = owner.to_string();
        self.lifetime = lifetime;
        if session_changed {
            self.previous = None;
            self.leaving = false;
            self.ready = None;
        }
        let key = self.key_of(&identity);
        let same_picture = self
            .current
            .as_ref()
            .is_some_and(|current| self.key_of(&current.identity) == key);
        if same_picture {
            // The same picture: the track behind it may have changed, the
            // layer and its readiness do not. A reader who asked for less
            // movement is left with no outgoing layer at all, even when the
            // request for less movement arrives in the middle of a fade.
            if let Some(current) = self.current.as_mut() {
                current.track = track;
                current.identity = identity;
            }
            if reduce {
                self.previous = None;
                self.leaving = false;
            }
            return;
        }
        let previous_picture = self
            .current
            .as_ref()
            .and_then(|current| current.identity.artwork_id.as_deref());
        let new_picture = identity.artwork_id.as_deref();
        let has_picture = new_picture.is_some();
        // A picture the browser has already confirmed has nothing left to
        // answer: its element is the one that is already loaded, so no load
        // event will come for the layer that asks for it now. Such a change is
        // applied at once — otherwise the outgoing layer would wait for an
        // answer that never arrives and stay on the screen for good.
        let known = self.ready.as_deref() == Some(key.as_str());
        let fades = !reduce
            && !session_changed
            && !known
            && previous_picture.is_some()
            && has_picture
            && previous_picture != new_picture;
        self.sequence = self.sequence.wrapping_add(1);
        let generation = self.sequence;
        self.leaving = false;
        let outgoing = self.current.take();
        self.previous = if fades { outgoing } else { None };
        if !has_picture || reduce {
            // A track without a picture shows the placeholder at once, and a
            // reader who asked for less movement gets the picture at once.
            // The readiness of another picture stays where it is: it is the
            // answer of the browser about a picture, not a counter.
            self.ready = Some(key);
        }
        self.current = Some(CoverLayer {
            identity,
            track,
            generation,
        });
    }

    /// Accept the result of a load. The result belongs to the layer that
    /// asked for it: a stale layer, a stale account or a stale session is
    /// dropped instead of starting an effect.
    ///
    /// A reader who asked for less movement has nothing to wait for: the
    /// picture is painted and the outgoing layer is dropped in the same
    /// change, so an effect that no longer runs never holds a layer.
    pub fn accept_ready(
        &mut self,
        key: &str,
        generation: u64,
        owner: &str,
        lifetime: u64,
        reduce: bool,
    ) -> bool {
        if owner != self.owner || lifetime != self.lifetime {
            return false;
        }
        let Some(current) = self.current.as_ref() else {
            return false;
        };
        if current.generation != generation || self.key_of(&current.identity) != key {
            return false;
        }
        if self.ready.as_deref() == Some(key) {
            return false;
        }
        self.ready = Some(key.to_string());
        if reduce {
            self.previous = None;
            self.leaving = false;
            return true;
        }
        self.leaving = self.previous.is_some();
        true
    }

    /// Drop the outgoing layer. The effect is over, or it was cancelled.
    pub fn release(&mut self, generation: u64) -> bool {
        if self.previous.as_ref().map(|layer| layer.generation) != Some(generation) {
            return false;
        }
        self.previous = None;
        self.leaving = false;
        true
    }

    /// The layers to paint.
    ///
    /// The incoming layer comes first and the outgoing one second. The
    /// renderer keeps the incoming element when the outgoing one leaves, so
    /// the picture that arrives is never fetched a second time. The paint
    /// order is the stylesheet's business: the outgoing layer paints below
    /// the incoming one.
    pub fn layers(&self) -> Vec<(CoverLayer, &'static str)> {
        let mut layers = Vec::new();
        if let Some(current) = &self.current {
            let class = if current.identity.artwork_id.is_none() {
                "cover-fade-layer"
            } else if self.ready.as_deref() == Some(self.key_of(&current.identity).as_str()) {
                "cover-fade-layer is-ready"
            } else {
                "cover-fade-layer is-pending"
            };
            layers.push((current.clone(), class));
        }
        if let Some(previous) = &self.previous {
            let class = if self.leaving {
                "cover-fade-layer is-leaving"
            } else {
                "cover-fade-layer"
            };
            layers.push((previous.clone(), class));
        }
        layers
    }
}

/// Whether the reader asked for less movement.
pub fn reduced_motion() -> bool {
    web_sys::window()
        .and_then(|window| window.match_media("(prefers-reduced-motion: reduce)").ok())
        .flatten()
        .map(|list| list.matches())
        .unwrap_or(false)
}

/// The listener that reads the cancelled effects of this surface. A cancel
/// never sends an end event, so the layer has to be released here or it
/// would stay on the screen for good.
#[derive(Clone)]
struct CancelListener(Rc<CancelListenerInner>);

struct CancelListenerInner {
    document: Option<web_sys::Document>,
    closure: Option<Closure<dyn FnMut(web_sys::Event)>>,
}

impl CancelListener {
    fn attach(
        model: Rc<RefCell<CoverFadeModel>>,
        revision: Signal<u64>,
        container: String,
    ) -> Self {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return Self(Rc::new(CancelListenerInner {
                document: None,
                closure: None,
            }));
        };
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
            let Some(target) = event.target() else {
                return;
            };
            let Some(element) = target.dyn_into::<web_sys::Element>().ok() else {
                return;
            };
            // Only a layer of this surface counts.
            if element
                .closest(&format!("#{container}"))
                .ok()
                .flatten()
                .is_none()
            {
                return;
            }
            let class = element.get_attribute("class").unwrap_or_default();
            if !class.split_whitespace().any(|name| name == "is-leaving") {
                return;
            }
            // The cancelled effect belongs to one layer: the generation of the
            // element says which one, so a cancel of an older layer never
            // releases the layer that replaced it.
            let Some(generation) = element
                .get_attribute("data-generation")
                .and_then(|raw| raw.parse::<u64>().ok())
            else {
                return;
            };
            let released = {
                let mut state = model.borrow_mut();
                state.release(generation)
            };
            if released {
                let mut revision = revision;
                let next = revision.peek().wrapping_add(1);
                revision.set(next);
            }
        });
        let _ = document
            .add_event_listener_with_callback("animationcancel", closure.as_ref().unchecked_ref());
        Self(Rc::new(CancelListenerInner {
            document: Some(document),
            closure: Some(closure),
        }))
    }

    fn detach(&self) {
        let inner = &*self.0;
        if let (Some(document), Some(closure)) = (inner.document.as_ref(), inner.closure.as_ref()) {
            let _ = document.remove_event_listener_with_callback(
                "animationcancel",
                closure.as_ref().unchecked_ref(),
            );
        }
    }
}

/// The listener that reads a change of the reader's motion preference.
///
/// A reader who turns reduced motion on while a picture is still on its way
/// is not waiting for the next track change: the surface re-renders in that
/// moment, and the outgoing layer is dropped there. The media query is the
/// only source of this decision, so it is listened to rather than polled.
#[derive(Clone)]
struct ReduceListener(Rc<ReduceListenerInner>);

struct ReduceListenerInner {
    query: Option<web_sys::MediaQueryList>,
    closure: Option<Closure<dyn FnMut(web_sys::Event)>>,
}

impl ReduceListener {
    fn attach(revision: Signal<u64>) -> Self {
        let Some(query) = web_sys::window()
            .and_then(|window| window.match_media("(prefers-reduced-motion: reduce)").ok())
            .flatten()
        else {
            return Self(Rc::new(ReduceListenerInner {
                query: None,
                closure: None,
            }));
        };
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            let mut revision = revision;
            let next = revision.peek().wrapping_add(1);
            revision.set(next);
        });
        let _ = query.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref());
        Self(Rc::new(ReduceListenerInner {
            query: Some(query),
            closure: Some(closure),
        }))
    }

    fn detach(&self) {
        let inner = &*self.0;
        if let (Some(query), Some(closure)) = (inner.query.as_ref(), inner.closure.as_ref()) {
            let _ = query
                .remove_event_listener_with_callback("change", closure.as_ref().unchecked_ref());
        }
    }
}

/// The artwork of one surface.
///
/// The component owns the crossfade: the outgoing picture stays until the
/// incoming one is really there, and both are layers of the same box. The
/// picture is never scaled, cropped or reflowed — only its opacity moves.
#[component]
pub fn CoverFade(
    track: Track,
    #[props(default = false)] large: bool,
    owner: String,
    lifetime: u64,
) -> Element {
    let model = use_hook(|| Rc::new(RefCell::new(CoverFadeModel::default())));
    let revision = use_signal(|| 0_u64);
    // The surface re-renders when a load result or an ended effect asks it
    // to. The identity itself is applied during the render, so the outgoing
    // layer is already in the document when the incoming one appears.
    let _ = revision.read();
    let instance = use_hook(|| COVER_FADE_SEQUENCE.fetch_add(1, Ordering::Relaxed));
    let container = format!("cover-fade-{instance}");
    let listener = use_hook(|| CancelListener::attach(model.clone(), revision, container.clone()));
    use_drop(move || listener.detach());
    let reduce_listener = use_hook(|| ReduceListener::attach(revision));
    use_drop(move || reduce_listener.detach());

    let reduce = reduced_motion();
    {
        let mut state = model.borrow_mut();
        state.select(track.clone(), &owner, lifetime, reduce);
    }
    let state = model.borrow().clone();
    let layers = state.layers();
    let artwork_class = if large {
        " artwork artwork-large"
    } else {
        " artwork"
    };
    rsx! {
        div { id: "{container}", class: "cover-fade{artwork_class}",
            for (layer, class) in layers {
                {
                    let key = state.key_of(&layer.identity);
                    let generation = layer.generation;
                    let track = layer.track.clone();
                    let model = model.clone();
                    let owner = owner.clone();
                    let leaving = class.ends_with("is-leaving");
                    rsx! {
                        div { key: "{key}", class: "{class}",
                            "data-generation": "{generation}",
                            "aria-hidden": if leaving { "true" } else { "false" },
                            onanimationend: {
                                let model = model.clone();
                                let mut revision = revision;
                                move |event: Event<AnimationData>| {
                                    if event.animation_name() != COVER_OUT { return; }
                                    let released = {
                                        let mut state = model.borrow_mut();
                                        state.release(generation)
                                    };
                                    if released {
                                        let next = revision.peek().wrapping_add(1);
                                        revision.set(next);
                                    }
                                }
                            },
                            Art {
                                key: "{key}",
                                track,
                                onready: {
                                    let model = model.clone();
                                    let mut revision = revision;
                                    let owner = owner.clone();
                                    let key = key.clone();
                                    move |_| {
                                        let accepted = {
                                            let mut state = model.borrow_mut();
                                            state.accept_ready(
                                                &key, generation, &owner, lifetime, reduce,
                                            )
                                        };
                                        if accepted {
                                            let next = revision.peek().wrapping_add(1);
                                            revision.set(next);
                                        }
                                    }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track_of(id: &str, artwork: Option<&str>) -> Track {
        Track {
            id: id.to_string(),
            title: id.to_string(),
            artist: "Vela".into(),
            album: String::new(),
            album_artist: String::new(),
            genre: String::new(),
            year: None,
            track_number: None,
            disc_number: None,
            duration_ms: 1000,
            codec: "FLAC".into(),
            bitrate: None,
            sample_rate: None,
            bit_depth: None,
            channels: None,
            size_bytes: 10,
            artwork_id: artwork.map(|artwork| artwork.to_string()),
            added_at: 0,
            favorite: false,
        }
    }

    fn model() -> CoverFadeModel {
        CoverFadeModel::default()
    }

    /// Select the track and return the key and the generation of the layer
    /// the surface is waiting for.
    fn select(
        state: &mut CoverFadeModel,
        id: &str,
        artwork: Option<&str>,
        reduce: bool,
    ) -> (String, u64) {
        state.select(track_of(id, artwork), "owner", 1, reduce);
        let current = state.current.as_ref().expect("a layer is selected");
        (state.key_of(&current.identity), current.generation)
    }

    #[test]
    fn the_first_picture_is_not_a_fade() {
        let mut state = model();
        select(&mut state, "a", Some("art-a"), false);
        assert_eq!(state.layers().len(), 1);
        assert!(state.previous.is_none());
        assert_eq!(state.sequence, 1);
    }

    #[test]
    fn another_picture_keeps_the_outgoing_layer_until_the_new_one_is_there() {
        let mut state = model();
        select(&mut state, "a", Some("art-a"), false);
        let (key, generation) = select(&mut state, "b", Some("art-b"), false);
        let layers = state.layers();
        assert_eq!(layers.len(), 2, "two layers at most");
        assert_eq!(
            layers[0].0.identity.track_id, "b",
            "the incoming layer comes first"
        );
        assert_eq!(
            layers[1].0.identity.track_id, "a",
            "the outgoing one is still there"
        );
        assert!(!state.leaving, "the outgoing layer waits for the new one");
        // The new picture is not there yet: it stays invisible.
        assert!(layers[0].1.ends_with("is-pending"));
        assert!(state.accept_ready(&key, generation, "owner", 1, false));
        assert!(state.leaving, "the outgoing layer leaves now");
        assert!(state.layers()[0].1.ends_with("is-ready"));
        let outgoing = state
            .previous
            .as_ref()
            .expect("the outgoing layer is still there")
            .generation;
        assert!(
            state.release(outgoing),
            "the answer of the outgoing layer releases it"
        );
        assert!(
            !state.release(generation),
            "the answer of the incoming layer releases nothing"
        );
        assert_eq!(state.layers().len(), 1, "the outgoing layer is gone");
    }

    #[test]
    fn the_same_picture_never_starts_a_fade() {
        let mut state = model();
        select(&mut state, "a", Some("art-a"), false);
        select(&mut state, "b", Some("art-a"), false);
        assert_eq!(
            state.layers().len(),
            1,
            "the same picture is the same layer"
        );
        assert!(state.previous.is_none());
        assert_eq!(state.sequence, 1, "the same layer is not a generation");
    }

    #[test]
    fn a_track_without_a_picture_shows_the_placeholder_at_once() {
        let mut state = model();
        select(&mut state, "a", Some("art-a"), false);
        select(&mut state, "b", None, false);
        assert_eq!(state.layers().len(), 1, "no outgoing layer");
        assert!(state.layers()[0].1.ends_with("cover-fade-layer"));
        assert!(!state.layers()[0].1.contains("is-pending"));
    }

    #[test]
    fn reduced_motion_changes_the_picture_at_once() {
        let mut state = model();
        select(&mut state, "a", Some("art-a"), false);
        select(&mut state, "b", Some("art-b"), false);
        assert_eq!(state.layers().len(), 2);
        let mut state = model();
        select(&mut state, "a", Some("art-a"), true);
        let (key, generation) = select(&mut state, "b", Some("art-b"), true);
        assert_eq!(state.layers().len(), 1, "no outgoing layer");
        assert!(
            state.layers()[0].1.ends_with("is-ready"),
            "the picture is there at once"
        );
        assert!(
            !state.accept_ready(&key, generation, "owner", 1, false),
            "the layer that is already there asks for nothing"
        );
    }

    #[test]
    fn a_new_session_keeps_nothing_of_the_old_one() {
        let mut state = model();
        select(&mut state, "a", Some("art-a"), false);
        select(&mut state, "b", Some("art-b"), false);
        state.select(track_of("c", Some("art-c")), "other", 2, false);
        assert_eq!(
            state.layers().len(),
            1,
            "the outgoing layer of the old session is gone"
        );
        assert_eq!(state.layers()[0].0.identity.track_id, "c");
        assert!(
            state.layers()[0].1.ends_with("is-pending"),
            "a picture of another session is never read as ready"
        );
    }

    #[test]
    fn a_stale_result_is_dropped() {
        let mut state = model();
        select(&mut state, "a", Some("art-a"), false);
        let (stale_key, stale) = select(&mut state, "a", Some("art-a"), false);
        let (key, generation) = select(&mut state, "b", Some("art-b"), false);
        assert!(
            !state.accept_ready(&stale_key, stale, "owner", 1, false),
            "the old layer says nothing"
        );
        assert!(
            !state.accept_ready(&key, generation, "other", 1, false),
            "another account says nothing"
        );
        assert!(
            !state.accept_ready(&key, generation, "owner", 2, false),
            "another session says nothing"
        );
        assert!(state.accept_ready(&key, generation, "owner", 1, false));
    }

    /// The late load of the picture that is already leaving is not the
    /// arrival of the picture that replaced it. The surface keeps waiting for
    /// the picture it asks for, and the leaving layer is never painted as
    /// the incoming one.
    #[test]
    fn the_late_load_of_the_outgoing_layer_is_not_the_arrival_of_the_new_one() {
        let mut state = model();
        select(&mut state, "a", Some("art-a"), false);
        let (leaving_key, leaving) = select(&mut state, "b", Some("art-b"), false);
        // The reader asks for a third picture before B ever answered.
        let (key, generation) = select(&mut state, "c", Some("art-c"), false);
        assert_eq!(state.layers().len(), 2, "two layers at most");
        assert!(
            !state.accept_ready(&leaving_key, leaving, "owner", 1, false),
            "the answer of the layer that is leaving is dropped"
        );
        assert!(
            state.layers()[0].1.ends_with("is-pending"),
            "the incoming picture is still not there"
        );
        assert!(!state.leaving, "the outgoing layer did not start to leave");
        assert!(state.accept_ready(&key, generation, "owner", 1, false));
        assert!(state.leaving, "the outgoing layer leaves on its own answer");
    }

    /// Two tracks with one artwork are one picture: the ready element is
    /// kept, so no load is waited for and the cover never disappears.
    #[test]
    fn another_track_with_the_same_artwork_keeps_the_loaded_picture() {
        let mut state = model();
        let (key, generation) = select(&mut state, "a", Some("art-a"), false);
        assert!(state.accept_ready(&key, generation, "owner", 1, false));
        let same = select(&mut state, "b", Some("art-a"), false);
        assert_eq!(state.layers().len(), 1, "one picture, one layer");
        assert!(
            state.layers()[0].1.ends_with("is-ready"),
            "the loaded picture is still the picture"
        );
        assert_eq!(
            state.layers()[0].0.identity.track_id,
            "b",
            "the track follows the picture"
        );
        assert_eq!(same.1, generation, "the layer is the same layer");
    }

    /// The reader who turns reduced motion on in the middle of a fade is
    /// left with one layer: the outgoing one is dropped, not held until an
    /// effect that no longer runs.
    #[test]
    fn reduced_motion_in_the_middle_of_a_fade_drops_the_outgoing_layer() {
        let mut state = model();
        select(&mut state, "a", Some("art-a"), false);
        let (key, generation) = select(&mut state, "b", Some("art-b"), false);
        assert_eq!(state.layers().len(), 2);
        // The same picture is asked for again, now with reduced motion on.
        state.select(track_of("b", Some("art-b")), "owner", 1, true);
        assert_eq!(
            state.layers().len(),
            1,
            "no outgoing layer after the reader asked for less movement"
        );
        assert!(
            !state.leaving,
            "nothing waits for an effect that will not run"
        );
        assert!(
            !state.layers()[0].1.contains("is-leaving"),
            "the layer that stayed is not a leaving one"
        );
        // The answer of the picture still arrives and paints the layer; the
        // surface is not left waiting for a fade that no longer runs.
        assert!(state.accept_ready(&key, generation, "owner", 1, true));
        assert!(state.layers()[0].1.ends_with("is-ready"));
        assert_eq!(state.layers().len(), 1, "still one layer");
        // A picture asked for under reduced motion is there at once.
        let (other_key, other) = select(&mut state, "c", Some("art-c"), true);
        assert!(
            !state.accept_ready(&other_key, other, "owner", 1, true),
            "a picture that is already there asks for nothing"
        );
        assert!(state.layers()[0].1.ends_with("is-ready"));
    }

    #[test]
    fn a_replaced_picture_keeps_two_layers_at_most() {
        let mut state = model();
        select(&mut state, "a", Some("art-a"), false);
        select(&mut state, "b", Some("art-b"), false);
        select(&mut state, "c", Some("art-c"), false);
        assert_eq!(state.layers().len(), 2, "never more than two layers");
        assert_eq!(state.layers()[0].0.identity.track_id, "c");
        assert_eq!(state.layers()[1].0.identity.track_id, "b");
    }

    /// A picture that comes back before the picture that replaced it ever
    /// answered is still the picture the browser already confirmed. Its
    /// element is the loaded one, no load event will come for the new layer,
    /// and a fade would wait for an answer that never arrives: the returned
    /// picture is painted at once and nothing of the interrupted one stays.
    #[test]
    fn a_confirmed_picture_that_comes_back_is_painted_at_once() {
        let mut state = model();
        let (key_a, generation_a) = select(&mut state, "a", Some("art-a"), false);
        assert!(state.accept_ready(&key_a, generation_a, "owner", 1, false));
        // B is asked for and never answers.
        select(&mut state, "b", Some("art-b"), false);
        assert_eq!(state.layers().len(), 2, "B waits for its own answer");
        // A is asked for again before B answered.
        let (key_a_again, _) = select(&mut state, "a", Some("art-a"), false);
        assert_eq!(key_a_again, key_a, "the same picture, the same element");
        assert_eq!(
            state.layers().len(),
            1,
            "nothing waits for a load that already happened"
        );
        assert!(
            state.layers()[0].1.ends_with("is-ready"),
            "the picture that is already loaded is painted"
        );
        assert!(!state.leaving, "there is no outgoing layer to leave");
    }
}
