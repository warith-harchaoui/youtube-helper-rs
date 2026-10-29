# Landscape

[🇫🇷 PAYSAGE.md](https://github.com/warith-harchaoui/youtube-helper-rs/blob/master/PAYSAGE.md) · 🇬🇧 English

Related and competing crates in the "drive yt-dlp from Rust" space, benchmarked against `youtube-helper-rs`.
Ratings are ⭐ (1) to ⭐⭐⭐⭐⭐ (5), scored on `youtube-helper-rs`'s intended job:
video metadata, audio download and direct media URL resolution, by shelling out to the real yt-dlp binary rather than reimplementing any of its extraction logic. A crate optimised for a very different job is not penalised; the score
just reflects fit to *this* niche.

## At a glance

<!-- TABLE:START -->
| Rust yt-dlp Wrappers | Metadata | Audio download | Video download | Playlist support | Live stream resolve | Binary provisioning | Sync (no forced runtime) | Typed errors |
| --- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **youtube-helper-rs** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ | ⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| yt-dlp | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ | ⭐⭐⭐ |
| ytd-rs | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐ | ⭐ | ⭐⭐⭐ |
| rustypipe | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ | ⭐⭐⭐⭐ |
| ytextract | ⭐⭐⭐⭐⭐ | ⭐ | ⭐ | ⭐⭐⭐ | ⭐ | ⭐⭐⭐⭐⭐ | ⭐ | ⭐⭐⭐ |
| ytdown | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐⭐ | ⭐⭐ | ⭐⭐ |
<!-- TABLE:END -->

## Positioning map

<!-- FIGURE:START -->
2D representation of the table above.

![Positioning map](https://raw.githubusercontent.com/warith-harchaoui/youtube-helper-rs/master/assets/landscape.png)

The map is a 2-D summary of the eight criteria, so read it as a shape, not a scoreboard. `youtube-helper-rs` is at the top-right corner. The axes read **Horizontal: Convenience ↔ Precision** and **Vertical: Versatility ↔ Efficiency**.

The horizontal axis is led by binary provisioning and metadata, with video download loading against it; the vertical axis is led by audio download and live-stream resolution, with playlist support loading against it. Read plainly: the top half can hand you audio, and the right half asks least of your machine while being clearest about failure.

`youtube-helper-rs` and the `yt-dlp` crate are the only two in both halves. They separate on scope — the `yt-dlp` crate does video and is the honest recommendation when video is the requirement, while this crate stays on audio, stays synchronous, and leaves the runtime choice to the caller. `ytd-rs` sits top-left: it downloads, but expects you to have the binary and brings tokio with it.

`rustypipe` and `ytextract` occupy the bottom right because they need no binary at all — they speak YouTube's Innertube API directly — and because neither hands back an audio file. That makes `rustypipe` the stronger pick for metadata and playlists, and the weaker one the day YouTube changes that API, which is precisely the risk yt-dlp absorbs on this crate's behalf.
<!-- FIGURE:END -->

## Where the claims come from

Versions and publication dates read from the crates.io API on 29 October 2026:

| Crate | Version | Last published |
|---|---|---|
| `yt-dlp` | 2.9.1 | 2026-10-26 |
| `ytd-rs` | 0.2.1 | 2026-03-22 |
| `rustypipe` | 0.11.4 | 2025-04-23 |
| `ytextract` | 0.11.2 | 2023-01-29 |
| `ytdown` | 0.8.1 | 2026-09-24 |

`youtube-helper-rs`'s own ratings are from this repository at version 0.1.6.

- **The two 1s — video download and playlists — are the v0.1 scope line, not an oversight.** The README says so in the same words. The `yt-dlp` crate scores 5 on video and is the honest recommendation when video is the requirement.

- **`Sync (no forced runtime)` is a fit-to-niche column, not a verdict on async.** `yt-dlp`, `ytd-rs` and `rustypipe` score 1 because they bring a tokio runtime into a library consumer that may not want one; this crate blocks on `std::process::Command` and leaves the choice to the caller.

- **`rustypipe` scores 5 on provisioning by not needing a binary at all** — it speaks YouTube's Innertube API directly. That makes it the stronger pick for metadata and playlists, and the weaker one the day YouTube changes the API, which is the risk yt-dlp absorbs on this crate's behalf.

- **`ytextract` was last published 2023-01-29** — the oldest release in this table by two years, recorded because a stale extractor is the one failure mode this whole category shares.

Deliberately left as opinion rather than sourced: the *pipeline ergonomics*
style columns, and the ordering of the rows below the reference.

## Ideas worth stealing

Researched rather than guessed: what each neighbour does better, and what this crate would have to change to match it. Nothing here is a commitment — it is the reading list the next version starts from.

- **the `yt-dlp` crate — provision ffmpeg too, not just yt-dlp** — `provision.rs` here finds or fetches `yt-dlp` and stops there. The `yt-dlp` crate downloads its dependencies including ffmpeg. Since `podcast-helper-rs` delegates to this crate and then needs ffmpeg anyway, the same four-step search applied to ffmpeg would close a gap that currently surfaces one layer up.

- **`rustypipe` — a metadata fast path with no binary at all** — It speaks the Innertube API directly, so metadata and playlists cost no subprocess and no download. `fetch_metadata` is the cheapest call this crate offers and the most likely to be made in bulk; an Innertube path behind a feature would make it cheaper still, with yt-dlp as the fallback when the API moves.

- **`ytd-rs` — a builder for options instead of a widening struct** — `DownloadOptions` grows a field per capability. A builder reads better at the call site and lets options be added without the struct literal in every caller changing shape.
