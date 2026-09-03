//! Internal process-spawning helper shared by [`crate::metadata`] and
//! [`crate::download`]. Owns exactly one concern: invoke the `yt-dlp`
//! binary with a given argument list and turn the raw `std::process::Output`
//! into a `Result` that already distinguishes "binary not found" from
//! "binary ran but failed".

use crate::error::{Result, YoutubeHelperError};
use std::process::{Command, Output, Stdio};

/// Name (or path) of the `yt-dlp` binary to invoke. Overridable via the
/// `YOUTUBE_HELPER_YTDLP_BIN` environment variable, primarily so tests can
/// point at a binary that does not exist to exercise [`YoutubeHelperError::BinaryNotFound`]
/// deterministically without touching a real install.
pub(crate) fn binary_name() -> String {
    std::env::var("YOUTUBE_HELPER_YTDLP_BIN").unwrap_or_else(|_| "yt-dlp".to_string())
}

/// Runs `yt-dlp` with the given arguments and returns the raw output,
/// mapping a spawn failure caused by a missing binary to
/// [`YoutubeHelperError::BinaryNotFound`].
pub(crate) fn run(args: &[&str]) -> Result<Output> {
    let binary = binary_name();
    let output = Command::new(&binary)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                YoutubeHelperError::BinaryNotFound {
                    binary: binary.clone(),
                    source,
                }
            } else {
                YoutubeHelperError::Io(source)
            }
        })?;
    Ok(output)
}

/// Basic sanity check performed before ever spawning a process: reject
/// obviously-invalid URLs up front (empty, no recognizable scheme) instead
/// of paying for a process spawn just to fail the same way.
pub(crate) fn validate_url(url: &str) -> Result<()> {
    if url.trim().is_empty() {
        return Err(YoutubeHelperError::InvalidUrl {
            url: url.to_string(),
            reason: "URL is empty".to_string(),
        });
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(YoutubeHelperError::InvalidUrl {
            url: url.to_string(),
            reason: "URL must start with http:// or https://".to_string(),
        });
    }
    Ok(())
}

/// Maps a non-zero `yt-dlp` exit into the right error variant: `yt-dlp`
/// reports unsupported/invalid URLs as a recognizable stderr message, so
/// that case gets its own [`YoutubeHelperError::InvalidUrl`] instead of the
/// generic [`YoutubeHelperError::CommandFailed`].
pub(crate) fn map_failure(url: &str, output: &Output) -> YoutubeHelperError {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if stderr.contains("Unsupported URL") || stderr.contains("is not a valid URL") {
        YoutubeHelperError::InvalidUrl {
            url: url.to_string(),
            reason: stderr,
        }
    } else {
        YoutubeHelperError::CommandFailed {
            status: output.status.to_string(),
            stderr,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a real, non-zero `ExitStatus` (there's no public constructor
    /// for one, so we get one the honest way: actually run a failing
    /// command) paired with a caller-chosen stderr, to test `map_failure`'s
    /// message-based branching without needing a real `yt-dlp` failure.
    fn failed_output_with_stderr(stderr: &str) -> Output {
        let status = Command::new("false").status().expect("run `false`");
        Output {
            status,
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[test]
    fn map_failure_detects_unsupported_url_message() {
        let output = failed_output_with_stderr("ERROR: [generic] is not a valid URL");
        let err = map_failure("https://example.com/bad", &output);
        assert!(matches!(err, YoutubeHelperError::InvalidUrl { .. }));
    }

    #[test]
    fn map_failure_defaults_to_command_failed() {
        let output = failed_output_with_stderr("ERROR: network unreachable");
        let err = map_failure("https://example.com/video", &output);
        assert!(matches!(err, YoutubeHelperError::CommandFailed { .. }));
    }

    #[test]
    fn validate_url_accepts_http_and_https() {
        assert!(validate_url("http://example.com").is_ok());
        assert!(validate_url("https://example.com").is_ok());
    }

    #[test]
    fn run_maps_non_notfound_spawn_error_to_io_variant() {
        // Pointing the "binary" at a directory makes the OS reject the
        // exec with something other than `NotFound` (e.g. "is a
        // directory" / permission denied), exercising the `else` branch
        // in `run`'s error mapping.
        let dir = tempfile::tempdir().unwrap();
        let _guard = crate::ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized via `ENV_MUTEX`.
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_YTDLP_BIN", dir.path());
        }
        let result = run(&["--version"]);
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_YTDLP_BIN");
        }
        assert!(matches!(result, Err(YoutubeHelperError::Io(_))));
    }
}
