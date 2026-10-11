# Bellcart

**Faites les bagages de votre ancien PC. Laissez le superflu derrière vous.**

[English](README.md)

Quand on change d'ordinateur, on ne sait jamais quoi garder, alors on garde
tout. Bellcart analyse votre ancien PC et vous dit ce qui mérite d'être
emporté, ce qui peut partir en archive, et ce qui peut rester sur place.
Comme un groom d'hôtel, il charge le chariot à votre place. Il ne le pousse
pas : le résultat est un plan que vous relisez, puis que vous confiez à
l'outil de copie de votre choix.

> **État : pré-alpha.** Le scan et le moteur de règles fonctionnent. Les
> recettes, l'arbitre par modèle et l'interface de revue sont spécifiés
> mais pas encore développés. Voir [ce qui marche aujourd'hui](#ce-qui-marche-aujourdhui).

## Ce que fait l'outil

Chaque dossier finit sur l'un des trois chariots :

| Chariot | Signification |
| --- | --- |
| **Migrer** | Vous en avez besoin sur le nouveau PC. |
| **Archiver** | À conserver, mais pas utile au quotidien. Part en archive à froid. |
| **Laisser** | Retéléchargeable, reconstructible ou obsolète. |

Tout ce dont Bellcart n'est pas sûr est marqué **à examiner** et vous est
soumis. Rien n'est jamais supprimé ni modifié sur la machine analysée.

Il s'occupe aussi de ce qu'on oublie : clés SSH, profils de navigateur,
réglages d'éditeur, liste des applications installées. Tout n'est pas un
fichier à copier. Les mots de passe de Chrome, par exemple, sont liés à
votre compte Windows : la bonne action est « activer la synchronisation »,
pas « copier ce dossier ». Bellcart connaît la différence grâce à des
**recettes** maintenues par la communauté.

## Comment il décide

1. **Scan.** Noms de dossiers, tailles, extensions et dates. Le contenu des
   fichiers n'est jamais lu.
2. **Règles.** La plupart des décisions sont évidentes (`node_modules`,
   caches, fichiers temporaires) et se passent de modèle.
3. **Arbitre (optionnel).** Pour les dossiers restants, une signature
   compacte peut être soumise à un modèle sous forme de questions typées.
   Bellcart fonctionne sans aucun modèle : les dossiers indécis vous sont
   alors simplement laissés à examiner.
4. **Revue.** Vous confirmez ou corrigez. Un dossier n'est jamais laissé
   sur place sur la seule foi d'un modèle non calibré.

Le détail est dans la [spécification V1](docs/SPEC-V1.md) (en anglais).

## Ce qui marche aujourd'hui

```console
$ bellcart --lang fr scan C:\Users\moi
Plan pour C:\Users\moi

  À migrer           2 B  1 fichier
  À archiver         0 B  0 fichier
  À laisser       7.1 MB  3 fichiers
  À examiner      1.4 MB  3 fichiers

Applications trouvées : OpenSSH keys

Décidé par les règles et les recettes
      3.8 MB  À laisser   dev/app/node_modules  (Dépendances réinstallées par un gestionnaire de paquets)
      2.4 MB  À laisser   AppData/Local/Google/Chrome/User Data/Default/Cache  (Cache d'application, reconstruit automatiquement)
    878.9 KB  À laisser   AppData/Local/Temp  (Fichiers temporaires)
         2 B  À migrer    .ssh  (Clés et identifiants : petits, et pénibles à perdre)

À examiner, dossier par dossier
      1.4 MB  .  (assez petit pour être jugé d'un bloc)
```

- `bellcart ui` ouvre l'interface d'examen dans votre navigateur :
  choisir le profil, l'analyser, décider dossier par dossier avec trois
  chariots et leurs totaux, corriger ce que les règles ont réglé, puis
  exporter le plan et le carnet. Elle ne tourne que sur cette machine.
  Ajoutez `--endpoint` et `--model` pour y proposer aussi un arbitre.
- `bellcart scan <dossier>` affiche ce résumé ; `--json` affiche le plan
  complet, `--out plan.json` l'enregistre, et `--signatures` affiche ce
  qu'un arbitre lirait sur chaque dossier à examiner (`--privacy redacted`
  masque les noms sous les dossiers personnels, comme un arbitre en ligne
  les recevrait). L'analyse lit
  plusieurs dossiers à la fois ; `--jobs 1` les lit un par un, ce qui
  peut être plus rapide sur un disque dur à plateaux.
- `--notebook carnet.html` écrit le carnet de migration : ce qu'il faut
  faire à la main avant de quitter l'ancien PC et à l'arrivée, pour chaque
  application trouvée. Un nom finissant par `.html` donne une page avec
  des cases à cocher, tout autre nom donne du Markdown. Bellcart refuse
  d'écrire un fichier dans le dossier qu'il analyse.
- `bellcart export <profil> --to <dossier>` lance les exports demandés
  par les recettes (liste winget des applications, extensions de VS Code,
  clés de registre). Chaque commande est affichée en entier et ne se
  lance que si vous répondez oui.
- `bellcart arbitrate <profil> --endpoint <url> --model <nom>` soumet les
  dossiers à examiner à un modèle derrière un point d'accès compatible
  OpenAI (un serveur local ou une API en ligne ; la clé éventuelle est lue
  dans `BELLCART_ARBITER_KEY`). Il montre la requête exacte et demande
  avant d'envoyer quoi que ce soit, masque les noms personnels par défaut
  (`--privacy full` pour les envoyer), n'applique jamais un « laisser »
  sur la seule parole du modèle, et consigne chaque réponse dans
  `bellcart-decisions.jsonl`.
- `bellcart label <profil> --out labels.jsonl` vous demande, dossier par
  dossier, ce que vous en feriez, et `bellcart eval --labels labels.jsonl
  --endpoint <url> --model <nom>` rejoue ces réponses face à un modèle :
  accord par décision, et surtout tout dossier à garder qui serait laissé
  derrière.
- `bellcart packs` liste les règles et recettes intégrées. `--pack
  <dossier>` charge un autre pack à côté, pour les auteurs de packs (voir
  [docs/PACKS.md](docs/PACKS.md)).
- La langue du système est utilisée par défaut ; `--lang en` force
  l'anglais.

## Compiler

Bellcart est écrit en Rust et vise Windows 10 et 11. Le moteur ne contient
aucun code propre à Windows : on peut compiler et tester sous Linux ou
macOS.

```console
cargo build --release
cargo test
```

Le binaire produit est un exécutable portable unique : pas d'installeur,
pas de runtime, et pas besoin des droits administrateur pour les fichiers
de l'utilisateur courant.

## Engagements de vie privée

- Aucun accès réseau tant que vous ne configurez pas vous-même un arbitre.
- Le contenu des fichiers n'est jamais lu, seulement la liste des dossiers.
- Quand l'arbitre existera, il montrera exactement ce qui serait envoyé
  avant que quoi que ce soit ne quitte la machine.

## Contribuer une recette

Bellcart ne vaut que par ce qu'il sait des applications, et ce savoir vit
dans de simples fichiers YAML sous [`packs/`](packs/core), pas dans le code.
Si vous savez où une application range ses données et ce qu'il faut pour
les déplacer, quelques lignes suffisent. Voir le
[guide des packs](docs/PACKS.md) et [CONTRIBUTING.md](CONTRIBUTING.md)
(en anglais).

## Licence

[Apache-2.0](LICENSE).
