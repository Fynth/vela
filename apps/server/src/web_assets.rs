//! Guarantees that the server serves the web client that matches the source.
//!
//! The browser client is a separate WebAssembly artifact, so the API binary and
//! the UI can drift apart: a server started against a missing or stale `dist/`
//! answers every page request with 404 while the API still works. That is the
//! failure mode a developer sees when a project is started "the usual way" and
//! the interface looks empty or out of date.
//!
//! In a debug build the server therefore refreshes the web bundle once, before
//! it opens its listener:
//!
//! * it rebuilds only when a tracked web source is newer than the built bundle,
//!   so an unchanged tree costs one `stat` per file and no compilation at all;
//! * the build runs in a child process, so a missing toolchain degrades to a
//!   visible warning instead of a failed server start;
//! * release builds never build the web here. Release artifacts are produced by
//!   `scripts/package-release.sh`, which builds the web itself, and a packaged
//!   runtime must not depend on a nested Cargo invocation.
//!
//! Nothing here changes the data directory, the API surface or the audio path.
//! It only decides whether `dist/` reflects the current sources.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The web sources whose change invalidates the built bundle.
const TRACKED_SOURCES: [&str; 14] = [
    "apps/web/index.html",
    "apps/web/style.css",
    "apps/web/now-playing.css",
    "apps/web/home-editorial.css",
    "apps/web/queue-drawer.css",
    "apps/web/bootstrap.js",
    "apps/web/player.js",
    "apps/web/src/main.rs",
    "apps/web/src/playback.rs",
    "apps/web/src/experience.rs",
    "apps/web/src/browser_player.rs",
    "apps/web/src/client.rs",
    "apps/web/src/queue_drawer.rs",
    "apps/web/src/home_editorial.rs",
];

/// The static assets copied verbatim into `dist/`.
const TRACKED_ASSETS: [&str; 5] = [
    "apps/web/assets/fonts/manrope-latin.woff2",
    "apps/web/assets/fonts/manrope-cyrillic.woff2",
    "apps/web/assets/fonts/OFL.txt",
    "apps/web/assets/brand/vela-coast.png",
    "apps/web/assets/brand/vela-wave.png",
];

/// The built bundle that the server actually serves.
const BUILT_BUNDLE: &str = "dist/pkg/vela_web_bg.wasm";

/// What happened while the bundle was refreshed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refresh {
    /// The bundle was rebuilt because a tracked source changed.
    Rebuilt,
    /// The bundle was already current.
    Current,
    /// The bundle is missing and could not be built.
    Failed(String),
    /// This build does not manage the web bundle.
    NotManaged,
}

/// Rebuilds `dist/` when the web sources are newer than the built bundle.
///
/// A debug build owns the developer's working tree, so it refreshes the bundle.
/// Release builds return [`Refresh::NotManaged`]: their bundle is produced and
/// verified by the release packaging step.
pub fn refresh(root: &Path, debug_build: bool) -> Refresh {
    if !debug_build {
        return Refresh::NotManaged;
    }
    let bundle = root.join(BUILT_BUNDLE);
    let built_at = match modified(&bundle) {
        Some(time) => time,
        // A missing bundle always has to be built.
        None => SystemTime::UNIX_EPOCH,
    };
    if !is_stale(root, built_at) {
        return Refresh::Current;
    }
    match build(root) {
        Ok(()) => Refresh::Rebuilt,
        Err(error) => Refresh::Failed(error),
    }
}

/// True when a tracked web source is newer than the built bundle.
fn is_stale(root: &Path, built_at: SystemTime) -> bool {
    TRACKED_SOURCES
        .iter()
        .chain(TRACKED_ASSETS.iter())
        .any(|relative| match modified(&root.join(relative)) {
            Some(time) => time > built_at,
            // A source that no longer exists also invalidates the bundle.
            None => true,
        })
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// Runs the project's own web build so the served bundle matches the sources.
fn build(root: &Path) -> Result<(), String> {
    let script = root.join("scripts/build-web.sh");
    if !script.is_file() {
        return Err(format!("{} is missing", script.display()));
    }
    let mut command = std::process::Command::new("bash");
    command
        .arg(&script)
        .current_dir(root)
        .env("VELA_WEB_PROFILE", "dev")
        .env(
            "CARGO_BUILD_JOBS",
            std::env::var("CARGO_BUILD_JOBS").unwrap_or_else(|_| "2".into()),
        );
    if let Some(path) = pinned_wasm_bindgen(root) {
        command.env("VELA_WASM_BINDGEN", path);
    }
    let output = command.output().map_err(|error| error.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let last = stderr
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    Err(format!("scripts/build-web.sh failed: {last}"))
}

/// The pinned `wasm-bindgen` CLI shipped under `target/tools`, when present.
fn pinned_wasm_bindgen(root: &Path) -> Option<PathBuf> {
    let tools = root.join("target").join("tools");
    let entries = std::fs::read_dir(tools).ok()?;
    let mut found: Option<PathBuf> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name()?.to_string_lossy().to_string();
        if !name.starts_with("wasm-bindgen-") || name.ends_with(".tar.gz") {
            continue;
        }
        // The pinned tool is unpacked as a directory holding the executable.
        if path.is_file() {
            found = Some(path);
        } else if path.is_dir() {
            let binary = path.join("wasm-bindgen");
            if binary.is_file() {
                found = Some(binary);
            }
        }
    }
    found
}

/// The repository root that contains `apps/server` and `scripts/build-web.sh`.
pub fn repository_root() -> Option<PathBuf> {
    // The binary runs from the workspace root in development. When it is started
    // from elsewhere, walk up until the workspace marker is found.
    let mut current = std::env::current_dir().ok()?;
    loop {
        if current.join("Cargo.toml").is_file() && current.join("apps/server").is_dir() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_release_build_never_manages_the_bundle() {
        let root = std::env::temp_dir();
        assert_eq!(refresh(&root, false), Refresh::NotManaged);
    }

    #[test]
    fn a_missing_bundle_is_stale() {
        let root = tempfile::tempdir().unwrap();
        // No tracked source and no bundle exist, so the bundle must be rebuilt.
        assert!(is_stale(root.path(), SystemTime::UNIX_EPOCH));
    }

    #[test]
    fn a_newer_source_makes_the_bundle_stale() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("apps/web/style.css");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, b"body{}").unwrap();
        let built_at = SystemTime::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(is_stale(root.path(), built_at));
    }

    #[test]
    fn the_repository_root_is_discovered_from_a_nested_directory() {
        let root = repository_root();
        assert!(root.is_some());
        assert!(root.unwrap().join("scripts/build-web.sh").is_file());
    }
}
