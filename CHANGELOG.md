# Changelog

All notable changes to `youtube-helper-rs` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project adheres to
[Semantic Versioning](https://semver.org/).

## [0.1.5] — 2026-10-19

### Fixed
- **The yt-dlp binary was fetched with no timeout.** Provisioning tried
  `curl -fsSL --retry 3` then `wget -q -O`. curl applies no total-transfer limit
  by default, so a stalled connection is waited on forever — and `--retry 3`
  never fires, because a retry only happens once an attempt *returns*. wget does
  give up, but its default read timeout is 900 s, which to anyone watching is
  indistinguishable from a hang.

  The blast radius is wider than the function. `binary_name()` calls into
  provisioning lazily on the first yt-dlp invocation, so an unbounded fetch here
  hangs the whole crate at first use, not just this call. Now
  `--connect-timeout 20 --max-time 300` for curl and
  `--connect-timeout=20 --read-timeout=60 --tries=3` for wget: 300 s tolerates a
  slow link for a ~30 MB asset while still being a bound.

  Same root cause as 0.1.4's `--socket-timeout`, one layer further out — found
  by auditing the Python twin, whose own 2.4.0 fix had missed three call sites
  for a structurally similar reason.

## [0.1.4] — 2026-10-18

### Added
- **`yt-dlp` is provisioned, not merely required.** A crate whose every function shells
  out to a binary should not leave finding it to the reader. `provision::ytdlp` looks on
  `PATH` first and otherwise fetches the official standalone build, so the documented
  prerequisite stops being a prerequisite.
- **A CLI (`youtube-helper-rs`)**, behind a `cli` feature that is on by default. Named
  after the crate rather than after the Python twin: the two are meant to coexist on one
  machine, and identical names would let `PATH` order decide which one runs. Library
  consumers drop `clap` with `default-features = false`.
- **`TRIGGERS.md` and an enforced agent skill** (`skills/`), with `tests/skill.rs` as the
  portal that fails when the catalogue and the skill description drift apart.

### Fixed
- **A stalled `yt-dlp` read could hang the caller forever.** `yt-dlp` applies no socket
  timeout unless asked, so a connection the remote has half-closed is waited on
  indefinitely -- the process just stops making progress, at 0 % CPU, until something
  kills it. The Python twin was observed doing exactly that for nine minutes, twice,
  with its sockets in `CLOSE_WAIT`; nothing about the failure is language-specific, since
  this crate drives the same binary over the same sockets.

  Every invocation in the crate funnels through `ytdlp::run`, so the bound lives there
  rather than at each call site: one place to get right, and no way for a future caller
  to forget it. Override with `YOUTUBE_HELPER_SOCKET_TIMEOUT`, the same variable the
  Python package reads.

### Changed
- **crates.io metadata completed** — `authors`, `homepage` and `documentation` were
  inconsistent or absent.

## [0.1.3] — 2026-10-02

### Added
- **`download_audio_with_options` and `AudioFormat`** — download as MP3, M4A, Opus, FLAC, or
  `Best` (the source's own format, no re-encoding at all) instead of always WAV. WAV is right
  for a decoder or an ML pipeline and wrong for anything that has to travel: it is roughly ten
  times the size of the original, and re-encoding a lossy source to it loses quality while
  gaining none back.

  `AudioFormat` is a closed enum rather than a string because the value is handed straight to
  `yt-dlp --audio-format`, and a caller-supplied string could just as easily be another
  command-line flag. It is `#[non_exhaustive]`, so adding a format later is not a breaking
  change.

  `download_audio` is unchanged — same signature, same WAV default.

### Changed
- **CI is one Linux job instead of a three-OS matrix.** The heavy, cross-platform testing is
  the local pre-push gate's job; CI confirms, it does not discover.
- `#![cfg_attr(not(test), forbid(unsafe_code))]` and `#![deny(missing_docs)]` replace
  `#![warn(missing_docs)]`. The `forbid` is lifted only under `cfg(test)`, where a few tests
  set a process-wide environment variable.
- `rust-version = "1.85"` is now declared. Verified, not guessed: the full suite passes on
  1.85.0.

## [0.1.2] — 2026-10-02

### Added
- **`resolve_media_url`** — resolve a direct, streamable media URL instead of downloading.
  Downloading works when the media ends; a live broadcast does not, so `download_audio` on one
  blocks forever while this returns an address a player can follow for as long as the broadcast
  lasts. The result is signed and expires within hours: resolve right before use, never cache.

### Changed
- The README no longer claims a scope the crate did not have.

## [0.1.1] — 2026-09-05

### Changed
- Repo hygiene only, no library change. Added `scripts/check-fresh-resolve.sh` — it builds the
  crate with no `Cargo.lock`, the way a downstream consumer resolves it — and wired it into CI
  and a versioned `.githooks` pre-push gate.
- Every public item documented, so docs.rs carries the whole API.

### Fixed
- The fake `yt-dlp` test fixture was not executable on Windows (an `sh` shebang with no `.bat`
  wrapper).

## [0.1.0] — 2026-09-05

First release. A thin, honest wrapper around the `yt-dlp` binary: video metadata
(`fetch_metadata`) and audio download (`download_audio`), with a `thiserror` enum that keeps a
missing binary, an unsupported URL, a network failure, a JSON parse failure and plain I/O
distinguishable. No part of `yt-dlp`'s extraction logic is reimplemented.
