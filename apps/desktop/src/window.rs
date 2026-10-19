//! Shared production/QA window policy. The compositor owns the outer frame.
use dioxus::desktop::{LogicalSize, WindowBuilder};

pub fn builder(title: &str) -> WindowBuilder {
    WindowBuilder::new()
        .with_title(title)
        .with_decorations(!cfg!(target_os = "linux"))
        .with_inner_size(LogicalSize::new(1280.0, 840.0))
        .with_min_inner_size(LogicalSize::new(640.0, 480.0))
}
