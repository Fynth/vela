//! Private, on-demand discovery using real library and consented listening signals.
//! Presentation never resolves audio or owns another playback engine.
use crate::{
    client::{self, RequestScope},
    experience::Actions,
    icons::Icon,
    listening::ListeningView,
    session::{ContextKind, PlaybackContext},
    token,
};
use dioxus::prelude::*;
use gloo_timers::callback::Timeout;
use std::{cell::Cell, rc::Rc};
use vela_domain::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

/// Check the live signals as well as the request generation. A privacy mutation
/// publishes `busy` before it publishes a new privacy generation/revision.
#[derive(Clone, PartialEq)]
struct Context {
    session: String,
    privacy_generation: u64,
    privacy_revision: Option<i64>,
    history_enabled: bool,
    listening_revision: u64,
    catalog_revision: u64,
    blocked: bool,
}
impl Context {
    fn current(
        auth: Signal<Option<AuthResponse>>,
        view: Signal<ListeningView>,
        revision: Signal<u64>,
    ) -> Self {
        let view = view.peek();
        Self {
            session: token(auth),
            privacy_generation: view.privacy_generation,
            privacy_revision: view.preferences.as_ref().map(|value| value.revision),
            history_enabled: view.preferences.as_ref().is_some_and(|value| value.enabled),
            listening_revision: view.revision,
            catalog_revision: *revision.peek(),
            blocked: view.loading || view.busy || view.preferences.is_none(),
        }
    }
    fn usable(&self) -> bool {
        !self.blocked && !self.session.is_empty()
    }
    fn same_privacy(&self, other: &Self) -> bool {
        self.session == other.session
            && self.privacy_generation == other.privacy_generation
            && self.privacy_revision == other.privacy_revision
            && self.history_enabled == other.history_enabled
            && self.blocked == other.blocked
    }
}

#[derive(Clone)]
struct Snapshot {
    context: Context,
    data: Recommendations,
}

#[derive(Clone)]
struct HiddenTrack {
    session: String,
    track: Track,
}

#[derive(Clone)]
struct Ui {
    auth: Signal<Option<AuthResponse>>,
    view: Signal<ListeningView>,
    revision: Signal<u64>,
    generation: Signal<u64>,
    refresh: Signal<u64>,
    context: Signal<Option<(Context, u64)>>,
    data: Signal<Option<Snapshot>>,
    loading: Signal<bool>,
    busy: Signal<bool>,
    exporting: Signal<bool>,
    error: Signal<String>,
    notice: Signal<String>,
    undo: Signal<Option<HiddenTrack>>,
    confirm: Signal<bool>,
    alive: Rc<Cell<bool>>,
    reads: RequestScope,
    writes: RequestScope,
}
impl Ui {
    fn current(&self) -> Context {
        Context::current(self.auth, self.view, self.revision)
    }
    fn matches(&self, context: &Context, generation: u64) -> bool {
        self.alive.get()
            && context.usable()
            && *self.generation.peek() == generation
            && self.current() == *context
    }
    fn matches_write(&self, context: &Context, generation: u64) -> bool {
        self.alive.get()
            && context.usable()
            && *self.generation.peek() == generation
            && context.same_privacy(&self.current())
    }
    fn next_generation(&mut self) -> u64 {
        let generation = self.generation.peek().wrapping_add(1);
        self.generation.set(generation);
        generation
    }
    fn changed(&mut self) {
        let refresh = self.refresh.peek().wrapping_add(1);
        self.refresh.set(refresh);
    }
    fn actionable(&self, context: &Context) -> bool {
        self.alive.get()
            && context.usable()
            && self.current() == *context
            && !*self.loading.peek()
            && !*self.busy.peek()
    }
}

fn deadline(controller: Option<web_sys::AbortController>) -> (Timeout, Rc<Cell<bool>>) {
    let expired = Rc::new(Cell::new(false));
    let timed_out = expired.clone();
    let timer = Timeout::new(10000, move || {
        timed_out.set(true);
        if let Some(controller) = controller {
            controller.abort();
        }
    });
    (timer, expired)
}

async fn read_mix(mut ui: Ui, context: Context, generation: u64) {
    // A queued task can start after its effect has already been replaced. It
    // must not begin a scope that aborts the successor's current request.
    if !ui.matches(&context, generation) {
        return;
    }
    let signal = match ui.reads.begin() {
        Ok(signal) => signal,
        Err(error) => {
            if ui.matches(&context, generation) {
                ui.loading.set(false);
                ui.error.set(error);
            }
            return;
        }
    };
    let (timer, timed_out) = deadline(ui.reads.controller());
    let result = client::get::<Recommendations>("/api/recommendations?limit=8", &signal).await;
    drop(timer);
    if !ui.matches(&context, generation) || (signal.aborted() && !timed_out.get()) {
        return;
    }
    ui.loading.set(false);
    match result {
        Ok(data)
            if context.privacy_revision == Some(data.privacy_revision)
                && context.history_enabled == data.history_enabled =>
        {
            ui.data.set(Some(Snapshot { context, data }));
        }
        Ok(_) => {
            ui.data.set(None);
            ui.error.set("Your listening preferences changed on the server. Refresh privacy settings before loading personal mixes again.".into());
        }
        Err(error) => {
            ui.data.set(None);
            ui.error.set(error);
        }
    }
}

#[derive(Clone)]
enum Feedback {
    Hide(Track),
    Undo(Track),
    Clear,
}

fn feedback(mut ui: Ui, action: Feedback) {
    let context = ui.current();
    if !context.usable() || *ui.busy.peek() || *ui.exporting.peek() || *ui.loading.peek() {
        return;
    }
    // Invalidate the old read before sending a write. No actionable stale rows
    // survive an in-flight or failed feedback operation.
    ui.reads.cancel();
    let generation = ui.next_generation();
    ui.data.set(None);
    ui.loading.set(false);
    ui.error.set(String::new());
    ui.notice.set(String::new());
    ui.confirm.set(false);
    let signal = match ui.writes.begin() {
        Ok(signal) => signal,
        Err(error) => {
            ui.error.set(error);
            return;
        }
    };
    let controller = ui.writes.controller();
    ui.busy.set(true);
    spawn_local(async move {
        let (method, body) = match &action {
            Feedback::Hide(track) | Feedback::Undo(track) => (
                "PUT",
                serde_json::to_value(RecommendationFeedback {
                    track_id: track.id.clone(),
                    hidden: matches!(&action, Feedback::Hide(_)),
                })
                .ok(),
            ),
            Feedback::Clear => ("DELETE", None),
        };
        let (timer, timed_out) = deadline(controller);
        let result = client::api_abortable::<RecommendationFeedbackReceipt>(
            method,
            "/api/recommendations/feedback",
            body,
            &context.session,
            Some(&signal),
        )
        .await;
        drop(timer);
        if !ui.matches_write(&context, generation) || (signal.aborted() && !timed_out.get()) {
            return;
        }
        ui.busy.set(false);
        match result {
            Ok(_) => {
                match action {
                    Feedback::Hide(track) => {
                        ui.notice.set(format!(
                            "{} is hidden from personal mixes. Your music and playlists are unchanged.",
                            track.title
                        ));
                        ui.undo.set(Some(HiddenTrack {
                            session: context.session,
                            track,
                        }));
                    }
                    Feedback::Undo(track) => {
                        ui.notice.set(format!(
                            "{} is eligible for personal mixes again. Its place still depends on ranking.",
                            track.title
                        ));
                        ui.undo.set(None);
                    }
                    Feedback::Clear => {
                        ui.notice.set("All your hidden tracks are eligible for personal mixes again. Ranking can still choose other music.".into());
                        ui.undo.set(None);
                    }
                }
                ui.changed();
            }
            Err(error) => {
                // A listening flush during the write is a data change, not a
                // permission change. Keep the failed operation visible and the
                // rows cleared until an explicit retry/refresh.
                let current = ui.current();
                let refresh = *ui.refresh.peek();
                ui.context.set(Some((current, refresh)));
                ui.error.set(error);
            }
        }
    });
}

fn download_exclusions(value: &RecommendationExclusions) -> Result<(), String> {
    if value.format_version != 1 || value.track_ids.len() > 1000 {
        return Err("The server returned an unsupported recommendation export.".into());
    }
    let raw = serde_json::to_string_pretty(value)
        .map_err(|_| "Could not prepare your recommendation export.".to_string())?;
    let parts = js_sys::Array::new();
    parts.push(&raw.into());
    let blob = web_sys::Blob::new_with_str_sequence(&parts)
        .map_err(|_| "Could not prepare your recommendation export.".to_string())?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|_| "Could not open your recommendation export.".to_string())?;
    let element = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.create_element("a").ok())
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok());
    let result = if let Some(element) = element {
        let _ = element.set_attribute("href", &url);
        let _ = element.set_attribute("download", "vela-private-recommendation-exclusions.json");
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

fn export(mut ui: Ui) {
    let context = ui.current();
    if !ui.actionable(&context) || *ui.exporting.peek() {
        return;
    }
    let generation = *ui.generation.peek();
    let signal = match ui.writes.begin() {
        Ok(signal) => signal,
        Err(error) => {
            ui.error.set(error);
            return;
        }
    };
    let controller = ui.writes.controller();
    ui.exporting.set(true);
    ui.error.set(String::new());
    spawn_local(async move {
        let (timer, timed_out) = deadline(controller);
        let result =
            client::get::<RecommendationExclusions>("/api/recommendations/feedback", &signal).await;
        drop(timer);
        if !ui.matches_write(&context, generation) || (signal.aborted() && !timed_out.get()) {
            return;
        }
        ui.exporting.set(false);
        match result.and_then(|value| download_exclusions(&value)) {
            Ok(()) => ui.notice.set("Your private hidden-track IDs were exported. This file contains no audio, credentials or listening history.".into()),
            Err(error) => ui.error.set(error),
        }
    });
}

fn shelf(
    ui: Ui,
    context: Context,
    items: Vec<RadioTrack>,
    rotation: bool,
    actions: Actions,
) -> Element {
    let title = if rotation {
        "Your rotation"
    } else {
        "Explore your library"
    };
    let origin = if rotation { "rotation" } else { "exploration" };
    let empty_icon = if rotation { "history" } else { "music" };
    let tracks: Vec<Track> = items.iter().map(|item| item.track.clone()).collect();
    let mix = tracks.clone();
    let play_ui = ui.clone();
    let play_context = context.clone();
    rsx! { section { class: "personal-mix-shelf", "aria-label": title,
        div { class: "section-toolbar",
            div { h3 { "{title}" } p { if rotation { "Music you have meaningfully listened to, ranked by your private listening activity." } else { "Fresh paths through music you actually own, with a reason for every suggestion." } } }
            if !tracks.is_empty() { button { class: "button secondary small", "aria-label": "Play {origin} mix", onclick: move |_| { if play_ui.actionable(&play_context) { actions.start.call((mix.clone(), 0, PlaybackContext::new(ContextKind::Home, title))); } }, Icon { name: "play" } "Play mix" } }
        }
        if items.is_empty() { div { class: "personal-mix-empty",
            Icon { name: empty_icon }
            div { strong { if rotation { "Give your favorites another turn" } else { "A little more music opens new paths" } }
                p { if rotation { if context.history_enabled { "Play at least half a track or 30 seconds with account and device collection enabled. Only qualified listens within your retention period appear here." } else { "Your listening history is off. Enable private collection for your account and device if you want a rotation based on your actual listening." } } else { "Add music or restore hidden tracks to open up more possibilities. Discovery keeps favorites and previously qualified listens as anchors rather than relabeling them as new suggestions." } }
            }
            button { class: "text-button", onclick: move |_| { if rotation { actions.navigate.call("settings".into()); } else { actions.add.call(()); } }, if rotation { "Manage listening privacy" } else { "Add music" } }
        } } else { div { class: "personal-mix-grid", for (at, item) in items.into_iter().enumerate() { {
            let queue = tracks.clone();
            let track = item.track.clone();
            let next = track.clone();
            let last = track.clone();
            let radio = track.clone();
            let hidden = track.clone();
            let play_ui = ui.clone();
            let next_ui = ui.clone();
            let last_ui = ui.clone();
            let radio_ui = ui.clone();
            let hide_ui = ui.clone();
            let play_context = context.clone();
            let next_context = context.clone();
            let last_context = context.clone();
            let radio_context = context.clone();
            let hide_context = context.clone();
            rsx! { article { class: "personal-mix-card", key: "{item.track.id}", "data-track-id": "{item.track.id}",
                button { class: "personal-mix-play", "aria-label": format!("{} from {origin}",actions.play_label(&item.track.id,&item.track.title)), onclick: move |_| { if play_ui.actionable(&play_context) { actions.play.call((queue.clone(), at, PlaybackContext::new(ContextKind::Home, title))); } },
                    div { class: "listening-cover", crate::Art { track } span { class: "cover-play", if actions.is_playing(&item.track.id){Icon {name:"pause"}}else{Icon {name:"play"}} } }
                    div { class: "personal-mix-description", strong { "{item.track.title}" } small { "{item.track.artist}" } }
                }
                p { class: "personal-mix-reason", "{item.reason}" }
                div { class: "personal-mix-actions",
                    button { class: "icon-button", "aria-label": "Play {item.track.title} next from personal mixes", title: "Play next", onclick: move |_| { if next_ui.actionable(&next_context) { actions.queue.call((next.clone(), true)); } }, Icon { name: "next-add" } }
                    button { class: "icon-button", "aria-label": "Add {item.track.title} to queue from personal mixes", title: "Add to queue", onclick: move |_| { if last_ui.actionable(&last_context) { actions.queue.call((last.clone(), false)); } }, Icon { name: "plus" } }
                    button { class: "icon-button", "aria-label": "Start radio from {item.track.title} in personal mixes", title: "Start track radio", onclick: move |_| { if radio_ui.actionable(&radio_context) { actions.radio.call(radio.clone()); } }, Icon { name: "radio" } }
                    button { class: "text-button personal-mix-hide", "aria-label": "Hide {item.track.title} from personal mixes", title: "Exclude this track from personal mixes only", onclick: move |_| { if hide_ui.actionable(&hide_context) { feedback(hide_ui.clone(), Feedback::Hide(hidden.clone())); } }, "Hide" }
                }
            } }
        } } } }
    } }
}

#[component]
pub fn PersonalMixes(
    auth: Signal<Option<AuthResponse>>,
    revision: Signal<u64>,
    view: Signal<ListeningView>,
    actions: Actions,
    library_state: Memo<Option<bool>>,
) -> Element {
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let reads = use_hook(RequestScope::default);
    let writes = use_hook(RequestScope::default);
    let cleanup_alive = alive.clone();
    let cleanup_reads = reads.clone();
    let cleanup_writes = writes.clone();
    use_drop(move || {
        cleanup_alive.set(false);
        cleanup_reads.cancel();
        cleanup_writes.cancel();
    });
    let ui = Ui {
        auth,
        view,
        revision,
        generation: use_signal(|| 0),
        refresh: use_signal(|| 0),
        context: use_signal(|| None),
        data: use_signal(|| None),
        loading: use_signal(|| false),
        busy: use_signal(|| false),
        exporting: use_signal(|| false),
        error: use_signal(String::new),
        notice: use_signal(String::new),
        undo: use_signal(|| None),
        confirm: use_signal(|| false),
        alive,
        reads,
        writes,
    };
    let effect = ui.clone();
    use_effect(move || {
        let mut ui = effect.clone();
        // Subscribe to the shared signals, then compare the relevant values.
        // Collector clock/outbox publication alone must not restart HTTP work.
        let _auth = auth.read();
        let _view = view.read();
        let _revision = revision.read();
        // An empty library has nothing to recommend. Skip the read entirely
        // instead of asking for candidates that cannot exist. An unknown
        // library state is not an empty one, so the read still runs.
        if library_state() == Some(true) {
            ui.reads.cancel();
            ui.writes.cancel();
            ui.context.set(None);
            ui.data.set(None);
            ui.loading.set(false);
            ui.busy.set(false);
            ui.exporting.set(false);
            return;
        }
        let refresh = *ui.refresh.read();
        let pending_write = *ui.busy.read() || *ui.exporting.read();
        let context = ui.current();
        let request_context = (context.clone(), refresh);
        if ui.context.peek().as_ref() == Some(&request_context) {
            return;
        }
        if pending_write
            && ui
                .context
                .peek()
                .as_ref()
                .is_some_and(|(old, _)| old.same_privacy(&context))
        {
            // Listening event acknowledgements and catalog edits may request a
            // fresh mix, but must not cancel feedback whose commit is pending.
            // Its receipt triggers a read in the latest data context.
            return;
        }
        let account_changed = ui
            .context
            .peek()
            .as_ref()
            .is_none_or(|(old, _)| old.session != context.session);
        ui.reads.cancel();
        ui.writes.cancel();
        let generation = ui.next_generation();
        ui.context.set(Some(request_context));
        ui.data.set(None);
        ui.loading.set(false);
        ui.busy.set(false);
        ui.exporting.set(false);
        ui.error.set(String::new());
        ui.confirm.set(false);
        if account_changed {
            ui.undo.set(None);
            ui.notice.set(String::new());
        }
        if !context.usable() {
            return;
        }
        ui.loading.set(true);
        spawn_local(read_mix(ui, context, generation));
    });
    let _auth = auth.read();
    let _view = view.read();
    let _revision = revision.read();
    // An empty library has no recommendations to gather. The section stays
    // unmounted so it cannot promise mixes, anchors or discoveries that do not
    // exist, and so it issues no recommendation read at all. Only a confirmed
    // empty state hides the section: an unknown count (a failed or stale
    // `/api/stats`) must not silently drop real content.
    if library_state() == Some(true) {
        return rsx! {};
    }
    let current = ui.current();
    // Render independently checks privacy, before an effect can run, so a held
    // mix cannot remain clickable for the frame in which deletion starts.
    let snapshot = ui.data.read().clone().filter(|snapshot| {
        current.usable() && snapshot.context == current && !*ui.loading.peek() && !*ui.busy.peek()
    });
    let loading = *ui.loading.read();
    let busy = *ui.busy.read();
    let exporting = *ui.exporting.read();
    let blocked = !current.usable() || loading || busy || exporting;
    let error = ui.error.read().clone();
    let notice = ui.notice.read().clone();
    let undo = ui
        .undo
        .read()
        .clone()
        .filter(|hidden| hidden.session == current.session);
    let confirm = *ui.confirm.read();
    let refresh_ui = ui.clone();
    let undo_ui = ui.clone();
    let restore_ui = ui.clone();
    let clear_ui = ui.clone();
    let cancel_ui = ui.clone();
    let export_ui = ui.clone();
    let has_hidden = snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.data.hidden_count > 0);
    rsx! { section { class: "home-section personal-mixes", "aria-label": "Personal recommendations",
        div { class: "section-toolbar personal-mixes-heading",
            div { span { class: "eyebrow", "A LITTLE FURTHER INTO YOUR MUSIC" } h2 { "Your next listening chapter" } p { "Fresh discoveries and familiar favorites, gathered from your music." } }
            button { class: "button secondary small", disabled: blocked, onclick: move |_| { let mut ui = refresh_ui.clone(); if ui.current().usable() { ui.changed(); } }, "Refresh personal mixes" }
        }
        if current.blocked { p { role: "status", "Waiting for your current listening privacy settings…" } }
        if loading { div { class: "loading-skeletons personal-mix-loading", role: "status", "aria-label": "Loading personal mixes", for _ in 0..4 { div { class: "skeleton-card", "aria-hidden": "true" } } } }
        if busy { p { role: "status", "Saving your recommendation preferences…" } }
        if !error.is_empty() { div { class: "alert error", role: "alert", "{error}" } }
        if !notice.is_empty() { div { class: "alert success personal-mix-notice", role: "status", span { "{notice}" } } }
        if let Some(hidden) = undo { button { class: "text-button personal-mix-undo", disabled: blocked, "aria-label": "Undo last recommendation hide", onclick: move |_| feedback(undo_ui.clone(), Feedback::Undo(hidden.track.clone())), Icon { name: "back" } "Undo last hide" } }
        if let Some(snapshot) = snapshot {
            div { class: "personal-mix-basis",
                span { class: "personal-mix-tag", if snapshot.data.history_enabled { "Private listening, personal suggestions" } else { "Listening history is off" } }
                p { if snapshot.data.anchor_count == 0 { "An invitation to explore your own collection. Favorite a few tracks to guide your next discoveries." } else if snapshot.data.history_enabled { "Guided by your favorites and meaningful listens, with room to find something different." } else { "Your favorites guide exploration. Listening history has no influence while collection is off." } }
                details { class: "personal-mix-explanation", summary { "How these mixes are made" }
                    p { "{snapshot.data.candidate_count} eligible candidates checked, from at most the 500 newest owned tracks · {snapshot.data.anchor_count} favorite/listening anchors · {snapshot.data.hidden_count} hidden" }
                    p { if snapshot.data.anchor_count == 0 { "Without favorite or qualified listening anchors, exploration offers a diverse selection of your available music." } else if snapshot.data.history_enabled { "Exploration excludes its favorites and listening anchors, plus qualified listens within your retention period. Your rotation brings listened music back." } else { "Explicit favorites and catalog signals guide suggestions. Retained listening events are excluded while account collection is off." } }
                    if snapshot.data.uses_audio_features { small { "Some returned suggestions use measured audio similarity. Your audio stays on your server." } }
                    else { small { "These sets use available catalog tags and diverse exploration. Track radio offers optional local audio analysis." } }
                    small { "Generated on demand. These are not scheduled daily or weekly mixes. Hiding a track changes only these personal recommendations." }
                }
            }
            {shelf(ui.clone(), current.clone(), snapshot.data.discovery, false, actions)}
            {shelf(ui.clone(), current.clone(), snapshot.data.rotation, true, actions)}
        }
        div { class: "personal-mix-preferences",
            button { class: "text-button", onclick: move |_| actions.navigate.call("settings".into()), "Manage listening privacy" }
            button { class: "text-button", disabled: blocked, onclick: move |_| export(export_ui.clone()), if exporting { "Preparing hidden-track export…" } else { "Export hidden recommendations" } }
            if has_hidden { button { class: "text-button", disabled: blocked, onclick: move |_| { let mut ui = restore_ui.clone(); ui.confirm.set(true); }, "Restore hidden recommendations…" } }
        }
        if confirm && current.usable() { div { class: "personal-mix-confirm", role: "group", "aria-label": "Confirm restoring hidden recommendations",
            p { "Make every hidden track eligible for your personal mixes again? This changes only your recommendation exclusions, not your listening history or music." }
            button { class: "button secondary", disabled: blocked, onclick: move |_| feedback(clear_ui.clone(), Feedback::Clear), "Restore all hidden recommendations" }
            button { class: "text-button", disabled: busy, onclick: move |_| { let mut ui = cancel_ui.clone(); ui.confirm.set(false); }, "Keep hidden tracks" }
        } }
    } }
}

#[cfg(test)]
mod tests {
    use super::Context;

    fn current() -> Context {
        Context {
            session: "current-session".into(),
            privacy_generation: 3,
            privacy_revision: Some(4),
            history_enabled: true,
            listening_revision: 8,
            catalog_revision: 10,
            blocked: false,
        }
    }

    #[test]
    fn data_updates_invalidate_reads_without_invalidating_feedback_permissions() {
        let original = current();
        let mut listening_sync = original.clone();
        listening_sync.listening_revision += 1;
        assert!(original != listening_sync);
        assert!(original.same_privacy(&listening_sync));
        let mut catalog_edit = original.clone();
        catalog_edit.catalog_revision += 1;
        assert!(original != catalog_edit);
        assert!(original.same_privacy(&catalog_edit));
    }

    #[test]
    fn session_privacy_and_pending_consent_changes_invalidate_feedback_permissions() {
        let original = current();
        let mut contexts = vec![original.clone(); 6];
        contexts[0].session = "another-session".into();
        contexts[1].privacy_generation += 1;
        contexts[2].privacy_revision = Some(5);
        contexts[3].privacy_revision = None;
        contexts[4].history_enabled = false;
        contexts[5].blocked = true;
        for changed in contexts {
            assert!(!original.same_privacy(&changed));
        }
        assert!(
            !Context {
                blocked: true,
                ..original.clone()
            }
            .usable()
        );
        assert!(
            !Context {
                session: String::new(),
                ..original
            }
            .usable()
        );
    }
}
