decision-migrate = À migrer
decision-archive = À archiver
decision-leave = À laisser
decision-review = À examiner

plan-title = Plan pour { $root }
plan-settled = Décidé par les règles
plan-nothing-settled = Aucune règle ne s'applique ici.
plan-unreadable = { $count ->
    [one] { $count } dossier illisible.
   *[other] { $count } dossiers illisibles.
} Relancez en administrateur pour les inclure.
plan-to-review = À examiner, dossier par dossier
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

unit-small = assez petit pour être jugé d'un bloc
unit-homogeneous = surtout des fichiers .{ $extension } ({ $percent } %)
unit-homogeneous-no-extension = surtout des fichiers sans extension ({ $percent } %)
unit-depth-limit = dossier profond, jugé d'un bloc
unit-leaf = aucun sous-dossier assez gros pour être jugé à part
unit-rest = fichiers posés dans ce dossier et ses petits sous-dossiers
