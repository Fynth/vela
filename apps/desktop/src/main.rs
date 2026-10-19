use dioxus::desktop::Config;

fn main() {
    if std::env::args().any(|argument| argument == "--version") {
        println!("vela-desktop {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let window = vela_desktop::window::builder("Vela Music");
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_menu(None)
                .with_navigation_handler(|_| false),
        )
        .launch(vela_desktop::ui::App);
}
