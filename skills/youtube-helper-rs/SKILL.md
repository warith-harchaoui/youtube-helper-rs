---
name: youtube-helper-rs
description: >-
  Typed Rust access to yt-dlp: read a video's metadata, resolve a direct player-readable media URL without downloading, and download the audio track as wav, mp3, m4a, opus, flac or the untouched source. Works on any site yt-dlp reaches (YouTube, Vimeo, Twitch, SoundCloud, direct media URLs). Use when the user is writing Rust and says "get this video's title or duration", "a direct URL I can hand to ffmpeg or a player", "download the audio from this link", "extract the mp3", or names cargo add youtube-helper-rs, cargo install youtube-helper-rs, youtube_helper_rs, or "yt-dlp from Rust". Do NOT use for the video track, thumbnails, subtitles, channel listings or engagement figures -- the Python twin youtube-helper covers those. Do NOT use to transcribe (vocal-helper), to edit the downloaded file (audio-helper-rs), or to read a podcast feed (podcast-helper-rs).
license: BSD-3-Clause
metadata:
  author: Warith Harchaoui
  contact: warithmetics@deraison.ai
  repository: https://github.com/warith-harchaoui/youtube-helper-rs
  crate: https://crates.io/crates/youtube-helper-rs
  language: rust
---

# youtube-helper-rs

A typed Rust wrapper around the `yt-dlp` binary. It never reimplements
extraction: `yt-dlp` already knows how to talk to hundreds of sites, and this
crate's job is to invoke it, parse its output into Rust types, and surface
failures as a `thiserror` enum rather than a string.

## Before anything else: the binary

Every function ends in a call to `yt-dlp`. The crate resolves it the first time
one is needed, in this order: `YOUTUBE_HELPER_YTDLP_BIN`, then `PATH`, then a
provisioned copy. If resolution fails you get
`YoutubeHelperError::BinaryNotFound`, not a panic — report that to the user
plainly, because the fix is on their machine, not in their code.

## The three operations

| Intent | CLI | Library |
|--------|-----|---------|
| Typed metadata for a URL | `youtube-helper-rs metadata` | `fetch_metadata` |
| Direct, player-readable media URL | `youtube-helper-rs resolve` | `resolve_media_url` |
| Download the audio track | `youtube-helper-rs download` | `download_audio_with_options` |

Full flags and the output contract: `references/cli-reference.md`.

## Writing Rust against it

```rust
use youtube_helper_rs::{fetch_metadata, download_audio_with_options,
                        AudioFormat, DownloadOptions};

let meta = fetch_metadata("https://www.youtube.com/watch?v=...")?;
println!("{} ({:?}s)", meta.title, meta.duration);

let options = DownloadOptions { audio_format: AudioFormat::Mp3, ..Default::default() };
let path = download_audio_with_options(url, Path::new("out"), &options)?;
```

`AudioFormat` is `#[non_exhaustive]` on purpose, so adding a format later is not
a breaking change. Match on it with a `_` arm.

`AudioFormat::Best` is the only variant that does **no** re-encoding: it keeps
whatever the source already serves. Prefer it when the goal is to archive the
audio untouched, and prefer `Wav` when the next step is a decoder or an ML
pipeline that wants PCM.

## Choosing this crate over the Python twin

Pick this one when the calling code is Rust, or when a single binary with no
Python runtime matters. Pick the Python `youtube-helper` when you need the video
track, thumbnails, subtitles, channel listings, engagement figures, the stream
catalogue, the HTTP API, MCP, or the GUI — it is the complete surface, this is
the core promise.

Both install side by side: this binary is named `youtube-helper-rs`, not
`youtube-helper`, so neither shadows the other on `PATH`.

## When not to reach for it

See `references/triggers.md` for the exhaustive catalogue, including the SKIP
list. In short: no transcription, no editing, no video track, no channel data,
and never as a substitute for `yt-dlp` itself.

## References

- `references/triggers.md` — the full trigger catalogue, mirroring the repo's
  root `TRIGGERS.md`.
- `references/cli-reference.md` — every subcommand, flag and exit code.
- `references/surfaces.md` — which surfaces exist here, and which deliberately
  do not.
