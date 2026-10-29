# Paysage

🇫🇷 Français · [🇬🇧 LANDSCAPE.md](https://github.com/warith-harchaoui/youtube-helper-rs/blob/master/LANDSCAPE.md)

Crates voisins puis concurrents dans l'espace « piloter yt-dlp depuis Rust », comparés à `youtube-helper-rs`. Les notes
vont de ⭐ (1) à ⭐⭐⭐⭐⭐ (5), évaluées sur la tâche visée par `youtube-helper-rs` :
les métadonnées d'une vidéo, le téléchargement audio puis la résolution d'une URL média directe, en déléguant au vrai binaire yt-dlp plutôt qu'en réimplémentant sa logique d'extraction. Un crate optimisé pour un tout autre usage n'est pas pénalisé : la note
reflète seulement l'adéquation à *ce* créneau.

## En un coup d'œil

<!-- TABLE:START -->
| Enveloppes yt-dlp en Rust | Métadonnées | Téléchargement audio | Téléchargement vidéo | Prise en charge des playlists | Résolution de flux en direct | Approvisionnement du binaire | Synchrone (sans runtime imposé) | Erreurs typées |
| --- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **youtube-helper-rs** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ | ⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| yt-dlp | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ | ⭐⭐⭐ |
| ytd-rs | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐ | ⭐ | ⭐⭐⭐ |
| rustypipe | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ | ⭐⭐⭐⭐ |
| ytextract | ⭐⭐⭐⭐⭐ | ⭐ | ⭐ | ⭐⭐⭐ | ⭐ | ⭐⭐⭐⭐⭐ | ⭐ | ⭐⭐⭐ |
| ytdown | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐⭐ | ⭐⭐ | ⭐⭐ |
<!-- TABLE:END -->

## Carte de positionnement

<!-- FIGURE:START -->
Représentation 2D du tableau ci-dessus.

![Carte de positionnement](https://raw.githubusercontent.com/warith-harchaoui/youtube-helper-rs/master/assets/paysage.png)

La carte est un résumé en 2D des 8 critères : à lire comme une forme, pas comme un classement. « youtube-helper-rs » se situe dans le coin en haut à droite. Les axes se lisent **Horizontal : Efficacité ↔ Rigueur** et **Vertical : Complétude ↔ Agilité**.

L'axe horizontal est mené par l'approvisionnement du binaire avec les métadonnées, le téléchargement vidéo pesant à l'inverse ; l'axe vertical est mené par le téléchargement audio avec la résolution de flux en direct, la prise en charge des playlists pesant à l'inverse. En clair : la moitié haute sait vous remettre de l'audio, puis la moitié droite exige le moins de votre machine tout en étant la plus claire sur l'échec.

`youtube-helper-rs` avec le crate `yt-dlp` sont les deux seuls présents dans les deux moitiés. Ils se séparent sur le périmètre — le crate `yt-dlp` fait la vidéo : c'est la recommandation honnête quand la vidéo est le besoin, tandis que ce crate s'en tient à l'audio, reste synchrone, puis laisse le choix du runtime à l'appelant. `ytd-rs` occupe le haut à gauche : il télécharge, mais suppose que vous avez le binaire puis impose tokio.

`rustypipe` avec `ytextract` occupent le bas à droite parce qu'ils n'ont besoin d'aucun binaire — ils parlent directement l'API Innertube de YouTube — puis parce qu'aucun des deux ne rend un fichier audio. Cela fait de `rustypipe` le meilleur choix pour les métadonnées avec les playlists, puis le moins bon le jour où YouTube change cette API : précisément le risque que yt-dlp absorbe à la place de ce crate.
<!-- FIGURE:END -->

## D'où viennent les affirmations

Versions puis dates de publication lues sur l'API crates.io le 29 octobre 2026 :

| Crate | Version | Dernière publication |
|---|---|---|
| `yt-dlp` | 2.9.1 | 2026-10-26 |
| `ytd-rs` | 0.2.1 | 2026-03-22 |
| `rustypipe` | 0.11.4 | 2025-04-23 |
| `ytextract` | 0.11.2 | 2023-01-29 |
| `ytdown` | 0.8.1 | 2026-09-24 |

Les notes de `youtube-helper-rs` lui-même proviennent de ce dépôt en version 0.1.6.

- **Les deux notes de 1 — téléchargement vidéo puis playlists — sont la ligne de périmètre de la v0.1, pas un oubli.** Le README le dit dans les mêmes termes. Le crate `yt-dlp` obtient 5 sur la vidéo : c'est la recommandation honnête quand la vidéo est le besoin.

- **« Synchrone (sans runtime imposé) » est une colonne d'adéquation au créneau, non un verdict sur l'asynchrone.** `yt-dlp`, `ytd-rs` puis `rustypipe` obtiennent 1 parce qu'ils imposent un runtime tokio à un consommateur de bibliothèque qui n'en veut peut-être pas ; ce crate bloque sur `std::process::Command` puis laisse le choix à l'appelant.

- **`rustypipe` obtient 5 sur l'approvisionnement en n'ayant besoin d'aucun binaire** : il parle directement l'API Innertube de YouTube. Cela en fait le meilleur choix pour les métadonnées avec les playlists, puis le moins bon le jour où YouTube change cette API — le risque que yt-dlp absorbe à la place de ce crate.

- **`ytextract` a été publié pour la dernière fois le 29 janvier 2023**, la plus ancienne version de ce tableau de deux ans. Noté parce qu'un extracteur périmé est le mode de défaillance que toute cette catégorie partage.

Laissé délibérément comme opinion plutôt que sourcé : les colonnes de style
*ergonomie*, puis l'ordre des lignes sous la référence.

## Idées à reprendre

Étudié plutôt que deviné : ce que chaque voisin fait mieux, puis ce que ce crate devrait changer pour l'égaler. Rien ici n'est un engagement : c'est la liste de lecture dont part la version suivante.

- **le crate `yt-dlp` — approvisionner aussi ffmpeg, pas seulement yt-dlp** — Le `provision.rs` d'ici trouve ou récupère `yt-dlp` puis s'arrête là. Le crate `yt-dlp` télécharge ses dépendances, ffmpeg compris. Comme `podcast-helper-rs` délègue à ce crate puis réclame ffmpeg de toute façon, la même recherche en quatre étapes appliquée à ffmpeg comblerait un manque qui remonte aujourd'hui d'un étage.

- **`rustypipe` — une voie rapide pour les métadonnées, sans aucun binaire** — Il parle directement l'API Innertube : les métadonnées avec les playlists ne coûtent ni sous-processus ni téléchargement. `fetch_metadata` est l'appel le moins cher qu'offre ce crate puis le plus susceptible d'être fait en masse ; une voie Innertube derrière une fonctionnalité le rendrait moins cher encore, yt-dlp servant de secours quand l'API bouge.

- **`ytd-rs` — un constructeur d'options plutôt qu'une structure qui s'élargit** — `DownloadOptions` gagne un champ par capacité. Un constructeur se lit mieux au point d'appel puis permet d'ajouter des options sans que la structure littérale change de forme chez chaque appelant.
