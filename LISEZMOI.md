# YouTube Helper (Rust)

[🇫🇷](https://github.com/warith-harchaoui/youtube-helper-rs/blob/master/LISEZMOI.md) · [🇬🇧](https://github.com/warith-harchaoui/youtube-helper-rs/blob/master/README.md)

[![crates.io](https://img.shields.io/crates/v/youtube-helper-rs.svg)](https://crates.io/crates/youtube-helper-rs) [![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD%203--Clause-blue.svg)](./LICENSE)

Réécriture en Rust de la promesse centrale de [`youtube-helper`](https://github.com/warith-harchaoui/youtube-helper), pas un portage ligne à ligne de son code. `youtube-helper-rs` invoque le binaire [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) en sous-processus via `std::process::Command` et transforme sa sortie en valeurs Rust typées et en une énumération d'erreurs `thiserror`. Il ne réimplémente aucune logique d'extraction de `yt-dlp` — `yt-dlp` sait déjà parler à des centaines de sites vidéo ; ce crate se contente d'envelopper son invocation dans une interface Rust cohérente.

## Périmètre v0.1 (honnête, pas aspirationnel)

Deux fonctions, volontairement :

- `fetch_metadata(url: &str) -> Result<VideoMetadata, YoutubeHelperError>` — lance `yt-dlp --dump-json <url>` et parse le résultat dans une structure `VideoMetadata` (`id`, `title`, `duration`, `uploader`, `channel`, `webpage_url`, `description`, `upload_date`, `view_count`, `like_count`, `thumbnail`). La présence de chaque champ a été vérifiée à la main sur un vrai appel `yt-dlp --dump-json`, pas devinée depuis la documentation.
- `download_audio(url: &str, out_dir: &Path) -> Result<PathBuf, YoutubeHelperError>` — lance `yt-dlp -x --audio-format wav <url>` avec `--print after_move:filepath`, si bien que le chemin renvoyé est exactement celui que `yt-dlp` lui-même rapporte comme fichier final, pas une reconstruction devinée depuis le gabarit de sortie.

C'est tout pour la v0.1. Pas de téléchargement vidéo, pas de miniature, pas de catalogue de flux/résolution d'URL directe, pas de métadonnées de chaîne/engagement, pas de sous-titres, pas de commentaires, pas de post-traitement ffmpeg, pas de repli Tor — tout cela existe dans l'original Python et reste volontairement hors périmètre ici, tant qu'aucun usage réel ne le réclame.

## Gestion des erreurs

`YoutubeHelperError` (via `thiserror`) donne à chaque mode d'échec sa propre variante plutôt qu'une erreur opaque unique :

- `BinaryNotFound` — `yt-dlp` n'est pas sur le `PATH` (ni à l'endroit pointé par `YOUTUBE_HELPER_YTDLP_BIN`).
- `InvalidUrl` — l'URL est vide/malformée, ou `yt-dlp` lui-même la rejette comme non supportée.
- `CommandFailed` — `yt-dlp` s'est exécuté mais a terminé en erreur pour une autre raison (échec réseau, restriction géographique, restriction d'âge, vidéo supprimée, limitation de débit, ...). Porte la sortie d'erreur brute.
- `JsonParse` — `yt-dlp --dump-json` a renvoyé une sortie qui ne correspond pas à la forme attendue.
- `OutputFileNotFound` — `yt-dlp` a rapporté un succès mais le fichier annoncé n'existe pas ensuite.
- `Io` — tout le reste (par exemple, l'échec de création du répertoire de sortie).

## Prérequis

`yt-dlp` doit être installé et accessible sur le `PATH` (ou via la variable d'environnement `YOUTUBE_HELPER_YTDLP_BIN`, qui sert justement à la suite de tests pour pointer vers un binaire inexistant afin d'exercer `BinaryNotFound` sans toucher à une vraie installation).

```bash
brew install yt-dlp       # macOS
pip install -U yt-dlp     # partout où Python est disponible
```

## Installation

```toml
[dependencies]
youtube-helper-rs = "0.1"
```

## Utilisation

```rust
use std::path::Path;
use youtube_helper_rs::{download_audio, fetch_metadata};

fn main() -> Result<(), youtube_helper_rs::YoutubeHelperError> {
    let meta = fetch_metadata("https://www.youtube.com/watch?v=jNQXAC9IVRw")?;
    println!("{} ({:?}s) par {:?}", meta.title, meta.duration, meta.uploader);

    let audio_path = download_audio(
        "https://www.youtube.com/watch?v=jNQXAC9IVRw",
        Path::new("./out"),
    )?;
    println!("audio enregistré dans {}", audio_path.display());

    Ok(())
}
```

## Tests

```bash
cargo test              # tests unitaires + validation d'URL + binaire manquant, sans réseau
cargo test -- --ignored # tests réseau réels contre une vidéo YouTube publique stable
```

Les tests `--ignored` sont ignorés par défaut car ils touchent le réseau. Limite connue à ce jour : `fetch_metadata` (métadonnées seules, pas de flux média) fonctionne de façon fiable ; le test ignoré `download_audio` peut échouer avec un `HTTP 403 Forbidden` renvoyé par `yt-dlp` dans des environnements en bac à sable/de type CI sans cookies de navigateur ni fournisseur de jeton PO configuré — c'est une mesure anti-robot côté YouTube sur le CDN média, une limite connue et largement documentée de `yt-dlp`, pas un bug de ce wrapper (l'erreur `CommandFailed` qui en résulte est exactement ce que ce crate est censé remonter dans ce cas).

La plupart des branches d'erreur (`CommandFailed`, `OutputFileNotFound`, le chemin de détection du message `InvalidUrl`, les erreurs de lancement autres que `NotFound`, une sortie `yt-dlp` malformée ou vide) sont exercées de façon déterministe, sans réseau, en pointant `YOUTUBE_HELPER_YTDLP_BIN` vers de petits scripts shell factices générés à la volée (`src/test_support.rs`) — c'est l'essentiel du nombre de tests de `cargo test`, pas les deux tests `#[ignore]`.

## État du projet

Ce qui compte ici, c'est le taux de couverture réel, pas le nombre de commits. Mesuré avec [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) :

| Suite | Lignes | Fonctions | Régions |
|---|---|---|---|
| `cargo test` (sans réseau, sûr pour la CI) | 89,66 % (286/319) | 64,44 % (29/45) | 84,01 % (452/538) |
| `cargo test -- --include-ignored` (avec les 2 tests réseau) | 97,49 % (311/319) | 88,89 % (40/45) | 94,24 % (507/538) |

Les lignes non couvertes même avec les tests réseau sont concentrées dans `download.rs` (quelques branches d'erreur `OutputFileNotFound` qui nécessiteraient un `yt-dlp` réel produisant un chemin absent d'une manière que le faux script ne reproduit pas exactement) — le détail est visible avec `cargo llvm-cov report --show-missing-lines`.

Pour relancer la mesure :

```bash
cargo install cargo-llvm-cov --locked

# Sur macOS sans rustup (toolchain Homebrew), pointer vers les outils LLVM d'Xcode :
export LLVM_COV=$(xcrun -f llvm-cov)
export LLVM_PROFDATA=$(xcrun -f llvm-profdata)
# Avec rustup : `rustup component add llvm-tools-preview` suffit, pas besoin des exports ci-dessus.

cargo llvm-cov --summary-only                                                   # sans réseau
cargo llvm-cov --summary-only --ignore-run-fail -- --include-ignored --test-threads=1  # avec les tests réseau
```

## Projets liés

Fait partie du même socle d'outils locaux que [`youtube-helper`](https://github.com/warith-harchaoui/youtube-helper) (Python) et la suite [AI Helpers](https://github.com/warith-harchaoui/ai-helpers). Réécriture indépendante, pas une liaison (*binding*).

## Licence

BSD-3-Clause, voir `LICENSE`.
