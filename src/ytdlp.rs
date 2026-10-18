//! Internal process-spawning helper shared by [`crate::metadata`] and
//! [`crate::download`]. Owns exactly one concern: invoke the `yt-dlp`
//! binary with a given argument list and turn the raw `std::process::Output`
//! into a `Result` that already distinguishes "binary not found" from
//! "binary ran but failed".

use crate::error::{Result, YoutubeHelperError};
use std::process::{Command, Output, Stdio};

/// Name (or path) of the `yt-dlp` binary to invoke.
///
/// `YOUTUBE_HELPER_YTDLP_BIN` still wins, and still lets a test point at a
/// binary that does not exist to exercise
/// [`YoutubeHelperError::BinaryNotFound`] deterministically. Without it, the
/// answer now comes from [`crate::provision::ytdlp`], which finds `yt-dlp` on
/// `PATH` or fetches the official standalone build — see that module for why a
/// crate whose every function needs a binary should not merely document it.
///
/// A provisioning failure falls back to the bare name rather than being
/// propagated here: the spawn that follows produces the error the caller
/// already knows how to read, with the real reason attached to it.
pub(crate) fn binary_name() -> String {
    crate::provision::ytdlp()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "yt-dlp".to_string())
}

/// Seconds a single `yt-dlp` socket read may stall before it gives up.
///
/// `yt-dlp` applies no timeout unless asked, so a connection the remote has
/// half-closed is waited on forever and the calling process simply stops making
/// progress. The Python twin hit exactly this: a caption fetch sat at 0 % CPU for
/// nine minutes with its sockets in `CLOSE_WAIT` and had to be killed. Nothing
/// about that failure is language-specific -- this crate drives the same binary
/// over the same sockets -- so it is bounded here too, and under the same
/// environment variable.
///
/// 30 s is far past any healthy response while still failing fast enough that a
/// caller can retry.
fn socket_timeout() -> String {
    std::env::var("YOUTUBE_HELPER_SOCKET_TIMEOUT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "30".to_string())
}

/// Runs `yt-dlp` with the given arguments and returns the raw output,
/// mapping a spawn failure caused by a missing binary to
/// [`YoutubeHelperError::BinaryNotFound`].
///
/// Every call in the crate funnels through here, which is why the socket bound
/// lives here rather than at each call site: one place to get right, and no way
/// for a new caller to forget it.
pub(crate) fn run(args: &[&str]) -> Result<Output> {
    let binary = binary_name();
    let timeout = socket_timeout();
    let output = Command::new(&binary)
        .arg("--socket-timeout")
        .arg(&timeout)
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

    #[test]
    fn socket_timeout_defaults_to_a_bounded_value() {
        // The point is that it is never unbounded: yt-dlp left to itself waits
        // forever on a half-closed socket.
        let _guard = crate::ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized via `ENV_MUTEX`.
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_SOCKET_TIMEOUT");
        }
        let value = socket_timeout();
        assert!(
            value.parse::<f64>().is_ok_and(|v| v > 0.0),
            "default socket timeout must be a positive number, got {value:?}"
        );
    }

    #[test]
    fn socket_timeout_is_overridable() {
        let _guard = crate::ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized via `ENV_MUTEX`.
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_SOCKET_TIMEOUT", "120");
        }
        let value = socket_timeout();
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_SOCKET_TIMEOUT");
        }
        assert_eq!(value, "120");
    }

    #[test]
    fn a_blank_override_falls_back_to_the_default() {
        // An exported-but-empty variable is a common shell accident; it must not
        // turn into `--socket-timeout ""`, which yt-dlp rejects.
        let _guard = crate::ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: serialized via `ENV_MUTEX`.
        unsafe {
            std::env::set_var("YOUTUBE_HELPER_SOCKET_TIMEOUT", "   ");
        }
        let value = socket_timeout();
        unsafe {
            std::env::remove_var("YOUTUBE_HELPER_SOCKET_TIMEOUT");
        }
        assert!(value.parse::<f64>().is_ok_and(|v| v > 0.0));
    }
}
