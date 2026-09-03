//! Audio download via `yt-dlp -x --audio-format wav`.

use crate::error::{Result, YoutubeHelperError};
use crate::ytdlp;
use std::path::{Path, PathBuf};

/// Downloads the audio track of `url` as a WAV file into `out_dir`, creating
/// the directory if it does not exist, and returns the path to the produced
/// file.
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
    ytdlp::validate_url(url)?;
    std::fs::create_dir_all(out_dir)?;

    let template = out_dir.join("%(id)s.%(ext)s");
    let template = template.to_string_lossy().into_owned();

    let output = ytdlp::run(&[
        "-x",
        "--audio-format",
        "wav",
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
