//! Explicit private analysis and playable, bounded local radio.
use crate::{
    client::{self, RequestScope},
    experience::Actions,
    icons::Icon,
    session::{ContextKind, PlaybackContext},
    token,
};
use dioxus::prelude::*;
use gloo_timers::{callback::Timeout, future::TimeoutFuture};
use std::{cell::Cell, collections::HashSet, rc::Rc};
use vela_domain::*;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Default)]
struct Snapshot {
    seed_id: String,
    radio: Option<TrackRadio>,
    analysis: Option<TrackAnalysis>,
    summary: Option<AnalysisLibraryStatus>,
}
impl Snapshot {
    fn active(&self) -> bool {
        self.summary
            .as_ref()
            .is_some_and(|s| s.queued > 0 || s.running > 0)
            || self.analysis.as_ref().is_some_and(|a| {
                matches!(a.status, AnalysisStatus::Queued | AnalysisStatus::Running)
            })
    }
}
#[derive(Clone)]
struct Ui {
    auth: Signal<Option<AuthResponse>>,
    seed: Signal<Option<Track>>,
    generation: Signal<u64>,
    refresh: Signal<u64>,
    data: Signal<Snapshot>,
    loading: Signal<bool>,
    busy: Signal<bool>,
    error: Signal<String>,
    notice: Signal<String>,
    alive: Rc<Cell<bool>>,
    reads: RequestScope,
    writes: RequestScope,
}
impl Ui {
    fn matches(&self, session: &str, id: &str, generation: u64) -> bool {
        self.alive.get()
            && token(self.auth) == session
            && *self.generation.peek() == generation
            && self.seed.peek().as_ref().is_some_and(|seed| seed.id == id)
    }
    fn changed(&mut self) {
        let next = self.refresh.peek().wrapping_add(1);
        self.refresh.set(next);
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

async fn refresh(mut ui: Ui, session: String, id: String, generation: u64) {
    let mut failures = 0_u32;
    loop {
        if !ui.matches(&session, &id, generation) {
            return;
        }
        if web_sys::window()
            .and_then(|window| window.document())
            .is_some_and(|document| document.hidden())
        {
            TimeoutFuture::new(2000).await;
            continue;
        }
        let Ok(signal) = ui.reads.begin() else {
            ui.loading.set(false);
            ui.error
                .set("Your browser could not start this request. Refresh to retry.".into());
            return;
        };
        let (timer, timed_out) = deadline(ui.reads.controller());
        let summary = client::get::<AnalysisLibraryStatus>("/api/audio-analysis", &signal).await;
        if !ui.matches(&session, &id, generation) {
            return;
        }
        let analysis =
            client::get::<TrackAnalysis>(&format!("/api/tracks/{id}/analysis"), &signal).await;
        if !ui.matches(&session, &id, generation) {
            return;
        }
        let radio =
            client::get::<TrackRadio>(&format!("/api/tracks/{id}/radio?limit=20"), &signal).await;
        drop(timer);
        if !ui.matches(&session, &id, generation) || (signal.aborted() && !timed_out.get()) {
            return;
        }
        ui.loading.set(false);
        let mut snapshot = ui.data.peek().clone();
        let mut errors = Vec::new();
        match summary {
            Ok(value) => snapshot.summary = Some(value),
            Err(error) => errors.push(format!("Analysis status: {error}")),
        }
        match analysis {
            Ok(value) if value.track_id == id => snapshot.analysis = Some(value),
            Ok(_) => errors.push("The server returned another track's analysis.".into()),
            Err(error) => errors.push(format!("Seed analysis: {error}")),
        }
        match radio {
            Ok(value) if value.seed.id == id => snapshot.radio = Some(value),
            Ok(_) => errors.push("The server returned another track's radio.".into()),
            Err(error) => errors.push(format!("Radio: {error}")),
        }
        let active = snapshot.active();
        ui.data.set(snapshot);
        ui.error.set(errors.join(" "));
        if !errors.is_empty() {
            failures += 1;
            if !active || failures >= 3 {
                return;
            }
        } else {
            failures = 0;
        }
        if !active {
            return;
        }
        // One serial request group, active jobs only, bounded backoff on failure.
        TimeoutFuture::new(2000 * (1 << failures)).await;
    }
}

fn enqueue(mut ui: Ui, bulk: bool) {
    if *ui.busy.peek() {
        return;
    }
    let Some(seed) = ui.seed.peek().clone() else {
        return;
    };
    let session = token(ui.auth);
    if session.is_empty() {
        return;
    }
    let generation = *ui.generation.peek();
    let Ok(signal) = ui.writes.begin() else {
        return;
    };
    let controller = ui.writes.controller();
    ui.busy.set(true);
    ui.error.set(String::new());
    ui.notice.set(String::new());
    spawn_local(async move {
        let (timer, timed_out) = deadline(controller);
        let result = async {
            let mut ids = vec![seed.id.clone()];
            if bulk {
                let page = client::get::<TrackPage>("/api/tracks?limit=50", &signal).await?;
                if !ui.matches(&session, &seed.id, generation) {
                    return Err("Selection changed.".into());
                }
                let mut seen = HashSet::from([seed.id.clone()]);
                for track in page.items {
                    if ids.len() == 50 {
                        break;
                    }
                    if seen.insert(track.id.clone()) {
                        ids.push(track.id);
                    }
                }
            }
            client::api_abortable::<AnalysisReceipt>(
                "POST",
                "/api/audio-analysis",
                serde_json::to_value(EnqueueAnalysis { track_ids: ids }).ok(),
                &session,
                Some(&signal),
            )
            .await
        }
        .await;
        drop(timer);
        if !ui.matches(&session, &seed.id, generation) || (signal.aborted() && !timed_out.get()) {
            return;
        }
        ui.busy.set(false);
        match result {
            Ok(receipt) => {
                ui.notice.set(if receipt.queued == 0 {
                    "Selected tracks are already analyzed or queued. Nothing was duplicated.".into()
                } else {
                    format!(
                        "{} audio files queued for local analysis. Playback is unaffected.",
                        receipt.queued
                    )
                });
                ui.changed();
            }
            Err(error) => ui.error.set(error),
        }
    });
}

fn status_label(status: AnalysisStatus) -> &'static str {
    match status {
        AnalysisStatus::NotAnalyzed => "Not analyzed",
        AnalysisStatus::Queued => "Queued",
        AnalysisStatus::Running => "Analyzing audio",
        AnalysisStatus::Complete => "Analysis complete",
        AnalysisStatus::Failed => "Analysis failed",
    }
}
fn measurement(value: Option<f64>, unit: &str) -> String {
    value.filter(|value| value.is_finite()).map_or_else(
        || "Unavailable (including digital silence)".into(),
        |value| format!("{value:.2}{unit}"),
    )
}

fn chroma_label(chroma: &[f64; 12]) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let mut indices = (0..12).collect::<Vec<_>>();
    indices.sort_by(|left, right| chroma[*right].total_cmp(&chroma[*left]));
    indices
        .into_iter()
        .take(3)
        .map(|index| format!("{} {:.0}%", NAMES[index], chroma[index] * 100.0))
        .collect::<Vec<_>>()
        .join(" · ")
}

#[component]
fn Measurements(features: AudioFeatures) -> Element {
    let seconds = features.analyzed_ms as f64 / 1000.0;
    rsx! { details { class: "radio-measurements",
        summary { "Measured audio details" }
        p { "{seconds:.2} seconds analyzed · {features.sample_rate} Hz · {features.channels} channels" }
        p { if features.complete { "The decoder reached the end of this file." } else { "These measurements describe a bounded prefix, not the entire recording." } }
        dl {
            dt { "RMS amplitude" } dd { "{features.rms:.6}" }
            dt { "RMS level (dBFS)" } dd { {measurement(features.rms_dbfs, " dBFS")} }
            dt { "Peak amplitude" } dd { "{features.peak:.6}" }
            dt { "Peak level (dBFS)" } dd { {measurement(features.peak_dbfs, " dBFS")} }
            dt { "Spectral centroid" } dd { {measurement(features.spectral_centroid_hz, " Hz")} }
            dt { "Spectral bandwidth" } dd { {measurement(features.spectral_bandwidth_hz, " Hz")} }
            dt { "85% spectral rolloff" } dd { {measurement(features.spectral_rolloff_hz, " Hz")} }
            dt { "Spectral flatness" } dd { {measurement(features.spectral_flatness, "")} }
            dt { "Zero-crossing fraction" } dd { "{features.zero_crossing_rate:.5}" }
            dt { "FFT windows" } dd { "{features.spectral_windows}" }
            dt { "Estimated tempo" } dd { match features.tempo_bpm { Some(bpm) => format!("{bpm:.1} BPM · {:.0}% onset regularity", features.tempo_confidence.unwrap_or(0.0) * 100.0), None => "Not established".to_owned() } }
            dt { "Chroma pitch classes" } dd { match features.chroma { Some(chroma) => chroma_label(&chroma), None => "Not established".to_owned() } }
            dt { "Estimated key" } dd { match features.key { Some(key) => format!("{key} · {:.0}% profile margin", features.key_confidence.unwrap_or(0.0) * 100.0), None => "Not established".to_owned() } }
            dt { "Extractor" } dd { "{features.extractor_version}" }
        }
        small { "RMS and peak describe digital samples. They are not LUFS, ReplayGain or perceived loudness. Tempo is an onset-regularity estimate, not a calibrated beat annotation. Chroma is nearest-semitone spectral pitch-class energy. Estimated key applies Krumhansl major/minor profiles to chroma; it is a heuristic profile fit, not calibrated key detection. This extractor does not infer genre or mood." }
    } }
}

#[component]
pub fn Radio(
    auth: Signal<Option<AuthResponse>>,
    seed: Signal<Option<Track>>,
    actions: Actions,
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
        seed,
        generation: use_signal(|| 0),
        refresh: use_signal(|| 0),
        data: use_signal(Snapshot::default),
        loading: use_signal(|| false),
        busy: use_signal(|| false),
        error: use_signal(String::new),
        notice: use_signal(String::new),
        alive,
        reads,
        writes,
    };
    let effect = ui.clone();
    use_effect(move || {
        let mut ui = effect.clone();
        let session = token(auth);
        let _auth = auth.read();
        let selected = seed.read().clone();
        let _refresh = *ui.refresh.read();
        ui.reads.cancel();
        ui.writes.cancel();
        let generation = ui.generation.peek().wrapping_add(1);
        ui.generation.set(generation);
        ui.busy.set(false);
        ui.error.set(String::new());
        let Some(selected) = selected else {
            ui.data.set(Snapshot::default());
            ui.loading.set(false);
            return;
        };
        if session.is_empty() {
            ui.data.set(Snapshot::default());
            ui.loading.set(false);
            return;
        }
        if ui.data.peek().seed_id != selected.id {
            ui.data.set(Snapshot {
                seed_id: selected.id.clone(),
                ..Snapshot::default()
            });
            ui.notice.set(String::new());
        }
        ui.loading.set(true);
        spawn_local(refresh(ui, session, selected.id, generation));
    });
    let selected = seed.read().clone();
    let snapshot = ui.data.read().clone();
    let snapshot = selected
        .as_ref()
        .filter(|seed| seed.id == snapshot.seed_id)
        .map(|_| snapshot);
    let loading = *ui.loading.read();
    let busy = *ui.busy.read();
    let error = ui.error.read().clone();
    let notice = ui.notice.read().clone();
    let refresh_ui = ui.clone();
    let seed_ui = ui.clone();
    let batch_ui = ui.clone();
    rsx! { section { class: "radio-page", "aria-label": "Track radio",
        if let (Some(selected), Some(snapshot)) = (selected, snapshot) {
            div { class: "radio-hero",
                crate::Art { track: selected.clone() }
                div { span { class: "eyebrow", "A NEW DIRECTION, FROM YOUR MUSIC" } h2 { "{selected.title} radio" } p { if !selected.artist.trim().is_empty() { "Starting with {selected.artist} · " } "Only music in your own accessible library" }
                    if let Some(radio) = snapshot.radio.as_ref() { { let mut tracks = vec![radio.seed.clone()]; tracks.extend(radio.items.iter().map(|item| item.track.clone())); let context = PlaybackContext::new(ContextKind::Radio, &format!("{} radio", radio.seed.title)); rsx! { button { class: "button primary", onclick: move |_| actions.play.call((tracks.clone(), 0, context.clone())), Icon { name: "play" } "Play radio" } } } }
                }
            }
            div { class: "section-toolbar", div { h3 { "A mix with a reason" } p { "Bounded local discovery, with an explanation for each track." } } button { class: "button secondary small", disabled: loading || busy, onclick: move |_| { let mut ui = refresh_ui.clone(); ui.changed(); }, "Refresh radio" } }
            if loading { p { role: "status", "Loading track radio and analysis status…" } }
            if !error.is_empty() { div { class: "alert error", role: "alert", "{error}" } }
            if !notice.is_empty() { div { class: "alert success", role: "status", "{notice}" } }
            if let Some(radio) = snapshot.radio {
                p { class: "radio-coverage", "{radio.candidate_count} candidates checked from at most the 500 newest other owned tracks. " if radio.uses_audio_features { "Some matches use measured audio descriptors." } else { "This mix currently uses catalog metadata and a diverse library fallback. Analyze tracks explicitly to add measured audio similarity." } }
                if radio.items.is_empty() { div { class: "empty-state", h3 { "A little more music opens more paths" } p { "Your seed is playable. Add another track to your library to build a longer radio queue." } button { class: "button secondary", onclick: move |_| actions.add.call(()), "Add music to my library" } } }
                { let tracks: Vec<Track> = radio.items.iter().map(|item| item.track.clone()).collect(); rsx! { div { class: "radio-track-list", for (at, item) in radio.items.into_iter().enumerate() { { let context = tracks.clone(); let row = item.track.clone(); let next = item.track.clone(); let last = item.track.clone(); let seed_title = radio.seed.title.clone(); rsx! {
                    article { class: "radio-track", key: "{item.track.id}",
                        button { class: "radio-track-play", "aria-label": "Play {item.track.title} from radio", onclick: move |_| actions.play.call((context.clone(), at, PlaybackContext::new(ContextKind::Radio, &format!("{seed_title} radio")))), crate::Art { track: row } div { class: "radio-track-description", strong { "{item.track.title}" } small { "{item.track.artist}" } } Icon { name: "play" } }
                        p { class: "radio-reason", "{item.reason}" }
                        div { class: "radio-track-actions", button { class: "icon-button", "aria-label": "Play {item.track.title} next from radio", title: "Play next", onclick: move |_| actions.queue.call((next.clone(), true)), Icon { name: "next-add" } } button { class: "icon-button", "aria-label": "Add {item.track.title} to queue from radio", title: "Add to queue", onclick: move |_| actions.queue.call((last.clone(), false)), Icon { name: "plus" } } }
                    }
                } } } } } }
            }
            section { class: "panel radio-analysis", "aria-labelledby": "radio-analysis-title",
                h2 { id: "radio-analysis-title", "Make the connection in the sound" }
                p { "Optional analysis decodes a bounded audio prefix locally and stores versioned measurements. It never changes your originals, runs on the playback request or sends audio to a cloud service." }
                if let Some(summary) = snapshot.summary { dl { class: "analysis-counts", dt { "Owned tracks" } dd { "{summary.total}" } dt { "Analyzed" } dd { "{summary.complete}" } dt { "Not analyzed" } dd { "{summary.not_analyzed}" } dt { "Queued" } dd { "{summary.queued}" } dt { "Running" } dd { "{summary.running}" } dt { "Failed" } dd { "{summary.failed}" } } }
                if let Some(analysis) = snapshot.analysis.as_ref() { p { class: "analysis-seed-status", role: "status", "Seed: {status_label(analysis.status)}" } if let Some(error) = analysis.error.as_ref() { p { role: "alert", "{error}" } } }
                div { class: "radio-analysis-actions",
                    button { class: "button secondary", disabled: busy || snapshot.analysis.as_ref().is_some_and(|a| matches!(a.status, AnalysisStatus::Queued | AnalysisStatus::Running | AnalysisStatus::Complete)), onclick: move |_| enqueue(seed_ui.clone(), false), if busy { "Queuing analysis…" } else { "Analyze seed" } }
                    button { class: "button secondary", disabled: busy, onclick: move |_| enqueue(batch_ui.clone(), true), "Analyze up to 50 library tracks" }
                }
                small { "The batch contains this seed plus tracks from one real 50-item library page. Already completed or active assets are reused. Status refreshes only while this page has active analysis; leaving the page does not cancel durable work." }
                if let Some(features) = snapshot.analysis.as_ref().and_then(|analysis| analysis.features.clone()) { Measurements { features } }
            }
        } else { div { class: "empty-state", h2 { "Let one track lead to another" } p { "Choose Track radio beside a real track in Home, Search or your Library." } button { class: "button primary", onclick: move |_| actions.navigate.call("library".into()), "Open my library" } } }
    } }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chroma_label_orders_pitch_classes_by_energy() {
        let mut chroma = [0.0_f64; 12];
        chroma[9] = 0.8;
        chroma[1] = 0.15;
        chroma[4] = 0.05;
        assert_eq!(chroma_label(&chroma), "A 80% · C# 15% · E 5%");
    }

    #[test]
    fn measurements_never_label_missing_or_nonfinite_values_as_real() {
        assert!(measurement(None, " Hz").contains("Unavailable"));
        assert!(measurement(Some(f64::NAN), " Hz").contains("Unavailable"));
        assert_eq!(measurement(Some(1000.125), " Hz"), "1000.12 Hz");
        assert_eq!(measurement(Some(-20.0), " dBFS"), "-20.00 dBFS");
    }
}
