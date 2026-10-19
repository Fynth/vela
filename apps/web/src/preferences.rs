//! Device-scoped presentation preferences for the web client.
//!
//! Contract (PRODUCT_EXPANSION `04_APPLICATION_SCALE.md`, `05_SETTINGS_REGISTRY.md`
//! S001/S002, acceptance SCALE01/SCALE02/SCALE05/SCALE07/SCALE08/SCALE13):
//!
//! - The document is presentation-only and device-only: one origin's `localStorage`.
//!   It never carries credentials, session/CSRF tokens, media paths, queue state,
//!   listening history or account identifiers, so it is safe to share between
//!   accounts that sign in on this installation.
//! - It is a versioned JSON object. Malformed, foreign-version or out-of-contract
//!   values are never passed to the renderer: they resolve to the documented
//!   defaults and the caller may explain the recovery. The unreadable text is not
//!   deleted, so a newer version can still read it.
//! - App size is an integer percentage on fixed 5-point steps in `80..=150`.
//!   100% means "follow the platform's own scaling".
//! - The web adapter applies the percentage exactly once as CSS `zoom` on the
//!   document root. That composes with browser page zoom instead of replacing,
//!   blocking or reimplementing it: the browser's own Ctrl/Cmd +/- stays the
//!   user's accessibility control. It is never multiplied into monitor/DPI
//!   settings and it is never applied twice.
//! - The root element's inline `style` attribute is owned exclusively by this
//!   adapter, which only ever writes the `--app-zoom` custom property.
use serde_json::Value;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

/// Storage key of the versioned device presentation document.
pub const STORAGE_KEY: &str = "vela.device.presentation.v1";
/// Pre-0.13 theme key. Read once, migrated into [`STORAGE_KEY`], then removed.
pub const LEGACY_THEME_KEY: &str = "vela.theme";
/// Document version understood by this build.
pub const DOCUMENT_VERSION: u32 = 1;
/// Lowest selectable app size, in percent.
pub const SCALE_MIN: u16 = 80;
/// Highest selectable app size, in percent.
pub const SCALE_MAX: u16 = 150;
/// Step between selectable app sizes, in percent.
pub const SCALE_STEP: u16 = 5;
/// Default app size, in percent. Follows the platform's own scaling.
pub const SCALE_DEFAULT: u16 = 100;

/// Appearance theme preference. `System` resolves through the documented browser
/// `prefers-color-scheme` signal and never through heuristics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    System,
    Dark,
    Light,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

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
        // Dark stays the shipped default appearance; `System` is an available,
        // honoured choice rather than a silent visual change for existing users.
        Self {
            app_scale_percent: SCALE_DEFAULT,
            theme: Theme::Dark,
        }
    }
}

/// Why stored presentation values could not be used verbatim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repair {
    /// The stored value was not a JSON object with this contract's shape.
    Unreadable,
    /// The stored document belongs to a version this build does not understand.
    UnsupportedVersion,
    /// Some fields were outside the documented contract and were replaced.
    InvalidValues,
}

impl Repair {
    /// Non-blocking explanation shown next to the controls.
    pub fn message(self) -> &'static str {
        match self {
            Self::Unreadable => {
                "Saved appearance settings on this device could not be read, so Vela started with the defaults. Choosing a value here replaces the unreadable copy."
            }
            Self::UnsupportedVersion => {
                "Appearance settings on this device were saved by a newer Vela version, so this build used the defaults and left them untouched. Choosing a value here replaces that copy."
            }
            Self::InvalidValues => {
                "Part of the saved appearance settings on this device was outside the supported range, so Vela used the defaults for those values."
            }
        }
    }
}

/// Result of resolving the device store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loaded {
    pub preferences: Preferences,
    /// Set when stored values had to be repaired; the UI explains it once.
    pub repair: Option<Repair>,
    /// Set when a valid legacy theme value still has to be written to the new key.
    pub migrated: bool,
}

/// Whether `percent` is a selectable app size: integer, 80..=150, step 5.
pub fn scale_is_valid(percent: u16) -> bool {
    (SCALE_MIN..=SCALE_MAX).contains(&percent) && (percent - SCALE_MIN).is_multiple_of(SCALE_STEP)
}

fn scale_from_number(value: Option<&Value>) -> Option<u16> {
    let number = value?.as_i64()?;
    let percent = u16::try_from(number).ok()?;
    scale_is_valid(percent).then_some(percent)
}

/// Serialise the bounded presentation document. Field order is deterministic and
/// only presentation fields are written.
pub fn document(preferences: Preferences) -> String {
    serde_json::json!({
        "app_scale_percent": preferences.app_scale_percent,
        "theme": preferences.theme.as_str(),
        "version": DOCUMENT_VERSION,
    })
    .to_string()
}

/// Validate a stored document without touching storage.
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

/// Resolve the device store from the current document and the legacy theme key.
///
/// Pure: it performs no I/O so startup behaviour is unit-testable.
pub fn resolve(stored: Option<&str>, legacy_theme: Option<&str>) -> Loaded {
    let legacy = legacy_theme.and_then(Theme::parse);
    if let Some(raw) = stored {
        match parse_document(raw) {
            Ok((preferences, repair)) => {
                return Loaded {
                    preferences,
                    repair,
                    migrated: false,
                };
            }
            Err(repair) => {
                // The stored document is unusable. Keep a valid legacy choice if
                // one exists so a corrupt write cannot discard the user's theme.
                return Loaded {
                    preferences: Preferences {
                        theme: legacy.unwrap_or(Preferences::default().theme),
                        ..Preferences::default()
                    },
                    repair: Some(repair),
                    migrated: legacy.is_some(),
                };
            }
        }
    }
    match legacy {
        Some(theme) => Loaded {
            preferences: Preferences {
                theme,
                ..Preferences::default()
            },
            repair: None,
            migrated: true,
        },
        None => Loaded {
            preferences: Preferences::default(),
            repair: None,
            migrated: false,
        },
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn storage_get(key: &str) -> Option<String> {
    storage()?.get_item(key).ok()?
}

fn root_element() -> Option<web_sys::Element> {
    web_sys::window()?.document()?.document_element()
}

/// Read the device store. A denied/absent storage area yields defaults.
pub fn load() -> Loaded {
    resolve(
        storage_get(STORAGE_KEY).as_deref(),
        storage_get(LEGACY_THEME_KEY).as_deref(),
    )
}

/// Write the presentation document. Returns whether the device accepted it.
///
/// A failure must not reset the user's live choice; the caller reports that the
/// value is applied for this session only.
pub fn save(preferences: Preferences) -> bool {
    let Some(store) = storage() else {
        return false;
    };
    if store.set_item(STORAGE_KEY, &document(preferences)).is_err() {
        return false;
    }
    // Only drop the legacy key once the new document is durable.
    let _ = store.remove_item(LEGACY_THEME_KEY);
    true
}

/// Resolve `System` through the browser's documented media signal.
pub fn effective_theme(preferences: Preferences) -> &'static str {
    match preferences.theme {
        Theme::System if system_prefers_light() => "light",
        Theme::System => "dark",
        Theme::Dark => "dark",
        Theme::Light => "light",
    }
}

/// Whether the browser currently reports a light color scheme.
pub fn system_prefers_light() -> bool {
    media_query_matches("(prefers-color-scheme: light)").unwrap_or(false)
}

fn media_query_matches(query: &str) -> Option<bool> {
    let window = web_sys::window()?;
    let match_media = js_sys::Reflect::get(&window, &JsValue::from_str("matchMedia")).ok()?;
    let match_media = match_media.dyn_into::<js_sys::Function>().ok()?;
    let list = match_media.call1(&window, &JsValue::from_str(query)).ok()?;
    js_sys::Reflect::get(&list, &JsValue::from_str("matches"))
        .ok()?
        .as_bool()
}

/// Whether this browser implements the CSS `zoom` the adapter relies on.
///
/// An inconclusive probe reports support so an unusual engine still gets the
/// preference; only a definite "unsupported" disables the control.
pub fn zoom_supported() -> bool {
    let global = js_sys::global();
    let Ok(css) = js_sys::Reflect::get(&global, &JsValue::from_str("CSS")) else {
        return true;
    };
    let Ok(supports) = js_sys::Reflect::get(&css, &JsValue::from_str("supports")) else {
        return true;
    };
    let Ok(supports) = supports.dyn_into::<js_sys::Function>() else {
        return true;
    };
    match supports.call2(&css, &JsValue::from_str("zoom"), &JsValue::from_str("1.25")) {
        Ok(result) => result.as_bool().unwrap_or(true),
        Err(_) => true,
    }
}

/// Apply the validated preference to the document exactly once.
///
/// `--app-zoom` drives the single CSS `zoom` declaration in `style.css`, so the
/// percentage is never stacked with a second scale. The theme attribute keeps the
/// pre-existing `data-theme` contract. Neither touches the media element, the
/// queue or any request.
pub fn apply(preferences: Preferences) {
    let Some(root) = root_element() else {
        return;
    };
    let style = format!(
        "--app-zoom:{:.2}",
        f64::from(preferences.app_scale_percent) / 100.0
    );
    if root.get_attribute("style").as_deref() != Some(style.as_str()) {
        let _ = root.set_attribute("style", &style);
    }
    let theme = effective_theme(preferences);
    if root.get_attribute("data-theme").as_deref() != Some(theme) {
        let _ = root.set_attribute("data-theme", theme);
    }
}

/// Load, apply and migrate before the first visible frame.
///
/// Called from `main` so a saved size and theme are in place before Dioxus
/// renders, including while signed out. A storage failure is silent here; the
/// Settings panel reports the temporary state if the user changes something.
pub fn boot() {
    let loaded = load();
    apply(loaded.preferences);
    if loaded.migrated && save(loaded.preferences) {
        // The legacy key is gone and the versioned document is authoritative.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_bounds_and_steps_are_exact() {
        for percent in [80_u16, 85, 100, 125, 150] {
            assert!(scale_is_valid(percent));
        }
        for percent in [0_u16, 79, 83, 151, 200, 1000] {
            assert!(!scale_is_valid(percent), "{percent} must be rejected");
        }
        assert_eq!(SCALE_DEFAULT, 100);
        assert!(scale_is_valid(SCALE_DEFAULT));
    }

    #[test]
    fn document_round_trips_without_private_fields() {
        let preferences = Preferences {
            app_scale_percent: 125,
            theme: Theme::System,
        };
        let text = document(preferences);
        assert_eq!(
            text,
            r#"{"app_scale_percent":125,"theme":"system","version":1}"#
        );
        assert_eq!(parse_document(&text), Ok((preferences, None)));
        for forbidden in ["csrf", "token", "cookie", "password", "queue", "artist"] {
            assert!(
                !text.to_ascii_lowercase().contains(forbidden),
                "device document must not contain {forbidden}"
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
        ] {
            assert!(parse_document(raw).is_err(), "{raw} must be unreadable");
        }
        assert_eq!(
            parse_document(r#"{"version":2,"app_scale_percent":200,"theme":"dark"}"#),
            Err(Repair::UnsupportedVersion)
        );
        assert_eq!(
            parse_document(r#"{"version":0,"app_scale_percent":100,"theme":"dark"}"#),
            Err(Repair::UnsupportedVersion)
        );
    }

    #[test]
    fn out_of_contract_values_fall_back_field_by_field() {
        assert_eq!(
            parse_document(r#"{"version":1,"app_scale_percent":175,"theme":"light"}"#),
            Ok((
                Preferences {
                    app_scale_percent: 100,
                    theme: Theme::Light,
                },
                Some(Repair::InvalidValues)
            ))
        );
        assert_eq!(
            parse_document(r#"{"version":1,"app_scale_percent":83,"theme":"dark"}"#),
            Ok((
                Preferences {
                    app_scale_percent: 100,
                    theme: Theme::Dark,
                },
                Some(Repair::InvalidValues)
            ))
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
            Ok((
                Preferences {
                    app_scale_percent: 125,
                    theme: Theme::Dark,
                },
                Some(Repair::InvalidValues)
            ))
        );
        assert_eq!(
            parse_document(r#"{"version":1,"app_scale_percent":125,"theme":"light"}"#),
            Ok((
                Preferences {
                    app_scale_percent: 125,
                    theme: Theme::Light,
                },
                None
            ))
        );
    }

    #[test]
    fn legacy_theme_migrates_once_and_the_stored_document_wins() {
        let fresh = resolve(None, None);
        assert_eq!(
            fresh,
            Loaded {
                preferences: Preferences::default(),
                repair: None,
                migrated: false,
            }
        );
        let migrated = resolve(None, Some("light"));
        assert_eq!(migrated.preferences.theme, Theme::Light);
        assert_eq!(migrated.preferences.app_scale_percent, 100);
        assert!(migrated.migrated);
        let stored = document(Preferences {
            app_scale_percent: 90,
            theme: Theme::Dark,
        });
        let stored = resolve(Some(&stored), Some("light"));
        assert_eq!(stored.preferences.app_scale_percent, 90);
        assert_eq!(stored.preferences.theme, Theme::Dark);
        assert!(!stored.migrated);
        let unknown_legacy = resolve(None, Some("sepia"));
        assert_eq!(unknown_legacy.preferences, Preferences::default());
        assert!(!unknown_legacy.migrated);
    }

    #[test]
    fn corrupt_storage_falls_back_safely_and_keeps_a_valid_legacy_theme() {
        let corrupt = resolve(Some("{not json"), None);
        assert_eq!(corrupt.preferences, Preferences::default());
        assert_eq!(corrupt.repair, Some(Repair::Unreadable));
        assert!(!corrupt.migrated);
        let both = resolve(Some("[1,2,3]"), Some("light"));
        assert_eq!(both.preferences.theme, Theme::Light);
        assert_eq!(both.repair, Some(Repair::Unreadable));
        assert!(both.migrated);
        let future = resolve(Some(r#"{"version":9,"app_scale_percent":150}"#), None);
        assert_eq!(future.preferences, Preferences::default());
        assert_eq!(future.repair, Some(Repair::UnsupportedVersion));
        assert!(!future.migrated);
    }

    #[test]
    fn theme_names_round_trip() {
        for theme in [Theme::System, Theme::Dark, Theme::Light] {
            assert_eq!(Theme::parse(theme.as_str()), Some(theme));
        }
        assert_eq!(Theme::parse("System"), None);
        assert_eq!(Theme::parse(""), None);
    }
}
