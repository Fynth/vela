//! Typed, explicit library filters and bounded owner-specific suggestions.
use crate::{api, spawn_authorized, token};
use dioxus::prelude::*;
use vela_domain::{AuthResponse, CatalogFacets, TrackFilters, TrackSort};

#[derive(Clone, Default, PartialEq, Eq)]
pub struct FilterDraft {
    genre: String,
    codec: String,
    year_from: String,
    year_to: String,
    min_bitrate: String,
    min_sample_rate: String,
    min_bit_depth: String,
    sort: TrackSort,
}

impl FilterDraft {
    fn parse(&self) -> Result<TrackFilters, String> {
        let genre = text(&self.genre, 1024, "Genre")?;
        let codec = text(&self.codec, 64, "Format")?;
        let year_from = number(&self.year_from, 9999, "Year from")?;
        let year_to = number(&self.year_to, 9999, "Year to")?;
        if year_from.zip(year_to).is_some_and(|(from, to)| from > to) {
            return Err("Year from must be no later than Year to.".into());
        }
        Ok(TrackFilters {
            genre,
            codec,
            year_from,
            year_to,
            min_bitrate: number(&self.min_bitrate, 1_000_000, "Minimum bitrate")?,
            min_sample_rate: number(&self.min_sample_rate, 1_536_000, "Minimum sample rate")?,
            min_bit_depth: number(&self.min_bit_depth, 64, "Minimum bit depth")?,
            sort: self.sort,
        })
    }
}

fn text(value: &str, max: usize, name: &str) -> Result<Option<String>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > max || value.chars().any(char::is_control) {
        return Err(format!(
            "{name} is too long or contains unsupported characters."
        ));
    }
    Ok(Some(value.into()))
}

fn number(value: &str, max: i64, name: &str) -> Result<Option<i64>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<i64>()
        .ok()
        .filter(|value| (1..=max).contains(value))
        .map(Some)
        .ok_or_else(|| format!("{name} must be a whole number between 1 and {max}."))
}

pub fn append_query(url: &mut String, key: &str, value: &str) {
    url.push(if url.contains('?') { '&' } else { '?' });
    url.push_str(key);
    url.push('=');
    url.push_str(
        &js_sys::encode_uri_component(value)
            .as_string()
            .unwrap_or_default(),
    );
}

pub fn append_filters(url: &mut String, filters: &TrackFilters) {
    if let Some(value) = &filters.genre {
        append_query(url, "genre", value);
    }
    if let Some(value) = &filters.codec {
        append_query(url, "codec", value);
    }
    for (key, value) in [
        ("year_from", filters.year_from),
        ("year_to", filters.year_to),
        ("min_bitrate", filters.min_bitrate),
        ("min_sample_rate", filters.min_sample_rate),
        ("min_bit_depth", filters.min_bit_depth),
    ] {
        if let Some(value) = value {
            append_query(url, key, &value.to_string());
        }
    }
    append_query(
        url,
        "sort",
        match filters.sort {
            TrackSort::Library => "library",
            TrackSort::Title => "title",
            TrackSort::Newest => "newest",
        },
    );
}

pub fn has_criteria(filters: &TrackFilters) -> bool {
    filters.genre.is_some()
        || filters.codec.is_some()
        || filters.year_from.is_some()
        || filters.year_to.is_some()
        || filters.min_bitrate.is_some()
        || filters.min_sample_rate.is_some()
        || filters.min_bit_depth.is_some()
}

fn summary(filters: &TrackFilters) -> String {
    let mut parts = Vec::new();
    if let Some(genre) = &filters.genre {
        parts.push(format!("Genre: {genre}"));
    }
    if let Some(codec) = &filters.codec {
        parts.push(format!("Format: {codec}"));
    }
    match (filters.year_from, filters.year_to) {
        (Some(from), Some(to)) => parts.push(format!("Years: {from}–{to}")),
        (Some(from), None) => parts.push(format!("Year: {from} onward")),
        (None, Some(to)) => parts.push(format!("Year: through {to}")),
        _ => {}
    }
    if let Some(value) = filters.min_bitrate {
        parts.push(format!("≥{value} kbps"));
    }
    if let Some(value) = filters.min_sample_rate {
        parts.push(format!("≥{value} Hz"));
    }
    if let Some(value) = filters.min_bit_depth {
        parts.push(format!("≥{value} bit"));
    }
    parts.push(
        match filters.sort {
            TrackSort::Library => "Library order",
            TrackSort::Title => "Title A–Z",
            TrackSort::Newest => "Recently added",
        }
        .into(),
    );
    parts.join(" · ")
}

#[component]
pub fn LibraryFilters(
    auth: Signal<Option<AuthResponse>>,
    mut open: Signal<bool>,
    mut draft: Signal<FilterDraft>,
    applied: TrackFilters,
    error: Signal<String>,
    loading: Signal<bool>,
    revision: Signal<u64>,
    onapply: EventHandler<TrackFilters>,
    onreset: EventHandler<()>,
    oninvalid: EventHandler<String>,
) -> Element {
    let mut facets = use_signal(|| None::<(String, CatalogFacets)>);
    let mut facets_loading = use_signal(|| false);
    let mut facets_error = use_signal(String::new);
    let mut generation = use_signal(|| 0_u64);
    let mut refresh = use_signal(|| 0_u64);
    use_effect(move || {
        let session = auth.read().as_ref().map(|auth| auth.csrf_token.clone());
        let visible = open();
        let expected_revision = revision();
        let expected_refresh = refresh();
        let request = generation.peek().wrapping_add(1);
        generation.set(request);
        facets_loading.set(false);
        if !visible || session.is_none() {
            // Private suggestions are not retained by a hidden form or account.
            facets.set(None);
            facets_error.set(String::new());
            return;
        }
        facets_loading.set(true);
        facets_error.set(String::new());
        spawn_authorized(auth, async move {
            let result = api::<CatalogFacets>("GET", "/api/catalog/filters", None, "").await;
            if *generation.peek() != request
                || !*open.peek()
                || *revision.peek() != expected_revision
                || *refresh.peek() != expected_refresh
                || session.as_deref() != Some(token(auth).as_str())
            {
                return;
            }
            match result {
                Ok(value) => facets.set(Some((token(auth), value))),
                Err(message) => facets_error.set(message),
            }
            facets_loading.set(false);
        });
    });
    let values = draft.read().clone();
    // Render only the current session's suggestions, even before the session
    // effect has run after a change of account.
    let current_session = auth.read().as_ref().map(|auth| auth.csrf_token.clone());
    let available = facets
        .read()
        .as_ref()
        .filter(|(session, _)| current_session.as_ref() == Some(session))
        .map(|(_, facets)| facets.clone());
    let applied_summary = summary(&applied);
    let changed = draft.read().parse().map_or(true, |draft| draft != applied);
    let active = applied != TrackFilters::default();
    // DR06: the default library order is a fact about the collection, not a
    // control the reader has to act on. The marker says exactly that — the
    // applied filters are the default ones — so the phone composition can keep
    // the sentence in the DOM as a status line without spending a visible row
    // of the first screen on it. An active filter or sort has no marker and
    // is still shown.
    let default_order = !active;
    let sort_value = match values.sort {
        TrackSort::Library => "library",
        TrackSort::Title => "title",
        TrackSort::Newest => "newest",
    };
    rsx! {
        section { class: "library-filters", "aria-label": "Library filters",
            div { class: "filter-toolbar",
                button { class: "button secondary small", r#type: "button",
                    "aria-expanded": open(), "aria-controls": "library-filter-form",
                    onclick: move |_| open.set(!open()),
                    if open() { "Hide filters" } else { "Filter and sort" }
                }
                p { class: "filter-summary", role: "status",
                    "data-default-order": default_order.then_some("true"),
                    "{applied_summary}" }
                if active && !open() {
                    button { class: "button secondary small", onclick: move |_| onreset.call(()), "Reset filters" }
                }
            }
            if !error().is_empty() { div { class: "alert error", role: "alert", "{error}" } }
            if open() {
                form { id: "library-filter-form", class: "filter-form", novalidate: true,
                    onsubmit: move |event| {
                        event.prevent_default();
                        match draft.peek().parse() {
                            Ok(filters) => onapply.call(filters),
                            Err(message) => oninvalid.call(message),
                        }
                    },
                    div { class: "filter-suggestions-heading",
                        p { "Choose details from your library, or type a complete genre tag." }
                        button { class: "button secondary small", r#type: "button", disabled: facets_loading(),
                            onclick: move |_| { let next = refresh.peek().wrapping_add(1); refresh.set(next); },
                            if facets_error().is_empty() { "Refresh suggestions" } else { "Retry suggestions" }
                        }
                    }
                    if facets_loading() { p { class: "filter-hint", role: "status", "Loading your library’s genres and formats…" } }
                    if !facets_error().is_empty() { div { class: "alert error", role: "alert", "Could not load filter suggestions. {facets_error}" } }
                    if available.as_ref().is_some_and(|facets| facets.genres_truncated) {
                        p { class: "filter-hint", "Showing the first 100 genre tags. You can type any complete genre tag." }
                    }
                    if available.as_ref().is_some_and(|facets| facets.codecs_truncated) {
                        p { class: "filter-hint", "Showing the first 32 formats. Other formats remain available without a format filter." }
                    }
                    div { class: "filter-fields",
                        div { class: "filter-field",
                            label { r#for: "filter-genre", "Genre" }
                            input { id: "filter-genre", list: "library-genre-options", value: "{values.genre}", maxlength: 1024,
                                placeholder: "Any genre", autocomplete: "off", oninput: move |event| draft.write().genre = event.value() }
                            datalist { id: "library-genre-options",
                                if let Some(facets) = &available { for genre in &facets.genres { option { value: "{genre}" } } }
                            }
                        }
                        div { class: "filter-field",
                            label { r#for: "filter-format", "Format" }
                            select { id: "filter-format", value: "{values.codec}", onchange: move |event| draft.write().codec = event.value(),
                                option { value: "", "Any format" }
                                // Retain an earlier real selection if a refresh is truncated
                                // or a metadata change removes its last visible track.
                                if !values.codec.is_empty() && !available.as_ref().is_some_and(|facets| facets.codecs.contains(&values.codec)) {
                                    option { value: "{values.codec}", "{values.codec} (selected)" }
                                }
                                if let Some(facets) = &available { for codec in &facets.codecs { option { value: "{codec}", "{codec}" } } }
                            }
                        }
                        div { class: "filter-field",
                            label { r#for: "filter-year-from", "Year from" }
                            input { id: "filter-year-from", r#type: "number", min: "1", max: "9999", step: "1", value: "{values.year_from}",
                                placeholder: "Any year", oninput: move |event| draft.write().year_from = event.value() }
                        }
                        div { class: "filter-field",
                            label { r#for: "filter-year-to", "Year to" }
                            input { id: "filter-year-to", r#type: "number", min: "1", max: "9999", step: "1", value: "{values.year_to}",
                                placeholder: "Any year", oninput: move |event| draft.write().year_to = event.value() }
                        }
                        div { class: "filter-field",
                            label { r#for: "filter-min-bitrate", "Minimum bitrate (kbps)" }
                            input { id: "filter-min-bitrate", r#type: "number", min: "1", max: "1000000", step: "1", value: "{values.min_bitrate}",
                                placeholder: "No minimum", oninput: move |event| draft.write().min_bitrate = event.value() }
                        }
                        div { class: "filter-field",
                            label { r#for: "filter-min-sample-rate", "Minimum sample rate (Hz)" }
                            input { id: "filter-min-sample-rate", r#type: "number", min: "1", max: "1536000", step: "1", value: "{values.min_sample_rate}",
                                placeholder: "No minimum", oninput: move |event| draft.write().min_sample_rate = event.value() }
                        }
                        div { class: "filter-field",
                            label { r#for: "filter-min-bit-depth", "Minimum bit depth" }
                            input { id: "filter-min-bit-depth", r#type: "number", min: "1", max: "64", step: "1", value: "{values.min_bit_depth}",
                                placeholder: "No minimum", oninput: move |event| draft.write().min_bit_depth = event.value() }
                        }
                        div { class: "filter-field",
                            label { r#for: "filter-sort", "Sort" }
                            select { id: "filter-sort", value: sort_value, onchange: move |event| {
                                draft.write().sort = match event.value().as_str() { "title" => TrackSort::Title, "newest" => TrackSort::Newest, _ => TrackSort::Library };
                            },
                                option { value: "library", "Library order" }
                                option { value: "title", "Title A–Z" }
                                option { value: "newest", "Recently added" }
                            }
                        }
                    }
                    p { class: "filter-hint", "Genre matches a complete tag. Year and minimums exclude tracks with unknown values. Formats and audio measurements do not establish whether a recording is lossless." }
                    div { class: "filter-actions",
                        button { class: "button primary small", r#type: "submit", "Apply filters" }
                        button { class: "button secondary small", r#type: "button", onclick: move |_| onreset.call(()), "Reset filters" }
                        if loading() { small { role: "status", "Loading tracks… You can apply another filter while waiting." } }
                        else if changed { small { "Draft changes apply when you choose Apply filters." } }
                    }
                }
            }
        }
    }
}
