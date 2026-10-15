//! Descriptor-relative, read-only traversal. Absolute paths stay in this module.
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt, OpenOptionsSyncExt};
use cap_std::fs::{Dir, OpenOptions, ReadDir};
use tempfile::NamedTempFile;
use tokio_util::sync::CancellationToken;

use crate::ScanLimits;

#[cfg(target_os = "linux")]
use std::path::Component;

#[derive(Debug, thiserror::Error)]
pub(crate) enum FileError {
    #[error("The configured music folder is unavailable or unsafe")]
    Root,
    #[error("Folder entry limit exceeded")]
    Entries,
    #[error("Audio file or fingerprint limit exceeded")]
    Files,
    #[error("Audio file exceeds the configured size limit")]
    Size,
    #[error("Temporary storage reserve is insufficient")]
    Space,
    #[error("Audio file could not be safely read")]
    Read,
    #[error("Temporary audio snapshot could not be written")]
    Write,
    #[error("Audio file changed while it was being copied")]
    Changed,
    #[error("Audio snapshot exceeded the copy deadline")]
    Timeout,
    #[error("Scan was cancelled")]
    Cancelled,
    #[error("Server is shutting down")]
    Shutdown,
}

pub(crate) fn open_root(path: &Path) -> Result<Dir, FileError> {
    #[cfg(target_os = "linux")]
    {
        if !path.is_absolute() {
            return Err(FileError::Root);
        }
        // Start at a trusted descriptor, then reject symlinks at *every* component.
        // No canonicalize/check/open race and no ambient source-file access.
        let mut directory = Dir::open_ambient_dir("/", cap_std::ambient_authority())
            .map_err(|_| FileError::Root)?;
        let mut components = 0;
        for component in path.components() {
            match component {
                Component::RootDir => {}
                Component::Normal(name) => {
                    directory = directory
                        .open_dir_nofollow(name)
                        .map_err(|_| FileError::Root)?;
                    components += 1;
                }
                _ => return Err(FileError::Root),
            }
        }
        if components == 0 {
            return Err(FileError::Root);
        }
        Ok(directory)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        Err(FileError::Root)
    }
}

struct Frame {
    directory: Dir,
    entries: ReadDir,
    relative: PathBuf,
}

pub(crate) struct Walker {
    stack: Vec<Frame>,
    limits: ScanLimits,
    pub visited: u64,
    pub skipped: u64,
    files: u64,
}

pub(crate) struct Candidate {
    pub file: File,
    pub filename: String,
    pub path_key: String,
    pub fingerprint: String,
    pub size: u64,
}

pub(crate) enum Step {
    Candidate(Candidate),
    Progress,
    Done,
}

impl Walker {
    pub fn new(path: &Path, limits: ScanLimits) -> Result<Self, FileError> {
        let directory = open_root(path)?;
        let entries = directory.entries().map_err(|_| FileError::Root)?;
        Ok(Self {
            stack: vec![Frame {
                directory,
                entries,
                relative: PathBuf::new(),
            }],
            limits,
            visited: 0,
            skipped: 0,
            files: 0,
        })
    }

    /// Return at most one descriptor and yield after 256 skipped entries.
    pub fn next(
        &mut self,
        shutdown: &CancellationToken,
        cancel: &CancellationToken,
    ) -> Result<Step, FileError> {
        for _ in 0..256 {
            interrupted(shutdown, cancel)?;
            let depth = self.stack.len();
            let Some(frame) = self.stack.last_mut() else {
                return Ok(Step::Done);
            };
            let Some(entry) = frame.entries.next() else {
                self.stack.pop();
                continue;
            };
            self.visited += 1;
            if self.visited > self.limits.max_entries {
                return Err(FileError::Entries);
            }
            let Ok(entry) = entry else {
                self.skipped += 1;
                continue;
            };
            let name = entry.file_name();
            let Some(filename) = name.to_str() else {
                self.skipped += 1;
                continue;
            };
            // Filename controls neither a managed path nor a database query.
            if filename.chars().any(char::is_control) {
                self.skipped += 1;
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                self.skipped += 1;
                continue;
            };
            if kind.is_dir() {
                if depth >= self.limits.max_depth as usize {
                    self.skipped += 1;
                    continue;
                }
                let relative = frame.relative.join(&name);
                let Ok(directory) = frame.directory.open_dir_nofollow(&name) else {
                    self.skipped += 1;
                    continue;
                };
                let Ok(entries) = directory.entries() else {
                    self.skipped += 1;
                    continue;
                };
                self.stack.push(Frame {
                    directory,
                    entries,
                    relative,
                });
                continue;
            }
            if !kind.is_file() || !supported(filename) {
                self.skipped += 1;
                continue;
            }
            self.files += 1;
            if self.files > self.limits.max_files {
                return Err(FileError::Files);
            }
            let mut options = OpenOptions::new();
            options.read(true).follow(FollowSymlinks::No).nonblock(true);
            // Nonblocking protects against an entry changing into a FIFO between
            // file_type and open; verify the actual descriptor before reading.
            let Ok(file) = frame.directory.open_with(&name, &options) else {
                self.skipped += 1;
                continue;
            };
            let file = file.into_std();
            let Ok(metadata) = file.metadata() else {
                self.skipped += 1;
                continue;
            };
            if !regular_single_link(&metadata) {
                self.skipped += 1;
                continue;
            }
            let Some(fingerprint) = fingerprint(&metadata) else {
                self.skipped += 1;
                continue;
            };
            let relative = frame.relative.join(filename);
            let Some(relative) = relative.to_str() else {
                self.skipped += 1;
                continue;
            };
            return Ok(Step::Candidate(Candidate {
                file,
                filename: filename.to_owned(),
                path_key: blake3::hash(relative.as_bytes()).to_hex().to_string(),
                fingerprint,
                size: metadata.len(),
            }));
        }
        Ok(Step::Progress)
    }
}

fn supported(filename: &str) -> bool {
    Path::new(filename)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "flac"
                    | "mp3"
                    | "ogg"
                    | "opus"
                    | "m4a"
                    | "mp4"
                    | "aac"
                    | "wav"
                    | "wave"
                    | "aif"
                    | "aiff"
                    | "ape"
                    | "wv"
            )
        })
}

fn regular_single_link(metadata: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        metadata.is_file() && metadata.nlink() == 1
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        false
    }
}

fn fingerprint(metadata: &std::fs::Metadata) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some(format!(
            "{}:{}:{}:{}:{}:{}:{}",
            metadata.len(),
            metadata.mtime(),
            metadata.mtime_nsec(),
            metadata.dev(),
            metadata.ino(),
            metadata.ctime(),
            metadata.ctime_nsec()
        ))
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        None
    }
}

pub(crate) fn interrupted(
    shutdown: &CancellationToken,
    cancel: &CancellationToken,
) -> Result<(), FileError> {
    if cancel.is_cancelled() {
        Err(FileError::Cancelled)
    } else if shutdown.is_cancelled() {
        Err(FileError::Shutdown)
    } else {
        Ok(())
    }
}

/// Snapshot one already authorized descriptor. Source files are never modified.
pub(crate) fn snapshot(
    mut candidate: Candidate,
    staging: &Path,
    max_bytes: u64,
    reserve: u64,
    shutdown: &CancellationToken,
    cancel: &CancellationToken,
) -> Result<(NamedTempFile, Candidate), FileError> {
    interrupted(shutdown, cancel)?;
    if candidate.size == 0 || candidate.size > max_bytes {
        return Err(FileError::Size);
    }
    let expected = candidate.size;
    check_space(staging, expected.saturating_mul(2), reserve)?;
    let mut temporary = tempfile::Builder::new()
        .prefix("scan-")
        .suffix(".part")
        .rand_bytes(16)
        .tempfile_in(staging)
        .map_err(|_| FileError::Write)?;
    let started = Instant::now();
    let mut buffer = [0_u8; 64 * 1024];
    let mut copied = 0_u64;
    let mut disk_checked = 0_u64;
    loop {
        interrupted(shutdown, cancel)?;
        if started.elapsed() > Duration::from_secs(900) {
            return Err(FileError::Timeout);
        }
        let count = candidate
            .file
            .read(&mut buffer)
            .map_err(|_| FileError::Read)?;
        if count == 0 {
            break;
        }
        copied = copied.checked_add(count as u64).ok_or(FileError::Size)?;
        if copied > max_bytes || copied > expected {
            return Err(FileError::Changed);
        }
        if copied.saturating_sub(disk_checked) >= 8 * 1024 * 1024 {
            check_space(
                staging,
                expected.saturating_sub(copied).saturating_add(expected),
                reserve,
            )?;
            disk_checked = copied;
        }
        temporary
            .write_all(&buffer[..count])
            .map_err(|_| FileError::Write)?;
    }
    let metadata = candidate.file.metadata().map_err(|_| FileError::Read)?;
    if copied != expected
        || !regular_single_link(&metadata)
        || fingerprint(&metadata).as_deref() != Some(candidate.fingerprint.as_str())
    {
        return Err(FileError::Changed);
    }
    interrupted(shutdown, cancel)?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| FileError::Write)?;
    check_space(staging, expected, reserve)?;
    Ok((temporary, candidate))
}

fn check_space(staging: &Path, additional: u64, reserve: u64) -> Result<(), FileError> {
    let free = fs2::available_space(staging).map_err(|_| FileError::Write)?;
    if free < additional.saturating_add(reserve) {
        Err(FileError::Space)
    } else {
        Ok(())
    }
}
