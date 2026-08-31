//! Test-only helper: writes a small fake `yt-dlp` shell script so the
//! various outcomes of `ytdlp::run` (generic failure, empty stdout on
//! success, a printed path that does or doesn't exist, ...) can be
//! exercised deterministically in `cargo test`, without touching the
//! network or depending on a real `yt-dlp` install being on `PATH`.
#![cfg(test)]

use std::io::Write;
use std::path::{Path, PathBuf};

/// Writes an executable `/bin/sh` script at `dir/yt-dlp-fake` with the
/// given body and returns its path. Point `YOUTUBE_HELPER_YTDLP_BIN` at the
/// returned path to make `ytdlp::run` invoke it instead of a real `yt-dlp`.
pub(crate) fn write_fake_ytdlp(dir: &Path, script_body: &str) -> PathBuf {
    let path = dir.join("yt-dlp-fake");
    let mut file = std::fs::File::create(&path).expect("create fake yt-dlp script");
    writeln!(file, "#!/bin/sh").expect("write shebang");
    write!(file, "{script_body}").expect("write script body");
    drop(file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod fake yt-dlp script");
    }

    path
}
