//! Bounded, private task history backed by the real import and scanner services.
use crate::{api, bytes, filters::append_query, mutate, spawn_authorized, token};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use std::{cell::Cell, rc::Rc};
use vela_domain::{
    AuthResponse, BackgroundJob, JobKind, JobPage, JobProgress, JobStateFilter, JobStatus, ScanJob,
};

#[derive(Clone, Default, PartialEq, Eq)]
struct Selection {
    kind: Option<JobKind>,
    state: JobStateFilter,
    cursor: Option<String>,
}

#[derive(Clone)]
struct Confirmed {
    selection: Selection,
    page: JobPage,
}

#[derive(Clone, Copy)]
struct Tasks {
    auth: Signal<Option<AuthResponse>>,
    selection: Signal<Selection>,
    confirmed: Signal<Option<Confirmed>>,
    loading: Signal<bool>,
    busy: Signal<bool>,
    error: Signal<String>,
    action_error: Signal<String>,
    notice: Signal<String>,
    generation: Signal<u64>,
    poll_generation: Signal<u64>,
    poll_tick: Signal<u64>,
    lifecycle: Signal<Rc<Cell<bool>>>,
}

fn invalidate(mut ui: Tasks) -> u64 {
    let generation = ui.generation.peek().wrapping_add(1);
    ui.generation.set(generation);
    generation
}

fn active(status: JobStatus) -> bool {
    matches!(
        status,
        JobStatus::Queued
            | JobStatus::Downloading
            | JobStatus::Processing
            | JobStatus::Scanning
            | JobStatus::Cancelling
    )
}

fn kind_name(kind: JobKind) -> &'static str {
    match kind {
        JobKind::Import => "Import",
        JobKind::Scan => "Folder scan",
    }
}

fn status_name(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Queued => "Queued",
        JobStatus::Downloading => "Downloading",
        JobStatus::Processing => "Processing audio",
        JobStatus::Scanning => "Scanning folder",
        JobStatus::Cancelling => "Finishing cancellation",
        JobStatus::Completed => "Completed",
        JobStatus::Failed => "Failed",
        JobStatus::Cancelled => "Cancelled",
    }
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn created_label(timestamp: i64) -> String {
    // Server timestamps are seconds; browser locale formatting stays local to
    // the client and never changes the ordering/cursor supplied by the server.
    js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(timestamp as f64 * 1000.0))
        .to_locale_string("default", &js_sys::Object::new())
        .as_string()
        .unwrap_or_else(|| "Time unavailable".into())
}

async fn refresh(mut ui: Tasks, onchanged: EventHandler<()>, polling: bool) {
    let alive = ui.lifecycle.peek().clone();
    if !alive.get() {
        return;
    }
    let session = token(ui.auth);
    if session.is_empty() {
        return;
    }
    let selection = ui.selection.peek().clone();
    let generation = invalidate(ui);
    ui.loading.set(true);
    let mut url = "/api/jobs?limit=50".to_string();
    if let Some(kind) = selection.kind {
        append_query(
            &mut url,
            "kind",
            match kind {
                JobKind::Import => "import",
                JobKind::Scan => "scan",
            },
        );
    }
    append_query(
        &mut url,
        "state",
        match selection.state {
            JobStateFilter::All => "all",
            JobStateFilter::Active => "active",
            JobStateFilter::Failed => "failed",
            JobStateFilter::Completed => "completed",
            JobStateFilter::Cancelled => "cancelled",
        },
    );
    if let Some(cursor) = &selection.cursor {
        append_query(&mut url, "cursor", cursor);
    }
    let result = api::<JobPage>("GET", &url, None, "").await;
    if !alive.get()
        || token(ui.auth) != session
        || *ui.generation.peek() != generation
        || *ui.selection.peek() != selection
    {
        return;
    }
    match result {
        Ok(page) => {
            let previous = ui.confirmed.peek().clone();
            let terminal = previous.as_ref().is_some_and(|previous| {
                (previous.page.has_active && !page.has_active)
                    || page.items.iter().any(|job| {
                        !active(job.status)
                            && previous.page.items.iter().any(|old| {
                                old.id == job.id && old.kind == job.kind && active(old.status)
                            })
                    })
            });
            // A filtered page can hide a job finishing while another remains
            // active. Refresh catalog aggregates during those bounded polls too:
            // scanner cancellation/failure can retain already-published files.
            let refresh_catalog =
                terminal || (polling && previous.as_ref().is_some_and(|p| p.page.has_active));
            ui.confirmed.set(Some(Confirmed { selection, page }));
            ui.error.set(String::new());
            if terminal {
                ui.notice.set(
                    "A background task finished. Music already added is available in your library."
                        .into(),
                );
            }
            if refresh_catalog {
                onchanged.call(());
            }
        }
        Err(message) => ui.error.set(message),
    }
    ui.loading.set(false);
    let tick = ui.poll_tick.peek().wrapping_add(1);
    ui.poll_tick.set(tick);
}

fn change_selection(mut ui: Tasks, selection: Selection, onchanged: EventHandler<()>) {
    invalidate(ui);
    ui.selection.set(selection);
    ui.busy.set(false);
    ui.action_error.set(String::new());
    ui.error.set(String::new());
    ui.notice.set(String::new());
    spawn_authorized(ui.auth, refresh(ui, onchanged, false));
}

#[derive(Clone)]
enum Action {
    Cancel(BackgroundJob),
    Retry(BackgroundJob),
    Rescan(BackgroundJob),
}

fn act(action: Action, mut ui: Tasks, onchanged: EventHandler<()>) {
    if *ui.busy.peek() || *ui.loading.peek() {
        return;
    }
    let alive = ui.lifecycle.peek().clone();
    let session = token(ui.auth);
    if !alive.get() || session.is_empty() {
        return;
    }
    let selection = ui.selection.peek().clone();
    let generation = invalidate(ui);
    ui.busy.set(true);
    ui.action_error.set(String::new());
    ui.notice.set(String::new());
    spawn_authorized(ui.auth, async move {
        let (result, success) = match action {
            Action::Cancel(job) => {
                let path = match job.kind {
                    JobKind::Import => format!("/api/imports/{}/cancel", job.id),
                    JobKind::Scan => format!("/api/scans/{}/cancel", job.id),
                };
                (
                    mutate("POST", &path, None, &session).await,
                    "Cancellation requested. Files already added stay in your library.",
                )
            }
            Action::Retry(job) => (
                mutate(
                    "POST",
                    &format!("/api/imports/{}/retry", job.id),
                    None,
                    &session,
                )
                .await,
                "Import queued again.",
            ),
            Action::Rescan(job) => {
                let result = match job.source_id {
                    Some(source) => api::<ScanJob>(
                        "POST",
                        &format!("/api/scan-sources/{source}/scan"),
                        None,
                        &session,
                    )
                    .await
                    .map(|_| ()),
                    None => Err("This music folder is no longer available. Refresh tasks.".into()),
                };
                (
                    result,
                    "A new folder scan was queued. Previous scan history is kept.",
                )
            }
        };
        if !alive.get()
            || token(ui.auth) != session
            || *ui.generation.peek() != generation
            || *ui.selection.peek() != selection
        {
            return;
        }
        ui.busy.set(false);
        match result {
            Ok(()) => {
                ui.notice.set(success.into());
                let mut latest = selection;
                latest.cursor = None;
                ui.selection.set(latest);
                // A small job may finish between the mutation and read. Refresh
                // parent aggregates without pretending the worker has finished.
                onchanged.call(());
            }
            Err(message) => ui.action_error.set(message),
        }
        refresh(ui, onchanged, false).await;
    });
}

#[component]
pub fn JobCenter(
    auth: Signal<Option<AuthResponse>>,
    onchanged: EventHandler<()>,
    onopen: EventHandler<String>,
) -> Element {
    let lifecycle = use_hook(|| Rc::new(Cell::new(true)));
    let cleanup = lifecycle.clone();
    use_drop(move || cleanup.set(false));
    let mut ui = Tasks {
        auth,
        selection: use_signal(Selection::default),
        confirmed: use_signal(|| None),
        loading: use_signal(|| false),
        busy: use_signal(|| false),
        error: use_signal(String::new),
        action_error: use_signal(String::new),
        notice: use_signal(String::new),
        generation: use_signal(|| 0),
        poll_generation: use_signal(|| 0),
        poll_tick: use_signal(|| 0),
        lifecycle: use_signal(|| lifecycle),
    };
    use_effect(move || {
        let session = auth.read().as_ref().map(|s| s.csrf_token.clone());
        invalidate(ui);
        ui.selection.set(Selection::default());
        ui.confirmed.set(None);
        ui.loading.set(false);
        ui.busy.set(false);
        ui.error.set(String::new());
        ui.action_error.set(String::new());
        ui.notice.set(String::new());
        if session.is_some() {
            spawn_authorized(auth, refresh(ui, onchanged, false));
        }
    });
    use_effect(move || {
        let session = auth.read().as_ref().map(|s| s.csrf_token.clone());
        let has_active = ui
            .confirmed
            .read()
            .as_ref()
            .is_some_and(|s| s.page.has_active);
        let waiting = *ui.loading.read() || *ui.busy.read();
        let _tick = *ui.poll_tick.read();
        let generation = ui.poll_generation.peek().wrapping_add(1);
        ui.poll_generation.set(generation);
        if session.is_none() || !has_active || waiting {
            return;
        }
        let alive = ui.lifecycle.peek().clone();
        spawn_authorized(auth, async move {
            TimeoutFuture::new(2_000).await;
            if !alive.get()
                || *ui.poll_generation.peek() != generation
                || *ui.loading.peek()
                || *ui.busy.peek()
            {
                return;
            }
            if web_sys::window()
                .and_then(|w| w.document())
                .is_some_and(|d| d.hidden())
            {
                // Avoid network activity in a hidden tab. Keep only a scoped
                // timer so an active task resumes refreshing on returning.
                let tick = ui.poll_tick.peek().wrapping_add(1);
                ui.poll_tick.set(tick);
                return;
            }
            refresh(ui, onchanged, true).await;
        });
    });
    let Some(administrator) = auth.read().as_ref().map(|s| s.user.is_admin) else {
        return rsx! {};
    };
    let selection = ui.selection.read().clone();
    let confirmed = ui.confirmed.read().clone();
    let waiting = (ui.loading)() || (ui.busy)();
    let stale =
        confirmed.as_ref().is_some_and(|s| s.selection != selection) || !(ui.error)().is_empty();
    let kind_value = match selection.kind {
        None => "",
        Some(JobKind::Import) => "import",
        Some(JobKind::Scan) => "scan",
    };
    let state_value = match selection.state {
        JobStateFilter::All => "all",
        JobStateFilter::Active => "active",
        JobStateFilter::Failed => "failed",
        JobStateFilter::Completed => "completed",
        JobStateFilter::Cancelled => "cancelled",
    };
    rsx! {
        section { class: "job-center", "aria-labelledby": "jobs-heading",
            div { class: "section-toolbar",
                div { h2 { id: "jobs-heading", "Background tasks" } p { "Your imports and music folder scans, in one private place." } }
                button { class: "button secondary small", disabled: waiting, onclick: move |_| { ui.action_error.set(String::new()); spawn_authorized(auth, refresh(ui, onchanged, false)); }, "Refresh tasks" }
            }
            div { class: "job-center-controls panel",
                div { label { r#for: "job-type", "Task type" }
                    select { id: "job-type", value: kind_value, onchange: move |event| {
                        let mut next = ui.selection.peek().clone();
                        next.kind = match event.value().as_str() { "import" => Some(JobKind::Import), "scan" if administrator => Some(JobKind::Scan), _ => None };
                        next.cursor = None; change_selection(ui, next, onchanged);
                    }, option { value: "", "All types" } option { value: "import", "Imports" } if administrator { option { value: "scan", "Folder scans" } } }
                }
                div { label { r#for: "job-state", "Task state" }
                    select { id: "job-state", value: state_value, onchange: move |event| {
                        let mut next = ui.selection.peek().clone();
                        next.state = match event.value().as_str() { "active" => JobStateFilter::Active, "failed" => JobStateFilter::Failed, "completed" => JobStateFilter::Completed, "cancelled" => JobStateFilter::Cancelled, _ => JobStateFilter::All };
                        next.cursor = None; change_selection(ui, next, onchanged);
                    }, option { value: "all", "All tasks" } option { value: "active", "Active" } option { value: "failed", "Failed" } option { value: "completed", "Completed" } option { value: "cancelled", "Cancelled" } }
                }
                p { "Uploads stay in their upload dialog while this tab sends the files. Only server-managed tasks appear here." }
            }
            if !(ui.error)().is_empty() { div { class: "alert error", role: "alert", "{ui.error}" } }
            if !(ui.action_error)().is_empty() { div { class: "alert error", role: "alert", "{ui.action_error}" } }
            if !(ui.notice)().is_empty() { div { class: "alert success", role: "status", "{ui.notice}" } }
            if (ui.loading)() { p { class: "job-center-update", role: "status", "Updating tasks…" } }
            if stale { p { class: "job-center-stale", role: "status", "Showing the last confirmed tasks. They may differ from the selected filters or current server state. Refresh to try again." } }
            if let Some(snapshot) = confirmed {
                if snapshot.page.items.is_empty() {
                    div { class: "empty-state job-center-empty",
                        span { class: "empty-icon", "↻" }
                        h3 { if snapshot.selection.kind.is_none() && snapshot.selection.state == JobStateFilter::All && snapshot.selection.cursor.is_none() { "No background tasks yet" } else { "No tasks match these filters" } }
                        p { if snapshot.page.has_active { "Other tasks are still running. This view keeps checking for updates." } else { "Add music or scan a configured music folder to start a task." } }
                    }
                }
                div { class: "background-jobs", "aria-busy": (ui.loading)(), for job in snapshot.page.items.iter().cloned() {
                    { let cancellation = job.clone(); let retry = job.clone(); let rescan = job.clone();
                        let id = short_id(&job.id); let created = created_label(job.created_at);
                        rsx! { article { class: "background-job panel", key: "{kind_name(job.kind)}-{job.id}", "data-job-id": "{job.id}", "data-job-kind": if job.kind == JobKind::Import { "import" } else { "scan" },
                            div { class: "background-job-heading",
                                div { span { class: "eyebrow", "{kind_name(job.kind)} · {id}" } h3 { "{job.title}" } small { "Created {created}" } }
                                span { class: "background-job-status", "{status_name(job.status)}" }
                            }
                            match job.progress {
                                JobProgress::Import { downloaded_bytes, total_bytes } => rsx! {
                                    div { class: "background-job-transfer", span { "{bytes(downloaded_bytes)}" if let Some(total) = total_bytes.filter(|total| *total > 0) { " of {bytes(total)}" } }
                                        if active(job.status) {
                                            if job.status == JobStatus::Downloading && let Some(total) = total_bytes.filter(|total| *total > 0) { progress { max: total.to_string(), value: downloaded_bytes.clamp(0, total).to_string(), "aria-label": "Import download progress" } }
                                            else { progress { "aria-label": "Import in progress" } }
                                        }
                                    }
                                },
                                JobProgress::Scan { visited, imported, duplicates, unchanged, skipped, failed_files } => rsx! {
                                    if active(job.status) { progress { class: "background-job-progress", "aria-label": "Folder scan in progress" } }
                                    dl { class: "background-job-counters",
                                        div { dt { "Visited" } dd { "{visited}" } } div { dt { "Added" } dd { "{imported}" } }
                                        div { dt { "Duplicates" } dd { "{duplicates}" } } div { dt { "Unchanged" } dd { "{unchanged}" } }
                                        div { dt { "Skipped" } dd { "{skipped}" } } div { dt { "Failed files" } dd { "{failed_files}" } }
                                    }
                                    if failed_files > 0 { p { class: "job-center-warning", "Some files could not be added. Music imported successfully is kept in your library." } }
                                },
                            }
                            if job.status == JobStatus::Processing { p { class: "job-center-hint", "Audio is being validated and added. This stage finishes safely and cannot be cancelled." } }
                            if job.status == JobStatus::Cancelling { p { class: "job-center-hint", "Cancellation is pending. The current file may finish safely before this scan stops." } }
                            if let Some(message) = job.error { p { class: "job-error", "{message}" } }
                            if job.can_cancel || job.can_retry || job.can_rescan || job.track_id.is_some() {
                                div { class: "background-job-actions",
                                    if job.can_cancel { button { class: "button secondary small", disabled: waiting || stale, "aria-label": "Cancel task", onclick: move |_| act(Action::Cancel(cancellation.clone()), ui, onchanged), "Cancel task" } }
                                    if job.can_retry { button { class: "button secondary small", disabled: waiting || stale, "aria-label": "Retry import", onclick: move |_| act(Action::Retry(retry.clone()), ui, onchanged), "Retry import" } }
                                    if job.can_rescan && administrator { button { class: "button secondary small", disabled: waiting || stale, "aria-label": "Scan again", onclick: move |_| act(Action::Rescan(rescan.clone()), ui, onchanged), "Scan again" } }
                                    if job.track_id.is_some() { button { class: "button secondary small", onclick: move |_| onopen.call("library".into()), "Open library" } }
                                }
                            }
                        } }
                    }
                } }
                div { class: "pagination job-center-pagination",
                    button { class: "button secondary small", disabled: waiting || selection.cursor.is_none(), onclick: move |_| { let mut next = ui.selection.peek().clone(); next.cursor = None; change_selection(ui, next, onchanged); }, "Latest tasks" }
                    span { "Up to 50 tasks per page" }
                    button { class: "button secondary small", disabled: waiting || stale || snapshot.page.next_cursor.is_none(), onclick: move |_| {
                        let cursor = ui.confirmed.peek().as_ref().and_then(|s| s.page.next_cursor.clone());
                        if cursor.is_some() { let mut next = ui.selection.peek().clone(); next.cursor = cursor; change_selection(ui, next, onchanged); }
                    }, "Older tasks" }
                }
                p { class: "job-center-hint", if snapshot.page.has_active { "Updates every 2 seconds while tasks are active and this page is visible." } else { "All known tasks are idle. Refresh to see tasks started elsewhere." } }
            }
            div { class: "job-center-links", button { class: "button secondary small", onclick: move |_| onopen.call("imports".into()), "Add music" } if administrator { button { class: "button secondary small", onclick: move |_| onopen.call("settings".into()), "Music folder settings" } } button { class: "button secondary small", onclick: move |_| onopen.call("library".into()), "Open library" } }
        }
    }
}
