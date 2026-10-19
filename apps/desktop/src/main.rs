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
                // Dioxus defaults to XWayland + WEBKIT_DISABLE_DMABUF_RENDERER=1
                // on Wayland, which caps WebKit to a software/compat path. Keep
                // the native Wayland surface and DMABUF so the compositor's real
                // refresh rate (e.g. 144 Hz) is usable.
                .with_disable_dma_buf_on_wayland(false)
                .with_navigation_handler(|_| false),
        )
        .launch(vela_desktop::ui::App);
}
