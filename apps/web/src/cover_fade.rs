//! DR-03: the artwork crossfade of the root player and the Now Playing
//! sheet.
//!
//! Two layers at most, one generation at a time and no timer anywhere. The
//! outgoing layer stays until the incoming one is really there, the effect
//! ends when the browser says it ended, and a cancelled effect — a reader
//! who turns reduced motion on, a surface that changes — never waits for an
//! end event that will not arrive.
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

    /// The stable key of the layer. The picture owns the key, so a track
    /// change that keeps the same cover reuses the same element and never
    /// asks the server for that picture twice.
    fn layer_key(&self) -> String {
        match &self.artwork_id {
            Some(artwork) => format!("art-{artwork}"),
            None => format!("placeholder-{}", self.track_id),
        }
    }
}

/// One layer of the crossfade: the picture on the screen and the track it
/// belongs to.
#[derive(Clone, Debug, PartialEq)]
pub struct CoverLayer {
    pub identity: CoverIdentity,
    pub track: Track,
}

/// The state of one crossfade surface.
///
/// `visual` counts the generations of the incoming layer. Every load
/// result and every ended effect carries the generation it belongs to, so a
/// result of an older layer is dropped instead of painting over a newer
/// one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CoverFadeModel {
    owner: String,
    lifetime: u64,
    current: Option<CoverLayer>,
    previous: Option<CoverLayer>,
    visual: u64,
    ready: u64,
    leaving: bool,
}

impl CoverFadeModel {
    /// Take the picture the caller asks for.
    ///
    /// The change is applied before the markup is built, so the outgoing
    /// layer is already in the document when the incoming one appears. A
    /// picture that did not change, a picture that appears where there was
    /// none, a track without a picture and a new session are not a
    /// crossfade: the picture changes at once and nothing of the previous
    /// session stays on the screen.
    pub fn select(&mut self, layer: CoverLayer, owner: &str, lifetime: u64, reduce: bool) {
        let session_changed = self.owner != owner || self.lifetime != lifetime;
        self.owner = owner.to_string();
        self.lifetime = lifetime;
        if session_changed {
            self.previous = None;
            self.leaving = false;
        }
        let same_identity =
            self.current.as_ref().map(|current| &current.identity) == Some(&layer.identity);
        if same_identity {
            // The same picture: the track behind it may have changed, the
            // layer does not.
            if let Some(current) = self.current.as_mut() {
                current.track = layer.track;
            }
            return;
        }
        let previous_picture = self
            .current
            .as_ref()
            .and_then(|current| current.identity.artwork_id.as_deref());
        let has_picture = layer.identity.artwork_id.is_some();
        let new_picture = layer.identity.artwork_id.as_deref();
        let fades = !reduce
            && !session_changed
            && previous_picture.is_some()
            && has_picture
            && previous_picture != new_picture;
        self.visual = self.visual.wrapping_add(1);
        self.ready = 0;
        self.leaving = false;
        self.previous = if fades { self.current.take() } else { None };
        self.current = Some(layer);
        if reduce || !has_picture {
            // A track without a picture shows the placeholder at once, and a
            // reader who asked for less movement gets the picture at once.
            self.ready = self.visual;
        }
    }

    /// Accept the result of a load. The result belongs to the layer that
    /// asked for it: a stale layer, a stale account or a stale session is
    /// dropped instead of starting an effect.
    pub fn accept_ready(&mut self, visual: u64, owner: &str, lifetime: u64) -> bool {
        if visual != self.visual || owner != self.owner || lifetime != self.lifetime {
            return false;
        }
        if self.ready == visual {
            return false;
        }
        self.ready = visual;
        self.leaving = self.previous.is_some();
        true
    }

    /// Drop the outgoing layer. The effect is over, or it was cancelled.
    pub fn release(&mut self, visual: u64) -> bool {
        if visual != self.visual || self.previous.is_none() {
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
            } else if self.ready == self.visual {
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
            let released = {
                let mut state = model.borrow_mut();
                let visual = state.visual;
                state.release(visual)
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

    let reduce = reduced_motion();
    {
        let mut state = model.borrow_mut();
        state.select(
            CoverLayer {
                identity: CoverIdentity::of(&track),
                track: track.clone(),
            },
            &owner,
            lifetime,
            reduce,
        );
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
                    let key = layer.identity.layer_key();
                    let visual = state.visual;
                    let track = layer.track.clone();
                    let model = model.clone();
                    let owner = owner.clone();
                    let leaving = class.ends_with("is-leaving");
                    rsx! {
                        div { key: "{key}", class: "{class}",
                            "aria-hidden": if leaving { "true" } else { "false" },
                            onanimationend: {
                                let model = model.clone();
                                let mut revision = revision;
                                move |event: Event<AnimationData>| {
                                    if event.animation_name() != COVER_OUT { return; }
                                    let released = {
                                        let mut state = model.borrow_mut();
                                        state.release(visual)
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
                                    move |_| {
                                        let accepted = {
                                            let mut state = model.borrow_mut();
                                            state.accept_ready(visual, &owner, lifetime)
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

    fn track(id: &str, artwork: Option<&str>) -> Track {
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

    fn layer(id: &str, artwork: Option<&str>) -> CoverLayer {
        CoverLayer {
            identity: CoverIdentity::of(&track(id, artwork)),
            track: track(id, artwork),
        }
    }

    fn model() -> CoverFadeModel {
        CoverFadeModel::default()
    }

    #[test]
    fn the_first_picture_is_not_a_fade() {
        let mut state = model();
        state.select(layer("a", Some("art-a")), "owner", 1, false);
        assert_eq!(state.layers().len(), 1);
        assert!(state.previous.is_none());
        assert_eq!(state.visual, 1);
    }

    #[test]
    fn another_picture_keeps_the_outgoing_layer_until_the_new_one_is_there() {
        let mut state = model();
        state.select(layer("a", Some("art-a")), "owner", 1, false);
        state.select(layer("b", Some("art-b")), "owner", 1, false);
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
        assert!(state.accept_ready(state.visual, "owner", 1));
        assert!(state.leaving, "the outgoing layer leaves now");
        assert!(state.layers()[0].1.ends_with("is-ready"));
        assert!(state.release(state.visual));
        assert_eq!(state.layers().len(), 1, "the outgoing layer is gone");
    }

    #[test]
    fn the_same_picture_never_starts_a_fade() {
        let mut state = model();
        state.select(layer("a", Some("art-a")), "owner", 1, false);
        state.select(layer("b", Some("art-a")), "owner", 1, false);
        assert_eq!(
            state.layers().len(),
            1,
            "the same picture is the same layer"
        );
        assert!(state.previous.is_none());
    }

    #[test]
    fn a_track_without_a_picture_shows_the_placeholder_at_once() {
        let mut state = model();
        state.select(layer("a", Some("art-a")), "owner", 1, false);
        state.select(layer("b", None), "owner", 1, false);
        assert_eq!(state.layers().len(), 1, "no outgoing layer");
        assert!(state.layers()[0].1.ends_with("cover-fade-layer"));
        assert!(!state.layers()[0].1.contains("is-pending"));
    }

    #[test]
    fn reduced_motion_changes_the_picture_at_once() {
        let mut state = model();
        state.select(layer("a", Some("art-a")), "owner", 1, false);
        state.select(layer("b", Some("art-b")), "owner", 1, false);
        assert_eq!(state.layers().len(), 2);
        let mut state = model();
        state.select(layer("a", Some("art-a")), "owner", 1, true);
        state.select(layer("b", Some("art-b")), "owner", 1, true);
        assert_eq!(state.layers().len(), 1, "no outgoing layer");
        assert!(
            state.layers()[0].1.ends_with("is-ready"),
            "the picture is there at once"
        );
    }

    #[test]
    fn a_new_session_keeps_nothing_of_the_old_one() {
        let mut state = model();
        state.select(layer("a", Some("art-a")), "owner", 1, false);
        state.select(layer("b", Some("art-b")), "owner", 1, false);
        state.select(layer("c", Some("art-c")), "other", 2, false);
        assert_eq!(
            state.layers().len(),
            1,
            "the outgoing layer of the old session is gone"
        );
        assert_eq!(state.layers()[0].0.identity.track_id, "c");
    }

    #[test]
    fn a_stale_result_is_dropped() {
        let mut state = model();
        state.select(layer("a", Some("art-a")), "owner", 1, false);
        let stale = state.visual;
        state.select(layer("b", Some("art-b")), "owner", 1, false);
        assert!(
            !state.accept_ready(stale, "owner", 1),
            "the old layer says nothing"
        );
        assert!(
            !state.accept_ready(state.visual, "other", 1),
            "another account says nothing"
        );
        assert!(
            !state.accept_ready(state.visual, "owner", 2),
            "another session says nothing"
        );
        assert!(state.accept_ready(state.visual, "owner", 1));
    }

    #[test]
    fn a_replaced_picture_keeps_two_layers_at_most() {
        let mut state = model();
        state.select(layer("a", Some("art-a")), "owner", 1, false);
        state.select(layer("b", Some("art-b")), "owner", 1, false);
        state.select(layer("c", Some("art-c")), "owner", 1, false);
        assert_eq!(state.layers().len(), 2, "never more than two layers");
        assert_eq!(state.layers()[0].0.identity.track_id, "c");
        assert_eq!(state.layers()[1].0.identity.track_id, "b");
    }
}
