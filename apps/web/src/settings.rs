//! Settings → Appearance for device-scoped presentation preferences.
//!
//! Only settings this build actually implements are exposed here: app size
//! (the SCALE01–SCALE14 web subset) and theme (S001). Every control applies live to
//! the document root, persists the versioned device document and reports honestly
//! whether the device accepted the write.
use crate::preferences::{
    self, Preferences, Repair, SCALE_DEFAULT, SCALE_MAX, SCALE_MIN, SCALE_STEP, Theme,
};
use dioxus::prelude::*;

/// Outcome of the last device-store write, for truthful status copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveStatus {
    /// Nothing changed in this session yet.
    Unchanged,
    /// The device accepted the document.
    Saved,
    /// Applied live, but the device rejected the write.
    SessionOnly,
}

/// Apply a validated app size live and persist it immediately (minus/plus/reset).
pub fn set_scale(device: Signal<Preferences>, status: Signal<SaveStatus>, percent: u16) {
    if !preferences::scale_is_valid(percent) {
        return;
    }
    let mut device = device;
    let mut next = device();
    next.app_scale_percent = percent;
    device.set(next);
    persist(device, status);
}

/// Apply a size live while the value is still moving (slider drag).
///
/// Persistence is coalesced: the caller saves on pointer release/blur.
pub fn preview_scale(device: Signal<Preferences>, percent: u16) {
    if !preferences::scale_is_valid(percent) {
        return;
    }
    let mut device = device;
    let mut next = device();
    next.app_scale_percent = percent;
    device.set(next);
}

/// Persist the currently applied preferences and record the outcome.
pub fn persist(device: Signal<Preferences>, status: Signal<SaveStatus>) {
    let mut status = status;
    status.set(if preferences::save(device()) {
        SaveStatus::Saved
    } else {
        SaveStatus::SessionOnly
    });
}

/// Choose a theme (System/Dark/Light) and persist it.
pub fn set_theme(device: Signal<Preferences>, status: Signal<SaveStatus>, theme: Theme) {
    let mut device = device;
    let mut next = device();
    next.theme = theme;
    device.set(next);
    persist(device, status);
}

/// Flip between the two explicit themes, preserving the one-click topbar toggle.
///
/// The result is always an explicit `Dark`/`Light`, so the document root's
/// `data-theme` attribute always states the theme that is actually painted.
pub fn toggle_theme(device: Signal<Preferences>, status: Signal<SaveStatus>) {
    let theme = if preferences::effective_theme(device()) == "light" {
        Theme::Dark
    } else {
        Theme::Light
    };
    set_theme(device, status, theme);
}

#[component]
pub fn Appearance(
    device: Signal<Preferences>,
    status: Signal<SaveStatus>,
    repair: Signal<Option<Repair>>,
    zoom_supported: bool,
) -> Element {
    let current = *device.read();
    let percent = current.app_scale_percent;
    let theme = current.theme;
    let saved = *status.read();
    let repaired = *repair.read();
    let at_min = percent <= SCALE_MIN;
    let at_max = percent >= SCALE_MAX;
    let at_default = percent == SCALE_DEFAULT;
    let supported = zoom_supported;
    rsx! {
        section {
            class: "panel settings-appearance",
            "aria-labelledby": "appearance-title",
            "data-testid": "settings-appearance",
            span { class: "eyebrow", "THIS DEVICE" }
            h2 { id: "appearance-title", "Appearance" }
            p { "These choices are stored in this browser only. They are never sent to the server, so they do not follow you to another device or account, and they work before you sign in." }
            if let Some(reason) = repaired {
                p { class: "appearance-note", role: "status", "data-testid": "appearance-recovery", "{reason.message()}" }
            }
            div { class: "setting-row",
                div { class: "setting-text",
                    h3 { id: "app-size-title", "App size" }
                    p { id: "app-size-help", "Changes this device only. It enlarges Vela's own layout without remounting playback. Your browser's zoom stays separate and its shortcuts keep working." }
                }
                div { class: "setting-value",
                    button {
                        class: "icon-button app-scale-step",
                        "aria-label": "Decrease app size",
                        disabled: at_min || !supported,
                        onclick: move |_| {
                            let next = device().app_scale_percent.saturating_sub(SCALE_STEP);
                            set_scale(device, status, next);
                        },
                        "−"
                    }
                    span { class: "app-scale-value", "data-testid": "app-scale-value", "{percent}%" }
                    button {
                        class: "icon-button app-scale-step",
                        "aria-label": "Increase app size",
                        disabled: at_max || !supported,
                        onclick: move |_| {
                            let next = device().app_scale_percent.saturating_add(SCALE_STEP);
                            set_scale(device, status, next);
                        },
                        "+"
                    }
                }
            }
            div { class: "app-scale-control",
                label { r#for: "app-scale", "App size slider" }
                input {
                    id: "app-scale",
                    r#type: "range",
                    min: "{SCALE_MIN}",
                    max: "{SCALE_MAX}",
                    step: "{SCALE_STEP}",
                    value: "{percent}",
                    disabled: !supported,
                    "aria-label": "App size",
                    "aria-valuetext": "{percent} percent",
                    "aria-describedby": "app-size-help",
                    oninput: move |event| {
                        if let Ok(value) = event.value().parse::<u16>() {
                            preview_scale(device, value);
                        }
                    },
                    onchange: move |_| persist(device, status),
                }
                div { class: "app-scale-legend", span { "{SCALE_MIN}%" } span { "{SCALE_DEFAULT}%" } span { "{SCALE_MAX}%" } }
            }
            div { class: "appearance-actions",
                button {
                    class: "button secondary small",
                    disabled: at_default,
                    onclick: move |_| set_scale(device, status, SCALE_DEFAULT),
                    "Reset to 100%"
                }
                if !supported {
                    p { class: "appearance-note", role: "status", "data-testid": "app-scale-unsupported", "This browser cannot resize Vela's own layout. Use your browser's zoom (Ctrl/Cmd with + or −) instead; the saved value stays for browsers that can apply it." }
                }
            }
            if saved == SaveStatus::Saved {
                p { class: "appearance-note", role: "status", "data-testid": "appearance-save-state", "Saved on this device." }
            } else if saved == SaveStatus::SessionOnly {
                p { class: "appearance-note warning", role: "alert", "data-testid": "appearance-save-state", "Applied for this session; Vela could not save it on this device. Playback and your saved queue are unaffected." }
            }
            div { class: "setting-divider" }
            div { class: "setting-row",
                div { class: "setting-text",
                    h3 { id: "theme-title", "Theme" }
                    p { id: "theme-help", "System follows your device appearance when Vela opens. Dark and Light apply immediately and keep a clear contrast for controls and focus." }
                }
                div { class: "theme-choices", "aria-labelledby": "theme-title", role: "radiogroup",
                    for (value, label) in [(Theme::System, "System"), (Theme::Dark, "Dark"), (Theme::Light, "Light")] {
                        label { class: "theme-choice", key: "{label}",
                            input {
                                r#type: "radio",
                                name: "vela-theme",
                                value: "{value.as_str()}",
                                checked: theme == value,
                                onchange: move |_| set_theme(device, status, value),
                            }
                            span { "{label}" }
                        }
                    }
                }
            }
        }
    }
}
