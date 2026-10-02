//! Audio download via `yt-dlp -x --audio-format <format>`.

use crate::error::{Result, YoutubeHelperError};
use crate::ytdlp;
use std::path::{Path, PathBuf};

/// The container/codec `yt-dlp` extracts the audio track to.
///
/// A closed set rather than a free-form string: the value is handed straight to
/// `yt-dlp --audio-format`, and a caller-supplied string could just as easily be
/// another command-line flag. Spelling the choices out costs one `match` and
/// removes the question entirely.
///
/// Marked `#[non_exhaustive]`: `yt-dlp` gains formats, and adding one here
/// should not be a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum AudioFormat {
    /// Uncompressed 16-bit PCM in a WAV container. The default, because it is
    /// what a decoder or an ML pipeline wants and it never loses a second time —
    /// at roughly ten times the size of the compressed original.
    #[default]
    Wav,
    /// MP3. Universally playable; re-encodes, so it loses quality a second time.
    Mp3,
    /// AAC in an MP4 container — what YouTube usually serves already, so this is
    /// often the cheapest lossy choice.
    M4a,
    /// Opus. The best quality per byte of the lossy options here.
    Opus,
    /// FLAC: lossless like WAV, roughly half the size, slower to encode.
    Flac,
    /// Whatever the source already provides, kept as-is. The only choice that
    /// does **no** re-encoding, so it is the fastest and the only one that cannot
    /// degrade the audio — at the cost of not knowing the extension in advance.
    Best,
}

impl AudioFormat {
    /// The literal passed to `yt-dlp --audio-format`.
    fn as_arg(self) -> &'static str {
        match self {
            AudioFormat::Wav => "wav",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::M4a => "m4a",
            AudioFormat::Opus => "opus",
            AudioFormat::Flac => "flac",
            AudioFormat::Best => "best",
        }
    }
}

/// How [`download_audio_with_options`] should download.
///
/// Public fields with a `Default`, like every other `*Options` in this suite:
/// build one with `DownloadOptions { audio_format: AudioFormat::Mp3,
/// ..Default::default() }`.
#[derive(Debug, Clone, Default)]
pub struct DownloadOptions {
    /// What to extract the audio to. Defaults to [`AudioFormat::Wav`], which is
    /// what [`download_audio`] has always used.
    pub audio_format: AudioFormat,
}

/// Downloads the audio track of `url` as a WAV file into `out_dir`, creating
/// the directory if it does not exist, and returns the path to the produced
/// file.
///
/// Equivalent to [`download_audio_with_options`] with the default
/// [`DownloadOptions`] (WAV).
///
/// Internally this runs `yt-dlp -x --audio-format wav <url>` with
/// `--print after_move:filepath`, which makes `yt-dlp` print the final file
/// path once its own post-processing (extraction + move into place) is
/// done, instead of this crate guessing the output name from the id/title
/// template.
///
/// # Errors
/// - [`YoutubeHelperError::BinaryNotFound`] if `yt-dlp` is not on `PATH`.
/// - [`YoutubeHelperError::InvalidUrl`] if `url` is empty/malformed, or
///   `yt-dlp` itself rejects it as unsupported.
/// - [`YoutubeHelperError::CommandFailed`] for any other non-zero exit
///   (network failure, removed video, age/region restriction, ...).
/// - [`YoutubeHelperError::OutputFileNotFound`] if `yt-dlp` reported success
///   but the file it printed does not actually exist afterwards.
/// - [`YoutubeHelperError::Io`] if `out_dir` could not be created.
pub fn download_audio(url: &str, out_dir: &Path) -> Result<PathBuf> {
    download_audio_with_options(url, out_dir, &DownloadOptions::default())
}

/// Downloads the audio track of `url` into `out_dir` in the format
/// `options.audio_format` asks for, and returns the path to the produced file.
///
/// Identical to [`download_audio`] in every other respect — including the
/// `--print after_move:filepath` trick that makes `yt-dlp` report the real
/// output path rather than leaving this crate to guess it, which matters more
/// here: with [`AudioFormat::Best`] the extension is not known in advance at all.
///
/// # Errors
/// The same set as [`download_audio`].
pub fn download_audio_with_options(
    url: &str,
    out_dir: &Path,
    options: &DownloadOptions,
) -> Result<PathBuf> {
    ytdlp::validate_url(url)?;
    std::fs::create_dir_all(out_dir)?;

    let template = out_dir.join("%(id)s.%(ext)s");
    let template = template.to_string_lossy().into_owned();

    let output = ytdlp::run(&[
        "-x",
        "--audio-format",
        options.audio_format.as_arg(),
        "--no-playlist",
        "--no-warnings",
        "--print",
        "after_move:filepath",
        "-o",
        &template,
        url,
    ])?;

    if !output.status.success() {
        return Err(ytdlp::map_failure(url, &output));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let printed_path = stdout
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .ok_or_else(|| YoutubeHelperError::OutputFileNotFound {
            directory: out_dir.to_path_buf(),
        })?;

    let path = PathBuf::from(printed_path);
    if !path.is_file() {
        return Err(YoutubeHelperError::OutputFileNotFound {
            directory: out_dir.to_path_buf(),
        });
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_audio_rejects_empty_url() {
        let dir = tempfile::tempdir().unwrap();
        let err = download_audio("", dir.path()).unwrap_err();
        assert!(matches!(err, YoutubeHelperError::InvalidUrl { .. }));
    }

    #[test]
    fn download_audio_reports_missing_binary() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = crate::ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized via `ENV_MUTEX`; see the equivalent test in
        // `metadata.rs`.
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_YTDLP_BIN", "yt-dlp-does-not-exist-anywhere");
        }
        let result = download_audio("https://www.youtube.com/watch?v=jNQXAC9IVRw", dir.path());
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_YTDLP_BIN");
        }
        assert!(matches!(
            result,
            Err(YoutubeHelperError::BinaryNotFound { .. })
        ));
    }

    /// Every variant must reach `yt-dlp` as the literal that tool expects, and
    /// the default must still be WAV — the format `download_audio` has always
    /// produced, so an existing caller sees no change.
    #[test]
    fn the_requested_audio_format_is_the_one_handed_to_ytdlp() {
        assert_eq!(DownloadOptions::default().audio_format, AudioFormat::Wav);

        let out_dir = tempfile::tempdir().unwrap();
        let script_dir = tempfile::tempdir().unwrap();
        let argv_log = script_dir.path().join("argv");
        // The fake records its own argument list, which is the only way to see
        // what this crate actually asked `yt-dlp` for.
        let script = crate::test_support::write_fake_ytdlp(
            script_dir.path(),
            &format!("echo \"$@\" > '{}'\nexit 1\n", argv_log.display()),
        );
        let _guard = crate::ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized via `ENV_MUTEX`.
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_YTDLP_BIN", &script);
        }

        for (format, expected) in [
            (AudioFormat::Wav, "wav"),
            (AudioFormat::Mp3, "mp3"),
            (AudioFormat::M4a, "m4a"),
            (AudioFormat::Opus, "opus"),
            (AudioFormat::Flac, "flac"),
            (AudioFormat::Best, "best"),
        ] {
            let _ = download_audio_with_options(
                "https://example.com/video",
                out_dir.path(),
                &DownloadOptions {
                    audio_format: format,
                },
            );
            let argv = std::fs::read_to_string(&argv_log).expect("the fake recorded its argv");
            assert!(
                argv.contains(&format!("--audio-format {expected}")),
                "{format:?} should reach yt-dlp as `{expected}`, got: {argv}"
            );
        }

        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_YTDLP_BIN");
        }
    }

    #[test]
    fn download_audio_maps_generic_ytdlp_failure_to_command_failed() {
        let out_dir = tempfile::tempdir().unwrap();
        let script_dir = tempfile::tempdir().unwrap();
        let script = crate::test_support::write_fake_ytdlp(
            script_dir.path(),
            "echo 'ERROR: network unreachable' 1>&2\nexit 1\n",
        );
        let _guard = crate::ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized via `ENV_MUTEX`.
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_YTDLP_BIN", &script);
        }
        let result = download_audio("https://example.com/video", out_dir.path());
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_YTDLP_BIN");
        }
        assert!(matches!(
            result,
            Err(YoutubeHelperError::CommandFailed { .. })
        ));
    }

    #[test]
    fn download_audio_errors_when_ytdlp_prints_no_path() {
        let out_dir = tempfile::tempdir().unwrap();
        let script_dir = tempfile::tempdir().unwrap();
        let script = crate::test_support::write_fake_ytdlp(script_dir.path(), "exit 0\n");
        let _guard = crate::ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized via `ENV_MUTEX`.
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_YTDLP_BIN", &script);
        }
        let result = download_audio("https://example.com/video", out_dir.path());
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_YTDLP_BIN");
        }
        assert!(matches!(
            result,
            Err(YoutubeHelperError::OutputFileNotFound { .. })
        ));
    }

    #[test]
    fn download_audio_errors_when_printed_file_does_not_exist() {
        let out_dir = tempfile::tempdir().unwrap();
        let script_dir = tempfile::tempdir().unwrap();
        let missing = out_dir.path().join("ghost.wav");
        let script = crate::test_support::write_fake_ytdlp(
            script_dir.path(),
            &format!("echo '{}'\nexit 0\n", missing.display()),
        );
        let _guard = crate::ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized via `ENV_MUTEX`.
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_YTDLP_BIN", &script);
        }
        let result = download_audio("https://example.com/video", out_dir.path());
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_YTDLP_BIN");
        }
        assert!(matches!(
            result,
            Err(YoutubeHelperError::OutputFileNotFound { .. })
        ));
    }

    #[test]
    fn download_audio_returns_the_path_ytdlp_prints_after_move() {
        let out_dir = tempfile::tempdir().unwrap();
        let script_dir = tempfile::tempdir().unwrap();
        let target = out_dir.path().join("fake.wav");
        let script = crate::test_support::write_fake_ytdlp(
            script_dir.path(),
            &format!(
                "touch '{}'\necho '{}'\nexit 0\n",
                target.display(),
                target.display()
            ),
        );
        let _guard = crate::ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized via `ENV_MUTEX`.
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_YTDLP_BIN", &script);
        }
        let result = download_audio("https://example.com/video", out_dir.path());
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_YTDLP_BIN");
        }
        assert_eq!(result.expect("fake download should succeed"), target);
    }

    /// Real network call, downloads a short public domain video's audio.
    /// Ignored by default; run explicitly with `cargo test -- --ignored`.
    /// As of this writing, YouTube's anti-bot / PO-token enforcement on
    /// direct media CDN URLs can make this fail in sandboxed environments
    /// even when `fetch_metadata` (which does not fetch the media stream)
    /// succeeds; see README.md for details.
    #[test]
    #[ignore]
    fn download_audio_real_video_produces_a_file() {
        // Hold the same lock as the env-var-mutating tests so this test
        // never observes `YOUTUBE_HELPER_YTDLP_BIN` mid-mutation from
        // another thread when run with `--include-ignored`.
        let _guard = crate::ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let path = download_audio("https://www.youtube.com/watch?v=jNQXAC9IVRw", dir.path())
            .expect("download_audio should succeed against a real, stable public video");

        assert!(path.is_file());
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("wav"));
        assert!(path.metadata().unwrap().len() > 0);
    }
}
