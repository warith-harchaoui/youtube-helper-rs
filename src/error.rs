//! Every fallible operation in this crate crosses [`YoutubeHelperError`]. It is
//! built to keep the failure modes of shelling out to `yt-dlp` honest and
//! distinguishable instead of collapsing them into one opaque "it failed":
//! a missing binary, an unsupported/invalid URL, a network-level failure
//! reported by `yt-dlp` itself, a JSON parse failure, and filesystem I/O each
//! get their own variant so a caller can branch on `match` instead of on
//! string content.

use std::path::PathBuf;
use thiserror::Error;

/// Errors produced by this crate.
#[derive(Debug, Error)]
pub enum YoutubeHelperError {
    /// The `yt-dlp` binary could not be found or executed (e.g. not on `PATH`).
    #[error("yt-dlp binary not found (looked for `{binary}`): {source}")]
    BinaryNotFound {
        binary: String,
        #[source]
        source: std::io::Error,
    },

    /// The URL given to a function is empty, malformed, or rejected by
    /// `yt-dlp` as unsupported before any network call could complete.
    #[error("invalid or unsupported URL `{url}`: {reason}")]
    InvalidUrl { url: String, reason: String },

    /// `yt-dlp` ran but exited with a non-zero status for a reason other
    /// than an invalid URL (network failure, geo-blocking, age restriction,
    /// removed video, rate limiting, ...). The raw stderr is preserved so
    /// the caller can decide how to react.
    #[error("yt-dlp command failed (exit status: {status}): {stderr}")]
    CommandFailed { status: String, stderr: String },

    /// `yt-dlp --dump-json` succeeded but its stdout could not be parsed as
    /// the expected JSON shape.
    #[error("failed to parse yt-dlp JSON output: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// `yt-dlp` reported success but the expected output file could not be
    /// located afterwards.
    #[error("expected output file not found after download in `{directory}`")]
    OutputFileNotFound { directory: PathBuf },

    /// Any other I/O failure (creating the output directory, reading it
    /// back, ...).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, YoutubeHelperError>;
