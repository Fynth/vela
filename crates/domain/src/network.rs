//! Explicit publication policy and bounded read-only catalog interoperability.
use crate::Track;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkSettings {
    pub server_id: String,
    pub enabled: bool,
    pub origin: Option<String>,
    pub name: String,
    pub revision: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateNetworkSettings {
    pub revision: i64,
    pub enabled: bool,
    pub origin: String,
    pub name: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PublicLicense {
    #[serde(rename = "CC0-1.0")]
    Cc0,
    #[serde(rename = "CC-BY-4.0")]
    Attribution,
    #[serde(rename = "CC-BY-SA-4.0")]
    ShareAlike,
}
impl PublicLicense {
    pub fn url(self) -> &'static str {
        match self {
            Self::Cc0 => "https://creativecommons.org/publicdomain/zero/1.0/",
            Self::Attribution => "https://creativecommons.org/licenses/by/4.0/",
            Self::ShareAlike => "https://creativecommons.org/licenses/by-sa/4.0/",
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::Cc0 => "CC0-1.0",
            Self::Attribution => "CC-BY-4.0",
            Self::ShareAlike => "CC-BY-SA-4.0",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatePublication {
    pub track_id: String,
    pub license: PublicLicense,
    pub attribution: String,
    pub rights_evidence: String,
    pub allow_stream: bool,
    pub allow_download: bool,
    pub rights_confirmed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokePublication {
    pub revision: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publication {
    pub id: String,
    pub track: Track,
    pub license: PublicLicense,
    pub attribution: String,
    pub rights_evidence: String,
    pub allow_stream: bool,
    pub allow_download: bool,
    pub revision: i64,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PublicationQuery {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub q: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationPage {
    pub items: Vec<Publication>,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicTrack {
    pub id: String,
    pub revision: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: Option<i64>,
    pub duration_ms: i64,
    pub codec: String,
    pub license: PublicLicense,
    pub attribution: String,
    pub can_stream: bool,
    pub can_download: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicCatalog {
    pub server_id: String,
    pub origin: String,
    pub items: Vec<PublicTrack>,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCapabilities {
    pub protocol_version: u8,
    pub server_id: String,
    pub origin: String,
    pub name: String,
    pub software_version: String,
    pub public_artwork: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewPeer {
    pub origin: String,
    #[serde(default)]
    pub cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerPreview {
    pub capabilities: NetworkCapabilities,
    pub catalog: PublicCatalog,
}

/// Canonical HTTPS DNS origin for this public catalog profile. Resolution and
/// public-unicast pinning are still mandatory at the outbound transport boundary.
pub fn canonical_public_origin(input: &str) -> Result<String, &'static str> {
    const ERROR: &str = "Use an HTTPS DNS origin without credentials, path, query or fragment";
    if input.is_empty()
        || input.len() > 2048
        || input.trim() != input
        || input.chars().any(|c| c.is_control() || c.is_whitespace())
        || input.contains('\\')
        || !input
            .split_once("://")
            .is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case("https"))
        || input.split_once("://").is_some_and(|(_, rest)| {
            rest.split(['/', '?', '#'])
                .next()
                .is_some_and(|a| a.contains('@'))
        })
    {
        return Err(ERROR);
    }
    let url = url::Url::parse(input).map_err(|_| ERROR)?;
    let Some(url::Host::Domain(host)) = url.host() else {
        return Err(ERROR);
    };
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port_or_known_default() != Some(443)
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !host.contains('.')
        || ["localhost", "local", "internal", "lan", "home.arpa"]
            .iter()
            .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || !label.as_bytes()[0].is_ascii_alphanumeric()
                || !label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                || !label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err(ERROR);
    }
    Ok(url.origin().ascii_serialization())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_origin_is_explicit_https_dns_without_ambient_access() {
        assert_eq!(
            canonical_public_origin("https://Music.Example.org:443/").unwrap(),
            "https://music.example.org"
        );
        for bad in [
            "http://music.example.org",
            "https://localhost",
            "https://home.local",
            "https://x.internal",
            "https://127.0.0.1",
            "https://169.254.169.254",
            "https://2130706433",
            "https://[::1]",
            "https://@music.example.org",
            "https://music.example.org/path",
            "https://music.example.org?q=x",
            "https://music.example.org#x",
            "https://music.example.org:8443",
            " https://music.example.org",
            "https://-x.example.org",
            "https:music.example.org",
            "https:\\music.example.org",
            "https://music.example.org/ /",
        ] {
            assert!(canonical_public_origin(bad).is_err(), "{bad}");
        }
    }
}
