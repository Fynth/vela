//! Administrator UI for explicitly configured, read-only music folders.
use crate::{api, mutate, spawn_authorized, token};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use serde_json::json;
use vela_domain::{AuthResponse, CreateScanSource, ScanJob, ScanSource, StorageRoot};

#[derive(Clone, Copy)]
struct ScannerUi {
    auth: Signal<Option<AuthResponse>>,
    roots: Signal<Vec<StorageRoot>>,
    sources: Signal<Vec<ScanSource>>,
    jobs: Signal<Vec<ScanJob>>,
    root: Signal<String>,
    name: Signal<String>,
    loading: Signal<bool>,
    busy: Signal<bool>,
    error: Signal<String>,
    notice: Signal<String>,
    generation: Signal<u64>,
    poll_generation: Signal<u64>,
    poll_tick: Signal<u64>,
}

fn active(status: &str) -> bool {
    matches!(status, "queued" | "running" | "processing" | "cancelling")
}

fn status_label(status: &str) -> &'static str {
    match status {
        "queued" => "Waiting to scan",
        "running" => "Scanning folder",
        "processing" => "Adding music",
        "cancelling" => "Finishing cancellation",
        "completed" => "Scan complete",
        "cancelled" => "Scan cancelled",
        "failed" => "Scan failed",
        _ => "Updating scan",
    }
}

fn invalidate(mut ui: ScannerUi) -> u64 {
    let generation = ui.generation.peek().wrapping_add(1);
    ui.generation.set(generation);
    generation
}

fn update_jobs(mut ui: ScannerUi, jobs: Vec<ScanJob>, onchanged: EventHandler<()>) {
    // A cancelled/failed pass may already have added files. Refresh the library
    // when any pass terminates, rather than assuming publication is all-or-none.
    let changed = jobs.iter().any(|job| {
        !active(&job.status)
            && ui
                .jobs
                .peek()
                .iter()
                .any(|old| old.id == job.id && active(&old.status))
    });
    ui.jobs.set(jobs);
    if changed {
        ui.notice.set(
            "Your scan has finished. Music added during the scan is available in your library."
                .into(),
        );
        onchanged.call(());
    }
}

async fn refresh(mut ui: ScannerUi, config: bool, onchanged: EventHandler<()>) {
    let generation = invalidate(ui);
    if config {
        ui.loading.set(true);
        ui.error.set(String::new());
        let roots = api::<Vec<StorageRoot>>("GET", "/api/storage/roots", None, "").await;
        let sources = api::<Vec<ScanSource>>("GET", "/api/scan-sources", None, "").await;
        if *ui.generation.peek() != generation {
            return;
        }
        match (roots, sources) {
            (Ok(roots), Ok(sources)) => {
                if !roots.iter().any(|root| {
                    root.id == *ui.root.peek()
                        && root.available
                        && !sources.iter().any(|source| source.root_id == root.id)
                }) {
                    ui.root.set(
                        roots
                            .iter()
                            .find(|root| {
                                root.available
                                    && !sources.iter().any(|source| source.root_id == root.id)
                            })
                            .map(|root| root.id.clone())
                            .unwrap_or_default(),
                    );
                }
                ui.roots.set(roots);
                ui.sources.set(sources);
            }
            (Err(message), _) | (_, Err(message)) => {
                ui.error.set(message);
                ui.loading.set(false);
                return;
            }
        }
    }
    let jobs = api::<Vec<ScanJob>>("GET", "/api/scans", None, "").await;
    if *ui.generation.peek() != generation {
        return;
    }
    match jobs {
        Ok(jobs) => {
            update_jobs(ui, jobs, onchanged);
            if !config {
                ui.error.set(String::new());
            }
        }
        Err(message) => ui.error.set(message),
    }
    if config {
        ui.loading.set(false);
    } else {
        // A transient request failure still retries while the last known job
        // state is active. No polling task remains when all jobs terminate.
        let next = ui.poll_tick.peek().wrapping_add(1);
        ui.poll_tick.set(next);
    }
}

fn add_source(mut ui: ScannerUi, onchanged: EventHandler<()>) {
    if *ui.busy.peek() || *ui.loading.peek() {
        return;
    }
    let name = ui.name.peek().trim().to_string();
    let root_id = ui.root.peek().clone();
    if name.is_empty() || name.len() > 120 {
        ui.error
            .set("Enter a source name, or shorten it if it is too long.".into());
        return;
    }
    if !ui.roots.peek().iter().any(|root| {
        root.id == root_id
            && root.available
            && !ui
                .sources
                .peek()
                .iter()
                .any(|source| source.root_id == root.id)
    }) {
        ui.error.set("Choose an available folder.".into());
        return;
    }
    invalidate(ui);
    ui.busy.set(true);
    ui.error.set(String::new());
    ui.notice.set(String::new());
    spawn_authorized(ui.auth, async move {
        let result = api::<ScanSource>(
            "POST",
            "/api/scan-sources",
            Some(json!(CreateScanSource { name, root_id })),
            &token(ui.auth),
        )
        .await;
        match result {
            Ok(source) => {
                ui.name.set(String::new());
                ui.notice.set(format!(
                    "{} is ready. Choose Scan to add its music.",
                    source.name
                ));
                refresh(ui, true, onchanged).await;
            }
            Err(message) => ui.error.set(message),
        }
        ui.busy.set(false);
    });
}

#[derive(Clone)]
enum Action {
    Scan(ScanSource),
    Remove(ScanSource),
    Cancel(String),
}

fn act(action: Action, mut ui: ScannerUi, onchanged: EventHandler<()>) {
    if *ui.busy.peek() || *ui.loading.peek() {
        return;
    }
    invalidate(ui);
    ui.busy.set(true);
    ui.error.set(String::new());
    ui.notice.set(String::new());
    spawn_authorized(ui.auth, async move {
        let result = match action {
            Action::Scan(source) => {
                match api::<ScanJob>(
                    "POST",
                    &format!("/api/scan-sources/{}/scan", source.id),
                    None,
                    &token(ui.auth),
                )
                .await
                {
                    Ok(job) => {
                        ui.jobs.write().insert(0, job);
                        ui.notice
                            .set(format!("{} was queued for scanning.", source.name));
                        Ok(())
                    }
                    Err(message) => Err(message),
                }
            }
            Action::Remove(source) => {
                let result = mutate(
                    "DELETE",
                    &format!("/api/scan-sources/{}", source.id),
                    None,
                    &token(ui.auth),
                )
                .await;
                if result.is_ok() {
                    ui.notice.set(format!(
                        "{} was removed from scanning. Your library music was kept.",
                        source.name
                    ));
                }
                result
            }
            Action::Cancel(id) => {
                let result = mutate(
                    "POST",
                    &format!("/api/scans/{id}/cancel"),
                    None,
                    &token(ui.auth),
                )
                .await;
                if result.is_ok() {
                    ui.notice.set(
                        "Cancellation requested. Music already added stays in your library.".into(),
                    );
                }
                result
            }
        };
        match result {
            Ok(()) => refresh(ui, true, onchanged).await,
            Err(message) => ui.error.set(message),
        }
        ui.busy.set(false);
    });
}

#[component]
pub fn ScannerPanel(auth: Signal<Option<AuthResponse>>, onchanged: EventHandler<()>) -> Element {
    let mut ui = ScannerUi {
        auth,
        roots: use_signal(Vec::new),
        sources: use_signal(Vec::new),
        jobs: use_signal(Vec::new),
        root: use_signal(String::new),
        name: use_signal(String::new),
        loading: use_signal(|| true),
        busy: use_signal(|| false),
        error: use_signal(String::new),
        notice: use_signal(String::new),
        generation: use_signal(|| 0),
        poll_generation: use_signal(|| 0),
        poll_tick: use_signal(|| 0),
    };
    use_effect(move || {
        let session = auth.read().clone();
        invalidate(ui);
        ui.roots.set(Vec::new());
        ui.sources.set(Vec::new());
        ui.jobs.set(Vec::new());
        ui.root.set(String::new());
        ui.name.set(String::new());
        ui.error.set(String::new());
        ui.notice.set(String::new());
        ui.busy.set(false);
        ui.loading.set(false);
        if session.is_some_and(|session| session.user.is_admin) {
            ui.loading.set(true);
            spawn_authorized(auth, refresh(ui, true, onchanged));
        }
    });
    use_effect(move || {
        let administrator = auth
            .read()
            .as_ref()
            .is_some_and(|session| session.user.is_admin);
        let scanning = ui.jobs.read().iter().any(|job| active(&job.status));
        let loading = *ui.loading.read();
        let busy = *ui.busy.read();
        let _tick = *ui.poll_tick.read();
        let generation = ui.poll_generation.peek().wrapping_add(1);
        ui.poll_generation.set(generation);
        if administrator && scanning && !loading && !busy {
            // The component scope owns this future. Leaving Settings drops it;
            // session and generation checks also discard obsolete responses.
            spawn_authorized(auth, async move {
                TimeoutFuture::new(2_000).await;
                if *ui.poll_generation.peek() == generation
                    && !*ui.loading.peek()
                    && !*ui.busy.peek()
                {
                    refresh(ui, false, onchanged).await;
                }
            });
        }
    });
    if !auth
        .read()
        .as_ref()
        .is_some_and(|session| session.user.is_admin)
    {
        return rsx! {};
    }
    let roots = ui.roots.read().clone();
    let sources = ui.sources.read().clone();
    let jobs = ui.jobs.read().clone();
    let unavailable = roots.iter().any(|root| !root.available);
    let can_add = roots
        .iter()
        .any(|root| root.available && !sources.iter().any(|source| source.root_id == root.id));
    let disabled = (ui.loading)() || (ui.busy)();
    rsx! {
        section { class: "panel scanner-panel", "aria-labelledby": "scanner-heading",
            div { class: "section-toolbar",
                div { span { class: "eyebrow", "ADD YOUR EXISTING MUSIC" } h2 { id: "scanner-heading", "Music folders" } }
                button { class: "button secondary small", disabled, onclick: move |_| {
                    spawn_authorized(auth, refresh(ui, true, onchanged));
                }, "Refresh folders" }
            }
            p { "Scan a folder configured on this server. Supported audio is copied into your private library; original files stay untouched. Rescanning keeps your catalog edits, and missing files never remove music from your library." }
            if !(ui.error)().is_empty() { div { class: "alert error", role: "alert", "{ui.error}" } }
            if !(ui.notice)().is_empty() { div { class: "alert success", role: "status", "{ui.notice}" } }
            if (ui.loading)() && roots.is_empty() { p { role: "status", "Loading music folders…" } }
            if !(ui.loading)() && roots.is_empty() && (ui.error)().is_empty() {
                div { class: "scanner-empty", h3 { "No folders configured" } p { "Folder scanning is disabled on this server. Upload files or import a supported URL from Add music." } }
            } else if !roots.is_empty() {
                form { class: "scanner-source-form", onsubmit: move |event| { event.prevent_default(); add_source(ui, onchanged); },
                    div { label { r#for: "scan-root", "Folder" }
                        select { id: "scan-root", value: "{ui.root}", required: true, disabled: disabled || !can_add,
                            onchange: move |event| ui.root.set(event.value()),
                            option { value: "", disabled: true, "Choose a folder" }
                            for root in roots.iter() { option { value: "{root.id}", disabled: !root.available || sources.iter().any(|source| source.root_id == root.id),
                                "{root.name}" if !root.available { " · Unavailable" } else if sources.iter().any(|source| source.root_id == root.id) { " · Already added" }
                            } }
                        }
                    }
                    div { label { r#for: "scan-source-name", "Source name" }
                        input { id: "scan-source-name", value: "{ui.name}", required: true, maxlength: 120, disabled: disabled || !can_add, placeholder: "My music collection", oninput: move |event| ui.name.set(event.value()) }
                    }
                    button { class: "button primary", r#type: "submit", disabled: disabled || !can_add || (ui.name)().trim().is_empty(), "Add source" }
                }
                if unavailable { p { class: "scanner-hint", "An unavailable folder needs the administrator to check its configured server mount. Refresh after it becomes available." } }
                if !can_add && !sources.is_empty() { p { class: "scanner-hint", "Each folder can have one source in your library." } }
            }
            if !sources.is_empty() {
                div { class: "scanner-sources", for source in sources.iter().cloned() {
                    { let scan = source.clone(); let remove = source.clone();
                        let source_active = jobs.iter().any(|job| job.source_id == source.id && active(&job.status));
                        let folder = roots.iter().find(|root| root.id == source.root_id);
                        let available = folder.is_some_and(|root| root.available);
                        rsx! { article { class: "scanner-source", key: "{source.id}",
                            div { h3 { "{source.name}" } small { {folder.map(|root| root.name.clone()).unwrap_or_else(|| "Folder no longer configured".into())} if !available { " · Unavailable" } } }
                            div { class: "scanner-actions",
                                button { class: "button secondary small", disabled: disabled || source_active || !available, "aria-label": "Scan {source.name}", onclick: move |_| act(Action::Scan(scan.clone()), ui, onchanged), "Scan now" }
                                button { class: "button danger small", disabled: disabled || source_active, "aria-label": "Remove {source.name}", onclick: move |_| act(Action::Remove(remove.clone()), ui, onchanged), "Remove source" }
                            }
                        } }
                    }
                } }
                small { class: "scanner-hint", "Removing a source only stops scanning it and clears its scan history. Music already in your library is kept." }
            }
            div { class: "scanner-jobs-heading", h3 { "Scan history" } small { "Most recent 100 scans. Progress updates while a scan is active." } }
            if jobs.is_empty() && !(ui.loading)() && (ui.error)().is_empty() { p { class: "scanner-empty", "No scans yet. Add a source, then choose Scan now." } }
            div { class: "scanner-jobs", for job in jobs.iter().cloned() {
                { let source_name = sources.iter().find(|source| source.id == job.source_id).map(|source| source.name.clone()).unwrap_or_else(|| "Music folder".into());
                    let cancellation = job.id.clone(); let running = active(&job.status);
                    rsx! { article { class: "scanner-job", key: "{job.id}",
                        div { class: "scanner-job-heading", div { strong { "{source_name}" } span { class: "scanner-job-status", role: "status", {status_label(&job.status)} } }
                            if running { button { class: "button secondary small", disabled, "aria-label": "Cancel scan", onclick: move |_| act(Action::Cancel(cancellation.clone()), ui, onchanged), "Cancel scan" } }
                        }
                        if running { progress { class: "scanner-progress", "aria-label": "Scanning {source_name}" } }
                        dl { class: "scanner-counters",
                            div { dt { "Visited" } dd { "{job.visited}" } }
                            div { dt { "Added" } dd { "{job.imported}" } }
                            div { dt { "Duplicates" } dd { "{job.duplicates}" } }
                            div { dt { "Unchanged" } dd { "{job.unchanged}" } }
                            div { dt { "Skipped" } dd { "{job.skipped}" } }
                            div { dt { "Failed files" } dd { "{job.failed_files}" } }
                        }
                        if let Some(message) = job.error { p { class: "scanner-job-error", "{message}" } }
                        if job.status == "failed" { small { "Check the folder and storage availability, then choose Scan now to try again." } }
                    } }
                }
            } }
        }
    }
}
