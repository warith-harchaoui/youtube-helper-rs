//! # youtube-helper-rs
//!
//! A thin, honest Rust wrapper around the [`yt-dlp`](https://github.com/yt-dlp/yt-dlp)
//! binary, invoked as a subprocess. This crate does not reimplement any part
//! of `yt-dlp`'s extraction logic; it shells out and turns the result into
//! typed Rust values and a `thiserror`-based error enum.
//!
//! v0.1 scope: video metadata retrieval ([`fetch_metadata`]), audio download
//! ([`download_audio`]) and direct media URL resolution
//! ([`resolve_media_url`]). See `README.md` for the full picture of what is
//! and is not covered.
//!
//! Download and resolve answer two different questions, and picking the wrong
//! one fails in a way that is hard to read. **Download** when the media ends
//! and you want a file. **Resolve** when you want to stream, and especially
//! when the media may not end: a live broadcast never finishes downloading, so
//! [`download_audio`] on one blocks forever, while [`resolve_media_url`]
//! returns an address a player can follow for as long as the broadcast lasts.

// Every public item carries a doc comment. Warned here, denied by CI's
// `-D warnings`, so a published API never reaches docs.rs undocumented.
#![warn(missing_docs)]

pub mod download;
pub mod error;
pub mod metadata;
pub mod resolve;
#[cfg(test)]
mod test_support;
mod ytdlp;

pub use download::download_audio;
pub use error::{Result, YoutubeHelperError};
pub use metadata::{fetch_metadata, VideoMetadata};
pub use resolve::resolve_media_url;

/// Serializes tests (across modules) that mutate the process-wide
/// `YOUTUBE_HELPER_YTDLP_BIN` environment variable, since `cargo test` runs
/// tests concurrently on multiple threads within one process by default.
#[cfg(test)]
pub(crate) static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
