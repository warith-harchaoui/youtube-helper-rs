# Trigger catalogue — youtube-helper-rs

The agent-facing mirror of the repository's root `TRIGGERS.md`. The two are
kept identical below the heading: when one changes, change the other. The
skill's `description` is the compressed form the host model sees before it
decides to load anything; this file is the reviewable superset.

The exhaustive catalogue of what `youtube-helper-rs` does and what should reach
for it. The Python twin, [`youtube-helper`](https://github.com/warith-harchaoui/youtube-helper),
has its own catalogue; the two overlap, and the section below on choosing
between them is the part that matters.

`youtube-helper-rs` is a typed Rust wrapper around the `yt-dlp` binary. It does
not reimplement any extraction logic — `yt-dlp` already knows how to talk to
hundreds of sites — and it does not transcribe, edit or transcode beyond what
`yt-dlp -x` performs.

## The three operations → how to invoke

| Intent | CLI | Library |
|--------|-----|---------|
| Read typed metadata for a URL | `youtube-helper-rs metadata` | `fetch_metadata` |
| Get a direct, player-readable media URL | `youtube-helper-rs resolve` | `resolve_media_url` |
| Download the audio track | `youtube-helper-rs download` | `download_audio`, `download_audio_with_options` |

There is no HTTP, MCP or GUI surface here, and that is deliberate — see
"Choosing between the Rust crate and the Python twin" below.

## Natural-language phrasings that should fire

- **Metadata**: "what is the title / duration / uploader of this URL", "read
  the video metadata in Rust", "give me a typed struct for this video".
- **Resolve**: "get a direct media URL without downloading", "a URL I can hand
  straight to a player or to ffmpeg", "stream it instead of saving it".
- **Download**: "download the audio from this link", "get the mp3 / wav / flac
  / opus from this video", "extract the audio track", "rip the sound".
- **The crate itself**: `cargo add youtube-helper-rs`, `cargo install
  youtube-helper-rs`, "the Rust youtube helper", "youtube_helper_rs".
- **The binary it drives**: "yt-dlp from Rust", "wrap yt-dlp in a typed API",
  "yt-dlp without shelling out by hand".

## What it accepts

Any URL `yt-dlp` can extract — YouTube, Vimeo, DailyMotion, Twitch, SoundCloud,
direct media URLs, and the several hundred other sites in `yt-dlp`'s extractor
list. A URL must start with `http://` or `https://`; anything else is rejected
before `yt-dlp` is ever invoked.

## Choosing between the Rust crate and the Python twin

Reach for **`youtube-helper-rs`** when the calling code is already Rust, when
you want a single static binary with no Python runtime to provision, or when
the typed `VideoMetadata` struct and `thiserror` enum are worth more than
breadth of features.

Reach for the **Python `youtube-helper`** when you need anything the Rust crate
does not have: video downloads (not just the audio track), thumbnails,
subtitles, channel listings, engagement figures, the stream catalogue and
picker, the HTTP API, the MCP tool set, or the browser GUI. The Python twin is
the complete surface; this crate is the core promise, in Rust.

Both can be installed side by side. The binary here is deliberately named
`youtube-helper-rs` rather than `youtube-helper`, so neither shadows the other
on `PATH`.

## When NOT to use youtube-helper-rs (SKIP)

- Transcription, captions or speech-to-text → `vocal-helper`.
- Editing, converting or trimming the file once downloaded → `audio-helper-rs`
  (Rust) or `audio-helper` / `video-helper` (Python).
- Reading a podcast RSS feed → `podcast-helper-rs` (which builds on this crate)
  or `podcast-helper`.
- Downloading the **video** track, thumbnails, subtitles, or anything about a
  channel → the Python `youtube-helper`.
- Re-implementing extraction for a site `yt-dlp` does not support. This crate
  wraps `yt-dlp`; it is not an extractor.

## See also

- [`README.md`](README.md) / [`LISEZMOI.md`](LISEZMOI.md): features, install,
  and how the `yt-dlp` binary is provisioned rather than merely required.
- [`CHANGELOG.md`](CHANGELOG.md): what changed, version by version.
