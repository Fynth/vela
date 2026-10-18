//! Linux desktop media integration through MPRIS2 over the session D-Bus.
//!
//! The D-Bus service runs on its own thread and never owns account state,
//! credentials, HTTP or audio. The UI publishes immutable snapshots; control
//! methods are delivered to the UI thread as typed commands. A failed bus
//! connection degrades to ordinary in-app controls without affecting playback.
//! Other targets keep the same contract with an honest Unsupported state until
//! their native media-session integration exists.
#[cfg(target_os = "linux")]
use mpris_server::{LoopStatus, Metadata, PlaybackStatus, Player, Time, TrackId, zbus::Result};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU8, Ordering},
};
use tokio::sync::mpsc::{Receiver, Sender, channel};

const STARTING: u8 = 0;
const READY: u8 = 1;
const FAILED: u8 = 2;
const UNSUPPORTED: u8 = 3;
#[cfg(target_os = "linux")]
const MAX_TEXT: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Stopped,
    Playing,
    Paused,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepeatKind {
    Off,
    All,
    One,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Update {
    pub status: Status,
    pub track: Option<Track>,
    pub position_ms: u64,
    pub volume: f64,
    pub shuffle: bool,
    pub repeat: RepeatKind,
    pub can_play: bool,
    pub can_pause: bool,
    pub can_seek: bool,
    pub can_go_next: bool,
    pub can_go_previous: bool,
    /// Increments only for an explicit seek; drives the MPRIS Seeked signal.
    pub seek_serial: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Command {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
    SeekRelative(i64),
    SetPosition { track_id: String, position_ms: u64 },
    SetVolume(f64),
    SetShuffle(bool),
    SetRepeat(RepeatKind),
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Starting,
    Ready,
    Failed,
    Unsupported,
}

#[derive(Clone)]
pub(crate) struct Mpris {
    updates: Sender<Update>,
    commands: Arc<Mutex<Receiver<Command>>>,
    state: Arc<AtomicU8>,
}

impl Mpris {
    pub(crate) fn publish(&self, update: Update) {
        // A closed or saturated observer must never surface an audio/UI error.
        // The next 200ms snapshot supersedes a dropped intermediate position.
        let _ = self.updates.try_send(update);
    }
    pub(crate) fn try_command(&self) -> Option<Command> {
        self.commands.lock().ok()?.try_recv().ok()
    }
    pub(crate) fn state(&self) -> State {
        match self.state.load(Ordering::Acquire) {
            READY => State::Ready,
            FAILED => State::Failed,
            UNSUPPORTED => State::Unsupported,
            _ => State::Starting,
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn spawn() -> Mpris {
    let (updates, _) = channel(64);
    let (_, command_rx) = channel(32);
    Mpris {
        updates,
        commands: Arc::new(Mutex::new(command_rx)),
        state: Arc::new(AtomicU8::new(UNSUPPORTED)),
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn spawn() -> Mpris {
    let (updates, update_rx) = channel(64);
    let (commands, command_rx) = channel(32);
    let state = Arc::new(AtomicU8::new(STARTING));
    let thread_state = state.clone();
    let thread = std::thread::Builder::new()
        .name("vela-mpris".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(_) => {
                    thread_state.store(FAILED, Ordering::Release);
                    return;
                }
            };
            let local = tokio::task::LocalSet::new();
            let outcome = local.block_on(&runtime, run(update_rx, commands, thread_state.clone()));
            if outcome.is_err() {
                thread_state.store(FAILED, Ordering::Release);
            }
        });
    if thread.is_err() {
        state.store(FAILED, Ordering::Release);
    }
    Mpris {
        updates,
        commands: Arc::new(Mutex::new(command_rx)),
        state,
    }
}

#[cfg(target_os = "linux")]
async fn run(
    mut updates: Receiver<Update>,
    commands: Sender<Command>,
    state: Arc<AtomicU8>,
) -> Result<()> {
    let player = Player::builder("vela")
        .identity("Vela Music")
        .desktop_entry("vela-desktop")
        .supported_mime_types([
            "audio/mpeg",
            "audio/flac",
            "audio/wav",
            "audio/ogg",
            "audio/mp4",
        ])
        .can_quit(true)
        .can_raise(false)
        .can_play(false)
        .can_pause(false)
        .can_seek(false)
        .can_go_next(false)
        .can_go_previous(false)
        .can_control(true)
        .build()
        .await?;
    let send = commands.clone();
    player.connect_play(move |_| {
        let _ = send.try_send(Command::Play);
    });
    let send = commands.clone();
    player.connect_pause(move |_| {
        let _ = send.try_send(Command::Pause);
    });
    let send = commands.clone();
    player.connect_play_pause(move |_| {
        let _ = send.try_send(Command::PlayPause);
    });
    let send = commands.clone();
    player.connect_stop(move |_| {
        let _ = send.try_send(Command::Stop);
    });
    let send = commands.clone();
    player.connect_next(move |_| {
        let _ = send.try_send(Command::Next);
    });
    let send = commands.clone();
    player.connect_previous(move |_| {
        let _ = send.try_send(Command::Previous);
    });
    let send = commands.clone();
    player.connect_seek(move |_, offset| {
        let _ = send.try_send(Command::SeekRelative(offset.as_micros()));
    });
    let send = commands.clone();
    player.connect_set_position(move |_, track_id, position| {
        let _ = send.try_send(Command::SetPosition {
            track_id: track_id.as_str().to_owned(),
            position_ms: u64::try_from(position.as_micros().max(0)).unwrap_or(0) / 1000,
        });
    });
    let send = commands.clone();
    player.connect_set_volume(move |_, volume| {
        let _ = send.try_send(Command::SetVolume(volume));
    });
    let send = commands.clone();
    player.connect_set_shuffle(move |_, shuffle| {
        let _ = send.try_send(Command::SetShuffle(shuffle));
    });
    let send = commands.clone();
    player.connect_set_loop_status(move |_, status| {
        let repeat = match status {
            LoopStatus::None => RepeatKind::Off,
            LoopStatus::Playlist => RepeatKind::All,
            LoopStatus::Track => RepeatKind::One,
        };
        let _ = send.try_send(Command::SetRepeat(repeat));
    });
    let send = commands.clone();
    player.connect_quit(move |_| {
        let _ = send.try_send(Command::Quit);
    });
    tokio::task::spawn_local(player.run());
    state.store(READY, Ordering::Release);
    let mut last: Option<Update> = None;
    while let Some(update) = updates.recv().await {
        apply(&player, last.as_ref(), &update).await?;
        last = Some(update);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
async fn apply(player: &Player, last: Option<&Update>, update: &Update) -> Result<()> {
    if last.is_none_or(|last| last.status != update.status) {
        player
            .set_playback_status(match update.status {
                Status::Stopped => PlaybackStatus::Stopped,
                Status::Playing => PlaybackStatus::Playing,
                Status::Paused => PlaybackStatus::Paused,
            })
            .await?;
    }
    if last.is_none() || last.is_none_or(|last| last.track != update.track) {
        let metadata = update
            .track
            .as_ref()
            .map_or_else(Metadata::new, metadata_for);
        player.set_metadata(metadata).await?;
    }
    if last.is_none_or(|last| last.volume != update.volume) {
        player.set_volume(update.volume).await?;
    }
    if last.is_none_or(|last| last.shuffle != update.shuffle) {
        player.set_shuffle(update.shuffle).await?;
    }
    if last.is_none_or(|last| last.repeat != update.repeat) {
        player
            .set_loop_status(match update.repeat {
                RepeatKind::Off => LoopStatus::None,
                RepeatKind::All => LoopStatus::Playlist,
                RepeatKind::One => LoopStatus::Track,
            })
            .await?;
    }
    if last.is_none_or(|last| last.can_play != update.can_play) {
        player.set_can_play(update.can_play).await?;
    }
    if last.is_none_or(|last| last.can_pause != update.can_pause) {
        player.set_can_pause(update.can_pause).await?;
    }
    if last.is_none_or(|last| last.can_seek != update.can_seek) {
        player.set_can_seek(update.can_seek).await?;
    }
    if last.is_none_or(|last| last.can_go_next != update.can_go_next) {
        player.set_can_go_next(update.can_go_next).await?;
    }
    if last.is_none_or(|last| last.can_go_previous != update.can_go_previous) {
        player.set_can_go_previous(update.can_go_previous).await?;
    }
    player.set_position(Time::from_micros(
        i64::try_from(update.position_ms.saturating_mul(1000)).unwrap_or(i64::MAX),
    ));
    if last.is_none_or(|last| last.seek_serial != update.seek_serial) {
        player
            .seeked(Time::from_millis(
                i64::try_from(update.position_ms).unwrap_or(i64::MAX),
            ))
            .await?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn metadata_for(track: &Track) -> Metadata {
    let track_id = TrackId::try_from(track_path(&track.id)).unwrap_or(TrackId::NO_TRACK);
    let mut metadata = Metadata::builder()
        .trackid(track_id)
        .title(safe_text(&track.title));
    if !track.artist.is_empty() {
        metadata = metadata.artist([safe_text(&track.artist)]);
    }
    if !track.album.is_empty() {
        metadata = metadata.album(safe_text(&track.album));
    }
    if track.duration_ms > 0 {
        metadata = metadata.length(Time::from_micros(
            i64::try_from(track.duration_ms.saturating_mul(1000)).unwrap_or(i64::MAX),
        ));
    }
    metadata.build()
}

#[cfg(target_os = "linux")]
fn safe_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_TEXT)
        .collect()
}

/// A stable D-Bus object path for one owned track. The server validates UUIDs;
/// only their hexadecimal digits enter the path, so no untrusted text can
/// create an invalid or colliding object path.
pub(crate) fn track_path(id: &str) -> String {
    let hex: String = id
        .chars()
        .filter(char::is_ascii_hexdigit)
        .map(|character| character.to_ascii_lowercase())
        .take(32)
        .collect();
    if hex.len() == 32 {
        format!("/org/vela/track/{hex}")
    } else {
        "/org/vela/track/unidentified".into()
    }
}

/// MPRIS metadata text is presentation only; the UI remains the authority.
#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use mpris_server::zbus;

    fn update() -> Update {
        Update {
            status: Status::Playing,
            track: Some(Track {
                id: "8d0c2cb0-0e6a-4d1e-8a4f-3f0f6a3a4b1c".into(),
                title: "Signal".into(),
                artist: "Vela Test".into(),
                album: "Real Fixtures".into(),
                duration_ms: 30_000,
            }),
            position_ms: 4200,
            volume: 0.7,
            shuffle: true,
            repeat: RepeatKind::All,
            can_play: true,
            can_pause: true,
            can_seek: true,
            can_go_next: true,
            can_go_previous: true,
            seek_serial: 1,
        }
    }

    #[test]
    fn dbus_track_paths_are_stable_and_reject_hostile_text() {
        let path = track_path("8D0C2CB0-0E6A-4D1E-8A4F-3F0F6A3A4B1C");
        assert_eq!(path, "/org/vela/track/8d0c2cb00e6a4d1e8a4f3f0f6a3a4b1c");
        assert_eq!(
            track_path("../../etc/passwd"),
            "/org/vela/track/unidentified"
        );
        assert_eq!(track_path(""), "/org/vela/track/unidentified");
        assert!(!track_path("x").contains('-'));
    }

    #[test]
    fn metadata_omits_unknown_duration_and_control_characters() {
        let mut track = Track {
            id: "8d0c2cb0-0e6a-4d1e-8a4f-3f0f6a3a4b1c".into(),
            title: "T\u{0}itle\n".into(),
            artist: String::new(),
            album: String::new(),
            duration_ms: 0,
        };
        let metadata = metadata_for(&track);
        assert_eq!(metadata.title(), Some("Title"));
        assert_eq!(
            metadata.trackid().map(|id| id.as_str().to_owned()),
            Some("/org/vela/track/8d0c2cb00e6a4d1e8a4f3f0f6a3a4b1c".to_owned())
        );
        assert_eq!(metadata.length(), None);
        assert_eq!(metadata.artist(), None);
        track.duration_ms = 30_000;
        assert_eq!(
            metadata_for(&track).length(),
            Some(Time::from_millis(30_000))
        );
    }

    #[test]
    #[ignore = "Requires a private D-Bus session bus; run under dbus-run-session"]
    fn real_session_bus_exposes_properties_and_forwards_controls() {
        let service = spawn();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while service.state() == State::Starting && std::time::Instant::now() < deadline {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            assert_eq!(service.state(), State::Ready, "MPRIS service did not start");
            let connection = zbus::Connection::session().await.unwrap();
            let proxy = zbus::Proxy::new(
                &connection,
                "org.mpris.MediaPlayer2.vela",
                "/org/mpris/MediaPlayer2",
                "org.mpris.MediaPlayer2.Player",
            )
            .await
            .unwrap();
            let status: String = proxy.get_property("PlaybackStatus").await.unwrap();
            assert!(matches!(status.as_str(), "Playing" | "Paused" | "Stopped"));
            service.publish(update());
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                let status: String = proxy.get_property("PlaybackStatus").await.unwrap();
                let metadata: std::collections::HashMap<String, zbus::zvariant::OwnedValue> =
                    proxy.get_property("Metadata").await.unwrap();
                let has_title =
                    metadata.contains_key("xesam:title") && metadata.contains_key("mpris:trackid");
                if status == "Playing" && has_title {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "MPRIS state did not update"
                );
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            proxy.call_method("PlayPause", &()).await.unwrap();
            proxy.call_method("Next", &()).await.unwrap();
            proxy
                .call_method("Seek", &Time::from_millis(1500))
                .await
                .unwrap();
            proxy
                .call_method(
                    "SetPosition",
                    &(
                        TrackId::try_from(track_path("8d0c2cb0-0e6a-4d1e-8a4f-3f0f6a3a4b1c"))
                            .unwrap(),
                        Time::from_millis(5000),
                    ),
                )
                .await
                .unwrap();
            proxy.set_property("Volume", 0.25_f64).await.unwrap();
            proxy.set_property("Shuffle", false).await.unwrap();
            proxy.set_property("LoopStatus", "Track").await.unwrap();
            proxy.call_method("Stop", &()).await.unwrap();
            let root = zbus::Proxy::new(
                &connection,
                "org.mpris.MediaPlayer2.vela",
                "/org/mpris/MediaPlayer2",
                "org.mpris.MediaPlayer2",
            )
            .await
            .unwrap();
            root.call_method("Quit", &()).await.unwrap();
            let mut received = Vec::new();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while received.len() < 9 && std::time::Instant::now() < deadline {
                if let Some(command) = service.try_command() {
                    received.push(command);
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            assert!(received.contains(&Command::PlayPause));
            assert!(received.contains(&Command::Next));
            assert!(received.contains(&Command::SeekRelative(1_500_000)));
            assert!(received.contains(&Command::SetPosition {
                track_id: track_path("8d0c2cb0-0e6a-4d1e-8a4f-3f0f6a3a4b1c"),
                position_ms: 5000,
            }));
            assert!(received.contains(&Command::SetVolume(0.25)));
            assert!(received.contains(&Command::SetShuffle(false)));
            assert!(received.contains(&Command::SetRepeat(RepeatKind::One)));
            assert!(received.contains(&Command::Stop));
            assert!(received.contains(&Command::Quit));
        });
    }
}
