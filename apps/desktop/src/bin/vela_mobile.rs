//! The mobile client entry point.
//!
//! The desktop and the mobile client are the same application: one UI, one
//! HTTP client, one audio output authority and one queue. Only the shell
//! differs. The desktop shell owns a compositor window; the mobile shell is the
//! platform webview, so this binary exists to choose the right one without
//! duplicating any behaviour.
//!
//! The mobile renderer needs the `mobile` feature of Dioxus, which pulls in the
//! platform webview bridge. It is only enabled for this binary, so the desktop
//! build keeps its own, smaller dependency set.

fn main() {
    if std::env::args().any(|argument| argument == "--version") {
        println!("vela-mobile {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    dioxus::LaunchBuilder::mobile().launch(vela_desktop::ui::App);
}
