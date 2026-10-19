use dioxus::desktop::{Config, LogicalSize, WindowBuilder};

fn main() {
    if std::env::args().any(|argument| argument == "--version") {
        println!("vela-desktop {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let window = WindowBuilder::new()
        .with_title("Vela Music")
        .with_inner_size(LogicalSize::new(1280.0, 840.0))
        .with_min_inner_size(LogicalSize::new(640.0, 480.0));
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_menu(None)
                .with_navigation_handler(|_| false),
        )
        .launch(vela_desktop::ui::App);
}
