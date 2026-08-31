//! Video metadata retrieval via `yt-dlp --dump-json`.

use crate::error::{Result, YoutubeHelperError};
use crate::ytdlp;
use serde::Deserialize;

/// A subset of the fields `yt-dlp --dump-json` actually emits, verified by
/// hand against a real invocation against a public YouTube video. `yt-dlp`
/// emits dozens more fields (formats, subtitles, chapters, ...); this struct
/// only claims the ones this crate promises to expose. Unknown fields in the
/// JSON are ignored rather than rejected, since the exact field set varies
/// by extractor (YouTube vs. Vimeo vs. DailyMotion, ...).
#[derive(Debug, Clone, Deserialize)]
pub struct VideoMetadata {
    /// Extractor-assigned video id (e.g. the YouTube `v=` value).
    pub id: String,
    /// Video title.
    pub title: String,
    /// Duration in seconds, when the extractor reports one (missing for
    /// some live streams).
    #[serde(default)]
    pub duration: Option<f64>,
    /// Uploader / account display name.
    #[serde(default)]
    pub uploader: Option<String>,
    /// Channel display name (YouTube-specific; `None` on extractors without
    /// the concept of a channel).
    #[serde(default)]
    pub channel: Option<String>,
    /// Canonical page URL for the video.
    #[serde(default)]
    pub webpage_url: Option<String>,
    /// Video description, when provided.
    #[serde(default)]
    pub description: Option<String>,
    /// Upload date as `YYYYMMDD`, when provided.
    #[serde(default)]
    pub upload_date: Option<String>,
    /// View count, when the site exposes it.
    #[serde(default)]
    pub view_count: Option<u64>,
    /// Like count, when the site exposes it.
    #[serde(default)]
    pub like_count: Option<u64>,
    /// Thumbnail URL, when provided.
    #[serde(default)]
    pub thumbnail: Option<String>,
}

/// Fetches metadata for `url` by running `yt-dlp --dump-json <url>` and
/// parsing its stdout. Does not download any media.
///
/// # Errors
/// - [`YoutubeHelperError::BinaryNotFound`] if `yt-dlp` is not on `PATH`.
/// - [`YoutubeHelperError::InvalidUrl`] if `url` is empty/malformed, or
///   `yt-dlp` itself rejects it as unsupported.
/// - [`YoutubeHelperError::CommandFailed`] for any other non-zero exit
///   (network failure, removed video, rate limiting, ...).
/// - [`YoutubeHelperError::JsonParse`] if stdout is not the expected JSON
///   shape.
pub fn fetch_metadata(url: &str) -> Result<VideoMetadata> {
    ytdlp::validate_url(url)?;

    let output = ytdlp::run(&["--dump-json", "--no-playlist", "--no-warnings", url])?;

    if !output.status.success() {
        return Err(ytdlp::map_failure(url, &output));
    }

    // yt-dlp --dump-json writes one JSON object per line for a single
    // video; take the first non-empty line to be robust against any
    // trailing newline or incidental warning text yt-dlp might still emit
    // on stdout.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout
        .lines()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| {
            YoutubeHelperError::CommandFailed {
                status: output.status.to_string(),
                stderr: "yt-dlp produced no JSON output on stdout".to_string(),
            }
        })?;

    let metadata: VideoMetadata = serde_json::from_str(first_line)?;
    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_metadata_rejects_empty_url() {
        let err = fetch_metadata("").unwrap_err();
        assert!(matches!(err, YoutubeHelperError::InvalidUrl { .. }));
    }

    #[test]
    fn fetch_metadata_rejects_url_without_scheme() {
        let err = fetch_metadata("www.youtube.com/watch?v=abc").unwrap_err();
        assert!(matches!(err, YoutubeHelperError::InvalidUrl { .. }));
    }

    #[test]
    fn fetch_metadata_reports_missing_binary() {
        let _guard = crate::ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized against other env-mutating tests via
        // `ENV_MUTEX` above (the env var is process-wide and `cargo test`
        // runs tests concurrently by default).
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_YTDLP_BIN", "yt-dlp-does-not-exist-anywhere");
        }
        let result = fetch_metadata("https://www.youtube.com/watch?v=jNQXAC9IVRw");
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_YTDLP_BIN");
        }
        assert!(matches!(result, Err(YoutubeHelperError::BinaryNotFound { .. })));
    }

    #[test]
    fn fetch_metadata_maps_generic_ytdlp_failure_to_command_failed() {
        let script_dir = tempfile::tempdir().unwrap();
        let script = crate::test_support::write_fake_ytdlp(
            script_dir.path(),
            "echo 'ERROR: rate limited' 1>&2\nexit 1\n",
        );
        let _guard = crate::ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized via `ENV_MUTEX`.
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_YTDLP_BIN", &script);
        }
        let result = fetch_metadata("https://example.com/video");
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_YTDLP_BIN");
        }
        assert!(matches!(result, Err(YoutubeHelperError::CommandFailed { .. })));
    }

    #[test]
    fn fetch_metadata_errors_when_ytdlp_prints_no_json() {
        let script_dir = tempfile::tempdir().unwrap();
        let script = crate::test_support::write_fake_ytdlp(script_dir.path(), "exit 0\n");
        let _guard = crate::ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized via `ENV_MUTEX`.
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_YTDLP_BIN", &script);
        }
        let result = fetch_metadata("https://example.com/video");
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_YTDLP_BIN");
        }
        assert!(matches!(result, Err(YoutubeHelperError::CommandFailed { .. })));
    }

    #[test]
    fn fetch_metadata_parses_a_minimal_fake_ytdlp_response() {
        let script_dir = tempfile::tempdir().unwrap();
        let script = crate::test_support::write_fake_ytdlp(
            script_dir.path(),
            r#"echo '{"id":"abc123","title":"Fake Title","duration":42.0,"uploader":"Someone"}'
exit 0
"#,
        );
        let _guard = crate::ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized via `ENV_MUTEX`.
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_YTDLP_BIN", &script);
        }
        let result = fetch_metadata("https://example.com/video");
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_YTDLP_BIN");
        }
        let metadata = result.expect("fake yt-dlp JSON should parse");
        assert_eq!(metadata.id, "abc123");
        assert_eq!(metadata.title, "Fake Title");
        assert_eq!(metadata.duration, Some(42.0));
        assert_eq!(metadata.uploader.as_deref(), Some("Someone"));
    }

    /// Real network call against a stable, long-lived public YouTube video
    /// (the first video ever uploaded to YouTube, "Me at the zoo"). Ignored
    /// by default; run explicitly with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn fetch_metadata_real_video_has_expected_fields() {
        // Hold the same lock as the env-var-mutating tests so this test
        // never observes `YOUTUBE_HELPER_YTDLP_BIN` mid-mutation from
        // another thread when run with `--include-ignored`.
        let _guard = crate::ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let metadata = fetch_metadata("https://www.youtube.com/watch?v=jNQXAC9IVRw")
            .expect("fetch_metadata should succeed against a real, stable public video");

        assert_eq!(metadata.id, "jNQXAC9IVRw");
        assert!(!metadata.title.is_empty());
        assert!(metadata.duration.unwrap_or(0.0) > 0.0);
        assert!(metadata.uploader.is_some());
        assert!(metadata.webpage_url.unwrap_or_default().contains("jNQXAC9IVRw"));
    }
}
