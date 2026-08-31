//! # youtube-helper-rs
//!
//! A thin, honest Rust wrapper around the [`yt-dlp`](https://github.com/yt-dlp/yt-dlp)
//! binary, invoked as a subprocess. This crate does not reimplement any part
//! of `yt-dlp`'s extraction logic; it shells out and turns the result into
//! typed Rust values and a `thiserror`-based error enum.
//!
//! v0.1 scope: video metadata retrieval ([`fetch_metadata`]) and audio
//! download ([`download_audio`]). See `README.md` for the full picture of
//! what is and is not covered.

pub mod download;
pub mod error;
pub mod metadata;
#[cfg(test)]
mod test_support;
mod ytdlp;

pub use download::download_audio;
pub use error::{Result, YoutubeHelperError};
pub use metadata::{fetch_metadata, VideoMetadata};

/// Serializes tests (across modules) that mutate the process-wide
/// `YOUTUBE_HELPER_YTDLP_BIN` environment variable, since `cargo test` runs
/// tests concurrently on multiple threads within one process by default.
#[cfg(test)]
pub(crate) static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
