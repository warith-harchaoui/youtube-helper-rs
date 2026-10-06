//! Shows — and lets you verify — where this crate gets its `yt-dlp`.
//!
//! Run it on a machine you suspect of being incomplete:
//!
//! ```text
//! cargo run --example provisionner
//! ```
//!
//! It prints the binary that would be used and the version it reports, which
//! answers the two questions a failing install actually raises: *which*
//! `yt-dlp` is being invoked, and *does it run*. To exercise the provisioning
//! path itself, hide the system one and point the cache somewhere fresh:
//!
//! ```text
//! env -i HOME="$HOME" PATH=/usr/bin:/bin \
//!   YOUTUBE_HELPER_CACHE_DIR=/tmp/essai cargo run --example provisionner
//! ```

fn main() {
    match youtube_helper_rs::provision::ytdlp() {
        Ok(chemin) => {
            println!("binary: {}", chemin.display());
            match std::process::Command::new(&chemin)
                .arg("--version")
                .output()
            {
                Ok(sortie) => {
                    println!(
                        "version: {}",
                        String::from_utf8_lossy(&sortie.stdout).trim()
                    )
                }
                Err(e) => println!("could not run it: {e}"),
            }
        }
        Err(e) => println!("no binary: {e}"),
    }
}
