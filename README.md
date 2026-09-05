# YouTube Helper (Rust)

[🇫🇷](https://github.com/warith-harchaoui/youtube-helper-rs/blob/master/LISEZMOI.md) · [🇬🇧](https://github.com/warith-harchaoui/youtube-helper-rs/blob/master/README.md)

[![crates.io](https://img.shields.io/crates/v/youtube-helper-rs.svg)](https://crates.io/crates/youtube-helper-rs) [![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD%203--Clause-blue.svg)](./LICENSE)

Rust rewrite of the core promise of [`youtube-helper`](https://github.com/warith-harchaoui/youtube-helper), not a line-by-line port of its code. `youtube-helper-rs` shells out to the [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) binary via `std::process::Command` and turns its output into typed Rust values and a `thiserror` error enum. It does not reimplement any of `yt-dlp`'s extraction logic — `yt-dlp` already knows how to talk to hundreds of video sites; this crate just wraps one consistent Rust interface around invoking it.

## v0.1 scope (honest, not aspirational)

Two functions, on purpose:

- `fetch_metadata(url: &str) -> Result<VideoMetadata, YoutubeHelperError>` — runs `yt-dlp --dump-json <url>` and parses the result into a `VideoMetadata` struct (`id`, `title`, `duration`, `uploader`, `channel`, `webpage_url`, `description`, `upload_date`, `view_count`, `like_count`, `thumbnail`). Field presence was checked by hand against a real `yt-dlp --dump-json` call, not guessed from documentation.
- `download_audio(url: &str, out_dir: &Path) -> Result<PathBuf, YoutubeHelperError>` — runs `yt-dlp -x --audio-format wav <url>` with `--print after_move:filepath`, so the returned path is exactly what `yt-dlp` itself reports as the final file, not a guess reconstructed from the output template.

That's it for v0.1. No video download, no thumbnail download, no stream catalog / direct-URL resolution, no channel/engagement metadata, no subtitles, no comments, no ffmpeg post-processing, no Tor fallback — all present in the Python original, all deliberately out of scope here until there's a real consumer that needs them.

## Error handling

`YoutubeHelperError` (via `thiserror`) gives each failure mode its own variant instead of one opaque error:

- `BinaryNotFound` — `yt-dlp` is not on `PATH` (or wherever `YOUTUBE_HELPER_YTDLP_BIN` points).
- `InvalidUrl` — the URL is empty/malformed, or `yt-dlp` itself rejects it as unsupported.
- `CommandFailed` — `yt-dlp` ran but exited non-zero for any other reason (network failure, geo-block, age restriction, removed video, rate limiting, ...). Carries the raw stderr.
- `JsonParse` — `yt-dlp --dump-json` returned something that didn't parse as the expected shape.
- `OutputFileNotFound` — `yt-dlp` reported success but the file it printed doesn't exist afterwards.
- `Io` — anything else (e.g. failing to create the output directory).

## Requirements

`yt-dlp` must be installed and reachable on `PATH` (or via the `YOUTUBE_HELPER_YTDLP_BIN` environment variable, which is how the test suite points at a nonexistent binary to exercise `BinaryNotFound` without touching a real install).

```bash
brew install yt-dlp       # macOS
pip install -U yt-dlp     # anywhere with Python
```

## Install

```toml
[dependencies]
youtube-helper-rs = "0.1"
```

## Usage

```rust
use std::path::Path;
use youtube_helper_rs::{download_audio, fetch_metadata};

fn main() -> Result<(), youtube_helper_rs::YoutubeHelperError> {
    let meta = fetch_metadata("https://www.youtube.com/watch?v=jNQXAC9IVRw")?;
    println!("{} ({:?}s) by {:?}", meta.title, meta.duration, meta.uploader);

    let audio_path = download_audio(
        "https://www.youtube.com/watch?v=jNQXAC9IVRw",
        Path::new("./out"),
    )?;
    println!("audio saved to {}", audio_path.display());

    Ok(())
}
```

## Testing

```bash
cargo test              # unit tests + URL-validation + missing-binary tests, no network
cargo test -- --ignored # real network tests against a stable public YouTube video
```

The `--ignored` tests are skipped by default because they touch the network. Known limitation as of this writing: `fetch_metadata` (metadata only, no media stream) works reliably; the `download_audio` ignored test can fail with an `HTTP 403 Forbidden` from `yt-dlp` in sandboxed/CI-like environments without browser cookies or a PO-token provider configured — this is YouTube-side anti-bot enforcement on the media CDN, a known and widely reported `yt-dlp` limitation, not a bug in this wrapper (the resulting `CommandFailed` error is exactly what this crate is supposed to surface in that case).

Most error branches (`CommandFailed`, `OutputFileNotFound`, the `InvalidUrl` message-detection path, non-`NotFound` spawn errors, malformed/empty `yt-dlp` stdout) are exercised deterministically, without the network, by pointing `YOUTUBE_HELPER_YTDLP_BIN` at small fake shell scripts written on the fly (`src/test_support.rs`) — that's most of `cargo test`'s test count, not the two `#[ignore]`d ones.

## Project status

What matters here is the real coverage percentage, not a commit count. Measured with [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov):

| Suite                                             | Lines       | Functions   | Regions     |
|----------------------------------------------------|--------------|-------------|-------------|
| `cargo test` (no network, CI-safe)                  | 89.66% (286/319) | 64.44% (29/45) | 84.01% (452/538) |
| `cargo test -- --include-ignored` (with the 2 network tests) | 97.49% (311/319) | 88.89% (40/45) | 94.24% (507/538) |

The lines still uncovered with network tests included are concentrated in `download.rs` (a few `OutputFileNotFound` error branches that would need a real `yt-dlp` producing a missing path in a way the fake script doesn't reproduce exactly) — see the detail with `cargo llvm-cov report --show-missing-lines`.

To reproduce:

```bash
cargo install cargo-llvm-cov --locked

# Sur macOS sans rustup (toolchain Homebrew), pointer vers les outils LLVM d'Xcode :
export LLVM_COV=$(xcrun -f llvm-cov)
export LLVM_PROFDATA=$(xcrun -f llvm-profdata)
# Avec rustup: `rustup component add llvm-tools-preview` suffit, pas besoin des exports ci-dessus.

cargo llvm-cov --summary-only                                                   # sans réseau
cargo llvm-cov --summary-only --ignore-run-fail -- --include-ignored --test-threads=1  # avec les tests réseau
```

## Checks before pushing

```bash
scripts/install-hooks.sh   # once per clone: installs the pre-push gate below
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all
scripts/check-fresh-resolve.sh   # builds with no Cargo.lock, like a downstream consumer
```

The last one exists because the other three, and CI, all build against the committed
`Cargo.lock` — which no consumer of the published crate ever sees. A dependency range that
has gone bad stays green here while breaking every fresh `cargo add` / `cargo install`, and
`cargo publish --dry-run` doesn't catch it either (it verifies with the same lock). CI runs
this check once, on Linux.

## Related

Part of the same author's local-first tooling as [`youtube-helper`](https://github.com/warith-harchaoui/youtube-helper) (Python) and the [AI Helpers](https://github.com/warith-harchaoui/ai-helpers) suite. Independent rewrite, not a binding.

## Author

[Warith HARCHAOUI](https://linkedin.com/in/warith-harchaoui)

## License

BSD-3-Clause, see `LICENSE`.
