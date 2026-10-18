//! Opt-in saved native session token in the freedesktop Secret Service.
//!
//! Only the opaque `vela_session` token and origin are stored — never the
//! password, username or CSRF token. The secret travels over stdin to
//! `secret-tool`, not through process arguments or files. The item is deleted on
//! explicit sign-out. Release uses a fixed service name; QA may override it only
//! in debug builds.
use crate::client::canonical_origin;
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    process::{Command, Stdio},
};

const DEFAULT_SERVICE: &str = "org.vela.music.desktop.session";
const MAX_SECRET: usize = 2048;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedSession {
    pub origin: String,
    pub token: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u8,
    origin: String,
    token: String,
}

fn service() -> String {
    #[cfg(debug_assertions)]
    if let Ok(service) = std::env::var("VELA_NATIVE_KEYRING_SERVICE")
        && !service.is_empty()
        && service.len() <= 128
        && service
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character))
    {
        return service;
    }
    DEFAULT_SERVICE.to_owned()
}

fn validate(origin: &str, token: &str) -> Result<(String, String), String> {
    let origin = canonical_origin(origin)?.as_str().to_owned();
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("The saved session is invalid. Sign in again.".into());
    }
    Ok((origin, token.to_ascii_lowercase()))
}

pub fn load() -> Result<Option<SavedSession>, String> {
    let output = Command::new("secret-tool")
        .args(["lookup", "service", &service()])
        .output()
        .map_err(|_| "Saved-session storage is unavailable on this device.".to_owned())?;
    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }
    if output.stdout.len() > MAX_SECRET {
        return Err("The saved session is invalid. Sign in again.".into());
    }
    let envelope: Envelope = serde_json::from_slice(&output.stdout)
        .map_err(|_| "The saved session is invalid. Sign in again.".to_owned())?;
    if envelope.version != 1 {
        return Err("The saved session uses an unsupported version.".into());
    }
    let (origin, token) = validate(&envelope.origin, &envelope.token)?;
    Ok(Some(SavedSession { origin, token }))
}

pub fn save(origin: &str, token: &str) -> Result<(), String> {
    let (origin, token) = validate(origin, token)?;
    let payload = serde_json::to_vec(&Envelope {
        version: 1,
        origin,
        token,
    })
    .map_err(|_| "The session could not be saved.".to_owned())?;
    // Store replaces any previous QA/application item for this attribute.
    let mut child = Command::new("secret-tool")
        .args([
            "store",
            "--label=Vela Music saved session",
            "service",
            &service(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Saved-session storage is unavailable on this device.".to_owned())?;
    child
        .stdin
        .take()
        .ok_or_else(|| "The session could not be saved.".to_owned())?
        .write_all(&payload)
        .map_err(|_| "The session could not be saved.".to_owned())?;
    let status = child
        .wait()
        .map_err(|_| "The session could not be saved.".to_owned())?;
    if !status.success() {
        return Err("The session could not be saved to the system keyring.".into());
    }
    Ok(())
}

pub fn clear() -> Result<(), String> {
    // Clearing a missing item is not an error.
    let _ = Command::new("secret-tool")
        .args(["clear", "service", &service()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_session_requires_canonical_origin_and_opaque_token() {
        let token = "a".repeat(64);
        assert_eq!(
            validate("https://music.example.org", &token).unwrap(),
            ("https://music.example.org/".to_owned(), token.clone())
        );
        assert!(validate("http://music.example.org", &token).is_err());
        assert!(validate("https://music.example.org", "secret").is_err());
        assert!(validate("https://music.example.org", &"g".repeat(64)).is_err());
    }
}
