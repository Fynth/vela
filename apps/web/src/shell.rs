//! Shared navigation renders separately from the authoritative root player.
use crate::icons::Icon;
use dioxus::prelude::*;
use vela_domain::{LibraryStats, PlaylistSummary, User};

#[component]
pub fn Sidebar(
    page: String,
    user: User,
    stats: Option<LibraryStats>,
    playlists: Vec<PlaylistSummary>,
    radio_available: bool,
    onnav: EventHandler<String>,
    onplaylist: EventHandler<String>,
) -> Element {
    rsx! { aside { class: "sidebar",
        a { class: "brand", href: "#home", "aria-label": "Vela home", onclick: move |e| { e.prevent_default(); onnav.call("home".into()); }, img { src:"/vela.svg", alt:"" } span { class:"brand-wordmark", "vela" span { class:"brand-dot", "." } } }
        nav { "aria-label": "Main navigation",
            for (id,label) in [("home","Home"),("search","Search"),("library","Library")] {
                button { class:if page==id {"nav-item active"} else {"nav-item"}, "aria-label":label, "data-destination":id, "aria-current":if page==id {"page"} else {"false"}, onclick:move |_| onnav.call(id.into()), Icon {name:id} "{label}" }
            }
        }
        div { class:"sidebar-library", span {class:"nav-label", "YOUR LIBRARY"}
            for (id,label,icon) in [("albums","Albums","albums"),("artists","Artists","artists"),("favorites","Favorites","heart"),("playlists","Playlists","playlists"),("history","Listening history","history")] {
                button {class:if page==id {"nav-item active"} else {"nav-item"}, "aria-label":label, "data-destination":id, "aria-current":if page==id {"page"} else {"false"}, onclick:move |_| onnav.call(id.into()), Icon {name:icon} "{label}"}
            }
            if !playlists.is_empty() { div {class:"sidebar-playlists", for list in playlists.into_iter().take(6) { button {class:"sidebar-playlist", "aria-label":"Open playlist {list.name}", onclick:move |_| onplaylist.call(list.id.clone()), Icon {name:"playlists"} span {"{list.name}"} } } } }
        }
        div {class:"sidebar-bottom",
            if radio_available { button { class:if page=="radio" {"nav-item active"} else {"nav-item"}, "aria-label":"Track radio", "data-destination":"radio", onclick:move |_| onnav.call("radio".into()), Icon {name:"radio"} "Track radio" } }
            button {class:"nav-item", "aria-label":"Add music", "data-destination":"imports", onclick:move |_| onnav.call("imports".into()), Icon {name:"imports"} "Add music"}
            if user.is_admin {button {class:if page=="jobs" {"nav-item active"} else {"nav-item"}, "aria-label":"Jobs", "data-destination":"jobs", onclick:move |_| onnav.call("jobs".into()), Icon {name:"jobs"} "Jobs"}}
            button {class:if page=="network" {"nav-item active"} else {"nav-item"}, "aria-label":"Network", "data-destination":"network", "aria-current":if page=="network" {"page"} else {"false"}, onclick:move |_| onnav.call("network".into()), Icon {name:"network"} "Network"}
            button {class:if page=="settings" {"nav-item active"} else {"nav-item"}, "aria-label":"Settings", "data-destination":"settings", onclick:move |_| onnav.call("settings".into()), Icon {name:"settings"} "Settings"}
            div {class:"account", div {class:"avatar", "{user.username.chars().next().unwrap_or('V').to_ascii_uppercase()}"} div {strong {"{user.username}"} small { if let Some(stats)=stats {"{stats.tracks} private tracks"} else {"Private library"} }} }
        }
    } }
}

#[component]
pub fn MobileNavigation(page: String, administrator: bool, onnav: EventHandler<String>) -> Element {
    let mut more = use_signal(|| false);
    rsx! {
        nav {class:"mobile-nav listening-mobile-nav", "aria-label":"Mobile navigation",
            for (id,label) in [("home","Home"),("search","Search"),("library","Library"),("playlists","Playlists")] {
                button {class:if page==id {"active"} else {""}, "aria-label":label, "aria-current":if page==id {"page"} else {"false"}, onclick:move |_| {more.set(false);onnav.call(id.into());}, Icon {name:id} small {"{label}"}}
            }
            button {"aria-label":"More destinations", "aria-expanded":more(), onclick:move |_| more.set(!more()), Icon {name:"more"} small {"More"}}
        }
        if more() {div {class:"more-backdrop", onclick:move |_|more.set(false), div {class:"more-sheet", role:"dialog", "aria-modal":"true", "aria-labelledby":"more-title", onclick:move |e|e.stop_propagation(), onkeydown:move |e| {if e.key()==Key::Escape {e.stop_propagation();more.set(false);}},
            div {class:"section-toolbar", h2 {id:"more-title", "More from your library"} button {class:"icon-button", "aria-label":"Close navigation", "data-dialog-dismiss":"true", onclick:move |_|more.set(false), Icon {name:"close"}}}
            nav {"aria-label":"More destinations", for (id,label,icon) in [("albums","Albums","albums"),("artists","Artists","artists"),("favorites","Favorites","heart"),("history","Listening history","history"),("imports","Add music","imports"),("jobs","Jobs","jobs"),("network","Network","network"),("settings","Settings","settings")] {
                if id!="jobs"||administrator {button {class:"more-destination", "aria-label":label, onclick:move |_| {more.set(false);onnav.call(id.into());}, Icon {name:icon} span {"{label}"}}}
            }}
        }}}
    }
}
