<p align="center">
  <img src="packaging/icons/128.png" width="96" alt="">
</p>

<h1 align="center">HappyGreaseFloppy</h1>

<p align="center">
  <a href="README.md">English</a> · <b>Français</b>
</p>

<p align="center">
  Une interface graphique conviviale pour les outils disquette <a href="https://github.com/keirf/greaseweazle">Greaseweazle</a>.<br>
  Lire, écrire, copier, formater et diagnostiquer des disquettes — Amiga, PC, Atari ST, Commodore, Apple II, BBC Micro et bien d'autres.<br>
  Windows · macOS · Linux — <b>v0.17.1</b>
</p>

<p align="center">
  <img src="docs/images/read.png" width="860" alt="Lecture d'une disquette Amiga usée : les deux faces piste par piste, 9 pistes récupérées et 3 illisibles, un conseil et la relecture en un clic">
</p>

HappyGreaseFloppy pilote pour vous les outils Greaseweazle (`gw`). Il couvre toutes les commandes
`gw`, montre la disquette piste par piste et secteur par secteur pendant la lecture ou l'écriture,
explique les erreurs en langage clair, et ne lance jamais une opération destructrice sans
demander confirmation.

*Anciennement FluxMyFluffyFloppy.* L'interface de l'application est en anglais ; les noms de
pages et de boutons sont donnés ici tels qu'ils apparaissent à l'écran.

## Points forts

* **Voir la disquette** : les deux faces dessinées comme elles tournent, chaque piste colorée au
  fil de la lecture ou de l'écriture (bonne, récupérée après plusieurs essais, illisible), le
  détail des secteurs au survol, la progression avec le temps restant.
* **Comprendre les échecs** : « No Index », « Track 0 not found », « Failed to verify », disquette
  protégée, débordement USB… chacun est accompagné de ce qu'il faut vérifier et d'un bouton vers
  le bon outil.
* **Récupérer les disquettes usées** : profil *Recovery*, puis *Re-read the failed tracks*, qui
  ne relit que les pistes en échec.
* **Copier des disquettes** avec un seul lecteur (on vous demande d'échanger les disquettes) ou
  deux, en secteurs ou en flux brut (conserve les protections anticopie).
* **Formater** des disquettes vierges pour Amiga, PC, Atari ST, MSX, Commodore 1541/1571/1581,
  BBC Micro et Apple II.
* **Contrôler un lecteur** en une fois : vitesse du moteur, liaison USB, lecture d'une disquette
  de référence, décalage des têtes, test d'écriture facultatif, avec un verdict.
* **Archiver une boîte de disquettes** : numérotation automatique (`disk_001.adf`,
  `disk_002.adf`…), réglages enregistrés par machine, journal de toutes les commandes.
* Thème sombre, clair ou celui du système ; taille du texte réglable ; la commande `gw` exacte
  affichée avant d'être lancée.

## Captures d'écran

| | |
|---|---|
| <img src="docs/images/format.png" alt="Page Format, thème clair : une disquette Commodore 1541 formatée et vérifiée"> | <img src="docs/images/health.png" alt="Drive health : verdict Drive healthy, contrôles et carte de la disquette de référence"> |
| **Format disk** — une disquette Commodore 1541 vierge, écrite et vérifiée (thème clair) | **Drive health** — contrôle guidé avec un verdict |
| <img src="docs/images/write.png" alt="Page Write avec la vérification de compatibilité Amiga"> | <img src="docs/images/rpm.png" alt="Drive RPM : 300,63 tr/min avec historique et statistiques"> |
| **Write disk** — avec la vérification de compatibilité Amiga | **Drive RPM** — vitesse, écart et historique |
| <img src="docs/images/inspector.png" alt="Image inspector affichant un volume AmigaDOS"> | |
| **Image inspector** — taille, SHA-256, explorateur AmigaDOS | |

## Disquettes prises en charge

**La lecture, l'écriture, la copie et la conversion** fonctionnent avec tous les formats et
types d'image gérés par votre `gw` (environ 150 formats : Amiga, PC, Atari, Commodore, Apple,
Acorn, Amstrad, MSX, Mac, PC-98, TRS-80… ; ADF, IMG, ST, D64, DSK, HFE, SCP, IPF, KryoFlux et
d'autres). Des préréglages en un clic couvrent les machines courantes.

**Le formatage** (HappyGreaseFloppy construit la disquette vierge, puis l'écrit et la vérifie, ou
l'enregistre comme image) :

| Ordinateur | Disquettes | Vérifié avec |
|---|---|---|
| Amiga | DD 880K, HD 1.76M — OFS ou FFS, mode International, nom de volume | amitools `xdftool` |
| PC | 360K, 720K, 1.2M, 1.44M, 2.88M — FAT12, nom de volume | mtools, dosfstools, pyfatfs |
| Atari ST | 360K simple face, 720K double face (TOS) | mtools, dosfstools |
| MSX | 1DD, 2DD (MSX-DOS) | mtools, dosfstools |
| Commodore | 1541 (C64), 1571 (C128), 1581 — nom et ID du disque | VICE `c1541` (octet pour octet) |
| BBC Micro | DFS 40 ou 80 pistes, simple ou double face, titre | dfsimage |
| Apple II | DOS 3.3, ProDOS (disquettes de données) | AppleCommander |

## Installation

Téléchargez l'archive de votre système depuis la page **Releases** de ce dépôt, ou compilez le
programme vous-même (voir [Compilation](#compilation)).

| Système | Archive | Démarrage |
|---|---|---|
| Windows 10/11 | `…-windows-x86_64.zip` | Décompressez et lancez `HappyGreaseFloppy.exe`. Décompressez la version officielle des outils Greaseweazle dans le même dossier : `gw` est alors trouvé automatiquement. |
| macOS 11+ (Apple Silicon et Intel) | `…-macos-universal.zip` | Décompressez et placez `HappyGreaseFloppy.app` dans Applications. L'application n'est pas notariée : la première fois, clic droit → **Ouvrir**. Installez `gw` avec `pip3 install greaseweazle`. |
| Linux x86_64 | `…-linux-x86_64.tar.gz` | Extrayez et lancez `./install.sh` (pour l'utilisateur courant, ajoute une entrée de menu). Installez `gw` avec `pip install --user greaseweazle`, et ajoutez-vous au groupe `dialout` pour l'accès USB. |

**`gw` est trouvé automatiquement** : chemin configuré, `PATH`, dossier du programme (et ses
sous-dossiers `greaseweazle-*`), installations Python de l'utilisateur (`~/.local/bin`, pipx,
`~/Library/Python/*/bin`, Homebrew) et, sous Windows, les dossiers `Scripts` de Python et les
dossiers `greaseweazle-*` de Téléchargements, Bureau et Documents. *Settings → Auto-detect*
relance la recherche ; la variable d'environnement `HGF_GW` impose un chemin.

## Premiers pas

1. **Branchez** le Greaseweazle en USB, et le lecteur avec sa nappe et son alimentation.
2. **Choisissez le lecteur** dans la barre du haut :
   * lecteur PC sur nappe croisée : **Drive A** (connecteur après le croisement) ou **B** ;
   * lecteur PC sur nappe droite : **Drive B** ;
   * lecteur Amiga, Atari ou autre lecteur ancien (nappe droite) : **Unit 0** ou **Unit 1**.
3. **Vérifiez l'installation** avec *Drive health* et une disquette que vous savez bonne.
4. **Lire une disquette** : choisissez le préréglage de la machine, le fichier de destination,
   puis *Read disk to image*. Les pistes rouges peuvent être relues avec *Re-read the failed
   tracks*.
5. **Écrire une disquette** : choisissez l'image, vérifiez le format (détecté d'après l'image),
   puis *Write image to disk*. Chaque piste est vérifiée.

Sans matériel, `HGF_GW=tools/fake-gw.py` fait tourner l'application avec un simulateur.

## Les pages

| Page | Commande `gw` | Rôle |
|---|---|---|
| Read disk | `read` | préréglages, profils *Standard* / *Recovery* / *Quick scan*, toutes les options avancées (révolutions, essais, PLL, index simulé, disquette réversible, densité…), numérotation des disquettes, options de fichier image (HFE, SCP, KryoFlux) |
| Write disk | `write` | détection du format, contrôle de la taille, effacement préalable, essais de vérification, précompensation, vérification de compatibilité Amiga |
| Copy disk | `read` + `write` | un ou deux lecteurs, copie en secteurs ou en flux, pistes source illisibles signalées avant l'écriture |
| Format disk | `write` | disquettes vierges pour les ordinateurs ci-dessus ; double pas pour les disquettes 40 pistes dans un lecteur 80 pistes |
| Convert image | `convert` | entre types d'image, plages de pistes, PLL, ajustement de vitesse |
| Erase disk | `erase` | plage de pistes, révolutions, effacement haute fréquence |
| Drive health | `info` `rpm` `bandwidth` `read` `write` | contrôle guidé avec verdict et rapport copiable |
| Drive RPM · Head alignment | `rpm` · `align` | mesures et graphiques en direct |
| Seek head · Clean drive | `seek` · `clean` | confirmation pour les cylindres extrêmes, progression du nettoyage |
| Device info · Delays · Pins · USB bandwidth | `info` `delays` `pin` `bandwidth` | informations et réglages du Greaseweazle |
| Reset · Firmware update | `reset` `update` | fichier local, version GitHub, bootloader |
| Image inspector | — | taille, SHA-256, format deviné, explorateur AmigaDOS |

Partout : réglages enregistrés (modèles) pour la lecture et l'écriture, la ligne de commande
`gw` avec un bouton de copie, un bouton *Stop* (ou `Échap`), une console avec un filtre
*problems only*, le glisser-déposer des fichiers image, un journal facultatif, et une
notification dans la barre des tâches ou le Dock quand une longue opération se termine.

## Dépannage

| Message | Cause habituelle |
|---|---|
| `No Index` | Pas de disquette, ou le lecteur ne tourne pas : disquette mal enfoncée, lecteur non alimenté, mauvais lecteur choisi (A/B ou 0/1), nappe à l'envers (voyant du lecteur toujours allumé). |
| `Track 0 not found` | Mauvais choix de lecteur ou nappe mal orientée (broche 1 aux deux bouts). |
| `Failed to verify Track …` | Disquette usée ou inadaptée (format DD sur disquette HD : masquez le trou de densité), têtes encrassées. Essayez une autre disquette, nettoyez les têtes, lancez *Drive health*. |
| Vérifiée, mais illisible sur l'Amiga | Voir la carte *Amiga compatibility* de la page Write : option restée active (inversion des têtes, double pas…), image FFS sur Kickstart 1.3, disquette déjà utilisée. |
| `Permission denied` (Linux) | Ajoutez-vous au groupe `dialout`, puis déconnectez-vous et reconnectez-vous. |
| macOS refuse d'ouvrir l'application | Clic droit → **Ouvrir** la première fois (l'application n'est pas notariée). |

Le [wiki Greaseweazle](https://github.com/keirf/greaseweazle/wiki) (en anglais) couvre la partie
matérielle :
[premiers pas](https://github.com/keirf/greaseweazle/wiki/Getting-Started),
[dépannage](https://github.com/keirf/greaseweazle/wiki/Troubleshooting-FAQ),
[sélection du lecteur](https://github.com/keirf/greaseweazle/wiki/Drive-Select).

## Compilation

Il faut Rust 1.95 ou plus récent et un compilateur C :

```bash
cargo build --release -p hgf-gui
./target/release/happygreasefloppy
```

La création des paquets, les tests avec les outils de référence, l'icône et la publication sont
décrits dans [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) (en anglais). L'historique des versions
est dans [CHANGELOG.md](CHANGELOG.md).

## Remerciements et licences

* Matériel et outils [Greaseweazle](https://github.com/keirf/greaseweazle) de Keir Fraser —
  HappyGreaseFloppy ne fait que lancer `gw`, il ne l'inclut pas.
* Inspiré de [FluxMyFluffyFloppy](https://github.com/FrankieTheFluff/FluxMyFluffyFloppy),
  l'interface Windows de FrankieTheFluff (code indépendant).
* Réalisé avec [egui/eframe](https://github.com/emilk/egui) et les
  [icônes Phosphor](https://phosphoricons.com) ; police
  [JetBrains Mono](https://www.jetbrains.com/lp/mono/) (SIL Open Font License 1.1, voir
  `crates/hgf-gui/assets/fonts/JetBrainsMono-OFL.txt`).
* Licences de toutes les bibliothèques utilisées :
  [THIRD-PARTY-LICENSES.txt](THIRD-PARTY-LICENSES.txt).

HappyGreaseFloppy n'a pas encore de licence : c'est un projet personnel, tous droits réservés à
son auteur.
