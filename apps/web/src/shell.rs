//! Shared navigation renders separately from the authoritative root player.
use crate::icons::Icon;
use dioxus::prelude::*;
use vela_domain::{LibraryStats, PlaylistSummary, User};

/// Who can actually open one of the remaining implemented routes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reach {
    /// Available to every signed-in account.
    Everyone,
    /// Only once a real track radio seed exists, matching the sidebar entry.
    RadioSeed,
    /// Administrator-only routes.
    Administrator,
}

impl Reach {
    fn available(self, radio_available: bool, administrator: bool) -> bool {
        match self {
            Reach::Everyone => true,
            Reach::RadioSeed => radio_available,
            Reach::Administrator => administrator,
        }
    }
}

/// TR08: every implemented route outside the five mobile destinations.
///
/// Settings is the hub of this set, so it is not repeated in the list itself.
/// Each entry names a route this build renders; unavailable entries are filtered
/// by [`Reach`] instead of being shown as controls that cannot work.
const MORE_ROUTES: [(&str, &str, &str, Reach); 8] = [
    ("albums", "Albums", "albums", Reach::Everyone),
    ("artists", "Artists", "artists", Reach::Everyone),
    ("favorites", "Favorites", "heart", Reach::Everyone),
    ("history", "Listening history", "history", Reach::Everyone),
    ("radio", "Track radio", "radio", Reach::RadioSeed),
    ("imports", "Add music", "imports", Reach::Everyone),
    ("network", "Network", "network", Reach::Everyone),
    ("jobs", "Jobs", "jobs", Reach::Administrator),
];

/// Is this route one that the mobile "More" tab reaches through Settings?
fn in_more_group(page: &str) -> bool {
    page == "settings" || MORE_ROUTES.iter().any(|(id, ..)| *id == page)
}

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
            if radio_available { button { class:if page=="radio" {"nav-item active"} else {"nav-item"}, "aria-label":"Track radio", "data-destination":"radio", "aria-current":if page=="radio" {"page"} else {"false"}, onclick: move |_| onnav.call("radio".into()), Icon {name:"radio"} "Track radio" } }
            button {class:if page=="imports" {"nav-item active"} else {"nav-item"}, "aria-label":"Add music", "data-destination":"imports", "aria-current":if page=="imports" {"page"} else {"false"}, onclick: move |_| onnav.call("imports".into()), Icon {name:"imports"} "Add music"}
            if user.is_admin {button {class:if page=="jobs" {"nav-item active"} else {"nav-item"}, "aria-label":"Jobs", "data-destination":"jobs", "aria-current":if page=="jobs" {"page"} else {"false"}, onclick: move |_| onnav.call("jobs".into()), Icon {name:"jobs"} "Jobs"}}
            button {class:if page=="network" {"nav-item active"} else {"nav-item"}, "aria-label":"Network", "data-destination":"network", "aria-current":if page=="network" {"page"} else {"false"}, onclick: move |_| onnav.call("network".into()), Icon {name:"network"} "Network"}
            button {class:if page=="settings" {"nav-item active"} else {"nav-item"}, "aria-label":"Settings", "data-destination":"settings", "aria-current":if page=="settings" {"page"} else {"false"}, onclick: move |_| onnav.call("settings".into()), Icon {name:"settings"} "Settings"}
            div {class:"account", div {class:"avatar", "{user.username.chars().next().unwrap_or('V').to_ascii_uppercase()}"} div {strong {"{user.username}"} small { if let Some(stats)=stats {"{stats.tracks} private tracks"} else {"Private library"} }} }
        }
    } }
}

/// TR08: the five persistent mobile destinations.
///
/// "More" is a real destination that opens Settings directly and never opens an
/// intermediate sheet; Settings owns the remaining routes in its "More from
/// Vela" list. Routes reached that way keep the group marked, so a phone always
/// states where it is: `aria-current="page"` on Settings itself, `"true"` for
/// the other members of the same group.
#[component]
pub fn MobileNavigation(page: String, onnav: EventHandler<String>) -> Element {
    let more_current = in_more_group(&page);
    rsx! {
        nav {class:"mobile-nav listening-mobile-nav", "aria-label":"Mobile navigation",
            for (id,label) in [("home","Home"),("search","Search"),("library","Library"),("playlists","Playlists")] {
                button {class:if page==id {"active"} else {""}, "aria-label":label, "aria-current":if page==id {"page"} else {"false"}, "data-destination":id, onclick:move |_| onnav.call(id.into()), Icon {name:id} small {"{label}"}}
            }
            button {class:if more_current {"active"} else {""}, "aria-label":"More destinations", "aria-current":if page=="settings" {"page"} else if more_current {"true"} else {"false"}, "data-destination":"settings", onclick:move |_| onnav.call("settings".into()), Icon {name:"more"} small {"More"}}
        }
    }
}

/// TR08: the compact Settings list that keeps every remaining route one real
/// step away, on the desktop sidebar compositions and on the mobile one.
#[component]
pub fn SettingsDestinations(
    page: String,
    administrator: bool,
    radio_available: bool,
    onnav: EventHandler<String>,
) -> Element {
    rsx! {
        nav {
            class: "panel settings-destinations",
            "aria-labelledby": "more-from-vela-title",
            "data-testid": "settings-destinations",
            span { class: "eyebrow", "EVERYWHERE ELSE" }
            h2 { id: "more-from-vela-title", "More from Vela" }
            p { "Albums, artists, your listening history and the rest of Vela stay one step from here, including on a phone, where the bar below keeps the four main tabs and this page." }
            div { class: "settings-destination-list",
                for (id, label, icon, reach) in MORE_ROUTES {
                    if reach.available(radio_available, administrator) {
                        button {
                            class: if page == id { "settings-destination active" } else { "settings-destination" },
                            "aria-label": label,
                            "aria-current": if page == id { "page" } else { "false" },
                            "data-destination": id,
                            onclick: move |_| onnav.call(id.into()),
                            Icon { name: icon }
                            span { "{label}" }
                        }
                    }
                }
            }
        }
    }
}
