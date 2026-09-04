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
///
/// On Windows, `Command::new` cannot execute a POSIX shell script directly
/// (no shebang support — it fails with "%1 is not a valid Win32
/// application"), so the returned path is instead a `.bat` wrapper that
/// shells out to `sh` (Git for Windows, present on `PATH` on GitHub's
/// `windows-latest` runners) with the real script; the script body itself
/// stays a plain `sh` snippet, unchanged across platforms.
pub(crate) fn write_fake_ytdlp(dir: &Path, script_body: &str) -> PathBuf {
    let script_path = dir.join("yt-dlp-fake.sh");
    let mut file = std::fs::File::create(&script_path).expect("create fake yt-dlp script");
    writeln!(file, "#!/bin/sh").expect("write shebang");
    write!(file, "{script_body}").expect("write script body");
    drop(file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod fake yt-dlp script");
        script_path
    }

    #[cfg(windows)]
    {
        let bat_path = dir.join("yt-dlp-fake.bat");
        let mut bat = std::fs::File::create(&bat_path).expect("create fake yt-dlp .bat wrapper");
        writeln!(bat, "@echo off\r\nsh \"{}\" %*\r\n", script_path.display())
            .expect("write .bat wrapper");
        bat_path
    }
}
