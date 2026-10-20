//! Typed same-origin HTTP transport; cookies remain browser managed.
use gloo_net::http::{Request, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{cell::RefCell, rc::Rc};
use vela_domain::ApiError;
use web_sys::{AbortController, AbortSignal};

/// One request group per mounted page. Replacing it aborts obsolete HTTP work.
#[derive(Clone, Default)]
pub(crate) struct RequestScope(Rc<RefCell<Option<AbortController>>>);
impl RequestScope {
    /// Capture the exact request for a deadline; replacing the group must not
    /// let an older timer abort its successor.
    pub fn controller(&self) -> Option<AbortController> {
        self.0.borrow().clone()
    }
    pub fn begin(&self) -> Result<AbortSignal, String> {
        self.cancel();
        let controller = AbortController::new().map_err(|_| {
            "Your browser could not start this request. Reload the page.".to_string()
        })?;
        let signal = controller.signal();
        *self.0.borrow_mut() = Some(controller);
        Ok(signal)
    }
    pub fn cancel(&self) {
        if let Some(controller) = self.0.borrow_mut().take() {
            controller.abort();
        }
    }
}

pub(crate) async fn get<T: DeserializeOwned>(
    path: &str,
    signal: &AbortSignal,
) -> Result<T, String> {
    let response = Request::get(path)
        .abort_signal(Some(signal))
        .send()
        .await
        .map_err(|_| "Connection interrupted. Check that your server is available.".to_string())?;
    decode(response).await
}

pub(crate) async fn api<T: DeserializeOwned>(
    method: &str,
    path: &str,
    body: Option<Value>,
    csrf: &str,
) -> Result<T, String> {
    api_abortable(method, path, body, csrf, None).await
}

pub(crate) async fn api_abortable<T: DeserializeOwned>(
    method: &str,
    path: &str,
    body: Option<Value>,
    csrf: &str,
    signal: Option<&AbortSignal>,
) -> Result<T, String> {
    api_classified(method, path, body, csrf, signal)
        .await
        .map_err(|failure| failure.message)
}

/// Transport uncertainty differs from a rejected mutation: a lost response may
/// follow a committed write. Keep the HTTP status without parsing user copy.
pub(crate) struct ApiFailure {
    pub status: Option<u16>,
    pub message: String,
}

impl ApiFailure {
    fn transport(message: &str) -> Self {
        Self {
            status: None,
            message: message.into(),
        }
    }

    pub fn revision_conflict(&self) -> bool {
        // This add endpoint reports revision conflicts before its transaction
        // commits. Other errors can originate in its post-commit detail read;
        // never infer rollback from a generic HTTP failure.
        self.status == Some(409)
    }
}

pub(crate) async fn api_classified<T: DeserializeOwned>(
    method: &str,
    path: &str,
    body: Option<Value>,
    csrf: &str,
    signal: Option<&AbortSignal>,
) -> Result<T, ApiFailure> {
    let request = match method {
        "POST" => Request::post(path),
        "PUT" => Request::put(path),
        "PATCH" => Request::patch(path),
        "DELETE" => Request::delete(path),
        _ => Request::get(path),
    };
    let request = request.abort_signal(signal);
    let request = if csrf.is_empty() {
        request
    } else {
        request.header("X-CSRF-Token", csrf)
    };
    let response = if let Some(body) = body {
        request
            .json(&body)
            .map_err(|_| ApiFailure::transport("Could not prepare the request."))?
            .send()
            .await
    } else {
        request.send().await
    }
    .map_err(|_| {
        ApiFailure::transport("Connection interrupted. Check that your server is available.")
    })?;
    decode_classified(response).await
}

pub(crate) async fn decode<T: DeserializeOwned>(response: Response) -> Result<T, String> {
    decode_classified(response)
        .await
        .map_err(|failure| failure.message)
}

async fn decode_classified<T: DeserializeOwned>(response: Response) -> Result<T, ApiFailure> {
    if !response.ok() {
        let status = response.status();
        let message = response
            .json::<ApiError>()
            .await
            .map(|e| e.error)
            .unwrap_or_else(|_| format!("The server could not complete the request ({status})."));
        return Err(ApiFailure {
            status: Some(status),
            message,
        });
    }
    response
        .json()
        .await
        .map_err(|_| ApiFailure::transport("The server returned an unreadable response."))
}

pub(crate) async fn mutate(
    method: &str,
    path: &str,
    body: Option<Value>,
    csrf: &str,
) -> Result<(), String> {
    let request = match method {
        "DELETE" => Request::delete(path),
        "PUT" => Request::put(path),
        "PATCH" => Request::patch(path),
        _ => Request::post(path),
    }
    .header("X-CSRF-Token", csrf);
    let response = if let Some(body) = body {
        request
            .json(&body)
            .map_err(|_| "Could not prepare the request.".to_string())?
            .send()
            .await
    } else {
        request.send().await
    }
    .map_err(|_| "Connection interrupted. Please try again.".to_string())?;
    if response.ok() {
        Ok(())
    } else {
        Err(response
            .json::<ApiError>()
            .await
            .map(|e| e.error)
            .unwrap_or_else(|_| "The request was refused.".to_string()))
    }
}
