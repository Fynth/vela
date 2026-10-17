//! Original Vela line icons. Accessible names belong to the enclosing control.
use dioxus::prelude::*;

#[component]
pub fn Icon(name: &'static str) -> Element {
    let path = match name {
        "home" => "M3 10 12 3l9 7v10a1 1 0 0 1-1 1h-5v-7H9v7H4a1 1 0 0 1-1-1Z",
        "search" => "M20 20l-5-5M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0Z",
        "library" => "M4 4v16M9 4v16M15 4l5 15M3 20h8",
        "albums" => "M4 3h16v18H4ZM16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0Zm-4 0h.01",
        "artists" => "M16 8a4 4 0 1 1-8 0 4 4 0 0 1 8 0ZM4 21v-2a8 8 0 0 1 16 0v2",
        "heart" => "M20 5a5 5 0 0 0-8 1 5 5 0 0 0-8-1c-4 5 1 10 8 15 7-5 12-10 8-15Z",
        "playlists" | "queue" => "M3 5h13M3 10h13M3 15h8m5-1 6 4-6 4Z",
        "imports" | "upload" => "M12 16V3m-5 5 5-5 5 5M4 15v6h16v-6",
        "jobs" => "M4 12h4l3-8 4 16 3-8h3",
        "radio" => {
            "M5 5a10 10 0 0 0 0 14M8 8a6 6 0 0 0 0 8m11-11a10 10 0 0 1 0 14m-3-11a6 6 0 0 1 0 8m-3-4h.01M12 12v9"
        }
        "history" => "M3 11a9 9 0 1 1 2 7M3 5v6h6m3-4v5l3 2",
        "settings" => {
            "M9 3h6l1 4 4 1v6l-4 1-1 4H9l-1-4-4-1V8l4-1Zm6 8a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z"
        }
        "play" => "m8 4 12 8-12 8Z",
        "pause" => "M8 4v16M16 4v16",
        "previous" => "M5 4v16m14-16L7 12l12 8Z",
        "next" => "M19 4v16M5 4l12 8-12 8Z",
        "next-add" => "M3 5h12M3 10h9M3 15h6m7-4v10m-5-5h10",
        "shuffle" => "M3 5h3c6 0 6 14 12 14h3m-4-4 4 4-4 4M3 19h3c6 0 6-14 12-14h3m-4-4 4 4-4 4",
        "repeat" => "M5 7h14l-4-4m4 4-4 4M19 17H5l4 4m-4-4 4-4",
        "repeat-one" => "M5 6h14l-3-3m3 3-3 3M19 18H5l3 3m-3-3 3-3M11 11l2-1v5",
        "volume" => "M3 9h4l5-5v16l-5-5H3Zm13-2a7 7 0 0 1 0 10m3-13a11 11 0 0 1 0 16",
        "sun" => {
            "M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0ZM12 2v2m0 16v2M2 12h2m16 0h2M5 5l1 1m12 12 1 1M5 19l1-1M18 6l1-1"
        }
        "moon" => "M21 13A9 9 0 1 1 11 3a7 7 0 0 0 10 10Z",
        "plus" => "M12 4v16M4 12h16",
        "close" => "m5 5 14 14M5 19 19 5",
        "more" => "M4 12h.01M12 12h.01M20 12h.01",
        "edit" => "m15 3 6 6L8 22H2v-6Zm-2 2 6 6",
        "up" => "m5 12 7-7 7 7M12 5v16",
        "check" => "m4 12 5 5L20 6",
        "back" => "m14 5-7 7 7 7",
        _ => "M9 18V5l12-2v13M9 18a3 3 0 1 1-3-3h3m12 1a3 3 0 1 1-3-3h3",
    };
    rsx! { svg { class: "vela-icon", view_box: "0 0 24 24", width: "24", height: "24", fill: "none", stroke: "currentColor", stroke_width: "1.7", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true", "focusable": "false", path { d: path } } }
}
