//! Keep multipart headers and parser read-ahead bounded before Axum/Multer consumes them.
use crate::HttpError;
use axum::{
    body::Body,
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use std::io;

const HEADER_LIMIT: usize = 8 * 1024;
const FRAME_LIMIT: usize = 16 * 1024;

#[derive(Default)]
enum Phase {
    #[default]
    Opening,
    Headers,
    Data,
    Closing,
    Complete,
}
struct Guard {
    phase: Phase,
    buffer: Vec<u8>,
    opening: Vec<u8>,
    marker: Vec<u8>,
    trailer_bytes: usize,
}
impl Guard {
    fn new(boundary: &str) -> Self {
        Self {
            phase: Phase::Opening,
            buffer: Vec::new(),
            opening: format!("--{boundary}\r\n").into_bytes(),
            marker: format!("\r\n--{boundary}").into_bytes(),
            trailer_bytes: 0,
        }
    }
    fn check(&mut self, data: &[u8]) -> io::Result<()> {
        let bad = || {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Multipart headers exceeded limits or more than one file was provided",
            )
        };
        match self.phase {
            Phase::Opening => {
                self.buffer.extend_from_slice(data);
                let compare = self.buffer.len().min(self.opening.len());
                if self.buffer[..compare] != self.opening[..compare] {
                    return Err(bad());
                }
                if self.buffer.len() >= self.opening.len() {
                    let tail = self.buffer.split_off(self.opening.len());
                    self.buffer.clear();
                    self.phase = Phase::Headers;
                    self.check(&tail)?;
                }
            }
            Phase::Headers => {
                self.buffer.extend_from_slice(data);
                if let Some(end) = self.buffer.windows(4).position(|v| v == b"\r\n\r\n") {
                    if end + 4 > HEADER_LIMIT {
                        return Err(bad());
                    }
                    let body = self.buffer.split_off(end + 4);
                    self.buffer.clear();
                    self.phase = Phase::Data;
                    self.check(&body)?;
                } else if self.buffer.len() > HEADER_LIMIT {
                    return Err(bad());
                }
            }
            Phase::Data => {
                self.buffer.extend_from_slice(data);
                if let Some(end) = self
                    .buffer
                    .windows(self.marker.len())
                    .position(|v| v == self.marker)
                {
                    let tail = self.buffer.split_off(end + self.marker.len());
                    self.buffer.clear();
                    self.phase = Phase::Closing;
                    self.check(&tail)?;
                } else {
                    let keep = self.marker.len().saturating_sub(1);
                    if self.buffer.len() > keep {
                        self.buffer.drain(..self.buffer.len() - keep);
                    }
                }
            }
            Phase::Closing => {
                self.buffer.extend_from_slice(data);
                if self.buffer.len() >= 2 {
                    if !self.buffer.starts_with(b"--") {
                        return Err(bad());
                    }
                    self.trailer_bytes = self.buffer.len() - 2;
                    self.buffer.clear();
                    self.phase = Phase::Complete;
                    if self.trailer_bytes > HEADER_LIMIT {
                        return Err(bad());
                    }
                }
            }
            Phase::Complete => {
                self.trailer_bytes = self.trailer_bytes.saturating_add(data.len());
                if self.trailer_bytes > HEADER_LIMIT {
                    return Err(bad());
                }
            }
        }
        Ok(())
    }
}

pub(crate) async fn bounded_multipart(request: Request, next: Next) -> Response {
    let boundary = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| multer::parse_boundary(v).ok());
    let Some(boundary) = boundary.filter(|v| !v.is_empty() && v.len() <= 70) else {
        return HttpError(StatusCode::BAD_REQUEST, "Invalid multipart boundary".into())
            .into_response();
    };
    let (parts, body) = request.into_parts();
    let stream = body.into_data_stream();
    let state = (
        stream,
        axum::body::Bytes::new(),
        Guard::new(&boundary),
        false,
    );
    let guarded = futures_util::stream::unfold(
        state,
        |(mut input, mut pending, mut guard, halted)| async move {
            if halted {
                return None;
            }
            // Multer eagerly drains ready frames; force a scheduling boundary so it
            // cannot accumulate the full upload before yielding a field chunk.
            tokio::task::yield_now().await;
            if pending.is_empty() {
                match input.next().await {
                    Some(Ok(chunk)) => pending = chunk,
                    Some(Err(_)) => {
                        return Some((
                            Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "Upload interrupted",
                            )),
                            (input, pending, guard, true),
                        ));
                    }
                    None => return None,
                }
            }
            let chunk = pending.split_to(pending.len().min(FRAME_LIMIT));
            match guard.check(&chunk) {
                Ok(()) => Some((Ok(chunk), (input, pending, guard, false))),
                Err(error) => Some((Err(error), (input, pending, guard, true))),
            }
        },
    );
    next.run(Request::from_parts(parts, Body::from_stream(guarded)))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn headers_and_closing_boundary_can_span_frames() {
        let mut guard = Guard::new("test");
        for bytes in [
            b"--test\r\nContent-Disposition: form-data; name=file\r\n\r".as_slice(),
            b"\naudio\r\n--tes",
            b"t-",
            b"-\r\n",
        ] {
            guard.check(bytes).unwrap();
        }
        assert!(matches!(guard.phase, Phase::Complete));
    }
    #[test]
    fn rejects_large_headers_and_extra_fields_before_header_parsing() {
        let mut guard = Guard::new("test");
        guard.check(b"--test\r\n").unwrap();
        assert!(guard.check(&vec![b'x'; HEADER_LIMIT + 1]).is_err());
        let mut guard = Guard::new("test");
        guard
            .check(b"--test\r\nName: file\r\n\r\naudio\r\n--test")
            .unwrap();
        assert!(
            guard
                .check(b"\r\nContent-Disposition: form-data; name=second")
                .is_err()
        );
    }
    #[test]
    fn opening_boundary_is_required_before_any_header_terminator() {
        let mut guard = Guard::new("test");
        assert!(
            guard
                .check(b"preamble\r\n\r\nx--test\r\nName: file\r\n")
                .is_err()
        );
        let mut guard = Guard::new("test");
        for bytes in [
            b"-".as_slice(),
            b"-te",
            b"st\r",
            b"\nName: file\r\n\r\naudio\r\n--test--\r\n",
        ] {
            guard.check(bytes).unwrap();
        }
        assert!(matches!(guard.phase, Phase::Complete));
    }
}
