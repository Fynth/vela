//! Opt-in saved native session token in the freedesktop Secret Service.
//!
//! Only the opaque `vela_session` token and origin are stored — never the
//! password, username or CSRF token. The secret travels over stdin to
//! `secret-tool`, not through process arguments or files. The item is deleted on
//! explicit sign-out. Release uses a fixed service name; QA may override it only
//! in debug builds.
//!
//! `secret-tool` is an external helper whose Secret Service backend can hang, so
//! this module never trusts it: every invocation is byte-bounded, deadline
//! bounded (kill and reap), and serialized against every other invocation in
//! this process. A login attempt that was superseded can no longer apply a
//! pending write (AUD-01, AUD-02, AUD-11).
use crate::client::canonical_origin;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError},
    },
    time::{Duration, Instant},
};

const DEFAULT_SERVICE: &str = "org.vela.music.desktop.session";
/// Upper bound for the whole stored envelope, enforced before JSON allocation.
const MAX_SECRET: usize = 2048;
const READ_CHUNK: usize = 512;
const POLL_INTERVAL: Duration = Duration::from_millis(5);
/// One external helper invocation may not stall its caller forever.
const OPERATION_DEADLINE: Duration = Duration::from_secs(5);

/// A saved login. The token is secret: any diagnostic output must stay redacted.
#[derive(Clone, PartialEq, Eq)]
pub struct SavedSession {
    pub origin: String,
    pub token: String,
}

impl std::fmt::Debug for SavedSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SavedSession")
            .field("origin", &self.origin)
            .field("token", &"<redacted>")
            .finish()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u8,
    origin: String,
    token: String,
}

/// Typed keyring failure. Every variant carries a fixed user-facing message and
/// no variant can ever include the stored token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyringError {
    /// The helper could not be started, or one of its pipes failed.
    Unavailable,
    /// The stored item is not a valid Vela envelope, or exceeds the byte bound.
    InvalidItem,
    /// The stored item declares a version this build does not understand.
    UnsupportedVersion,
    /// The helper ran but did not store the item.
    StoreFailed,
    /// An item was still readable after a delete attempt.
    ClearUnconfirmed,
    /// The helper outlived its deadline and was killed and reaped.
    Deadline,
}

impl KeyringError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Unavailable => "Saved-session storage is unavailable on this device.",
            Self::InvalidItem => "The saved session is invalid. Sign in again.",
            Self::UnsupportedVersion => "The saved session uses an unsupported version.",
            Self::StoreFailed => "The session could not be saved to the system keyring.",
            Self::ClearUnconfirmed => {
                "The saved session could not be removed from the system keyring."
            }
            Self::Deadline => "The system keyring did not respond in time. Try again.",
        }
    }

    /// Only a malformed record may be cleaned up automatically. A timeout, a
    /// missing helper or a failed spawn says nothing at all about the stored
    /// token, so that item must be preserved.
    pub fn is_corrupt_record(self) -> bool {
        matches!(self, Self::InvalidItem | Self::UnsupportedVersion)
    }
}

impl std::fmt::Display for KeyringError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message())
    }
}

/// The result of a guarded save. A superseded attempt must never touch an item
/// that a newer login attempt already owns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreStatus {
    Stored,
    Superseded,
}

/// One login/resume attempt's claim on the saved item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attempt(u64);

impl Attempt {
    fn is_current(self) -> bool {
        GENERATION.load(Ordering::SeqCst) == self.0
    }
}

/// Monotonic attempt sequence. Bumping it is lock-free so the UI thread never
/// waits for a hung keyring helper (AUD-02).
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// Serializes external helper invocations: a save's "clear then store" pair and a
/// concurrent clear would otherwise race and leave a deleted or duplicated item.
static STORE_LOCK: Mutex<()> = Mutex::new(());

/// Claim the keyring for a new login attempt. Earlier attempts still running
/// finish their helper, but their write and delete are refused.
pub fn begin_attempt() -> Attempt {
    Attempt(GENERATION.fetch_add(1, Ordering::SeqCst).wrapping_add(1))
}

/// A superseded attempt owns nothing any more.
fn invalidate_attempts() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
}

fn lock_store() -> MutexGuard<'static, ()> {
    // A panic inside one helper invocation must not disable the keyring for the
    // rest of the process; the guarded state is a unit value.
    STORE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The longest a caller can be made to wait by one complete store operation:
/// two bounded helper invocations (pre-clear + store, or clear + confirmation)
/// plus a scheduling margin.
pub fn worst_case_deadline() -> Duration {
    OPERATION_DEADLINE
        .saturating_mul(2)
        .saturating_add(Duration::from_secs(1))
}

fn service() -> String {
    #[cfg(debug_assertions)]
    if let Ok(value) = std::env::var("VELA_NATIVE_KEYRING_SERVICE")
        && let Some(service) = service_override(&value)
    {
        return service;
    }
    DEFAULT_SERVICE.to_owned()
}

/// QA may isolate its item from the release service name. The override is a flat
/// short identifier so it can never become extra `secret-tool` arguments, and it
/// is ignored entirely by release builds.
#[cfg(debug_assertions)]
fn service_override(value: &str) -> Option<String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character))
    {
        return None;
    }
    Some(value.to_owned())
}

fn lookup_command(service: &str) -> Command {
    let mut command = Command::new("secret-tool");
    command.args(["lookup", "service", service]);
    command
}

fn store_command(service: &str) -> Command {
    let mut command = Command::new("secret-tool");
    command.args([
        "store",
        "--label=Vela Music saved session",
        "service",
        service,
    ]);
    command
}

fn clear_command(service: &str) -> Command {
    let mut command = Command::new("secret-tool");
    command.args(["clear", "service", service]);
    command
}

fn validate(origin: &str, token: &str) -> Result<(String, String), KeyringError> {
    let origin = canonical_origin(origin)
        .map_err(|_| KeyringError::InvalidItem)?
        .as_str()
        .to_owned();
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(KeyringError::InvalidItem);
    }
    Ok((origin, token.to_ascii_lowercase()))
}

pub fn load() -> Result<Option<SavedSession>, KeyringError> {
    let _guard = lock_store();
    let service = service();
    let output = run_bounded(lookup_command(&service), None, true, OPERATION_DEADLINE)?;
    // A missing item is a normal, non-error result.
    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }
    parse_envelope(&output.stdout).map(Some)
}

/// Parse an already byte-bounded envelope. Kept pure so the stored format can be
/// tested without a Secret Service.
fn parse_envelope(bytes: &[u8]) -> Result<SavedSession, KeyringError> {
    let envelope: Envelope =
        serde_json::from_slice(bytes).map_err(|_| KeyringError::InvalidItem)?;
    if envelope.version != 1 {
        return Err(KeyringError::UnsupportedVersion);
    }
    let (origin, token) = validate(&envelope.origin, &envelope.token)?;
    Ok(SavedSession { origin, token })
}

pub fn save(attempt: Attempt, origin: &str, token: &str) -> Result<StoreStatus, KeyringError> {
    let (origin, token) = validate(origin, token)?;
    let payload = serde_json::to_vec(&Envelope {
        version: 1,
        origin,
        token,
    })
    .map_err(|_| KeyringError::StoreFailed)?;
    // Every stored item must stay readable under `MAX_SECRET`; origins are limited
    // by `canonical_origin`, so this only rejects pathological inputs.
    if payload.len() > MAX_SECRET {
        return Err(KeyringError::StoreFailed);
    }
    let _guard = lock_store();
    // AUD-01: a login that was canceled or replaced owns nothing. Refuse before
    // touching the helper so a newer account's item cannot be overwritten.
    if !attempt.is_current() {
        return Ok(StoreStatus::Superseded);
    }
    let service = service();
    // `secret-tool store` adds an item, so clear the fixed attribute first: a
    // repeated opt-in must not accumulate stale session tokens. The pre-clear
    // result is best effort because a missing item already makes `clear` fail;
    // the `store` below is the confirming write.
    let _ = run_bounded(clear_command(&service), None, false, OPERATION_DEADLINE);
    let stored = run_bounded(
        store_command(&service),
        Some(payload),
        false,
        OPERATION_DEADLINE,
    )?;
    if !stored.status.success() {
        return Err(KeyringError::StoreFailed);
    }
    Ok(StoreStatus::Stored)
}

/// Unconditional deletion for an explicit sign-out or an unusable item. It also
/// invalidates pending attempts: a canceled login may not recreate the item after
/// sign-out.
pub fn clear() -> Result<(), KeyringError> {
    let _guard = lock_store();
    invalidate_attempts();
    remove(&service())
}

/// Deletion that only runs while its attempt still owns the keyring, so a stale
/// resume cannot delete the item of a newer login.
pub fn clear_attempt(attempt: Attempt) -> Result<(), KeyringError> {
    let _guard = lock_store();
    if !attempt.is_current() {
        return Ok(());
    }
    remove(&service())
}

fn remove(service: &str) -> Result<(), KeyringError> {
    let cleared = run_bounded(clear_command(service), None, false, OPERATION_DEADLINE)?;
    if cleared.status.success() {
        return Ok(());
    }
    // A missing item makes `clear` exit non-zero. Confirmation is a bounded
    // lookup: an item that is still present is a real failure, and an
    // unconfirmed lookup is reported as itself rather than as success.
    let lookup = run_bounded(lookup_command(service), None, true, OPERATION_DEADLINE)?;
    if lookup.status.success() && !lookup.stdout.is_empty() {
        return Err(KeyringError::ClearUnconfirmed);
    }
    Ok(())
}

/// Deadline for one helper invocation, measured on the monotonic clock.
#[derive(Clone, Copy, Debug)]
struct Deadline {
    at: Instant,
}

impl Deadline {
    fn after(budget: Duration) -> Self {
        let now = Instant::now();
        Self {
            at: now.checked_add(budget).unwrap_or(now),
        }
    }

    fn expired(self, now: Instant) -> bool {
        now >= self.at
    }

    fn remaining(self, now: Instant) -> Duration {
        self.at.saturating_duration_since(now)
    }
}

struct HelperOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
}

/// Run one short-lived helper without ever waiting unboundedly on it.
///
/// The helper's stdout is read on a worker thread that stops at `MAX_SECRET + 1`
/// bytes, so a hostile or broken item is rejected before anything larger is
/// allocated. The calling thread polls the child and kills plus reaps it at the
/// deadline: a hung Secret Service therefore costs a bounded wait, never a
/// blocked UI.
fn run_bounded(
    mut command: Command,
    payload: Option<Vec<u8>>,
    capture: bool,
    budget: Duration,
) -> Result<HelperOutput, KeyringError> {
    let mut child = command
        .stdin(if payload.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(if capture {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| KeyringError::Unavailable)?;
    if let Some(payload) = payload {
        let Some(mut stdin) = child.stdin.take() else {
            terminate(&mut child);
            return Err(KeyringError::StoreFailed);
        };
        // The payload is far smaller than a pipe buffer, so this write cannot
        // wait for a helper that is not reading yet; a closed pipe is reported.
        let written = stdin.write_all(&payload);
        drop(stdin);
        written.map_err(|_| KeyringError::StoreFailed)?;
    }
    let reader = if capture {
        let Some(mut stdout) = child.stdout.take() else {
            terminate(&mut child);
            return Err(KeyringError::Unavailable);
        };
        let (sender, receiver) = mpsc::channel();
        // The worker owns the read end. When the child exits, or is killed and
        // reaped below, the pipe reaches EOF and the worker ends.
        std::thread::spawn(move || {
            let _ = sender.send(read_bounded(&mut stdout, MAX_SECRET));
        });
        Some(receiver)
    } else {
        None
    };
    let deadline = Deadline::after(budget);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = match reader {
                    Some(receiver) => receive_reader(&receiver, deadline)?,
                    None => Vec::new(),
                };
                return Ok(HelperOutput { status, stdout });
            }
            Ok(None) => {}
            Err(_) => {
                terminate(&mut child);
                return Err(KeyringError::Unavailable);
            }
        }
        if deadline.expired(Instant::now()) {
            terminate(&mut child);
            return Err(KeyringError::Deadline);
        }
        if let Some(receiver) = reader.as_ref()
            && let Ok(Err(error)) = receiver.try_recv()
        {
            // An oversized item must not keep the helper alive until the deadline.
            terminate(&mut child);
            return Err(error);
        }
        std::thread::sleep(POLL_INTERVAL.min(deadline.remaining(Instant::now())));
    }
}

/// Kill and reap. Both calls are required: SIGKILL ends the helper and `wait`
/// reaps it, so no zombie or pipe descriptor outlives the deadline.
fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn receive_reader(
    receiver: &Receiver<Result<Vec<u8>, KeyringError>>,
    deadline: Deadline,
) -> Result<Vec<u8>, KeyringError> {
    match receiver.recv_timeout(deadline.remaining(Instant::now())) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => Err(KeyringError::Deadline),
        Err(RecvTimeoutError::Disconnected) => Err(KeyringError::Unavailable),
    }
}

/// Read at most `limit` bytes plus the one byte that proves the output is longer,
/// so allocation is bounded by the limit and not by what the helper writes.
fn read_bounded(reader: &mut impl Read, limit: usize) -> Result<Vec<u8>, KeyringError> {
    let mut collected = Vec::new();
    let mut chunk = [0u8; READ_CHUNK];
    loop {
        let allowance = limit
            .saturating_sub(collected.len())
            .saturating_add(1)
            .min(chunk.len());
        let read = reader
            .read(&mut chunk[..allowance])
            .map_err(|_| KeyringError::Unavailable)?;
        if read == 0 {
            return Ok(collected);
        }
        if collected.len() + read > limit {
            return Err(KeyringError::InvalidItem);
        }
        collected.extend_from_slice(&chunk[..read]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn token() -> String {
        "a".repeat(64)
    }

    #[test]
    fn saved_session_requires_canonical_origin_and_opaque_token() {
        let token = token();
        assert_eq!(
            validate("https://music.example.org", &token).unwrap(),
            ("https://music.example.org/".to_owned(), token.clone())
        );
        assert_eq!(
            validate("http://music.example.org", &token),
            Err(KeyringError::InvalidItem)
        );
        assert_eq!(
            validate("https://music.example.org", "secret"),
            Err(KeyringError::InvalidItem)
        );
        assert_eq!(
            validate("https://music.example.org", &"g".repeat(64)),
            Err(KeyringError::InvalidItem)
        );
        assert_eq!(
            validate("https://music.example.org", &token.to_ascii_uppercase()),
            Ok(("https://music.example.org/".to_owned(), token))
        );
    }

    #[test]
    fn saved_session_debug_never_prints_the_token() {
        let saved = SavedSession {
            origin: "https://music.example.org/".to_owned(),
            token: token(),
        };
        let rendered = format!("{saved:?}");
        assert!(!rendered.contains(&saved.token), "{rendered}");
        assert!(rendered.contains("<redacted>"), "{rendered}");
        assert!(
            rendered.contains("https://music.example.org/"),
            "{rendered}"
        );
    }

    #[test]
    fn envelope_parsing_rejects_foreign_and_invalid_items() {
        let valid = format!(
            "{{\"version\":1,\"origin\":\"https://music.example.org\",\"token\":\"{}\"}}",
            token()
        );
        assert_eq!(
            parse_envelope(valid.as_bytes()).unwrap(),
            SavedSession {
                origin: "https://music.example.org/".to_owned(),
                token: token(),
            }
        );
        let future = valid.replace("\"version\":1", "\"version\":2");
        assert_eq!(
            parse_envelope(future.as_bytes()),
            Err(KeyringError::UnsupportedVersion)
        );
        let short = valid.replace(&token(), "abc");
        assert_eq!(
            parse_envelope(short.as_bytes()),
            Err(KeyringError::InvalidItem)
        );
        let password = valid.replace("token", "password");
        assert_eq!(
            parse_envelope(password.as_bytes()),
            Err(KeyringError::InvalidItem)
        );
        assert_eq!(parse_envelope(b"not json"), Err(KeyringError::InvalidItem));
    }

    #[test]
    fn keyring_output_is_bounded_before_it_is_buffered() {
        let mut oversized = Cursor::new(vec![b'a'; MAX_SECRET * 4]);
        assert_eq!(
            read_bounded(&mut oversized, MAX_SECRET),
            Err(KeyringError::InvalidItem)
        );

        let mut exact = Cursor::new(vec![b'a'; MAX_SECRET]);
        let read = read_bounded(&mut exact, MAX_SECRET).unwrap();
        assert_eq!(read.len(), MAX_SECRET);

        let mut empty = Cursor::new(Vec::new());
        assert_eq!(read_bounded(&mut empty, MAX_SECRET), Ok(Vec::new()));
    }

    struct CountingReader {
        remaining: usize,
        consumed: usize,
    }

    impl Read for CountingReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let amount = buffer.len().min(self.remaining);
            buffer[..amount].fill(b'a');
            self.remaining -= amount;
            self.consumed += amount;
            Ok(amount)
        }
    }

    #[test]
    fn oversized_output_is_read_one_byte_past_the_limit() {
        let mut reader = CountingReader {
            remaining: MAX_SECRET * 8,
            consumed: 0,
        };
        assert_eq!(
            read_bounded(&mut reader, MAX_SECRET),
            Err(KeyringError::InvalidItem)
        );
        // The helper is never drained: the bound is the limit, not its output.
        assert_eq!(reader.consumed, MAX_SECRET + 1);
    }

    #[test]
    fn deadline_expires_and_remaining_never_underflows() {
        // Both deadlines are observed after they were armed, so the assertions
        // are deterministic on the monotonic clock.
        let expired = Deadline::after(Duration::ZERO);
        let later = Deadline::after(Duration::from_secs(30));
        let now = Instant::now();
        assert!(expired.expired(now));
        assert_eq!(expired.remaining(now), Duration::ZERO);
        assert!(!later.expired(now));
        assert!(later.remaining(now) <= Duration::from_secs(30));
        assert!(later.remaining(now) > Duration::ZERO);
    }

    #[test]
    fn a_complete_store_operation_is_bounded_by_two_deadlines() {
        // The UI-side bound must cover the whole store operation (pre-clear plus
        // store, or clear plus confirmation) and still be finite.
        assert!(worst_case_deadline() > OPERATION_DEADLINE * 2);
        assert!(worst_case_deadline() <= Duration::from_secs(30));
    }

    #[test]
    fn superseded_attempts_may_not_write_or_delete() {
        let first = begin_attempt();
        assert!(first.is_current());
        let second = begin_attempt();
        assert!(!first.is_current(), "an older login lost its claim");
        assert!(second.is_current());
        invalidate_attempts();
        assert!(!second.is_current(), "sign-out invalidates every attempt");
    }

    #[cfg(debug_assertions)]
    #[test]
    fn debug_service_override_is_a_flat_identifier() {
        assert_eq!(
            service_override("org.vela.test.qa"),
            Some("org.vela.test.qa".to_owned())
        );
        assert_eq!(service_override(""), None);
        assert_eq!(service_override(&"a".repeat(129)), None);
        assert_eq!(service_override("org.vela; rm -rf /"), None);
        assert_eq!(service_override("org.vela service"), None);
    }

    #[test]
    fn only_corrupt_records_may_be_cleaned_up_automatically() {
        assert!(KeyringError::InvalidItem.is_corrupt_record());
        assert!(KeyringError::UnsupportedVersion.is_corrupt_record());
        // A timeout or a missing helper must never destroy a valid item.
        for error in [
            KeyringError::Unavailable,
            KeyringError::StoreFailed,
            KeyringError::ClearUnconfirmed,
            KeyringError::Deadline,
        ] {
            assert!(!error.is_corrupt_record(), "{error:?}");
        }
    }

    #[test]
    fn keyring_errors_expose_fixed_messages() {
        for error in [
            KeyringError::Unavailable,
            KeyringError::InvalidItem,
            KeyringError::UnsupportedVersion,
            KeyringError::StoreFailed,
            KeyringError::ClearUnconfirmed,
            KeyringError::Deadline,
        ] {
            assert!(!error.message().is_empty());
            assert_eq!(error.to_string(), error.message());
            assert!(!error.message().contains(&token()));
        }
    }

    /// The external-helper contract, exercised with ordinary POSIX processes so
    /// no Secret Service and no user data are involved: a helper that never
    /// finishes is killed and reaped at its deadline instead of blocking the
    /// caller (AUD-11).
    #[cfg(unix)]
    #[test]
    fn hung_helper_is_killed_and_reaped_at_its_deadline() {
        let mut command = Command::new("sleep");
        command.arg("30");
        let started = Instant::now();
        let result = run_bounded(command, None, false, Duration::from_millis(150));
        assert!(matches!(result, Err(KeyringError::Deadline)));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    /// A helper that keeps writing is stopped at the byte bound instead of
    /// being drained into memory (AUD-11).
    #[cfg(unix)]
    #[test]
    fn endless_helper_output_is_bounded_and_stopped() {
        let started = Instant::now();
        let result = run_bounded(Command::new("yes"), None, true, OPERATION_DEADLINE);
        assert!(matches!(result, Err(KeyringError::InvalidItem)));
        assert!(started.elapsed() < OPERATION_DEADLINE);
    }
}
