//! Direct media URL resolution via `yt-dlp --get-url`, for callers that want
//! to **stream** rather than download.
//!
//! WHY THIS EXISTS ALONGSIDE [`crate::download_audio`]. Downloading works when
//! the media ends. A live broadcast does not: `download_audio` would block
//! until the stream stops, which for a live event means "never". Resolution
//! returns the address of the media itself, which `ffmpeg` (or any HLS client)
//! can then follow for as long as it keeps producing — including indefinitely.
//!
//! The returned URL is short-lived. YouTube signs its media URLs and expires
//! them within hours, so resolve right before use and never persist the
//! result: a URL cached yesterday fails today with an opaque HTTP 403.

use crate::error::{Result, YoutubeHelperError};
use crate::ytdlp;

/// The format selector handed to `yt-dlp`.
///
/// `bestaudio/best` is yt-dlp's fallback syntax: take the best audio-only
/// stream, and if the extractor offers none, take the best combined stream.
/// The fallback is not theoretical — many live HLS broadcasts publish only
/// muxed renditions, and asking for `bestaudio` alone fails on exactly the
/// sources this function exists to serve. A combined stream costs bandwidth
/// a caller who only wants audio does not need, which is why audio-only is
/// tried first rather than simply asking for `best`.
const FORMAT: &str = "bestaudio/best";

/// Resolves `url` to a direct media URL that a streaming client can read,
/// without downloading anything.
///
/// Runs `yt-dlp --get-url -f bestaudio/best --no-playlist <url>`. Works on any
/// site `yt-dlp` supports, live broadcasts included — which is the case that
/// motivates it, since a live stream never finishes downloading.
///
/// The result is a signed, expiring URL: resolve immediately before use.
///
/// # Errors
/// - [`YoutubeHelperError::BinaryNotFound`] if `yt-dlp` is not on `PATH`.
/// - [`YoutubeHelperError::InvalidUrl`] if `url` is empty/malformed, or
///   `yt-dlp` itself rejects it as unsupported.
/// - [`YoutubeHelperError::CommandFailed`] for any other non-zero exit
///   (network failure, removed video, members-only broadcast, ...).
/// - [`YoutubeHelperError::OutputFileNotFound`] — reused here for its literal
///   meaning, "the command succeeded but produced nothing we can use" — if
///   `yt-dlp` exits zero while printing no URL at all. This happens on a
///   scheduled premiere that has not started: the extractor knows the page and
///   has nothing to hand out yet.
pub fn resolve_media_url(url: &str) -> Result<String> {
    ytdlp::validate_url(url)?;

    let output = ytdlp::run(&[
        "--get-url",
        "-f",
        FORMAT,
        "--no-playlist",
        "--no-warnings",
        url,
    ])?;

    if !output.status.success() {
        return Err(ytdlp::map_failure(url, &output));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The LAST non-empty line, not the first. A format selector that resolves
    // to separate video and audio tracks makes `yt-dlp` print both, video
    // first; taking the first line would hand a caller who asked for audio a
    // video-only stream with no sound at all — a failure that looks like a
    // silent recording rather than an error.
    let resolved = stdout
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .ok_or_else(|| YoutubeHelperError::OutputFileNotFound {
            directory: std::path::PathBuf::from(url),
        })?;

    Ok(resolved.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::write_fake_ytdlp;

    fn with_fake_ytdlp<T>(body: &str, call: impl FnOnce() -> T) -> T {
        let _guard = crate::ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempfile::tempdir().expect("tempdir");
        let script = write_fake_ytdlp(dir.path(), body);
        // SAFETY: serialized via `ENV_MUTEX`.
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_YTDLP_BIN", &script);
        }
        let out = call();
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_YTDLP_BIN");
        }
        out
    }

    #[test]
    fn a_single_printed_url_is_returned_as_is() {
        let resolved = with_fake_ytdlp(
            "echo 'https://rr3.googlevideo.com/videoplayback?x=1'\n",
            || resolve_media_url("https://www.youtube.com/watch?v=abc"),
        )
        .expect("a successful resolve should return the URL");
        assert_eq!(resolved, "https://rr3.googlevideo.com/videoplayback?x=1");
    }

    #[test]
    fn two_printed_urls_yield_the_audio_one_not_the_video_one() {
        // When the selector resolves to separate tracks, yt-dlp prints video
        // first and audio second. Returning the first would hand the caller a
        // stream with no sound — a failure that looks like a silent recording
        // rather than an error, so it would be found late and blamed on the
        // microphone.
        let resolved = with_fake_ytdlp(
            "echo 'https://example.invalid/video-only'\necho 'https://example.invalid/audio-only'\n",
            || resolve_media_url("https://www.youtube.com/watch?v=abc"),
        )
        .expect("a successful resolve should return a URL");
        assert_eq!(resolved, "https://example.invalid/audio-only");
    }

    #[test]
    fn trailing_blank_lines_do_not_become_the_answer() {
        let resolved = with_fake_ytdlp(
            "echo 'https://example.invalid/audio'\necho ''\necho '  '\n",
            || resolve_media_url("https://www.youtube.com/watch?v=abc"),
        )
        .expect("blank trailing output should be ignored");
        assert_eq!(resolved, "https://example.invalid/audio");
    }

    #[test]
    fn success_without_any_url_is_an_error_and_not_an_empty_string() {
        // A scheduled premiere that has not started exits zero and prints
        // nothing. Returning "" here would hand ffmpeg an empty address and
        // turn a clear "nothing to stream yet" into an obscure ffmpeg failure.
        let err = with_fake_ytdlp("exit 0\n", || {
            resolve_media_url("https://www.youtube.com/watch?v=abc")
        })
        .expect_err("empty stdout on success must be an error");
        assert!(
            matches!(err, YoutubeHelperError::OutputFileNotFound { .. }),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn an_unsupported_url_keeps_its_own_error_variant() {
        let err = with_fake_ytdlp("echo 'ERROR: Unsupported URL: x' >&2\nexit 1\n", || {
            resolve_media_url("https://example.com/not-a-video")
        })
        .expect_err("yt-dlp rejecting the URL must surface as InvalidUrl");
        assert!(
            matches!(err, YoutubeHelperError::InvalidUrl { .. }),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn an_empty_url_never_spawns_a_process() {
        // No fake binary is installed here on purpose: if validation did not
        // short-circuit, this would fail with BinaryNotFound (or worse, spawn
        // a real yt-dlp) instead of InvalidUrl.
        let err = resolve_media_url("   ").expect_err("an empty URL must be rejected");
        assert!(
            matches!(err, YoutubeHelperError::InvalidUrl { .. }),
            "unexpected error: {err}"
        );
    }

    /// Real network call against a stable, long-lived public YouTube video.
    /// Ignored by default; run explicitly with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn resolve_media_url_real_video_returns_a_playable_address() {
        let _guard = crate::ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let resolved = resolve_media_url("https://www.youtube.com/watch?v=jNQXAC9IVRw")
            .expect("resolve should succeed against a real, stable public video");
        assert!(
            resolved.starts_with("https://"),
            "expected an https media URL, got: {resolved}"
        );
    }
}
