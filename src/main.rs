//! `youtube-helper-rs` — the command-line front end for [`youtube_helper_rs`].
//!
//! Deliberately thin, like `md2star-rs`'s binaries: parse arguments, call one
//! library function, print its result, map the outcome to an exit code. Every
//! piece of real logic stays in the crate so a binary, a server or a WASM shell
//! can share it.
//!
//! The binary is named after the crate rather than after the Python twin's
//! `youtube-helper` command on purpose. The two are meant to coexist: giving
//! both the same name would mean whichever directory comes first on `PATH`
//! silently wins, and the user would have no way to tell which one just ran.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use youtube_helper_rs::{
    download_audio_with_options, fetch_metadata, resolve_media_url, AudioFormat, DownloadOptions,
};

/// Typed metadata, direct media URLs and audio downloads, by driving `yt-dlp`.
#[derive(Parser, Debug)]
#[command(name = "youtube-helper-rs", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print the video's metadata as JSON.
    Metadata {
        /// Any URL `yt-dlp` can extract (YouTube, Vimeo, DailyMotion, ...).
        url: String,
        /// Indent the JSON instead of emitting one dense line.
        #[arg(short, long)]
        pretty: bool,
    },
    /// Print a direct, player-readable media URL for the given page.
    Resolve {
        /// Any URL `yt-dlp` can extract.
        url: String,
    },
    /// Download the audio track and print the path of the produced file.
    Download {
        /// Any URL `yt-dlp` can extract.
        url: String,
        /// Directory to write into; created if it does not exist.
        #[arg(short, long, default_value = ".")]
        out_dir: PathBuf,
        /// Audio format to extract to.
        #[arg(short, long, value_enum, default_value_t = Format::Wav)]
        format: Format,
    },
}

/// The subset of [`AudioFormat`] a caller can ask for by name.
///
/// A separate enum rather than deriving `ValueEnum` on the library type:
/// `AudioFormat` is `#[non_exhaustive]` precisely so new formats are not a
/// breaking change, and tying the CLI's accepted values to it would undo that.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum Format {
    /// Uncompressed 16-bit PCM. Lossless, large, what a decoder wants.
    Wav,
    Mp3,
    M4a,
    Opus,
    /// Lossless like WAV, roughly half the size, slower to encode.
    Flac,
    /// Keep whatever the source already provides — the only choice that does no
    /// re-encoding, so the fastest and the only one that cannot degrade audio.
    Best,
}

impl From<Format> for AudioFormat {
    fn from(f: Format) -> Self {
        match f {
            Format::Wav => AudioFormat::Wav,
            Format::Mp3 => AudioFormat::Mp3,
            Format::M4a => AudioFormat::M4a,
            Format::Opus => AudioFormat::Opus,
            Format::Flac => AudioFormat::Flac,
            Format::Best => AudioFormat::Best,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Each arm produces the one line we print on success. Errors are reported
    // the same way everywhere, so the match returns a Result rather than
    // printing and exiting inside each branch.
    let outcome = match cli.command {
        Command::Metadata { url, pretty } => fetch_metadata(&url)
            .map_err(|e| e.to_string())
            .and_then(|meta| {
                if pretty {
                    serde_json::to_string_pretty(&meta)
                } else {
                    serde_json::to_string(&meta)
                }
                .map_err(|e| e.to_string())
            }),
        Command::Resolve { url } => resolve_media_url(&url).map_err(|e| e.to_string()),
        Command::Download {
            url,
            out_dir,
            format,
        } => {
            let options = DownloadOptions {
                audio_format: format.into(),
            };
            download_audio_with_options(&url, &out_dir, &options)
                .map(|p| p.display().to_string())
                .map_err(|e| e.to_string())
        }
    };

    match outcome {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("youtube-helper-rs: {message}");
            ExitCode::FAILURE
        }
    }
}
