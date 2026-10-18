//! Native client services. Server/database policy stays in the existing server.
pub mod audio;
pub mod client;
pub mod mpris;
pub mod offline;
pub mod peers;
#[path = "../../web/src/playback.rs"]
pub mod playback;
pub mod remote;
pub mod ui;

/// A private temporary original. Its lease, not a public DTO, owns file cleanup.
pub struct PreparedAudio {
    #[cfg(not(target_os = "linux"))]
    file: tempfile::NamedTempFile,
    #[cfg(target_os = "linux")]
    _file: std::fs::File,
    #[cfg(target_os = "linux")]
    reader_path: std::path::PathBuf,
    pub length_bytes: u64,
}
impl std::fmt::Debug for PreparedAudio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedAudio")
            .field("length_bytes", &self.length_bytes)
            .finish_non_exhaustive()
    }
}
impl PreparedAudio {
    #[cfg(any(not(target_os = "linux"), test))]
    pub(crate) fn new(file: tempfile::NamedTempFile, length_bytes: u64) -> Self {
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            // No completed original remains named on disk, even if the GUI
            // event loop exits before the audio worker can drop its lease.
            // Reopening procfd gives each decoder an independent file cursor;
            // File::try_clone would incorrectly share its seek offset.
            let file = file.into_file();
            let reader_path = format!("/proc/self/fd/{}", file.as_raw_fd()).into();
            Self {
                _file: file,
                reader_path,
                length_bytes,
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            Self { file, length_bytes }
        }
    }
    /// Take ownership of an already unnamed temporary file. The platform keeps
    /// no directory entry for it, so a process exit during preparation cannot
    /// leave a complete or partial private original behind.
    #[cfg(target_os = "linux")]
    pub(crate) fn from_file(file: std::fs::File, length_bytes: u64) -> Self {
        use std::os::fd::AsRawFd;
        let reader_path = format!("/proc/self/fd/{}", file.as_raw_fd()).into();
        Self {
            _file: file,
            reader_path,
            length_bytes,
        }
    }
    pub(crate) fn path(&self) -> &std::path::Path {
        #[cfg(target_os = "linux")]
        {
            &self.reader_path
        }
        #[cfg(not(target_os = "linux"))]
        {
            self.file.path()
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioPhase {
    #[default]
    Idle,
    Loading,
    Playing,
    Paused,
    Ended,
    Error,
}
#[derive(Debug, Clone)]
pub struct AudioSnapshot {
    pub generation: u64,
    pub phase: AudioPhase,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub error: Option<String>,
}
impl Default for AudioSnapshot {
    fn default() -> Self {
        Self {
            generation: 0,
            phase: AudioPhase::Idle,
            position_ms: 0,
            duration_ms: 0,
            volume: 0.7,
            error: None,
        }
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct DownloadProgress {
    pub received_bytes: u64,
    pub total_bytes: u64,
}
