//! Device-only presentation preferences for the native Linux client.
//!
//! Contract (PRODUCT_EXPANSION `03_SETTINGS_EXPERIENCE.md`, `04_APPLICATION_SCALE.md`,
//! `05_SETTINGS_REGISTRY.md` S001/S002, `11_ACCEPTANCE_AND_QA.md`):
//!
//! - The document is presentation-only and device-only. It is a small versioned JSON
//!   object in `$XDG_CONFIG_HOME/vela/desktop.presentation.json`
//!   (`$HOME/.config/vela/desktop.presentation.json` when that variable is unset or
//!   relative). It never carries credentials, session/CSRF tokens, cookies, media
//!   paths, queue/journal state, listening history or account identifiers, so it is
//!   safe to share between the accounts that sign in on this installation and safe to
//!   read before sign-in with no server contact (SCALE01/SCALE12).
//! - The directory is owner-only `0700` and the file owner-only `0600`. Writes are
//!   staged in a private temporary file in the same directory, flushed, then renamed
//!   over the target, so a crash or a short write can never publish half a document.
//! - App size is an integer percentage on fixed 5-point steps in `80..=150` (the
//!   validated range this device slice ships; the wider 75–200 preset ladder in
//!   `04_APPLICATION_SCALE.md` is not implemented here) and the theme is
//!   `system|dark|light`. `100%` means "follow the host's own monitor scaling".
//! - A malformed, oversized, unreadable or foreign-version document is never passed to
//!   the renderer: it resolves to the documented defaults (100% / Dark) with a typed
//!   [`Repair`] reason the Settings page explains. The unreadable text is left exactly
//!   where it is; only an explicit choice in Settings replaces it.
//! - This module owns no audio, HTTP, account, queue or session state. It returns
//!   values and writes one private file; `ui.rs` owns applying them.
use serde_json::Value;
use std::{
    ffi::OsStr,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

/// Directory of the versioned device presentation document, under the config base.
const STORE_DIR: &str = "vela";
/// File name of the versioned device presentation document.
pub const STORE_FILE: &str = "desktop.presentation.json";
/// Document version understood by this build.
pub const DOCUMENT_VERSION: u32 = 1;
/// Lowest selectable app size, in percent.
pub const SCALE_MIN: u16 = 80;
/// Highest selectable app size, in percent.
pub const SCALE_MAX: u16 = 150;
/// Step between selectable app sizes, in percent.
pub const SCALE_STEP: u16 = 5;
/// Default app size, in percent. Follows the host's own monitor scaling.
pub const SCALE_DEFAULT: u16 = 100;
/// Largest document this build is willing to read. The writer produces ~60 bytes.
const MAX_FILE_BYTES: u64 = 4 * 1024;

const WRITE_ERROR: &str = "Appearance settings could not be saved on this device.";

/// Appearance theme preference. `System` follows the desktop's own documented
/// `prefers-color-scheme` signal through CSS; it is never inferred in Rust.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    System,
    Dark,
    Light,
}

impl Theme {
    /// Stored/rendered name. Also the wrapper's `data-theme` attribute value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    /// Parse an exact stored name; anything else is out of contract.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "system" => Some(Self::System),
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            _ => None,
        }
    }
}

/// Validated device presentation values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preferences {
    pub app_scale_percent: u16,
    pub theme: Theme,
}

impl Default for Preferences {
    fn default() -> Self {
        // Dark stays the shipped native appearance, so an absent file changes nothing
        // for existing installations; System is an available choice, not a surprise.
        Self {
            app_scale_percent: SCALE_DEFAULT,
            theme: Theme::Dark,
        }
    }
}

/// Why stored presentation values could not be used verbatim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repair {
    /// Absent, unreadable, oversized, non-UTF-8 or non-object document.
    Unreadable,
    /// A document written by a version this build does not understand.
    UnsupportedVersion,
    /// Some fields were outside the documented contract and were replaced.
    InvalidValues,
}

impl Repair {
    /// Non-blocking explanation. It never claims the stored file was deleted.
    pub fn message(self) -> &'static str {
        match self {
            Self::Unreadable => {
                "Vela could not read the saved appearance settings on this device, so it started with the defaults: 100% app size and the dark theme. The unreadable file was left in place; choosing a value here replaces it."
            }
            Self::UnsupportedVersion => {
                "The appearance settings on this device were saved by a newer version of Vela. This build used the defaults, left that file untouched and will not overwrite it on its own; choosing a value here replaces it."
            }
            Self::InvalidValues => {
                "Part of the saved appearance settings on this device was outside the supported range, so Vela used the defaults for those values. The file was left untouched until you change something here."
            }
        }
    }
}

/// Result of resolving the device store.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Loaded {
    pub preferences: Preferences,
    /// Set when the stored values had to be repaired; the UI explains it once.
    pub repair: Option<Repair>,
}

/// Outcome of the last device-store write, for truthful status copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveStatus {
    /// Nothing on this device has been changed in this session yet.
    Unchanged,
    /// A private write is in flight.
    Pending,
    /// The device accepted the document.
    Saved,
    /// Applied live, but the device rejected the write.
    SessionOnly,
}

impl SaveStatus {
    /// Copy for the Settings page. `None` means "show no save line".
    pub fn message(self) -> Option<&'static str> {
        match self {
            Self::Unchanged => None,
            Self::Pending => Some("Saving on this device…"),
            Self::Saved => Some("Saved on this device."),
            Self::SessionOnly => Some(
                "Applied for this session; Vela could not save it on this device. Playback and your queue are unaffected.",
            ),
        }
    }
}

/// Whether `percent` is a selectable app size: integer percentage, `80..=150`,
/// on a 5-point step.
pub fn scale_is_valid(percent: u16) -> bool {
    (SCALE_MIN..=SCALE_MAX).contains(&percent) && percent.is_multiple_of(SCALE_STEP)
}

/// Normalise any stored integer to the nearest selectable value, clamped to the
/// documented range. Used only for already-integer input from this build.
pub fn scale_normalized(percent: u16) -> u16 {
    percent.clamp(SCALE_MIN, SCALE_MAX)
}

/// One stepper step from `percent`, clamped to the documented range.
///
/// Pure so the decrease/increase controls are unit-tested without a renderer.
pub fn stepped(percent: u16, delta: i32) -> u16 {
    let value = i32::from(percent).saturating_add(delta);
    let clamped = value.clamp(i32::from(SCALE_MIN), i32::from(SCALE_MAX));
    let clamped = u16::try_from(clamped).unwrap_or(SCALE_DEFAULT);
    // Re-snap onto the documented step grid in case a caller starts off-grid.
    let offset = clamped - SCALE_MIN;
    let snapped = SCALE_MIN + (offset / SCALE_STEP) * SCALE_STEP;
    snapped.clamp(SCALE_MIN, SCALE_MAX)
}

/// Exact CSS zoom factor text for a validated percentage: `100 -> "1.00"`.
///
/// Formatting is integer arithmetic with a fixed two-digit fraction, so the
/// renderer never receives a locale-dependent or rounded-to-nothing value.
pub fn zoom_css(percent: u16) -> String {
    format!("{}.{:02}", percent / 100, percent % 100)
}

/// The wrapper presentation attributes applied exactly once per value.
///
/// `--app-zoom` is the only custom property this adapter writes and `zoom` is the
/// only scale declaration in the packaged stylesheet. The percentage is therefore
/// never stacked with a second zoom, a `transform` scale or monitor DPI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationAttributes {
    /// Inline `style` value for the existing root `.native-app` wrapper.
    pub style: String,
    /// `data-theme` value: `system`, `dark` or `light`.
    pub theme: &'static str,
    /// `data-app-scale` value: the requested percentage as a decimal string.
    pub app_scale: String,
}

/// Build the wrapper attributes for validated preferences.
pub fn attributes(preferences: Preferences) -> PresentationAttributes {
    debug_assert!(scale_is_valid(preferences.app_scale_percent));
    let zoom = zoom_css(preferences.app_scale_percent);
    PresentationAttributes {
        style: format!("--app-zoom:{zoom};zoom:{zoom}"),
        theme: preferences.theme.as_str(),
        app_scale: preferences.app_scale_percent.to_string(),
    }
}

fn scale_from_number(value: Option<&Value>) -> Option<u16> {
    let number = value?.as_i64()?;
    let percent = u16::try_from(number).ok()?;
    scale_is_valid(percent).then_some(percent)
}

/// Serialise the bounded presentation document.
///
/// Field order and content are deterministic and only presentation fields exist.
pub fn document(preferences: Preferences) -> String {
    serde_json::json!({
        "version": DOCUMENT_VERSION,
        "app_scale_percent": preferences.app_scale_percent,
        "theme": preferences.theme.as_str(),
    })
    .to_string()
}

/// Validate a stored document without touching the filesystem.
///
/// A readable document keeps every valid field and reports the first contract
/// violation; an unreadable or foreign-version document is rejected wholesale.
pub fn parse_document(raw: &str) -> Result<(Preferences, Option<Repair>), Repair> {
    let value: Value = serde_json::from_str(raw).map_err(|_| Repair::Unreadable)?;
    let object = value.as_object().ok_or(Repair::Unreadable)?;
    let version = object
        .get("version")
        .and_then(Value::as_u64)
        .ok_or(Repair::Unreadable)?;
    if version != u64::from(DOCUMENT_VERSION) {
        return Err(Repair::UnsupportedVersion);
    }
    let mut repair = None;
    let app_scale_percent = match scale_from_number(object.get("app_scale_percent")) {
        Some(percent) => percent,
        None => {
            repair = Some(Repair::InvalidValues);
            SCALE_DEFAULT
        }
    };
    let theme = match object
        .get("theme")
        .and_then(Value::as_str)
        .and_then(Theme::parse)
    {
        Some(theme) => theme,
        None => {
            repair = Some(Repair::InvalidValues);
            Preferences::default().theme
        }
    };
    Ok((
        Preferences {
            app_scale_percent,
            theme,
        },
        repair,
    ))
}

/// Resolve the device store from the text that was read, if any.
///
/// Pure: it performs no I/O, so startup behaviour is unit-testable. An unusable
/// document yields the defaults; nothing here deletes or rewrites the stored text.
pub fn resolve(stored: Option<&str>) -> Loaded {
    let Some(raw) = stored else {
        return Loaded::default();
    };
    match parse_document(raw) {
        Ok((preferences, repair)) => Loaded {
            preferences,
            repair,
        },
        Err(repair) => Loaded {
            preferences: Preferences::default(),
            repair: Some(repair),
        },
    }
}

/// Config base: `$XDG_CONFIG_HOME` when absolute, else `$HOME/.config`.
fn store_base(config: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf> {
    match config.map(Path::new).filter(|path| path.is_absolute()) {
        Some(config) => Some(config.to_path_buf()),
        None => Some(home.map(Path::new)?.join(".config")),
    }
}

/// Exact store path for explicit config/home inputs. Pure for tests.
fn store_path_in(config: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf> {
    Some(store_base(config, home)?.join(STORE_DIR).join(STORE_FILE))
}

/// Exact store path of the device presentation document, or `None` when the
/// environment exposes no config directory at all.
pub fn store_path() -> Option<PathBuf> {
    store_path_in(
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

/// Abbreviate an owned path under `$HOME` for display. Pure for tests.
fn abbreviate_home(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if !rest.as_os_str().is_empty() => format!("~/{}", rest.display()),
        _ => path.display().to_string(),
    }
}

/// Where this device's settings live, abbreviated, or empty when unavailable.
///
/// Presentation only: the path is the local listener's own config file, never a
/// server/operator path, credential or media location.
pub fn store_note() -> String {
    let Some(path) = store_path() else {
        return String::new();
    };
    abbreviate_home(
        &path,
        std::env::var_os("HOME").map(PathBuf::from).as_deref(),
    )
}

/// Read the stored document text without interpreting it.
///
/// `Ok(None)` means "no file"; `Ok(Some(text))` the exact stored text; `Err` a
/// present but unreadable/oversized/non-UTF-8 file. Nothing is removed in any case.
fn read_text(path: &Path) -> Result<Option<String>, Repair> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Repair::Unreadable),
    };
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err(Repair::Unreadable);
    }
    let bytes = fs::read(path).map_err(|_| Repair::Unreadable)?;
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| Repair::Unreadable)
}

/// Resolve the device store at `path`. Read-only; a bad file is never deleted.
pub fn load_from(path: &Path) -> Loaded {
    match read_text(path) {
        Ok(text) => resolve(text.as_deref()),
        Err(repair) => Loaded {
            preferences: Preferences::default(),
            repair: Some(repair),
        },
    }
}

/// Resolve the device store for this device.
pub fn load() -> Loaded {
    match store_path() {
        Some(path) => load_from(&path),
        None => Loaded {
            preferences: Preferences::default(),
            repair: None,
        },
    }
}

#[cfg(unix)]
fn private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn private_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn private_file(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn private_file(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Publish `preferences` at `path` atomically.
///
/// The staged file is created owner-only in the same directory, written, flushed,
/// then renamed over the target; the previous document is replaced only on a
/// complete write. A failure leaves the previous file untouched and removes the
/// staging file.
pub fn save_to(path: &Path, preferences: Preferences) -> Result<(), String> {
    if !scale_is_valid(preferences.app_scale_percent) {
        return Err(WRITE_ERROR.to_owned());
    }
    let parent = path.parent().ok_or_else(|| WRITE_ERROR.to_owned())?;
    fs::create_dir_all(parent).map_err(|_| WRITE_ERROR.to_owned())?;
    private_directory(parent).map_err(|_| WRITE_ERROR.to_owned())?;
    let temporary = parent.join(format!(".{}-{}.tmp", STORE_FILE, uuid::Uuid::new_v4()));
    #[cfg(unix)]
    let file = {
        use std::os::unix::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
    };
    #[cfg(not(unix))]
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary);
    let mut file = file.map_err(|_| WRITE_ERROR.to_owned())?;
    let payload = document(preferences);
    let result = (|| {
        file.write_all(payload.as_bytes())?;
        file.sync_all()?;
        // Make the name durable in its directory before reporting success.
        fs::rename(&temporary, path)?;
        private_file(path)?;
        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok::<_, std::io::Error>(())
    })();
    drop(file);
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(WRITE_ERROR.to_owned());
    }
    Ok(())
}

/// Persist `preferences` for this device.
///
/// Blocking file I/O: callers on the UI thread run this on a blocking worker.
pub fn save(preferences: Preferences) -> Result<(), String> {
    let path = store_path().ok_or_else(|| WRITE_ERROR.to_owned())?;
    save_to(&path, preferences)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(percent: u16, theme: Theme) -> Preferences {
        Preferences {
            app_scale_percent: percent,
            theme,
        }
    }

    fn mode(path: &Path) -> u32 {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::metadata(path).unwrap().permissions().mode() & 0o777
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            0o600
        }
    }

    #[test]
    fn scale_range_steps_and_steppers_are_exact() {
        for percent in [80_u16, 85, 100, 125, 150] {
            assert!(scale_is_valid(percent), "{percent} must be selectable");
        }
        for percent in [0_u16, 75, 79, 83, 151, 200, 1000] {
            assert!(!scale_is_valid(percent), "{percent} must be rejected");
        }
        assert_eq!(SCALE_DEFAULT, 100);
        assert!(scale_is_valid(SCALE_DEFAULT));
        assert_eq!(stepped(100, i32::from(SCALE_STEP)), 105);
        assert_eq!(stepped(100, -i32::from(SCALE_STEP)), 95);
        assert_eq!(stepped(SCALE_MIN, -i32::from(SCALE_STEP)), SCALE_MIN);
        assert_eq!(stepped(SCALE_MAX, i32::from(SCALE_STEP)), SCALE_MAX);
        assert_eq!(
            stepped(148, 0),
            145,
            "off-grid values snap down to the grid"
        );
        assert_eq!(scale_normalized(151), SCALE_MAX);
        assert_eq!(scale_normalized(0), SCALE_MIN);
    }

    #[test]
    fn zoom_text_and_wrapper_attributes_are_deterministic() {
        assert_eq!(zoom_css(100), "1.00");
        assert_eq!(zoom_css(80), "0.80");
        assert_eq!(zoom_css(95), "0.95");
        assert_eq!(zoom_css(125), "1.25");
        assert_eq!(zoom_css(150), "1.50");
        let wrapper = attributes(sample(125, Theme::System));
        assert_eq!(wrapper.style, "--app-zoom:1.25;zoom:1.25");
        assert_eq!(wrapper.theme, "system");
        assert_eq!(wrapper.app_scale, "125");
        assert_eq!(
            attributes(sample(100, Theme::Dark)).style,
            "--app-zoom:1.00;zoom:1.00",
            "100% is an explicit no-op rather than an absent attribute"
        );
    }

    #[test]
    fn document_round_trips_without_private_fields() {
        let preferences = sample(125, Theme::System);
        let text = document(preferences);
        assert_eq!(
            text,
            r#"{"app_scale_percent":125,"theme":"system","version":1}"#
        );
        assert_eq!(parse_document(&text), Ok((preferences, None)));
        let lower = text.to_ascii_lowercase();
        for forbidden in [
            "csrf", "token", "cookie", "password", "session", "queue", "history", "origin",
            "username", "path",
        ] {
            assert!(
                !lower.contains(forbidden),
                "the device document must not contain {forbidden}"
            );
        }
    }

    #[test]
    fn malformed_documents_are_rejected_before_the_renderer() {
        for raw in [
            "",
            "not json",
            "null",
            "[]",
            "\"125\"",
            "{}",
            r#"{"version":"1","app_scale_percent":125,"theme":"dark"}"#,
            r#"{"app_scale_percent":125,"theme":"dark"}"#,
            r#"{"version":1,"app_scale_percent":125,"theme":"dark""#,
        ] {
            assert_eq!(
                parse_document(raw),
                Err(Repair::Unreadable),
                "{raw} must be unreadable"
            );
        }
        for version in [0, 2, 9] {
            let raw = format!(r#"{{"version":{version},"app_scale_percent":150,"theme":"dark"}}"#);
            assert_eq!(parse_document(&raw), Err(Repair::UnsupportedVersion));
        }
    }

    #[test]
    fn out_of_contract_values_fall_back_field_by_field() {
        assert_eq!(
            parse_document(r#"{"version":1,"app_scale_percent":175,"theme":"light"}"#),
            Ok((sample(100, Theme::Light), Some(Repair::InvalidValues)))
        );
        assert_eq!(
            parse_document(r#"{"version":1,"app_scale_percent":83,"theme":"dark"}"#),
            Ok((sample(100, Theme::Dark), Some(Repair::InvalidValues)))
        );
        assert_eq!(
            parse_document(r#"{"version":1,"app_scale_percent":-5,"theme":"dark"}"#),
            Ok((Preferences::default(), Some(Repair::InvalidValues)))
        );
        assert_eq!(
            parse_document(r#"{"version":1,"app_scale_percent":1.5,"theme":"dark"}"#),
            Ok((Preferences::default(), Some(Repair::InvalidValues)))
        );
        assert_eq!(
            parse_document(r#"{"version":1,"app_scale_percent":125,"theme":"sepia"}"#),
            Ok((sample(125, Theme::Dark), Some(Repair::InvalidValues)))
        );
        assert_eq!(
            parse_document(r#"{"version":1,"app_scale_percent":125,"theme":"light"}"#),
            Ok((sample(125, Theme::Light), None))
        );
    }

    #[test]
    fn every_repair_explains_itself_without_claiming_a_deletion() {
        for repair in [
            Repair::Unreadable,
            Repair::UnsupportedVersion,
            Repair::InvalidValues,
        ] {
            let message = repair.message();
            assert!(message.contains("default"), "{message}");
            assert!(
                !message.to_ascii_lowercase().contains("deleted"),
                "{message}"
            );
            assert!(
                message.to_ascii_lowercase().contains("left"),
                "{message} must say the stored copy stays"
            );
        }
        assert_eq!(SaveStatus::Unchanged.message(), None);
        assert!(SaveStatus::Saved.message().unwrap().contains("Saved"));
        assert!(
            SaveStatus::SessionOnly
                .message()
                .unwrap()
                .contains("this session")
        );
    }

    #[test]
    fn resolve_uses_defaults_and_reports_the_reason() {
        assert_eq!(resolve(None), Loaded::default());
        assert_eq!(
            resolve(Some("{not json")),
            Loaded {
                preferences: Preferences::default(),
                repair: Some(Repair::Unreadable),
            }
        );
        assert_eq!(
            resolve(Some(r#"{"version":9,"app_scale_percent":150}"#)),
            Loaded {
                preferences: Preferences::default(),
                repair: Some(Repair::UnsupportedVersion),
            }
        );
        assert_eq!(
            resolve(Some(&document(sample(80, Theme::Light)))),
            Loaded {
                preferences: sample(80, Theme::Light),
                repair: None,
            }
        );
    }

    #[test]
    fn store_path_prefers_an_absolute_xdg_config_home_then_home() {
        let xdg = store_path_in(
            Some(OsStr::new("/tmp/xdg")),
            Some(OsStr::new("/home/listener")),
        );
        assert_eq!(
            xdg.as_deref(),
            Some(Path::new("/tmp/xdg/vela/desktop.presentation.json"))
        );
        let fallback = store_path_in(None, Some(OsStr::new("/home/listener")));
        assert_eq!(
            fallback.as_deref(),
            Some(Path::new(
                "/home/listener/.config/vela/desktop.presentation.json"
            ))
        );
        let relative = store_path_in(Some(OsStr::new("relative")), Some(OsStr::new("/home/l")));
        assert_eq!(
            relative.as_deref(),
            Some(Path::new("/home/l/.config/vela/desktop.presentation.json")),
            "a relative XDG_CONFIG_HOME is not a private absolute directory"
        );
        assert_eq!(store_path_in(None, None), None);
        assert_eq!(
            abbreviate_home(
                Path::new("/home/listener/.config/vela/desktop.presentation.json"),
                Some(Path::new("/home/listener"))
            ),
            "~/.config/vela/desktop.presentation.json"
        );
        assert_eq!(
            abbreviate_home(
                Path::new("/etc/vela.json"),
                Some(Path::new("/home/listener"))
            ),
            "/etc/vela.json"
        );
    }

    #[cfg(unix)]
    #[test]
    fn save_is_private_atomic_and_loadable_and_keeps_no_staging_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("vela").join(STORE_FILE);
        assert_eq!(
            load_from(&path),
            Loaded::default(),
            "absent file means defaults"
        );
        let preferences = sample(125, Theme::System);
        save_to(&path, preferences).unwrap();
        assert_eq!(mode(&path), 0o600, "the document is owner-only");
        assert_eq!(
            mode(path.parent().unwrap()),
            0o700,
            "the private config directory is owner-only"
        );
        assert_eq!(
            load_from(&path),
            Loaded {
                preferences,
                repair: None
            }
        );
        // A second write replaces the first atomically.
        let second = sample(150, Theme::Light);
        save_to(&path, second).unwrap();
        assert_eq!(
            load_from(&path),
            Loaded {
                preferences: second,
                repair: None
            }
        );
        let staged: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(staged.is_empty(), "no staging file may survive a write");
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_document_is_reported_and_never_deleted() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("vela").join(STORE_FILE);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"{ this is not json").unwrap();
        let loaded = load_from(&path);
        assert_eq!(loaded.preferences, Preferences::default());
        assert_eq!(loaded.repair, Some(Repair::Unreadable));
        assert_eq!(
            fs::read(&path).unwrap(),
            b"{ this is not json",
            "loading must not delete or rewrite the unreadable text"
        );
        // An explicit reset recovers even though the stored text is unreadable.
        save_to(&path, Preferences::default()).unwrap();
        assert_eq!(load_from(&path), Loaded::default());
        // A foreign version is preserved the same way.
        fs::write(
            &path,
            br#"{"version":7,"app_scale_percent":150,"theme":"light"}"#,
        )
        .unwrap();
        assert_eq!(
            load_from(&path),
            Loaded {
                preferences: Preferences::default(),
                repair: Some(Repair::UnsupportedVersion),
            }
        );
        assert!(fs::read(&path).unwrap().starts_with(b"{\"version\":7"));
        // Oversized and non-UTF-8 files are unreadable, not fatal.
        fs::write(&path, vec![b'x'; (MAX_FILE_BYTES + 1) as usize]).unwrap();
        assert_eq!(load_from(&path).repair, Some(Repair::Unreadable));
        fs::write(&path, [0xff_u8, 0xfe, 0x00]).unwrap();
        assert_eq!(load_from(&path).repair, Some(Repair::Unreadable));
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_write_keeps_the_previous_document_and_reports_it() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("vela").join(STORE_FILE);
        let preferences = sample(90, Theme::Dark);
        save_to(&path, preferences).unwrap();
        // A directory at the target name cannot be replaced by a rename, and the
        // failure must be reported rather than shown as saved.
        let blocked = directory.path().join("blocked").join(STORE_FILE);
        fs::create_dir_all(&blocked).unwrap();
        assert!(save_to(&blocked, sample(125, Theme::Light)).is_err());
        assert_eq!(
            load_from(&path),
            Loaded {
                preferences,
                repair: None
            }
        );
        assert!(
            save_to(&path, sample(151, Theme::Dark)).is_err(),
            "out-of-range values are refused before any write"
        );
        assert_eq!(
            load_from(&path),
            Loaded {
                preferences,
                repair: None
            }
        );
        let staged: Vec<_> = fs::read_dir(blocked.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(staged.is_empty(), "a failed write removes its staging file");
    }

    #[test]
    fn theme_names_round_trip() {
        for theme in [Theme::System, Theme::Dark, Theme::Light] {
            assert_eq!(Theme::parse(theme.as_str()), Some(theme));
        }
        assert_eq!(Theme::parse("System"), None);
        assert_eq!(Theme::parse(""), None);
        assert_eq!(Preferences::default().theme, Theme::Dark);
    }
}
