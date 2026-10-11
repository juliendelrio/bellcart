decision-migrate = À migrer
decision-archive = À archiver
decision-leave = À laisser
decision-review = À examiner

plan-title = Plan pour { $root }
plan-settled = Décidé par les règles et les recettes
plan-nothing-settled = Aucune règle ne s'applique ici.
plan-unreadable = { $count ->
    [one] { $count } dossier illisible.
   *[other] { $count } dossiers illisibles.
} Relancez en administrateur pour les inclure.
plan-to-review = À examiner, dossier par dossier
plan-applications = Applications trouvées : { $names }
source-recipe = Selon la recette { $name }
source-arbiter = Réponses de l'arbitre { $backend }
scan-progress = Analyse : { $folders } dossiers, { $size } parcourus
plan-files ={ $count ->
    [one] { $count } fichier
   *[other] { $count } fichiers
}

packs-title = { $name } { $version }
packs-rules = { $count ->
    [one] { $count } règle
   *[other] { $count } règles
}
packs-recipes = { $count ->
    [one] { $count } recette
   *[other] { $count } recettes
}
packs-questions = { $count ->
    [one] { $count } question d'arbitrage
   *[other] { $count } questions d'arbitrage
}

reason-regenerable-dependencies = Dépendances réinstallées par un gestionnaire de paquets
reason-build-output = Résultat de compilation, recréé à la prochaine compilation
reason-package-cache = Cache de gestionnaire de paquets, retéléchargé à la demande
reason-application-cache = Cache d'application, reconstruit automatiquement
reason-temporary-files = Fichiers temporaires
reason-system-managed = Géré par Windows
reason-keys-and-credentials = Clés et identifiants : petits, et pénibles à perdre
reason-cloud-sync-cache = Cache local d'un stockage en ligne, resynchronisé par son logiciel
reason-crash-reports = Rapports de plantage, utiles seulement aux développeurs de l'application
reason-installed-program = Programme installé, à réinstaller sur le nouveau PC
reason-update-download = Mise à jour déjà téléchargée, récupérée de nouveau au besoin

unit-small = assez petit pour être jugé d'un bloc
unit-homogeneous = surtout des fichiers .{ $extension } ({ $percent } %)
unit-homogeneous-no-extension = surtout des fichiers sans extension ({ $percent } %)
unit-depth-limit = dossier profond, jugé d'un bloc
unit-leaf = aucun sous-dossier assez gros pour être jugé à part
unit-rest = fichiers posés dans ce dossier et ses petits sous-dossiers

notebook-title = Carnet de migration pour { $root }
notebook-intro = Cochez chaque point au fur et à mesure. Les dossiers que le plan migre sont copiés avec le reste : ces points sont ce que les fichiers seuls n'emportent pas.
notebook-empty = Aucune application connue d'une recette n'a été trouvée sur ce profil.
notebook-before = Avant de quitter l'ancien PC
notebook-after = À l'arrivée sur le nouveau PC
notebook-copy = Emportez { $path }
notebook-copy-note = Emportez { $path } : { $note }
notebook-export-winget = Enregistrez la liste des applications installées avec winget (bellcart export peut le faire pour vous) : { $command }
notebook-export-vscode = Enregistrez la liste des extensions de Visual Studio Code (bellcart export peut le faire pour vous) : { $command }
notebook-export-registry = Enregistrez la clé de registre { $key } (bellcart export peut le faire pour vous) : { $command }
notebook-install = Installez-la avec winget : { $command }

export-intro = Bellcart ne lance chaque commande ci-dessous que si vous répondez oui. Les fichiers vont dans { $folder }.
export-none = Rien à exporter : aucune application trouvée sur ce profil n'a d'étape d'export.
export-winget = Applications installées avec winget
export-vscode = Extensions de Visual Studio Code
export-registry = Clé de registre { $key }
export-ask = La lancer ? [o/N]
export-skipped = Ignorée.
export-done = Enregistré : { $file }
export-failed = Échec : { $error }
export-missing = { $program } est introuvable sur ce PC.

arbitrate-none = Rien à soumettre à un arbitre : aucun dossier n'est à examiner.
arbitrate-preview = Voici la requête exacte pour le premier dossier. Les autres ne diffèrent que par leur résumé.
arbitrate-confirm = Envoyer { $count } résumés de dossiers à { $url } ? [o/N]
arbitrate-nothing-sent = Rien n'a été envoyé.
arbitrate-failed = { $path } : { $error }
arbitrate-stopped = Arrêt après { $count } échecs d'affilée : le point d'accès répond-il ?
arbitrate-leave-to-confirm = l'arbitre propose de le laisser : à confirmer

label-prompt = m (migrer), a (archiver), l (laisser), s (sauter), q (quitter) ?
label-already = { $count } dossiers déjà étiquetés sont sautés.
label-done = { $count } nouvelles étiquettes dans { $file }.
eval-confirm = Envoyer { $count } résumés étiquetés à { $url } ? [o/N]
eval-title = { $backend } sur { $count } dossiers étiquetés : { $agreed } en accord
eval-label = { $label } : { $agreed } sur { $total } en accord
eval-lost-none = Aucun dossier à garder ne serait laissé derrière.
eval-lost = Dossiers à garder qui seraient laissés derrière : { $paths }
eval-leave-suggested = Laisser proposé, et retenu pour l'utilisateur, sur des dossiers à migrer : { $paths }
eval-failed = Sans réponse : { $paths }
