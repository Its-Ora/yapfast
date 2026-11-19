# Yapfast · fork Its-Ora

Fork perso de [Yapfast](https://github.com/yapfast/yapfast), client Discord natif en Rust.
Ce README ne liste que ce qui change par rapport au client de base.

## Télécharger

Onglet **[Releases](https://github.com/Its-Ora/yapfast/releases/latest)** → `Yapfast-…-windows-x64.zip` : extrais-le et lance `yapfast.exe`.
Windows uniquement. Chaque push sur `main` publie une nouvelle release automatiquement.
N'utilise pas « Update » dans Paramètres → About : il installerait la version officielle à la place du fork.

## Changements

**Barre des serveurs**
- Plus de petite barre blanche à gauche des serveurs pour les messages non lus.
- Cacher des serveurs ou des dossiers : clic droit → **Hide server** / **Hide folder**. Un bouton **Show All** (œil) en bas de la liste les réaffiche (estompés) ; re-clic pour les recacher. Clic droit → **Show server again** pour l'enlever des cachés.
- Pings : seuls les @ directs (`@toi`) comptent. Les @everyone, @here et @rôle ne font plus de pastille rouge.

**Notifications et sons**
- En ligne, seuls les MP et les @ directs font une notif et un son. Tout le reste est muet.

**MP**
- Groupes d'amis dans la liste des MP à gauche : clic droit sur un MP → **Move to group** → **New group** (tu tapes le nom direct dans le titre, Entrée pour valider). Clic sur le titre d'un groupe pour le replier ; clic droit pour le renommer, changer sa couleur ou le supprimer. Stockés sur le PC.
- La recherche « Find a DM » trouve aussi les serveurs.

**Salons**
- Bouton rond pour revenir tout en bas d'un salon d'un coup.
- Clic sur « Voice connected » : ouvre le salon vocal.

**Emojis**
- Tous les emojis custom sont utilisables sans Nitro. Ceux que tu n'as pas le droit d'utiliser partent en lien d'image, comme FakeNitro de BetterDiscord.

**Spotify**
- Mini-lecteur Spotify en bas à gauche, au-dessus de l'appel : pochette, titre, temps, précédent / pause-lecture / suivant (contrôle = Spotify Premium). L'activité « Écoute Spotify » elle-même est maintenant dans la version officielle.

**Jeux**
- Les jeux protégés par un anti-cheat (VALORANT avec Vanguard…) sont détectés tout seuls avec leur vraie fiche Discord (nom + image), plus besoin de les ajouter à la main.

**Paramètres**
- Paramètres → Discord : la liste des comptes bloqués est repliée derrière un bouton **See blocked accounts**.

## Licence

GPL-3.0-or-later, comme l'original : voir [LICENSE](LICENSE).
