//! Source adapters return downloadable assets; all network operations use controlled egress.
use crate::{ControlledHttp, ImportError};
use percent_encoding::percent_decode_str;
use reqwest::{
    Method, Response, StatusCode,
    header::{CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE},
};
use std::future::Future;
use url::Url;

#[derive(Debug, Clone)]
pub struct ResolvedAsset {
    pub url: Url,
    pub filename: String,
    pub content_type: Option<String>,
    pub size_bytes: Option<u64>,
}

/// Compile-time adapter contract. Trusted-storage adapters must use a separate egress policy.
pub trait ImportSource: Send + Sync {
    fn name(&self) -> &'static str;
    fn recognizes(&self, url: &Url) -> bool;
    fn resolve(
        &self,
        http: &ControlledHttp,
        url: Url,
    ) -> impl Future<Output = Result<Vec<ResolvedAsset>, ImportError>> + Send;
}

#[derive(Clone, Copy, Default)]
pub struct DirectAudioSource;

impl ImportSource for DirectAudioSource {
    fn name(&self) -> &'static str {
        "Direct audio download"
    }
    fn recognizes(&self, url: &Url) -> bool {
        matches!(url.scheme(), "http" | "https")
    }

    async fn resolve(
        &self,
        http: &ControlledHttp,
        url: Url,
    ) -> Result<Vec<ResolvedAsset>, ImportError> {
        let mut response = http.request(url.clone(), Method::HEAD).await?;
        if matches!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_IMPLEMENTED
        ) {
            // Drop immediately after headers. Servers without HEAD need not have their whole file read.
            response = http.request(url.clone(), Method::GET).await?;
        }
        check_status(response.status())?;
        let filename = filename(response.url());
        let (content_type, size_bytes) = inspect_response(&response, &filename)?;
        Ok(vec![ResolvedAsset {
            url,
            filename,
            content_type,
            size_bytes,
        }])
    }
}

pub(crate) fn check_status(status: StatusCode) -> Result<(), ImportError> {
    if status == StatusCode::OK {
        Ok(())
    } else if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
        Err(ImportError::Transient(
            "Remote server is temporarily unavailable",
        ))
    } else {
        Err(ImportError::Http(status.as_u16()))
    }
}

pub(crate) fn filename(url: &Url) -> String {
    let raw = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .unwrap_or("");
    let decoded = percent_decode_str(raw).decode_utf8_lossy();
    let clean: String = decoded
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '/' | '\\'))
        .take(160)
        .collect();
    if clean.is_empty() || matches!(clean.as_str(), "." | "..") {
        "downloaded-audio".to_string()
    } else {
        clean
    }
}

pub(crate) fn inspect_response(
    response: &Response,
    filename: &str,
) -> Result<(Option<String>, Option<u64>), ImportError> {
    if response
        .headers()
        .get(CONTENT_ENCODING)
        .is_some_and(|v| v.as_bytes() != b"identity")
    {
        return Err(ImportError::Policy(
            "Compressed HTTP responses are not supported",
        ));
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            v.split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase()
        });
    let extension = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let supported_extension = matches!(
        extension.as_str(),
        "flac"
            | "mp3"
            | "aac"
            | "m4a"
            | "mp4"
            | "opus"
            | "ogg"
            | "oga"
            | "wav"
            | "aiff"
            | "aif"
            | "ape"
            | "wv"
    );
    if !acceptable_type(content_type.as_deref(), supported_extension) {
        return Err(ImportError::Policy(
            "URL does not advertise a supported downloadable audio file",
        ));
    }
    let size = response
        .headers()
        .get(CONTENT_LENGTH)
        .map(|v| {
            v.to_str()
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .ok_or(ImportError::Policy("Invalid remote file size"))
        })
        .transpose()?;
    if size == Some(0) {
        return Err(ImportError::Policy("Remote audio file is empty"));
    }
    Ok((content_type, size))
}

fn acceptable_type(content_type: Option<&str>, supported_extension: bool) -> bool {
    match content_type {
        Some(value) if value.starts_with("audio/") => {
            !matches!(value, "audio/x-mpegurl" | "audio/mpegurl" | "audio/x-scpls")
        }
        Some("application/ogg" | "video/mp4") => true,
        Some("application/octet-stream" | "binary/octet-stream") | None => supported_extension,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filenames_are_display_values_and_not_paths() {
        assert_eq!(
            filename(&Url::parse("https://example.com/a%2fb%5cc%00.flac").unwrap()),
            "abc.flac"
        );
        assert_eq!(
            filename(&Url::parse("https://example.com/").unwrap()),
            "downloaded-audio"
        );
        assert!(!acceptable_type(Some("text/html"), true));
        assert!(!acceptable_type(Some("audio/x-mpegurl"), false));
        assert!(acceptable_type(Some("audio/flac"), false));
        assert!(acceptable_type(Some("application/octet-stream"), true));
        assert!(!acceptable_type(None, false));
        assert!(check_status(StatusCode::PARTIAL_CONTENT).is_err());
    }
}
