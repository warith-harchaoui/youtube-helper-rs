//! Making sure the `yt-dlp` binary this crate needs is actually there.
//!
//! WHY THIS MODULE EXISTS. Every public function of this crate ends in a
//! subprocess call to `yt-dlp`. Without that binary the crate is a shell: it
//! compiles, its tests pass, and the first real call fails with
//! [`crate::error::YoutubeHelperError::BinaryNotFound`]. That failure lands far
//! from its cause — in a service, it surfaces as a transcription error weeks
//! after deployment, and reads as "the product is broken" rather than "the
//! image is incomplete". Declaring the dependency in prose (a README line
//! saying "install yt-dlp first") puts the burden on every consumer, and the
//! Python sibling of this crate does not: it lists `yt-dlp` among its
//! dependencies, so installing it installs the binary. This module gives the
//! Rust crate the same property.
//!
//! WHAT IT DOES, in order, the first time a binary is needed:
//!
//! 1. `YOUTUBE_HELPER_YTDLP_BIN` — an explicit choice always wins, and is
//!    never second-guessed.
//! 2. `yt-dlp` on `PATH` — the normal case on a developer machine and in any
//!    image that installs it.
//! 3. A copy this crate downloaded earlier, under the user cache directory.
//! 4. Otherwise: download the official standalone build for this platform from
//!    the `yt-dlp` GitHub release, make it executable, and use it.
//!
//! WHAT IT DOES NOT DO. It never installs system-wide, never writes outside
//! the user cache, and never upgrades a `yt-dlp` that is already reachable —
//! the version on `PATH` is the operator's choice, not this crate's business.
//! `YOUTUBE_HELPER_NO_AUTO_INSTALL=1` turns step 4 off for environments where
//! an unannounced network call at runtime is unacceptable; the old
//! `BinaryNotFound` error comes back, unchanged.
//!
//! THE DOWNLOAD IS A REAL COST, and the honest way to avoid it is to install
//! `yt-dlp` properly: `pip install yt-dlp`, a distribution package, or the
//! standalone build placed on `PATH`. Step 4 is a floor, not a plan.

use crate::error::{Result, YoutubeHelperError};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Environment variable that names the binary (or its full path) to invoke.
pub const BIN_ENV: &str = "YOUTUBE_HELPER_YTDLP_BIN";

/// Environment variable that disables the automatic download.
pub const NO_AUTO_INSTALL_ENV: &str = "YOUTUBE_HELPER_NO_AUTO_INSTALL";

/// Environment variable that overrides where a downloaded copy is kept.
pub const CACHE_DIR_ENV: &str = "YOUTUBE_HELPER_CACHE_DIR";

/// The release the standalone builds are taken from. `latest` rather than a
/// pinned version on purpose: YouTube changes its defences every few weeks and
/// an old `yt-dlp` fails on half the links, which is a worse failure than a
/// version that moves — it looks like the URL is wrong.
const RELEASE_BASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";

/// Resolved once per process: the path is stable for the life of the program,
/// and probing `PATH` on every call would pay for the same answer repeatedly.
static RESOLU: OnceLock<PathBuf> = OnceLock::new();

/// The asset name for the platform this was compiled for, or `None` when the
/// `yt-dlp` project publishes no standalone build for it.
fn asset_de_la_plateforme() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("yt-dlp_linux"),
        ("linux", "aarch64") => Some("yt-dlp_linux_aarch64"),
        ("linux", "arm") => Some("yt-dlp_linux_armv7l"),
        // The macOS build is universal: one asset for both architectures.
        ("macos", _) => Some("yt-dlp_macos"),
        ("windows", _) => Some("yt-dlp.exe"),
        _ => None,
    }
}

/// Where a downloaded copy lives. Under the user cache directory, never under
/// a system prefix: a library has no business writing where a package manager
/// writes.
fn dossier_de_cache() -> PathBuf {
    if let Ok(declare) = std::env::var(CACHE_DIR_ENV) {
        if !declare.trim().is_empty() {
            return PathBuf::from(declare);
        }
    }
    let base = if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join("Library").join("Caches"))
            .unwrap_or_else(|_| std::env::temp_dir())
    } else if cfg!(target_os = "windows") {
        std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir())
    } else {
        std::env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".cache")))
            .unwrap_or_else(|_| std::env::temp_dir())
    };
    base.join("youtube-helper-rs").join("bin")
}

/// Does this path point at something runnable?
fn executable(chemin: &Path) -> bool {
    if !chemin.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(chemin)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// The first `yt-dlp` found on `PATH`, if any.
fn sur_le_chemin() -> Option<PathBuf> {
    let nom = if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    };
    let chemin = std::env::var_os("PATH")?;
    std::env::split_paths(&chemin)
        .map(|d| d.join(nom))
        .find(|c| executable(c))
}

/// Downloads the standalone build into the cache directory and returns its
/// path.
///
/// `curl` then `wget`, because this crate already spawns processes and adding
/// an HTTP client — with its TLS stack — to pull one file would weigh more on
/// every consumer than it saves. A machine with neither is a machine that
/// cannot reach GitHub anyway.
fn telecharger() -> Result<PathBuf> {
    let asset = asset_de_la_plateforme().ok_or_else(|| YoutubeHelperError::CommandFailed {
        status: "unsupported platform".to_string(),
        stderr: format!(
            "the yt-dlp project publishes no standalone build for {}/{}; install yt-dlp yourself \
             (pip install yt-dlp) or point {BIN_ENV} at a binary",
            std::env::consts::OS,
            std::env::consts::ARCH
        ),
    })?;

    let dossier = dossier_de_cache();
    std::fs::create_dir_all(&dossier).map_err(YoutubeHelperError::Io)?;
    let cible = dossier.join(if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    });
    let partiel = dossier.join("yt-dlp.partial");
    let url = format!("{RELEASE_BASE}/{asset}");

    let mut derniere_erreur = String::new();
    let tentatives: [(&str, Vec<String>); 2] = [
        (
            "curl",
            vec![
                "-fsSL".into(),
                "--retry".into(),
                "3".into(),
                "-o".into(),
                partiel.display().to_string(),
                url.clone(),
            ],
        ),
        (
            "wget",
            vec![
                "-q".into(),
                "-O".into(),
                partiel.display().to_string(),
                url.clone(),
            ],
        ),
    ];

    for (outil, args) in tentatives {
        match std::process::Command::new(outil).args(&args).output() {
            Ok(sortie) if sortie.status.success() => {
                // The partial file becomes the real one only once it is whole:
                // a half-written binary that kept the final name would be
                // "found" by the next run and fail as a corrupt executable,
                // which is a much harder failure to read than a missing file.
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&partiel, std::fs::Permissions::from_mode(0o755))
                        .map_err(YoutubeHelperError::Io)?;
                }
                std::fs::rename(&partiel, &cible).map_err(YoutubeHelperError::Io)?;
                return Ok(cible);
            }
            Ok(sortie) => {
                derniere_erreur = format!(
                    "{outil} exited with {}: {}",
                    sortie.status,
                    String::from_utf8_lossy(&sortie.stderr).trim()
                );
            }
            Err(e) => derniere_erreur = format!("{outil} could not be started: {e}"),
        }
    }

    let _ = std::fs::remove_file(&partiel);
    Err(YoutubeHelperError::CommandFailed {
        status: "download failed".to_string(),
        stderr: format!(
            "could not fetch {url}: {derniere_erreur}. Install yt-dlp yourself (pip install \
             yt-dlp) or point {BIN_ENV} at a binary."
        ),
    })
}

/// What resolving the binary concluded, before anything is done about it.
///
/// SEPARATED FROM THE DOING ON PURPOSE. The decision depends on four facts —
/// a declared path, what is on `PATH`, what is in the cache, whether fetching
/// is allowed — and testing it through the environment means mutating
/// process-wide state that every other test in this crate also reads. That is
/// how a test suite becomes flaky without anyone changing the code it covers.
/// Here the decision is a pure function of its four inputs, and the tests pass
/// them in.
#[derive(Debug, PartialEq, Eq)]
pub enum Choix {
    /// An explicit `YOUTUBE_HELPER_YTDLP_BIN`, taken as given.
    Declare(PathBuf),
    /// Found on `PATH`.
    SurLeChemin(PathBuf),
    /// A copy fetched earlier, still executable.
    EnCache(PathBuf),
    /// Nothing available: fetch the standalone build.
    ATelecharger,
    /// Nothing available and fetching was refused.
    Refuse,
}

/// The decision itself, from facts rather than from the environment.
#[must_use]
pub fn choisir(
    declare: Option<&str>,
    sur_le_chemin: Option<PathBuf>,
    en_cache: Option<PathBuf>,
    telechargement_autorise: bool,
) -> Choix {
    if let Some(d) = declare.map(str::trim).filter(|d| !d.is_empty()) {
        return Choix::Declare(PathBuf::from(d));
    }
    if let Some(c) = sur_le_chemin {
        return Choix::SurLeChemin(c);
    }
    if let Some(c) = en_cache {
        return Choix::EnCache(c);
    }
    if telechargement_autorise {
        Choix::ATelecharger
    } else {
        Choix::Refuse
    }
}

/// The `yt-dlp` to invoke, provisioning one if the platform has none.
///
/// The expensive part of the answer — probing `PATH`, and at worst a download —
/// is computed once per process. The declared path is re-read every time: it is
/// how a caller changes the binary mid-process, and a cached answer would
/// ignore the change in silence.
pub fn ytdlp() -> Result<PathBuf> {
    let declare = std::env::var(BIN_ENV).unwrap_or_default();
    if let Choix::Declare(chemin) = choisir(Some(&declare), None, None, false) {
        return Ok(chemin);
    }

    if let Some(deja) = RESOLU.get() {
        return Ok(deja.clone());
    }

    let cache = dossier_de_cache().join(if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    });
    let refuse = std::env::var(NO_AUTO_INSTALL_ENV)
        .map(|v| matches!(v.trim(), "1" | "true" | "yes" | "oui"))
        .unwrap_or(false);

    let choix = choisir(
        None,
        sur_le_chemin(),
        executable(&cache).then_some(cache),
        !refuse,
    );

    let chemin = match choix {
        Choix::Declare(c) | Choix::SurLeChemin(c) | Choix::EnCache(c) => c,
        Choix::ATelecharger => telecharger()?,
        Choix::Refuse => {
            return Err(YoutubeHelperError::BinaryNotFound {
                binary: "yt-dlp".to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!(
                        "yt-dlp is not on PATH and {NO_AUTO_INSTALL_ENV} forbids fetching it; \
                         install it (pip install yt-dlp) or point {BIN_ENV} at a binary"
                    ),
                ),
            })
        }
    };

    let _ = RESOLU.set(chemin.clone());
    Ok(chemin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_plateforme_courante_a_un_binaire_publie() {
        // Si cela cassait sur une plateforme que l'on prétend servir, la
        // panne arriverait à l'exécution, sous la forme d'un « unsupported
        // platform » que personne n'attend, et non à la compilation.
        assert!(
            asset_de_la_plateforme().is_some(),
            "no standalone yt-dlp build is known for {}/{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    }

    #[test]
    fn un_chemin_declare_passe_avant_tout_le_reste() {
        // Y COMPRIS AVANT UN BINAIRE PARFAITEMENT VALABLE SUR LE CHEMIN :
        // c'est ainsi qu'un appelant impose le sien, et c'est ce que font les
        // tests des autres modules de cette caisse.
        let choix = choisir(
            Some("/opt/a-moi/yt-dlp"),
            Some(PathBuf::from("/usr/bin/yt-dlp")),
            Some(PathBuf::from("/cache/yt-dlp")),
            true,
        );
        assert_eq!(choix, Choix::Declare(PathBuf::from("/opt/a-moi/yt-dlp")));
    }

    #[test]
    fn un_chemin_declare_vide_ne_compte_pas() {
        // Une variable posée mais vide est une variable que personne n'a
        // voulue : la lire comme un chemin donnerait « binaire introuvable »
        // sur un nom vide, erreur que rien n'explique.
        let choix = choisir(
            Some("   "),
            Some(PathBuf::from("/usr/bin/yt-dlp")),
            None,
            true,
        );
        assert_eq!(choix, Choix::SurLeChemin(PathBuf::from("/usr/bin/yt-dlp")));
    }

    #[test]
    fn le_chemin_passe_avant_le_cache() {
        // LE BINAIRE DE L'EXPLOITANT EST PRIORITAIRE sur celui que cette
        // caisse a téléchargé un jour : sa version est un choix, pas un
        // accident, et le nôtre ne doit pas le supplanter en silence.
        let choix = choisir(
            None,
            Some(PathBuf::from("/usr/bin/yt-dlp")),
            Some(PathBuf::from("/cache/yt-dlp")),
            true,
        );
        assert_eq!(choix, Choix::SurLeChemin(PathBuf::from("/usr/bin/yt-dlp")));
    }

    #[test]
    fn sans_rien_on_telecharge_sauf_si_cest_refuse() {
        assert_eq!(choisir(None, None, None, true), Choix::ATelecharger);
        assert_eq!(choisir(None, None, None, false), Choix::Refuse);
    }

    #[test]
    fn le_cache_ne_sort_pas_du_dossier_de_lutilisateur() {
        let d = dossier_de_cache();
        assert!(
            d.ends_with("youtube-helper-rs/bin")
                || d.ends_with("youtube-helper-rs\\bin")
                || std::env::var(CACHE_DIR_ENV).is_ok(),
            "unexpected cache directory: {}",
            d.display()
        );
        assert!(
            !d.starts_with("/usr") && !d.starts_with("/opt"),
            "a library must not write under a system prefix: {}",
            d.display()
        );
    }
}
