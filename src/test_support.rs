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
///
/// On Unix the executable is never opened for writing by this process. Tests
/// run on parallel threads, and a sibling test that forks to spawn its own
/// fake inherits every descriptor open at that instant until its `exec`: if
/// that includes the write side of this script, executing the script fails
/// with `ETXTBSY` ("Text file busy", rust-lang/rust#114554), which turned CI
/// red at random. So the body goes to a source file and `cp`, in a process of
/// its own, creates the executable.
pub(crate) fn write_fake_ytdlp(dir: &Path, script_body: &str) -> PathBuf {
    let script_path = dir.join("yt-dlp-fake.sh");
    #[cfg(unix)]
    let written = dir.join("yt-dlp-fake.src");
    #[cfg(windows)]
    let written = script_path.clone();
    let mut file = std::fs::File::create(&written).expect("create fake yt-dlp script");
    writeln!(file, "#!/bin/sh").expect("write shebang");
    write!(file, "{script_body}").expect("write script body");
    drop(file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let copied = std::process::Command::new("cp")
            .arg(&written)
            .arg(&script_path)
            .status()
            .expect("run cp for the fake yt-dlp script");
        assert!(copied.success(), "cp of the fake yt-dlp script failed");
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
