use std::{
    fs::File,
    io::{self, BufReader, Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
};

use anyhow::{Context, Result, ensure};
use image::{ImageFormat, ImageReader, Limits, codecs::jpeg::JpegEncoder};
use lofty::{
    config::{GlobalOptions, ParseOptions, ParsingMode, apply_global_options},
    file::{AudioFile, FileType, TaggedFileExt},
    picture::PictureType,
    probe::Probe,
    tag::{Accessor, ItemKey},
};
use tempfile::NamedTempFile;

const FILE_BUFFER_BYTES: usize = 64 * 1024;
const METADATA_READ_BYTES: u64 = 32 * 1024 * 1024;
const TAG_ITEM_BYTES: usize = 8 * 1024 * 1024;
const MAX_COVER_DIMENSION: u32 = 4096;
const MAX_COVER_PIXELS: u64 = 16 * 1024 * 1024;

pub(crate) struct PreparedMedia {
    pub file: NamedTempFile,
    pub hash: String,
    pub size: u64,
    pub content_type: &'static str,
    pub metadata: Metadata,
    pub artwork: Option<Vec<u8>>,
}

pub(crate) struct Metadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: String,
    pub year: Option<i64>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub duration_ms: i64,
    pub codec: &'static str,
    pub bitrate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub bit_depth: Option<i64>,
    pub channels: Option<i64>,
}

/// Build an immutable, hashed snapshot before metadata parsing and publication.
/// All I/O here runs on a bounded blocking worker.
pub(crate) fn prepare(
    staging_root: &Path,
    media_root: &Path,
    staged_path: &Path,
    filename: &str,
    max_bytes: u64,
) -> Result<PreparedMedia> {
    let source_metadata =
        std::fs::symlink_metadata(staged_path).context("staged audio is unavailable")?;
    ensure!(
        source_metadata.is_file(),
        "staged audio must be a regular file"
    );
    ensure!(source_metadata.len() > 0, "audio file is empty");
    ensure!(
        source_metadata.len() <= max_bytes,
        "audio exceeds the configured file limit"
    );
    let root = staging_root
        .canonicalize()
        .context("staging storage is unavailable")?;
    let staged = staged_path
        .canonicalize()
        .context("staged audio is unavailable")?;
    ensure!(
        staged.starts_with(root),
        "audio must originate in configured staging storage"
    );

    let mut source = File::open(&staged).context("staged audio is unavailable")?;
    let mut file = tempfile::Builder::new()
        .prefix(".vela-pending-")
        .tempfile_in(media_root)
        .context("could not stage managed audio")?;
    let mut hasher = blake3::Hasher::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; FILE_BUFFER_BYTES];
    loop {
        let read = source
            .read(&mut buffer)
            .context("could not read staged audio")?;
        if read == 0 {
            break;
        }
        size = size
            .checked_add(read as u64)
            .context("audio size overflow")?;
        ensure!(size <= max_bytes, "audio exceeds the configured file limit");
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .context("could not write managed audio")?;
    }
    ensure!(
        size == source_metadata.len(),
        "staged audio changed during ingestion"
    );
    file.as_file_mut().seek(SeekFrom::Start(0))?;

    apply_global_options(
        GlobalOptions::new()
            .use_custom_resolvers(false)
            .preserve_format_specific_items(false)
            .allocation_limit(TAG_ITEM_BYTES),
    );
    let _reset_options = ResetLoftyOptions;
    let reader = BufReader::with_capacity(
        FILE_BUFFER_BYTES,
        ReadBudget {
            inner: file.as_file_mut(),
            remaining: METADATA_READ_BYTES,
        },
    );
    let parsed = Probe::new(reader)
        .options(ParseOptions::new().parsing_mode(ParsingMode::Strict))
        .guess_file_type()
        .context("could not identify the audio format")?
        .read()
        .context("file is not valid supported audio")?;
    let (codec, content_type) = format_info(parsed.file_type())?;
    let properties = parsed.properties();
    let duration_ms =
        i64::try_from(properties.duration().as_millis()).context("audio duration is too large")?;
    ensure!(duration_ms > 0, "audio has no playable duration");
    ensure!(
        properties.sample_rate().is_some_and(|value| value > 0),
        "audio has no valid sample rate"
    );
    ensure!(
        properties
            .channels()
            .is_some_and(|value| (1..=32).contains(&value)),
        "audio has no valid channel layout"
    );
    let tag = parsed.primary_tag().or_else(|| parsed.first_tag());
    let fallback = fallback_title(filename);
    let metadata = Metadata {
        title: tag
            .and_then(|value| value.title())
            .map(|value| clean_text(&value))
            .filter(|value| !value.is_empty())
            .unwrap_or(fallback),
        artist: tag
            .and_then(|value| value.artist())
            .map(|value| clean_text(&value))
            .unwrap_or_default(),
        album: tag
            .and_then(|value| value.album())
            .map(|value| clean_text(&value))
            .unwrap_or_default(),
        album_artist: tag
            .and_then(|value| value.get_string(ItemKey::AlbumArtist))
            .map(clean_text)
            .unwrap_or_default(),
        genre: tag
            .and_then(|value| value.genre())
            .map(|value| clean_text(&value))
            .unwrap_or_default(),
        year: tag
            .and_then(|value| value.date())
            .map(|value| i64::from(value.year)),
        track_number: tag.and_then(|value| value.track()).map(i64::from),
        disc_number: tag.and_then(|value| value.disk()).map(i64::from),
        duration_ms,
        codec,
        bitrate: properties
            .audio_bitrate()
            .or(properties.overall_bitrate())
            .map(i64::from),
        sample_rate: properties.sample_rate().map(i64::from),
        bit_depth: properties.bit_depth().map(i64::from),
        channels: properties.channels().map(i64::from),
    };
    let picture = parsed
        .tags()
        .iter()
        .find_map(|tag| tag.get_picture_type(PictureType::CoverFront))
        .or_else(|| parsed.tags().iter().find_map(|tag| tag.pictures().first()));
    let artwork = picture.and_then(|picture| match thumbnail(picture.data()) {
        Ok(bytes) => Some(bytes),
        Err(_) => {
            tracing::warn!("discarded invalid or oversized embedded artwork");
            None
        }
    });
    // Durability precedes publication and the database commit.
    file.as_file()
        .sync_all()
        .context("could not flush managed audio")?;
    Ok(PreparedMedia {
        file,
        hash: hasher.finalize().to_hex().to_string(),
        size,
        content_type,
        metadata,
        artwork,
    })
}

fn format_info(format: FileType) -> Result<(&'static str, &'static str)> {
    Ok(match format {
        FileType::Flac => ("FLAC", "audio/flac"),
        // MPEG may contain layers I, II or III; do not assert MP3 for every file.
        FileType::Mpeg => ("MPEG audio", "audio/mpeg"),
        FileType::Aac => ("AAC", "audio/aac"),
        // The container alone does not prove AAC or lossless ALAC encoding.
        FileType::Mp4 => ("MP4 audio", "audio/mp4"),
        FileType::Opus => ("Opus", "audio/ogg"),
        FileType::Vorbis => ("Vorbis", "audio/ogg"),
        FileType::Wav => ("WAV audio", "audio/wav"),
        FileType::Aiff => ("AIFF audio", "audio/aiff"),
        FileType::Ape => ("Monkey's Audio", "audio/ape"),
        FileType::WavPack => ("WavPack", "audio/wavpack"),
        FileType::Mpc => ("Musepack", "audio/musepack"),
        FileType::Speex => ("Speex", "audio/ogg"),
        _ => anyhow::bail!("unsupported audio format"),
    })
}

pub(crate) fn thumbnail(bytes: &[u8]) -> Result<Vec<u8>> {
    ensure!(
        bytes.len() <= TAG_ITEM_BYTES,
        "embedded artwork exceeds size limit"
    );
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    ensure!(
        matches!(
            reader.format(),
            Some(ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP)
        ),
        "unsupported embedded artwork format"
    );
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_COVER_DIMENSION);
    limits.max_image_height = Some(MAX_COVER_DIMENSION);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let decoder = reader.into_decoder()?;
    use image::ImageDecoder;
    let (width, height) = decoder.dimensions();
    ensure!(
        u64::from(width) * u64::from(height) <= MAX_COVER_PIXELS,
        "embedded artwork has too many pixels"
    );
    let image = image::DynamicImage::from_decoder(decoder)?
        .thumbnail(640, 640)
        .to_rgb8();
    let mut encoded = Vec::new();
    JpegEncoder::new_with_quality(&mut encoded, 85).encode_image(&image)?;
    ensure!(
        encoded.len() <= 2 * 1024 * 1024,
        "generated artwork exceeds size limit"
    );
    Ok(encoded)
}

pub(crate) fn clean_text(text: &str) -> String {
    text.trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(512)
        .collect()
}

fn fallback_title(filename: &str) -> String {
    let basename = filename.rsplit(['/', '\\']).next().unwrap_or_default();
    let text = Path::new(basename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let text = clean_text(text);
    if text.is_empty() {
        "Untitled track".to_owned()
    } else {
        text
    }
}

struct ResetLoftyOptions;
impl Drop for ResetLoftyOptions {
    fn drop(&mut self) {
        apply_global_options(GlobalOptions::default());
    }
}

struct ReadBudget<R> {
    inner: R,
    remaining: u64,
}

impl<R: Read> Read for ReadBudget<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            return Err(io::Error::other("metadata read limit exceeded"));
        }
        let allowed = buffer
            .len()
            .min(usize::try_from(self.remaining).unwrap_or(usize::MAX));
        let read = self.inner.read(&mut buffer[..allowed])?;
        self.remaining -= read as u64;
        Ok(read)
    }
}

impl<R: Seek> Seek for ReadBudget<R> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filenames_never_control_paths_or_include_control_characters() {
        assert_eq!(fallback_title("../../example.flac"), "example");
        assert_eq!(fallback_title("C:\\music\\example.mp3"), "example");
        assert_eq!(fallback_title("\0.flac"), "Untitled track");
        assert_eq!(clean_text(" a\0\r\nb "), "ab");
        assert_eq!(clean_text(&"界".repeat(1000)).chars().count(), 512);
    }

    #[test]
    fn valid_cover_is_reencoded_and_script_payload_is_rejected() {
        let image = image::RgbImage::from_pixel(12, 10, image::Rgb([20, 60, 90]));
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut png, ImageFormat::Png)
            .unwrap();
        let cover = thumbnail(png.get_ref()).unwrap();
        assert_eq!(image::guess_format(&cover).unwrap(), ImageFormat::Jpeg);
        assert!(thumbnail(b"<svg><script>alert(1)</script></svg>").is_err());
    }

    #[test]
    fn oversized_dimensions_and_encoded_payloads_are_rejected() {
        let image = image::RgbImage::from_pixel(5000, 1, image::Rgb([20, 60, 90]));
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut png, ImageFormat::Png)
            .unwrap();
        assert!(thumbnail(png.get_ref()).is_err());
        assert!(thumbnail(&vec![0_u8; TAG_ITEM_BYTES + 1]).is_err());
    }

    #[test]
    fn read_budget_cannot_be_reset_by_seeking() {
        let mut input = ReadBudget {
            inner: Cursor::new(b"abcdef"),
            remaining: 3,
        };
        let mut output = [0_u8; 3];
        input.read_exact(&mut output).unwrap();
        input.seek(SeekFrom::Start(0)).unwrap();
        assert!(input.read_exact(&mut output).is_err());
    }
}
